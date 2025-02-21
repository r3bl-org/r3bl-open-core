/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

use crate::{ColIndex,
            ColWidth,
            GraphemeClusterSegment,
            Size,
            StringStorage,
            UnicodeString,
            UnicodeStringSegmentSliceResult,
            ch,
            pad_fmt,
            usize};

impl UnicodeString {
    /// If any segment in `self.vec_segment` has a `display_col_offset` greater than 1
    /// then this is true. The semantic is that the string is displayed using more than 1
    /// column of the terminal.
    pub fn contains_wide_segments(&self) -> bool {
        let mut contains_wide_segments = false;
        for seg in self.iter() {
            if *seg.unicode_width > ch(1) {
                contains_wide_segments = true;
                break;
            }
        }
        contains_wide_segments
    }

    /// The `size` is a column index and row index. Not width or height.
    /// - To convert width -> size / column index subtract 1.
    /// - To convert size / column index to width add 1.
    ///
    /// Note the [Self::truncate_end_by_n_col] and [Self::truncate_start_by_n_col]
    /// functions take a width.
    pub fn truncate_to_fit_size(&self, size: Size) -> &str {
        self.truncate_end_to_fit_width(size.col_width)
    }

    /// The `n_col` is a width, not a [Size].
    /// - To convert width -> size / column index subtract 1.
    /// - To convert size / column index to width add 1.
    ///
    /// Note the [Self::truncate_to_fit_size] function takes a size / column index.
    pub fn truncate_end_by_n_col(&self, n_col: ColWidth) -> &str {
        let mut countdown_col_count = n_col;
        let mut string_end_byte_index = ch(0);

        for seg in self.iter().rev() {
            let seg_display_width = seg.unicode_width;
            string_end_byte_index = seg.start_byte_index;
            countdown_col_count -= seg_display_width;
            if *countdown_col_count == ch(0) {
                // We are done skipping.
                break;
            }
        }

        &self.string[..usize(string_end_byte_index)]
    }

    /// Removes segments from the start of the string so that `col_count` (width) is
    /// skipped.
    ///
    /// ```rust
    /// use r3bl_core::{UnicodeString, UnicodeStringExt, width};
    ///
    /// let col_count = width(2);
    /// let display_cols = width(5);
    ///
    /// let expected_clipped_string = "rst s";
    ///
    /// let line = "first second";
    /// let line_us = line.unicode_string();
    ///
    /// let truncated_line = line_us.truncate_start_by_n_col(col_count);
    /// let truncated_line_us = truncated_line.unicode_string();
    ///
    /// let truncated_line = truncated_line_us.truncate_end_to_fit_width(display_cols);
    ///
    /// assert_eq!(truncated_line, expected_clipped_string);
    /// ```
    pub fn truncate_start_by_n_col(&self, n_col: ColWidth) -> &str {
        let mut skip_col_count = n_col;
        let mut string_start_byte_index = 0;

        for segment in self.iter() {
            let seg_display_width = segment.unicode_width;
            if *skip_col_count != ch(0) {
                // Skip segment.unicode_width.
                skip_col_count -= seg_display_width;
                string_start_byte_index += segment.byte_size;
            } else {
                // We are done skipping.
                break;
            }
        }

        &self.string[string_start_byte_index..]
    }

    /// Returns a string slice from `self.string` w/ the segments removed from the end of
    /// the string that don't fit in the given viewport width (which is 1 based, and not 0
    /// based). Note that the character at `display_col_count` *index* is NOT included in
    /// the result; please see the example below.
    ///
    /// ```text
    ///   ⎛ 3 ⎫ : size (or "width" or "col count" or "count", 1 based)
    /// R ┌───┐
    /// 0 │fir│st second
    ///   └───┘
    ///   C012 345678901 : index (0 based)
    /// ```
    ///
    /// Example.
    /// ```rust
    /// use r3bl_core::{UnicodeString, width, UnicodeStringExt};
    ///
    /// let scroll_offset_col = width(0);
    /// let display_cols = width(3);
    /// let expected_clipped_string = "fir";
    ///
    /// let line = "first second";
    /// let line_us = line.unicode_string();
    ///
    /// let truncated_line = line_us.truncate_start_by_n_col(scroll_offset_col);
    /// let truncated_line_us = truncated_line.unicode_string();
    ///
    /// let truncated_line_2 = truncated_line_us.truncate_end_to_fit_width(display_cols);
    ///
    /// assert_eq!(truncated_line_2, expected_clipped_string);
    /// ```
    pub fn truncate_end_to_fit_width(&self, display_width: ColWidth) -> &str {
        let mut avail_cols = display_width;
        let mut string_end_byte_index = 0;

        for seg in self.iter() {
            let seg_display_width = seg.unicode_width;
            if avail_cols < seg_display_width {
                break;
            }
            string_end_byte_index += seg.byte_size;
            avail_cols -= seg_display_width;
        }

        &self.string[..string_end_byte_index]
    }

