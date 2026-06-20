// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`SGR`] (Select Graphic Rendition) operations for [`VT-100`]/[`ANSI`] terminal
//! emulation.
//!
//! This module implements [`SGR`] operations that correspond to [`ANSI`] [`SGR`]
//! sequences handled by the [`vt_100_pty_output_parser::ops::sgr_ops`] module. These
//! include:
//!
//! - **[`SGR`] 0** (Reset) - [`reset_all_style_attributes()`]
//! - **[`SGR`] 1** (Bold) - [`apply_style_attribute()`]
//! - **[`SGR`] 2** (Dim) - [`apply_style_attribute()`]
//! - **[`SGR`] 3** (Italic) - [`apply_style_attribute()`]
//! - **[`SGR`] 4** (Underline) - [`apply_style_attribute()`]
//! - **[`SGR`] 5/6** (Blink) - [`apply_style_attribute()`]
//! - **[`SGR`] 7** (Reverse) - [`apply_style_attribute()`]
//! - **[`SGR`] 8** (Hidden) - [`apply_style_attribute()`]
//! - **[`SGR`] 9** (Strikethrough) - [`apply_style_attribute()`]
//! - **[`SGR`] 30-37** (Foreground colors) - [`set_foreground_color()`]
//! - **[`SGR`] 40-47** (Background colors) - [`set_background_color()`]
//! - **[`SGR`] 90-97** (Bright foreground) - [`set_foreground_color()`]
//! - **[`SGR`] 100-107** (Bright background) - [`set_background_color()`]
//!
//! All operations maintain [`VT-100`] compliance and handle proper style state management for
//! terminal text rendering.
//!
//! This module implements the business logic for [`SGR`] operations delegated from the
//! parser shim. The `impl_` prefix follows our naming convention for searchable code
//! organization. See the architecture documentation above for the complete three-layer
//! architecture.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`apply_style_attribute()`]: crate::OfsBufVT100::apply_style_attribute
//! [`reset_all_style_attributes()`]: crate::OfsBufVT100::reset_all_style_attributes
//! [`set_background_color()`]: crate::OfsBufVT100::set_background_color
//! [`set_foreground_color()`]: crate::OfsBufVT100::set_foreground_color
//! [`SGR`]: crate::SgrCode
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`vt_100_pty_output_parser::ops::sgr_ops`]:
//!     crate::core::ansi::vt_100_pty_output_parser::ops::vt_100_shim_sgr_ops

use crate::{AnsiValue, ColorTarget, OfsBufVT100, RgbValue, SgrColorSequence, TuiColor,
            TuiStyle, TuiStyleAttribs};

impl OfsBufVT100 {
    /// Reset all [`SGR`] attributes to default state.
    ///
    /// [`SGR`]: crate::SgrCode
    pub fn reset_all_style_attributes(&mut self) {
        self.parser_global_state.current_style = TuiStyle::default();
    }

    /// Apply style attributes to the current style by merging with new attributes.
    pub fn apply_style_attribute(&mut self, attribs: TuiStyleAttribs) {
        self.parser_global_state.current_style.attribs =
            self.parser_global_state.current_style.attribs + attribs;
    }

    /// Reset specific style attributes. Only fields that are [`Some`] in the input are
    /// reset.
    pub fn reset_style_attribute(&mut self, attribs: TuiStyleAttribs) {
        let style = &mut self.parser_global_state.current_style;

        if attribs.bold.is_some() || attribs.dim.is_some() {
            style.attribs.bold = None;
            style.attribs.dim = None; // Bold and dim are mutually exclusive
        }
        if attribs.italic.is_some() {
            style.attribs.italic = None;
        }
        if attribs.underline.is_some() {
            style.attribs.underline = None;
        }
        if attribs.blink.is_some() {
            style.attribs.blink = None;
        }
        if attribs.reverse.is_some() {
            style.attribs.reverse = None;
        }
        if attribs.hidden.is_some() {
            style.attribs.hidden = None;
        }
        if attribs.strikethrough.is_some() {
            style.attribs.strikethrough = None;
        }
        if attribs.overline.is_some() {
            style.attribs.overline = None;
        }
    }

