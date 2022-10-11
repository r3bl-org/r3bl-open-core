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

use crate::*;

impl UnicodeString {
  /// Returns a new [String]. Does not modify [self.string](UnicodeString::string).
  pub fn merge_with(&self, other: UnicodeString) -> Option<String> {
    let mut new_string: String = self.string.clone();
    new_string.push_str(&other.string);
    Some(new_string)
  }

  /// Returns a new ([NewUnicodeStringResult]) tuple. Does not modify
  /// [self.string](UnicodeString::string).
  pub fn insert_char_at_display_col(&self, display_col: ChUnit, chunk: &str) -> Option<NewUnicodeStringResult> {
    let maybe_logical_index = self.logical_index_at_display_col(display_col);
    match maybe_logical_index {
      // Insert somewhere inside bounds of self.string.
      Some(logical_index) => {
        // Convert the character into a grapheme cluster.
        let character_g_c_s: GraphemeClusterSegment = make_new_grapheme_cluster_segment_from(chunk);
        let character_display_width: ChUnit = character_g_c_s.unicode_width;

        // Insert self grapheme cluster to self.vec_segment.
        let mut vec_segment_clone = self.vec_segment.clone();
        vec_segment_clone.insert(logical_index, character_g_c_s);

        // Generate a new string from self.vec_segment and return it and the unicode width of the
        // character.
        let new_string = make_new_string_from(vec_segment_clone);

        // In the caller - update the caret position based on the unicode width of the character.
        Some(NewUnicodeStringResult::new(new_string, character_display_width))
      }
      // Add to end of self.string.
      None => {
        // Push character to the end of the cloned string.
        let mut new_string: String = self.string.clone();
        new_string.push_str(chunk);

        // Get the unicode width of the character.
        let character_display_width = UnicodeString::str_display_width(chunk);

        // In the caller - update the caret position based on the unicode width of the character.
        Some(NewUnicodeStringResult::new(new_string, ch!(character_display_width)))
      }
    }
  }

  /// Returns two new tuples: *left* ([NewUnicodeStringResult]), *right*
  /// ([NewUnicodeStringResult]). Does not modify [self.string](UnicodeString::string).
  pub fn split_at_display_col(&self, display_col: ChUnit) -> Option<(NewUnicodeStringResult, NewUnicodeStringResult)> {
    let split_logical_index = self.logical_index_at_display_col(display_col)?;
    let max_logical_index = self.len();

    let mut str_left = String::new();
    let mut str_left_unicode_width = ch!(0);
    {
      for logical_idx in 0..split_logical_index {
        let segment = self.at_logical_index(logical_idx)?;
        str_left.push_str(&segment.string);
        str_left_unicode_width += segment.unicode_width;
      }
    }

    let mut str_right = String::new();
    let mut str_right_unicode_width = ch!(0);
    {
      for logical_idx in split_logical_index..max_logical_index {
        let seg_at_logical_idx = self.at_logical_index(logical_idx)?;
        str_right.push_str(&seg_at_logical_idx.string);
        str_right_unicode_width += seg_at_logical_idx.unicode_width;
      }
    }

    if *str_right_unicode_width > 0 || *str_left_unicode_width > 0 {
      Some((
        NewUnicodeStringResult::new(str_left, str_left_unicode_width),
        NewUnicodeStringResult::new(str_right, str_right_unicode_width),
      ))
    } else {
      None
    }
  }

  /// Returns a new ([NewUnicodeStringResult]) tuple. Does not modify
  /// [self.string](UnicodeString::string).
  pub fn delete_char_at_display_col(&self, display_col: ChUnit) -> Option<NewUnicodeStringResult> {
    // There is only one segment present.
    if self.len() == 1 {
      return Some(NewUnicodeStringResult::default());
    }

    // There are more than 1 segments present.i
    let split_logical_index = self.logical_index_at_display_col(display_col)?;
    let max_logical_index = self.len();

    let mut str_left = String::new();
    let mut str_left_unicode_width = ch!(0);
    {
      for logical_idx in 0..split_logical_index {
        let segment = self.at_logical_index(logical_idx)?;
        str_left.push_str(&segment.string);
        str_left_unicode_width += segment.unicode_width;
      }
    }

    let skip_split_logical_index = split_logical_index + 1; // Drop one segment.
    let mut str_right = String::new();
    let mut str_right_unicode_width = ch!(0);
    {
      for logical_idx in skip_split_logical_index..max_logical_index {
        let seg_at_logical_idx = self.at_logical_index(logical_idx)?;
        str_right.push_str(&seg_at_logical_idx.string);
        str_right_unicode_width += seg_at_logical_idx.unicode_width;
      }
    }

    str_left.push_str(&str_right);
    str_left_unicode_width += str_right_unicode_width;

    if *str_left_unicode_width > 0 {
      Some(NewUnicodeStringResult::new(str_left, str_left_unicode_width))
    } else {
      None
    }
  }
}

/// Convert [&str] to [GraphemeClusterSegment]. This is used to create a new [String] after the
/// [UnicodeString] is modified.
fn make_new_grapheme_cluster_segment_from(chunk: &str) -> GraphemeClusterSegment {
  let my_string: String = chunk.to_string();
  let unicode_string: UnicodeString = my_string.into();
  let result = unicode_string[0].clone();

  GraphemeClusterSegment {
    string: result.string,
    ..result
  }
}

/// Convert [Vec<GraphemeClusterSegment>] to [String]. This is used to create a new [String] after
/// the [UnicodeString] is modified.
fn make_new_string_from(vec_grapheme_cluster_segment: Vec<GraphemeClusterSegment>) -> String {
  let mut my_string = String::new();
  for grapheme_cluster_segment in vec_grapheme_cluster_segment {
    my_string.push_str(&grapheme_cluster_segment.string);
  }
  my_string
}
