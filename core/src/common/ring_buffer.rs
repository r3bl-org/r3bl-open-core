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

//! A fixed-size ring buffer implementation. There are two variants of this [RingBuffer]
//! based on which [RingBufferStorage] you choose:
//! - [RingBufferStorage::Stack]: This variant uses a fixed-size array to store the
//!   elements. You can use the [RingBuffer::new_stack()] method to create a new instance
//!   of this variant.
//! - [RingBufferStorage::Heap]: This variant uses a `Vec` to store the elements. You can
//!   use the [RingBuffer::new_heap()] method to create a new instance of this variant.
//!
//! The [RingBuffer] struct is a generic data structure that allows for efficient
//! insertion and removal of elements in a circular buffer. It maintains a fixed capacity
//! and overwrites the oldest elements when the buffer is full. It behaves like a queue
//! with a fixed size [RingBuffer::add()] and [RingBuffer::remove()] methods.
//!
//! # Type Parameters
//!
//! * `T`: The type of elements stored in the ring buffer.
//! * `N`: The fixed capacity of the ring buffer.
//!
//! # Fields
//!
//! * `internal_storage`: An array of `Option<T>` used to store the elements if the stack
//!   variant is used. Or a `Vec<T>` if the heap variant is used.
//! * `head`: The index of the next insertion point.
//! * `tail`: The index of the next removal point.
//! * `count`: The current number of elements in the buffer.
//!
//! # Modules
//!
//! * `constructor`: Contains the implementation of the [RingBuffer::new_heap()] and
//!   [RingBuffer::new_stack()] methods and the `Default` trait.
//! * `mutator`: Contains methods for inserting and removing elements.
//! * `size`: Contains methods for querying the size and state of the buffer.
//! * `iterator`: Contains the implementation of an iterator for the ring buffer.
//!
//! # Examples
//!
//! ```
//! use r3bl_core::RingBuffer;
//!
//! let mut ring_buffer: RingBuffer<i32, 3> = RingBuffer::new_stack();
//!
//! ring_buffer.add(1);
//! ring_buffer.add(2);
//! ring_buffer.add(3);
//!
//! assert_eq!(ring_buffer.len(), 3);
//! assert_eq!(ring_buffer.is_full(), true);
//! assert_eq!(ring_buffer.iter().collect::<Vec<&i32>>(), vec![&1, &2, &3]);
//!
//! ring_buffer.remove();
//!
//! assert_eq!(ring_buffer.len(), 2);
//! assert_eq!(ring_buffer.is_empty(), false);
//! assert_eq!(ring_buffer.iter().collect::<Vec<&i32>>(), vec![&2, &3]);
//! ```

