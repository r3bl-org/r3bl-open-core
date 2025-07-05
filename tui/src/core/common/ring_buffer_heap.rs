/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

//! A fixed-size ring buffer implementation using heap allocation. For stack allocated
//! version, take a look at [`super::RingBufferStack`].

use std::fmt::Debug;

use super::RingBuffer;
use crate::{len, Index, Length};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RingBufferHeap<T, const N: usize> {
    internal_storage: Vec<Option<T>>,
    head: usize,
    tail: usize,
    count: usize,
}

impl<T, const N: usize> Default for RingBufferHeap<T, N> {
    fn default() -> Self { Self::new() }
}

impl<T, const N: usize> RingBufferHeap<T, N> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            internal_storage: Vec::with_capacity(N),
            head: 0,
            tail: 0,
            count: 0,
        }
    }
}

impl<T, const N: usize> RingBuffer<T, N> for RingBufferHeap<T, N> {
    fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.count = 0;
        self.internal_storage.iter_mut().for_each(|x| *x = None);
    }

    fn get(&self, index_arg: impl Into<Index>) -> Option<&T> {
        let index = {
            let it: Index = index_arg.into();
            it.as_usize()
        };

        if index >= self.count {
            return None;
        }
        let actual_index = (self.tail + index) % N;
        self.internal_storage
            .get(actual_index)
            .and_then(|item| item.as_ref())
    }

    fn len(&self) -> Length { len(self.count) }

    /// Insert at head (ie, insert the newest item).
    fn add(&mut self, value: T) {
        if self.count == N {
            let _unused: Option<_> = self.remove();
        }
        if self.internal_storage.len() < N {
            self.internal_storage.push(Some(value));
        } else {
            self.internal_storage[self.head] = Some(value);
        }
        self.head = (self.head + 1) % N;
        self.count = std::cmp::min(self.count + 1, N); // Make sure count doesn't exceed
                                                       // capacity
    }

    /// Remove from tail (ie, remove the oldest item).
    fn remove(&mut self) -> Option<T> {
        if self.count == 0 {
            return None;
        }

        if self.internal_storage.is_empty() {
            return None;
        }

        let value = self.internal_storage[self.tail].take();
        self.tail = (self.tail + 1) % N;
        self.count -= 1;
        value
    }

    /// Remove from head (ie, remove the newest item). This is the opposite of
    /// [`Self::remove`].
    fn remove_head(&mut self) -> Option<T> {
        if self.count == 0 {
            return None;
        }

        if self.internal_storage.is_empty() {
            return None;
        }

        self.head = (self.head + N - 1) % N;
        let value = self.internal_storage[self.head].take();
        self.count -= 1;
        value
    }

    // Delete the items from the given index to the end of the buffer.
    fn truncate(&mut self, arg_index: impl Into<Index>) {
        let index = {
            let it: Index = arg_index.into();
            it.as_usize()
        };

        if index >= self.count {
            return;
        }

        let actual_index = (self.tail + index) % N;

        // Clear elements from actual_index to the end of the buffer.
        for i in 0..N {
            let wrapped_index = (actual_index + i) % N;
            if i < self.count - index {
                self.internal_storage[wrapped_index] = None;
            } else {
                break;
            }
        }

        self.head = actual_index;
        self.count = index;
    }

    fn as_slice_raw(&self) -> &[Option<T>] { &self.internal_storage }
}

impl<T, const N: usize> RingBufferHeap<T, N> {
    #[must_use]
    pub fn iter(&self) -> RingBufferHeapIterator<'_, T, N> {
        RingBufferHeapIterator {
            ring_buffer: self,
            iterator_index: 0,
        }
    }
}

/// This implementation allows the ring buffer to be used in a for loop directly.
impl<'a, T, const N: usize> IntoIterator for &'a RingBufferHeap<T, N> {
    type Item = &'a T;
    type IntoIter = RingBufferHeapIterator<'a, T, N>;

    fn into_iter(self) -> Self::IntoIter { self.iter() }
}

#[derive(Debug)]
pub struct RingBufferHeapIterator<'a, T, const N: usize> {
    ring_buffer: &'a RingBufferHeap<T, N>,
    iterator_index: usize,
}

