// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! SGR (Select Graphic Rendition) operations for VT100/ANSI terminal emulation.
//!
//! This module implements SGR operations that correspond to ANSI SGR
//! sequences handled by the `vt_100_ansi_parser::operations::sgr_ops` module. These
//! include:
//!
//! - **SGR 0** (Reset) - [`reset_all_style_attributes`]
//! - **SGR 1** (Bold) - [`apply_style_attribute`]
//! - **SGR 2** (Dim) - [`apply_style_attribute`]
//! - **SGR 3** (Italic) - [`apply_style_attribute`]
//! - **SGR 4** (Underline) - [`apply_style_attribute`]
//! - **SGR 5/6** (Blink) - [`apply_style_attribute`]
//! - **SGR 7** (Reverse) - [`apply_style_attribute`]
//! - **SGR 8** (Hidden) - [`apply_style_attribute`]
//! - **SGR 9** (Strikethrough) - [`apply_style_attribute`]
//! - **SGR 30-37** (Foreground colors) - [`set_foreground_color`]
//! - **SGR 40-47** (Background colors) - [`set_background_color`]
//! - **SGR 90-97** (Bright foreground) - [`set_foreground_color`]
//! - **SGR 100-107** (Bright background) - [`set_background_color`]
//!
//! All operations maintain VT100 compliance and handle proper style state
//! management for terminal text rendering.
//!
//! This module implements the business logic for SGR operations delegated from
//! the parser shim. The `impl_` prefix follows our naming convention for searchable
//! code organization. See [parser module docs](crate::core::pty_mux::vt_100_ansi_parser)
//! for the complete three-layer architecture.
//!
//! **Related Files:**
//! - **Shim**: [`sgr_ops`] - Parameter translation and delegation (no direct tests)
//! - **Integration Tests**: [`test_sgr_ops`] - Full ANSI pipeline testing
//!
//! [`reset_all_style_attributes`]: crate::OffscreenBuffer::reset_all_style_attributes
//! [`apply_style_attribute`]: crate::OffscreenBuffer::apply_style_attribute
//! [`set_foreground_color`]: crate::OffscreenBuffer::set_foreground_color
//! [`set_background_color`]: crate::OffscreenBuffer::set_background_color
//! [`sgr_ops`]: crate::core::pty_mux::vt_100_ansi_parser::operations::vt_100_shim_sgr_ops
//! [`test_sgr_ops`]: crate::core::pty_mux::vt_100_ansi_parser::vt_100_ansi_conformance_tests::tests::vt_100_test_sgr_ops

#[allow(clippy::wildcard_imports)]
use super::super::*;
use crate::{TuiStyle,
            core::pty_mux::vt_100_ansi_parser::ansi_to_tui_color::{
                ansi256_to_tui_color, ansi_to_tui_color, rgb_to_tui_color,
            },
            tui_style_attrib};

impl OffscreenBuffer {
    /// Reset all SGR attributes to default state.
    pub fn reset_all_style_attributes(&mut self) {
        self.ansi_parser_support.current_style = TuiStyle::default();
    }

    /// Apply a style attribute to the current style.
    pub fn apply_style_attribute(&mut self, attribute: StyleAttribute) {
        let style = &mut self.ansi_parser_support.current_style;

        match attribute {
            StyleAttribute::Bold => {
                style.attribs.bold = Some(tui_style_attrib::Bold);
            }
            StyleAttribute::Dim => {
                style.attribs.dim = Some(tui_style_attrib::Dim);
            }
            StyleAttribute::Italic => {
                style.attribs.italic = Some(tui_style_attrib::Italic);
            }
            StyleAttribute::Underline => {
                style.attribs.underline = Some(tui_style_attrib::Underline);
            }
            StyleAttribute::Blink => {
                style.attribs.blink = Some(tui_style_attrib::Blink);
            }
            StyleAttribute::Reverse => {
                style.attribs.reverse = Some(tui_style_attrib::Reverse);
            }
            StyleAttribute::Hidden => {
                style.attribs.hidden = Some(tui_style_attrib::Hidden);
            }
            StyleAttribute::Strikethrough => {
                style.attribs.strikethrough = Some(tui_style_attrib::Strikethrough);
            }
        }
    }

