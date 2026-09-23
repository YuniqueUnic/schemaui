//! Whitelist sanitiser for SVG sources.
//!
//! A schema is 半受信 input: it may arrive from an agent or be pasted from
//! anywhere, and raw SVG can carry `<script>`, `on*` handlers, `foreignObject`
//! and off-document `href`s. This module rebuilds an SVG document keeping
//! only whitelisted elements and attributes, so every frontend receives
//! markup that is inert by construction rather than trusted by convention.
//! The same pass lifts the intrinsic size off the root element, which is what
//! lets thumbnails preserve their aspect ratio.
//!
//! Dropped on principle: `<style>` (CSS can smuggle `url()` exfiltration and
//! legacy `expression()`), `<image>` (external fetches), comments, the doctype
//! and processing instructions. Everything else SVG needs for figures —
//! shapes, gradients, markers, text, transforms — survives.

use quick_xml::Reader;
use quick_xml::events::Event;

use super::asset::{RenderedSvg, RichAsset};

/// Elements a figure is allowed to be built from.
const ALLOWED_ELEMENTS: &[&str] = &[
    "svg",
    "g",
    "defs",
    "symbol",
    "marker",
    "title",
    "desc",
    "clipPath",
    "mask",
    "linearGradient",
    "radialGradient",
    "stop",
    "path",
    "rect",
    "circle",
    "ellipse",
    "line",
    "polyline",
    "polygon",
    "text",
    "tspan",
    "use",
];

/// Attributes kept on surviving elements, by local name.
const ALLOWED_ATTRIBUTES: &[&str] = &[
    // Document structure
    "id",
    "version",
    "viewBox",
    "preserveAspectRatio",
    "xml:space",
    "role",
    "aria-label",
    "aria-hidden",
    "aria-describedby",
    // Geometry and transforms
    "x",
    "y",
    "width",
    "height",
    "cx",
    "cy",
    "r",
    "rx",
    "ry",
    "x1",
    "x2",
    "y1",
    "y2",
    "d",
    "points",
    "transform",
    // Paint and stroke
    "fill",
    "fill-opacity",
    "fill-rule",
    "stroke",
    "stroke-width",
    "stroke-opacity",
    "stroke-dasharray",
    "stroke-dashoffset",
    "stroke-linecap",
    "stroke-linejoin",
    "stroke-miterlimit",
    "opacity",
    "color",
    "shape-rendering",
    // Gradients, markers, clipping
    "gradientUnits",
    "gradientTransform",
    "spreadMethod",
    "offset",
    "stop-color",
    "stop-opacity",
    "clip-path",
    "clip-rule",
    "mask",
    "marker-start",
    "marker-mid",
    "marker-end",
    "markerWidth",
    "markerHeight",
    "markerUnits",
    "orient",
    "refX",
    "refY",
    // Text
    "text-anchor",
    "dominant-baseline",
    "font-family",
    "font-size",
    "font-style",
    "font-weight",
    "textLength",
    "dx",
    "dy",
    "letter-spacing",
    "word-spacing",
];

pub(super) fn svg_asset(source: &str) -> RichAsset {
    match sanitize(source) {
        Ok(svg) => RichAsset::Svg { svg },
        Err(message) => RichAsset::Failed { message },
    }
}

