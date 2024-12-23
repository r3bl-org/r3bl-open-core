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

use crate::{ChUnit, TinyVecBackingStore, UnicodeString, ch};

impl UnicodeString {
    /// Returns a new ([UnicodeString], [ChUnit]) tuple. Does not modify
    /// [self.string](UnicodeString::string).
    pub fn insert_char_at_display_col(
        &self,
        display_col: ChUnit,
        chunk: &str,
    ) -> (String, ChUnit) {
        // Insert self grapheme cluster to self.vec_segment.
        let mut acc = TinyVecBackingStore::with_capacity(self.len() + 1);
        for seg in self.vec_segment.iter() {
            acc.push(self.get_str(seg));
        }

        let maybe_logical_index = self.logical_index_at_display_col_index(display_col);
        match maybe_logical_index {
            // Insert somewhere inside bounds of self.string.
            Some(logical_index) => acc.insert(logical_index, chunk),
            // Add to end of self.string.
            None => acc.push(chunk),
        };

        // Generate a new string from self.vec_segment and return it and the unicode width of the
        // character.
        (acc.join(""), ch(UnicodeString::str_display_width(chunk)))
    }

    /// Returns a new [String] option. Does not modify [self.string](UnicodeString::string).
    pub fn delete_char_at_display_col(&self, display_col: ChUnit) -> Option<String> {
        // There is no segment present (Deref trait makes `len()` apply to `vec_segment`).
        if self.len() == 0 {
            return None;
        }

        // There is only one segment present.
        if self.len() == 1 {
            return Some("".to_string());
        }

        // There are more than 1 segments present.
        let split_logical_index = self.logical_index_at_display_col_index(display_col)?;
        let max_logical_index = self.len();

        let mut vec_left = TinyVecBackingStore::with_capacity(self.len());
        let mut str_left_unicode_width = ch(0);
        {
            for logical_idx in 0..split_logical_index {
                let seg_at_logical_idx = self.at_logical_index(logical_idx)?;
                let string = self.get_str(seg_at_logical_idx);
                vec_left.push(string);
                str_left_unicode_width += seg_at_logical_idx.unicode_width;
            }
        }

        let skip_split_logical_index = split_logical_index + 1; // Drop one segment.
        let mut vec_right = TinyVecBackingStore::with_capacity(self.len());
        let mut str_right_unicode_width = ch(0);
        {
            for logical_idx in skip_split_logical_index..max_logical_index {
                let seg_at_logical_idx = self.at_logical_index(logical_idx)?;
                let string = self.get_str(seg_at_logical_idx);
                vec_right.push(string);
                str_right_unicode_width += seg_at_logical_idx.unicode_width;
            }
        }

        // Merge the two vectors.
        vec_left.append(&mut vec_right);
        Some(vec_left.join(""))
    }

    /// Does not modify [self.string](UnicodeString::string) & returns two new tuples:
    /// 1. *left* [String],
    /// 2. *right* [String].
    pub fn split_at_display_col(&self, display_col: ChUnit) -> Option<(String, String)> {
        let split_logical_index = self.logical_index_at_display_col_index(display_col)?;
        let max_logical_index = self.len();

        let mut vec_left = TinyVecBackingStore::with_capacity(self.len());
        let mut str_left_unicode_width = ch(0);
        {
            for logical_idx in 0..split_logical_index {
                let segment = self.at_logical_index(logical_idx)?;
                vec_left.push(self.get_str(segment));
                str_left_unicode_width += segment.unicode_width;
            }
        }

        let mut vec_right = TinyVecBackingStore::with_capacity(self.len());
        let mut str_right_unicode_width = ch(0);
        {
            for logical_idx in split_logical_index..max_logical_index {
                let seg_at_logical_idx = self.at_logical_index(logical_idx)?;
                vec_right.push(self.get_str(seg_at_logical_idx));
                str_right_unicode_width += seg_at_logical_idx.unicode_width;
            }
        }

        if str_right_unicode_width > ch(0) || str_left_unicode_width > ch(0) {
            Some((vec_left.join(""), vec_right.join("")))
        } else {
            None
        }
    }
}
