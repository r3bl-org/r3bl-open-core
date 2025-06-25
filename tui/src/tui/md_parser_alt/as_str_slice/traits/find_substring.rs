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

use nom::FindSubstring;

use crate::AsStrSlice;

/// Implement [FindSubstring] trait for [AsStrSlice]. This is required by the
/// [nom::bytes::complete::take_until] parser function.
impl<'a> FindSubstring<&str> for AsStrSlice<'a> {
    fn find_substring(&self, sub_str: &str) -> Option<usize> {
        // Convert the AsStrSlice to a string representation.
        let full_text = self.extract_to_slice_end();

        // Find the substring in the full text.
        full_text.as_ref().find(sub_str)
    }
}
