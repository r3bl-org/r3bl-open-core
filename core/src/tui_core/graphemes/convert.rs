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

use std::borrow::Cow;

use smallstr::SmallString;
use smallvec::Array;

use crate::UnicodeString;

/// UnicodeStringExt trait.
pub trait UnicodeStringExt {
    fn unicode_string(&self) -> UnicodeString;
}

// PERF: [ ] perf
impl<A: Array<Item = u8>> UnicodeStringExt for SmallString<A> {
    fn unicode_string(&self) -> UnicodeString { UnicodeString::new(self.as_str()) }
}

// PERF: [ ] perf
impl<A: Array<Item = u8>> UnicodeStringExt for &SmallString<A> {
    fn unicode_string(&self) -> UnicodeString { UnicodeString::new(self.as_str()) }
}

impl UnicodeStringExt for dyn AsRef<str> {
    fn unicode_string(&self) -> UnicodeString { UnicodeString::new(self.as_ref()) }
}

impl UnicodeStringExt for Cow<'_, str> {
    fn unicode_string(&self) -> UnicodeString { UnicodeString::new(self) }
}

impl UnicodeStringExt for &str {
    fn unicode_string(&self) -> UnicodeString { UnicodeString::new(self) }
}

impl UnicodeStringExt for &&str {
    fn unicode_string(&self) -> UnicodeString { UnicodeString::new(self) }
}

impl UnicodeStringExt for String {
    fn unicode_string(&self) -> UnicodeString { UnicodeString::new(self) }
}

// REFACTOR: [x] write test for all of the above!
#[cfg(test)]
mod tests_unicode_string_ext {
    use super::*;

    #[test]
    fn test_unicode_string_for_smallstring() {
        let small_str: SmallString<[u8; 16]> = SmallString::from("hello");
        assert_eq!(small_str.unicode_string(), UnicodeString::new("hello"));
    }

    #[test]
    fn test_unicode_string_for_cow() {
        let cow_str: Cow<str> = Cow::Borrowed("hello");
        assert_eq!(cow_str.unicode_string(), UnicodeString::new("hello"));
    }

    #[test]
    fn test_unicode_string_for_str() {
        let str_slice: &str = "hello";
        assert_eq!(str_slice.unicode_string(), UnicodeString::new("hello"));
    }

    #[test]
    fn test_unicode_string_for_string() {
        let string: String = String::from("hello");
        assert_eq!(string.unicode_string(), UnicodeString::new("hello"));
    }
}
