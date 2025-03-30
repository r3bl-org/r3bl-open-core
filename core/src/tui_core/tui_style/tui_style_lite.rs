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

/// Macro to create a new `TuiStyle` with the given properties. And return it.
///
/// - Note that all the symbols that are values must be passed in enclosing `{` and `}`.
/// - Commas are not used to separate the tokens in the macro.
/// - All the attributes are simply symbols like `bold`, `italic`, `dim`, `underline`,
///   `reverse`, that correspond to the field names of [crate::TuiStyle].
/// - If you set the `computed` attribute, then it will set the `id` to `u8::MAX`. This is
///   what [crate::TuiStyle] does.
///
/// Example:
///
/// ```ignore
/// use r3bl_tui_core::{TuiColor, TuiStyle, RgbValue};
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
        $style.bold = true;
        $crate::apply_style!($style, $($rem)*);
    }};
    ($style:ident, italic $($rem:tt)*) => {{
        $style.italic = true;
        $crate::apply_style!($style, $($rem)*);
    }};
    ($style:ident, dim $($rem:tt)*) => {{
        $style.dim = true;
        $crate::apply_style!($style, $($rem)*);
    }};
    ($style:ident, underline $($rem:tt)*) => {{
        $style.underline = true;
        $crate::apply_style!($style, $($rem)*);
    }};
    ($style:ident, reverse $($rem:tt)*) => {{
        $style.reverse = true;
        $crate::apply_style!($style, $($rem)*);
    }};
    ($style:ident, hidden $($rem:tt)*) => {{
        $style.hidden = true;
        $crate::apply_style!($style, $($rem)*);
    }};
    ($style:ident, strikethrough $($rem:tt)*) => {{
        $style.strikethrough = true;
        $crate::apply_style!($style, $($rem)*);
    }};
    ($style:ident, lolcat $($rem:tt)*) => {{
        $style.lolcat = true;
        $crate::apply_style!($style, $($rem)*);
    }};
    // Computed.
    ($style:ident, computed $($rem:tt)*) => {{
        $style.computed = true;
        $style.id = u8::MAX;
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
        $style.id = $id as u8;
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
        assert!(s.bold);
        assert!(s.italic);
    }

    #[test]
    fn test_apply_style_multiple_attributes() {
        let mut s = TuiStyle::default();
        apply_style!(s, bold italic lolcat);
        assert!(s.bold);
        assert!(s.italic);
        assert!(s.lolcat);
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
        assert!(s.bold);
        assert!(s.italic);
        assert_eq!(s.color_fg, Some(BLACK));
        assert_eq!(s.color_bg, Some(BLACK));
    }

    #[test]
    fn test_apply_style_bold_italic_color_fg_padding() {
        let mut s = TuiStyle::default();
        apply_style!(s, bold italic color_fg: {BLACK} padding: {2});
        assert!(s.bold);
        assert!(s.italic);
        assert_eq!(s.color_fg, Some(BLACK));
        assert_eq!(s.padding, Some(ch(2)));
    }

    #[test]
    fn test_apply_style_bold_italic_id_color_fg_color_bg() {
        let mut s = TuiStyle::default();
        apply_style!(s, bold italic id: {100} color_fg: {BLACK} color_bg: {BLACK});
        assert_eq!(s.id, 100);
        assert!(s.bold);
        assert!(s.italic);
        assert_eq!(s.color_fg, Some(BLACK));
        assert_eq!(s.padding, None);
        assert_eq!(s.color_bg, Some(BLACK));
    }

    #[test]
    fn test_computed() {
        let mut s = TuiStyle::default();
        apply_style!(s, computed);
        assert_eq!(s.id, u8::MAX);
        assert!(s.computed);
    }
}