/// Rebuild `source` keeping only whitelisted elements and attributes, and
/// lift the intrinsic size off the root element.
pub(crate) fn sanitize(source: &str) -> Result<RenderedSvg, String> {
    let mut reader = Reader::from_str(source);

    let mut out = String::new();
    // Local names of the allowed elements currently open, replayed in order
    // for the closing tags — the rebuilt document is prefix-free. Owned:
    // element events are dropped as the reader moves on.
    let mut open: Vec<String> = Vec::new();
    // Depth inside a dropped subtree: everything until it closes is skipped,
    // including nested allowed elements (an allowed shape inside a dropped
    // `<foreignObject>` must not resurface).
    let mut skipped_depth = 0usize;
    let mut root: Option<RootSize> = None;
    // Character data is buffered across the reader's split events (it reports
    // every entity reference as its own event, so "Hello &amp; welcome"
    // arrives as three) and flushed as one text node at the next markup
    // boundary. Only then is it trimmed: trimming per event would eat the
    // whitespace an entity split left at a piece's edge.
    let mut text = String::new();

    loop {
        let event = reader
            .read_event()
            .map_err(|error| format!("SVG is not well-formed: {error}"))?;
        match event {
            Event::Start(element) => {
                flush_text(&mut out, &mut text, &open, skipped_depth);
                let name = local_name(element.name().as_ref()).to_string();
                let attrs = attributes(&element)?;
                if skipped_depth > 0 {
                    skipped_depth += 1;
                } else if !ALLOWED_ELEMENTS.contains(&name.as_str()) {
                    skipped_depth = 1;
                } else {
                    if root.is_none() {
                        if name != "svg" {
                            return Err("expected an <svg> document".to_string());
                        }
                        root = Some(root_size(&attrs));
                    }
                    write_start(&mut out, &name, &attrs);
                    out.push('>');
                    open.push(name);
                }
            }
            Event::Empty(element) => {
                flush_text(&mut out, &mut text, &open, skipped_depth);
                let name = local_name(element.name().as_ref()).to_string();
                let attrs = attributes(&element)?;
                if skipped_depth == 0 {
                    if !ALLOWED_ELEMENTS.contains(&name.as_str()) {
                        continue;
                    }
                    if root.is_none() {
                        if name != "svg" {
                            return Err("expected an <svg> document".to_string());
                        }
                        root = Some(root_size(&attrs));
                    }
                    // Self-closing: same output as open+close with no text
                    // between them, without touching the open stack.
                    write_start(&mut out, &name, &attrs);
                    out.push_str("/>");
                }
            }
            Event::End(_) => {
                flush_text(&mut out, &mut text, &open, skipped_depth);
                if skipped_depth > 0 {
                    skipped_depth -= 1;
                } else {
                    let Some(name) = open.pop() else {
                        return Err("SVG is not well-formed: unmatched closing tag".to_string());
                    };
                    out.push_str("</");
                    out.push_str(&name);
                    out.push('>');
                }
            }
            Event::Text(text_event) => {
                if skipped_depth == 0 {
                    // Entity-free by construction: the reader reports every
                    // reference as its own event, so what is left is literal.
                    text.push_str(&text_event.xml10_content());
                }
            }
            Event::GeneralRef(reference) => {
                // The reference itself ("amp", "#38", "#x26"), resolved back
                // to the characters the author meant. Anything beyond XML's
                // built-ins has no DTD left to define it and passes through
                // as written.
                if skipped_depth == 0 {
                    let name = reference.as_ref();
                    let resolved = match name {
                        "amp" => "&".to_string(),
                        "lt" => "<".to_string(),
                        "gt" => ">".to_string(),
                        "quot" => "\"".to_string(),
                        "apos" => "'".to_string(),
                        _ => numeric_reference(name)
                            .and_then(char::from_u32)
                            .map(|character| character.to_string())
                            .unwrap_or_else(|| format!("&{name};")),
                    };
                    text.push_str(&resolved);
                }
            }
            Event::CData(cdata) => {
                if skipped_depth == 0 {
                    text.push_str(cdata.as_ref());
                }
            }
            Event::Eof => {
                flush_text(&mut out, &mut text, &open, skipped_depth);
                break;
            }
            // Declaration, comments, doctype and processing instructions are
            // dropped along with everything else the whitelist never names.
            _ => {}
        }
    }

    if !open.is_empty() {
        return Err("SVG is not well-formed: unclosed elements".to_string());
    }
    // Prefer the viewBox: unlike `width="120px"` it always describes the
    // drawing's own aspect ratio rather than a particular export's sizing.
    let Some(size) = root.and_then(RootSize::into_size) else {
        return Err("expected an <svg> document".to_string());
    };
    Ok(RenderedSvg {
        body: out,
        width: Some(size.0),
        height: Some(size.1),
    })
}