    /// Returns a new [StringStorage] that is the result of padding `self.string` to fit
    /// the given width w/ the given spacer character.
    pub fn pad_end_with_spaces_to_fit_width(
        &self,
        chunk: &str,
        spacer: impl AsRef<str>,
        max_display_width: ColWidth,
    ) -> StringStorage {
        let pad_len = max_display_width - self.display_width;
        if *pad_len > ch(0) {
            let mut acc = StringStorage::from(chunk);
            pad_fmt!(fmt: acc, pad_str: spacer.as_ref(), repeat_count: usize(*pad_len));
            acc
        } else {
            chunk.into()
        }
    }

    /// Clip the content starting from `start_display_col_index` and take as many columns
    /// as possible until `max_display_col_count` is reached.
    ///
    /// # Arguments
    /// - `start_display_col_index`: This an index value.
    /// - `max_display_col_count`: The is not an index value, but a size or count value.
    pub fn clip_to_width(
        &self,
        /* index */ start_display_col_index: ColIndex,
        /* width */ max_display_col_count: ColWidth,
    ) -> &str {
        let string_start_byte_index = {
            let mut it = 0;
            let mut skip_col_count = start_display_col_index;
            for seg in self.iter() {
                let seg_display_width = seg.unicode_width;
                // Skip scroll_offset_col_index columns.
                if *skip_col_count != ch(0) {
                    // Skip segment.unicode_width.
                    skip_col_count -= seg_display_width;
                    it += seg.byte_size;
                } else {
                    // We are done skipping.
                    break;
                }
            }
            it
        };

        let string_end_byte_index = {
            let mut it = 0;
            let mut avail_col_count = max_display_col_count;
            let mut skip_col_count = start_display_col_index;
            for seg in self.iter() {
                let seg_display_width = seg.unicode_width;
                // Skip scroll_offset_col_index columns (again).
                if *skip_col_count != ch(0) {
                    // Skip segment.unicode_width.
                    skip_col_count -= seg_display_width;
                    it += seg.byte_size;
                }
                // Clip max_display_col_count columns.
                else {
                    if avail_col_count < seg_display_width {
                        break;
                    }
                    it += seg.byte_size;
                    avail_col_count -= seg_display_width;
                }
            }
            it
        };

        &self.string[string_start_byte_index..string_end_byte_index]
    }

    /// If `self.string`'s display width is less than `max_display_width`, this returns a
    /// padding string consisting of the `pad_char` repeated to make up the difference.
    /// Otherwise, if `self.string` is already as wide or wider than `max_display_width`,
    /// it returns `None`.
    pub fn try_get_postfix_padding_for(
        &self,
        chunk: &str,
        pad_char: impl AsRef<str>,
        max_display_width: ColWidth,
    ) -> Option<StringStorage> {
        // Pad the line to the max cols w/ spaces. This removes any "ghost" carets that
        // were painted in a previous render.
        let chunk_display_width = UnicodeString::str_display_width(chunk);
        if chunk_display_width < max_display_width {
            let pad_count = {
                let it = max_display_width - chunk_display_width;
                usize(*it)
            };
            let mut acc = StringStorage::new();
            pad_fmt!(fmt: acc, pad_str: pad_char.as_ref(), repeat_count: pad_count);
            Some(acc)
        } else {
            None
        }
    }

    /// `local_index` is the index of the grapheme cluster in the `vec_segment`.
    pub fn at_logical_index(
        &self,
        logical_index: usize,
    ) -> Option<&GraphemeClusterSegment> {
        self.get(logical_index)
    }

