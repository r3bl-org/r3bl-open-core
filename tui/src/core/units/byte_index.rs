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

use std::ops::{Deref, DerefMut};

use crate::ChUnit;

/// Represents a byte index inside of the underlying [`crate::InlineString`] of
/// [`crate::GCString`].
#[derive(Debug, Copy, Clone, Default, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct ByteIndex(pub usize);

impl ByteIndex {
    #[must_use] pub fn as_usize(&self) -> usize { self.0 }
}

pub fn byte_index(arg_byte_index: impl Into<ByteIndex>) -> ByteIndex {
    arg_byte_index.into()
}

impl Deref for ByteIndex {
    type Target = usize;
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for ByteIndex {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl From<usize> for ByteIndex {
    fn from(it: usize) -> Self { Self(it) }
}

impl From<ChUnit> for ByteIndex {
    fn from(it: ChUnit) -> Self { Self(crate::usize(it)) }
}