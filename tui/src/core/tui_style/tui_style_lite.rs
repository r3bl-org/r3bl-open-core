// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Macro to create a new `TuiStyle` with the given properties. And return it.
///
/// - Note that all the symbols that are values must be passed in enclosing `{` and `}`.
/// - Commas are not used to separate the tokens in the macro.
/// - All the attributes are simply symbols like `bold`, `italic`, `dim`, `underline`,
///   `reverse`, that correspond to the field names of [`crate::TuiStyle`].
/// - If you set the `computed` attribute, then it will set the `id` to `u8::MAX`. This is
///   what [`crate::TuiStyle`] does.
///
/// Example:
///
/// ```no_run
/// use r3bl_tui::{TuiColor, TuiStyle, RgbValue, new_style};
/// let color_bg = TuiColor::Rgb(RgbValue::from_hex("#076DEB"));
/// let color_fg = TuiColor::Rgb(RgbValue::from_hex("#E9C940"));
/// let style = new_style!(
///     id:{1} bold dim color_fg: {color_fg} color_bg: {color_bg}
/// );
/// ```
#[macro_export]
macro_rules! new_style {
    ($($rem:tt)*) => {{
        #[allow(unused_mut)]
        let mut style = $crate::TuiStyle::default();
        $crate::apply_style!(style, $($rem)*);
        style
    }};
}

#[macro_export]
macro_rules! apply_style {
    // Attrib.
    ($style:ident, bold $($rem:tt)*) => {{
        $style.bold = Some($crate::tui_style_attrib::Bold);
        $crate::apply_style!($style, $($rem)*);
    }};
    ($style:ident, italic $($rem:tt)*) => {{
        $style.italic = Some($crate::tui_style_attrib::Italic);
        $crate::apply_style!($style, $($rem)*);
    }};
    ($style:ident, dim $($rem:tt)*) => {{
        $style.dim = Some($crate::tui_style_attrib::Dim);
        $crate::apply_style!($style, $($rem)*);
    }};
    ($style:ident, underline $($rem:tt)*) => {{
        $style.underline = Some($crate::tui_style_attrib::Underline);
        $crate::apply_style!($style, $($rem)*);
    }};
    ($style:ident, reverse $($rem:tt)*) => {{
        $style.reverse = Some($crate::tui_style_attrib::Reverse);
        $crate::apply_style!($style, $($rem)*);
    }};
    ($style:ident, hidden $($rem:tt)*) => {{
        $style.hidden = Some($crate::tui_style_attrib::Hidden);
        $crate::apply_style!($style, $($rem)*);
    }};
    ($style:ident, strikethrough $($rem:tt)*) => {{
        $style.strikethrough = Some($crate::tui_style_attrib::Strikethrough);
        $crate::apply_style!($style, $($rem)*);
    }};
    ($style:ident, lolcat $($rem:tt)*) => {{
        $style.lolcat = Some($crate::tui_style_attrib::Lolcat);
        $crate::apply_style!($style, $($rem)*);
    }};
    // Computed.
    ($style:ident, computed $($rem:tt)*) => {{
        $style.computed = Some($crate::tui_style_attrib::Computed);
        $style.id = None;
        $crate::apply_style!($style, $($rem)*);
    }};
    // Color fg.
    ($style:ident, color_fg: $color:block $($rem:tt)*) => {{
        $style.color_fg = Some($color);
        $crate::apply_style!($style, $($rem)*);
    }};
    // Color bg.
    ($style:ident, color_bg: $color:block $($rem:tt)*) => {{
        $style.color_bg = Some($color);
        $crate::apply_style!($style, $($rem)*);
    }};
    // Padding.
    ($style:ident, padding: $padding:block $($rem:tt)*) => {{
        $style.padding = Some($crate::ChUnit::from($padding));
        $crate::apply_style!($style, $($rem)*);
    }};
    // Id.
    ($style:ident, id: $id:block $($rem:tt)*) => {{
        $style.id = $crate::tui_style_attrib::id($id);
        $crate::apply_style!($style, $($rem)*);
    }};
    // Base case: do nothing if no tokens are left.
    ($style:ident,) => {};
}

#[cfg(test)]
mod tests {
    use crate::{TuiStyle, ch, tui_color};
    const BLACK: crate::TuiColor = tui_color!(black);

    #[test]
    fn test_syntax_bold_italic() {
        let s = new_style!(bold italic);
        assert!(s.bold.is_some());
        assert!(s.italic.is_some());
    }

    #[test]
    fn test_apply_style_multiple_attributes() {
        let mut s = TuiStyle::default();
        apply_style!(s, bold italic lolcat);
        assert!(s.bold.is_some());
        assert!(s.italic.is_some());
        assert!(s.lolcat.is_some());
    }

    #[test]
    fn test_apply_style_color_fg() {
        let mut s = TuiStyle::default();
        apply_style!(s, color_fg: {BLACK});
        assert_eq!(s.color_fg, Some(BLACK));
    }

    #[test]
    fn test_apply_style_color_bg() {
        let mut s = TuiStyle::default();
        apply_style!(s, color_bg: {BLACK});
        assert_eq!(s.color_bg, Some(BLACK));
    }

    #[test]
    fn test_apply_style_bold_italic_color_fg_color_bg() {
        let mut s = TuiStyle::default();
        apply_style!(s, bold italic color_fg: {BLACK} color_bg: {BLACK});
        assert!(s.bold.is_some());
        assert!(s.italic.is_some());
        assert_eq!(s.color_fg, Some(BLACK));
        assert_eq!(s.color_bg, Some(BLACK));
    }

    #[test]
    fn test_apply_style_bold_italic_color_fg_padding() {
        let mut s = TuiStyle::default();
        apply_style!(s, bold italic color_fg: {BLACK} padding: {2});
        assert!(s.bold.is_some());
        assert!(s.italic.is_some());
        assert_eq!(s.color_fg, Some(BLACK));
        assert_eq!(s.padding, Some(ch(2)));
    }

    #[test]
    fn test_apply_style_bold_italic_id_color_fg_color_bg() {
        let mut s = TuiStyle::default();
        apply_style!(s, bold italic id: {100} color_fg: {BLACK} color_bg: {BLACK});
        assert_eq!(s.id, Some(crate::tui_style_attrib::Id(100)));
        assert!(s.bold.is_some());
        assert!(s.italic.is_some());
        assert_eq!(s.color_fg, Some(BLACK));
        assert_eq!(s.padding, None);
        assert_eq!(s.color_bg, Some(BLACK));
    }

    #[test]
    fn test_computed() {
        let mut s = TuiStyle::default();
        apply_style!(s, computed);
        assert!(s.id.is_none());
        assert!(s.computed.is_some());
    }
}