impl<'a, T, const N: usize> Iterator for RingBufferHeapIterator<'a, T, N> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iterator_index == self.ring_buffer.count {
            return None;
        }

        let actual_index = (self.ring_buffer.tail + self.iterator_index) % N;
        self.iterator_index += 1;
        self.ring_buffer
            .internal_storage
            .get(actual_index)
            .and_then(|x| x.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use smallstr::SmallString;

    use super::*;
    pub type SmallStringBackingStore = SmallString<[u8; DEFAULT_SMALL_STRING_SIZE]>;
    pub const DEFAULT_SMALL_STRING_SIZE: usize = 32;

    #[test]
    fn test_empty_ring_buffer_heap() {
        let ring_buffer: RingBufferHeap<SmallStringBackingStore, 3> =
            RingBufferHeap::new();
        assert_eq!(ring_buffer.len(), len(0));
        assert_eq!(ring_buffer.head, 0);
        assert_eq!(ring_buffer.tail, 0);
        assert_eq!(ring_buffer.count, 0);
        let mut iter = ring_buffer.iter();
        assert_eq!(iter.next(), None);

        assert_eq!(ring_buffer.get(0), None);
        assert_eq!(ring_buffer.get(1), None);
        assert_eq!(ring_buffer.get(2), None);

        assert_eq!(ring_buffer.first(), None);
        assert_eq!(ring_buffer.last(), None);
    }

    #[test]
    fn test_normal_insert_heap() {
        let mut ring_buffer: RingBufferHeap<SmallStringBackingStore, 3> =
            RingBufferHeap::new();
        ring_buffer.add("Hello".into());
        assert_eq!(ring_buffer.len(), len(1));
        assert_eq!(ring_buffer.head, 1);
        assert_eq!(ring_buffer.tail, 0);
        assert_eq!(ring_buffer.count, 1);
        let mut iter = ring_buffer.iter();
        assert_eq!(iter.next().unwrap(), "Hello");
        assert_eq!(iter.next(), None);

        assert_eq!(ring_buffer.get(0).unwrap(), "Hello");
        assert_eq!(ring_buffer.get(1), None);
        assert_eq!(ring_buffer.get(2), None);

        assert_eq!(ring_buffer.first().unwrap(), "Hello");
        assert_eq!(ring_buffer.last().unwrap(), "Hello");
    }

    #[test]
    fn test_multiple_inserts_heap() {
        let mut ring_buffer: RingBufferHeap<SmallStringBackingStore, 3> =
            RingBufferHeap::new();
        ring_buffer.add("Hello".into());
        ring_buffer.add("World".into());
        ring_buffer.add("Rust".into());
        assert_eq!(ring_buffer.len(), len(3));
        assert_eq!(ring_buffer.head, 0);
        assert_eq!(ring_buffer.tail, 0);
        assert_eq!(ring_buffer.count, 3);
        let mut iter = ring_buffer.iter();
        assert_eq!(iter.next().unwrap(), "Hello");
        assert_eq!(iter.next().unwrap(), "World");
        assert_eq!(iter.next().unwrap(), "Rust");
        assert_eq!(iter.next(), None);

        assert_eq!(ring_buffer.get(0).unwrap(), "Hello");
        assert_eq!(ring_buffer.get(1).unwrap(), "World");
        assert_eq!(ring_buffer.get(2).unwrap(), "Rust");
        assert_eq!(ring_buffer.get(3), None);

        assert_eq!(ring_buffer.first().unwrap(), "Hello");
        assert_eq!(ring_buffer.last().unwrap(), "Rust");
    }

    #[test]
    fn test_normal_remove_heap() {
        let mut ring_buffer: RingBufferHeap<SmallStringBackingStore, 3> =
            RingBufferHeap::new();
        ring_buffer.add("Hello".into());
        ring_buffer.add("World".into());
        ring_buffer.add("Rust".into());
        ring_buffer.remove();
        assert_eq!(ring_buffer.len(), len(2));
        assert_eq!(ring_buffer.head, 0);
        assert_eq!(ring_buffer.tail, 1);
        assert_eq!(ring_buffer.count, 2);
        let mut iter = ring_buffer.iter();
        assert_eq!(iter.next().unwrap(), "World");
        assert_eq!(iter.next().unwrap(), "Rust");
        assert_eq!(iter.next(), None);

        assert_eq!(ring_buffer.get(0).unwrap(), "World");
        assert_eq!(ring_buffer.get(1).unwrap(), "Rust");
        assert_eq!(ring_buffer.get(2), None);
        assert_eq!(ring_buffer.get(3), None);

        assert_eq!(ring_buffer.first().unwrap(), "World");
        assert_eq!(ring_buffer.last().unwrap(), "Rust");
    }

    #[test]
    fn test_wrap_around_insert_heap() {
        let mut ring_buffer: RingBufferHeap<SmallStringBackingStore, 3> =
            RingBufferHeap::new();
        ring_buffer.add("Hello".into());
        ring_buffer.add("World".into());
        ring_buffer.add("Rust".into());
        ring_buffer.add("R3BL".into());
        assert_eq!(ring_buffer.len(), len(3));
        assert_eq!(ring_buffer.head, 1);
        assert_eq!(ring_buffer.tail, 1);
        assert_eq!(ring_buffer.count, 3);
        let mut iter = ring_buffer.iter();
        assert_eq!(iter.next().unwrap(), "World");
        assert_eq!(iter.next().unwrap(), "Rust");
        assert_eq!(iter.next().unwrap(), "R3BL");
        assert_eq!(iter.next(), None);

        assert_eq!(ring_buffer.get(0).unwrap(), "World");
        assert_eq!(ring_buffer.get(1).unwrap(), "Rust");
        assert_eq!(ring_buffer.get(2).unwrap(), "R3BL");
        assert_eq!(ring_buffer.get(3), None);

        assert_eq!(ring_buffer.first().unwrap(), "World");
        assert_eq!(ring_buffer.last().unwrap(), "R3BL");
    }

    #[test]
    fn test_wrap_around_remove_heap() {
        let mut ring_buffer: RingBufferHeap<SmallStringBackingStore, 3> =
            RingBufferHeap::new();
        ring_buffer.add("Hello".into());
        ring_buffer.add("World".into());
        ring_buffer.add("Rust".into());
        ring_buffer.add("R3BL".into());
        ring_buffer.remove();
        assert_eq!(ring_buffer.len(), len(2));
        assert_eq!(ring_buffer.head, 1);
        assert_eq!(ring_buffer.tail, 2);
        assert_eq!(ring_buffer.count, 2);
        let mut iter = ring_buffer.iter();
        assert_eq!(iter.next().unwrap(), "Rust");
        assert_eq!(iter.next().unwrap(), "R3BL");
        assert_eq!(iter.next(), None);

        assert_eq!(ring_buffer.get(0).unwrap(), "Rust");
        assert_eq!(ring_buffer.get(1).unwrap(), "R3BL");
        assert_eq!(ring_buffer.get(2), None);
        assert_eq!(ring_buffer.get(3), None);

        assert_eq!(ring_buffer.first().unwrap(), "Rust");
        assert_eq!(ring_buffer.last().unwrap(), "R3BL");
    }

    #[test]
    fn test_clear_heap() {
        let mut ring_buffer: RingBufferHeap<SmallStringBackingStore, 3> =
            RingBufferHeap::new();
        ring_buffer.add("Hello".into());
        ring_buffer.add("World".into());
        ring_buffer.add("Rust".into());
        ring_buffer.clear();
        assert_eq!(ring_buffer.len(), len(0));
        assert_eq!(ring_buffer.head, 0);
        assert_eq!(ring_buffer.tail, 0);
        assert_eq!(ring_buffer.count, 0);
        let mut iter = ring_buffer.iter();
        assert_eq!(iter.next(), None);

        assert_eq!(ring_buffer.get(0), None);
        assert_eq!(ring_buffer.get(1), None);
        assert_eq!(ring_buffer.get(2), None);
        assert_eq!(ring_buffer.get(3), None);

        assert_eq!(ring_buffer.first(), None);
        assert_eq!(ring_buffer.last(), None);
    }

    #[test]
    fn test_normal_truncate() {
        // Vec::truncate() behavior for comparison.
        let mut vec: Vec<String> = vec![];
        vec.push("Hello".into());
        vec.push("World".into());
        vec.push("Rust".into());
        vec.truncate(2);
        assert_eq!(vec.len(), 2);
        assert_eq!(vec.first().unwrap(), "Hello");
        assert_eq!(vec.get(1).unwrap(), "World");
        assert_eq!(vec.get(2), None);

        let mut ring_buffer: RingBufferHeap<SmallStringBackingStore, 3> =
            RingBufferHeap::new();
        ring_buffer.add("Hello".into());
        ring_buffer.add("World".into());
        ring_buffer.add("Rust".into());
        ring_buffer.truncate(2);

        assert_eq!(ring_buffer.len(), len(2));
        assert_eq!(ring_buffer.head, 2);
        assert_eq!(ring_buffer.tail, 0);
        assert_eq!(ring_buffer.count, 2);

        let mut iter = ring_buffer.iter();
        assert_eq!(iter.next().unwrap(), "Hello");
        assert_eq!(iter.next().unwrap(), "World");
        assert_eq!(iter.next(), None);

        assert_eq!(ring_buffer.get(0).unwrap(), "Hello");
        assert_eq!(ring_buffer.get(1).unwrap(), "World");
        assert_eq!(ring_buffer.get(2), None);
        assert_eq!(ring_buffer.get(3), None);

        assert_eq!(ring_buffer.first().unwrap(), "Hello");
        assert_eq!(ring_buffer.last().unwrap(), "World");
    }

    #[test]
    fn test_wrap_around_truncate() {
        // Vec::truncate() behavior for comparison.
        let mut vec: Vec<String> = vec![];
        vec.push("Hello".into());
        vec.push("World".into());
        vec.push("Rust".into());
        vec.truncate(2);
        assert_eq!(vec.len(), 2);
        assert_eq!(vec.first().unwrap(), "Hello");
        assert_eq!(vec.get(1).unwrap(), "World");
        assert_eq!(vec.get(2), None);

        let mut ring_buffer: RingBufferHeap<SmallStringBackingStore, 3> =
            RingBufferHeap::new();
        ring_buffer.add("Hello".into());
        ring_buffer.add("World".into());
        ring_buffer.add("Rust".into());
        ring_buffer.add("R3BL".into());
        ring_buffer.truncate(2);

        assert_eq!(ring_buffer.len(), len(2));
        assert_eq!(ring_buffer.head, 0);
        assert_eq!(ring_buffer.tail, 1);
        assert_eq!(ring_buffer.count, 2);

        assert_eq!(ring_buffer.get(0).unwrap(), "World");
        assert_eq!(ring_buffer.get(1).unwrap(), "Rust");
        assert_eq!(ring_buffer.get(2), None);
        assert_eq!(ring_buffer.get(3), None);

        let mut iter = ring_buffer.iter();
        assert_eq!(iter.next().unwrap(), "World");
        assert_eq!(iter.next().unwrap(), "Rust");
        assert_eq!(iter.next(), None);

        assert_eq!(ring_buffer.first().unwrap(), "World");
        assert_eq!(ring_buffer.last().unwrap(), "Rust");
    }

    #[test]
    fn test_into_iterator_implementation() {
        let mut ring_buffer: RingBufferHeap<SmallStringBackingStore, 3> =
            RingBufferHeap::new();
        ring_buffer.add("Hello".into());
        ring_buffer.add("World".into());
        ring_buffer.add("Rust".into());

        // Test that we can use the ring buffer directly in a for loop (this is why
        // IntoIterator is needed!)
        let mut collected = Vec::new();
        for item in &ring_buffer {
            collected.push(item.clone());
        }

        assert_eq!(collected.len(), 3);
        assert_eq!(collected[0], "Hello");
        assert_eq!(collected[1], "World");
        assert_eq!(collected[2], "Rust");

        // Test using for loop with explicit into_iter() call
        let mut explicit_collected = Vec::new();
        for item in (&ring_buffer).into_iter() {
            explicit_collected.push(item.clone());
        }
        assert_eq!(collected, explicit_collected);

        // Test using for loop to find specific items
        let mut found_world = false;
        for item in &ring_buffer {
            if item == "World" {
                found_world = true;
                break;
            }
        }
        assert!(found_world);

        // Test using for loop with enumerate to get indices
        for (index, item) in (&ring_buffer).into_iter().enumerate() {
            match index {
                0 => assert_eq!(item, "Hello"),
                1 => assert_eq!(item, "World"),
                2 => assert_eq!(item, "Rust"),
                _ => panic!("Unexpected index: {index}"),
            }
        }

        // Compare with manual iter() usage (without for loop)
        let iter_results: Vec<_> = ring_buffer.iter().cloned().collect();
        assert_eq!(iter_results, collected);
    }
}
