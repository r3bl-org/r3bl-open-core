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
///
/// # Buffer Management Algorithm
///
/// This implementation uses a growable buffer with lazy compaction to avoid
/// copying bytes on every parse while preventing unbounded memory growth:
///
/// ```text
/// Initial state: buffer = [], consumed = 0
///
/// After read #1: buffer = [0x1B, 0x5B, 0x41], consumed = 0
///                         ├─────────────────┤
///                         Parser tries [0..3]
///                         Parses Up Arrow (3 bytes)
///                         consumed = 3
///
/// After read #2: buffer = [0x1B, 0x5B, 0x41, 0x61], consumed = 3
///                         └──── parsed ────┘ ├───┤
///                                            Parser tries [3..4]
///                                            Parses 'a' (1 byte)
///                                            consumed = 4
///
/// After read #3: buffer = [      ...many bytes...      ], consumed = 2100
///                         └ consumed > 2048 threshold! ┘
///                         Compact: drain [0..2100], consumed = 0
///                         buffer now starts fresh
/// ```
///
/// **Key operations:**
/// - [`unconsumed()`] - Get only unprocessed bytes for parsing
/// - [`consume(n)`] - Mark n bytes as processed
/// - When consumed > 2048 - Buffer compacts automatically
///
/// **Why not a true ring buffer?**
/// - Variable-length ANSI sequences (1-20+ bytes) make fixed-size wrapping complex
/// - Growing Vec handles overflow naturally without wrap-around logic
/// - Lazy compaction (every 2KB) amortizes cost: O(1) per event on average
///
/// **Memory behavior:**
/// - Typical: 100 events → ~500 bytes consumed, no compaction needed
/// - Worst case: `PARSE_BUFFER_SIZE` buffer + 2KB consumed = 6KB maximum before
///   compaction
/// - After compaction: resets to current unconsumed data only
///
/// [`unconsumed()`]: Self::unconsumed
/// [`consume(n)`]: Self::consume
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
