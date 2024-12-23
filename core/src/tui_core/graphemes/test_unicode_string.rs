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

#[cfg(test)]
mod tests {
    use crate::{ChUnit,
                GraphemeClusterSegment,
                UnicodeString,
                UnicodeStringExt,
                assert_eq2,
                ch};

    const TEST_STRING: &str = "Hi 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿.";

    #[test]
    fn test_unicode_string_ext() {
        let test_string: String = TEST_STRING.to_string();
        let u_s = test_string.unicode_string();

        // Check overall sizes and counts on the `UnicodeString` struct.
        assert_eq2!(u_s.string, test_string);
        assert_eq2!(u_s.len(), 11);
        assert_eq2!(u_s.grapheme_cluster_segment_count, 11);
        assert_eq2!(u_s.byte_size, test_string.len());
        assert_eq2!(u_s.display_width, ch(15));
    }

    #[allow(clippy::zero_prefixed_literal)]
    #[test]
    fn test_grapheme_cluster_segment() {
        fn assert_segment(
            us: &UnicodeString,
            segment: &GraphemeClusterSegment,
            byte_offset: usize,
            unicode_width: ChUnit,
            logical_index: usize,
            byte_size: usize,
            string: &str,
        ) {
            let segment_string = segment.get_str(&us.string);
            assert_eq2!(segment_string, string);
            assert_eq2!(segment.byte_offset, ch(byte_offset));
            assert_eq2!(segment.unicode_width, ch(unicode_width));
            assert_eq2!(segment.logical_index, ch(logical_index));
            assert_eq2!(segment.byte_size, byte_size);
        }

        let test_string: String = TEST_STRING.to_string();
        let u_s: UnicodeString = test_string.unicode_string();

        // Check the individual `GraphemeClusterSegment` structs.
        assert_segment(&u_s, &u_s[00], 00, 01.into(), 00, 01, "H");
        assert_segment(&u_s, &u_s[01], 01, 01.into(), 01, 01, "i");
        assert_segment(&u_s, &u_s[02], 02, 01.into(), 02, 01, " ");
        assert_segment(&u_s, &u_s[03], 03, 02.into(), 03, 04, "😃");
        assert_segment(&u_s, &u_s[04], 07, 01.into(), 04, 01, " ");
        assert_segment(&u_s, &u_s[05], 08, 02.into(), 05, 04, "📦");
        assert_segment(&u_s, &u_s[06], 12, 01.into(), 06, 01, " ");
        assert_segment(&u_s, &u_s[07], 13, 02.into(), 07, 08, "🙏🏽");
        assert_segment(&u_s, &u_s[08], 21, 01.into(), 08, 01, " ");
        assert_segment(&u_s, &u_s[09], 22, 02.into(), 09, 26, "👨🏾‍🤝‍👨🏿");
        assert_segment(&u_s, &u_s[10], 48, 01.into(), 10, 01, ".");
    }

