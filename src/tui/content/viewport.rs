use ratatui::layout::Size;

use crate::tui::app::options::ImageCellSize;

#[derive(Clone, Copy, PartialEq)]
enum Zoom {
    Fit,
    Scale(f64),
}

#[derive(Clone, PartialEq)]
pub(super) struct ImageViewport {
    width: f64,
    height: f64,
    available: Size,
    cell_size: ImageCellSize,
    zoom: Zoom,
    center: (f64, f64),
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct ImageRegion {
    pub size: Size,
    pub scale: f64,
    pub x: f64,
    pub y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

pub(super) enum ViewportAction {
    Resize(Size, ImageCellSize),
    Pan(i32, i32, bool),
    Zoom(i32),
    Fit,
    Home,
    End,
}

impl ViewportAction {
    pub fn apply(self, viewport: &mut ImageViewport) {
        match self {
            Self::Resize(available, cell) => viewport.resize(available, cell),
            Self::Pan(horizontal, vertical, page) => viewport.pan(horizontal, vertical, page),
            Self::Zoom(delta) => viewport.zoom(delta),
            Self::Fit => viewport.fit(),
            Self::Home => viewport.home(),
            Self::End => viewport.end(),
        }
    }
}

impl ImageViewport {
    pub fn new(width: f32, height: f32, cell_size: ImageCellSize) -> Self {
        let (width, height) = (f64::from(width), f64::from(height));
        Self {
            width,
            height,
            available: Size::default(),
            cell_size,
            zoom: Zoom::Fit,
            center: (width / 2.0, height / 2.0),
        }
    }

    pub fn resize(&mut self, available: Size, cell_size: ImageCellSize) {
        if self.available != available || self.cell_size != cell_size {
            self.available = available;
            self.cell_size = cell_size;
            self.clamp_center();
        }
    }

    pub fn pan(&mut self, horizontal: i32, vertical: i32, page: bool) {
        let region = self.region();
        let rows = if page {
            self.available.height.saturating_sub(1).max(1)
        } else {
            1
        };
        self.center.0 += f64::from(horizontal) / region.scale;
        self.center.1 +=
            f64::from(vertical) * f64::from(rows) / (region.scale * self.cell_aspect());
        self.clamp_center();
    }

    pub fn zoom(&mut self, delta: i32) {
        let current = self.region().scale;
        let fit = self.fit_scale();
        let minimum = (fit / 16.0).min(0.0625).min(current);
        let maximum = (fit * 16.0).max(16.0).max(current);
        let scale = current * 2.0_f64.powf(f64::from(delta) / 2.0);
        self.zoom = Zoom::Scale(scale.clamp(minimum, maximum));
        self.clamp_center();
    }

    pub fn fit(&mut self) {
        self.zoom = Zoom::Fit;
        self.center = (self.width / 2.0, self.height / 2.0);
    }

    pub fn reduce_area(&mut self) -> bool {
        if self.available.width <= 1 && self.available.height <= 1 {
            return false;
        }
        let reduce = |length: u16| (f64::from(length) / 2.0_f64.sqrt()).floor().max(1.0) as u16;
        let available = Size::new(reduce(self.available.width), reduce(self.available.height));
        // Manual zoom keeps its scale; Fit still shows the whole image.
        self.resize(available, self.cell_size);
        true
    }

    pub fn available(&self) -> Size {
        self.available
    }

    pub fn home(&mut self) {
        self.center = (0.0, 0.0);
        self.clamp_center();
    }

    pub fn end(&mut self) {
        self.center = (self.width, self.height);
        self.clamp_center();
    }

    pub fn region(&self) -> ImageRegion {
        let scale = match self.zoom {
            Zoom::Fit => self.fit_scale(),
            Zoom::Scale(scale) => scale,
        };
        let [width, height] =
            [self.width * scale, self.height * scale * self.cell_aspect()].map(|length| {
                let rounded = length.round();
                // Round-trip zoom noise must not add a blank cell or a scrollbar.
                if (length - rounded).abs() <= f64::EPSILON * length * 4.0 {
                    rounded
                } else {
                    length
                }
            });
        let max_x = (width - f64::from(self.available.width)).max(0.0);
        let max_y = (height - f64::from(self.available.height)).max(0.0);
        ImageRegion {
            size: Size::new(
                width.ceil().min(f64::from(self.available.width)) as u16,
                height.ceil().min(f64::from(self.available.height)) as u16,
            ),
            scale,
            x: (self.center.0 * scale - f64::from(self.available.width) / 2.0).clamp(0.0, max_x),
            y: (self.center.1 * scale * self.cell_aspect()
                - f64::from(self.available.height) / 2.0)
                .clamp(0.0, max_y),
            max_x,
            max_y,
        }
    }

    pub fn description(&self) -> String {
        let region = self.region();
        let mode = if matches!(self.zoom, Zoom::Fit) {
            "fit"
        } else {
            "zoom"
        };
        format!(
            "{mode} {:.0}% · x {:.0}/{:.0} · y {:.0}/{:.0}",
            region.scale * 100.0,
            region.x,
            region.max_x,
            region.y,
            region.max_y,
        )
    }

    fn cell_aspect(&self) -> f64 {
        f64::from(self.cell_size.width()) / f64::from(self.cell_size.height())
    }

    fn fit_scale(&self) -> f64 {
        (f64::from(self.available.width.max(1)) / self.width)
            .min(f64::from(self.available.height.max(1)) / (self.height * self.cell_aspect()))
    }

    fn clamp_center(&mut self) {
        if self.available.width == 0 || self.available.height == 0 {
            return;
        }
        let scale = self.region().scale;
        let half_width = (f64::from(self.available.width) / scale / 2.0).min(self.width / 2.0);
        let half_height = (f64::from(self.available.height) / (scale * self.cell_aspect()) / 2.0)
            .min(self.height / 2.0);
        self.center.0 = self.center.0.clamp(half_width, self.width - half_width);
        self.center.1 = self.center.1.clamp(half_height, self.height - half_height);
    }
}
