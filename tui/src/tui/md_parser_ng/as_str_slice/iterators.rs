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

use crate::{as_str_slice::AsStrSlice,
            core::tui_core::units::{idx, Index}};

/// Iterator over the characters in an [AsStrSlice].
pub struct StringChars<'a> {
    slice: AsStrSlice<'a>,
}

impl<'a> StringChars<'a> {
    /// Creates a new iterator over the characters in the given slice.
    pub fn new(slice: AsStrSlice<'a>) -> Self { Self { slice } }
}

impl<'a> Iterator for StringChars<'a> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        let ch = self.slice.current_char();
        if ch.is_some() {
            self.slice.advance();
        }
        ch
    }
}

/// Iterator over the characters in an [AsStrSlice] with their indices.
pub struct StringCharIndices<'a> {
    slice: AsStrSlice<'a>,
    position: Index,
}

impl<'a> StringCharIndices<'a> {
    pub fn new(slice: AsStrSlice<'a>) -> Self {
        Self {
            slice,
            position: idx(0),
        }
    }
}

impl<'a> Iterator for StringCharIndices<'a> {
    type Item = (usize, char);

    fn next(&mut self) -> Option<Self::Item> {
        let ch = self.slice.current_char()?;
        let pos = self.position.as_usize();
        self.slice.advance();
        self.position += idx(1);
        Some((pos, ch))
    }
}
