/*
 *   Copyright (c) 2023 R3BL LLC
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
use r3bl_rs_utils_core::*;

#[derive(Copy, Clone, Debug)]
pub struct StyleSheet {
    pub normal_style: Style,
    pub selected_style: Style,
    pub header_style: Style,
}

impl Default for StyleSheet {
    fn default() -> Self {
        let normal_style = Style {
            color_fg: Some(TuiColor::Rgb(RgbValue {
                red: 200, green: 200, blue:1
            })),
            color_bg: Some(TuiColor::Rgb(RgbValue {
                red: 100, green: 60, blue: 150
            })),
            ..Style::default()

        };
        let selected_style = Style {
            color_fg: Some(TuiColor::Rgb(RgbValue {
                red:250, green:250, blue:250
            })),
            color_bg: Some(TuiColor::Rgb(RgbValue {
                red:39, green:45, blue:239
            })),
            ..Style::default()
        };
        let header_style = Style {
            color_fg: Some(TuiColor::Rgb(RgbValue {
                red:50, green:50, blue:50
            })),
            color_bg: Some(TuiColor::Rgb(RgbValue {
                red:150, green:150, blue:150
            })),
            bold: true,
            ..Style::default()
        };
        StyleSheet {
            normal_style,
            selected_style,
            header_style,
        }
    }
}
