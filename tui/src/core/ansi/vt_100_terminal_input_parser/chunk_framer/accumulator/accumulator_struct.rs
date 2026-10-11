// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Dedicated buffer accumulator for terminal input bytes. See [`ChunkAccumulator`].

use super::{constants::ACCUMULATOR_INITIAL_CAPACITY,
            unparsed_buffer_action::UnparsedBufferAction};
use crate::{ByteLength, ByteOffset, DEBUG_TUI_SHOW_DIRECT_TO_ANSI};

/// Dedicated buffer accumulator for terminal input bytes.
///
/// Encapsulates raw byte buffering, unparsed sequence recovery action evaluation via
/// [`Self::determine_unparsed_action()`], safe draining via [`ByteOffset`], and
/// accumulator poisoning prevention.
///
/// For the overarching parsing (framing & decoding) pipeline and architecture, see
/// [Parser Architecture and Mental Model][parser_arch_mental_model] in the parent module.
///
/// [`ByteOffset`]: crate::ByteOffset
/// [parser_arch_mental_model]: mod@crate::core::ansi::vt_100_terminal_input_parser#parser-architecture-and-mental-model
#[derive(Debug)]
pub struct ChunkAccumulator {
    buffer: Vec<u8>,
}

impl Default for ChunkAccumulator {
    fn default() -> Self {
        Self {
            buffer: Vec::with_capacity(ACCUMULATOR_INITIAL_CAPACITY),
        }
    }
}

impl ChunkAccumulator {
    /// Append incoming raw bytes to the accumulator buffer.
    pub fn append(&mut self, bytes: &[u8]) { self.buffer.extend_from_slice(bytes); }

    /// Returns a borrowed slice of the accumulated bytes.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] { &self.buffer }

    /// Returns `true` if the accumulator contains no bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.buffer.is_empty() }

    /// Returns the number of accumulated bytes as a type-safe [`ByteLength`].
    ///
    /// [`ByteLength`]: crate::ByteLength
    #[must_use]
    pub fn len(&self) -> ByteLength { ByteLength::from(self.buffer.len()) }

    /// Safely drain the specified number of parsed bytes from the start of the
    /// accumulator.
    pub fn consume(&mut self, bytes_consumed: ByteOffset) {
        self.buffer.drain(..bytes_consumed.as_usize());
    }

    /// Evaluates the unparsed bytes currently in the accumulator to determine the
    /// recovery action the framer must take when the sequence decoder returns `None`.
    #[must_use]
    pub fn determine_unparsed_action(&self) -> UnparsedBufferAction {
        UnparsedBufferAction::determine_action(self.buffer.as_slice())
    }

    /// Discard current buffer contents to prevent accumulator poisoning from
    /// malformed or unrecognized escape sequences, logging diagnostic warnings when
    /// enabled.
    pub fn purge_malformed(&mut self) {
        DEBUG_TUI_SHOW_DIRECT_TO_ANSI.then(|| {
            tracing::warn! {
                message = "ChunkAccumulator::purge_malformed",
                status = "discarding unrecognized/malformed escape sequence",
                discarded_hex = %format!("{:02X?}", self.buffer),
                discarded_str = %String::from_utf8_lossy(&self.buffer),
                buffer_len = self.buffer.len(),
            };
        });
        self.buffer.clear();
    }

    /// Clear the accumulator and return the count of purged bytes as a [`ByteOffset`]
    /// to initialize an [`OscCircuitBreaker`] in open state.
    ///
    /// [`OscCircuitBreaker`]: crate::core::ansi::vt_100_terminal_input_parser::chunk_framer::OscCircuitBreaker
    pub fn drain_for_trip(&mut self) -> ByteOffset {
        let acc_len: ByteOffset = self.buffer.len().into();
        self.buffer.clear();
        acc_len
    }

    /// Clear the accumulator buffer completely.
    pub fn clear(&mut self) { self.buffer.clear(); }
}