#[derive(Clone, Debug, PartialEq)]
pub enum RingBufferStorage<T, const N: usize> {
    Stack([Option<T>; N]),
    Heap(Vec<Option<T>>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct RingBuffer<T, const N: usize> {
    internal_storage: RingBufferStorage<T, N>,
    head: usize,
    tail: usize,
    count: usize,
}

mod constructor {
    use super::*;

    impl<T, const N: usize> RingBuffer<T, N> {
        pub fn new_alloc_on_stack() -> Self {
            RingBuffer {
                internal_storage: RingBufferStorage::Stack([(); N].map(|_| None)),
                head: 0,
                tail: 0,
                count: 0,
            }
        }

        pub fn new_alloc_on_heap() -> Self {
            RingBuffer {
                internal_storage: RingBufferStorage::Heap(Vec::with_capacity(N)),
                head: 0,
                tail: 0,
                count: 0,
            }
        }
    }

    impl<T, const N: usize> Default for RingBuffer<T, N> {
        /// The default implementation is the "stack" variant.
        fn default() -> Self { Self::new_alloc_on_stack() }
    }
}

mod mutator {
    use super::*;

    impl<T, const N: usize> RingBuffer<T, N> {
        /// Insert at head (ie, insert the newest item).
        pub fn add(&mut self, value: T) {
            match &mut self.internal_storage {
                RingBufferStorage::Stack(arr) => {
                    if self.count == N {
                        arr[self.head] = Some(value);
                        self.head = (self.head + 1) % N;
                        self.tail = (self.tail + 1) % N;
                    } else {
                        arr[self.head] = Some(value);
                        self.head = (self.head + 1) % N;
                        self.count += 1;
                    }
                }
                RingBufferStorage::Heap(vec) => {
                    if self.count == N {
                        vec[self.head] = Some(value);
                        self.head = (self.head + 1) % N;
                        self.tail = (self.tail + 1) % N;
                    } else {
                        vec.push(Some(value));
                        self.head = (self.head + 1) % N;
                        self.count += 1;
                    }
                }
            }
        }

        /// Remove from tail (ie, remove the oldest item).
        pub fn remove(&mut self) -> Option<T> {
            if self.count == 0 {
                return None;
            }

            match &mut self.internal_storage {
                RingBufferStorage::Stack(arr) => {
                    let value = arr[self.tail].take();
                    self.tail = (self.tail + 1) % N;
                    self.count -= 1;
                    value
                }
                RingBufferStorage::Heap(vec) => {
                    let value = vec[self.tail].take();
                    self.tail = (self.tail + 1) % N;
                    self.count -= 1;
                    value
                }
            }
        }

        pub fn clear(&mut self) {
            self.head = 0;
            self.tail = 0;
            self.count = 0;
        }
    }
}

mod size {
    use super::*;

    impl<T, const N: usize> RingBuffer<T, N> {
        pub fn len(&self) -> usize { self.count }

        pub fn is_empty(&self) -> bool { self.count == 0 }

        pub fn is_full(&self) -> bool { self.count == N }
    }
}

mod iterator {
    use super::*;

    pub struct RingBufferIterator<'a, T, const N: usize> {
        ring_buffer: &'a RingBuffer<T, N>,
        iterator_index: usize,
    }

    impl<'a, T, const N: usize> Iterator for RingBufferIterator<'a, T, N> {
        type Item = &'a T;

        fn next(&mut self) -> Option<Self::Item> {
            if self.iterator_index == self.ring_buffer.count {
                return None;
            }

            let actual_index = (self.ring_buffer.tail + self.iterator_index) % N;
            self.iterator_index += 1;

            match &self.ring_buffer.internal_storage {
                RingBufferStorage::Stack(arr) => arr[actual_index].as_ref(),
                RingBufferStorage::Heap(vec) => vec[actual_index].as_ref(),
            }
        }
    }

    impl<T, const N: usize> RingBuffer<T, N> {
        pub fn iter(&self) -> RingBufferIterator<T, N> {
            RingBufferIterator {
                ring_buffer: self,
                iterator_index: 0,
            }
        }
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
        let ring_buffer: RingBuffer<SmallStringBackingStore, 3> = RingBuffer::new_alloc_on_stack();
        assert_eq!(ring_buffer.len(), 0);
        assert_eq!(ring_buffer.head, 0);
        assert_eq!(ring_buffer.tail, 0);
        assert_eq!(ring_buffer.count, 0);
        let mut iter = ring_buffer.iter();
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_empty_ring_buffer_heap() {
        let ring_buffer: RingBuffer<SmallStringBackingStore, 3> = RingBuffer::new_alloc_on_heap();
        assert_eq!(ring_buffer.len(), 0);
        assert_eq!(ring_buffer.head, 0);
        assert_eq!(ring_buffer.tail, 0);
        assert_eq!(ring_buffer.count, 0);
        let mut iter = ring_buffer.iter();
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_normal_insert_stack() {
        let mut ring_buffer: RingBuffer<SmallStringBackingStore, 3> =
            RingBuffer::new_alloc_on_stack();
        ring_buffer.add("Hello".into());
        assert_eq!(ring_buffer.len(), 1);
        assert_eq!(ring_buffer.head, 1);
        assert_eq!(ring_buffer.tail, 0);
        assert_eq!(ring_buffer.count, 1);
        let mut iter = ring_buffer.iter();
        assert_eq!(iter.next().unwrap(), "Hello");
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_normal_insert_heap() {
        let mut ring_buffer: RingBuffer<SmallStringBackingStore, 3> =
            RingBuffer::new_alloc_on_heap();
        ring_buffer.add("Hello".into());
        assert_eq!(ring_buffer.len(), 1);
        assert_eq!(ring_buffer.head, 1);
        assert_eq!(ring_buffer.tail, 0);
        assert_eq!(ring_buffer.count, 1);
        let mut iter = ring_buffer.iter();
        assert_eq!(iter.next().unwrap(), "Hello");
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_multiple_inserts_stack() {
        let mut ring_buffer: RingBuffer<SmallStringBackingStore, 3> =
            RingBuffer::new_alloc_on_stack();
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
    }

    #[test]
    fn test_multiple_inserts_heap() {
        let mut ring_buffer: RingBuffer<SmallStringBackingStore, 3> =
            RingBuffer::new_alloc_on_heap();
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
    }

    #[test]
    fn test_normal_remove_stack() {
        let mut ring_buffer: RingBuffer<SmallStringBackingStore, 3> =
            RingBuffer::new_alloc_on_stack();
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
    }

    #[test]
    fn test_normal_remove_heap() {
        let mut ring_buffer: RingBuffer<SmallStringBackingStore, 3> =
            RingBuffer::new_alloc_on_heap();
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
    }

    #[test]
    fn test_wrap_around_insert_stack() {
        let mut ring_buffer: RingBuffer<SmallStringBackingStore, 3> =
            RingBuffer::new_alloc_on_stack();
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
    }

    #[test]
    fn test_wrap_around_insert_heap() {
        let mut ring_buffer: RingBuffer<SmallStringBackingStore, 3> =
            RingBuffer::new_alloc_on_heap();
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
    }

    #[test]
    fn test_wrap_around_remove_stack() {
        let mut ring_buffer: RingBuffer<SmallStringBackingStore, 3> =
            RingBuffer::new_alloc_on_stack();
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
    }

    #[test]
    fn test_wrap_around_remove_heap() {
        let mut ring_buffer: RingBuffer<SmallStringBackingStore, 3> =
            RingBuffer::new_alloc_on_heap();
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
    }

    #[test]
    fn test_clear_stack() {
        let mut ring_buffer: RingBuffer<SmallStringBackingStore, 3> =
            RingBuffer::new_alloc_on_stack();
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
    }

    #[test]
    fn test_clear_heap() {
        let mut ring_buffer: RingBuffer<SmallStringBackingStore, 3> =
            RingBuffer::new_alloc_on_heap();
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
    }
}
