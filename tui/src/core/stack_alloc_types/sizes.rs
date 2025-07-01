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

// PERF: If you make this number too large, eg: more than 16, then it will slow down the
// editor performance
pub const DEFAULT_STRING_STORAGE_SIZE: usize = 16;

use std::fmt::Display;

use smallstr::SmallString;
use smallvec::SmallVec;

/// Intermediate struct used to insert a grapheme cluster segment into an existing unicode
/// string. When this gets larger than [`INLINE_VEC_SIZE`], it will be
/// [`smallvec::SmallVec::spilled`] on the heap.
pub type InlineVecStr<'a> = InlineVec<&'a str>;

/// Stack allocated string storage for small strings. When this gets larger than
/// [`DEFAULT_STRING_STORAGE_SIZE`], it will be [`smallvec::SmallVec::spilled`] on the
/// heap.
pub type InlineString = SmallString<[u8; DEFAULT_STRING_STORAGE_SIZE]>;

/// Replacement for [`std::borrow::Cow`] that uses [`InlineString`] if it is owned.
/// And `&str` if it is borrowed.
#[derive(Clone, Debug, PartialEq)]
pub enum InlineStringCow<'a> {
    Borrowed(&'a str),
    Owned(InlineString),
}

impl<'a> InlineStringCow<'a> {
    #[must_use]
    pub fn new_empty_borrowed() -> Self { InlineStringCow::Borrowed("") }
    #[must_use]
    pub fn new_borrowed(arg: &'a str) -> Self { InlineStringCow::Borrowed(arg) }
    #[must_use]
    pub fn new_owned(arg: InlineString) -> Self { InlineStringCow::Owned(arg) }
}

impl AsRef<str> for InlineStringCow<'_> {
    fn as_ref(&self) -> &str {
        match self {
            Self::Borrowed(s) => s,
            Self::Owned(s) => s.as_str(),
        }
    }
}

impl Display for InlineStringCow<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Borrowed(as_ref_str) => write!(f, "{as_ref_str}"),
            Self::Owned(as_ref_str) => write!(f, "{as_ref_str}"),
        }
    }
}

/// Stack allocated tiny string storage for small char sequences. When this gets larger
/// than [`DEFAULT_CHAR_STORAGE_SIZE`], it will be [`smallvec::SmallVec::spilled`] on the
/// heap.
pub type TinyInlineString = SmallString<[u8; DEFAULT_CHAR_STORAGE_SIZE]>;
pub const DEFAULT_CHAR_STORAGE_SIZE: usize = 4;

/// Stack allocated string storage for small documents. When this gets larger than
/// [`DEFAULT_DOCUMENT_SIZE`], it will be [`smallvec::SmallVec::spilled`] on the heap.
pub type DocumentStorage = SmallString<[u8; DEFAULT_DOCUMENT_SIZE]>;
/// 128KB, or approximately 2200 lines of Markdown text (assuming 60 chars per line).
pub const DEFAULT_DOCUMENT_SIZE: usize = 131_072;

// 16KB buffer for reasonable performance on Linux, which typically has a 4KB page size. A
// page is a fixed sized block of memory, and memory is managed in terms of pages. It is
// the fundamental unit of memory management in Linux, and it is used to manage virtual
// memory, physical memory, and memory mapped files.
pub const DEFAULT_READ_BUFFER_SIZE: usize = 16384;

/// Stack allocated list, that can [`smallvec::SmallVec::spilled`] into the heap if it
/// gets larger than [`INLINE_VEC_SIZE`].
pub type InlineVec<T> = SmallVec<[T; INLINE_VEC_SIZE]>;
pub const INLINE_VEC_SIZE: usize = 8;
