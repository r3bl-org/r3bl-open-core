/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

use crate::GCString;

/// `GCStringExt` trait that allows converting lots of different types into
/// [`GCString`].
///
/// Once converted to a [`GCString`], the text can be manipulated using the three
/// index types: [`ByteIndex`](super::ByteIndex), [`SegIndex`](super::SegIndex),
/// and [`ColIndex`](super::ColIndex) for proper Unicode handling.
pub trait GCStringExt {
    fn grapheme_string(&self) -> GCString;
}

mod convert_impl_blocks {
    use super::{Array, Cow, GCString, GCStringExt, SmallString};

    impl<A: Array<Item = u8>> GCStringExt for SmallString<A> {
        fn grapheme_string(&self) -> GCString { GCString::new(self.as_str()) }
    }

    impl<A: Array<Item = u8>> GCStringExt for &SmallString<A> {
        fn grapheme_string(&self) -> GCString { GCString::new(self.as_str()) }
    }

    impl GCStringExt for dyn AsRef<str> {
        fn grapheme_string(&self) -> GCString { GCString::new(self.as_ref()) }
    }

    impl GCStringExt for Cow<'_, str> {
        fn grapheme_string(&self) -> GCString { GCString::new(self) }
    }

    impl GCStringExt for &str {
        fn grapheme_string(&self) -> GCString { GCString::new(self) }
    }

    impl GCStringExt for &&str {
        fn grapheme_string(&self) -> GCString { GCString::new(self) }
    }

    impl GCStringExt for String {
        fn grapheme_string(&self) -> GCString { GCString::new(self) }
    }
}

#[cfg(test)]
mod tests_unicode_string_ext {
    use super::*;

    #[test]
    fn test_unicode_string_for_smallstring() {
        let small_str: SmallString<[u8; 16]> = SmallString::from("hello");
        assert_eq!(small_str.grapheme_string(), GCString::new("hello"));
    }

    #[test]
    fn test_unicode_string_for_cow() {
        let cow_str: Cow<'_, str> = Cow::Borrowed("hello");
        assert_eq!(cow_str.grapheme_string(), GCString::new("hello"));
    }

    #[test]
    fn test_unicode_string_for_str() {
        let str_slice: &str = "hello";
        assert_eq!(str_slice.grapheme_string(), GCString::new("hello"));
    }

    #[test]
    fn test_unicode_string_for_string() {
        let string: String = String::from("hello");
        assert_eq!(string.grapheme_string(), GCString::new("hello"));
    }

    #[test]
    fn test_unicode_string_for_ref_small_str() {
        let small_str: SmallString<[u8; 16]> = SmallString::from("hello");
        let ref_small_str: &SmallString<[u8; 16]> = &small_str;
        assert_eq!(ref_small_str.grapheme_string(), GCString::new("hello"));
    }

    #[test]
    fn test_unicode_string_for_dyn_as_ref_str() {
        let string: String = String::from("hello");
        let dyn_as_ref_str: &dyn AsRef<str> = &string;
        assert_eq!(dyn_as_ref_str.grapheme_string(), GCString::new("hello"));
    }

    #[test]
    fn test_unicode_string_for_ref_str() {
        let str_slice: &str = "hello";
        let ref_str_slice: &&str = &str_slice;
        assert_eq!(ref_str_slice.grapheme_string(), GCString::new("hello"));
    }
}
