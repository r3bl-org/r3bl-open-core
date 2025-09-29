// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Non-editor implementation modules for [`GCStringOwned`].
//!
//! This module contains [`GCStringOwned`] functionality that is used by the general
//! TUI system (not editor-specific operations). These modules provide text formatting,
//! clipping, conversion, and truncation utilities used across various TUI components.
//!
//! The modules here are:
//! - `pad`: String padding operations for UI layout
//! - `clip`: Text clipping for viewport management
//! - `convert`: Type conversions for terminal rendering
//! - `trunc_start`/`trunc_end`: Text truncation utilities
//!
//! These are kept separate from editor-specific operations to maintain clear
//! separation of concerns during the [`crate::ZeroCopyGapBuffer`] migration.

use std::ops::Add;

use super::GCStringOwned;
use crate::{ByteIndex, ColIndex, ColWidth, InlineString, NumericValue, SegIndex,
            byte_index, ch, pad_fmt, seg_index, usize};

/// Convert between different types of indices. This unifies the API so that different
/// index types are all converted into [`SegIndex`] for use with this struct. Here's the
/// list:
/// - [`GCStringOwned`] + [`crate::ByteIndex`] = [Option]<[`SegIndex`]>
/// - [`GCStringOwned`] + [`ColIndex`] = [Option]<[`SegIndex`]>
/// - [`GCStringOwned`] + [`SegIndex`] = [Option]<[`ColIndex`]>
///
/// # Why These Conversions Are Essential
///
/// These conversion operators are the heart of Unicode text handling in the editor:
///
/// 1. **`ByteIndex` → `SegIndex`**: When we have a byte position (e.g., from a file
///    offset or string slice operation), we need to find which grapheme cluster it
///    belongs to. This is crucial for ensuring we never split a multi-byte character.
///
/// 2. **`ColIndex` → `SegIndex`**: When the user clicks at a screen position or we need
///    to render at a specific column, we must find which grapheme cluster is at that
///    display position. This handles wide characters correctly.
///
/// 3. **`SegIndex` → `ColIndex`**: When we have a logical character position and need to
///    know where it appears on screen. This is used for cursor positioning and rendering.
///
/// # Examples
///
/// ```text
/// String: "a😀b"
///
/// ByteIndex 0 → SegIndex 0 (start of 'a')
/// ByteIndex 1 → SegIndex 1 (start of '😀')
/// ByteIndex 2 → None (middle of '😀' - invalid!)
/// ByteIndex 5 → SegIndex 2 (start of 'b')
///
/// ColIndex 0 → SegIndex 0 ('a' at column 0)
/// ColIndex 1 → SegIndex 1 ('😀' starts at column 1)
/// ColIndex 2 → SegIndex 1 ('😀' spans columns 1-2)
/// ColIndex 3 → SegIndex 2 ('b' at column 3)
/// ```
mod convert {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Convert a `byte_index` to a `seg_index`.
    ///
    /// Try and convert a [`GCStringOwned`] + [`ByteIndex`] to a grapheme index
    /// [`SegIndex`].
    impl Add<ByteIndex> for &GCStringOwned {
        type Output = Option<SegIndex>;

        /// Find the grapheme cluster segment (index) that is at the `byte_index` of the
        /// underlying string.
        fn add(self, byte_index: ByteIndex) -> Self::Output {
            let byte_index = byte_index.as_usize();
            for seg in &self.segments {
                let start = usize(seg.start_byte_index);
                let end = usize(seg.end_byte_index);
                if byte_index >= start && byte_index < end {
                    return Some(seg.seg_index);
                }
            }
            None
        }
    }

    /// Convert a `display_col_index` to a `seg_index`.
    ///
    /// Try and convert a [`GCStringOwned`] + [`ColIndex`] (display column index) to a
    /// grapheme index [`SegIndex`].
    impl Add<ColIndex> for &GCStringOwned {
        type Output = Option<SegIndex>;

        /// Find the grapheme cluster segment (index) that can be displayed at the
        /// `display_col_index` of the terminal.
        fn add(self, display_col_index: ColIndex) -> Self::Output {
            self.segments
                .iter()
                .find(|seg| {
                    let seg_display_width = seg.display_width;
                    let seg_start = seg.start_display_col_index;
                    let seg_end = seg_start + seg_display_width;
                    /* is within segment */
                    display_col_index >= seg_start && display_col_index < seg_end
                })
                .map(|seg| seg_index(seg.seg_index))
        }
    }

    /// Convert a `seg_index` to `display_col_index`.
    ///
    /// Try and convert a [`GCStringOwned`] + [`SegIndex`] to a [`ColIndex`] (display
    /// column index).
    impl Add<SegIndex> for &GCStringOwned {
        type Output = Option<ColIndex>;

