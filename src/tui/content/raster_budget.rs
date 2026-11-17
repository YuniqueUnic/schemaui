use std::fmt;

use resvg::{tiny_skia, usvg};

use crate::tui::app::options::ImageProtocol;

const MAX_RASTER_BYTES: u64 = 128 * 1024 * 1024;
const ENCODER_WORKSPACE: u64 = 1024 * 1024;
const BUDGET_ERROR: &str = "SVG exceeds the 128 MiB raster working-set estimate; use source";

pub(super) enum RasterError {
    Budget,
    Unsupported(String),
}

impl From<&str> for RasterError {
    fn from(message: &str) -> Self {
        Self::Unsupported(message.into())
    }
}

impl fmt::Display for RasterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Budget => formatter.write_str(BUDGET_ERROR),
            Self::Unsupported(message) => formatter.write_str(message),
        }
    }
}

pub(super) fn pixel_limit(protocol: ImageProtocol, is_tmux: bool) -> u64 {
    (MAX_RASTER_BYTES - ENCODER_WORKSPACE) / encoding_bytes_per_pixel(protocol, is_tmux)
}

// Estimates for the pinned encoders include RGBA copies, growing encoded strings
// and terminal cells. Compression reduces actual wire size, not this allowance.
fn encoding_bytes_per_pixel(protocol: ImageProtocol, is_tmux: bool) -> u64 {
    match protocol {
        ImageProtocol::Kitty if is_tmux => 40,
        ImageProtocol::Kitty => 32,
        ImageProtocol::Iterm2 => 36,
        ImageProtocol::Sixel => 192,
        ImageProtocol::Auto | ImageProtocol::Source => 64,
    }
}

pub(super) fn check(
    tree: &usvg::Tree,
    size: tiny_skia::IntSize,
    transform: tiny_skia::Transform,
    protocol: ImageProtocol,
    is_tmux: bool,
) -> Result<(), RasterError> {
    // Check geometry before requesting lazy bounding boxes, which can stroke paths.
    let geometry = geometry_peak(tree.root())?;
    let bounds = size.to_int_rect(0, 0);
    let pixels = u64::from(size.width()) * u64::from(size.height());
    let rendering = pixels * 4 + geometry + nodes_peak(tree.root(), bounds, transform)?;
    let encoding = pixels * encoding_bytes_per_pixel(protocol, is_tmux) + ENCODER_WORKSPACE;
    check_bytes(rendering.max(encoding))
}

fn check_bytes(bytes: u64) -> Result<(), RasterError> {
    if bytes > MAX_RASTER_BYTES {
        return Err(RasterError::Budget);
    }
    Ok(())
}

fn nodes_peak(
    group: &usvg::Group,
    bounds: tiny_skia::IntRect,
    transform: tiny_skia::Transform,
) -> Result<u64, RasterError> {
    let mut peak = 0;
    for node in group.children() {
        let bytes = match node {
            usvg::Node::Group(child) => group_peak(child, bounds, transform)?,
            usvg::Node::Text(text) => group_peak(text.flattened(), bounds, transform)?,
            _ => 0,
        };
        peak = peak.max(bytes);
    }
    Ok(peak)
}

fn group_peak(
    group: &usvg::Group,
    bounds: tiny_skia::IntRect,
    transform: tiny_skia::Transform,
) -> Result<u64, RasterError> {
    let transform = transform.pre_concat(group.transform());
    if !group.should_isolate() {
        return nodes_peak(group, bounds, transform);
    }
    let Some(bbox) = group.layer_bounding_box() else {
        return Ok(0);
    };
    let error = "SVG layer bounds exceed terminal preview limits; use source";
    let bbox = bbox.transform(transform).ok_or(error)?;
    let layer = padded_layer_bounds(bbox).ok_or(error)?;
    let Some(layer) = layer.intersect(&bounds) else {
        return Ok(0);
    };
    let pixels = u64::from(layer.width()) * u64::from(layer.height());
    check_bytes(pixels * 4)?;

    // Match the pinned resvg's layer-local transform and needed pixel bounds.
    // Filters are rejected, so its larger filter bounds do not affect allocation.
    let dx = bbox.x() - (bbox.x() - layer.x() as f32);
    let dy = bbox.y() - (bbox.y() - layer.y() as f32);
    let transform = tiny_skia::Transform::from_translate(-dx, -dy).pre_concat(transform);
    let bounds = layer.size().to_int_rect(0, 0);
    let mut peak = nodes_peak(group, bounds, transform)?;
    if let Some(clip) = group.clip_path() {
        peak = peak.max(clip_peak(clip, pixels));
    }
    if let Some(mask) = group.mask() {
        peak = peak.max(mask_peak(mask, bounds, transform, pixels)?);
    }
    let peak = pixels * 4 + peak;
    check_bytes(peak)?;
    Ok(peak)
}

