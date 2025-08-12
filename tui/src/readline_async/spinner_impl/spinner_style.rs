// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use smallvec::smallvec;

use crate::{ColorWheel, ColorWheelConfig, ColorWheelSpeed};

#[derive(Debug, Clone, Copy)]
pub enum SpinnerTemplate {
    Braille,
    Block,
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum SpinnerColor {
    None,
    ColorWheel(ColorWheel),
}

impl SpinnerColor {
    /// Gradients: <https://uigradients.com/#JShine>
    #[must_use]
    pub fn default_color_wheel() -> SpinnerColor {
        let color_wheel_config = ColorWheelConfig::Rgb(
            // Stops.
            smallvec!["#12c2e9".into(), "#c471ed".into(), "#f64f59".into()],
            // Speed.
            ColorWheelSpeed::Fast,
            // Steps.
            10,
        );
        let mut it = ColorWheel::new(smallvec![color_wheel_config]);
        it.generate_color_wheel(None);
        SpinnerColor::ColorWheel(it)
    }
}

#[derive(Debug, Clone)]
pub struct SpinnerStyle {
    pub template: SpinnerTemplate,
    pub color: SpinnerColor,
}

impl Default for SpinnerStyle {
    fn default() -> Self {
        SpinnerStyle {
            template: SpinnerTemplate::Braille,
            color: SpinnerColor::default_color_wheel(),
        }
    }
}
