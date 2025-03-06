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

use smallstr::SmallString;
use smallvec::SmallVec;

/// Intermediate struct used to insert a grapheme cluster segment into an existing unicode
/// string. When this gets larger than `DEFAULT_STRING_SIZE`, it will be
/// [smallvec::SmallVec::spilled] on the heap.
pub type VecArrayStr<'a> = SmallVec<[&'a str; VEC_STR_BUFFER_CAPACITY]>;
const VEC_STR_BUFFER_CAPACITY: usize = 16;

/// Stack allocated string storage for small strings. When this gets larger than
/// `DEFAULT_STRING_SIZE`, it will be [smallvec::SmallVec::spilled] on the heap.
pub type StringStorage = SmallString<[u8; DEFAULT_STRING_STORAGE_SIZE]>;

// PERF: If you make this number too large, eg: more than 16, then it will slow down the editor performance
pub const DEFAULT_STRING_STORAGE_SIZE: usize = 16;

/// Stack allocated string storage for small chars. When this gets larger than
/// `DEFAULT_CHAR_SIZE`, it will be [smallvec::SmallVec::spilled] on the heap.
pub type CharStorage = SmallString<[u8; DEFAULT_CHAR_STORAGE_SIZE]>;
pub const DEFAULT_CHAR_STORAGE_SIZE: usize = 4;

/// Stack allocated string storage for small documents. When this gets larger than
/// `DEFAULT_DOCUMENT_SIZE`, it will be [smallvec::SmallVec::spilled] on the heap.
pub type DocumentStorage = SmallString<[u8; DEFAULT_DOCUMENT_SIZE]>;
/// 128KB, or approximately 2200 lines of Markdown text (assuming 60 chars per line).
pub const DEFAULT_DOCUMENT_SIZE: usize = 131072;

// 16KB buffer for reasonable performance on Linux, which typically has a 4KB page size. A
// page is a fixed sized block of memory, and memory is managed in terms of pages. It is
// the fundamental unit of memory management in Linux, and it is used to manage virtual
// memory, physical memory, and memory mapped files.
pub const DEFAULT_READ_BUFFER_SIZE: usize = 16384;

/// Stack allocated list, that can [smallvec::SmallVec::spilled] into the heap if it gets
/// larger than `VEC_ARRAY_SIZE`.
pub type VecArray<T> = SmallVec<[T; VEC_ARRAY_SIZE]>;
pub const VEC_ARRAY_SIZE: usize = 8;
