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
use crate::{ByteIndex, ColIndex, ColWidth, InlineString, SegIndex, ch, pad_fmt,
            seg_index, usize, width};

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
    use super::{Add, ByteIndex, ColIndex, GCStringOwned, SegIndex, seg_index, usize};

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
    use super::{ColWidth, GCStringOwned, ch, usize};

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
            let mut string_end_byte_index = ch(0);

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
    use super::{ColWidth, GCStringOwned, ch};

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
    use super::{ColWidth, GCStringOwned, InlineString, pad_fmt, width};

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

            if pad_count > width(0) {
                let mut acc = InlineString::from(self_str);
                pad_fmt!(fmt: acc, pad_str: pad_str, repeat_count: **pad_count);
                acc
            } else {
                self_str.into()
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

            if pad_count > width(0) {
                let mut acc = InlineString::new();
                pad_fmt!(fmt: acc, pad_str: pad_str, repeat_count: **pad_count);
                acc.push_str(self_str);
                acc
            } else {
                self_str.into()
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
    use super::{ColIndex, ColWidth, GCStringOwned, ch};

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