fn padded_layer_bounds(bbox: tiny_skia::NonZeroRect) -> Option<tiny_skia::IntRect> {
    tiny_skia::IntRect::from_xywh(
        (bbox.x().floor() as i32).checked_sub(2)?,
        (bbox.y().floor() as i32).checked_sub(2)?,
        (bbox.width().ceil() as u32).checked_add(4)?,
        (bbox.height().ceil() as u32).checked_add(4)?,
    )
}

fn clip_peak(clip: &usvg::ClipPath, pixels: u64) -> u64 {
    let children = clip_nodes_peak(clip.root(), pixels);
    let chained = clip.clip_path().map_or(0, |clip| clip_peak(clip, pixels));
    // Each active apply/clip_group owns a full-size RGBA layer and alpha scratch.
    pixels * 6 + children.max(chained)
}

fn clip_nodes_peak(group: &usvg::Group, pixels: u64) -> u64 {
    let mut peak = 0;
    for node in group.children() {
        let bytes = match node {
            usvg::Node::Group(child) => {
                let children = clip_nodes_peak(child, pixels);
                if let Some(clip) = child.clip_path() {
                    pixels * 6 + children.max(clip_peak(clip, pixels))
                } else {
                    children
                }
            }
            usvg::Node::Text(text) => clip_nodes_peak(text.flattened(), pixels),
            _ => 0,
        };
        peak = peak.max(bytes);
    }
    peak
}

fn mask_peak(
    mask: &usvg::Mask,
    bounds: tiny_skia::IntRect,
    transform: tiny_skia::Transform,
    pixels: u64,
) -> Result<u64, RasterError> {
    if mask.root().children().is_empty() {
        return Ok(0);
    }
    let children = nodes_peak(mask.root(), bounds, transform)?;
    let chained = match mask.mask() {
        Some(mask) => mask_peak(mask, bounds, transform, pixels)?,
        None => 0,
    };
    let peak = pixels * 6 + children.max(chained);
    check_bytes(peak)?;
    Ok(peak)
}

fn geometry_peak(group: &usvg::Group) -> Result<u64, RasterError> {
    if !group.filters().is_empty() {
        return Err("SVG filters are not supported in terminal previews; use source".into());
    }
    let mut peak = 0;
    let mut clip = group.clip_path();
    while let Some(current) = clip {
        peak = peak.max(geometry_peak(current.root())?);
        clip = current.clip_path();
    }
    let mut mask = group.mask();
    while let Some(current) = mask {
        peak = peak.max(geometry_peak(current.root())?);
        mask = current.mask();
    }
    for node in group.children() {
        let bytes = match node {
            usvg::Node::Group(child) => geometry_peak(child)?,
            usvg::Node::Text(text) => geometry_peak(text.flattened())?,
            usvg::Node::Path(path) => path_geometry(path)?,
            usvg::Node::Image(_) => {
                return Err(
                    "Embedded SVG images are not supported in terminal previews; use source".into(),
                );
            }
        };
        peak = peak.max(bytes);
    }
    Ok(peak)
}

fn path_geometry(path: &usvg::Path) -> Result<u64, RasterError> {
    if path
        .fill()
        .is_some_and(|fill| matches!(fill.paint(), usvg::Paint::Pattern(_)))
        || path
            .stroke()
            .is_some_and(|stroke| matches!(stroke.paint(), usvg::Paint::Pattern(_)))
    {
        return Err("SVG patterns are not supported in terminal previews; use source".into());
    }
    let mut segments = path.data().len() as f64;
    if let Some(dashes) = path.stroke().and_then(usvg::Stroke::dasharray) {
        let minimum = dashes.iter().copied().fold(f32::INFINITY, f32::min);
        if !minimum.is_finite() || minimum <= 0.0 {
            return Err(
                "SVG dash spacing is not supported in terminal previews; use source".into(),
            );
        }
        segments += control_polygon_length(path.data()) / f64::from(minimum);
    }
    if !segments.is_finite() || segments > 65_536.0 {
        return Err("SVG path complexity exceeds the terminal preview limit; use source".into());
    }
    // A geometry allowance, not an allocator guarantee. Paths render sequentially.
    Ok(segments.ceil() as u64 * 1024)
}

fn control_polygon_length(path: &tiny_skia::Path) -> f64 {
    use tiny_skia::PathSegment;

    let mut previous = tiny_skia::Point::from_xy(0.0, 0.0);
    let mut start = previous;
    let mut length = 0.0;
    for segment in path.segments() {
        let mut edge = |point: tiny_skia::Point| {
            length += (f64::from(point.x) - f64::from(previous.x))
                .hypot(f64::from(point.y) - f64::from(previous.y));
            previous = point;
        };
        match segment {
            PathSegment::MoveTo(point) => {
                previous = point;
                start = point;
            }
            PathSegment::LineTo(point) => edge(point),
            PathSegment::QuadTo(control, point) => {
                edge(control);
                edge(point);
            }
            PathSegment::CubicTo(first, second, point) => {
                edge(first);
                edge(second);
                edge(point);
            }
            PathSegment::Close => edge(start),
        }
    }
    length
}