/// Emit the buffered text node, if there is content to show for it.
///
/// Whitespace-only runs between elements are markup formatting and go; the
/// boundary whitespace of a real text node goes with them. Interior
/// whitespace — including what an entity split left at a piece's edge — is
/// content and survives.
fn flush_text(out: &mut String, text: &mut String, open: &[String], skipped_depth: usize) {
    if skipped_depth == 0 && !open.is_empty() && !text.is_empty() {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            escape_into(out, trimmed);
        }
    }
    text.clear();
}

/// Write `<name` plus the surviving attributes; the caller closes the tag
/// (`>` for a container, `/>` for a self-closing element).
fn write_start(out: &mut String, name: &str, attrs: &[(String, String)]) {
    out.push('<');
    out.push_str(name);
    for (key, value) in attrs {
        if !attribute_allowed(key, value) {
            continue;
        }
        out.push(' ');
        out.push_str(key);
        out.push_str("=\"");
        escape_into(out, value);
        out.push('"');
    }
}

/// The code point of a numeric character reference (`#38`, `#x26`).
fn numeric_reference(name: &str) -> Option<u32> {
    let digits = name.strip_prefix('#')?;
    u32::from_str_radix(
        digits.strip_prefix('x').unwrap_or(digits),
        if digits.starts_with('x') { 16 } else { 10 },
    )
    .ok()
}

fn attributes(
    element: &quick_xml::events::BytesStart<'_>,
) -> Result<Vec<(String, String)>, String> {
    let mut attrs = Vec::new();
    for attr in element.attributes() {
        let attr = attr.map_err(|error| format!("SVG is not well-formed: {error}"))?;
        // Names arrive as `&str` behind whatever namespace prefix they carry;
        // the whitelist match is on the local name.
        let key = attr.key.as_ref().to_string();
        let value = attr
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
            .map_err(|error| format!("SVG is not well-formed: {error}"))?
            .into_owned();
        attrs.push((key, value));
    }
    Ok(attrs)
}

fn attribute_allowed(key: &str, value: &str) -> bool {
    let local = local_name(key);
    if key.starts_with("xmlns") || key == "xmlns" {
        // Namespace declarations on the root are structural, and the rebuilt
        // document only ever carries the SVG ones.
        return matches!(
            value,
            "http://www.w3.org/2000/svg" | "http://www.w3.org/1999/xlink"
        );
    }
    if matches!(local, "href" | "xlink:href") {
        // Internal references only: an off-document URL is a fetch (and a
        // `javascript:` one is an attack), while intra-document `#id` refs
        // are how gradients and markers point at their definitions.
        return value.starts_with('#');
    }
    ALLOWED_ATTRIBUTES.contains(&local)
}

/// The name without any namespace prefix, ASCII-cased the way SVG spells it.
fn local_name(name: &str) -> &str {
    match name.rsplit_once(':') {
        Some((_, local)) => local,
        None => name,
    }
}

/// The intrinsic size the root element declares, in the three shapes SVG
/// allows it: `width`/`height` attributes, a `viewBox`, or both.
struct RootSize {
    width: Option<f64>,
    height: Option<f64>,
    viewbox: Option<(f64, f64)>,
}

impl RootSize {
    /// `(width, height)` as declared, `None` when the source sizes itself by
    /// its container (percentages, no viewBox).
    fn into_size(self) -> Option<(f64, f64)> {
        self.viewbox.or_else(|| self.width.zip(self.height))
    }
}

/// Read [`RootSize`] off the root element's attributes.
fn root_size(attrs: &[(String, String)]) -> RootSize {
    let mut size = RootSize {
        width: None,
        height: None,
        viewbox: None,
    };
    for (key, value) in attrs {
        match local_name(key) {
            // Percent widths describe the container, not the drawing.
            "width" if !value.ends_with('%') => size.width = value.parse().ok(),
            "height" if !value.ends_with('%') => size.height = value.parse().ok(),
            "viewBox" => {
                let parts = value
                    .split_whitespace()
                    .filter_map(|n| n.parse::<f64>().ok());
                let coords: Vec<f64> = parts.collect();
                if coords.len() == 4 {
                    size.viewbox = Some((coords[2], coords[3]));
                }
            }
            _ => {}
        }
    }
    size
}

/// Escape text for re-emission inside an element or attribute value.
fn escape_into(out: &mut String, text: &str) {
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
}