        /// Find the display column index that corresponds to the grapheme cluster segment
        /// at the `seg_index`.
        fn add(self, seg_index: SegIndex) -> Self::Output {
            self.get(seg_index).map(|seg| seg.start_display_col_index)
        }
    }
}

/// Methods for easily truncating grapheme cluster segments (at the end) for common TUI
/// use cases.
pub mod trunc_end {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl GCStringOwned {
        /// Returns a string slice from `self.string` w/ the segments removed from the end
        /// of the string that don't fit in the given viewport width (which is 1 based,
        /// and not 0 based). Note that the character at `display_col_count` *index* is
        /// NOT included in the result; please see the example below.
        ///
        /// ```text
        ///   ⎧ 3 ⎫ : size (or "width" or "col count" or "count", 1 based)
        /// R ╭───╮
        /// 0 │fir│st second
        ///   ╰───╯
        ///   C012┋345678901 : index (0 based)
        /// ```
        ///
        /// Here's a visual depiction of the different indices.
        ///
        /// *How it appears in the terminal (displayed)*:
        ///
        /// ```text
        /// R ╭──────────────╮
        /// 0 │Hi📦XelLo🙏🏽Bye│
        ///   ╰──────────────╯
        ///  DC01234567890123 : index (0 based)
        /// ```
        ///
        /// *Detailed breakdown*:
        ///
        /// ```text
        /// DW   1 2 34 5 6 7 8 9 01 234 : width (1 based)
        /// DC   0 1 23 4 5 6 7 8 90 123 : index (0 based)
        ///  R ╭ ─ ─ ── ─ ─ ─ ─ ─ ── ───╮
        ///  0 │ H i 📦 X e l L o 🙏🏽 Bye│
        ///    ╰ ─ ─ ── ─ ─ ─ ─ ─ ── ───╯
        ///   SI 0 1 2  3 4 5 6 7 8  901 : index (0 based)
        ///
        /// ❯ DC: display column index | DW: display width
        /// ❯ R: row index | SI: segment index
        /// ```
        pub fn trunc_end_to_fit(&self, arg_col_width: impl Into<ColWidth>) -> &str {
            let mut avail_cols: ColWidth = arg_col_width.into();
            let mut string_end_byte_index = 0;

            for seg in self.seg_iter() {
                let seg_display_width = seg.display_width;
                if avail_cols < seg_display_width {
                    break;
                }
                string_end_byte_index += seg.bytes_size.as_usize();
                avail_cols -= seg_display_width;
            }

            &self.string[..string_end_byte_index]
        }

        /// Removes some number of segments from the end of the string so that `col_count`
        /// (width) is skipped.
        ///
        /// Here's a visual depiction of the different indices.
        ///
        /// *How it appears in the terminal (displayed)*:
        ///
        /// ```text
        /// R ╭──────────────╮
        /// 0 │Hi📦XelLo🙏🏽Bye│
        ///   ╰──────────────╯
        ///  DC01234567890123 : index (0 based)
        /// ```
        ///
        /// *Detailed breakdown*:
        ///
        /// ```text
        /// DW   1 2 34 5 6 7 8 9 01 234 : width (1 based)
        /// DC   0 1 23 4 5 6 7 8 90 123 : index (0 based)
        ///  R ╭ ─ ─ ── ─ ─ ─ ─ ─ ── ───╮
        ///  0 │ H i 📦 X e l L o 🙏🏽 Bye│
        ///    ╰ ─ ─ ── ─ ─ ─ ─ ─ ── ───╯
        ///   SI 0 1 2  3 4 5 6 7 8  901 : index (0 based)
        ///
        /// ❯ DC: display column index | DW: display width
        /// ❯ R: row index | SI: segment index
        /// ```
        pub fn trunc_end_by(&self, arg_col_width: impl Into<ColWidth>) -> &str {
            let mut countdown_col_count: ColWidth = arg_col_width.into();
            let mut string_end_byte_index = byte_index(0);

            let rev_iter = self.segments.iter().rev();

            for seg in rev_iter {
                let seg_display_width = seg.display_width;
                string_end_byte_index = seg.start_byte_index;
                countdown_col_count -= seg_display_width;
                if *countdown_col_count == ch(0) {
                    // We are done skipping.
                    break;
                }
            }

            &self.string[..usize(string_end_byte_index)]
        }
    }
}

