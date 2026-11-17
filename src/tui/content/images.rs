use std::sync::Arc;

use ratatui::{
    Frame,
    layout::{Constraint, Margin, Rect, Size},
    style::{Color, Style},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use ratatui_image::{Image, protocol::Protocol};

use crate::{
    tui::app::options::{ImageCellSize, ImageProtocol},
    ui_ast::RichContent,
};

use super::{
    GraphicsOutput, raster_budget,
    viewport::{ImageRegion, ImageViewport, ViewportAction},
};

mod raster;
mod worker;

use raster::RasterInput;
use worker::{ImageResult, ImageWork, ImageWorker, ImageWorkerClient};

const MAX_RASTER_EDGE: u32 = 8192;
const MAX_RASTER_PIXELS: u64 = 4_000_000;
const MAX_PENDING_VIEWPORT_ACTIONS: usize = 1024;
// ratatui-image 11.1 encodes Kitty placeholder coordinates with 297 diacritics.
const KITTY_VIEWPORT_EDGE: u16 = 297;

#[derive(Default)]
pub(super) struct ImageRenderer {
    worker: Option<Result<ImageWorker, String>>,
}

impl ImageRenderer {
    pub fn render(
        &mut self,
        content: &RichContent,
        output: GraphicsOutput,
    ) -> Result<ImagePreview, String> {
        let worker = self
            .worker
            .get_or_insert_with(ImageWorker::new)
            .as_ref()
            .map_err(Clone::clone)?
            .client();
        Ok(ImagePreview {
            content: Arc::new(content.clone()),
            viewport: None,
            geometry: None,
            deferred: Vec::new(),
            output,
            worker,
            target: None,
            state: ImageState::Empty,
        })
    }
}

#[derive(Clone, PartialEq)]
struct ImageTarget {
    viewport: ImageViewport,
    frame: Rect,
}

struct EncodedFrame {
    region: ImageRegion,
    protocol: Protocol,
}

enum ImageState {
    Empty,
    Preparing {
        id: u64,
    },
    Rendering {
        id: u64,
        previous: Option<EncodedFrame>,
    },
    Ready(EncodedFrame),
    Failed(String),
}

pub(super) struct ImagePreview {
    content: Arc<RichContent>,
    viewport: Option<ImageViewport>,
    geometry: Option<(Size, ImageCellSize)>,
    deferred: Vec<ViewportAction>,
    output: GraphicsOutput,
    worker: ImageWorkerClient,
    target: Option<ImageTarget>,
    state: ImageState,
}

impl Drop for ImagePreview {
    fn drop(&mut self) {
        self.suspend();
    }
}

impl ImagePreview {
    pub fn protocol(&self) -> ImageProtocol {
        self.output.protocol
    }

    pub fn pending(&self) -> bool {
        matches!(
            self.state,
            ImageState::Preparing { .. } | ImageState::Rendering { .. }
        )
    }

    pub fn description(&self) -> String {
        self.viewport
            .as_ref()
            .map_or_else(|| "fit".into(), ImageViewport::description)
    }

    pub fn scroll(&mut self, horizontal: i32, vertical: i32, page: bool) {
        self.apply(ViewportAction::Pan(horizontal, vertical, page));
    }

    pub fn zoom(&mut self, delta: i32) {
        self.apply(ViewportAction::Zoom(delta));
    }

    pub fn fit(&mut self) {
        self.apply(ViewportAction::Fit);
    }

    pub fn home(&mut self) {
        self.apply(ViewportAction::Home);
    }

    pub fn end(&mut self) {
        self.apply(ViewportAction::End);
    }

    fn apply(&mut self, action: ViewportAction) {
        if let Some(viewport) = &mut self.viewport {
            action.apply(viewport);
        } else if !matches!(self.state, ImageState::Failed(_)) {
            // Dimensions arrive before encoding. Until then, preserve order,
            // including resize: pan and zoom are not interchangeable.
            if self.deferred.len() < MAX_PENDING_VIEWPORT_ACTIONS {
                self.deferred.push(action);
            } else {
                self.suspend();
                self.deferred.clear();
                self.state = ImageState::Failed(
                    "Too many viewport actions before image dimensions were ready; reopen the preview"
                        .into(),
                );
            }
        }
    }

    pub fn suspend(&mut self) {
        match self.state {
            ImageState::Preparing { id } | ImageState::Rendering { id, .. } => {
                self.worker.cancel(id);
                let state = std::mem::replace(&mut self.state, ImageState::Empty);
                if let ImageState::Rendering {
                    previous: Some(previous),
                    ..
                } = state
                {
                    self.state = ImageState::Ready(previous);
                }
                self.target = None;
            }
            _ => {}
        }
    }

    pub fn invalidate(&mut self) {
        self.target = None;
        // Preparation has no terminal geometry or encoded placement to invalidate.
        if !matches!(self.state, ImageState::Preparing { .. }) {
            self.suspend();
            self.state = ImageState::Empty;
        }
    }

    pub fn render(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<(), String> {
        let geometry = self.output.cell_size().and_then(|cell| {
            self.bounded_viewport(area.inner(Margin::new(1, 1)), cell)
                .map(|viewport| (viewport, cell))
        });
        let (viewport, cell) = match geometry {
            Ok(geometry) => geometry,
            Err(error) => {
                self.suspend();
                return Err(error);
            }
        };
        if viewport.is_empty() {
            self.suspend();
            return Ok(());
        }
        let available = viewport.as_size();
        if self.geometry != Some((available, cell)) {
            self.apply(ViewportAction::Resize(available, cell));
            self.geometry = Some((available, cell));
        }
        if let Some(image) = &self.viewport
            && self
                .target
                .as_ref()
                .is_some_and(|target| target.viewport != *image || target.frame != frame.area())
        {
            self.suspend();
        }
        self.receive(available, cell);
        let viewport = if let Some(image) = &self.viewport {
            viewport.centered(
                Constraint::Length(image.available().width),
                Constraint::Length(image.available().height),
            )
        } else {
            viewport
        };
        if let Some(image) = &self.viewport {
            let target = ImageTarget {
                viewport: image.clone(),
                frame: frame.area(),
            };
            if self.target.as_ref() != Some(&target) {
                self.suspend();
                let previous = match std::mem::replace(&mut self.state, ImageState::Empty) {
                    ImageState::Ready(previous) => Some(previous),
                    _ => None,
                };
                let id = self.worker.submit(ImageWork::Render(RasterInput {
                    content: self.content.clone(),
                    viewport: target.viewport.clone(),
                    available,
                    cell,
                    output: self.output,
                }));
                self.target = Some(target);
                self.state = ImageState::Rendering { id, previous };
            }
        } else if matches!(self.state, ImageState::Empty) {
            let id = self.worker.submit(ImageWork::Prepare(self.content.clone()));
            self.state = ImageState::Preparing { id };
        }
        let encoded = match &self.state {
            ImageState::Ready(encoded) => Some(encoded),
            ImageState::Rendering { previous, .. } => previous.as_ref(),
            ImageState::Failed(error) => return Err(error.clone()),
            ImageState::Empty | ImageState::Preparing { .. } => None,
        };
        if let Some(encoded) = encoded {
            let image_area = viewport.centered(
                Constraint::Length(encoded.region.size.width),
                Constraint::Length(encoded.region.size.height),
            );
            self.output.arm_cleanup();
            frame.render_widget(Image::new(&encoded.protocol), image_area);
        }
        let Some(image) = &self.viewport else {
            return Ok(());
        };
        let region = image.region();
        for (maximum, position, visible, orientation) in [
            (
                region.max_x,
                region.x,
                viewport.width,
                ScrollbarOrientation::HorizontalBottom,
            ),
            (
                region.max_y,
                region.y,
                viewport.height,
                ScrollbarOrientation::VerticalRight,
            ),
        ] {
            if maximum > 0.0 {
                let mut state = ScrollbarState::new(maximum.ceil() as usize + 1)
                    .position(position.round() as usize)
                    .viewport_content_length(usize::from(visible));
                frame.render_stateful_widget(
                    Scrollbar::new(orientation)
                        .begin_symbol(None)
                        .end_symbol(None)
                        .thumb_style(Style::default().fg(Color::Cyan)),
                    area,
                    &mut state,
                );
            }
        }
        Ok(())
    }

    fn receive(&mut self, available: Size, cell: ImageCellSize) {
        let (ImageState::Preparing { id } | ImageState::Rendering { id, .. }) = self.state else {
            return;
        };
        let Some(result) = self.worker.take(id) else {
            return;
        };
        self.state = match result {
            Ok(ImageResult::Prepared(width, height)) => {
                let mut viewport = ImageViewport::new(width, height, cell);
                for action in self.deferred.drain(..) {
                    action.apply(&mut viewport);
                }
                viewport.resize(available, cell);
                self.viewport = Some(viewport);
                ImageState::Empty
            }
            Ok(ImageResult::Rendered(output)) => {
                if let Some(viewport) = &mut self.viewport {
                    viewport.resize(output.available, cell);
                    if let Some(target) = &mut self.target {
                        target.viewport = viewport.clone();
                    }
                }
                ImageState::Ready(EncodedFrame {
                    region: output.region,
                    protocol: output.protocol,
                })
            }
            Err(error) => ImageState::Failed(error),
        };
    }

    fn bounded_viewport(&self, area: Rect, cell: ImageCellSize) -> Result<Rect, String> {
        if area.is_empty() {
            return Ok(area);
        }
        let coordinate_limit = if self.output.protocol == ImageProtocol::Kitty {
            KITTY_VIEWPORT_EDGE
        } else {
            u16::MAX
        };
        let mut width = area
            .width
            .min(coordinate_limit)
            .min((MAX_RASTER_EDGE / u32::from(cell.width())) as u16);
        let mut height = area
            .height
            .min(coordinate_limit)
            .min((MAX_RASTER_EDGE / u32::from(cell.height())) as u16);
        let cell_pixels = u64::from(cell.width()) * u64::from(cell.height());
        let limit = MAX_RASTER_PIXELS.min(raster_budget::pixel_limit(
            self.output.protocol,
            self.output.is_tmux,
        ));
        if width == 0 || height == 0 || cell_pixels > limit {
            return Err("Terminal cell pixels exceed native preview limits; use source".into());
        }
        let pixels = u64::from(width) * u64::from(height) * cell_pixels;
        if pixels > limit {
            // Keep native pixel density; reduce the visible region, not image resolution.
            let ratio = (limit as f64 / pixels as f64).sqrt();
            width = (f64::from(width) * ratio).floor().max(1.0) as u16;
            height = (f64::from(height) * ratio).floor().max(1.0) as u16;
            let cells = limit / cell_pixels;
            width = u64::from(width).min(cells) as u16;
            height = u64::from(height).min(cells / u64::from(width)) as u16;
        }
        Ok(area.centered(Constraint::Length(width), Constraint::Length(height)))
    }
}
