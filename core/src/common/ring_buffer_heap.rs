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

//! A fixed-size ring buffer implementation using heap allocation.

use std::{fmt::Debug,
          ops::{Index, IndexMut}};

#[derive(Clone, Debug, PartialEq)]
pub struct RingBufferHeap<T, const N: usize> {
    internal_storage: Vec<Option<T>>,
    head: usize,
    tail: usize,
    count: usize,
}

impl<T, const N: usize> RingBufferHeap<T, N> {
    pub fn new() -> Self {
        RingBufferHeap {
            internal_storage: Vec::with_capacity(N),
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    /// Insert at head (ie, insert the newest item).
    pub fn add(&mut self, value: T) {
        if self.count == N {
            let _ = self.remove(); // Remove the oldest element to make space.
        }
        if self.internal_storage.len() < N {
            self.internal_storage.push(Some(value));
        } else {
            self.internal_storage[self.head] = Some(value);
        }
        self.head = (self.head + 1) % N;
        self.count = std::cmp::min(self.count + 1, N); // Make sure count doesn't exceed capacity
    }

    /// Remove from tail (ie, remove the oldest item).
    pub fn remove(&mut self) -> Option<T> {
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
    /// [Self::remove].
    pub fn remove_head(&mut self) -> Option<T> {
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

    // Shortens the buffer, keeping the first `new_len` elements and dropping the rest.
    pub fn truncate(&mut self, new_len: usize) {
        if new_len >= self.count {
            return;
        }

        while self.count > new_len {
            self.remove_head();
        }
    }

    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.count = 0;
        self.internal_storage.iter_mut().for_each(|x| *x = None);
    }

    pub fn len(&self) -> usize { self.count }

    pub fn is_empty(&self) -> bool { self.count == 0 }

    pub fn is_full(&self) -> bool { self.count == N }

    pub fn iter(&self) -> RingBufferHeapIterator<'_, T, N> {
        RingBufferHeapIterator {
            ring_buffer: self,
            iterator_index: 0,
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.count {
            return None;
        }

        let actual_index = (self.tail + index) % N;
        self.internal_storage
            .get(actual_index)
            .and_then(|x| x.as_ref())
    }
}

impl<T, const N: usize> Index<usize> for RingBufferHeap<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.count {
            panic!(
                "Index out of bounds: the len is {} but the index is {}",
                self.count, index
            );
        }

        let actual_index = (self.tail + index) % N;
        self.internal_storage[actual_index]
            .as_ref()
            .expect("Should be Some")
    }
}

impl<T, const N: usize> IndexMut<usize> for RingBufferHeap<T, N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.count {
            panic!(
                "Index out of bounds: the len is {} but the index is {}",
                self.count, index
            );
        }

        let actual_index = (self.tail + index) % N;
        self.internal_storage[actual_index]
            .as_mut()
            .expect("Should be Some")
    }
}

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
        assert_eq!(ring_buffer.len(), 0);
        assert_eq!(ring_buffer.head, 0);
        assert_eq!(ring_buffer.tail, 0);
        assert_eq!(ring_buffer.count, 0);
        let mut iter = ring_buffer.iter();
        assert_eq!(iter.next(), None);

        assert_eq!(ring_buffer.get(0), None);
        assert_eq!(ring_buffer.get(1), None);
        assert_eq!(ring_buffer.get(2), None);
    }

    #[test]
    fn test_normal_insert_heap() {
        let mut ring_buffer: RingBufferHeap<SmallStringBackingStore, 3> =
            RingBufferHeap::new();
        ring_buffer.add("Hello".into());
        assert_eq!(ring_buffer.len(), 1);
        assert_eq!(ring_buffer.head, 1);
        assert_eq!(ring_buffer.tail, 0);
        assert_eq!(ring_buffer.count, 1);
        let mut iter = ring_buffer.iter();
        assert_eq!(iter.next().unwrap(), "Hello");
        assert_eq!(iter.next(), None);

        assert_eq!(ring_buffer.get(0).unwrap(), "Hello");
        assert_eq!(ring_buffer.get(1), None);
        assert_eq!(ring_buffer.get(2), None);
    }

    #[test]
    fn test_multiple_inserts_heap() {
        let mut ring_buffer: RingBufferHeap<SmallStringBackingStore, 3> =
            RingBufferHeap::new();
        ring_buffer.add("Hello".into());
        ring_buffer.add("World".into());
        ring_buffer.add("Rust".into());
        assert_eq!(ring_buffer.len(), 3);
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
    }

    #[test]
    fn test_normal_remove_heap() {
        let mut ring_buffer: RingBufferHeap<SmallStringBackingStore, 3> =
            RingBufferHeap::new();
        ring_buffer.add("Hello".into());
        ring_buffer.add("World".into());
        ring_buffer.add("Rust".into());
        ring_buffer.remove();
        assert_eq!(ring_buffer.len(), 2);
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
    }

    #[test]
    fn test_wrap_around_insert_heap() {
        let mut ring_buffer: RingBufferHeap<SmallStringBackingStore, 3> =
            RingBufferHeap::new();
        ring_buffer.add("Hello".into());
        ring_buffer.add("World".into());
        ring_buffer.add("Rust".into());
        ring_buffer.add("R3BL".into());
        assert_eq!(ring_buffer.len(), 3);
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
        assert_eq!(ring_buffer.len(), 2);
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
    }

    #[test]
    fn test_clear_heap() {
        let mut ring_buffer: RingBufferHeap<SmallStringBackingStore, 3> =
            RingBufferHeap::new();
        ring_buffer.add("Hello".into());
        ring_buffer.add("World".into());
        ring_buffer.add("Rust".into());
        ring_buffer.clear();
        assert_eq!(ring_buffer.len(), 0);
        assert_eq!(ring_buffer.head, 0);
        assert_eq!(ring_buffer.tail, 0);
        assert_eq!(ring_buffer.count, 0);
        let mut iter = ring_buffer.iter();
        assert_eq!(iter.next(), None);

        assert_eq!(ring_buffer.get(0), None);
        assert_eq!(ring_buffer.get(1), None);
        assert_eq!(ring_buffer.get(2), None);
        assert_eq!(ring_buffer.get(3), None);
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
        assert_eq!(vec[0], "Hello");
        assert_eq!(vec[1], "World");

        let mut ring_buffer: RingBufferHeap<SmallStringBackingStore, 3> =
            RingBufferHeap::new();
        ring_buffer.add("Hello".into());
        ring_buffer.add("World".into());

        ring_buffer.add("Rust".into());
        ring_buffer.truncate(2);

        assert_eq!(ring_buffer.len(), 2);
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
        assert_eq!(vec[0], "Hello");
        assert_eq!(vec[1], "World");

        let mut ring_buffer: RingBufferHeap<SmallStringBackingStore, 3> =
            RingBufferHeap::new();
        ring_buffer.add("Hello".into());
        ring_buffer.add("World".into());
        ring_buffer.add("Rust".into());
        ring_buffer.add("R3BL".into());
        ring_buffer.truncate(2);

        assert_eq!(ring_buffer.len(), 2);
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
    }
}
