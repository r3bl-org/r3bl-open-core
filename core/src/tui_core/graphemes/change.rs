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

use crate::{ChUnit, StringStorage, UnicodeString, VecArrayStr, ch, join};

impl UnicodeString {
    /// Inserts the given `chunk` in the correct position of the `string`, and returns a
    /// new ([StringStorage], [ChUnit]) tuple:
    /// 1. The first item in the tuple is the new [StringStorage] after the insertion.
    /// 2. The second item in the tuple is the unicode width / display width of the
    ///    inserted `chunk`.
    pub fn insert_chunk_at_display_col(
        &self,
        display_col: ChUnit,
        chunk: &str,
    ) -> (StringStorage, ChUnit) {
        // Create an array-vec of &str from self.vec_segment, using self.iter().
        let mut acc = VecArrayStr::with_capacity(self.len() + 1);
        acc.extend(self.iter().map(|seg| seg.get_str(&self.string)));

        match self.logical_index_at_display_col_index(display_col) {
            // Insert somewhere inside bounds of self.string.
            Some(logical_index) => acc.insert(logical_index, chunk),
            // Add to end of self.string.
            None => acc.push(chunk),
        };

        // Generate a new StringStorage from acc and return it and the unicode width of
        // the character.
        (
            join!(from: acc, each: item, delim: "", format: "{item}"),
            ch(UnicodeString::str_display_width(chunk)),
        )
    }

    /// Returns a new [StringStorage] option.
    pub fn delete_char_at_display_col(
        &self,
        display_col: ChUnit,
    ) -> Option<StringStorage> {
        // There is no segment present (Deref trait makes `len()` apply to `vec_segment`).
        if self.is_empty() {
            return None;
        }

        // There is only one segment present.
        if self.len() == 1 {
            return Some("".into());
        }

        // There are more than 1 segments present.
        let split_logical_index = self.logical_index_at_display_col_index(display_col)?;
        let max_logical_index = self.len();

        let mut vec_left = VecArrayStr::with_capacity(self.len());
        let mut str_left_unicode_width = ch(0);
        {
            for logical_idx in 0..split_logical_index {
                let seg_at_logical_idx = self.at_logical_index(logical_idx)?;
                let string = self.get_str(&self.string, seg_at_logical_idx);
                vec_left.push(string);
                str_left_unicode_width += seg_at_logical_idx.unicode_width;
            }
        }

        let skip_split_logical_index = split_logical_index + 1; // Drop one segment.
        let mut vec_right = VecArrayStr::with_capacity(self.len());
        let mut str_right_unicode_width = ch(0);
        {
            for logical_idx in skip_split_logical_index..max_logical_index {
                let seg_at_logical_idx = self.at_logical_index(logical_idx)?;
                let string = self.get_str(&self.string, seg_at_logical_idx);
                vec_right.push(string);
                str_right_unicode_width += seg_at_logical_idx.unicode_width;
            }
        }

        // Merge the two vectors.
        vec_left.append(&mut vec_right);
        Some(join!(from: vec_left, each: it, delim: "", format: "{it}"))
    }

    /// Returns two new tuples:
    /// 1. *left* [StringStorage],
    /// 2. *right* [StringStorage].
    pub fn split_at_display_col(
        &self,
        display_col: ChUnit,
    ) -> Option<(StringStorage, StringStorage)> {
        let split_logical_index = self.logical_index_at_display_col_index(display_col)?;
        let max_logical_index = self.len();

        let mut vec_left = VecArrayStr::with_capacity(self.len());
        let mut str_left_unicode_width = ch(0);
        {
            for logical_idx in 0..split_logical_index {
                let segment = self.at_logical_index(logical_idx)?;
                vec_left.push(self.get_str(&self.string, segment));
                str_left_unicode_width += segment.unicode_width;
            }
        }

        let mut vec_right = VecArrayStr::with_capacity(self.len());
        let mut str_right_unicode_width = ch(0);
        {
            for logical_idx in split_logical_index..max_logical_index {
                let seg_at_logical_idx = self.at_logical_index(logical_idx)?;
                vec_right.push(self.get_str(&self.string, seg_at_logical_idx));
                str_right_unicode_width += seg_at_logical_idx.unicode_width;
            }
        }

        if str_right_unicode_width > ch(0) || str_left_unicode_width > ch(0) {
            let lhs = { join!(from: vec_left, each: it, delim: "", format: "{it}") };
            let rhs = { join!(from: vec_right, each: it, delim: "", format: "{it}") };
            Some((lhs, rhs))
        } else {
            None
        }
    }
}
