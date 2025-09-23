// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Implementation of [`PixelCharLine`] struct and its methods.
//!
//! [`PixelCharLine`] represents a single row of pixels/characters in the offscreen
//! buffer.
//!
//! [`PixelCharLine`]: crate::PixelCharLine
//! Each line can contain various types of pixel characters including plain text,
//! spacers, and void characters.

use std::{fmt::{self, Debug},
          ops::{Deref, DerefMut}};

use smallvec::smallvec;

use super::PixelChar;
use crate::{ColWidth, GetMemSize, InlineVec, TinyInlineString, dim_underline,
            get_mem_size, ok, tiny_inline_string};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PixelCharLine {
    pub pixel_chars: Vec<PixelChar>,
}

impl GetMemSize for PixelCharLine {
    fn get_mem_size(&self) -> usize {
        get_mem_size::slice_size(self.pixel_chars.as_ref())
    }
}

impl Debug for PixelCharLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Pretty print only so many chars per line (depending on the terminal width
        // in which log.fish is run).
        const MAX_PIXEL_CHARS_PER_LINE: usize = 6;

        let mut void_indices: InlineVec<usize> = smallvec![];
        let mut spacer_indices: InlineVec<usize> = smallvec![];
        let mut void_count: InlineVec<TinyInlineString> = smallvec![];
        let mut spacer_count: InlineVec<TinyInlineString> = smallvec![];

        let mut char_count = 0;

        // Loop: for each PixelChar in a line (pixel_chars_lines[row_index]).
        for (col_index, pixel_char) in self.iter().enumerate() {
            match pixel_char {
                PixelChar::Void => {
                    void_count.push(TinyInlineString::from(col_index.to_string()));
                    void_indices.push(col_index);
                }
                PixelChar::Spacer => {
                    spacer_count.push(TinyInlineString::from(col_index.to_string()));
                    spacer_indices.push(col_index);
                }
                PixelChar::PlainText { .. } => {}
            }

            // Index message.
            write!(
                f,
                "{}{:?}",
                dim_underline(&tiny_inline_string!("{col_index:03}")),
                pixel_char
            )?;

            // Add \n every MAX_CHARS_PER_LINE characters.
            char_count += 1;
            if char_count >= MAX_PIXEL_CHARS_PER_LINE {
                char_count = 0;
                writeln!(f)?;
            }
        }

        // Pretty print the spacers & voids (of any of either or both) at the end of
        // the output.
        {
            if !void_count.is_empty() {
                write!(f, "void [ ")?;
                fmt_impl_index_values(&void_indices, f)?;
                write!(f, " ]")?;

                // Add spacer divider if spacer count exists (next).
                if !spacer_count.is_empty() {
                    write!(f, " | ")?;
                }
            }

            if !spacer_count.is_empty() {
                // Add comma divider if void count exists (previous).
                if !void_count.is_empty() {
                    write!(f, ", ")?;
                }
                write!(f, "spacer [ ")?;
                fmt_impl_index_values(&spacer_indices, f)?;
                write!(f, " ]")?;
            }
        }

        ok!()
    }
}

fn fmt_impl_index_values(
    values: &[usize],
    f: &mut fmt::Formatter<'_>,
) -> std::fmt::Result {
    mod helpers {
        pub enum Peek {
            NextItemContinuesRange,
            NextItemDoesNotContinueRange,
        }

        pub fn peek_does_next_item_continues_range(
            values: &[usize],
            index: usize,
        ) -> Peek {
            if values.get(index + 1).is_none() {
                return Peek::NextItemDoesNotContinueRange;
            }
            if values[index + 1] == values[index] + 1 {
                Peek::NextItemContinuesRange
            } else {
                Peek::NextItemDoesNotContinueRange
            }
        }

        pub enum CurrentRange {
            DoesNotExist,
            Exists,
        }

        pub fn does_current_range_exist(current_range: &[usize]) -> CurrentRange {
            if current_range.is_empty() {
                CurrentRange::DoesNotExist
            } else {
                CurrentRange::Exists
            }
        }
    }

    // Track state thru loop iteration.
    let mut acc_current_range: InlineVec<usize> = smallvec![];

    // Main loop.
    for (index, value) in values.iter().enumerate() {
        match (
            helpers::peek_does_next_item_continues_range(values, index),
            helpers::does_current_range_exist(&acc_current_range),
        ) {
            // Start new current range OR the next value continues the current range.
            (
                helpers::Peek::NextItemContinuesRange,
                helpers::CurrentRange::DoesNotExist | helpers::CurrentRange::Exists,
            ) => {
                acc_current_range.push(*value);
            }
            // The next value does not continue the current range & the current range
            // does not exist.
            (
                helpers::Peek::NextItemDoesNotContinueRange,
                helpers::CurrentRange::DoesNotExist,
            ) => {
                if index > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{value}")?;
            }
            // The next value does not continue the current range & the current range
            // exists.
            (
                helpers::Peek::NextItemDoesNotContinueRange,
                helpers::CurrentRange::Exists,
            ) => {
                if index > 0 {
                    write!(f, ", ")?;
                }
                acc_current_range.push(*value);
                write!(
                    f,
                    "{}-{}",
                    acc_current_range[0],
                    acc_current_range[acc_current_range.len() - 1]
                )?;
                acc_current_range.clear();
            }
        }
    }

    ok!()
}

// This represents a single row on the screen (i.e. a line of text).
impl PixelCharLine {
    /// Create a new row with the given width and fill it with the empty chars.
    #[must_use]
    pub fn new_empty(arg_window_width: impl Into<ColWidth>) -> Self {
        let window_width = arg_window_width.into();
        Self {
            pixel_chars: vec![PixelChar::Spacer; window_width.as_usize()],
        }
    }
}

