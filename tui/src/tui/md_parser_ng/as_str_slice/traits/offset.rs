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

use nom::Offset;

use crate::as_str_slice::AsStrSlice;

/// Implement [Offset] trait for [AsStrSlice]. This is required for the
/// [nom::combinator::recognize] parser to work.
impl<'a> Offset for AsStrSlice<'a> {
    fn offset(&self, second: &Self) -> usize {
        // Calculate the character offset between two AsStrSlice instances.
        // The second slice must be a part of self (advanced from self).

        // If they point to different line arrays, we can't calculate a meaningful offset.
        if !std::ptr::eq(self.lines.as_ptr(), second.lines.as_ptr()) {
            return 0;
        }

        // If second is before self, return 0 (invalid case).
        if second.line_index.as_usize() < self.line_index.as_usize()
            || (second.line_index == self.line_index
                && second.char_index.as_usize() < self.char_index.as_usize())
        {
            return 0;
        }

        let mut offset = 0;

        // Count characters from self's position to second's position.
        let mut current_line = self.line_index.as_usize();
        let mut current_char = self.char_index.as_usize();

        while current_line < second.line_index.as_usize()
            || (current_line == second.line_index.as_usize()
                && current_char < second.char_index.as_usize())
        {
            if current_line >= self.lines.len() {
                break;
            }

            let line = &self.lines[current_line].string;

            if current_line < second.line_index.as_usize() {
                // Count remaining characters in current line
                if current_char < line.len() {
                    offset += line.len() - current_char;
                }
                // Add synthetic newline if not the last line
                if current_line < self.lines.len() - 1 {
                    offset += 1;
                }
                // Move to next line
                current_line += 1;
                current_char = 0;
            } else {
                // We're on the same line as second, count up to second's char_index
                let end_char = second.char_index.as_usize().min(line.len());
                if current_char < end_char {
                    offset += end_char - current_char;
                }
                break;
            }
        }

        offset
    }
}
