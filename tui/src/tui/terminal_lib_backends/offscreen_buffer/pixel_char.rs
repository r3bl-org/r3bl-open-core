// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Implementation of [`PixelChar`] enum and its methods.
//!
//! [`PixelChar`] represents a single character cell in the offscreen buffer.
//!
//! [`PixelChar`]: crate::PixelChar
//! It can be a void (invisible), spacer (empty but visible), or plain text
//! with optional styling information.

use crate::{GetMemSize, TuiStyle, fg_magenta, ok};
use std::fmt::{self, Debug};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PixelChar {
    Void,
    #[default]
    Spacer,
    PlainText {
        display_char: char,
        style: TuiStyle,
    },
}

pub const EMPTY_CHAR: char = '╳';
pub const VOID_CHAR: char = '❯';

impl GetMemSize for PixelChar {
    fn get_mem_size(&self) -> usize {
        // Since PixelChar is now Copy, its size is fixed.
        std::mem::size_of::<PixelChar>()
    }
}

impl Debug for PixelChar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const WIDTH: usize = 16;

        match self {
            PixelChar::Void => {
                write!(f, " V {VOID_CHAR:░^WIDTH$}")?;
            }
            PixelChar::Spacer => {
                write!(f, " S {EMPTY_CHAR:░^WIDTH$}")?;
            }
            PixelChar::PlainText {
                display_char,
                style,
            } => {
                if style == &TuiStyle::default() {
                    // Content, no style.
                    write!(f, " {} '{display_char}': ^WIDTH$", fg_magenta("P"))?;
                } else {
                    // Content + style.
                    write!(f, " {} '{display_char}'→{style: ^WIDTH$}", fg_magenta("P"))?;
                }
            }
        }

        ok!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{height, new_style,
                tui::terminal_lib_backends::offscreen_buffer::OffscreenBuffer,
                tui_color, tui_style_attrib::Underline, tui_style_attribs, width};

    fn create_test_buffer() -> OffscreenBuffer {
        let window_size = width(4) + height(2);
        OffscreenBuffer::new_empty(window_size)
    }

    #[test]
    fn test_default_pixel_char() {
        // Test PixelChar::default().
        let default_char = PixelChar::default();
        assert!(matches!(default_char, PixelChar::Spacer));

        // Test that new buffer uses default.
        let window_size = width(1) + height(1);
        let ofs_buf = OffscreenBuffer::new_empty(window_size);
        assert!(matches!(ofs_buf.buffer[0][0], PixelChar::Spacer));
    }

    #[test]
    fn test_pixel_char_variants() {
        // Test different PixelChar variants and their behavior.
        let mut ofs_buf = create_test_buffer();

        // Test Spacer variant (default).
        assert!(matches!(ofs_buf.buffer[0][0], PixelChar::Spacer));

        // Test PlainText with no style.
        ofs_buf.buffer[0][1] = PixelChar::PlainText {
            display_char: 'a',
            style: TuiStyle::default(),
        };

        match &ofs_buf.buffer[0][1] {
            PixelChar::PlainText {
                display_char,
                style,
            } => {
                assert_eq!(*display_char, 'a');
                assert_eq!(*style, TuiStyle::default());
            }
            _ => panic!("Expected PlainText variant"),
        }

        // Test PlainText with style.
        ofs_buf.buffer[0][2] = PixelChar::PlainText {
            display_char: 'b',
            style: new_style!(underline color_fg: {tui_color!(red)}),
        };

        match &ofs_buf.buffer[0][2] {
            PixelChar::PlainText {
                display_char,
                style,
            } => {
                assert_eq!(*display_char, 'b');
                assert_eq!(style.attribs, tui_style_attribs(Underline));
                assert_eq!(style.color_fg, Some(tui_color!(red)));
            }
            _ => panic!("Expected styled PlainText variant"),
        }

        // Test Void variant.
        ofs_buf.buffer[1][0] = PixelChar::Void;
        assert!(matches!(ofs_buf.buffer[1][0], PixelChar::Void));

        // Test space character with no style.
        ofs_buf.buffer[1][1] = PixelChar::PlainText {
            display_char: ' ',
            style: TuiStyle::default(),
        };

        match &ofs_buf.buffer[1][1] {
            PixelChar::PlainText {
                display_char,
                style,
            } => {
                assert_eq!(*display_char, ' ');
                assert_eq!(*style, TuiStyle::default());
            }
            _ => panic!("Expected PlainText with space"),
        }

        // Test space character with style.
        ofs_buf.buffer[1][2] = PixelChar::PlainText {
            display_char: ' ',
            style: new_style!(color_bg: {tui_color!(blue)}),
        };

        match &ofs_buf.buffer[1][2] {
            PixelChar::PlainText {
                display_char,
                style: actual_style,
            } => {
                assert_eq!(*display_char, ' ');
                assert_eq!(actual_style.color_bg, Some(tui_color!(blue)));
            }
            _ => panic!("Expected styled space"),
        }
    }

    #[test]
    fn test_pixel_char_memory_size() {
        let void_char = PixelChar::Void;
        let spacer_char = PixelChar::Spacer;
        let plain_char = PixelChar::PlainText {
            display_char: 'x',
            style: TuiStyle::default(),
        };
        let styled_char = PixelChar::PlainText {
            display_char: 'y',
            style: new_style!(color_fg: {tui_color!(green)}),
        };

        // All variants should have the same size since PixelChar is Copy.
        let void_size = void_char.get_mem_size();
        let spacer_size = spacer_char.get_mem_size();
        let plain_size = plain_char.get_mem_size();
        let styled_size = styled_char.get_mem_size();

        assert_eq!(void_size, spacer_size);
        assert_eq!(spacer_size, plain_size);
        assert_eq!(plain_size, styled_size);
        assert_eq!(void_size, std::mem::size_of::<PixelChar>());
    }

    #[test]
    fn test_pixel_char_equality() {
        let char1 = PixelChar::PlainText {
            display_char: 'a',
            style: TuiStyle::default(),
        };
        let char2 = PixelChar::PlainText {
            display_char: 'a',
            style: TuiStyle::default(),
        };
        let char3 = PixelChar::PlainText {
            display_char: 'b',
            style: TuiStyle::default(),
        };
        let char4 = PixelChar::PlainText {
            display_char: 'a',
            style: new_style!(bold),
        };

        assert_eq!(char1, char2); // Same character, no style
        assert_ne!(char1, char3); // Different character
        assert_ne!(char1, char4); // Same character, different style
        assert_ne!(char1, PixelChar::Spacer); // Different variant
        assert_ne!(char1, PixelChar::Void); // Different variant

        assert_eq!(PixelChar::Spacer, PixelChar::Spacer);
        assert_eq!(PixelChar::Void, PixelChar::Void);
        assert_ne!(PixelChar::Spacer, PixelChar::Void);
    }

    #[test]
    fn test_pixel_char_copy_clone() {
        let original = PixelChar::PlainText {
            display_char: 'z',
            style: new_style!(italic color_bg: {tui_color!(yellow)}),
        };

        // Test Copy trait.
        let copied = original;
        assert_eq!(original, copied);

        // Test Clone trait.
        let cloned = original;
        assert_eq!(original, cloned);

        // Modify copied/cloned shouldn't affect original (they're independent).
        // Since PixelChar is Copy, assignment creates a new independent value.
        let modified = PixelChar::Spacer;
        assert_ne!(original, modified);
        assert_eq!(original, copied); // original copy unchanged
    }
}
