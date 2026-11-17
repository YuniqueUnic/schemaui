use std::time::Duration;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};

use crate::ui_ast::RichContent;

use super::{
    app::options::{ImageCellSize, ImageProtocol},
    view::components::{fields::text_layout::wrap_preserving_spaces, footer::countdown_label},
};

#[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
use super::app::terminal::graphics::GraphicsOutput;

#[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
mod images;
#[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
mod raster_budget;
#[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
mod viewport;

pub(crate) struct ContentRenderer {
    #[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
    images: images::ImageRenderer,
    #[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
    output: Result<GraphicsOutput, String>,
}

impl ContentRenderer {
    pub fn new(protocol: ImageProtocol, cell_size: Option<ImageCellSize>) -> Self {
        let _ = (protocol, cell_size);
        Self {
            #[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
            images: images::ImageRenderer::default(),
            #[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
            output: GraphicsOutput::detect(protocol, cell_size),
        }
    }
}

pub(crate) struct ContentItem {
    pub title: String,
    pub content: RichContent,
}

#[derive(Clone, Copy)]
enum ContentMode {
    Preview,
    Source,
}

enum Preview {
    SourceOnly(String),
    #[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
    Failed(String),
    #[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
    Image {
        image: Box<images::ImagePreview>,
        rendered: Result<(), String>,
    },
}

impl Preview {
    fn render_image(&mut self, frame: &mut Frame<'_>, area: Rect) -> bool {
        #[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
        if let Self::Image { image, rendered } = self {
            *rendered = image.render(frame, area);
            if rendered.is_err() {
                frame.render_widget(Clear, area);
            }
            return rendered.is_ok();
        }
        let _ = (frame, area);
        false
    }

    fn message(&self) -> (String, Color) {
        match self {
            Self::SourceOnly(reason) => (reason.clone(), Color::Cyan),
            #[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
            Self::Failed(error) => (
                format!("Preview failed: {error} · showing source"),
                Color::Red,
            ),
            #[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
            Self::Image {
                rendered: Err(error),
                ..
            } => (
                format!("Preview failed: {error} · showing source"),
                Color::Red,
            ),
            #[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
            Self::Image {
                image,
                rendered: Ok(()),
            } => (
                format!(
                    "{} · native {} pixels · {} · i/o zoom · f fit · arrows pan",
                    if image.pending() {
                        "Rendering"
                    } else {
                        "Preview"
                    },
                    image.protocol(),
                    image.description(),
                ),
                Color::Cyan,
            ),
        }
    }
}

pub(crate) struct ContentViewer {
    items: Vec<ContentItem>,
    selected: usize,
    mode: ContentMode,
    preview: Preview,
    scroll: usize,
    source_lines: usize,
    page_height: usize,
    help: String,
}

impl ContentViewer {
    pub fn new(items: Vec<ContentItem>, help: String, renderer: &mut ContentRenderer) -> Self {
        let mut viewer = Self {
            items,
            selected: 0,
            mode: ContentMode::Preview,
            preview: Preview::SourceOnly(String::new()),
            scroll: 0,
            source_lines: 0,
            page_height: 1,
            help,
        };
        viewer.prepare(renderer);
        viewer
    }

    fn prepare(&mut self, renderer: &mut ContentRenderer) {
        self.scroll = 0;
        let Some(item) = self.items.get(self.selected) else {
            return;
        };
        if matches!(item.content, RichContent::Markdown { .. }) {
            self.preview =
                Preview::SourceOnly("Markdown is shown as source in the terminal.".into());
            return;
        }
        #[cfg(not(feature = "mermaid"))]
        if matches!(item.content, RichContent::Mermaid { .. }) {
            self.preview = Preview::SourceOnly(
                "Mermaid preview requires the `mermaid` feature; showing source.".into(),
            );
            return;
        }
        #[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
        {
            let output = match renderer.output.as_ref() {
                Ok(output) => *output,
                Err(reason) => {
                    self.preview = Preview::SourceOnly(reason.clone());
                    return;
                }
            };
            if let Err(reason) = output.cell_size() {
                self.preview = Preview::SourceOnly(reason);
                return;
            }
            self.preview = match renderer.images.render(&item.content, output) {
                Ok(image) => Preview::Image {
                    image: Box::new(image),
                    rendered: Ok(()),
                },
                Err(error) => Preview::Failed(error),
            };
        }
        #[cfg(not(all(feature = "tui-images", not(target_arch = "wasm32"))))]
        {
            let _ = renderer;
            self.preview = Preview::SourceOnly(
                "Image preview requires the native `tui-images` feature; showing source.".into(),
            );
        }
    }

    pub fn step(&mut self, delta: i32, renderer: &mut ContentRenderer) {
        if self.items.is_empty() {
            return;
        }
        self.selected =
            (self.selected as i64 + i64::from(delta)).rem_euclid(self.items.len() as i64) as usize;
        self.prepare(renderer);
    }