    #[rustfmt::skip]
    #[allow(clippy::zero_prefixed_literal)]
    #[test]
    fn test_unicode_string_logical_index_tofro_display_col() {
        let test_string: String = TEST_STRING.to_string();
        let u_s: UnicodeString = test_string.unicode_string();

        // Spot check some individual grapheme clusters at logical indices (the previous test does this exhaustively).
        assert_eq2!(u_s.at_logical_index(00).unwrap().get_str(u_s.string.as_ref()), "H");
        assert_eq2!(u_s.at_logical_index(01).unwrap().get_str(u_s.string.as_ref()), "i");
        assert_eq2!(u_s.at_logical_index(10).unwrap().get_str(u_s.string.as_ref()), ".");

        // Convert display column to logical index.
        assert_eq2!(u_s.at_display_col_index(00.into()).unwrap().get_str(u_s.string.as_ref()), "H");
        assert_eq2!(u_s.at_display_col_index(01.into()).unwrap().get_str(u_s.string.as_ref()), "i");
        assert_eq2!(u_s.at_display_col_index(02.into()).unwrap().get_str(u_s.string.as_ref()), " ");
        assert_eq2!(u_s.at_display_col_index(03.into()).unwrap().get_str(u_s.string.as_ref()), "😃");
        assert_eq2!(u_s.at_display_col_index(04.into()).unwrap().get_str(u_s.string.as_ref()), "😃");
        assert_eq2!(u_s.at_display_col_index(05.into()).unwrap().get_str(u_s.string.as_ref()), " ");
        assert_eq2!(u_s.at_display_col_index(06.into()).unwrap().get_str(u_s.string.as_ref()), "📦");
        assert_eq2!(u_s.at_display_col_index(07.into()).unwrap().get_str(u_s.string.as_ref()), "📦");
        assert_eq2!(u_s.at_display_col_index(08.into()).unwrap().get_str(u_s.string.as_ref()), " ");
        assert_eq2!(u_s.at_display_col_index(09.into()).unwrap().get_str(u_s.string.as_ref()), "🙏🏽");
        assert_eq2!(u_s.at_display_col_index(10.into()).unwrap().get_str(u_s.string.as_ref()), "🙏🏽");
        assert_eq2!(u_s.at_display_col_index(11.into()).unwrap().get_str(u_s.string.as_ref()), " ");
        assert_eq2!(u_s.at_display_col_index(12.into()).unwrap().get_str(u_s.string.as_ref()), "👨🏾‍🤝‍👨🏿");
        assert_eq2!(u_s.at_display_col_index(13.into()).unwrap().get_str(u_s.string.as_ref()), "👨🏾‍🤝‍👨🏿");
        assert_eq2!(u_s.at_display_col_index(14.into()).unwrap().get_str(u_s.string.as_ref()), ".");

        // Spot check convert logical index to display column.
        assert_eq2!(u_s.logical_index_at_display_col_index(0.into()).unwrap(), 0); // "H"
        assert_eq2!(u_s.logical_index_at_display_col_index(1.into()).unwrap(), 1); // "i"
        assert_eq2!(u_s.logical_index_at_display_col_index(2.into()).unwrap(), 2); // " "
        assert_eq2!(u_s.logical_index_at_display_col_index(3.into()).unwrap(), 3); // "😃"
        assert_eq2!(u_s.logical_index_at_display_col_index(4.into()).unwrap(), 3); // (same as above)
        assert_eq2!(u_s.logical_index_at_display_col_index(5.into()).unwrap(), 4); // " "

        // Spot check convert display col to logical index.
        assert_eq2!(u_s.display_col_index_at_logical_index(0).unwrap(), 0.into()); // "H"
        assert_eq2!(u_s.display_col_index_at_logical_index(1).unwrap(), 1.into()); // "i"
        assert_eq2!(u_s.display_col_index_at_logical_index(2).unwrap(), 2.into()); // " "
        assert_eq2!(u_s.display_col_index_at_logical_index(3).unwrap(), 3.into()); // "😃"
        assert_eq2!(u_s.display_col_index_at_logical_index(4).unwrap(), 5.into());
        // " "
    }

