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

use crate::{Index, Length, idx, len};

/// There are two implementations of this trait:
/// - [super::RingBufferStack] which uses a fixed-size array on the stack.
/// - [super::RingBufferHeap] which uses a [Vec] on the heap.
pub trait RingBuffer<T, const N: usize> {
    fn len(&self) -> Length;

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

    fn as_slice(&self) -> &[Option<T>];
}
