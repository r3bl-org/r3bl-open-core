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

use r3bl_ansi_color::Color;

#[derive(Copy, Clone, Debug)]
pub struct StyleSheet {
    pub focused_and_selected_style: Style,
    pub focused_style: Style,
    pub unselected_style: Style,
    pub selected_style: Style,
    pub header_style: Style,
}

impl Default for StyleSheet {
    fn default() -> Self {
        let focused_and_selected_style = Style {
            fg_color: Color::Rgb(20, 244, 0),
            bg_color: Color::Rgb(51, 32, 66),
            ..Style::default()
        };
        let focused_style = Style {
            fg_color: Color::Rgb(20, 244, 0),
            ..Style::default()
        };
        let unselected_style = Style { ..Style::default() };
        let selected_style = Style {
            fg_color: Color::Rgb(203, 170, 250),
            bg_color: Color::Rgb(51, 32, 66),
            ..Style::default()
        };
        let header_style = Style {
            fg_color: Color::Rgb(171, 204, 242),
            bg_color: Color::Rgb(31, 36, 46),
            ..Style::default()
        };
        StyleSheet {
            focused_and_selected_style,
            focused_style,
            unselected_style,
            selected_style,
            header_style,
        }
    }
}

impl StyleSheet {
    pub fn sea_foam_style() -> Self {
        let focused_and_selected_style = Style {
            fg_color: Color::Rgb(19, 227, 255),
            bg_color: Color::Rgb(6, 41, 52),
            ..Style::default()
        };
        let focused_style = Style {
            fg_color: Color::Rgb(19, 227, 255),
            bg_color: Color::Rgb(14, 17, 23),
            ..Style::default()
        };
        let unselected_style = Style {
            fg_color: Color::Rgb(241, 241, 241),
            bg_color: Color::Rgb(14, 17, 23),
            ..Style::default()
        };
        let selected_style = Style {
            fg_color: Color::Rgb(209, 244, 255),
            bg_color: Color::Rgb(6, 41, 52),
            ..Style::default()
        };
        let header_style = Style {
            fg_color: Color::Rgb(229, 239, 123),
            bg_color: Color::Rgb(31, 36, 46),
            ..Style::default()
        };
        StyleSheet {
            focused_and_selected_style,
            focused_style,
            unselected_style,
            selected_style,
            header_style,
        }
    }

    pub fn hot_pink_style() -> Self {
        let focused_and_selected_style = Style {
            fg_color: Color::Rgb(255, 0, 214),
            bg_color: Color::Rgb(62, 14, 74),
            ..Style::default()
        };
        let focused_style = Style {
            fg_color: Color::Rgb(255, 0, 214),
            bg_color: Color::Rgb(14, 17, 23),
            ..Style::default()
        };
        let unselected_style = Style {
            fg_color: Color::Rgb(219, 202, 232),
            bg_color: Color::Rgb(14, 17, 23),
            ..Style::default()
        };
        let selected_style = Style {
            fg_color: Color::Rgb(255, 181, 234),
            bg_color: Color::Rgb(62, 14, 74),
            ..Style::default()
        };
        let header_style = Style {
            fg_color: Color::Rgb(190, 253, 249),
            bg_color: Color::Rgb(31, 36, 46),
            ..Style::default()
        };
        StyleSheet {
            focused_and_selected_style,
            focused_style,
            unselected_style,
            selected_style,
            header_style,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub dim: bool,
    pub underline: bool,
    pub reverse: bool,
    pub hidden: bool,
    pub strikethrough: bool,
    pub fg_color: Color,
    pub bg_color: Color,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            bold: false,
            italic: false,
            dim: false,
            underline: false,
            reverse: false,
            hidden: false,
            strikethrough: false,
            fg_color: Color::Rgb(193, 193, 193),
            bg_color: Color::Rgb(14, 17, 23),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_theme() {
        let stylesheet = StyleSheet::default();

        assert_eq!(
            stylesheet.focused_and_selected_style.fg_color,
            Color::Rgb(20, 244, 0)
        );
        assert_eq!(
            stylesheet.focused_and_selected_style.bg_color,
            Color::Rgb(51, 32, 66)
        );

        assert_eq!(stylesheet.focused_style.fg_color, Color::Rgb(20, 244, 0));

        assert_eq!(
            stylesheet.unselected_style.fg_color,
            Color::Rgb(193, 193, 193)
        );
        assert_eq!(stylesheet.unselected_style.bg_color, Color::Rgb(14, 17, 23));

        assert_eq!(
            stylesheet.selected_style.fg_color,
            Color::Rgb(203, 170, 250)
        );
        assert_eq!(stylesheet.selected_style.bg_color, Color::Rgb(51, 32, 66));

        assert_eq!(stylesheet.header_style.fg_color, Color::Rgb(171, 204, 242));
        assert_eq!(stylesheet.header_style.bg_color, Color::Rgb(31, 36, 46));
    }

    #[test]
    fn test_sea_foam_theme() {
        let stylesheet = StyleSheet::sea_foam_style();

        assert_eq!(
            stylesheet.focused_and_selected_style.fg_color,
            Color::Rgb(19, 227, 255)
        );
        assert_eq!(
            stylesheet.focused_and_selected_style.bg_color,
            Color::Rgb(6, 41, 52)
        );

        assert_eq!(stylesheet.focused_style.fg_color, Color::Rgb(19, 227, 255));
        assert_eq!(stylesheet.focused_style.bg_color, Color::Rgb(14, 17, 23));

        assert_eq!(
            stylesheet.unselected_style.fg_color,
            Color::Rgb(241, 241, 241)
        );
        assert_eq!(stylesheet.unselected_style.bg_color, Color::Rgb(14, 17, 23));

        assert_eq!(
            stylesheet.selected_style.fg_color,
            Color::Rgb(209, 244, 255)
        );
        assert_eq!(stylesheet.selected_style.bg_color, Color::Rgb(6, 41, 52));

        assert_eq!(stylesheet.header_style.fg_color, Color::Rgb(229, 239, 123));
        assert_eq!(stylesheet.header_style.bg_color, Color::Rgb(31, 36, 46));
    }

    #[test]
    fn test_hot_pink_style() {
        let style_sheet = StyleSheet::hot_pink_style();

        assert_eq!(
            style_sheet.focused_and_selected_style.fg_color,
            Color::Rgb(255, 0, 214)
        );
        assert_eq!(
            style_sheet.focused_and_selected_style.bg_color,
            Color::Rgb(62, 14, 74)
        );
        assert_eq!(style_sheet.focused_style.fg_color, Color::Rgb(255, 0, 214));
        assert_eq!(style_sheet.focused_style.bg_color, Color::Rgb(14, 17, 23));
        assert_eq!(
            style_sheet.unselected_style.fg_color,
            Color::Rgb(219, 202, 232)
        );
        assert_eq!(
            style_sheet.unselected_style.bg_color,
            Color::Rgb(14, 17, 23)
        );
        assert_eq!(
            style_sheet.selected_style.fg_color,
            Color::Rgb(255, 181, 234)
        );
        assert_eq!(style_sheet.selected_style.bg_color, Color::Rgb(62, 14, 74));
        assert_eq!(style_sheet.header_style.fg_color, Color::Rgb(190, 253, 249));
        assert_eq!(style_sheet.header_style.bg_color, Color::Rgb(31, 36, 46));
    }
}