    /// `display_col_index` is the col index in the terminal where this grapheme cluster can be
    /// displayed.
    pub fn at_display_col_index(
        &self,
        display_col_index: ColIndex,
    ) -> Option<&GraphemeClusterSegment> {
        self.iter().find(|&seg| {
            let seg_display_width = seg.unicode_width;
            let seg_start = seg.start_display_col_index;
            let seg_end = seg_start + seg_display_width;
            /* is within segment */
            display_col_index >= seg_start && display_col_index < seg_end
        })
    }

    /// Convert a `display_col_index` to a `logical_index`.
    /// - `local_index` is the index of the grapheme cluster in the `vec_segment`.
    /// - `display_col_index` is the col index in the terminal where this grapheme cluster can
    ///   be displayed.
    pub fn logical_index_at_display_col_index(
        &self,
        display_col_index: ColIndex,
    ) -> Option<usize> {
        self.at_display_col_index(display_col_index)
            .map(|segment| usize(segment.logical_index))
    }

    /// Convert a `logical_index` to a `display_col_index`.
    /// - `local_index` is the index of the grapheme cluster in the `vec_segment`.
    /// - `display_col_index` is the col index in the terminal where this grapheme cluster can
    ///   be displayed.
    pub fn display_col_index_at_logical_index(
        &self,
        logical_index: usize,
    ) -> Option<ColIndex> {
        self.at_logical_index(logical_index)
            .map(|segment| segment.start_display_col_index)
    }

    /// Return the string and unicode width of the grapheme cluster segment at the given
    /// `display_col_index`. If this `display_col_index` falls in the middle of a grapheme cluster,
    /// then return [None].
    pub fn get_string_at_display_col_index(
        &self,
        display_col_index: ColIndex,
    ) -> Option<UnicodeStringSegmentSliceResult> {
        let seg: &GraphemeClusterSegment =
            self.at_display_col_index(display_col_index)?;
        let seg_string = seg.get_str(&self.string);
        let seg_display_width = seg.unicode_width;

        if display_col_index != seg.start_display_col_index {
            // The display_col_index is in the middle of a grapheme cluster 👎.
            None
        } else {
            // The display_col_index is at the start of a grapheme cluster 👍.
            Some(UnicodeStringSegmentSliceResult::new(
                seg_string,
                seg_display_width,
                seg.start_display_col_index,
            ))
        }
    }

    /// If the given `display_col_index` falls in the middle of a grapheme cluster, then return
    /// the [GraphemeClusterSegment] at that `display_col_index`. Otherwise return [None].
    pub fn is_display_col_index_in_middle_of_grapheme_cluster(
        &self,
        display_col_index: ColIndex,
    ) -> Option<GraphemeClusterSegment> {
        let seg = self.at_display_col_index(display_col_index);

        if let Some(segment) = seg {
            if display_col_index != segment.start_display_col_index {
                return Some(*segment);
            }
        }

        None
    }

    pub fn get_string_at_right_of_display_col_index(
        &self,
        display_col_index: ColIndex,
    ) -> Option<UnicodeStringSegmentSliceResult> {
        let seg_at_col = self.at_display_col_index(display_col_index)?;

        if seg_at_col.logical_index < ch(self.len()) - ch(1) {
            let seg_right_of_col =
                self.at_logical_index(usize(seg_at_col.logical_index + 1))?;
            Some(UnicodeStringSegmentSliceResult::new(
                seg_right_of_col.get_str(&self.string),
                seg_right_of_col.unicode_width,
                seg_right_of_col.start_display_col_index,
            ))
        } else {
            None
        }
    }

    pub fn get_string_at_left_of_display_col_index(
        &self,
        display_col_index: ColIndex,
    ) -> Option<UnicodeStringSegmentSliceResult> {
        let seg_at_col = self.at_display_col_index(display_col_index)?;

        if seg_at_col.logical_index > ch(0) {
            let seg_left_of_col =
                self.at_logical_index(usize(seg_at_col.logical_index - ch(1)))?;
            Some(UnicodeStringSegmentSliceResult::new(
                seg_left_of_col.get_str(&self.string),
                seg_left_of_col.unicode_width,
                seg_left_of_col.start_display_col_index,
            ))
        } else {
            None
        }
    }

    pub fn get_string_at_end(&self) -> Option<UnicodeStringSegmentSliceResult> {
        let seg = self.last()?;

        Some(UnicodeStringSegmentSliceResult::new(
            seg.get_str(&self.string),
            seg.unicode_width,
            seg.start_display_col_index,
        ))
    }
}
