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

use super::ColorWheelControl;

pub struct ColorUtils;

impl ColorUtils {
    /// More info on luminance:
    /// - <https://stackoverflow.com/a/49092130/2085356>
    /// - <https://stackoverflow.com/a/3118280/2085356>
    pub fn calc_fg_color(bg: (u8, u8, u8)) -> (u8, u8, u8) {
        let luminance =
            0.2126 * (bg.0 as f32) + 0.7152 * (bg.1 as f32) + 0.0722 * (bg.2 as f32);
        if luminance < 140.0 {
            (255, 255, 255)
        } else {
            (0, 0, 0)
        }
    }

    pub fn get_color_tuple(c: &ColorWheelControl) -> (u8, u8, u8) {
        let i = c.frequency * c.seed / c.spread;
        let red = i.sin() * 127.00 + 128.00;
        let green = (i + (std::f64::consts::PI * 2.00 / 3.00)).sin() * 127.00 + 128.00;
        let blue = (i + (std::f64::consts::PI * 4.00 / 3.00)).sin() * 127.00 + 128.00;

        (red as u8, green as u8, blue as u8)
    }
}
