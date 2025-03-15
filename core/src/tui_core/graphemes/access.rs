/*
 *   Copyright (c) 2022 R3BL LLC
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

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{ch,
            ChUnit,
            GraphemeClusterSegment,
            SelectionRange,
            Size,
            UnicodeString,
            UnicodeStringSegmentSliceResult};

impl UnicodeString {
    /// If any segment in `self.vec_segment` has a `display_col_offset` greater than 1
    /// then this is true. The semantic is that the string is displayed using more than 1
    /// column of the terminal.
    pub fn contains_wide_segments(&self) -> bool {
        let mut contains_wide_segments = false;

        for grapheme_cluster_segment in self.iter() {
            if grapheme_cluster_segment.unicode_width > ch!(1) {
                contains_wide_segments = true;
                break;
            }
        }

        contains_wide_segments
    }

    pub fn char_display_width(character: char) -> usize {
        let display_width: usize = UnicodeWidthChar::width(character).unwrap_or(0);
        display_width
    }

    pub fn str_display_width(string: &str) -> usize {
        let display_width: usize = UnicodeWidthStr::width(string);
        display_width
    }

    /// The `size` is a column index and row index. Not width or height.
    /// - To convert width -> size / column index subtract 1.
    /// - To convert size / column index to width add 1.
    ///
    /// Note the [Self::truncate_end_by_n_col] and [Self::truncate_start_by_n_col]
    /// functions take a width.
    pub fn truncate_to_fit_size(&self, size: Size) -> &str {
        let display_cols: ChUnit = size.col_count;
        self.truncate_end_to_fit_width(display_cols)
    }

    /// The `n_display_col` is a width, not a [Size].
    /// - To convert width -> size / column index subtract 1.
    /// - To convert size / column index to width add 1.
    ///
    /// Note the [Self::truncate_to_fit_size] function takes a size / column index.
    pub fn truncate_end_by_n_col(&self, n_display_col: ChUnit) -> &str {
        let mut countdown_col_count = n_display_col;
        let mut string_end_byte_index = 0;

        for segment in self.iter().rev() {
            let segment_display_width = segment.unicode_width;
            string_end_byte_index = segment.byte_offset;
            countdown_col_count -= segment_display_width;
            if countdown_col_count == ch!(0) {
                // We are done skipping.
                break;
            }
        }

        &self.string[..string_end_byte_index]
    }

    /// Removes segments from the start of the string so that `col_count` (width) is
    /// skipped.
    ///
    /// ```rust
    /// use r3bl_core::UnicodeString;
    /// use r3bl_core::ChUnit;
    ///
    /// let col_count:r3bl_core::ChUnit = 2.into();
    /// let display_cols:r3bl_core::ChUnit = 5.into();
    /// let expected_clipped_string = "rst s";
    /// let line = "first second";
    /// let line = UnicodeString::from(line);
    ///
    /// let truncated_line = line.truncate_start_by_n_col(col_count);
    /// let truncated_line = UnicodeString::from(truncated_line);
    ///
    /// let truncated_line = truncated_line.truncate_end_to_fit_width(display_cols);
    ///
    /// assert_eq!(truncated_line, expected_clipped_string);
    /// ```
    pub fn truncate_start_by_n_col(&self, n_display_col: ChUnit) -> &str {
        let mut skip_col_count = n_display_col;
        let mut string_start_byte_index = 0;

        for segment in self.iter() {
            if skip_col_count != ch!(0) {
                // Skip segment.unicode_width.
                skip_col_count -= segment.unicode_width;
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
    ///   ←─3─→ : size (or "width" or "col count" or "count", 1 based)
    /// R ┌───┐
    /// 0 │fir│st second
    ///   └───┘
    ///   C012 345678901 : index (0 based)
    /// ```
    ///
    /// Example.
    /// ```rust
    /// use r3bl_core::UnicodeString;
    /// use r3bl_core::ChUnit;
    ///
    /// let scroll_offset_col:r3bl_core::ChUnit = 0.into();
    /// let display_cols:r3bl_core::ChUnit = 3.into();
    /// let expected_clipped_string = "fir";
    /// let line = "first second";
    /// let line = UnicodeString::from(line);
    ///
    /// let truncated_line = line.truncate_start_by_n_col(scroll_offset_col);
    /// let truncated_line = UnicodeString::from(truncated_line);
    ///
    /// let truncated_line = truncated_line.truncate_end_to_fit_width(display_cols);
    ///
    /// assert_eq!(truncated_line, expected_clipped_string);
    /// ```
    pub fn truncate_end_to_fit_width(&self, display_col_count: ChUnit) -> &str {
        let mut avail_cols = display_col_count;
        let mut string_end_byte_index = 0;

        for segment in self.iter() {
            if avail_cols < segment.unicode_width {
                break;
            }
            string_end_byte_index += segment.byte_size;
            avail_cols -= segment.unicode_width;
        }

        &self.string[..string_end_byte_index]
    }

    /// Returns a new [String] that is the result of padding `self.string` to fit the
    /// given width w/ the given spacer character.
    pub fn pad_end_with_spaces_to_fit_width(
        &self,
        spacer: &str,
        max_display_col_count: ChUnit,
    ) -> String {
        let pad_len = max_display_col_count - self.display_width;
        if pad_len > ch!(0) {
            let pad_str = spacer.repeat(ch!(@to_usize pad_len));
            format!("{}{}", self.string, pad_str)
        } else {
            self.string.to_owned()
        }
    }

    /// Uses [SelectionRange] to calculate width and simply calls
    /// [clip_to_width](Self::clip_to_width).
    pub fn clip_to_range(&self, range: SelectionRange) -> &str {
        let SelectionRange {
            start_display_col_index,
            end_display_col_index,
        } = range;
        let max_display_col_count = end_display_col_index - start_display_col_index;
        self.clip_to_width(start_display_col_index, max_display_col_count)
    }

    /// Clip the content starting from `start_col_index` and take as many columns as
    /// possible until `max_display_col_count` is reached.
    ///
    /// # Arguments
    /// - `start_display_col_index`: This an index value.
    /// - `max_display_col_count`: The is not an index value, but a size or count value.
    pub fn clip_to_width(
        &self,
        /* index */ start_display_col_index: ChUnit,
        /* width */ max_display_col_count: ChUnit,
    ) -> &str {
        let string_start_byte_index = {
            let mut it = 0;
            let mut skip_col_count = start_display_col_index;
            for segment in self.iter() {
                // Skip scroll_offset_col_index columns.
                if skip_col_count != ch!(0) {
                    // Skip segment.unicode_width.
                    skip_col_count -= segment.unicode_width;
                    it += segment.byte_size;
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
            for segment in self.iter() {
                // Skip scroll_offset_col_index columns (again).
                if skip_col_count != ch!(0) {
                    // Skip segment.unicode_width.
                    skip_col_count -= segment.unicode_width;
                    it += segment.byte_size;
                }
                // Clip max_display_col_count columns.
                else {
                    if avail_col_count < segment.unicode_width {
                        break;
                    }
                    it += segment.byte_size;
                    avail_col_count -= segment.unicode_width;
                }
            }
            it
        };

        &self.string[string_start_byte_index..string_end_byte_index]
    }

    /// If `self.string` is shorter than `max_display_col_count` then a padding string is
    /// returned (that is comprised of the `pad_char` repeated).
    pub fn try_get_postfix_padding_for(
        &self,
        pad_char: char,
        max_display_col_count: ChUnit,
    ) -> Option<String> {
        // Pad the line to the max cols w/ spaces. This removes any "ghost" carets that
        // were painted in a previous render.
        let display_width = UnicodeString::from(&self.string).display_width;
        if display_width < max_display_col_count {
            let padding = max_display_col_count - display_width;
            Some(pad_char.to_string().repeat(ch!(@to_usize padding)))
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

    /// `display_col` is the col index in the terminal where this grapheme cluster can be
    /// displayed.
    pub fn at_display_col_index(
        &self,
        display_col: ChUnit,
    ) -> Option<&GraphemeClusterSegment> {
        self.iter().find(|&grapheme_cluster_segment| {
            let segment_display_col_start: ChUnit =
                grapheme_cluster_segment.display_col_offset;
            let segment_display_col_end: ChUnit =
                segment_display_col_start + grapheme_cluster_segment.unicode_width;
            display_col >= segment_display_col_start
                && display_col < segment_display_col_end
        })
    }

    /// Convert a `display_col` to a `logical_index`.
    /// - `local_index` is the index of the grapheme cluster in the `vec_segment`.
    /// - `display_col` is the col index in the terminal where this grapheme cluster can
    ///   be displayed.
    pub fn logical_index_at_display_col_index(
        &self,
        display_col: ChUnit,
    ) -> Option<usize> {
        self.at_display_col_index(display_col)
            .map(|segment| segment.logical_index)
    }

    /// Convert a `logical_index` to a `display_col`.
    /// - `local_index` is the index of the grapheme cluster in the `vec_segment`.
    /// - `display_col` is the col index in the terminal where this grapheme cluster can
    ///   be displayed.
    pub fn display_col_index_at_logical_index(
        &self,
        logical_index: usize,
    ) -> Option<ChUnit> {
        self.at_logical_index(logical_index)
            .map(|segment| segment.display_col_offset)
    }

    /// Return the string and unicode width of the grapheme cluster segment at the given
    /// `display_col`. If this `display_col` falls in the middle of a grapheme cluster,
    /// then return [None].
    pub fn get_string_at_display_col_index(
        &self,
        display_col: ChUnit,
    ) -> Option<UnicodeStringSegmentSliceResult> {
        let segment = self.at_display_col_index(display_col)?;
        // What if the display_col is in the middle of a grapheme cluster?
        if display_col != segment.display_col_offset {
            None
        } else {
            Some(UnicodeStringSegmentSliceResult::new(
                &segment.string,
                segment.unicode_width,
                segment.display_col_offset,
            ))
        }
    }

    /// If the given `display_col` falls in the middle of a grapheme cluster, then return
    /// the [GraphemeClusterSegment] at that `display_col`. Otherwise return [None].
    pub fn is_display_col_index_in_middle_of_grapheme_cluster(
        &self,
        display_col: ChUnit,
    ) -> Option<GraphemeClusterSegment> {
        let segment = self.at_display_col_index(display_col);
        if let Some(segment) = segment {
            if display_col != segment.display_col_offset {
                return Some(segment.clone());
            }
        }
        None
    }

    pub fn get_string_at_right_of_display_col_index(
        &self,
        display_col: ChUnit,
    ) -> Option<UnicodeStringSegmentSliceResult> {
        let segment_at_col = self.at_display_col_index(display_col)?;
        if segment_at_col.logical_index < self.len() - 1 {
            let segment_right_of_col =
                self.at_logical_index(segment_at_col.logical_index + 1)?;
            Some(UnicodeStringSegmentSliceResult::new(
                &segment_right_of_col.string,
                segment_right_of_col.unicode_width,
                segment_right_of_col.display_col_offset,
            ))
        } else {
            None
        }
    }

    pub fn get_string_at_left_of_display_col_index(
        &self,
        display_col: ChUnit,
    ) -> Option<UnicodeStringSegmentSliceResult> {
        let segment_at_col = self.at_display_col_index(display_col)?;
        if segment_at_col.logical_index > 0 {
            let segment_left_of_col =
                self.at_logical_index(segment_at_col.logical_index - 1)?;
            Some(UnicodeStringSegmentSliceResult::new(
                &segment_left_of_col.string,
                segment_left_of_col.unicode_width,
                segment_left_of_col.display_col_offset,
            ))
        } else {
            None
        }
    }

    pub fn get_string_at_end(&self) -> Option<UnicodeStringSegmentSliceResult> {
        let segment = self.last()?;
        Some(UnicodeStringSegmentSliceResult::new(
            &segment.string,
            segment.unicode_width,
            segment.display_col_offset,
        ))
    }
}