/// Methods for easily truncating grapheme cluster segments (from the start) for common
/// TUI use cases.
pub mod trunc_start {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl GCStringOwned {
        /// Removes segments from the start of the string so that `col_count` (width) is
        /// skipped.
        ///
        /// Here's a visual depiction of the different indices.
        ///
        /// *How it appears in the terminal (displayed)*:
        ///
        /// ```text
        /// R ╭──────────────╮
        /// 0 │Hi📦XelLo🙏🏽Bye│
        ///   ╰──────────────╯
        ///  DC01234567890123 : index (0 based)
        /// ```
        ///
        /// *Detailed breakdown*:
        ///
        /// ```text
        /// DW   1 2 34 5 6 7 8 9 01 234 : width (1 based)
        /// DC   0 1 23 4 5 6 7 8 90 123 : index (0 based)
        ///  R ╭ ─ ─ ── ─ ─ ─ ─ ─ ── ───╮
        ///  0 │ H i 📦 X e l L o 🙏🏽 Bye│
        ///    ╰ ─ ─ ── ─ ─ ─ ─ ─ ── ───╯
        ///   SI 0 1 2  3 4 5 6 7 8  901 : index (0 based)
        ///
        /// ❯ DC: display column index | DW: display width
        /// ❯ R: row index | SI: segment index
        /// ```
        pub fn trunc_start_by(&self, arg_col_width: impl Into<ColWidth>) -> &str {
            let mut skip_col_count: ColWidth = arg_col_width.into();
            let mut string_start_byte_index = 0;

            for segment in self.seg_iter() {
                let seg_display_width = segment.display_width;
                if *skip_col_count == ch(0) {
                    // We are done skipping.
                    break;
                }

                // Skip segment.unicode_width.
                skip_col_count -= seg_display_width;
                string_start_byte_index += segment.bytes_size.as_usize();
            }

            &self.string[string_start_byte_index..]
        }
    }
}

/// Methods for easily padding grapheme cluster segments for common TUI use cases.
mod pad {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl GCStringOwned {
        /// Returns a new [`InlineString`] that is the result of padding `self.string` to
        /// fit the given width w/ the given spacer character.
        ///
        /// Here's a visual depiction of the different indices.
        ///
        /// *How it appears in the terminal (displayed)*:
        ///
        /// ```text
        /// R ╭──────────────╮
        /// 0 │Hi📦XelLo🙏🏽Bye│
        ///   ╰──────────────╯
        ///  DC01234567890123 : index (0 based)
        /// ```
        ///
        /// *Detailed breakdown*:
        ///
        /// ```text
        /// DW   1 2 34 5 6 7 8 9 01 234 : width (1 based)
        /// DC   0 1 23 4 5 6 7 8 90 123 : index (0 based)
        ///  R ╭ ─ ─ ── ─ ─ ─ ─ ─ ── ───╮
        ///  0 │ H i 📦 X e l L o 🙏🏽 Bye│
        ///    ╰ ─ ─ ── ─ ─ ─ ─ ─ ── ───╯
        ///   SI 0 1 2  3 4 5 6 7 8  901 : index (0 based)
        ///
        /// ❯ DC: display column index | DW: display width
        /// ❯ R: row index | SI: segment index
        /// ```
        pub fn pad_end_to_fit(
            &self,
            arg_pad_str: impl AsRef<str>,
            arg_col_width: impl Into<ColWidth>,
        ) -> InlineString {
            let pad_str = arg_pad_str.as_ref();
            let max_display_width: ColWidth = arg_col_width.into();
            let pad_count = max_display_width - self.display_width;
            let self_str = self.string.as_str();

            if pad_count.is_zero() {
                self_str.into()
            } else {
                let mut acc = InlineString::from(self_str);
                pad_fmt!(fmt: acc, pad_str: pad_str, repeat_count: **pad_count);
                acc
            }
        }

        pub fn pad_start_to_fit(
            &self,
            arg_pad_str: impl AsRef<str>,
            arg_col_width: impl Into<ColWidth>,
        ) -> InlineString {
            let pad_str = arg_pad_str.as_ref();
            let max_display_width: ColWidth = arg_col_width.into();
            let pad_count = max_display_width - self.display_width;
            let self_str = self.string.as_str();

            if pad_count.is_zero() {
                self_str.into()
            } else {
                let mut acc = InlineString::new();
                pad_fmt!(fmt: acc, pad_str: pad_str, repeat_count: **pad_count);
                acc.push_str(self_str);
                acc
            }
        }