    #[allow(clippy::zero_prefixed_literal)]
    #[test]
    fn test_unicode_string_truncate_to_fit_display_cols() {
        let test_string: String = TEST_STRING.to_string();
        let u_s = test_string.unicode_string();

        assert_eq2! {u_s.truncate_end_to_fit_width(00.into()), ""};
        assert_eq2! {u_s.truncate_end_to_fit_width(01.into()), "H"};
        assert_eq2! {u_s.truncate_end_to_fit_width(02.into()), "Hi"};
        assert_eq2! {u_s.truncate_end_to_fit_width(03.into()), "Hi "};
        assert_eq2! {u_s.truncate_end_to_fit_width(04.into()), "Hi "};
        assert_eq2! {u_s.truncate_end_to_fit_width(05.into()), "Hi 😃"};
        assert_eq2! {u_s.truncate_end_to_fit_width(06.into()), "Hi 😃 "};
        assert_eq2! {u_s.truncate_end_to_fit_width(07.into()), "Hi 😃 "};
        assert_eq2! {u_s.truncate_end_to_fit_width(08.into()), "Hi 😃 📦"};
        assert_eq2! {u_s.truncate_end_to_fit_width(09.into()), "Hi 😃 📦 "};
        assert_eq2! {u_s.truncate_end_to_fit_width(10.into()), "Hi 😃 📦 "};
        assert_eq2! {u_s.truncate_end_to_fit_width(11.into()), "Hi 😃 📦 🙏🏽"};
        assert_eq2! {u_s.truncate_end_to_fit_width(12.into()), "Hi 😃 📦 🙏🏽 "};
        assert_eq2! {u_s.truncate_end_to_fit_width(13.into()), "Hi 😃 📦 🙏🏽 "};
        assert_eq2! {u_s.truncate_end_to_fit_width(14.into()), "Hi 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿"};
        assert_eq2! {u_s.truncate_end_to_fit_width(15.into()), "Hi 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
    }

    #[allow(clippy::zero_prefixed_literal)]
    #[test]
    fn test_unicode_string_truncate_end_by_n_col() {
        let test_string: String = TEST_STRING.to_string();
        let u_s = test_string.unicode_string();

        assert_eq2! {u_s.truncate_end_by_n_col(01.into()), "Hi 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿"};
        assert_eq2! {u_s.truncate_end_by_n_col(02.into()), "Hi 😃 📦 🙏🏽 "};
        assert_eq2! {u_s.truncate_end_by_n_col(03.into()), "Hi 😃 📦 🙏🏽 "};
        assert_eq2! {u_s.truncate_end_by_n_col(04.into()), "Hi 😃 📦 🙏🏽"};
        assert_eq2! {u_s.truncate_end_by_n_col(05.into()), "Hi 😃 📦 "};
        assert_eq2! {u_s.truncate_end_by_n_col(06.into()), "Hi 😃 📦 "};
        assert_eq2! {u_s.truncate_end_by_n_col(07.into()), "Hi 😃 📦"};
        assert_eq2! {u_s.truncate_end_by_n_col(08.into()), "Hi 😃 "};
        assert_eq2! {u_s.truncate_end_by_n_col(09.into()), "Hi 😃 "};
        assert_eq2! {u_s.truncate_end_by_n_col(10.into()), "Hi 😃"};
        assert_eq2! {u_s.truncate_end_by_n_col(11.into()), "Hi "};
        assert_eq2! {u_s.truncate_end_by_n_col(12.into()), "Hi "};
        assert_eq2! {u_s.truncate_end_by_n_col(13.into()), "Hi"};
        assert_eq2! {u_s.truncate_end_by_n_col(14.into()), "H"};
        assert_eq2! {u_s.truncate_end_by_n_col(15.into()), ""};
        assert_eq2! {u_s.truncate_end_by_n_col(16.into()), ""};
    }

    #[allow(clippy::zero_prefixed_literal)]
    #[test]
    fn test_unicode_string_truncate_start_by_n_col() {
        let test_string: String = TEST_STRING.to_string();
        let u_s = test_string.unicode_string();

        assert_eq2! {u_s.truncate_start_by_n_col(01.into()), "i 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
        assert_eq2! {u_s.truncate_start_by_n_col(02.into()), " 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
        assert_eq2! {u_s.truncate_start_by_n_col(03.into()), "😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
        assert_eq2! {u_s.truncate_start_by_n_col(04.into()), " 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
        assert_eq2! {u_s.truncate_start_by_n_col(05.into()), " 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
        assert_eq2! {u_s.truncate_start_by_n_col(06.into()), "📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
    }

    #[allow(clippy::zero_prefixed_literal)]
    #[test]
    fn test_unicode_string2_insert_at_display_col() {
        let test_string: String = TEST_STRING.to_string();
        let u_s = test_string.unicode_string();
        assert_eq2!(u_s.display_width, ch(15));

        // Insert "😃" at display col 1, just after `H`.
        let (new_string, display_width_of_inserted_chunk) =
            u_s.insert_char_at_display_col(1.into(), "😃");

        let new_unicode_string = new_string.unicode_string();

        assert_eq2! {display_width_of_inserted_chunk, ch(2)};
        assert_eq2! {new_unicode_string.truncate_start_by_n_col(00.into()), "H😃i 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
        assert_eq2! {new_unicode_string.truncate_start_by_n_col(01.into()), "😃i 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
    }

    #[allow(clippy::zero_prefixed_literal)]
    #[test]
    fn test_unicode_string2_delete_at_display_col() {
        let test_string: String = TEST_STRING.to_string();
        let u_s = test_string.unicode_string();
        assert_eq2!(u_s.display_width, ch(15));

        // Delete "i" at display col 1, just after `H`.
        let Some(new_string) = u_s.delete_char_at_display_col(1.into()) else {
            panic!("Failed to delete char");
        };

        let new_unicode_string = new_string.unicode_string();
        assert_eq2! {new_unicode_string.display_width, ch(14)};
        assert_eq2! {new_unicode_string.truncate_start_by_n_col(00.into()), "H 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
        assert_eq2! {new_unicode_string.truncate_start_by_n_col(01.into()), " 😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
    }

    #[allow(clippy::zero_prefixed_literal)]
    #[test]
    fn test_unicode_string2_split_at_display_col() {
        let test_string: String = TEST_STRING.to_string();
        let u_s = test_string.unicode_string();
        assert_eq2!(u_s.display_width, ch(15));

        // Split at display col 4.
        let Some((lhs_string, rhs_string)) = u_s.split_at_display_col(4.into()) else {
            panic!("Failed to split unicode string");
        };

        let lhs_u_s = lhs_string.unicode_string();
        let rhs_u_s = rhs_string.unicode_string();

        assert_eq2! {lhs_u_s.display_width, ch(3)};
        assert_eq2! {rhs_u_s.display_width, ch(12)};

        assert_eq2! {lhs_u_s.truncate_start_by_n_col(00.into()), "Hi "};
        assert_eq2! {rhs_u_s.truncate_start_by_n_col(00.into()), "😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};

        let acc = [lhs_u_s, rhs_u_s];
        assert_eq2! {acc[0].string, "Hi "};
        assert_eq2! {acc[1].string, "😃 📦 🙏🏽 👨🏾‍🤝‍👨🏿."};
    }
}
