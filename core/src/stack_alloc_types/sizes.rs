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

//! Be very careful when adjusting these tuning parameters. The rule of thumb is that
//! smaller static allocation sizes are better than larger. There is a tradeoff between
//! pre-allocating large amounts of memory and allocating small amounts (on the heap) as
//! you need it. Also huge stack allocations can cause stack overflow errors. Please test
//! your changes extensively using the demo examples in the `examples` directory to verify
//! that they actually speed things up and cause performance regressions.

// PERF: If you make this number too large, eg: more than 16, then it will slow down the editor performance
pub const DEFAULT_STRING_STORAGE_SIZE: usize = 16;

use smallstr::SmallString;
use smallvec::SmallVec;

/// Intermediate struct used to insert a grapheme cluster segment into an existing unicode
/// string. When this gets larger than [INLINE_VEC_SIZE], it will be
/// [smallvec::SmallVec::spilled] on the heap.
pub type InlineVecStr<'a> = InlineVec<&'a str>;

/// Stack allocated string storage for small strings. When this gets larger than
/// [DEFAULT_STRING_STORAGE_SIZE], it will be [smallvec::SmallVec::spilled] on the heap.
pub type InlineString = SmallString<[u8; DEFAULT_STRING_STORAGE_SIZE]>;

/// Stack allocated really small string storage for small char sequences. When this gets
/// larger than [DEFAULT_CHAR_STORAGE_SIZE], it will be [smallvec::SmallVec::spilled] on
/// the heap.
pub type TinyInlineString = SmallString<[u8; DEFAULT_CHAR_STORAGE_SIZE]>;
pub const DEFAULT_CHAR_STORAGE_SIZE: usize = 4;

/// Stack allocated string storage for small documents. When this gets larger than
/// [DEFAULT_DOCUMENT_SIZE], it will be [smallvec::SmallVec::spilled] on the heap.
pub type DocumentStorage = SmallString<[u8; DEFAULT_DOCUMENT_SIZE]>;
/// 128KB, or approximately 2200 lines of Markdown text (assuming 60 chars per line).
pub const DEFAULT_DOCUMENT_SIZE: usize = 131072;

// 16KB buffer for reasonable performance on Linux, which typically has a 4KB page size. A
// page is a fixed sized block of memory, and memory is managed in terms of pages. It is
// the fundamental unit of memory management in Linux, and it is used to manage virtual
// memory, physical memory, and memory mapped files.
pub const DEFAULT_READ_BUFFER_SIZE: usize = 16384;

/// Stack allocated list, that can [smallvec::SmallVec::spilled] into the heap if it gets
/// larger than [INLINE_VEC_SIZE].
pub type InlineVec<T> = SmallVec<[T; INLINE_VEC_SIZE]>;
pub const INLINE_VEC_SIZE: usize = 8;

/// Takes a slice of:
/// - `&str`
/// - `Vec<&str>`
/// - `Vec<String>`
/// - `InlineVec<&str>`
///
/// And converts it into `InlineVec` of type `O`.
///
/// # Examples
///
/// ```rust
/// use r3bl_core::{to_inline_vec, InlineVec, InlineString};
///
/// _ = to_inline_vec(&vec!["one", "two", "three"]);
/// _ = to_inline_vec(&["one", "two", "three"]);
/// _ = to_inline_vec(&vec!["one".to_string(), "two".to_string(), "three".to_string()]);
/// _ = to_inline_vec(&{
///     let items: InlineVec<&str> = smallvec::smallvec!["one", "two", "three"];
///     items
/// });
/// ```
pub fn to_inline_vec(arg_items: &[impl AsRef<str>]) -> InlineVec<InlineString> {
    let mut inline_vec = InlineVec::new();
    for item in arg_items {
        inline_vec.push(item.as_ref().into());
    }
    inline_vec
}

#[test]
fn test_convert_to_inline_vec() {
    // Case 1: vec!["one", "two", "three"]
    {
        let items = vec!["one", "two", "three"];
        let inline_vec = to_inline_vec(&items); // Removed explicit type parameter
        assert_eq!(inline_vec.len(), 3);
        assert_eq!(inline_vec[0], "one");
        assert_eq!(inline_vec[1], "two");
        assert_eq!(inline_vec[2], "three");
    }

    // Case 2: &["one", "two", "three"]
    {
        let items = &["one", "two", "three"];
        let inline_vec = to_inline_vec(items); // Removed explicit type parameter
        assert_eq!(inline_vec.len(), 3);
        assert_eq!(inline_vec[0], "one");
        assert_eq!(inline_vec[1], "two");
        assert_eq!(inline_vec[2], "three");
    }

    // Case 3: Vec<String>
    {
        let items: Vec<String> =
            vec!["one".to_string(), "two".to_string(), "three".to_string()];
        let inline_vec = to_inline_vec(&items); // Removed explicit type parameter
        assert_eq!(inline_vec.len(), 3);
        assert_eq!(inline_vec[0], "one");
        assert_eq!(inline_vec[1], "two");
        assert_eq!(inline_vec[1], "two");
        assert_eq!(inline_vec[2], "three");
    }

    // Case 4: smallvec::smallvec!["one", "two", "three"]
    {
        let items: InlineVec<&str> = smallvec::smallvec!["one", "two", "three"];
        let inline_vec = to_inline_vec(&items); // Removed explicit type parameter
        assert_eq!(inline_vec.len(), 3);
        assert_eq!(inline_vec[0], "one");
        assert_eq!(inline_vec[1], "two");
        assert_eq!(inline_vec[2], "three");
    }
}
