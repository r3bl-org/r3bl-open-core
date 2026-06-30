// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{GetMemSize, Length, PixelCharLine};
use std::{collections::VecDeque, mem::size_of};

/// State of the terminal's scrollback buffer.
///
/// This struct holds the history of lines that have scrolled off the top of the terminal
/// screen. It manages its capacity using:
/// 1. a [`VecDeque`] and
/// 2. a [`ScrollbackBufferLimit`] policy (e.g. [`Fixed`] or [`Unlimited`]).
///
/// It supports caching its memory footprint, making [`GetMemSize`] highly efficient.
///
/// [`Fixed`]: ScrollbackBufferLimit::Fixed
/// [`Unlimited`]: ScrollbackBufferLimit::Unlimited
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollbackBuffer {
    pub lines: VecDeque<PixelCharLine>,
    pub limit: ScrollbackBufferLimit,
    pub cached_mem_size: usize,
}

mod impl_scrollback_state {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Converts a [`ScrollbackBufferLimit`] into a new, empty [`ScrollbackBuffer`].
    impl From<ScrollbackBufferLimit> for ScrollbackBuffer {
        fn from(limit: ScrollbackBufferLimit) -> Self {
            Self {
                lines: VecDeque::new(),
                limit,
                cached_mem_size: 0,
            }
        }
    }

    impl ScrollbackBuffer {
        /// Pushes a new line to the scrollback buffer and enforces the capacity limit. It
        /// also updates the memory size cache to reflect the added and evicted lines.
        pub fn push_and_enforce_limit(&mut self, line: PixelCharLine) {
            // Add the new line's memory footprint to the cache.
            self.cached_mem_size += line.get_mem_size();

            // Add the evicted line to the scrollback buffer.
            self.lines.push_back(line);

            // Bail out early if capacity is unlimited.
            let ScrollbackBufferLimit::Fixed(max_scrollback_buffer_limit) = self.limit else {
                return;
            };

            // We know we have a fixed capacity, enforce it by removing the oldest line.
            let overflows_scrollback_buffer_limit =
                self.lines.len() > max_scrollback_buffer_limit.as_usize();
            if overflows_scrollback_buffer_limit
                && let Some(evicted) = self.lines.pop_front() {
                    self.cached_mem_size -= evicted.get_mem_size();
                }
        }

        /// Clears all lines from the scrollback buffer and resets the memory size cache.
        pub fn clear(&mut self) {
            self.lines.clear();
            self.cached_mem_size = 0;
        }
    }

    impl GetMemSize for ScrollbackBuffer {
        fn get_mem_size(&self) -> usize {
            let struct_overhead = size_of::<ScrollbackBuffer>();
            let lines_vec_overhead = self.lines.capacity() * size_of::<PixelCharLine>();
            let lines_content_size = self.cached_mem_size;

            struct_overhead + lines_vec_overhead + lines_content_size
        }
    }
}

/// Configuration for the maximum number of lines retained in the scrollback buffer.
///
/// When the terminal scrolls and lines are pushed to the history, this capacity dictates
/// whether old lines are evicted to cap memory usage, or if it grows indefinitely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollbackBufferLimit {
    /// No limit. Lines are never evicted.
    Unlimited,

    /// A fixed capacity limit on the number of lines in the scrollback buffer.
    ///
    /// When new lines are added, if the buffer length exceeds this limit, the oldest line
    /// is popped from the front of the queue and evicted to enforce the maximum capacity.
    Fixed(Length),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::len;

    #[test]
    fn test_scrollback_buffer_limit_fixed() {
        let mut scrollback_buffer: ScrollbackBuffer = ScrollbackBufferLimit::Fixed(len(2)).into();

        // Push first item
        scrollback_buffer.push_and_enforce_limit(PixelCharLine::new_empty(0));
        assert_eq!(scrollback_buffer.lines.len(), 1);

        // Push second item
        scrollback_buffer.push_and_enforce_limit(PixelCharLine::new_empty(0));
        assert_eq!(scrollback_buffer.lines.len(), 2);

        // Push third item, should evict the first
        scrollback_buffer.push_and_enforce_limit(PixelCharLine::new_empty(0));
        assert_eq!(scrollback_buffer.lines.len(), 2);
    }

    #[test]
    fn test_scrollback_buffer_limit_unlimited() {
        let mut scrollback_buffer: ScrollbackBuffer = ScrollbackBufferLimit::Unlimited.into();

        // Push items well beyond a typical small limit
        for _ in 0..10 {
            scrollback_buffer.push_and_enforce_limit(PixelCharLine::new_empty(0));
        }

        assert_eq!(scrollback_buffer.lines.len(), 10);
    }
}
