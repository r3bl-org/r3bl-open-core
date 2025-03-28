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

use crate::ASTStyle;

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

/// Attributes are: color_fg, color_bg, bold, dim, italic, underline, reverse, hidden,
/// etc. which are in [crate::ASTStyle].
pub const MAX_ANSI_STYLED_TEXT_STYLE_ATTRIB_SIZE: usize = 12;
pub type InlineVecASTStyles =
    SmallVec<[ASTStyle; MAX_ANSI_STYLED_TEXT_STYLE_ATTRIB_SIZE]>;