        /// If `self.string`'s display width is less than `display_width`, this returns a
        /// padding [`InlineString`] consisting of the `pad_str` repeated to make up the
        /// difference. Otherwise, if `self.string` is already as wide or wider than
        /// `display_width`, it returns `None`.
        ///
        /// Here's a visual depiction of the different indices.
        ///
        /// *How it appears in the terminal (displayed)*:
        ///
        /// ```text
        /// R ╭──────────────╮
        /// 0 │Hi📦XelLo🙏🏽Bye│
        ///   ╰──────────────╯
        ///  DC01234567890123 : index (0 based)
        /// ```
        ///
        /// *Detailed breakdown*:
        ///
        /// ```text
        /// DW   1 2 34 5 6 7 8 9 01 234 : width (1 based)
        /// DC   0 1 23 4 5 6 7 8 90 123 : index (0 based)
        ///  R ╭ ─ ─ ── ─ ─ ─ ─ ─ ── ───╮
        ///  0 │ H i 📦 X e l L o 🙏🏽 Bye│
        ///    ╰ ─ ─ ── ─ ─ ─ ─ ─ ── ───╯
        ///   SI 0 1 2  3 4 5 6 7 8  901 : index (0 based)
        ///
        /// ❯ DC: display column index | DW: display width
        /// ❯ R: row index | SI: segment index
        /// ```
        pub fn try_get_postfix_padding_for(
            &self,
            arg_pad_str: impl AsRef<str>,
            arg_col_width: impl Into<ColWidth>,
        ) -> Option<InlineString> {
            // Pad the line to the max cols w/ spaces. This removes any "ghost" carets
            // that were painted in a previous render.
            let pad_str = arg_pad_str.as_ref();
            let max_display_width: ColWidth = arg_col_width.into();

            if self.display_width < max_display_width {
                let pad_count = max_display_width - self.display_width;
                let mut acc = InlineString::new();
                pad_fmt!(fmt: acc, pad_str: pad_str, repeat_count: **pad_count);
                Some(acc)
            } else {
                None
            }
        }
    }
}