impl Deref for PixelCharLine {
    type Target = Vec<PixelChar>;
    fn deref(&self) -> &Self::Target { &self.pixel_chars }
}

impl DerefMut for PixelCharLine {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.pixel_chars }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TuiStyle, width};

    #[test]
    fn test_pixel_char_line_new_empty() {
        let width = width(5);
        let line = PixelCharLine::new_empty(width);

        assert_eq!(line.pixel_chars.len(), 5);
        for pixel_char in &line.pixel_chars {
            assert!(matches!(pixel_char, PixelChar::Spacer));
        }
    }

    #[test]
    fn test_pixel_char_line_new_empty_zero_width() {
        let width = width(0);
        let line = PixelCharLine::new_empty(width);

        assert_eq!(line.pixel_chars.len(), 0);
        assert!(line.pixel_chars.is_empty());
    }

    #[test]
    fn test_pixel_char_line_deref() {
        let width = width(3);
        let line = PixelCharLine::new_empty(width);

        // Test deref functionality.
        assert_eq!(line.len(), 3);
        assert_eq!(line[0], PixelChar::Spacer);
        assert_eq!(line[1], PixelChar::Spacer);
        assert_eq!(line[2], PixelChar::Spacer);
    }

    #[test]
    fn test_pixel_char_line_deref_mut() {
        let width = width(2);
        let mut line = PixelCharLine::new_empty(width);

        // Test deref_mut functionality.
        line[0] = PixelChar::PlainText {
            display_char: 'A',
            style: TuiStyle::default(),
        };
        line[1] = PixelChar::Void;

        assert!(matches!(
            line[0],
            PixelChar::PlainText {
                display_char: 'A',
                ..
            }
        ));
        assert!(matches!(line[1], PixelChar::Void));
    }

    #[test]
    fn test_pixel_char_line_memory_size() {
        let width = width(10);
        let line = PixelCharLine::new_empty(width);

        let mem_size = line.get_mem_size();
        assert!(mem_size > 0);
    }

    #[test]
    fn test_pixel_char_line_debug_formatting() {
        let width = width(3);
        let mut line = PixelCharLine::new_empty(width);

        // Add some different pixel char types.
        line[0] = PixelChar::PlainText {
            display_char: 'X',
            style: TuiStyle::default(),
        };
        line[1] = PixelChar::Void;
        line[2] = PixelChar::Spacer;

        let debug_output = format!("{line:?}");

        // Check that debug output contains expected elements.
        assert!(debug_output.contains("000")); // Index
        assert!(debug_output.contains("001")); // Index
        assert!(debug_output.contains("002")); // Index
        assert!(debug_output.contains("void")); // Void section
        assert!(debug_output.contains("spacer")); // Spacer section
    }

    #[test]
    fn test_pixel_char_line_debug_with_only_voids() {
        let width = width(3);
        let mut line = PixelCharLine::new_empty(width);

        // Fill with voids.
        for pixel_char in &mut line.pixel_chars {
            *pixel_char = PixelChar::Void;
        }

        let debug_output = format!("{line:?}");
        assert!(debug_output.contains("void"));
        assert!(debug_output.contains("0-2")); // Range should be collapsed
    }

    #[test]
    fn test_pixel_char_line_debug_with_only_spacers() {
        let width = width(2);
        let line = PixelCharLine::new_empty(width);

        let debug_output = format!("{line:?}");
        assert!(debug_output.contains("spacer"));
        assert!(debug_output.contains("0-1")); // Range should be collapsed
    }

    #[test]
    fn test_pixel_char_line_equality() {
        let width = width(2);
        let line1 = PixelCharLine::new_empty(width);
        let line2 = PixelCharLine::new_empty(width);

        assert_eq!(line1, line2);

        // Modify one line.
        let mut line3 = PixelCharLine::new_empty(width);
        line3[0] = PixelChar::Void;

        assert_ne!(line1, line3);
    }

    #[test]
    fn test_pixel_char_line_clone() {
        let width = width(2);
        let mut line = PixelCharLine::new_empty(width);
        line[0] = PixelChar::PlainText {
            display_char: 'Z',
            style: TuiStyle::default(),
        };

        let cloned = line.clone();
        assert_eq!(line, cloned);

        // Verify deep clone.
        assert!(matches!(
            cloned[0],
            PixelChar::PlainText {
                display_char: 'Z',
                ..
            }
        ));
    }

    #[test]
    fn test_fmt_impl_index_values_single_values() {
        // Test the helper function indirectly through debug formatting.
        let width = width(5);
        let mut line = PixelCharLine::new_empty(width);

        // Set non-consecutive void positions.
        line[0] = PixelChar::Void;
        line[2] = PixelChar::Void;
        line[4] = PixelChar::Void;

        let debug_output = format!("{line:?}");
        assert!(debug_output.contains("void"));
        // Should contain individual indices, not ranges.
        assert!(
            debug_output.contains('0')
                && debug_output.contains('2')
                && debug_output.contains('4')
        );
    }

    #[test]
    fn test_fmt_impl_index_values_ranges() {
        // Test the helper function with consecutive ranges.
        let width = width(6);
        let mut line = PixelCharLine::new_empty(width);

        // Set consecutive void positions.
        line[1] = PixelChar::Void;
        line[2] = PixelChar::Void;
        line[3] = PixelChar::Void;

        let debug_output = format!("{line:?}");
        assert!(debug_output.contains("void"));
        assert!(debug_output.contains("1-3")); // Should be collapsed to range
    }
}
