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

use super::OffscreenBuffer;
use crate::{RingBuffer, RingBufferStack, Size};

const OFFSCREEN_BUFFER_POOL_SIZE: usize = 3;

/// Creating [`OffscreenBuffer`]s is expensive, so we keep a pool of them to reuse. This
/// struct manages the pool. When a buffer is needed, it can be taken from the pool. When
/// a buffer is no longer needed, it can be given back to the pool. If you take a buffer
/// and don't give it back, it is lost from the pool (and will be dropped).
#[derive(Debug)]
pub struct OffscreenBufferPool {
    pub pool: RingBufferStack<OffscreenBuffer, OFFSCREEN_BUFFER_POOL_SIZE>,
    pub window_size: Size,
}

impl OffscreenBufferPool {
    #[must_use]
    pub fn new(window_size: Size) -> Self {
        let mut pool = RingBufferStack::new();
        for _ in 0..OFFSCREEN_BUFFER_POOL_SIZE {
            pool.add(OffscreenBuffer::new_with_capacity_initialized(window_size));
        }

        Self { pool, window_size }
    }

    /// Get a buffer from the pool. If the pool is empty, a new buffer is created.
    pub fn take(&mut self) -> Option<OffscreenBuffer> {
        if self.pool.is_empty() {
            Some(OffscreenBuffer::new_with_capacity_initialized(
                self.window_size,
            ))
        } else {
            self.pool.pop()
        }
    }

    /// Add a buffer back to the pool. If the pool is full, the buffer is dropped. Only
    /// take the buffer back if it is still the correct size, otherwise drop it.
    pub fn give_back(&mut self, mut buffer: OffscreenBuffer) {
        buffer.clear();
        if self.pool.is_full() {
            self.pool.pop();
        }
        if buffer.window_size == self.window_size {
            self.pool.push(buffer);
        }
    }

    /// Resize the buffers in the pool. This will drop all buffers in the pool and create
    /// new ones with the new size.
    pub fn resize(&mut self, new_window_size: Size) {
        if self.window_size != new_window_size {
            self.window_size = new_window_size;
            self.rebuild_pool();
        }
    }

    fn rebuild_pool(&mut self) {
        self.pool.clear();
        for _ in 0..OFFSCREEN_BUFFER_POOL_SIZE {
            self.pool
                .push(OffscreenBuffer::new_with_capacity_initialized(
                    self.window_size,
                ));
        }
    }

    /// Returns the number of buffers currently in the pool.
    #[must_use]
    pub fn len(&self) -> usize { self.pool.len().as_usize() }

    #[must_use]
    pub fn is_empty(&self) -> bool { self.pool.is_empty() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{height, width};

    #[test]
    fn test_offscreen_buffer_pool_new() {
        let window_size = width(10) + height(5);
        let pool = OffscreenBufferPool::new(window_size);
        assert_eq!(pool.len(), OFFSCREEN_BUFFER_POOL_SIZE);
        assert_eq!(pool.window_size, window_size);
    }

    #[test]
    fn test_offscreen_buffer_pool_take_give_back() {
        let window_size = width(10) + height(5);
        let mut pool = OffscreenBufferPool::new(window_size);
        assert_eq!(pool.len(), OFFSCREEN_BUFFER_POOL_SIZE);

        let buffer = pool.take().unwrap();
        assert_eq!(pool.len(), OFFSCREEN_BUFFER_POOL_SIZE - 1);

        pool.give_back(buffer);
        assert_eq!(pool.len(), OFFSCREEN_BUFFER_POOL_SIZE);

        let _unused: OffscreenBuffer = pool.take().unwrap();
        assert_eq!(pool.len(), OFFSCREEN_BUFFER_POOL_SIZE - 1);
    }

    #[test]
    fn test_offscreen_buffer_pool_resize() {
        let window_size = width(10) + height(5);
        let mut pool = OffscreenBufferPool::new(window_size);
        assert_eq!(pool.len(), OFFSCREEN_BUFFER_POOL_SIZE);
        assert_eq!(pool.window_size, window_size);
        let item = pool.take().unwrap();
        assert_eq!(item.window_size, window_size);

        let new_window_size = width(20) + height(10);
        pool.resize(new_window_size);
        assert_eq!(pool.window_size, new_window_size);
        assert_eq!(pool.len(), OFFSCREEN_BUFFER_POOL_SIZE);
        let item = pool.take().unwrap();
        assert_eq!(item.window_size, new_window_size);
    }

    #[test]
    fn test_offscreen_buffer_pool_is_empty() {
        let window_size = width(10) + height(5);
        let mut pool = OffscreenBufferPool::new(window_size);
        assert_eq!(pool.len(), OFFSCREEN_BUFFER_POOL_SIZE);
        assert!(!pool.is_empty());
        for _ in 0..OFFSCREEN_BUFFER_POOL_SIZE {
            pool.take().unwrap();
        }
        assert_eq!(pool.len(), 0);
        assert!(pool.is_empty());
    }

    #[test]
    fn test_offscreen_buffer_pool_give_back_when_full() {
        let window_size = width(10) + height(5);
        let mut pool = OffscreenBufferPool::new(window_size);

        // Take all buffers from the pool.
        let mut taken_buffers = Vec::new();
        for _ in 0..OFFSCREEN_BUFFER_POOL_SIZE {
            taken_buffers.push(pool.take().unwrap());
        }
        assert_eq!(pool.len(), 0);
        assert!(pool.is_empty());

        // Give back one buffer to fill the pool.
        pool.give_back(taken_buffers.pop().unwrap());
        assert_eq!(pool.len(), 1);

        // Give back the rest of the buffers. The first one should be dropped.
        while let Some(buffer) = taken_buffers.pop() {
            pool.give_back(buffer);
        }
        assert_eq!(pool.len(), OFFSCREEN_BUFFER_POOL_SIZE);
    }

    #[test]
    fn test_offscreen_buffer_pool_take_returns_some_when_empty() {
        let window_size = width(10) + height(5);
        let mut pool = OffscreenBufferPool::new(window_size);

        // Take all buffers from the pool.
        for _ in 0..OFFSCREEN_BUFFER_POOL_SIZE {
            pool.take().unwrap();
        }

        // The pool is now empty.
        assert_eq!(pool.len(), 0);
        assert!(pool.is_empty());

        // Taking from an empty pool should return Some.
        assert!(pool.take().is_some());
    }
}