/// Methods for easily clipping grapheme cluster segments for common TUI use cases.
mod clip {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl GCStringOwned {
        /// Clip the content starting from `arg_start_at_col_index` and take as many
        /// columns as possible until `arg_col_width` is reached.
        ///
        /// # Arguments
        /// - `arg_start_at_col_index`: This an index value.
        /// - `arg_col_width`: The is not an index value, but a size or count value.
        ///
        /// Here's a visual depiction of the different indices.
        ///
        /// *How it appears in the terminal (displayed)*:
        ///
        /// ```text
        /// R ╭──────────────╮
        /// 0 │Hi📦XelLo🙏🏽Bye│
        ///   ╰──────────────╯
        ///  DC01234567890123 : index (0 based)
        /// ```
        ///
        /// *Detailed breakdown*:
        ///
        /// ```text
        /// DW   1 2 34 5 6 7 8 9 01 234 : width (1 based)
        /// DC   0 1 23 4 5 6 7 8 90 123 : index (0 based)
        ///  R ╭ ─ ─ ── ─ ─ ─ ─ ─ ── ───╮
        ///  0 │ H i 📦 X e l L o 🙏🏽 Bye│
        ///    ╰ ─ ─ ── ─ ─ ─ ─ ─ ── ───╯
        ///   SI 0 1 2  3 4 5 6 7 8  901 : index (0 based)
        ///
        /// ❯ DC: display column index | DW: display width
        /// ❯ R: row index | SI: segment index
        /// ```
        pub fn clip(
            &self,
            arg_start_at_col_index: impl Into<ColIndex>,
            arg_col_width: impl Into<ColWidth>,
        ) -> &str {
            let start_display_col_index: ColIndex = arg_start_at_col_index.into();
            let max_display_col_count: ColWidth = arg_col_width.into();

            let string_start_byte_index = {
                let mut it = 0;
                let mut skip_col_count = start_display_col_index;
                for seg in self.seg_iter() {
                    let seg_display_width = seg.display_width;
                    // Skip scroll_offset_col_index columns.
                    if *skip_col_count == ch(0) {
                        // We are done skipping.
                        break;
                    }

                    // Skip segment.unicode_width.
                    skip_col_count -= seg_display_width;
                    it += seg.bytes_size.as_usize();
                }
                it
            };

            let string_end_byte_index = {
                let mut it = 0;
                let mut avail_col_count = max_display_col_count;
                let mut skip_col_count = start_display_col_index;
                for seg in self.seg_iter() {
                    let seg_display_width = seg.display_width;
                    // Skip scroll_offset_col_index columns (again).
                    if *skip_col_count == ch(0) {
                        if avail_col_count < seg_display_width {
                            break;
                        }
                        it += seg.bytes_size.as_usize();
                        avail_col_count -= seg_display_width;
                    } else {
                        // Skip segment.unicode_width.
                        skip_col_count -= seg_display_width;
                        it += seg.bytes_size.as_usize();
                    }
                }
                it
            };

            &self.string[string_start_byte_index..string_end_byte_index]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{col, width};

    // Helper function to create test strings
    fn create_test_string(s: &str) -> GCStringOwned { GCStringOwned::from(s) }

    // Test module: convert (index conversion operations)
    mod convert_tests {
        use super::*;

        #[test]
        fn test_byte_index_to_seg_index_ascii() {
            let gc_string = create_test_string("hello");

            // Test valid byte indices
            assert_eq!(&gc_string + byte_index(0), Some(seg_index(0))); // 'h'
            assert_eq!(&gc_string + byte_index(1), Some(seg_index(1))); // 'e'
            assert_eq!(&gc_string + byte_index(2), Some(seg_index(2))); // 'l'
            assert_eq!(&gc_string + byte_index(3), Some(seg_index(3))); // 'l'
            assert_eq!(&gc_string + byte_index(4), Some(seg_index(4))); // 'o'

            // Test invalid byte index
            assert_eq!(&gc_string + byte_index(5), None);
        }

        #[test]
        fn test_byte_index_to_seg_index_emoji() {
            let gc_string = create_test_string("a😀b");

            // 'a' at byte 0
            assert_eq!(&gc_string + byte_index(0), Some(seg_index(0)));
            // '😀' starts at byte 1
            assert_eq!(&gc_string + byte_index(1), Some(seg_index(1)));
            // Inside '😀' (invalid)
            assert_eq!(&gc_string + byte_index(2), Some(seg_index(1)));
            assert_eq!(&gc_string + byte_index(3), Some(seg_index(1)));
            assert_eq!(&gc_string + byte_index(4), Some(seg_index(1)));
            // 'b' starts at byte 5
            assert_eq!(&gc_string + byte_index(5), Some(seg_index(2)));

            // Out of bounds
            assert_eq!(&gc_string + byte_index(6), None);
        }

        #[test]
        fn test_col_index_to_seg_index_ascii() {
            let gc_string = create_test_string("hello");

            // Each ASCII char is 1 column wide
            assert_eq!(&gc_string + col(0), Some(seg_index(0))); // 'h'
            assert_eq!(&gc_string + col(1), Some(seg_index(1))); // 'e'
            assert_eq!(&gc_string + col(2), Some(seg_index(2))); // 'l'
            assert_eq!(&gc_string + col(3), Some(seg_index(3))); // 'l'
            assert_eq!(&gc_string + col(4), Some(seg_index(4))); // 'o'

            // Out of bounds
            assert_eq!(&gc_string + col(5), None);
        }

        #[test]
        fn test_col_index_to_seg_index_emoji() {
            let gc_string = create_test_string("a😀b");

            // 'a' at column 0 (width 1)
            assert_eq!(&gc_string + col(0), Some(seg_index(0)));
            // '😀' at columns 1-2 (width 2)
            assert_eq!(&gc_string + col(1), Some(seg_index(1)));
            assert_eq!(&gc_string + col(2), Some(seg_index(1)));
            // 'b' at column 3 (width 1)
            assert_eq!(&gc_string + col(3), Some(seg_index(2)));

            // Out of bounds
            assert_eq!(&gc_string + col(4), None);
        }

        #[test]
        fn test_seg_index_to_col_index_ascii() {
            let gc_string = create_test_string("hello");

            assert_eq!(&gc_string + seg_index(0), Some(col(0))); // 'h'
            assert_eq!(&gc_string + seg_index(1), Some(col(1))); // 'e'
            assert_eq!(&gc_string + seg_index(2), Some(col(2))); // 'l'
            assert_eq!(&gc_string + seg_index(3), Some(col(3))); // 'l'
            assert_eq!(&gc_string + seg_index(4), Some(col(4))); // 'o'

            // Out of bounds
            assert_eq!(&gc_string + seg_index(5), None);
        }

        #[test]
        fn test_seg_index_to_col_index_emoji() {
            let gc_string = create_test_string("a😀b");

            assert_eq!(&gc_string + seg_index(0), Some(col(0))); // 'a' at col 0
            assert_eq!(&gc_string + seg_index(1), Some(col(1))); // '😀' starts at col 1
            assert_eq!(&gc_string + seg_index(2), Some(col(3))); // 'b' at col 3

            // Out of bounds
            assert_eq!(&gc_string + seg_index(3), None);
        }

        #[test]
        fn test_empty_string_conversions() {
            let gc_string = create_test_string("");

            // All conversions should return None for empty string
            assert_eq!(&gc_string + byte_index(0), None);
            assert_eq!(&gc_string + col(0), None);
            assert_eq!(&gc_string + seg_index(0), None);
        }

        #[test]
        fn test_complex_unicode_conversions() {
            let gc_string = create_test_string("🙏🏽");

            // This is a complex emoji (8 bytes, 2 columns, 1 segment)
            assert_eq!(&gc_string + byte_index(0), Some(seg_index(0)));
            // All byte positions within the emoji should map to same segment
            for i in 1..8 {
                assert_eq!(&gc_string + byte_index(i), Some(seg_index(0)));
            }
            assert_eq!(&gc_string + byte_index(8), None); // Out of bounds

            // Both columns should map to the same segment
            assert_eq!(&gc_string + col(0), Some(seg_index(0)));
            assert_eq!(&gc_string + col(1), Some(seg_index(0)));
            assert_eq!(&gc_string + col(2), None); // Out of bounds

            // Segment 0 should start at column 0
            assert_eq!(&gc_string + seg_index(0), Some(col(0)));
            assert_eq!(&gc_string + seg_index(1), None); // Out of bounds
        }
    }

    // Test module: trunc_end (text truncation from end)
    mod trunc_end_tests {
        use super::*;

        #[test]
        fn test_trunc_end_to_fit_ascii() {
            let gc_string = create_test_string("hello world");

            // Truncate to fit within 5 columns
            assert_eq!(gc_string.trunc_end_to_fit(width(5)), "hello");

            // Truncate to fit within 11 columns (exact fit)
            assert_eq!(gc_string.trunc_end_to_fit(width(11)), "hello world");

            // Truncate to fit within 15 columns (no truncation needed)
            assert_eq!(gc_string.trunc_end_to_fit(width(15)), "hello world");

            // Truncate to fit within 0 columns
            assert_eq!(gc_string.trunc_end_to_fit(width(0)), "");
        }

        #[test]
        fn test_trunc_end_to_fit_emoji() {
            let gc_string = create_test_string("a😀b😎c");
            // Display: a😀b😎c (1+2+1+2+1 = 7 columns)

            assert_eq!(gc_string.trunc_end_to_fit(width(3)), "a😀"); // Stops at emoji boundary
            assert_eq!(gc_string.trunc_end_to_fit(width(4)), "a😀b"); // Includes single char
            assert_eq!(gc_string.trunc_end_to_fit(width(6)), "a😀b😎"); // Stops before final char
            assert_eq!(gc_string.trunc_end_to_fit(width(7)), "a😀b😎c"); // Full string
        }

        #[test]
        fn test_trunc_end_by_ascii() {
            let gc_string = create_test_string("hello world");

            // Remove 0 columns (no truncation) - special case
            assert_eq!(gc_string.trunc_end_by(width(0)), "hello worl");

            // Remove 1 column from end ('d')
            assert_eq!(gc_string.trunc_end_by(width(1)), "hello worl");

            // Remove 5 columns from end ("world")
            assert_eq!(gc_string.trunc_end_by(width(5)), "hello ");

            // Remove 6 columns from end (" world")
            assert_eq!(gc_string.trunc_end_by(width(6)), "hello");

            // Remove all columns
            assert_eq!(gc_string.trunc_end_by(width(11)), "");

            // Remove more than available (should return empty)
            assert_eq!(gc_string.trunc_end_by(width(15)), "");
        }

        #[test]
        fn test_trunc_end_by_emoji() {
            let gc_string = create_test_string("a😀b😎c");
            // Display: a😀b😎c (1+2+1+2+1 = 7 columns)

            // Remove 1 column from end ('c')
            assert_eq!(gc_string.trunc_end_by(width(1)), "a😀b😎");

            // Remove 3 columns from end ('c' + '😎')
            assert_eq!(gc_string.trunc_end_by(width(3)), "a😀b");

            // Remove 4 columns from end
            assert_eq!(gc_string.trunc_end_by(width(4)), "a😀");

            // Remove all columns
            assert_eq!(gc_string.trunc_end_by(width(7)), "");
        }

        #[test]
        fn test_trunc_end_empty_string() {
            let gc_string = create_test_string("");

            assert_eq!(gc_string.trunc_end_to_fit(width(5)), "");
            assert_eq!(gc_string.trunc_end_by(width(5)), "");
        }
    }

    // Test module: trunc_start (text truncation from start)
    mod trunc_start_tests {
        use super::*;

        #[test]
        fn test_trunc_start_by_ascii() {
            let gc_string = create_test_string("hello world");

            // Skip 0 columns (no truncation)
            assert_eq!(gc_string.trunc_start_by(width(0)), "hello world");

            // Skip 5 columns from start
            assert_eq!(gc_string.trunc_start_by(width(5)), " world");

            // Skip 6 columns from start
            assert_eq!(gc_string.trunc_start_by(width(6)), "world");

            // Skip all columns
            assert_eq!(gc_string.trunc_start_by(width(11)), "");

            // Skip more than available
            assert_eq!(gc_string.trunc_start_by(width(15)), "");
        }

        #[test]
        fn test_trunc_start_by_emoji() {
            let gc_string = create_test_string("a😀b😎c");
            // Display: a😀b😎c (1+2+1+2+1 = 7 columns)

            // Skip 1 column ('a')
            assert_eq!(gc_string.trunc_start_by(width(1)), "😀b😎c");

            // Skip 3 columns ('a' + '😀')
            assert_eq!(gc_string.trunc_start_by(width(3)), "b😎c");

            // Skip 4 columns ('a' + '😀' + 'b')
            assert_eq!(gc_string.trunc_start_by(width(4)), "😎c");

            // Skip 6 columns ('a' + '😀' + 'b' + '😎')
            assert_eq!(gc_string.trunc_start_by(width(6)), "c");

            // Skip all columns
            assert_eq!(gc_string.trunc_start_by(width(7)), "");
        }

        #[test]
        fn test_trunc_start_empty_string() {
            let gc_string = create_test_string("");

            assert_eq!(gc_string.trunc_start_by(width(0)), "");
            assert_eq!(gc_string.trunc_start_by(width(5)), "");
        }
    }

    // Test module: pad (string padding operations)
    mod pad_tests {
        use super::*;

        #[test]
        fn test_pad_end_to_fit_no_padding_needed() {
            let gc_string = create_test_string("hello");

            // String is already 5 columns, no padding needed
            let result = gc_string.pad_end_to_fit(" ", width(5));
            assert_eq!(result.as_str(), "hello");
        }

        #[test]
        fn test_pad_end_to_fit_with_padding() {
            let gc_string = create_test_string("hi");

            // Pad to 5 columns with spaces
            let result = gc_string.pad_end_to_fit(" ", width(5));
            assert_eq!(result.as_str(), "hi   ");

            // Pad to 5 columns with dots
            let result = gc_string.pad_end_to_fit(".", width(5));
            assert_eq!(result.as_str(), "hi...");
        }

        #[test]
        fn test_pad_start_to_fit_no_padding_needed() {
            let gc_string = create_test_string("hello");

            // String is already 5 columns, no padding needed
            let result = gc_string.pad_start_to_fit(" ", width(5));
            assert_eq!(result.as_str(), "hello");
        }

        #[test]
        fn test_pad_start_to_fit_with_padding() {
            let gc_string = create_test_string("hi");

            // Pad to 5 columns with spaces
            let result = gc_string.pad_start_to_fit(" ", width(5));
            assert_eq!(result.as_str(), "   hi");

            // Pad to 5 columns with dots
            let result = gc_string.pad_start_to_fit(".", width(5));
            assert_eq!(result.as_str(), "...hi");
        }

        #[test]
        fn test_pad_emoji_strings() {
            let gc_string = create_test_string("😀"); // 2 columns wide

            // Pad end to 5 columns
            let result = gc_string.pad_end_to_fit(" ", width(5));
            assert_eq!(result.as_str(), "😀   ");

            // Pad start to 5 columns
            let result = gc_string.pad_start_to_fit(" ", width(5));
            assert_eq!(result.as_str(), "   😀");
        }

        #[test]
        fn test_try_get_postfix_padding_for_none() {
            let gc_string = create_test_string("hello");

            // String is already 5 columns, no padding needed
            assert_eq!(gc_string.try_get_postfix_padding_for(" ", width(5)), None);

            // String is wider than requested width
            assert_eq!(gc_string.try_get_postfix_padding_for(" ", width(3)), None);
        }

        #[test]
        fn test_try_get_postfix_padding_for_some() {
            let gc_string = create_test_string("hi");

            // Need 3 spaces to pad to 5 columns
            let result = gc_string.try_get_postfix_padding_for(" ", width(5));
            assert!(result.is_some());
            assert_eq!(result.unwrap().as_str(), "   ");

            // Need 3 dots to pad to 5 columns
            let result = gc_string.try_get_postfix_padding_for(".", width(5));
            assert!(result.is_some());
            assert_eq!(result.unwrap().as_str(), "...");
        }

        #[test]
        fn test_pad_empty_string() {
            let gc_string = create_test_string("");

            // Pad empty string to 3 columns
            let result = gc_string.pad_end_to_fit(" ", width(3));
            assert_eq!(result.as_str(), "   ");

            let result = gc_string.pad_start_to_fit(" ", width(3));
            assert_eq!(result.as_str(), "   ");

            let result = gc_string.try_get_postfix_padding_for(" ", width(3));
            assert!(result.is_some());
            assert_eq!(result.unwrap().as_str(), "   ");
        }
    }

    // Test module: clip (text clipping operations)
    mod clip_tests {
        use super::*;

        #[test]
        fn test_clip_ascii_from_start() {
            let gc_string = create_test_string("hello world");

            // Clip from start, take 5 columns
            assert_eq!(gc_string.clip(col(0), width(5)), "hello");

            // Clip from start, take all columns
            assert_eq!(gc_string.clip(col(0), width(11)), "hello world");

            // Clip from start, take more than available
            assert_eq!(gc_string.clip(col(0), width(15)), "hello world");
        }

        #[test]
        fn test_clip_ascii_from_middle() {
            let gc_string = create_test_string("hello world");

            // Clip starting at column 6, take 5 columns
            assert_eq!(gc_string.clip(col(6), width(5)), "world");

            // Clip starting at column 3, take 4 columns
            assert_eq!(gc_string.clip(col(3), width(4)), "lo w");

            // Clip starting at column 6, take 3 columns
            assert_eq!(gc_string.clip(col(6), width(3)), "wor");
        }

        #[test]
        fn test_clip_emoji_strings() {
            let gc_string = create_test_string("a😀b😎c");
            // Display: a😀b😎c (1+2+1+2+1 = 7 columns)

            // Clip from start, take 3 columns
            assert_eq!(gc_string.clip(col(0), width(3)), "a😀");

            // Clip from column 1 (start of emoji), take 3 columns
            assert_eq!(gc_string.clip(col(1), width(3)), "😀b");

            // Clip from column 3, take 3 columns
            assert_eq!(gc_string.clip(col(3), width(3)), "b😎");

            // Clip from column 4 (middle of second emoji), take 2 columns
            assert_eq!(gc_string.clip(col(4), width(2)), "😎");

            // Clip from column 6, take 2 columns (only 1 available)
            assert_eq!(gc_string.clip(col(6), width(2)), "c");
        }

        #[test]
        fn test_clip_zero_width() {
            let gc_string = create_test_string("hello");

            // Clip with zero width should return empty string
            assert_eq!(gc_string.clip(col(0), width(0)), "");
            assert_eq!(gc_string.clip(col(2), width(0)), "");
        }

        #[test]
        fn test_clip_beyond_string_bounds() {
            let gc_string = create_test_string("hello");

            // Start clipping beyond string end
            assert_eq!(gc_string.clip(col(10), width(5)), "");

            // Start clipping at string end
            assert_eq!(gc_string.clip(col(5), width(5)), "");
        }

        #[test]
        fn test_clip_empty_string() {
            let gc_string = create_test_string("");

            assert_eq!(gc_string.clip(col(0), width(5)), "");
            assert_eq!(gc_string.clip(col(3), width(2)), "");
        }

        #[test]
        fn test_clip_complex_unicode() {
            let gc_string = create_test_string("🙏🏽👨‍👩‍👧‍👦");
            // Complex emojis with varying widths

            // Should handle complex unicode correctly
            let result = gc_string.clip(col(0), width(2));
            assert_eq!(result, "🙏🏽");
        }
    }

    // Integration tests combining multiple operations
    mod integration_tests {
        use super::*;

        #[test]
        fn test_convert_and_clip_consistency() {
            let gc_string = create_test_string("hello world");

            // Find segment at column 6
            let seg_opt = &gc_string + col(6);
            assert!(seg_opt.is_some());
            let seg_idx = seg_opt.unwrap();

            // Convert back to column
            let col_opt = &gc_string + seg_idx;
            assert!(col_opt.is_some());
            let col_idx = col_opt.unwrap();

            // Should be at column 6 (start of 'w' in "world")
            assert_eq!(col_idx, col(6));

            // Clipping from that position should give expected result
            assert_eq!(gc_string.clip(col_idx, width(5)), "world");
        }

        #[test]
        fn test_pad_and_clip_combination() {
            let gc_string = create_test_string("hi");

            // Pad to 10 columns
            let padded = gc_string.pad_end_to_fit(" ", width(10));
            let padded_gc = create_test_string(padded.as_str());

            // Clip first 5 columns
            assert_eq!(padded_gc.clip(col(0), width(5)), "hi   ");

            // Clip last 5 columns
            assert_eq!(padded_gc.clip(col(5), width(5)), "     ");
        }

        #[test]
        fn test_truncate_and_convert() {
            let gc_string = create_test_string("hello world test");

            // Truncate to fit 10 columns
            let truncated_str = gc_string.trunc_end_to_fit(width(10));
            let truncated_gc = create_test_string(truncated_str);

            // Should be "hello worl"
            assert_eq!(truncated_str, "hello worl");

            // Test conversion on truncated string
            assert_eq!(&truncated_gc + col(0), Some(seg_index(0))); // 'h'
            assert_eq!(&truncated_gc + col(9), Some(seg_index(9))); // 'l'
            assert_eq!(&truncated_gc + col(10), None); // Beyond end
        }
    }
}
