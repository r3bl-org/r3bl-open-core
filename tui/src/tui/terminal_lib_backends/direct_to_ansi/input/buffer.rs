// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Parse buffer for accumulating and consuming terminal input bytes.
//!
//! The [`ParseBuffer`] type encapsulates buffer management for ANSI sequence parsing,
//! providing a clean abstraction over the underlying byte storage and position tracking.

use crate::{ByteIndex, ByteOffset};
use smallvec::SmallVec;

/// Initial buffer capacity for efficient ANSI sequence buffering.
///
/// Most terminal input consists of either:
/// - Individual keypresses (~5-10 bytes for special keys like arrows, function keys)
/// - Paste events (variable, but rare to exceed buffer capacity)
/// - Mouse events (~20 bytes for typical terminal coordinates)
///
/// 4096 bytes accommodates multiple complete ANSI sequences without frequent
/// reallocations. This is a good balance: large enough to handle typical bursts,
/// small enough to avoid excessive memory overhead for idle periods.
pub const PARSE_BUFFER_SIZE: usize = 4096;

/// Temporary read buffer size for stdin reads.
///
/// This is the read granularity: how much data we pull from the kernel in one
/// syscall. Too small (< 256): Excessive syscalls increase latency. Too large
/// (> 256): Delays response to time-sensitive input (e.g., arrow key repeat).
///
/// 256 bytes is optimal for terminal input: it's one page boundary on many
/// architectures, fits comfortably in the input buffer, and provides good syscall
/// efficiency without introducing noticeable latency.
pub const STDIN_READ_BUFFER_SIZE: usize = 256;

/// Buffer for accumulating and consuming terminal input bytes.
///
/// This type encapsulates the byte storage and position tracking needed for parsing
/// ANSI sequences. It behaves like a ring buffer that compacts when the consumed
/// portion exceeds half capacity.
///
/// # Design
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │ data: [consumed bytes...][unconsumed bytes...]              │
/// │                          ^                                  │
/// │                          position                           │
/// └─────────────────────────────────────────────────────────────┘
/// ```
///
/// - Bytes before `position` have been parsed and can be discarded.
/// - Bytes from `position` onward are pending parsing.
/// - When `position` exceeds `PARSE_BUFFER_SIZE / 2`, the buffer compacts
///   by draining consumed bytes.
#[derive(Debug)]
pub struct ParseBuffer {
    /// Raw byte storage with inline capacity for typical terminal input.
    data: SmallVec<[u8; PARSE_BUFFER_SIZE]>,

    /// Position marking the boundary between consumed and unconsumed bytes.
    position: ByteIndex,
}

impl Default for ParseBuffer {
    fn default() -> Self { Self::new() }
}

impl ParseBuffer {
    /// Create an empty parse buffer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: SmallVec::new(),
            position: ByteIndex::default(),
        }
    }

    /// Get the slice of unconsumed bytes (from position to end).
    #[must_use]
    pub fn unconsumed(&self) -> &[u8] { &self.data[self.position.as_usize()..] }

    /// Get the current position (number of consumed bytes).
    #[cfg(test)]
    #[must_use]
    pub fn position(&self) -> ByteIndex { self.position }

    /// Total number of bytes in the buffer (consumed + unconsumed).
    #[cfg(test)]
    #[must_use]
    pub fn len(&self) -> usize { self.data.len() }

    /// Check if the buffer is empty.
    #[cfg(test)]
    #[must_use]
    pub fn is_empty(&self) -> bool { self.data.is_empty() }

    /// Append bytes to the end of the buffer.
    ///
    /// Use this after reading from stdin to add new data for parsing.
    pub fn append(&mut self, bytes: &[u8]) { self.data.extend_from_slice(bytes); }

    /// Consume N bytes from the buffer.
    ///
    /// Increments the position and compacts the buffer if threshold exceeded.
    /// This behaves like a ring buffer (except that it is not fixed size).
    ///
    /// # Semantic Correctness
    ///
    /// Takes [`ByteOffset`] (displacement from parser) and applies it to
    /// the position (location in buffer): `position += displacement`.
    pub fn consume(&mut self, displacement: ByteOffset) {
        self.position += displacement;

        // Compact buffer if consumed bytes exceed half of PARSE_BUFFER_SIZE.
        if self.position.as_usize() > PARSE_BUFFER_SIZE / 2 {
            self.data.drain(..self.position.as_usize());
            self.position = ByteIndex::default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer_is_empty() {
        let buffer = ParseBuffer::new();
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.position().as_usize(), 0);
        assert!(buffer.unconsumed().is_empty());
    }

    #[test]
    fn test_append_and_unconsumed() {
        let mut buffer = ParseBuffer::new();
        buffer.append(b"hello");
        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer.unconsumed(), b"hello");
    }

    #[test]
    fn test_consume_updates_position() {
        let mut buffer = ParseBuffer::new();
        buffer.append(b"hello world");
        buffer.consume(ByteOffset(6));
        assert_eq!(buffer.unconsumed(), b"world");
        assert_eq!(buffer.position().as_usize(), 6);
    }

    #[test]
    fn test_compaction_on_threshold() {
        let mut buffer = ParseBuffer::new();
        // Fill with enough data to trigger compaction.
        let data = vec![b'x'; PARSE_BUFFER_SIZE];
        buffer.append(&data);

        // Consume more than half to trigger compaction.
        buffer.consume(ByteOffset((PARSE_BUFFER_SIZE / 2) + 100));

        // After compaction, position resets to 0.
        assert_eq!(buffer.position().as_usize(), 0);
        // Remaining unconsumed bytes.
        assert_eq!(buffer.unconsumed().len(), PARSE_BUFFER_SIZE / 2 - 100);
    }
}
