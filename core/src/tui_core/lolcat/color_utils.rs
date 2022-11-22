/*
 *   Copyright (c) 2022 R3BL LLC
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

use crate::ColorWheelControl;

pub struct ColorUtils;

/* cSpell:disable */
impl ColorUtils {
    pub fn calc_fg_color(bg: (u8, u8, u8)) -> (u8, u8, u8) {
        // Currently, it only computes the foreground color based on some threshold
        // on grayscale value.
        // HACK: Add a better algorithm for computing foreground color.
        if ColorUtils::convert_grayscale(bg) > 0xA0_u8 {
            (0u8, 0u8, 0u8)
        } else {
            (0xffu8, 0xffu8, 0xffu8)
        }
    }

    pub fn linear_to_srgb(intensity: f64) -> f64 {
        if intensity <= 0.003_130_8 {
            12.92 * intensity
        } else {
            1.055 * intensity.powf(1.0 / 2.4) - 0.055
        }
    }

    pub fn srgb_to_linear(intensity: f64) -> f64 {
        if intensity < 0.04045 {
            intensity / 12.92
        } else {
            ((intensity + 0.055) / 1.055).powf(2.4)
        }
    }

    pub fn convert_grayscale(color: (u8, u8, u8)) -> u8 {
        // See https://en.wikipedia.org/wiki/Grayscale#Converting_color_to_grayscale
        const SCALE: f64 = 256.0;

        // Changing SRGB to Linear for gamma correction.
        let red = ColorUtils::srgb_to_linear(f64::from(color.0) / SCALE);
        let green = ColorUtils::srgb_to_linear(f64::from(color.1) / SCALE);
        let blue = ColorUtils::srgb_to_linear(f64::from(color.2) / SCALE);

        // Converting to grayscale.
        let gray_linear = red * 0.299 + green * 0.587 + blue * 0.114;

        // Gamma correction.
        let gray_srgb = ColorUtils::linear_to_srgb(gray_linear);

        (gray_srgb * SCALE) as u8
    }

    pub fn get_color_tuple(c: &ColorWheelControl) -> (u8, u8, u8) {
        let i = c.frequency * c.seed / c.spread;
        let red = i.sin() * 127.00 + 128.00;
        let green = (i + (std::f64::consts::PI * 2.00 / 3.00)).sin() * 127.00 + 128.00;
        let blue = (i + (std::f64::consts::PI * 4.00 / 3.00)).sin() * 127.00 + 128.00;

        (red as u8, green as u8, blue as u8)
    }
}
