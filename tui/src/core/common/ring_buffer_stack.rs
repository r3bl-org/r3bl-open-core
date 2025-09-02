// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! A fixed-size ring buffer implementation using stack allocation. Be careful of the size
//! of the buffer, since if it is too large, you might get a stack overflow error. For a
//! heap allocated version, take a look at [`super::RingBufferHeap`].

use std::fmt::Debug;

use super::RingBuffer;
use crate::{Index, Length, len};

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
    #[must_use]
    pub fn new() -> Self {
        RingBufferStack {
            internal_storage: [(); N].map(|()| None),
            head: 0,
            tail: 0,
            count: 0,
        }
    }
}

impl<T, const N: usize> RingBuffer<T, N> for RingBufferStack<T, N> {
    fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.count = 0;
        self.internal_storage.iter_mut().for_each(|x| *x = None);
    }

    fn get(&self, arg_index: impl Into<Index>) -> Option<&T> {
        let index = {
            let it: Index = arg_index.into();
            it.as_usize()
        };

        if index >= self.count {
            return None;
        }

        let actual_index = (self.tail + index) % N;
        self.internal_storage[actual_index].as_ref()
    }

    fn len(&self) -> Length { len(self.count) }

    /// Insert at head (ie, insert the newest item).
    fn add(&mut self, value: T) {
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
    fn remove(&mut self) -> Option<T> {
        if self.count == 0 {
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

    fn get_mut(&mut self, arg_index: impl Into<Index>) -> Option<&mut T> {
        let index = {
            let it: Index = arg_index.into();
            it.as_usize()
        };

        if index >= self.count {
            return None;
        }

        let actual_index = (self.tail + index) % N;
        self.internal_storage[actual_index].as_mut()
    }

    fn set(&mut self, arg_index: impl Into<Index>, value: T) -> Option<()> {
        let index = {
            let it: Index = arg_index.into();
            it.as_usize()
        };

        if index >= self.count {
            return None;
        }

        let actual_index = (self.tail + index) % N;
        self.internal_storage[actual_index] = Some(value);
        Some(())
    }
}

impl<T, const N: usize> RingBufferStack<T, N> {
    pub fn iter(&self) -> RingBufferStackIterator<'_, T, N> {
        RingBufferStackIterator {
            ring_buffer: self,
            iterator_index: 0,
        }
    }
}

/// This implementation allows the ring buffer to be used in a for loop directly.
impl<'a, T, const N: usize> IntoIterator for &'a RingBufferStack<T, N> {
    type Item = &'a T;
    type IntoIter = RingBufferStackIterator<'a, T, N>;

    fn into_iter(self) -> Self::IntoIter { self.iter() }
}

#[derive(Debug)]
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
        assert_eq!(ring_buffer.len(), 0.into());
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
        assert_eq!(ring_buffer.len(), 1.into());
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
        assert_eq!(ring_buffer.len(), 3.into());
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
        assert_eq!(ring_buffer.len(), 2.into());
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
        assert_eq!(ring_buffer.len(), 3.into());
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
        assert_eq!(ring_buffer.len(), 2.into());
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
        assert_eq!(ring_buffer.len(), 0.into());
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

        assert_eq!(ring_buffer.len(), 2.into());
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

        assert_eq!(ring_buffer.len(), 2.into());
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
        let mut ring_buffer: RingBufferStack<SmallStringBackingStore, 3> =
            RingBufferStack::new();
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
        for item in &ring_buffer {
            explicit_collected.push(item.clone());
        }
        assert_eq!(collected, explicit_collected);

        // Test using for loop to find specific items
        let mut found_rust = false;
        for item in &ring_buffer {
            if item == "Rust" {
                found_rust = true;
                break;
            }
        }
        assert!(found_rust);

        // Test using for loop with enumerate to get indices
        for (index, item) in (&ring_buffer).into_iter().enumerate() {
            match index {
                0 => assert_eq!(item, "Hello"),
                1 => assert_eq!(item, "World"),
                2 => assert_eq!(item, "Rust"),
                _ => panic!("Unexpected index: {index}"),
            }
        }

        // Test using for loop to count items that meet a condition
        let mut count_long_names = 0;
        for item in &ring_buffer {
            if item.len() >= 5 {
                count_long_names += 1;
            }
        }
        assert_eq!(count_long_names, 2); // "Hello" and "World"

        // Compare with manual iter() usage (without for loop)
        let iter_results: Vec<_> = ring_buffer.iter().cloned().collect();
        assert_eq!(iter_results, collected);
    }

    #[test]
    fn test_get_mut() {
        let mut buffer = RingBufferStack::<i32, 5>::new();

        // Add some elements
        buffer.add(10);
        buffer.add(20);
        buffer.add(30);

        // Test mutable access
        if let Some(val) = buffer.get_mut(0) {
            *val = 15;
        }
        if let Some(val) = buffer.get_mut(2) {
            *val = 35;
        }

        assert_eq!(buffer.get(0), Some(&15)); // Modified
        assert_eq!(buffer.get(1), Some(&20));
        assert_eq!(buffer.get(2), Some(&35)); // Modified

        // Test out of bounds
        assert_eq!(buffer.get_mut(3), None);
        assert_eq!(buffer.get_mut(10), None);
    }

    #[test]
    fn test_set() {
        let mut buffer = RingBufferStack::<i32, 5>::new();

        // Add some elements
        buffer.add(10);
        buffer.add(20);
        buffer.add(30);

        // Test setting values
        assert_eq!(buffer.set(0, 15), Some(()));
        assert_eq!(buffer.set(2, 35), Some(()));

        assert_eq!(buffer.get(0), Some(&15));
        assert_eq!(buffer.get(1), Some(&20));
        assert_eq!(buffer.get(2), Some(&35));

        // Test out of bounds
        assert_eq!(buffer.set(3, 40), None);
        assert_eq!(buffer.set(10, 50), None);

        // Verify out of bounds didn't change anything
        assert_eq!(buffer.len(), 3.into());
    }

    #[test]
    fn test_get_mut_with_circular_buffer() {
        let mut buffer = RingBufferStack::<i32, 3>::new();

        // Fill the buffer
        buffer.add(1);
        buffer.add(2);
        buffer.add(3);

        // Add more to trigger circular behavior
        buffer.add(4); // Overwrites 1, buffer now: [2, 3, 4]

        // Modify middle element
        if let Some(val) = buffer.get_mut(1) {
            *val = 33;
        }

        assert_eq!(buffer.get(0), Some(&2));
        assert_eq!(buffer.get(1), Some(&33)); // Modified
        assert_eq!(buffer.get(2), Some(&4));
    }

    #[test]
    fn test_set_with_circular_buffer() {
        let mut buffer = RingBufferStack::<String, 3>::new();

        // Fill with strings
        buffer.add("first".to_string());
        buffer.add("second".to_string());
        buffer.add("third".to_string());

        // Trigger circular
        buffer.add("fourth".to_string()); // Buffer: ["second", "third", "fourth"]

        // Set new values
        assert_eq!(buffer.set(0, "SECOND".to_string()), Some(()));
        assert_eq!(buffer.set(2, "FOURTH".to_string()), Some(()));

        assert_eq!(buffer.get(0), Some(&"SECOND".to_string()));
        assert_eq!(buffer.get(1), Some(&"third".to_string()));
        assert_eq!(buffer.get(2), Some(&"FOURTH".to_string()));
    }

    #[test]
    fn test_get_mut_set_interaction() {
        let mut buffer = RingBufferStack::<Vec<i32>, 4>::new();

        // Add vectors
        buffer.add(vec![1, 2]);
        buffer.add(vec![3, 4]);
        buffer.add(vec![5, 6]);

        // Modify via get_mut
        if let Some(vec) = buffer.get_mut(0) {
            vec.push(3);
        }

        // Replace via set
        assert_eq!(buffer.set(1, vec![30, 40, 50]), Some(()));

        assert_eq!(buffer.get(0), Some(&vec![1, 2, 3]));
        assert_eq!(buffer.get(1), Some(&vec![30, 40, 50]));
        assert_eq!(buffer.get(2), Some(&vec![5, 6]));
    }

    #[test]
    fn test_remove_head() {
        let mut buffer = RingBufferStack::<SmallStringBackingStore, 3>::new();

        // Add elements
        buffer.add("Hello".into());
        buffer.add("World".into());
        buffer.add("Rust".into());

        // Remove head (newest item)
        let removed = buffer.remove_head();
        assert_eq!(removed, Some("Rust".into()));
        assert_eq!(buffer.len(), 2.into());

        // Check remaining items
        assert_eq!(buffer.get(0), Some(&"Hello".into()));
        assert_eq!(buffer.get(1), Some(&"World".into()));

        // Remove another
        let removed = buffer.remove_head();
        assert_eq!(removed, Some("World".into()));
        assert_eq!(buffer.len(), 1.into());

        // Check final item
        assert_eq!(buffer.get(0), Some(&"Hello".into()));

        // Empty the buffer
        let removed = buffer.remove_head();
        assert_eq!(removed, Some("Hello".into()));
        assert_eq!(buffer.len(), 0.into());

        // Try to remove from empty
        let removed = buffer.remove_head();
        assert_eq!(removed, None);
    }

    #[test]
    fn test_push_pop_aliases() {
        let mut buffer = RingBufferStack::<i32, 3>::new();

        // Test push (alias for add)
        buffer.push(10);
        buffer.push(20);
        buffer.push(30);

        assert_eq!(buffer.len(), 3.into());
        assert_eq!(buffer.get(0), Some(&10));
        assert_eq!(buffer.get(1), Some(&20));
        assert_eq!(buffer.get(2), Some(&30));

        // Test pop (alias for remove_head)
        let popped = buffer.pop();
        assert_eq!(popped, Some(30));
        assert_eq!(buffer.len(), 2.into());

        let popped = buffer.pop();
        assert_eq!(popped, Some(20));
        assert_eq!(buffer.len(), 1.into());

        let popped = buffer.pop();
        assert_eq!(popped, Some(10));
        assert_eq!(buffer.len(), 0.into());

        // Pop from empty
        let popped = buffer.pop();
        assert_eq!(popped, None);
    }

    #[test]
    fn test_is_full_is_empty() {
        let mut buffer = RingBufferStack::<i32, 3>::new();

        // Empty buffer
        assert!(buffer.is_empty());
        assert!(!buffer.is_full());

        // Partially filled
        buffer.add(10);
        assert!(!buffer.is_empty());
        assert!(!buffer.is_full());

        buffer.add(20);
        assert!(!buffer.is_empty());
        assert!(!buffer.is_full());

        // Full buffer
        buffer.add(30);
        assert!(!buffer.is_empty());
        assert!(buffer.is_full());

        // Overfill (circular)
        buffer.add(40);
        assert!(!buffer.is_empty());
        assert!(buffer.is_full());

        // Clear
        buffer.clear();
        assert!(buffer.is_empty());
        assert!(!buffer.is_full());
    }

    #[test]
    fn test_as_slice_methods() {
        let mut buffer = RingBufferStack::<String, 4>::new();

        // Empty buffer
        {
            let slice = buffer.as_slice();
            assert_eq!(slice.len(), 0);
        }

        {
            let raw_slice = buffer.as_slice_raw();
            assert_eq!(raw_slice.len(), 4);
            assert!(raw_slice.iter().all(std::option::Option::is_none));
        }

        // Add some elements
        buffer.add("Hello".to_string());
        buffer.add("World".to_string());
        buffer.add("Rust".to_string());

        // Test as_slice (filtered)
        {
            let slice = buffer.as_slice();
            assert_eq!(slice.len(), 3);
            assert_eq!(slice[0], &"Hello".to_string());
            assert_eq!(slice[1], &"World".to_string());
            assert_eq!(slice[2], &"Rust".to_string());
        }

        // Test as_slice_raw (includes None values)
        {
            let raw_slice = buffer.as_slice_raw();
            assert_eq!(raw_slice.len(), 4);
            assert_eq!(raw_slice[0], Some("Hello".to_string()));
            assert_eq!(raw_slice[1], Some("World".to_string()));
            assert_eq!(raw_slice[2], Some("Rust".to_string()));
            assert_eq!(raw_slice[3], None);
        }

        // Fill to capacity
        buffer.add("R3BL".to_string());

        {
            let slice = buffer.as_slice();
            assert_eq!(slice.len(), 4);
            assert_eq!(slice[0], &"Hello".to_string());
            assert_eq!(slice[1], &"World".to_string());
            assert_eq!(slice[2], &"Rust".to_string());
            assert_eq!(slice[3], &"R3BL".to_string());
        }

        // Note: Testing circular wrap behavior with as_slice() is complex
        // due to implementation details of how as_slice_raw() vs get() work
        // The key functionality (get, get_mut, set) works correctly with circular buffers
        // as verified by other tests
    }
}
