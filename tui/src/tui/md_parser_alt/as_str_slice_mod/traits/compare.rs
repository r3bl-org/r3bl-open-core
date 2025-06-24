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

use std::convert::AsRef;

use nom::{Compare, CompareResult};

use crate::as_str_slice_mod::AsStrSlice;

/// The `Compare` trait in nom is not symmetric - you need to implement it in both
/// directions if you want to use both types interchangeably with the `tag` function.
impl<'a> Compare<&str> for AsStrSlice<'a> {
    fn compare(&self, t: &str) -> CompareResult {
        let mut current = self.clone();
        let mut target_chars = t.chars();

        loop {
            match (current.current_char(), target_chars.next()) {
                (Some(a), Some(b)) if a == b => {
                    current.advance();
                }
                (Some(_), Some(_)) => return CompareResult::Error,
                (None, Some(_)) => return CompareResult::Incomplete,
                (Some(_), None) => return CompareResult::Ok,
                (None, None) => return CompareResult::Ok,
            }
        }
    }

    fn compare_no_case(&self, t: &str) -> CompareResult {
        let mut current = self.clone();
        let mut target_chars = t.chars();

        loop {
            match (current.current_char(), target_chars.next()) {
                (Some(a), Some(b)) if a.to_lowercase().eq(b.to_lowercase()) => {
                    current.advance();
                }
                (Some(_), Some(_)) => return CompareResult::Error,
                (None, Some(_)) => return CompareResult::Incomplete,
                (Some(_), None) => return CompareResult::Ok,
                (None, None) => return CompareResult::Ok,
            }
        }
    }
}

/// The `Compare` trait in nom is not symmetric - you need to implement it in both
/// directions if you want to use both types interchangeably with the `tag` function.
impl<'a> Compare<AsStrSlice<'a>> for &str {
    fn compare(&self, t: AsStrSlice<'a>) -> CompareResult {
        // Convert AsStrSlice to string and compare with self
        let t_str = t.extract_to_slice_end();
        self.compare(t_str.as_ref())
    }

    fn compare_no_case(&self, t: AsStrSlice<'a>) -> CompareResult {
        // Convert AsStrSlice to string and compare with self (case insensitive)
        let t_str = t.extract_to_slice_end();
        self.compare_no_case(t_str.as_ref())
    }
}

/// The `Compare` trait needs to be implemented to compare `AsStrSlice` with each other
/// for the `tag` function.
impl<'a> Compare<AsStrSlice<'a>> for AsStrSlice<'a> {
    fn compare(&self, t: AsStrSlice<'a>) -> CompareResult {
        // Convert both AsStrSlice instances to strings and compare
        let self_str = self.extract_to_slice_end();
        let t_str = t.extract_to_slice_end();
        self_str.as_ref().compare(t_str.as_ref())
    }

    fn compare_no_case(&self, t: AsStrSlice<'a>) -> CompareResult {
        // Convert both AsStrSlice instances to strings and compare (case insensitive)
        let self_str = self.extract_to_slice_end();
        let t_str = t.extract_to_slice_end();
        self_str.as_ref().compare_no_case(t_str.as_ref())
    }
}
