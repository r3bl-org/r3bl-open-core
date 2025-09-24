// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::ops::{Deref, DerefMut};

use crate::{ChUnit, Index};

/// Represents a byte index inside of the underlying [`crate::InlineString`] of
/// [`crate::GCStringOwned`].
#[derive(Debug, Copy, Clone, Default, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct ByteIndex(pub usize);

impl ByteIndex {
    #[must_use]
    pub fn as_usize(&self) -> usize { self.0 }
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

impl From<ByteIndex> for Index {
    fn from(it: ByteIndex) -> Self { Self::from(it.0) }
}