    pub fn invalidate_image(&mut self) {
        #[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
        if let Preview::Image { image, .. } = &mut self.preview {
            image.invalidate();
        }
    }

    pub fn image_pending(&self) -> bool {
        #[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
        if matches!(self.mode, ContentMode::Preview)
            && let Preview::Image { image, .. } = &self.preview
        {
            return image.pending();
        }
        false
    }

    pub fn suspend_image(&mut self) {
        #[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
        if let Preview::Image { image, .. } = &mut self.preview {
            image.suspend();
        }
    }

    pub fn toggle_source(&mut self) {
        self.suspend_image();
        self.mode = match self.mode {
            ContentMode::Preview => ContentMode::Source,
            ContentMode::Source => ContentMode::Preview,
        };
    }

    pub fn scroll(&mut self, horizontal: i32, vertical: i32, page: bool) {
        #[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
        if matches!(self.mode, ContentMode::Preview)
            && let Preview::Image { image, .. } = &mut self.preview
        {
            image.scroll(horizontal, vertical, page);
            return;
        }
        let _ = horizontal;
        let step = vertical.unsigned_abs() as usize * if page { self.page_height } else { 1 };
        self.scroll = if vertical < 0 {
            self.scroll.saturating_sub(step)
        } else {
            self.scroll.saturating_add(step).min(self.max_scroll())
        };
    }

    pub fn zoom(&mut self, delta: i32) {
        #[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
        if matches!(self.mode, ContentMode::Preview)
            && let Preview::Image { image, .. } = &mut self.preview
        {
            image.zoom(delta);
        }
        let _ = delta;
    }

    pub fn fit(&mut self) {
        #[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
        if matches!(self.mode, ContentMode::Preview)
            && let Preview::Image { image, .. } = &mut self.preview
        {
            image.fit();
        }
    }

    pub fn home(&mut self) {
        self.scroll = 0;
        #[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
        if matches!(self.mode, ContentMode::Preview)
            && let Preview::Image { image, .. } = &mut self.preview
        {
            image.home();
        }
    }

    pub fn end(&mut self) {
        self.scroll = self.max_scroll();
        #[cfg(all(feature = "tui-images", not(target_arch = "wasm32")))]
        if matches!(self.mode, ContentMode::Preview)
            && let Preview::Image { image, .. } = &mut self.preview
        {
            image.end();
        }
    }

    fn max_scroll(&self) -> usize {
        self.source_lines.saturating_sub(self.page_height)
    }

    pub fn render(
        &mut self,
        frame: &mut Frame<'_>,
        time_remaining: Option<Duration>,
        show_image: bool,
    ) {
        if !show_image {
            self.suspend_image();
        }
        let Some(item) = self.items.get(self.selected) else {
            return;
        };
        let area = frame.area();
        frame.render_widget(Clear, area);
        let title = format!(
            "{} · {}/{} · {}",
            item.title,
            self.selected + 1,
            self.items.len(),
            item.content.kind()
        );
        let mut block = Block::bordered()
            .title(title)
            .border_style(Style::default().fg(Color::Cyan));
        if let Some(remaining) = time_remaining {
            block = block.title_bottom(countdown_label(remaining));
        }
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let help = wrap_preserving_spaces(&self.help, inner.width.max(1) as usize);
        let help_height = help.len().min(usize::from(inner.height / 3)).min(6) as u16;
        let chunks = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(help_height),
        ])
        .split(inner);
        self.page_height = chunks[1].height.max(1) as usize;
        let show_source = matches!(self.mode, ContentMode::Source)
            || (show_image && !self.preview.render_image(frame, chunks[1]));
        let (message, color) = if matches!(self.mode, ContentMode::Source) {
            (
                "Source · read-only; the original document is unchanged.".to_string(),
                Color::Cyan,
            )
        } else {
            self.preview.message()
        };
        if show_source {
            let source =
                wrap_preserving_spaces(item.content.source(), chunks[1].width.max(1) as usize);
            self.source_lines = source.len();
            self.scroll = self.scroll.min(self.max_scroll());
            let lines: Vec<_> = source
                .into_iter()
                .skip(self.scroll)
                .take(self.page_height)
                .map(Line::raw)
                .collect();
            frame.render_widget(Paragraph::new(lines), chunks[1]);
        }
        frame.render_widget(
            Paragraph::new(
                wrap_preserving_spaces(&message, chunks[0].width.max(1) as usize)
                    .into_iter()
                    .map(|text| Line::from(Span::styled(text, Style::default().fg(color))))
                    .collect::<Vec<_>>(),
            ),
            chunks[0],
        );
        frame.render_widget(
            Paragraph::new(help.into_iter().map(Line::raw).collect::<Vec<_>>())
                .style(Style::default().fg(Color::Gray)),
            chunks[2],
        );
    }
}
