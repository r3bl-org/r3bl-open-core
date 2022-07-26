/*
 *   Copyright (c) 2022 Nazmul Idris
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

use r3bl_rs_utils::{assert_eq2, GraphemeClusterSegment, UnicodeStringExt};

const TEST_STRING: &str = "Hi 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿.";

#[test]
fn test_unicode_string_ext() {
  let test_string: String = TEST_STRING.to_string();
  let u_s = test_string.unicode_string();

  // Check overall sizes and counts on the `UnicodeString` struct.
  assert_eq2!(u_s.string, test_string);
  assert_eq2!(u_s.grapheme_cluster_segment_vec.len(), 11);
  assert_eq2!(u_s.grapheme_cluster_segment_count, 11);
  assert_eq2!(u_s.byte_size, test_string.len());
}

#[test]
fn test_grapheme_cluster_segment() {
  fn assert_segment(
    segment: GraphemeClusterSegment, string: &str, byte_offset: usize, unicode_width: usize, logical_index: usize,
    byte_size: usize,
  ) {
    assert_eq2!(segment.string, string);
    assert_eq2!(segment.byte_offset, byte_offset);
    assert_eq2!(segment.unicode_width, unicode_width);
    assert_eq2!(segment.logical_index, logical_index);
    assert_eq2!(segment.byte_size, byte_size);
  }

  let test_string: String = TEST_STRING.to_string();
  let u_s = test_string.unicode_string();

  // Check the individual `GraphemeClusterSegment` structs.
  assert_segment(u_s.grapheme_cluster_segment_vec[0], "H", 0, 1, 0, 1);
  assert_segment(u_s.grapheme_cluster_segment_vec[1], "i", 1, 1, 1, 1);
  assert_segment(u_s.grapheme_cluster_segment_vec[2], " ", 2, 1, 2, 1);
  assert_segment(u_s.grapheme_cluster_segment_vec[3], "😃", 3, 2, 3, 4);
  assert_segment(u_s.grapheme_cluster_segment_vec[4], " ", 7, 1, 4, 1);
  assert_segment(u_s.grapheme_cluster_segment_vec[5], "📦", 8, 2, 5, 4);
  assert_segment(u_s.grapheme_cluster_segment_vec[6], " ", 12, 1, 6, 1);
  assert_segment(u_s.grapheme_cluster_segment_vec[7], "🙏🏽", 13, 4, 7, 8);
  assert_segment(u_s.grapheme_cluster_segment_vec[8], " ", 21, 1, 8, 1);
  assert_segment(u_s.grapheme_cluster_segment_vec[9], "👨🏾‍🤝‍👨🏿", 22, 10, 9, 26);
  assert_segment(u_s.grapheme_cluster_segment_vec[10], ".", 48, 1, 10, 1);
}

#[test]
fn test_unicode_string_logical_index_tofro_display_col() {
  let test_string: String = TEST_STRING.to_string();
  let u_s = test_string.unicode_string();

  // Spot check some individual grapheme clusters at logical indices (the previous test does this exhaustively).
  assert_eq2!(u_s.at_logical_index(0).unwrap().string, "H");
  assert_eq2!(u_s.at_logical_index(1).unwrap().string, "i");
  assert_eq2!(u_s.at_logical_index(10).unwrap().string, ".");

  // Convert display column to logical index.
  assert_eq2!(u_s.at_display_col(0).unwrap().string, "H");
  assert_eq2!(u_s.at_display_col(1).unwrap().string, "i");
  assert_eq2!(u_s.at_display_col(2).unwrap().string, " ");
  assert_eq2!(u_s.at_display_col(3).unwrap().string, "😃");
  assert_eq2!(u_s.at_display_col(4).unwrap().string, "😃");
  assert_eq2!(u_s.at_display_col(5).unwrap().string, " ");
  assert_eq2!(u_s.at_display_col(6).unwrap().string, "📦");
  assert_eq2!(u_s.at_display_col(7).unwrap().string, "📦");
  assert_eq2!(u_s.at_display_col(8).unwrap().string, " ");
  assert_eq2!(u_s.at_display_col(9).unwrap().string, "🙏🏽");
  assert_eq2!(u_s.at_display_col(10).unwrap().string, "🙏🏽");
  assert_eq2!(u_s.at_display_col(11).unwrap().string, "🙏🏽");
  assert_eq2!(u_s.at_display_col(12).unwrap().string, "🙏🏽");
  assert_eq2!(u_s.at_display_col(13).unwrap().string, " ");
  assert_eq2!(u_s.at_display_col(14).unwrap().string, "👨🏾‍🤝‍👨🏿");
  assert_eq2!(u_s.at_display_col(15).unwrap().string, "👨🏾‍🤝‍👨🏿");
  assert_eq2!(u_s.at_display_col(16).unwrap().string, "👨🏾‍🤝‍👨🏿");
  assert_eq2!(u_s.at_display_col(17).unwrap().string, "👨🏾‍🤝‍👨🏿");
  assert_eq2!(u_s.at_display_col(18).unwrap().string, "👨🏾‍🤝‍👨🏿");
  assert_eq2!(u_s.at_display_col(19).unwrap().string, "👨🏾‍🤝‍👨🏿");
  assert_eq2!(u_s.at_display_col(20).unwrap().string, "👨🏾‍🤝‍👨🏿");
  assert_eq2!(u_s.at_display_col(21).unwrap().string, "👨🏾‍🤝‍👨🏿");
  assert_eq2!(u_s.at_display_col(22).unwrap().string, "👨🏾‍🤝‍👨🏿");
  assert_eq2!(u_s.at_display_col(23).unwrap().string, "👨🏾‍🤝‍👨🏿");
  assert_eq2!(u_s.at_display_col(24).unwrap().string, ".");

  // Spot check convert logical index to display column.
  assert_eq2!(u_s.logical_index_at_display_col(0).unwrap(), 0); // "H"
  assert_eq2!(u_s.logical_index_at_display_col(1).unwrap(), 1); // "i"
  assert_eq2!(u_s.logical_index_at_display_col(2).unwrap(), 2); // " "
  assert_eq2!(u_s.logical_index_at_display_col(3).unwrap(), 3); // "😃"
  assert_eq2!(u_s.logical_index_at_display_col(4).unwrap(), 3); // (same as above)
  assert_eq2!(u_s.logical_index_at_display_col(5).unwrap(), 4); // " "

  // Spot check convert display col to logical index.
  assert_eq2!(u_s.display_col_at_logical_index(0).unwrap(), 0); // "H"
  assert_eq2!(u_s.display_col_at_logical_index(1).unwrap(), 1); // "i"
  assert_eq2!(u_s.display_col_at_logical_index(2).unwrap(), 2); // " "
  assert_eq2!(u_s.display_col_at_logical_index(3).unwrap(), 3); // "😃"
  assert_eq2!(u_s.display_col_at_logical_index(4).unwrap(), 5); // " "
}

#[allow(clippy::zero_prefixed_literal)]
#[test]
fn test_unicode_string_truncate() {
  let test_string: String = TEST_STRING.to_string();
  let u_s = test_string.unicode_string();

  assert_eq2!(u_s.truncate_up_to_display_cols(00), "");
  assert_eq2!(u_s.truncate_up_to_display_cols(01), "H");
  assert_eq2!(u_s.truncate_up_to_display_cols(02), "Hi");
  assert_eq2!(u_s.truncate_up_to_display_cols(03), "Hi ");
  assert_eq2!(u_s.truncate_up_to_display_cols(04), "Hi ");
  assert_eq2!(u_s.truncate_up_to_display_cols(05), "Hi 😃");
  assert_eq2!(u_s.truncate_up_to_display_cols(06), "Hi 😃 ");
  assert_eq2!(u_s.truncate_up_to_display_cols(07), "Hi 😃 ");
  assert_eq2!(u_s.truncate_up_to_display_cols(08), "Hi 😃 📦");
  assert_eq2!(u_s.truncate_up_to_display_cols(09), "Hi 😃 📦 ");
  assert_eq2!(u_s.truncate_up_to_display_cols(10), "Hi 😃 📦 ");
  assert_eq2!(u_s.truncate_up_to_display_cols(11), "Hi 😃 📦 ");
  assert_eq2!(u_s.truncate_up_to_display_cols(12), "Hi 😃 📦 ");
  assert_eq2!(u_s.truncate_up_to_display_cols(13), "Hi 😃 📦 🙏🏽");
  assert_eq2!(u_s.truncate_up_to_display_cols(14), "Hi 😃 📦 🙏🏽 ");
  assert_eq2!(u_s.truncate_up_to_display_cols(15), "Hi 😃 📦 🙏🏽 ");
  assert_eq2!(u_s.truncate_up_to_display_cols(16), "Hi 😃 📦 🙏🏽 ");
  assert_eq2!(u_s.truncate_up_to_display_cols(17), "Hi 😃 📦 🙏🏽 ");
  assert_eq2!(u_s.truncate_up_to_display_cols(18), "Hi 😃 📦 🙏🏽 ");
  assert_eq2!(u_s.truncate_up_to_display_cols(19), "Hi 😃 📦 🙏🏽 ");
  assert_eq2!(u_s.truncate_up_to_display_cols(20), "Hi 😃 📦 🙏🏽 ");
  assert_eq2!(u_s.truncate_up_to_display_cols(21), "Hi 😃 📦 🙏🏽 ");
  assert_eq2!(u_s.truncate_up_to_display_cols(22), "Hi 😃 📦 🙏🏽 ");
  assert_eq2!(u_s.truncate_up_to_display_cols(23), "Hi 😃 📦 🙏🏽 ");
  assert_eq2!(u_s.truncate_up_to_display_cols(24), "Hi 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿");
  assert_eq2!(u_s.truncate_up_to_display_cols(25), "Hi 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿.");
}
