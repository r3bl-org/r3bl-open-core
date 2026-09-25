// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Text attribute sequence generation for [`SGR`] parameters.
//!
//! This module provides the [`SgrAttributeSequence`] enum which represents text
//! formatting attributes (bold, italic, underline, etc.) and can serialize them into
//! [`ANSI`] escape codes.
//!
//! # Architecture Note
//!
//! **[`SGR`] (Select Graphic Rendition) is a subset of [`CSI`] sequences**, not a
//! separate protocol. All [`SGR`] sequences:
//! - Begin with the [`CSI`] introducer `ESC [`
//! - Contain parameters for text attributes or colors
//! - End with the `'m'` character ([`SGR_SET_GRAPHICS`])
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`CSI`]: crate::CsiSequence
//! [`SGR_SET_GRAPHICS`]: crate::core::ansi::constants::SGR_SET_GRAPHICS
//! [`SGR`]: crate::SgrCode

use crate::{BufTextStorage, FastStringify, TuiStyle, TuiStyleAttribs,
            core::ansi::constants::{CSI_PARAM_SEPARATOR, CSI_START, SGR_BLINK,
                                    SGR_BOLD, SGR_DIM, SGR_HIDDEN, SGR_ITALIC,
                                    SGR_OVERLINE, SGR_RAPID_BLINK, SGR_RESET_STR,
                                    SGR_REVERSE, SGR_SET_GRAPHICS, SGR_STRIKETHROUGH,
                                    SGR_UNDERLINE},
            generate_impl_display_for_fast_stringify, ok, tui_style_attrib};
use smallvec::SmallVec;

/// Represents text attribute [`SGR`] operations.
///
/// Can represent setting a set of attributes from [`TuiStyleAttribs`], or resetting all
/// attributes to default.
///
/// [`SGR`]: crate::SgrCode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SgrAttributeSequence {
    /// Set one or more text attributes (e.g., bold, italic).
    Set(TuiStyleAttribs),

    /// Reset all text attributes to default ([`SGR`] 0).
    ///
    /// [`SGR`]: crate::SgrCode
    Reset,
}

impl From<&TuiStyle> for SgrAttributeSequence {
    fn from(style: &TuiStyle) -> Self { Self::Set(style.attribs) }
}

impl FastStringify for SgrAttributeSequence {
    fn write_to_buf(&self, buf: &mut BufTextStorage) -> std::fmt::Result {
        match *self {
            SgrAttributeSequence::Set(attribs) => {
                if attribs.is_none() {
                    return ok!();
                }

                let mut codes: SmallVec<[u16; 9]> = SmallVec::new();
                if attribs.bold.is_some() {
                    codes.push(SGR_BOLD);
                }
                if attribs.dim.is_some() {
                    codes.push(SGR_DIM);
                }
                if attribs.italic.is_some() {
                    codes.push(SGR_ITALIC);
                }
                if attribs.underline.is_some() {
                    codes.push(SGR_UNDERLINE);
                }
                if let Some(blink) = attribs.blink {
                    match blink {
                        tui_style_attrib::BlinkMode::Slow => codes.push(SGR_BLINK),
                        tui_style_attrib::BlinkMode::Rapid => codes.push(SGR_RAPID_BLINK),
                    }
                }
                if attribs.reverse.is_some() {
                    codes.push(SGR_REVERSE);
                }
                if attribs.hidden.is_some() {
                    codes.push(SGR_HIDDEN);
                }
                if attribs.strikethrough.is_some() {
                    codes.push(SGR_STRIKETHROUGH);
                }
                if attribs.overline.is_some() {
                    codes.push(SGR_OVERLINE);
                }

                // Build the SGR sequence using the constants
                buf.push_str(CSI_START);
                for (i, code) in codes.iter().enumerate() {
                    if i > 0 {
                        buf.push(CSI_PARAM_SEPARATOR);
                    }
                    buf.push_str(&code.to_string());
                }
                buf.push(SGR_SET_GRAPHICS);

                ok!()
            }
            SgrAttributeSequence::Reset => {
                buf.push_str(SGR_RESET_STR);
                ok!()
            }
        }
    }
}

generate_impl_display_for_fast_stringify!(SgrAttributeSequence);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui_style_attrib::{Bold, Dim, Italic, Underline};

    #[test]
    fn test_empty_attributes() {
        let seq = SgrAttributeSequence::Set(TuiStyleAttribs::default());
        assert_eq!(seq.to_string(), "");
    }

    #[test]
    fn test_single_attribute() {
        let seq = SgrAttributeSequence::Set(Bold.into());
        assert_eq!(seq.to_string(), "\x1b[1m");
    }

    #[test]
    fn test_multiple_attributes() {
        let attribs = Bold + Italic + Underline;
        let seq = SgrAttributeSequence::Set(attribs);
        assert_eq!(seq.to_string(), "\x1b[1;3;4m");
    }

    #[test]
    fn test_reset_sequence() {
        let seq = SgrAttributeSequence::Reset;
        assert_eq!(seq.to_string(), "\x1b[0m");
    }

    #[test]
    fn test_from_tui_style() {
        let style = TuiStyle {
            attribs: Bold + Dim,
            ..Default::default()
        };
        let seq = SgrAttributeSequence::from(&style);
        assert_eq!(seq.to_string(), "\x1b[1;2m");
    }
}
