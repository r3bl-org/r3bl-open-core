// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{Index, InlineVec, Length, idx, len};

/// There are two implementations of this trait:
/// - [`super::RingBufferStack`] which uses a fixed-size array on the stack.
/// - [`super::RingBufferHeap`] which uses a [Vec] on the heap.
pub trait RingBuffer<T, const N: usize> {
    fn len(&self) -> Length;

    fn is_full(&self) -> bool { self.len() == len(N) }

    fn clear(&mut self);

    fn get(&self, arg_index: impl Into<Index>) -> Option<&T>;

    fn is_empty(&self) -> bool { self.len() == len(0) }

    fn first(&self) -> Option<&T> { self.get(idx(0)) }

    fn last(&self) -> Option<&T> { self.get(self.len().convert_to_index()) }

    fn add(&mut self, value: T);

    fn remove(&mut self) -> Option<T>;

    fn remove_head(&mut self) -> Option<T>;

    fn truncate(&mut self, arg_index: impl Into<Index>);

    fn push(&mut self, value: T) { self.add(value); }

    fn pop(&mut self) -> Option<T> { self.remove_head() }

    /// Returns a view of the underlying internal storage of the struct that implements
    /// this trait.
    fn as_slice_raw(&self) -> &[Option<T>];

    /// Take a [`RingBuffer::as_slice_raw`] which yields an slice of [`Option<&T>`], then
    /// remove the [None] items, and return a [`InlineVec<&T>`].
    /// - This uses [`Iterator::filter_map`] function.
    /// - Even though `T` is not cloned, the collection has to be allocated and moved to
    ///   the caller, via return. A slice can't be returned because it would be owned by
    ///   this function.
    fn as_slice(&self) -> InlineVec<&T> {
        let slice = self.as_slice_raw();
        let acc: InlineVec<&T> =
            slice.iter().filter_map(|style| style.as_ref()).collect();
        acc
    }
}