    /// Reset a specific style attribute.
    pub fn reset_style_attribute(&mut self, attribute: StyleAttribute) {
        let style = &mut self.ansi_parser_support.current_style;

        match attribute {
            StyleAttribute::Bold => {
                style.attribs.bold = None;
                style.attribs.dim = None; // Bold and dim are mutually exclusive
            }
            StyleAttribute::Dim => {
                style.attribs.dim = None;
                style.attribs.bold = None; // Bold and dim are mutually exclusive
            }
            StyleAttribute::Italic => {
                style.attribs.italic = None;
            }
            StyleAttribute::Underline => {
                style.attribs.underline = None;
            }
            StyleAttribute::Blink => {
                style.attribs.blink = None;
            }
            StyleAttribute::Reverse => {
                style.attribs.reverse = None;
            }
            StyleAttribute::Hidden => {
                style.attribs.hidden = None;
            }
            StyleAttribute::Strikethrough => {
                style.attribs.strikethrough = None;
            }
        }
    }

    /// Set foreground color using ANSI color code.
    pub fn set_foreground_color(&mut self, ansi_color: u16) {
        self.ansi_parser_support.current_style.color_fg =
            Some(ansi_to_tui_color(ansi_color.into()));
    }

    /// Set background color using ANSI color code.
    pub fn set_background_color(&mut self, ansi_color: u16) {
        self.ansi_parser_support.current_style.color_bg =
            Some(ansi_to_tui_color(ansi_color.into()));
    }

    /// Reset foreground color to default.
    pub fn reset_foreground_color(&mut self) {
        self.ansi_parser_support.current_style.color_fg = None;
    }

    /// Reset background color to default.
    pub fn reset_background_color(&mut self) {
        self.ansi_parser_support.current_style.color_bg = None;
    }

    /// Set foreground color using 256-color palette index.
    ///
    /// # Arguments
    ///
    /// * `index` - Palette index (0-255)
    ///
    /// # VT100 Sequences
    ///
    /// Used with: `ESC[38;5;nm` or `ESC[38:5:nm`
    pub fn set_foreground_ansi256(&mut self, index: u8) {
        self.ansi_parser_support.current_style.color_fg = Some(ansi256_to_tui_color(index));
    }

    /// Set background color using 256-color palette index.
    ///
    /// # Arguments
    ///
    /// * `index` - Palette index (0-255)
    ///
    /// # VT100 Sequences
    ///
    /// Used with: `ESC[48;5;nm` or `ESC[48:5:nm`
    pub fn set_background_ansi256(&mut self, index: u8) {
        self.ansi_parser_support.current_style.color_bg = Some(ansi256_to_tui_color(index));
    }

    /// Set foreground color using RGB values.
    ///
    /// # Arguments
    ///
    /// * `r` - Red component (0-255)
    /// * `g` - Green component (0-255)
    /// * `b` - Blue component (0-255)
    ///
    /// # VT100 Sequences
    ///
    /// Used with: `ESC[38;2;r;g;bm` or `ESC[38:2:r:g:bm`
    pub fn set_foreground_rgb(&mut self, r: u8, g: u8, b: u8) {
        self.ansi_parser_support.current_style.color_fg = Some(rgb_to_tui_color(r, g, b));
    }

    /// Set background color using RGB values.
    ///
    /// # Arguments
    ///
    /// * `r` - Red component (0-255)
    /// * `g` - Green component (0-255)
    /// * `b` - Blue component (0-255)
    ///
    /// # VT100 Sequences
    ///
    /// Used with: `ESC[48;2;r;g;bm` or `ESC[48:2:r:g:bm`
    pub fn set_background_rgb(&mut self, r: u8, g: u8, b: u8) {
        self.ansi_parser_support.current_style.color_bg = Some(rgb_to_tui_color(r, g, b));
    }
}

/// Represents different style attributes that can be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleAttribute {
    Bold,
    Dim,
    Italic,
    Underline,
    Blink,
    Reverse,
    Hidden,
    Strikethrough,
}

