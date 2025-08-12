// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{new_style, tui_color, TuiStyle};

/// This is different from [`crate::TuiStylesheet`], since this encapsulates styling
/// information that is specific to [`crate::choose()`] that are not generalized.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct StyleSheet {
    pub focused_and_selected_style: TuiStyle,
    pub focused_style: TuiStyle,
    pub unselected_style: TuiStyle,
    pub selected_style: TuiStyle,
    pub header_style: TuiStyle,
}

#[must_use]
pub fn get_default_style() -> TuiStyle {
    new_style!(
        color_fg: {tui_color!(medium_gray)} color_bg: {tui_color!(night_blue)}
    )
}

impl Default for StyleSheet {
    fn default() -> Self {
        let focused_and_selected_style = TuiStyle {
            color_fg: tui_color!(lizard_green).into(),
            color_bg: tui_color!(dark_purple).into(),
            ..TuiStyle::default()
        };
        let focused_style = TuiStyle {
            color_fg: tui_color!(lizard_green).into(),
            ..TuiStyle::default()
        };
        let unselected_style = TuiStyle {
            ..TuiStyle::default()
        };
        let selected_style = TuiStyle {
            color_fg: tui_color!(lavender).into(),
            color_bg: tui_color!(dark_purple).into(),
            ..TuiStyle::default()
        };
        let header_style = TuiStyle {
            color_fg: tui_color!(frozen_blue).into(),
            color_bg: tui_color!(moonlight_blue).into(),
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
    #[must_use]
    pub fn sea_foam_style() -> Self {
        let focused_and_selected_style = TuiStyle {
            color_fg: tui_color!(bright_cyan).into(),
            color_bg: tui_color!(dark_teal).into(),
            ..TuiStyle::default()
        };
        let focused_style = TuiStyle {
            color_fg: tui_color!(bright_cyan).into(),
            color_bg: tui_color!(night_blue).into(),
            ..TuiStyle::default()
        };
        let unselected_style = TuiStyle {
            color_fg: tui_color!(light_gray).into(),
            color_bg: tui_color!(night_blue).into(),
            ..TuiStyle::default()
        };
        let selected_style = TuiStyle {
            color_fg: tui_color!(light_cyan).into(),
            color_bg: tui_color!(dark_teal).into(),
            ..TuiStyle::default()
        };
        let header_style = TuiStyle {
            color_fg: tui_color!(light_yellow_green).into(),
            color_bg: tui_color!(moonlight_blue).into(),
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

    #[must_use]
    pub fn hot_pink_style() -> Self {
        let focused_and_selected_style = TuiStyle {
            color_fg: tui_color!(hot_pink).into(),
            color_bg: tui_color!(deep_purple).into(),
            ..TuiStyle::default()
        };
        let focused_style = TuiStyle {
            color_fg: tui_color!(hot_pink).into(),
            color_bg: tui_color!(night_blue).into(),
            ..TuiStyle::default()
        };
        let unselected_style = TuiStyle {
            color_fg: tui_color!(light_purple).into(),
            color_bg: tui_color!(night_blue).into(),
            ..TuiStyle::default()
        };
        let selected_style = TuiStyle {
            color_fg: tui_color!(soft_pink).into(),
            color_bg: tui_color!(deep_purple).into(),
            ..TuiStyle::default()
        };
        let header_style = TuiStyle {
            color_fg: tui_color!(light_cyan).into(),
            color_bg: tui_color!(moonlight_blue).into(),
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
        assert_eq!(style.color_fg, tui_color!(medium_gray).into());
        assert_eq!(style.color_bg, tui_color!(night_blue).into());
    }

    #[test]
    fn test_default_theme() {
        let stylesheet = StyleSheet::default();

        assert_eq!(
            stylesheet.focused_and_selected_style.color_fg,
            tui_color!(lizard_green).into()
        );
        assert_eq!(
            stylesheet.focused_and_selected_style.color_bg,
            tui_color!(dark_purple).into()
        );

        assert_eq!(
            stylesheet.focused_style.color_fg,
            tui_color!(lizard_green).into()
        );
        assert_eq!(stylesheet.focused_style.color_bg, None);

        assert_eq!(stylesheet.unselected_style.color_fg, None);
        assert_eq!(stylesheet.unselected_style.color_bg, None);

        assert_eq!(
            stylesheet.selected_style.color_fg,
            tui_color!(lavender).into()
        );
        assert_eq!(
            stylesheet.selected_style.color_bg,
            tui_color!(dark_purple).into()
        );

        assert_eq!(
            stylesheet.header_style.color_fg,
            tui_color!(frozen_blue).into()
        );
        assert_eq!(
            stylesheet.header_style.color_bg,
            tui_color!(moonlight_blue).into()
        );
    }

    #[test]
    fn test_sea_foam_theme() {
        let stylesheet = StyleSheet::sea_foam_style();

        assert_eq!(
            stylesheet.focused_and_selected_style.color_fg,
            tui_color!(bright_cyan).into()
        );
        assert_eq!(
            stylesheet.focused_and_selected_style.color_bg,
            tui_color!(dark_teal).into()
        );

        assert_eq!(
            stylesheet.focused_style.color_fg,
            tui_color!(bright_cyan).into()
        );
        assert_eq!(
            stylesheet.focused_style.color_bg,
            tui_color!(night_blue).into()
        );

        assert_eq!(
            stylesheet.unselected_style.color_fg,
            tui_color!(light_gray).into()
        );
        assert_eq!(
            stylesheet.unselected_style.color_bg,
            tui_color!(night_blue).into()
        );

        assert_eq!(
            stylesheet.selected_style.color_fg,
            tui_color!(light_cyan).into()
        );
        assert_eq!(
            stylesheet.selected_style.color_bg,
            tui_color!(dark_teal).into()
        );

        assert_eq!(
            stylesheet.header_style.color_fg,
            tui_color!(light_yellow_green).into()
        );
        assert_eq!(
            stylesheet.header_style.color_bg,
            tui_color!(moonlight_blue).into()
        );
    }

    #[test]
    fn test_hot_pink_style() {
        let style_sheet = StyleSheet::hot_pink_style();

        assert_eq!(
            style_sheet.focused_and_selected_style.color_fg,
            tui_color!(hot_pink).into()
        );
        assert_eq!(
            style_sheet.focused_and_selected_style.color_bg,
            tui_color!(deep_purple).into()
        );
        assert_eq!(
            style_sheet.focused_style.color_fg,
            tui_color!(hot_pink).into()
        );
        assert_eq!(
            style_sheet.focused_style.color_bg,
            tui_color!(night_blue).into()
        );
        assert_eq!(
            style_sheet.unselected_style.color_fg,
            tui_color!(light_purple).into()
        );
        assert_eq!(
            style_sheet.unselected_style.color_bg,
            tui_color!(night_blue).into()
        );
        assert_eq!(
            style_sheet.selected_style.color_fg,
            tui_color!(soft_pink).into()
        );
        assert_eq!(
            style_sheet.selected_style.color_bg,
            tui_color!(deep_purple).into()
        );
        assert_eq!(
            style_sheet.header_style.color_fg,
            tui_color!(light_cyan).into()
        );
        assert_eq!(
            style_sheet.header_style.color_bg,
            tui_color!(moonlight_blue).into()
        );
    }
}
