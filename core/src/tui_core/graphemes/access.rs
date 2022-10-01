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

use crate::*;

impl UnicodeString {
  /// If any segment in `self.vec_segment` has a `display_col_offset` greater than 1 then this is
  /// true. The semantic is that the string is displayed using more than 1 column of the terminal.
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

  pub fn truncate_to_fit_size(&self, size: Size) -> &str {
    let display_cols: ChUnit = size.cols;
    self.truncate_end_to_fit_display_cols(display_cols)
  }

  /// Removes segments from the start of the string so that scroll_offset_col width is skipped.
  pub fn truncate_start_by_n_col(&self, scroll_offset_col: ChUnit) -> &str {
    let mut skip_col_count = scroll_offset_col;
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

  /// Removes segments from the end of the string that don't fit in the display_cols width.
  pub fn truncate_end_to_fit_display_cols(&self, display_cols: ChUnit) -> &str {
    let mut avail_cols = display_cols;
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

  /// `local_index` is the index of the grapheme cluster in the `vec_segment`.
  pub fn at_logical_index(&self, logical_index: usize) -> Option<&GraphemeClusterSegment> {
    self.get(logical_index)
  }

  /// `display_col` is the col index in the terminal where this grapheme cluster can be displayed.
  pub fn at_display_col(&self, display_col: ChUnit) -> Option<&GraphemeClusterSegment> {
    self.iter().find(|&grapheme_cluster_segment| {
      let segment_display_col_start: ChUnit = grapheme_cluster_segment.display_col_offset;
      let segment_display_col_end: ChUnit =
        segment_display_col_start + grapheme_cluster_segment.unicode_width;
      display_col >= segment_display_col_start && display_col < segment_display_col_end
    })
  }

  /// Convert a `display_col` to a `logical_index`.
  /// - `local_index` is the index of the grapheme cluster in the `vec_segment`.
  /// - `display_col` is the col index in the terminal where this grapheme cluster can be displayed.
  pub fn logical_index_at_display_col(&self, display_col: ChUnit) -> Option<usize> {
    self
      .at_display_col(display_col)
      .map(|segment| segment.logical_index)
  }

  /// Convert a `logical_index` to a `display_col`.
  /// - `local_index` is the index of the grapheme cluster in the `vec_segment`.
  /// - `display_col` is the col index in the terminal where this grapheme cluster can be displayed.
  pub fn display_col_at_logical_index(&self, logical_index: usize) -> Option<ChUnit> {
    self
      .at_logical_index(logical_index)
      .map(|segment| segment.display_col_offset)
  }

  /// Return the string and unicode width of the grapheme cluster segment at the given `display_col`.
  /// If this `display_col` falls in the middle of a grapheme cluster, then return [None].
  pub fn get_string_at_display_col(
    &self,
    display_col: ChUnit,
  ) -> Option<UnicodeStringSegmentSliceResult> {
    let segment = self.at_display_col(display_col)?;
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

  /// If the given `display_col` falls in the middle of a grapheme cluster, then return the
  /// [GraphemeClusterSegment] at that `display_col`. Otherwise return [None].
  pub fn is_display_col_in_middle_of_grapheme_cluster(
    &self,
    display_col: ChUnit,
  ) -> Option<GraphemeClusterSegment> {
    let segment = self.at_display_col(display_col);
    if let Some(segment) = segment {
      if display_col != segment.display_col_offset {
        return Some(segment.clone());
      }
    }
    None
  }

  pub fn get_string_at_left_of_display_col(
    &self,
    display_col: ChUnit,
  ) -> Option<UnicodeStringSegmentSliceResult> {
    let segment_at_col = self.at_display_col(display_col)?;
    if segment_at_col.logical_index > 0 {
      let segment_left_of_col = self.at_logical_index(segment_at_col.logical_index - 1)?;
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