#[cfg(test)]
mod tests_sgr_ops {
    use super::*;
    use crate::{TuiColor, height, width};

    fn create_test_buffer() -> OffscreenBuffer {
        let size = width(10) + height(6);
        OffscreenBuffer::new_empty(size)
    }

    #[test]
    fn test_reset_all_style_attributes() {
        let mut buffer = create_test_buffer();

        // Set some attributes first
        buffer.apply_style_attribute(StyleAttribute::Bold);
        buffer.set_foreground_color(31); // Red

        // Verify they're set
        assert!(
            buffer
                .ansi_parser_support
                .current_style
                .attribs
                .bold
                .is_some()
        );
        assert!(buffer.ansi_parser_support.current_style.color_fg.is_some());

        buffer.reset_all_style_attributes();

        // Should be reset to defaults
        let style = &buffer.ansi_parser_support.current_style;
        assert!(style.attribs.bold.is_none());
        assert!(style.color_fg.is_none());
        assert!(style.color_bg.is_none());
    }

    #[test]
    fn test_apply_style_attribute_bold() {
        let mut buffer = create_test_buffer();

        buffer.apply_style_attribute(StyleAttribute::Bold);

        assert!(
            buffer
                .ansi_parser_support
                .current_style
                .attribs
                .bold
                .is_some()
        );
    }

    #[test]
    fn test_apply_style_attribute_italic() {
        let mut buffer = create_test_buffer();

        buffer.apply_style_attribute(StyleAttribute::Italic);

        assert!(
            buffer
                .ansi_parser_support
                .current_style
                .attribs
                .italic
                .is_some()
        );
    }

    #[test]
    fn test_reset_style_attribute_bold_and_dim() {
        let mut buffer = create_test_buffer();

        buffer.apply_style_attribute(StyleAttribute::Bold);
        buffer.apply_style_attribute(StyleAttribute::Dim);

        // Reset bold should also reset dim (they're mutually exclusive)
        buffer.reset_style_attribute(StyleAttribute::Bold);

        assert!(
            buffer
                .ansi_parser_support
                .current_style
                .attribs
                .bold
                .is_none()
        );
        assert!(
            buffer
                .ansi_parser_support
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

        assert!(buffer.ansi_parser_support.current_style.color_fg.is_some());
        if let Some(color) = buffer.ansi_parser_support.current_style.color_fg {
            // Should be red color
            // Red is ANSI color 31, which maps to standard red
            assert!(matches!(color, TuiColor::Basic(_)));
        }
    }

    #[test]
    fn test_set_background_color() {
        let mut buffer = create_test_buffer();

        buffer.set_background_color(42); // Green background

        assert!(buffer.ansi_parser_support.current_style.color_bg.is_some());
        if let Some(color) = buffer.ansi_parser_support.current_style.color_bg {
            // Should be green color
            // Green is ANSI color 42, which maps to standard green
            assert!(matches!(color, TuiColor::Basic(_)));
        }
    }

    #[test]
    fn test_reset_foreground_color() {
        let mut buffer = create_test_buffer();

        buffer.set_foreground_color(31);
        assert!(buffer.ansi_parser_support.current_style.color_fg.is_some());

        buffer.reset_foreground_color();
        assert!(buffer.ansi_parser_support.current_style.color_fg.is_none());
    }

    #[test]
    fn test_reset_background_color() {
        let mut buffer = create_test_buffer();

        buffer.set_background_color(42);
        assert!(buffer.ansi_parser_support.current_style.color_bg.is_some());

        buffer.reset_background_color();
        assert!(buffer.ansi_parser_support.current_style.color_bg.is_none());
    }

    #[test]
    fn test_multiple_attributes() {
        let mut buffer = create_test_buffer();

        buffer.apply_style_attribute(StyleAttribute::Bold);
        buffer.apply_style_attribute(StyleAttribute::Italic);
        buffer.apply_style_attribute(StyleAttribute::Underline);

        let style = &buffer.ansi_parser_support.current_style;
        assert!(style.attribs.bold.is_some());
        assert!(style.attribs.italic.is_some());
        assert!(style.attribs.underline.is_some());
    }
}
