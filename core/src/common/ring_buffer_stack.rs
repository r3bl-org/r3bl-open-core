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

//! A fixed-size ring buffer implementation using stack allocation. Be careful of the size
//! of the buffer, since if it is too large, you might get a stack overflow error. For a
//! heap allocated version, take a look at [super::RingBufferHeap].

use std::fmt::Debug;

#[derive(Clone, Debug, PartialEq)]
pub struct RingBufferStack<T, const N: usize> {
    internal_storage: [Option<T>; N],
    head: usize,
    tail: usize,
    count: usize,
}

impl<T, const N: usize> Default for RingBufferStack<T, N> {
    fn default() -> Self { Self::new() }
}

impl<T, const N: usize> RingBufferStack<T, N> {
    pub fn new() -> Self {
        RingBufferStack {
            internal_storage: [(); N].map(|_| None),
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    /// Insert at head (ie, insert the newest item).
    pub fn add(&mut self, value: T) {
        if self.count == N {
            self.internal_storage[self.head] = Some(value);
            self.head = (self.head + 1) % N;
            self.tail = (self.tail + 1) % N;
        } else {
            self.internal_storage[self.head] = Some(value);
            self.head = (self.head + 1) % N;
            self.count += 1;
        }
    }

    /// Remove from tail (ie, remove the oldest item).
    pub fn remove(&mut self) -> Option<T> {
        if self.count == 0 {
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

        self.head = (self.head + N - 1) % N;
        let value = self.internal_storage[self.head].take();
        self.count -= 1;
        value
    }

    // Delete the items from the given index to the end of the buffer.
    pub fn truncate(&mut self, index: usize) {
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

    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.count = 0;
        self.internal_storage.iter_mut().for_each(|x| *x = None);
    }

    pub fn len(&self) -> usize { self.count }

    pub fn is_empty(&self) -> bool { self.count == 0 }

    pub fn is_full(&self) -> bool { self.count == N }

    pub fn iter(&self) -> RingBufferStackIterator<'_, T, N> {
        RingBufferStackIterator {
            ring_buffer: self,
            iterator_index: 0,
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.count {
            return None;
        }

        let actual_index = (self.tail + index) % N;
        self.internal_storage[actual_index].as_ref()
    }

    pub fn first(&self) -> Option<&T> { self.get(0) }
    pub fn last(&self) -> Option<&T> { self.get(self.count.saturating_sub(1)) }
}

pub struct RingBufferStackIterator<'a, T, const N: usize> {
    ring_buffer: &'a RingBufferStack<T, N>,
    iterator_index: usize,
}

impl<'a, T, const N: usize> Iterator for RingBufferStackIterator<'a, T, N> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iterator_index == self.ring_buffer.count {
            return None;
        }

        let actual_index = (self.ring_buffer.tail + self.iterator_index) % N;
        self.iterator_index += 1;
        self.ring_buffer.internal_storage[actual_index].as_ref()
    }
}

#[cfg(test)]
mod tests {
    use smallstr::SmallString;

    use super::*;
    pub type SmallStringBackingStore = SmallString<[u8; DEFAULT_SMALL_STRING_SIZE]>;
    pub const DEFAULT_SMALL_STRING_SIZE: usize = 32;

    #[test]
    fn test_empty_ring_buffer_stack() {
        let ring_buffer: RingBufferStack<SmallStringBackingStore, 3> =
            RingBufferStack::new();
        assert_eq!(ring_buffer.len(), 0);
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
    fn test_normal_insert_stack() {
        let mut ring_buffer: RingBufferStack<SmallStringBackingStore, 3> =
            RingBufferStack::new();
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

        assert_eq!(ring_buffer.first().unwrap(), "Hello");
        assert_eq!(ring_buffer.last().unwrap(), "Hello");
    }

    #[test]
    fn test_multiple_inserts_stack() {
        let mut ring_buffer: RingBufferStack<SmallStringBackingStore, 3> =
            RingBufferStack::new();
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

        assert_eq!(ring_buffer.first().unwrap(), "Hello");
        assert_eq!(ring_buffer.last().unwrap(), "Rust");
    }

    #[test]
    fn test_normal_remove_stack() {
        let mut ring_buffer: RingBufferStack<SmallStringBackingStore, 3> =
            RingBufferStack::new();
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

        assert_eq!(ring_buffer.first().unwrap(), "World");
        assert_eq!(ring_buffer.last().unwrap(), "Rust");
    }

    #[test]
    fn test_wrap_around_insert_stack() {
        let mut ring_buffer: RingBufferStack<SmallStringBackingStore, 3> =
            RingBufferStack::new();
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

        assert_eq!(ring_buffer.first().unwrap(), "World");
        assert_eq!(ring_buffer.last().unwrap(), "R3BL");
    }

    #[test]
    fn test_wrap_around_remove_stack() {
        let mut ring_buffer: RingBufferStack<SmallStringBackingStore, 3> =
            RingBufferStack::new();
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

        assert_eq!(ring_buffer.first().unwrap(), "Rust");
        assert_eq!(ring_buffer.last().unwrap(), "R3BL");
    }

    #[test]
    fn test_clear_stack() {
        let mut ring_buffer: RingBufferStack<SmallStringBackingStore, 3> =
            RingBufferStack::new();
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

        let mut ring_buffer: RingBufferStack<SmallStringBackingStore, 3> =
            RingBufferStack::new();
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

        let mut ring_buffer: RingBufferStack<SmallStringBackingStore, 3> =
            RingBufferStack::new();
        ring_buffer.add("Hello".into());
        ring_buffer.add("World".into());
        ring_buffer.add("Rust".into());
        ring_buffer.add("R3BL".into());
        ring_buffer.truncate(2);

        assert_eq!(ring_buffer.len(), 2);
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
}
