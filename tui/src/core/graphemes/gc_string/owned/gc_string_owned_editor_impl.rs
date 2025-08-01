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

//! Editor-specific implementation modules for [`GCStringOwned`].
//!
//! This module contains [`GCStringOwned`] functionality that is used exclusively by
//! editor operations. These modules provide text mutation, cursor validation, and
//! string slicing operations used specifically for text editing workflows.
//!
//! # Migration Notice
//!
//! These modules are **candidates for deprecation** as part of the `ZeroCopyGapBuffer`
//! migration (Phase 4.3). Their functionality should be migrated to:
//! - `ZeroCopyGapBuffer` methods for text mutation operations
//! - `LineMetadata` methods for cursor validation and string slicing
//!
//! The modules here are:
//! - `mutate`: Text insertion, deletion, and line splitting operations
//! - `at_display_col_index`: Cursor position validation and string slicing by column
//!
//! These are kept separate from general TUI operations to clearly identify code
//! that needs to be migrated during the [`crate::ZeroCopyGapBuffer`] transition.

use super::{GCStringOwned, SegStringOwned};
use crate::{ColIndex, ColWidth, InlineString, InlineVecStr, Seg, ch, join, seg_index,
            seg_width, usize, width};

/// Methods to make it easy to work with getting owned string (from slices) at a given
/// display col index.
///
/// **Migration Notice**: This module is a candidate for deprecation during the
/// `ZeroCopyGapBuffer` migration (Phase 4.3). Its functionality should be migrated to
/// `LineMetadata` methods for cursor validation and string slicing.
pub mod at_display_col_index {
    use super::{ColIndex, GCStringOwned, Seg, SegStringOwned, ch, seg_index};

    impl GCStringOwned {
        /// If the given `display_col_index` falls in the middle of a grapheme cluster,
        /// then return the [Seg] at that `display_col_index`. Otherwise return [None].
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
        pub fn check_is_in_middle_of_grapheme(
            &self,
            arg_col_index: impl Into<ColIndex>,
        ) -> Option<Seg> {
            let col: ColIndex = arg_col_index.into();
            let seg_index_at_col = (self + col)?;
            let seg = self.get(seg_index_at_col)?;
            if col != seg.start_display_col_index {
                return Some(seg);
            }
            None
        }

        /// Return the string and display width of the grapheme cluster segment at the
        /// given `display_col_index`. If this `display_col_index` falls in the middle of
        /// a grapheme cluster, then return [None].
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
        pub fn get_string_at(
            &self,
            arg_col_index: impl Into<ColIndex>,
        ) -> Option<SegStringOwned> {
            // Convert display_col_index to seg_index.
            let col: ColIndex = arg_col_index.into();
            let seg_index_at_col = (self + col)?;

            // Get the segment at seg_index.
            let seg = self.get(seg_index_at_col)?;
            let seg_start_at = seg.start_display_col_index;
            (col == seg_start_at).then(|| {
                // The display_col_index is at the start of a grapheme cluster 👍.
                (seg, self).into()
            })
        }

        /// Return the string at the right of the given `display_col_index`. If the
        /// `display_col_index` is at the end of the string, then return [None]. If the
        /// `display_col_index` is in the middle of a grapheme cluster, then return the
        /// grapheme cluster segment that includes that `display_col_index`.
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
        pub fn get_string_at_right_of(
            &self,
            arg_col_index: impl Into<ColIndex>,
        ) -> Option<SegStringOwned> {
            let col: ColIndex = arg_col_index.into();
            let seg_index_at_col = (self + col)?;
            let seg = self.get(seg_index_at_col)?;
            (seg.seg_index < self.get_max_seg_index()).then(|| {
                let right_neighbor_seg = self.get(*seg.seg_index + ch(1))?;
                Some((right_neighbor_seg, self).into())
            })?
        }

        /// Return the string at the left of the given `display_col_index`. If the
        /// `display_col_index` is at the start of the string, or past the end of the
        /// string, then return [None]. If the `display_col_index` is in the middle of a
        /// grapheme cluster, then return the grapheme cluster segment that includes that
        /// `display_col_index`.
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
        pub fn get_string_at_left_of(
            &self,
            arg_col_index: impl Into<ColIndex>,
        ) -> Option<SegStringOwned> {
            let col: ColIndex = arg_col_index.into();
            let seg_index_at_col = (self + col)?;
            let seg = self.get(seg_index_at_col)?;
            (seg.seg_index > seg_index(0)).then(|| {
                let left_neighbor_seg = self.get(*seg.seg_index - ch(1))?;
                Some((left_neighbor_seg, self).into())
            })?
        }

        /// Return the last grapheme cluster segment in the grapheme string.
        /// If the grapheme string is empty, then return [None].
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
        #[must_use]
        pub fn get_string_at_end(&self) -> Option<SegStringOwned> {
            let seg = self.last()?;
            Some((seg, self).into())
        }
    }
}

/// Methods for easily modifying grapheme cluster segments for common TUI use cases.
///
/// **Migration Notice**: This module is a candidate for deprecation during the
/// `ZeroCopyGapBuffer` migration (Phase 4.3). Its functionality should be migrated to
/// `ZeroCopyGapBuffer` methods for text mutation operations.
pub mod mutate {
    use super::{ColIndex, ColWidth, GCStringOwned, InlineString, InlineVecStr, ch, join,
                seg_width, usize, width};

