use std::sync::Arc;

use image::{DynamicImage, RgbaImage};
use ratatui::layout::Size;
use ratatui_image::protocol::{Protocol, iterm2::Iterm2, kitty::Kitty, sixel::Sixel};
use resvg::{tiny_skia, usvg};

use crate::{
    rich::{self, RichAsset},
    tui::app::options::{ImageCellSize, ImageProtocol},
    ui_ast::RichContent,
};

use super::super::{
    GraphicsOutput,
    raster_budget::{self, RasterError},
    viewport::{ImageRegion, ImageViewport},
};

pub(super) struct RasterInput {
    pub content: Arc<RichContent>,
    pub viewport: ImageViewport,
    pub available: Size,
    pub cell: ImageCellSize,
    pub output: GraphicsOutput,
}

pub(super) struct RasterOutput {
    pub available: Size,
    pub region: ImageRegion,
    pub protocol: Protocol,
}

struct Document {
    tree: usvg::Tree,
    background: tiny_skia::Color,
}

#[derive(Default)]
pub(super) struct Rasterizer {
    fonts: Option<Arc<usvg::fontdb::Database>>,
    document: Option<(Arc<RichContent>, Result<Document, String>)>,
}

impl Rasterizer {
    pub fn prepare(
        &mut self,
        content: Arc<RichContent>,
        current: &impl Fn() -> bool,
    ) -> Result<Option<(f32, f32)>, String> {
        if !current() {
            return Ok(None);
        }
        if self
            .document
            .as_ref()
            .is_none_or(|(source, _)| !Arc::ptr_eq(source, &content))
        {
            self.document = None;
            let document = match self.parse(content.clone(), current) {
                Ok(Some(document)) => Ok(document),
                Ok(None) => return Ok(None),
                Err(error) => Err(error),
            };
            self.document = Some((content, document));
        }
        if !current() {
            return Ok(None);
        }
        let document = self
            .document
            .as_ref()
            .expect("image document was prepared")
            .1
            .as_ref()
            .map_err(Clone::clone)?;
        let size = document.tree.size();
        Ok(Some((size.width(), size.height())))
    }

    pub fn render(
        &mut self,
        input: RasterInput,
        current: &impl Fn() -> bool,
    ) -> Result<Option<RasterOutput>, String> {
        if self.prepare(input.content, current)?.is_none() {
            return Ok(None);
        }
        let document = self
            .document
            .as_ref()
            .expect("image document was prepared")
            .1
            .as_ref()
            .map_err(Clone::clone)?;
        let mut available = input.available;
        let (region, width, height, transform) = loop {
            if !current() {
                return Ok(None);
            }
            // Each candidate starts from the UI's center, not a rejected larger area's clamp.
            let mut viewport = input.viewport.clone();
            viewport.resize(available, input.cell);
            let region = viewport.region();
            let width = u32::from(region.size.width) * u32::from(input.cell.width());
            let height = u32::from(region.size.height) * u32::from(input.cell.height());
            let raster_size = tiny_skia::IntSize::from_wh(width, height)
                .ok_or("Native image viewport is empty")?;
            let scale = (region.scale * f64::from(input.cell.width())) as f32;
            let transform = tiny_skia::Transform::from_row(
                scale,
                0.0,
                0.0,
                scale,
                (-region.x * f64::from(input.cell.width())) as f32,
                (-region.y * f64::from(input.cell.height())) as f32,
            );
            match raster_budget::check(
                &document.tree,
                raster_size,
                transform,
                input.output.protocol,
                input.output.is_tmux,
            ) {
                Ok(()) => break (region, width, height, transform),
                Err(RasterError::Budget) if viewport.reduce_area() => {
                    available = viewport.available();
                }
                Err(error) => return Err(error.to_string()),
            }
        };
        if !current() {
            return Ok(None);
        }
        let mut pixmap =
            tiny_skia::Pixmap::new(width, height).ok_or("cannot allocate image viewport raster")?;
        pixmap.fill(document.background);
        resvg::render(&document.tree, transform, &mut pixmap.as_mut());
        if !current() {
            return Ok(None);
        }
        let rgba = RgbaImage::from_raw(width, height, pixmap.take_demultiplied())
            .ok_or("cannot use image viewport RGBA buffer")?;
        if !current() {
            return Ok(None);
        }
        let image = DynamicImage::ImageRgba8(rgba);
        let protocol = match input.output.protocol {
            ImageProtocol::Kitty => Kitty::new(
                image,
                region.size,
                input.output.kitty_id,
                input.output.is_tmux,
                true,
            )
            .map(Protocol::Kitty),
            ImageProtocol::Iterm2 => {
                Iterm2::new(image, region.size, input.output.is_tmux).map(Protocol::ITerm2)
            }
            ImageProtocol::Sixel => Sixel::new(image, region.size, false).map(Protocol::Sixel),
            ImageProtocol::Auto | ImageProtocol::Source => {
                return Err("No native image protocol was selected; use source".into());
            }
        }
        .map_err(|error| format!("image encoding: {error}"))?;
        Ok(current().then_some(RasterOutput {
            available,
            region,
            protocol,
        }))
    }

