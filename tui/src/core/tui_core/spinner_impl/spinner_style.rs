/*
 *   Copyright (c) 2024-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

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
