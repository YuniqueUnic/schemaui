use super::{FieldKind, FieldSchema};

#[derive(Debug, Clone, Copy)]
pub(crate) struct NumericScale {
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

impl NumericScale {
    pub fn from_schema(schema: &FieldSchema) -> Option<Self> {
        let bounds = schema.presentation.bounds.as_ref()?;
        let min = bounds.minimum?;
        let max = bounds.maximum?;
        if !min.is_finite() || !max.is_finite() || max <= min || !(max - min).is_finite() {
            return None;
        }
        let kind = match schema.kind.non_nullable() {
            FieldKind::Array(inner) => inner.non_nullable(),
            kind => kind,
        };
        let integer = matches!(kind, FieldKind::Integer);
        let step = match bounds.step {
            Some(step) if step.is_finite() && step > 0.0 => step,
            Some(_) => return None,
            None if integer => 1.0,
            None => {
                let raw = (max - min) / 100.0;
                let magnitude = 10_f64.powf(raw.log10().floor());
                (raw / magnitude).round() * magnitude
            }
        };
        if !step.is_finite() || step <= 0.0 || (integer && step.fract() != 0.0) {
            return None;
        }
        let scale = Self { min, max, step };
        let first = scale.first_stop();
        let last = scale.last_stop();
        (first.is_finite() && last.is_finite() && first <= last).then_some(scale)
    }

    pub fn first_stop(self) -> f64 {
        snap_ratio(self.min / self.step).ceil() * self.step
    }

    pub fn last_stop(self) -> f64 {
        snap_ratio(self.max / self.step).floor() * self.step
    }

    pub fn adjust(self, current: Option<f64>, direction: i32, fast: bool) -> f64 {
        let Some(current) = current.filter(|value| value.is_finite()) else {
            return self.first_stop();
        };
        let ratio = current / self.step;
        let nearest = ratio.round();
        let aligned = (ratio - nearest).abs() <= f64::EPSILON * ratio.abs().max(1.0) * 8.0;
        let next = if aligned {
            nearest + f64::from(direction) * if fast { 10.0 } else { 1.0 }
        } else if direction < 0 {
            ratio.floor()
        } else {
            ratio.ceil()
        };
        let value = (next * self.step).clamp(self.first_stop(), self.last_stop());
        let precision =
            (12.0 - value.abs().max(self.step).log10().floor()).clamp(0.0, 324.0) as usize;
        format!("{value:.precision$}").parse().unwrap_or(value)
    }
}

fn snap_ratio(ratio: f64) -> f64 {
    let nearest = ratio.round();
    if (ratio - nearest).abs() <= f64::EPSILON * ratio.abs().max(1.0) * 8.0 {
        nearest
    } else {
        ratio
    }
}