    /// Set foreground color using [`ANSI`] color code.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    pub fn set_foreground_color(&mut self, ansi_color: u16) {
        self.parser_global_state.current_style.color_fg =
            Some(TuiColor::from(AnsiValue::from(ansi_color)));
    }

    /// Set background color using [`ANSI`] color code.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    pub fn set_background_color(&mut self, ansi_color: u16) {
        self.parser_global_state.current_style.color_bg =
            Some(TuiColor::from(AnsiValue::from(ansi_color)));
    }

    /// Reset foreground color to default.
    pub fn reset_foreground_color(&mut self) {
        self.parser_global_state.current_style.color_fg = None;
    }

    /// Reset background color to default.
    pub fn reset_background_color(&mut self) {
        self.parser_global_state.current_style.color_bg = None;
    }

    /// Set foreground color using 256-color palette index.
    ///
    /// # Arguments
    ///
    /// * `index` - Palette index (0-255)
    ///
    /// # [`VT-100`] Sequences
    ///
    /// Used with: `ESC[38;5;nm` or `ESC[38:5:nm`
    ///
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    pub fn set_foreground_ansi256(&mut self, index: u8) {
        self.parser_global_state.current_style.color_fg =
            Some(TuiColor::Ansi(AnsiValue::new(index)));
    }

    /// Set background color using 256-color palette index.
    ///
    /// # Arguments
    ///
    /// * `index` - Palette index (0-255)
    ///
    /// # [`VT-100`] Sequences
    ///
    /// Used with: `ESC[48;5;nm` or `ESC[48:5:nm`
    ///
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    pub fn set_background_ansi256(&mut self, index: u8) {
        self.parser_global_state.current_style.color_bg =
            Some(TuiColor::Ansi(AnsiValue::new(index)));
    }

    /// Set foreground color using RGB values.
    ///
    /// # Arguments
    ///
    /// * `r` - Red component (0-255)
    /// * `g` - Green component (0-255)
    /// * `b` - Blue component (0-255)
    ///
    /// # [`VT-100`] Sequences
    ///
    /// Used with: `ESC[38;2;r;g;bm` or `ESC[38:2:r:g:bm`
    ///
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    pub fn set_foreground_rgb(&mut self, r: u8, g: u8, b: u8) {
        self.parser_global_state.current_style.color_fg =
            Some(TuiColor::Rgb(RgbValue::from_u8(r, g, b)));
    }

    /// Set background color using RGB values.
    ///
    /// # Arguments
    ///
    /// * `r` - Red component (0-255)
    /// * `g` - Green component (0-255)
    /// * `b` - Blue component (0-255)
    ///
    /// # [`VT-100`] Sequences
    ///
    /// Used with: `ESC[48;2;r;g;bm` or `ESC[48:2:r:g:bm`
    ///
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    pub fn set_background_rgb(&mut self, r: u8, g: u8, b: u8) {
        self.parser_global_state.current_style.color_bg =
            Some(TuiColor::Rgb(RgbValue::from_u8(r, g, b)));
    }
    /// Apply an extended color sequence to the current style.
    ///
    /// This is a convenience method that takes an [`SgrColorSequence`] and
    /// automatically routes it to the appropriate foreground or background color
    /// setter based on the sequence's target layer.
    ///
    /// # Arguments
    ///
    /// * `color_seq` - The parsed extended color sequence
    ///
    /// # Example
    ///
    /// ```
    /// use r3bl_tui::{SgrColorSequence, OfsBufVT100, height, width};
    ///
    /// let mut buffer = OfsBufVT100::new_empty(height(10) + width(20));
    ///
    /// // Parse an extended color sequence (256-color foreground)
    /// let params = &[38, 5, 196];
    /// if let Some(color_seq) = SgrColorSequence::parse_from_raw_slice(params) {
    ///     // Apply to current style - automatically routes to foreground setter
    ///     buffer.apply_extended_color_sequence(color_seq);
    /// }
    /// ```
    ///
    /// [`SgrColorSequence`]: SgrColorSequence
    pub fn apply_extended_color_sequence(&mut self, color_seq: SgrColorSequence) {
        let tui_color = TuiColor::from(color_seq);
        match color_seq.target() {
            ColorTarget::Foreground => {
                self.parser_global_state.current_style.color_fg = Some(tui_color);
            }
            ColorTarget::Background => {
                self.parser_global_state.current_style.color_bg = Some(tui_color);
            }
        }
    }
}

#[cfg(test)]
mod tests_sgr_ops {
    use super::*;
    use crate::{OfsBufVT100, height, tui_style_attrib, width};

    fn create_test_buffer() -> OfsBufVT100 {
        let size = width(10) + height(6);
        OfsBufVT100::new_empty(size)
    }

