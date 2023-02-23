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
    /// Returns a new ([UnicodeString], [ChUnit]) tuple. Does not modify
    /// [self.string](UnicodeString::string).
    pub fn insert_char_at_display_col(
        &self,
        display_col: ChUnit,
        chunk: &str,
    ) -> Option<(
        /* new string */ UnicodeString,
        /* display width of chunk */ ChUnit,
    )> {
        // Insert self grapheme cluster to self.vec_segment.
        let mut acc = Vec::with_capacity(self.len() + 1);
        for item in self.vec_segment.iter() {
            acc.push(item.string.as_str());
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
        let new_string = acc.join("");

        // In the caller - update the caret position based on the unicode width of the character.
        let new_unicode_string = UnicodeString::from(new_string);
        let chunk_display_width: ChUnit = ch!(UnicodeString::str_display_width(chunk));
        Some((new_unicode_string, chunk_display_width))
    }

    /// Returns a new [UnicodeString] option. Does not modify [self.string](UnicodeString::string).
    pub fn delete_char_at_display_col(&self, display_col: ChUnit) -> Option<UnicodeString> {
        // There is no segment present (Deref trait makes `len()` apply to `vec_segment`).
        if self.len() == 0 {
            return None;
        }

        // There is only one segment present.
        if self.len() == 1 {
            return Some(UnicodeString::default());
        }

        // There are more than 1 segments present.
        let split_logical_index = self.logical_index_at_display_col_index(display_col)?;
        let max_logical_index = self.len();

        let mut vec_left = Vec::with_capacity(self.len());
        let mut str_left_unicode_width = ch!(0);
        {
            for logical_idx in 0..split_logical_index {
                let segment = self.at_logical_index(logical_idx)?;
                vec_left.push(segment.string.as_str());
                str_left_unicode_width += segment.unicode_width;
            }
        }

        let skip_split_logical_index = split_logical_index + 1; // Drop one segment.
        let mut vec_right = Vec::with_capacity(self.len());
        let mut str_right_unicode_width = ch!(0);
        {
            for logical_idx in skip_split_logical_index..max_logical_index {
                let seg_at_logical_idx = self.at_logical_index(logical_idx)?;
                vec_right.push(seg_at_logical_idx.string.as_str());
                str_right_unicode_width += seg_at_logical_idx.unicode_width;
            }
        }

        // Merge the two vectors.
        vec_left.append(&mut vec_right);
        let new_string = vec_left.join("");
        Some(UnicodeString::from(new_string))
    }

    /// Does not modify [self.string](UnicodeString::string) & returns two new tuples:
    /// 1. *left* [UnicodeString],
    /// 2. *right* [UnicodeString].
    pub fn split_at_display_col(
        &self,
        display_col: ChUnit,
    ) -> Option<(UnicodeString, UnicodeString)> {
        let split_logical_index = self.logical_index_at_display_col_index(display_col)?;
        let max_logical_index = self.len();

        let mut vec_left = Vec::with_capacity(self.len());
        let mut str_left_unicode_width = ch!(0);
        {
            for logical_idx in 0..split_logical_index {
                let segment = self.at_logical_index(logical_idx)?;
                vec_left.push(segment.string.as_ref());
                str_left_unicode_width += segment.unicode_width;
            }
        }

        let mut vec_right = Vec::with_capacity(self.len());
        let mut str_right_unicode_width = ch!(0);
        {
            for logical_idx in split_logical_index..max_logical_index {
                let seg_at_logical_idx = self.at_logical_index(logical_idx)?;
                vec_right.push(seg_at_logical_idx.string.as_ref());
                str_right_unicode_width += seg_at_logical_idx.unicode_width;
            }
        }

        if str_right_unicode_width > ch!(0) || str_left_unicode_width > ch!(0) {
            Some((
                UnicodeString::from(vec_left.join("")),
                UnicodeString::from(vec_right.join("")),
            ))
        } else {
            None
        }
    }
}