    fn parse(
        &mut self,
        content: Arc<RichContent>,
        current: &impl Fn() -> bool,
    ) -> Result<Option<Document>, String> {
        if content.source().len() > rich::MAX_SOURCE_BYTES {
            return Err(format!("source exceeds {} bytes", rich::MAX_SOURCE_BYTES));
        }
        let svg = match content.as_ref() {
            RichContent::Svg { .. } => match rich::render(&content) {
                Some(RichAsset::Svg { svg }) => svg,
                Some(RichAsset::Failed { message }) => return Err(message),
                _ => return Err("SVG rendering is unavailable in this build".into()),
            },
            #[cfg(feature = "mermaid")]
            RichContent::Mermaid { source } => {
                rich::render_diagram(source, rich::DiagramTheme::Dark)?
            }
            #[cfg(not(feature = "mermaid"))]
            RichContent::Mermaid { .. } => {
                return Err(
                    "Mermaid preview requires the `mermaid` feature; source remains available"
                        .into(),
                );
            }
            RichContent::Markdown { .. } => return Err("Markdown is displayed as source".into()),
        };
        if !current() {
            return Ok(None);
        }
        let fonts = self.fonts.get_or_insert_with(|| {
            let mut fonts = usvg::fontdb::Database::new();
            fonts.load_system_fonts();
            let family = fonts
                .faces()
                .find_map(|face| face.families.first().map(|family| family.0.clone()));
            if let Some(family) = family {
                if fonts
                    .query(&usvg::fontdb::Query {
                        families: &[usvg::fontdb::Family::Serif],
                        ..Default::default()
                    })
                    .is_none()
                {
                    fonts.set_serif_family(family.clone());
                }
                if fonts
                    .query(&usvg::fontdb::Query {
                        families: &[usvg::fontdb::Family::SansSerif],
                        ..Default::default()
                    })
                    .is_none()
                {
                    fonts.set_sans_serif_family(family);
                }
            }
            Arc::new(fonts)
        });
        if !current() {
            return Ok(None);
        }
        if fonts.faces().next().is_none() && svg.body.contains("<text") {
            return Err("no system fonts are available for SVG labels; use source".into());
        }
        let options = usvg::Options {
            fontdb: fonts.clone(),
            image_href_resolver: usvg::ImageHrefResolver {
                resolve_string: Box::new(|_, _| None),
                resolve_data: Box::new(|_, _, _| None),
            },
            max_depth: 128,
            ..Default::default()
        };
        let tree = usvg::Tree::from_str(&svg.body, &options)
            .map_err(|error| format!("SVG rasterisation: {error}"))?;
        let background = if matches!(content.as_ref(), RichContent::Mermaid { .. }) {
            tiny_skia::Color::from_rgba8(51, 51, 51, 255)
        } else {
            tiny_skia::Color::WHITE
        };
        Ok(Some(Document { tree, background }))
    }
}
