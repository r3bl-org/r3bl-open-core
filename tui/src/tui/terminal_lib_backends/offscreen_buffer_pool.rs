// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::OfsBuf;
use crate::{Flat2DArray, PixelChar, RingBuffer, RingBufferStack, VPSize};

const OFFSCREEN_BUFFER_POOL_SIZE: usize = 3;

/// Creating [`OfsBuf`]s is expensive, so we keep a pool of them to reuse. This
/// struct manages the pool. When a buffer is needed, it can be taken from the pool. When
/// a buffer is no longer needed, it can be given back to the pool. If you take a buffer
/// and don't give it back, it is lost from the pool (and will be dropped).
#[derive(Debug)]
pub struct OfsBufPool {
    pub pool: RingBufferStack<OfsBuf, OFFSCREEN_BUFFER_POOL_SIZE>,
    pub window_size: VPSize,
}

impl OfsBufPool {
    #[must_use]
    pub fn new(window_size: VPSize) -> Self {
        let mut pool = RingBufferStack::new();
        for _ in 0..OFFSCREEN_BUFFER_POOL_SIZE {
            pool.add(OfsBuf::new(Flat2DArray::new_empty(
                window_size,
                PixelChar::Spacer,
            )));
        }

        Self { pool, window_size }
    }

    /// Gets a buffer from the pool. If the pool is empty, a new buffer is created.
    pub fn take(&mut self) -> Option<OfsBuf> {
        if self.pool.is_empty() {
            Some(OfsBuf::new(Flat2DArray::new_empty(
                self.window_size,
                PixelChar::Spacer,
            )))
        } else {
            self.pool.pop()
        }
    }

    /// Add a buffer back to the pool. If the pool is full, the buffer is dropped. Only
    /// take the buffer back if it is still the correct size, otherwise drop it.
    pub fn give_back(&mut self, mut buffer: OfsBuf) {
        buffer.clear();
        if self.pool.is_full() {
            self.pool.pop();
        }
        if buffer.get_window_size() == self.window_size {
            self.pool.push(buffer);
        }
    }

    /// Resize the buffers in the pool. This will drop all buffers in the pool and create
    /// new ones with the new size.
    pub fn resize(&mut self, new_window_size: VPSize) {
        if self.window_size != new_window_size {
            self.window_size = new_window_size;
            self.rebuild_pool();
        }
    }

    fn rebuild_pool(&mut self) {
        self.pool.clear();
        for _ in 0..OFFSCREEN_BUFFER_POOL_SIZE {
            self.pool.push(OfsBuf::new(Flat2DArray::new_empty(
                self.window_size,
                PixelChar::Spacer,
            )));
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
    use crate::{vp_height, vp_width};

    #[test]
    fn test_ofs_buf_pool_new() {
        let window_size = vp_width(10) + vp_height(5);
        let pool = OfsBufPool::new(window_size);
        assert_eq!(pool.len(), OFFSCREEN_BUFFER_POOL_SIZE);
        assert_eq!(pool.window_size, window_size);
    }

    #[test]
    fn test_ofs_buf_pool_take_give_back() {
        let window_size = vp_width(10) + vp_height(5);
        let mut pool = OfsBufPool::new(window_size);
        assert_eq!(pool.len(), OFFSCREEN_BUFFER_POOL_SIZE);

        let buffer = pool.take().expect("conversion error");
        assert_eq!(pool.len(), OFFSCREEN_BUFFER_POOL_SIZE - 1);

        pool.give_back(buffer);
        assert_eq!(pool.len(), OFFSCREEN_BUFFER_POOL_SIZE);

        let _unused: OfsBuf = pool.take().expect("conversion error");
        assert_eq!(pool.len(), OFFSCREEN_BUFFER_POOL_SIZE - 1);
    }

    #[test]
    fn test_ofs_buf_pool_resize() {
        let window_size = vp_width(10) + vp_height(5);
        let mut pool = OfsBufPool::new(window_size);
        assert_eq!(pool.len(), OFFSCREEN_BUFFER_POOL_SIZE);
        assert_eq!(pool.window_size, window_size);
        let item = pool.take().expect("conversion error");
        assert_eq!(item.get_window_size(), window_size);

        let new_window_size = vp_width(20) + vp_height(10);
        pool.resize(new_window_size);
        assert_eq!(pool.window_size, new_window_size);
        assert_eq!(pool.len(), OFFSCREEN_BUFFER_POOL_SIZE);
        let item = pool.take().expect("conversion error");
        assert_eq!(item.get_window_size(), new_window_size);
    }

    #[test]
    fn test_ofs_buf_pool_is_empty() {
        let window_size = vp_width(10) + vp_height(5);
        let mut pool = OfsBufPool::new(window_size);
        assert_eq!(pool.len(), OFFSCREEN_BUFFER_POOL_SIZE);
        assert!(!pool.is_empty());
        for _ in 0..OFFSCREEN_BUFFER_POOL_SIZE {
            pool.take().expect("conversion error");
        }
        assert_eq!(pool.len(), 0);
        assert!(pool.is_empty());
    }

    #[test]
    fn test_ofs_buf_pool_give_back_when_full() {
        let window_size = vp_width(10) + vp_height(5);
        let mut pool = OfsBufPool::new(window_size);

        // Take all buffers from the pool.
        let mut taken_buffers = Vec::new();
        for _ in 0..OFFSCREEN_BUFFER_POOL_SIZE {
            taken_buffers.push(pool.take().expect("conversion error"));
        }
        assert_eq!(pool.len(), 0);
        assert!(pool.is_empty());

        // Give back one buffer to fill the pool.
        pool.give_back(taken_buffers.pop().expect("conversion error"));
        assert_eq!(pool.len(), 1);

        // Give back the rest of the buffers. The first one should be dropped.
        while let Some(buffer) = taken_buffers.pop() {
            pool.give_back(buffer);
        }
        assert_eq!(pool.len(), OFFSCREEN_BUFFER_POOL_SIZE);
    }

    #[test]
    fn test_ofs_buf_pool_take_returns_some_when_empty() {
        let window_size = vp_width(10) + vp_height(5);
        let mut pool = OfsBufPool::new(window_size);

        // Take all buffers from the pool.
        for _ in 0..OFFSCREEN_BUFFER_POOL_SIZE {
            pool.take().expect("conversion error");
        }

        // The pool is now empty.
        assert_eq!(pool.len(), 0);
        assert!(pool.is_empty());

        // Taking from an empty pool should return Some.
        assert!(pool.take().is_some());
    }
}