    #[test]
    fn test_reset_all_style_attributes() {
        let mut buffer = create_test_buffer();

        // Set some attributes first
        buffer.apply_style_attribute(TuiStyleAttribs::from(tui_style_attrib::Bold));
        buffer.set_foreground_color(31); // Red

        // Verify they're set
        assert!(
            buffer
                .parser_global_state
                .current_style
                .attribs
                .bold
                .is_some()
        );
        assert!(buffer.parser_global_state.current_style.color_fg.is_some());

        buffer.reset_all_style_attributes();

        // Should be reset to defaults
        let style = &buffer.parser_global_state.current_style;
        assert!(style.attribs.bold.is_none());
        assert!(style.color_fg.is_none());
        assert!(style.color_bg.is_none());
    }

    #[test]
    fn test_apply_style_attribute_bold() {
        let mut buffer = create_test_buffer();

        buffer.apply_style_attribute(TuiStyleAttribs::from(tui_style_attrib::Bold));

        assert!(
            buffer
                .parser_global_state
                .current_style
                .attribs
                .bold
                .is_some()
        );
    }

    #[test]
    fn test_apply_style_attribute_italic() {
        let mut buffer = create_test_buffer();

        buffer.apply_style_attribute(TuiStyleAttribs::from(tui_style_attrib::Italic));

        assert!(
            buffer
                .parser_global_state
                .current_style
                .attribs
                .italic
                .is_some()
        );
    }

    #[test]
    fn test_reset_style_attribute_bold_and_dim() {
        let mut buffer = create_test_buffer();

        buffer.apply_style_attribute(TuiStyleAttribs::from(tui_style_attrib::Bold));
        buffer.apply_style_attribute(TuiStyleAttribs::from(tui_style_attrib::Dim));

        // Reset bold should also reset dim (they're mutually exclusive)
        buffer.reset_style_attribute(TuiStyleAttribs::from(tui_style_attrib::Bold));

        assert!(
            buffer
                .parser_global_state
                .current_style
                .attribs
                .bold
                .is_none()
        );
        assert!(
            buffer
                .parser_global_state
                .current_style
                .attribs
                .dim
                .is_none()
        );
    }

    #[test]
    fn test_set_foreground_color() {
        let mut buffer = create_test_buffer();

        buffer.set_foreground_color(31); // Red

        assert!(buffer.parser_global_state.current_style.color_fg.is_some());
        if let Some(color) = buffer.parser_global_state.current_style.color_fg {
            // Should be red color
            // Red is ANSI color 31, which maps to palette index 9 (basic red)
            assert!(matches!(color, TuiColor::Ansi(a) if a.index < 16));
        }
    }

    #[test]
    fn test_set_background_color() {
        let mut buffer = create_test_buffer();

        buffer.set_background_color(42); // Green background

        assert!(buffer.parser_global_state.current_style.color_bg.is_some());
        if let Some(color) = buffer.parser_global_state.current_style.color_bg {
            // Should be green color
            // Green is ANSI color 42, which maps to palette index 10 (basic green)
            assert!(matches!(color, TuiColor::Ansi(a) if a.index < 16));
        }
    }

    #[test]
    fn test_reset_foreground_color() {
        let mut buffer = create_test_buffer();

        buffer.set_foreground_color(31);
        assert!(buffer.parser_global_state.current_style.color_fg.is_some());

        buffer.reset_foreground_color();
        assert!(buffer.parser_global_state.current_style.color_fg.is_none());
    }

    #[test]
    fn test_reset_background_color() {
        let mut buffer = create_test_buffer();

        buffer.set_background_color(42);
        assert!(buffer.parser_global_state.current_style.color_bg.is_some());

        buffer.reset_background_color();
        assert!(buffer.parser_global_state.current_style.color_bg.is_none());
    }

    #[test]
    fn test_multiple_attributes() {
        let mut buffer = create_test_buffer();

        buffer.apply_style_attribute(TuiStyleAttribs::from(tui_style_attrib::Bold));
        buffer.apply_style_attribute(TuiStyleAttribs::from(tui_style_attrib::Italic));
        buffer.apply_style_attribute(TuiStyleAttribs::from(tui_style_attrib::Underline));

        let style = &buffer.parser_global_state.current_style;
        assert!(style.attribs.bold.is_some());
        assert!(style.attribs.italic.is_some());
        assert!(style.attribs.underline.is_some());
    }
}