    impl GCStringOwned {
        /// Inserts the given `chunk` in the correct position of the `string`, and returns
        /// a new ([`InlineString`], [`ColWidth`]) tuple:
        /// 1. The new [`InlineString`] produced containing the inserted chunk.
        /// 2. The unicode width / display width of the inserted `chunk`.
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
        pub fn insert_chunk_at_col(
            &self,
            arg_col_index: impl Into<ColIndex>,
            arg_chunk: impl AsRef<str>,
        ) -> (InlineString, ColWidth) {
            let chunk = arg_chunk.as_ref();

            // Create an array-vec of &str from self.vec_segment, using self.iter().
            let mut vec = InlineVecStr::with_capacity(self.len().as_usize() + 1);
            // Add each seg's &str to the acc.
            vec.extend(
                // Turn self.segments into a list of &str.
                self.seg_iter().map(|seg| seg.get_str(&self.string)),
            );

            // Get seg_index at display_col_index.
            let col: ColIndex = arg_col_index.into();
            let seg_index_at_col = self + col;

            match seg_index_at_col {
                // Insert somewhere inside bounds of self.string.
                Some(seg_index) => vec.insert(usize(*seg_index), chunk),
                // Add to end of self.string.
                None => vec.push(chunk),
            }

            // Generate a new InlineString from acc and return it and the unicode width of
            // the character.
            (
                join!(from: vec, each: item, delim: "", format: "{item}"),
                GCStringOwned::new(chunk).width(),
            )
        }

        /// Returns a new [`InlineString`] that is the result of deleting the character at
        /// the given `display_col_index`.
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
        pub fn delete_char_at_col(
            &self,
            arg_col_index: impl Into<ColIndex>,
        ) -> Option<InlineString> {
            // There is no segment present (Deref trait makes `len()` apply to
            // `vec_segment`).
            if self.is_empty() {
                return None;
            }

            // There is only one segment present.
            if self.len() == seg_width(1) {
                return Some("".into());
            }

            // There are more than 1 segments present.

            // Get seg_index at display_col_index.
            let col: ColIndex = arg_col_index.into();
            let split_seg_index = (self + col)?;
            let split_seg_index = usize(*split_seg_index);

            let mut vec_left = InlineVecStr::with_capacity(self.len().as_usize());
            let mut str_left_display_width = width(0);
            {
                for seg_index in 0..split_seg_index {
                    let seg = *self.segments.get(seg_index)?;
                    let string = seg.get_str(&self.string);
                    vec_left.push(string);
                    str_left_display_width += seg.display_width;
                }
            }

            let mut vec_right = InlineVecStr::with_capacity(self.len().as_usize());
            let mut str_right_display_width = width(0);
            {
                // Drop one segment.
                let max_seg_index = self.len();
                for seg_index in (split_seg_index + 1)..max_seg_index.as_usize() {
                    let seg = *self.segments.get(seg_index)?;
                    let string = seg.get_str(&self.string);
                    vec_right.push(string);
                    str_right_display_width += seg.display_width;
                }
            }

            // Merge the two vectors.
            vec_left.append(&mut vec_right);
            Some(join!(from: vec_left, each: it, delim: "", format: "{it}"))
        }

        /// Splits the string at the given `display_col_index` and returns a tuple of the
        /// left and right parts of the split. If the `display_col_index` falls in the
        /// middle of a grapheme cluster, then the split is done at the start of the
        /// cluster.
        ///
        /// Returns two new tuples:
        /// 1. *left* [`InlineString`],
        /// 2. *right* [`InlineString`].
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
        pub fn split_at_display_col(
            &self,
            arg_col_index: impl Into<ColIndex>,
        ) -> Option<(InlineString, InlineString)> {
            // Get seg_index at display_col_index.
            let col: ColIndex = arg_col_index.into();
            let split_seg_index = (self + col)?;
            let split_seg_index = usize(*split_seg_index);

            let mut acc_left = InlineVecStr::with_capacity(self.len().as_usize());
            let mut str_left_display_width = width(0);
            {
                for seg_index in 0..split_seg_index {
                    let seg = *self.segments.get(seg_index)?;
                    acc_left.push(seg.get_str(&self.string));
                    str_left_display_width += seg.display_width;
                }
            }

            let mut acc_right = InlineVecStr::with_capacity(self.len().as_usize());
            let mut str_right_unicode_width = width(0);
            {
                let max_seg_index = self.len();
                for seg_idx in split_seg_index..max_seg_index.as_usize() {
                    let seg = *self.segments.get(seg_idx)?;
                    acc_right.push(seg.get_str(&self.string));
                    str_right_unicode_width += seg.display_width;
                }
            }

            (*str_right_unicode_width > ch(0) || *str_left_display_width > ch(0)).then(
                || {
                    (
                        join!(from: acc_left, each: it, delim: "", format: "{it}"),
                        join!(from: acc_right, each: it, delim: "", format: "{it}"),
                    )
                },
            )
        }
    }
}
