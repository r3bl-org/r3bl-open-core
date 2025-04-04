/*
 *   Copyright (c) 2025 R3BL LLC
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

// 00: migrate this into tuify (to replace what's currently there)

use r3bl_core::{TuiStyle, new_style, tui_color};

/// This is not the same as [r3bl_core::TuiStylesheet], since this encapsulates
/// information that is specific to [crate::choose()] that are not generalized.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct StyleSheet {
    pub focused_and_selected_style: TuiStyle,
    pub focused_style: TuiStyle,
    pub unselected_style: TuiStyle,
    pub selected_style: TuiStyle,
    pub header_style: TuiStyle,
}

pub fn get_default_style() -> TuiStyle {
    new_style!(
        color_fg: {tui_color!(193, 193, 193)} color_bg: {tui_color!(14, 17, 23)}
    )
}

impl Default for StyleSheet {
    fn default() -> Self {
        let focused_and_selected_style = TuiStyle {
            color_fg: tui_color!(20, 244, 0).into(),
            color_bg: tui_color!(51, 32, 66).into(),
            ..TuiStyle::default()
        };
        let focused_style = TuiStyle {
            color_fg: tui_color!(20, 244, 0).into(),
            ..TuiStyle::default()
        };
        let unselected_style = TuiStyle {
            ..TuiStyle::default()
        };
        let selected_style = TuiStyle {
            color_fg: tui_color!(203, 170, 250).into(),
            color_bg: tui_color!(51, 32, 66).into(),
            ..TuiStyle::default()
        };
        let header_style = TuiStyle {
            color_fg: tui_color!(171, 204, 242).into(),
            color_bg: tui_color!(31, 36, 46).into(),
            ..TuiStyle::default()
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
        let focused_and_selected_style = TuiStyle {
            color_fg: tui_color!(19, 227, 255).into(),
            color_bg: tui_color!(6, 41, 52).into(),
            ..TuiStyle::default()
        };
        let focused_style = TuiStyle {
            color_fg: tui_color!(19, 227, 255).into(),
            color_bg: tui_color!(14, 17, 23).into(),
            ..TuiStyle::default()
        };
        let unselected_style = TuiStyle {
            color_fg: tui_color!(241, 241, 241).into(),
            color_bg: tui_color!(14, 17, 23).into(),
            ..TuiStyle::default()
        };
        let selected_style = TuiStyle {
            color_fg: tui_color!(209, 244, 255).into(),
            color_bg: tui_color!(6, 41, 52).into(),
            ..TuiStyle::default()
        };
        let header_style = TuiStyle {
            color_fg: tui_color!(229, 239, 123).into(),
            color_bg: tui_color!(31, 36, 46).into(),
            ..TuiStyle::default()
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
        let focused_and_selected_style = TuiStyle {
            color_fg: tui_color!(255, 0, 214).into(),
            color_bg: tui_color!(62, 14, 74).into(),
            ..TuiStyle::default()
        };
        let focused_style = TuiStyle {
            color_fg: tui_color!(255, 0, 214).into(),
            color_bg: tui_color!(14, 17, 23).into(),
            ..TuiStyle::default()
        };
        let unselected_style = TuiStyle {
            color_fg: tui_color!(219, 202, 232).into(),
            color_bg: tui_color!(14, 17, 23).into(),
            ..TuiStyle::default()
        };
        let selected_style = TuiStyle {
            color_fg: tui_color!(255, 181, 234).into(),
            color_bg: tui_color!(62, 14, 74).into(),
            ..TuiStyle::default()
        };
        let header_style = TuiStyle {
            color_fg: tui_color!(190, 253, 249).into(),
            color_bg: tui_color!(31, 36, 46).into(),
            ..TuiStyle::default()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_style() {
        let style = get_default_style();
        assert_eq!(style.color_fg, tui_color!(193, 193, 193).into());
        assert_eq!(style.color_bg, tui_color!(14, 17, 23).into());
    }

    #[test]
    fn test_default_theme() {
        let stylesheet = StyleSheet::default();

        assert_eq!(
            stylesheet.focused_and_selected_style.color_fg,
            tui_color!(20, 244, 0).into()
        );
        assert_eq!(
            stylesheet.focused_and_selected_style.color_bg,
            tui_color!(51, 32, 66).into()
        );

        assert_eq!(
            stylesheet.focused_style.color_fg,
            tui_color!(20, 244, 0).into()
        );
        assert_eq!(stylesheet.focused_style.color_bg, None);

        assert_eq!(stylesheet.unselected_style.color_fg, None);
        assert_eq!(stylesheet.unselected_style.color_bg, None);

        assert_eq!(
            stylesheet.selected_style.color_fg,
            tui_color!(203, 170, 250).into()
        );
        assert_eq!(
            stylesheet.selected_style.color_bg,
            tui_color!(51, 32, 66).into()
        );

        assert_eq!(
            stylesheet.header_style.color_fg,
            tui_color!(171, 204, 242).into()
        );
        assert_eq!(
            stylesheet.header_style.color_bg,
            tui_color!(31, 36, 46).into()
        );
    }

    #[test]
    fn test_sea_foam_theme() {
        let stylesheet = StyleSheet::sea_foam_style();

        assert_eq!(
            stylesheet.focused_and_selected_style.color_fg,
            tui_color!(19, 227, 255).into()
        );
        assert_eq!(
            stylesheet.focused_and_selected_style.color_bg,
            tui_color!(6, 41, 52).into()
        );

        assert_eq!(
            stylesheet.focused_style.color_fg,
            tui_color!(19, 227, 255).into()
        );
        assert_eq!(
            stylesheet.focused_style.color_bg,
            tui_color!(14, 17, 23).into()
        );

        assert_eq!(
            stylesheet.unselected_style.color_fg,
            tui_color!(241, 241, 241).into()
        );
        assert_eq!(
            stylesheet.unselected_style.color_bg,
            tui_color!(14, 17, 23).into()
        );

        assert_eq!(
            stylesheet.selected_style.color_fg,
            tui_color!(209, 244, 255).into()
        );
        assert_eq!(
            stylesheet.selected_style.color_bg,
            tui_color!(6, 41, 52).into()
        );

        assert_eq!(
            stylesheet.header_style.color_fg,
            tui_color!(229, 239, 123).into()
        );
        assert_eq!(
            stylesheet.header_style.color_bg,
            tui_color!(31, 36, 46).into()
        );
    }

    #[test]
    fn test_hot_pink_style() {
        let style_sheet = StyleSheet::hot_pink_style();

        assert_eq!(
            style_sheet.focused_and_selected_style.color_fg,
            tui_color!(255, 0, 214).into()
        );
        assert_eq!(
            style_sheet.focused_and_selected_style.color_bg,
            tui_color!(62, 14, 74).into()
        );
        assert_eq!(
            style_sheet.focused_style.color_fg,
            tui_color!(255, 0, 214).into()
        );
        assert_eq!(
            style_sheet.focused_style.color_bg,
            tui_color!(14, 17, 23).into()
        );
        assert_eq!(
            style_sheet.unselected_style.color_fg,
            tui_color!(219, 202, 232).into()
        );
        assert_eq!(
            style_sheet.unselected_style.color_bg,
            tui_color!(14, 17, 23).into()
        );
        assert_eq!(
            style_sheet.selected_style.color_fg,
            tui_color!(255, 181, 234).into()
        );
        assert_eq!(
            style_sheet.selected_style.color_bg,
            tui_color!(62, 14, 74).into()
        );
        assert_eq!(
            style_sheet.header_style.color_fg,
            tui_color!(190, 253, 249).into()
        );
        assert_eq!(
            style_sheet.header_style.color_bg,
            tui_color!(31, 36, 46).into()
        );
    }
}
