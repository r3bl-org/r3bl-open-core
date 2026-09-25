// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Streaming circuit breaker state machine for runaway [`OSC`] sequences. See
//! [`OscCircuitBreaker`] for more details.
//!
//! [`OSC`]: crate::osc_codes::OscSequence

use crate::{ANSI_BEL, ANSI_ESC, ANSI_ST_7BIT_LEN, ANSI_ST_FINAL, ByteOffset,
            CARRIAGE_RETURN, DEBUG_TUI_SHOW_DIRECT_TO_ANSI, LINE_FEED,
            MAX_OSC_DRAIN_BYTES, byte_offset};
use std::fmt::Display;
use tracing::Level;

/// Circuit breaker for discarding runaway or oversized escape sequences.
///
/// Prevents framing desynchronization, text leakage, and spurious keystroke generation
/// when an [`OSC`] sequence exceeds [`MAX_OSC_SEQUENCE_LENGTH`] (1 MiB).
///
/// [`MAX_OSC_SEQUENCE_LENGTH`]: crate::MAX_OSC_SEQUENCE_LENGTH
/// [`OSC`]: crate::osc_codes::OscSequence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OscCircuitBreaker {
    /// Normal parsing state (circuit closed). Bytes are accumulated into
    /// [`InputByteStreamToIrParser`]'s internal buffer.
    ///
    /// [`InputByteStreamToIrParser`]: super::InputByteStreamToIrParser
    #[default]
    Closed,

    /// Circuit breaker tripped (open). Discarding bytes of an in-flight runaway [`OSC`]
    /// sequence until a terminator ([`ANSI_BEL`] `0x07` or 7-bit [`ANSI_ST_7BIT`]
    /// `\x1b\\`) or abort condition is encountered.
    ///
    /// [`ANSI_BEL`]: crate::ANSI_BEL
    /// [`ANSI_ST_7BIT`]: crate::ANSI_ST_7BIT
    /// [`OSC`]: crate::osc_codes::OscSequence
    Open {
        /// If the previous chunk ended in a lone [`ANSI_ESC`] (`0x1B`), this tracks
        /// whether the next byte completes a 7-bit [`ANSI_ST_7BIT`] (`\x1b\\`).
        ///
        /// [`ANSI_ESC`]: crate::ANSI_ESC
        /// [`ANSI_ST_7BIT`]: crate::ANSI_ST_7BIT
        saw_partial_esc: bool,

        /// Total bytes drained so far across chunks (bounded by
        /// [`MAX_OSC_DRAIN_BYTES`]).
        ///
        /// [`MAX_OSC_DRAIN_BYTES`]: crate::MAX_OSC_DRAIN_BYTES
        drained_bytes: usize,
    },
}

impl OscCircuitBreaker {
    /// Trips the circuit breaker to [`Self::Open`] with the specified initial count of
    /// drained bytes (typically the length of the purged accumulator).
    pub fn trip(&mut self, initial_bytes: usize) {
        *self = Self::Open {
            saw_partial_esc: false,
            drained_bytes: initial_bytes,
        };
    }

    /// Resets the circuit breaker to [`Self::Closed`], logs the transition at the
    /// specified [`Level`], and returns the resulting [`OscDrainResult`].
    fn reset_and_log(
        &mut self,
        level: Level,
        reason: OscDrainReason,
        bytes_consumed: ByteOffset,
        chunk_len: usize,
        total_drained_bytes: usize,
    ) -> OscDrainResult {
        DEBUG_TUI_SHOW_DIRECT_TO_ANSI.then(|| {
            // % is Display, ? is Debug.
            match level {
                Level::WARN => {
                    tracing::warn! {
                        message = "OscCircuitBreaker::drain_chunk",
                        status = %reason,
                        total_drained_bytes,
                    };
                }
                _ => {
                    tracing::info! {
                        message = "OscCircuitBreaker::drain_chunk",
                        status = %reason,
                        total_drained_bytes,
                    };
                }
            }
        });
        *self = Self::Closed;
        OscDrainResult::from_consumption(reason, bytes_consumed, chunk_len)
    }

    /// Drains bytes from an incoming chunk while in [`Self::Open`].
    ///
    /// Scans for:
    /// 1. `BEL` (`\x07`) -> cleanly terminates [`OSC`].
    /// 2. `ST` (`\x1b\\`) -> cleanly terminates [`OSC`] (including across chunk boundary
    ///    if previous chunk ended in lone [`ANSI_ESC`]).
    /// 3. Abort conditions:
    ///    - Raw `\r` or `\n` (ECMA-48 / [`OSC`] payloads never contain raw CR/LF).
    ///    - [`ANSI_ESC`] followed by any byte other than `\\` (starts a new escape
    ///      sequence, aborting [`OSC`]).
    /// 4. Safety upper bound: cumulative drained bytes reaching [`MAX_OSC_DRAIN_BYTES`].
    ///
    /// Returns an [`OscDrainResult`] classifying whether the chunk was fully or
    /// partially consumed, along with the consumed [`ByteOffset`] and domain reason.
    ///
    /// [`ANSI_ESC`]: crate::ANSI_ESC
    /// [`MAX_OSC_DRAIN_BYTES`]: crate::MAX_OSC_DRAIN_BYTES
    /// [`OSC`]: crate::osc_codes::OscSequence
    pub fn drain_chunk(&mut self, chunk: &[u8]) -> OscDrainResult {
        // Early return if not in Open state.
        let Self::Open {
            saw_partial_esc,
            mut drained_bytes,
        } = *self
        else {
            return OscDrainResult::from_consumption(
                OscDrainReason::RunawayPayloadOngoing,
                byte_offset(0),
                chunk.len(),
            );
        };

        // Resolve partial ESC from previous chunk.
        if saw_partial_esc {
            return self.resolve_partial_esc(chunk, drained_bytes);
        }

        let mut byte_index_in_chunk = 0;

        while byte_index_in_chunk < chunk.len() {
            // Check safety ceiling.
            if drained_bytes >= MAX_OSC_DRAIN_BYTES {
                return self.reset_and_log(
                    Level::WARN,
                    OscDrainReason::ExceededSafetyCeiling,
                    byte_offset(byte_index_in_chunk),
                    chunk.len(),
                    drained_bytes,
                );
            }

            match chunk[byte_index_in_chunk..] {
                // Terminated by BEL (0x07).
                [ANSI_BEL, ..] => {
                    return self.reset_and_log(
                        Level::INFO,
                        OscDrainReason::TerminatedByBel,
                        byte_offset(byte_index_in_chunk + 1),
                        chunk.len(),
                        drained_bytes + 1,
                    );
                }

                // Terminated by 7-bit ST (\x1b\).
                [ANSI_ESC, ANSI_ST_FINAL, ..] => {
                    return self.reset_and_log(
                        Level::INFO,
                        OscDrainReason::TerminatedBySt,
                        byte_offset(byte_index_in_chunk + ANSI_ST_7BIT_LEN),
                        chunk.len(),
                        drained_bytes + ANSI_ST_7BIT_LEN,
                    );
                }

                // ESC followed by non-backslash: aborts OSC string! Leave ESC for normal
                // parsing.
                [ANSI_ESC, _, ..] => {
                    return self.reset_and_log(
                        Level::INFO,
                        OscDrainReason::AbortedByNewEsc,
                        byte_offset(byte_index_in_chunk),
                        chunk.len(),
                        drained_bytes,
                    );
                }

                // Lone ESC at end of chunk: remember across chunk boundaries.
                [ANSI_ESC] => {
                    drained_bytes += 1;
                    *self = Self::Open {
                        saw_partial_esc: true,
                        drained_bytes,
                    };
                    return OscDrainResult::from_consumption(
                        OscDrainReason::LoneEscAtBoundary,
                        byte_offset(chunk.len()),
                        chunk.len(),
                    );
                }

                // Raw newline/CR aborts OSC syntax. Leave newline for normal parsing.
                [CARRIAGE_RETURN | LINE_FEED, ..] => {
                    return self.reset_and_log(
                        Level::INFO,
                        OscDrainReason::AbortedByNewline,
                        byte_offset(byte_index_in_chunk),
                        chunk.len(),
                        drained_bytes,
                    );
                }

                // Regular payload byte: consume and continue draining.
                _ => {
                    drained_bytes += 1;
                    byte_index_in_chunk += 1;
                }
            }
        }

        // Entire chunk consumed without encountering terminator or abort.
        *self = Self::Open {
            saw_partial_esc: false,
            drained_bytes,
        };
        OscDrainResult::from_consumption(
            OscDrainReason::RunawayPayloadOngoing,
            byte_offset(chunk.len()),
            chunk.len(),
        )
    }

    /// Resolves a lone [`ANSI_ESC`] that occurred at the end of the previous chunk.
    ///
    /// [`ANSI_ESC`]: crate::ANSI_ESC
    fn resolve_partial_esc(
        &mut self,
        chunk: &[u8],
        drained_bytes: usize,
    ) -> OscDrainResult {
        match chunk.first() {
            None => OscDrainResult::from_consumption(
                OscDrainReason::LoneEscAtBoundary,
                byte_offset(0),
                0,
            ),
            Some(&ANSI_ST_FINAL) => self.reset_and_log(
                Level::INFO,
                OscDrainReason::TerminatedAcrossBoundary,
                byte_offset(1),
                chunk.len(),
                drained_bytes + 1,
            ),
            Some(_) => self.reset_and_log(
                Level::INFO,
                OscDrainReason::AbortedByNewEsc,
                byte_offset(0),
                chunk.len(),
                drained_bytes,
            ),
        }
    }
}

/// Result of an [`OSC`] drain operation, categorizing whether the chunk was
/// fully or partially consumed.
///
/// [`OSC`]: crate::osc_codes::OscSequence
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OscDrainResult {
    /// The entire chunk was consumed by the circuit breaker. There are zero remaining
    /// bytes for normal input parsing.
    Full {
        bytes_consumed: ByteOffset,
        reason: OscDrainReason,
    },

    /// The chunk was partially consumed. Remaining bytes starting at `bytes_consumed`
    /// must be processed as normal input.
    Partial {
        bytes_consumed: ByteOffset,
        reason: OscDrainReason,
    },
}

impl OscDrainResult {
    /// Creates an [`OscDrainResult`] classifying consumption as [`OscDrainResult::Full`]
    /// if `bytes_consumed == chunk_len`, or [`OscDrainResult::Partial`] otherwise.
    #[must_use]
    pub fn from_consumption(
        reason: OscDrainReason,
        bytes_consumed: ByteOffset,
        chunk_len: usize,
    ) -> Self {
        if *bytes_consumed == chunk_len {
            Self::Full {
                bytes_consumed,
                reason,
            }
        } else {
            Self::Partial {
                bytes_consumed,
                reason,
            }
        }
    }

    /// Returns the number of bytes consumed from the chunk.
    #[must_use]
    pub fn bytes_consumed(&self) -> ByteOffset {
        match *self {
            Self::Full { bytes_consumed, .. } | Self::Partial { bytes_consumed, .. } => {
                bytes_consumed
            }
        }
    }

    /// Returns the domain reason explaining the drain outcome.
    #[must_use]
    pub fn reason(&self) -> OscDrainReason {
        match *self {
            Self::Full { reason, .. } | Self::Partial { reason, .. } => reason,
        }
    }
}

/// Reason specifying the exact condition, termination, or transition that occurred
/// during an [`OSC`] drain operation in [`OscCircuitBreaker`].
///
/// Each variant represents a distinct parsing outcome (clean termination by `BEL`/`ST`,
/// syntax abort by raw newline or new escape sequence, safety ceiling overflow, or
/// ongoing runaway payload consumption).
///
/// [`OSC`]: crate::osc_codes::OscSequence
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OscDrainReason {
    /// Terminated cleanly by [`ANSI_BEL`] (`0x07`).
    ///
    /// [`ANSI_BEL`]: crate::ANSI_BEL
    TerminatedByBel,

    /// Terminated cleanly by 7-bit [`ANSI_ST_7BIT`] (`\x1b\\`).
    ///
    /// [`ANSI_ST_7BIT`]: crate::ANSI_ST_7BIT
    TerminatedBySt,

    /// Terminated cleanly by final byte of 7-bit [`ANSI_ST_7BIT`] (`\\`) across a chunk
    /// boundary.
    ///
    /// [`ANSI_ST_7BIT`]: crate::ANSI_ST_7BIT
    TerminatedAcrossBoundary,

    /// Aborted by an [`ANSI_ESC`] followed by a non-backslash character (starting a
    /// new escape sequence). Draining stopped before the [`ANSI_ESC`].
    ///
    /// [`ANSI_ESC`]: crate::ANSI_ESC
    AbortedByNewEsc,

    /// Aborted by a raw newline ([`CARRIAGE_RETURN`] or [`LINE_FEED`]). Draining
    /// stopped before the newline character.
    ///
    /// [`CARRIAGE_RETURN`]: crate::CARRIAGE_RETURN
    /// [`LINE_FEED`]: crate::LINE_FEED
    AbortedByNewline,

    /// Aborted because total drained bytes reached [`MAX_OSC_DRAIN_BYTES`] safety
    /// ceiling.
    ///
    /// [`MAX_OSC_DRAIN_BYTES`]: crate::MAX_OSC_DRAIN_BYTES
    ExceededSafetyCeiling,

    /// The entire chunk was consumed as runaway [`OSC`] payload. The circuit breaker
    /// remains in [`OscCircuitBreaker::Open`].
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    RunawayPayloadOngoing,

    /// The chunk ended with a lone [`ANSI_ESC`] (`0x1B`). The byte was consumed, and
    /// the circuit breaker remains in [`OscCircuitBreaker::Open`] waiting for the
    /// next chunk.
    ///
    /// [`ANSI_ESC`]: crate::ANSI_ESC
    LoneEscAtBoundary,
}

impl Display for OscDrainReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::TerminatedByBel => "runaway OSC terminated by BEL",
            Self::TerminatedBySt => "runaway OSC terminated by ST",
            Self::TerminatedAcrossBoundary => {
                "runaway OSC terminated by ST across boundary"
            }
            Self::AbortedByNewEsc => "runaway OSC aborted by new ESC sequence",
            Self::AbortedByNewline => "runaway OSC aborted by raw newline",
            Self::ExceededSafetyCeiling => {
                "runaway OSC exceeded MAX_OSC_DRAIN_BYTES safety ceiling"
            }
            Self::RunawayPayloadOngoing => "runaway OSC chunk fully consumed",
            Self::LoneEscAtBoundary => "runaway OSC chunk ended in lone ESC",
        };
        f.write_str(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc_drain_reason_display() {
        assert_eq!(
            format!("{}", OscDrainReason::TerminatedByBel),
            "runaway OSC terminated by BEL"
        );
        assert_eq!(
            format!("{}", OscDrainReason::TerminatedBySt),
            "runaway OSC terminated by ST"
        );
        assert_eq!(
            format!("{}", OscDrainReason::TerminatedAcrossBoundary),
            "runaway OSC terminated by ST across boundary"
        );
        assert_eq!(
            format!("{}", OscDrainReason::AbortedByNewEsc),
            "runaway OSC aborted by new ESC sequence"
        );
        assert_eq!(
            format!("{}", OscDrainReason::AbortedByNewline),
            "runaway OSC aborted by raw newline"
        );
        assert_eq!(
            format!("{}", OscDrainReason::ExceededSafetyCeiling),
            "runaway OSC exceeded MAX_OSC_DRAIN_BYTES safety ceiling"
        );
        assert_eq!(
            format!("{}", OscDrainReason::RunawayPayloadOngoing),
            "runaway OSC chunk fully consumed"
        );
        assert_eq!(
            format!("{}", OscDrainReason::LoneEscAtBoundary),
            "runaway OSC chunk ended in lone ESC"
        );
    }

    #[test]
    fn test_osc_drain_result_from_consumption() {
        let full = OscDrainResult::from_consumption(
            OscDrainReason::RunawayPayloadOngoing,
            byte_offset(5),
            5,
        );
        assert_eq!(
            full,
            OscDrainResult::Full {
                bytes_consumed: byte_offset(5),
                reason: OscDrainReason::RunawayPayloadOngoing,
            }
        );
        assert_eq!(full.bytes_consumed(), byte_offset(5));
        assert_eq!(full.reason(), OscDrainReason::RunawayPayloadOngoing);

        let partial = OscDrainResult::from_consumption(
            OscDrainReason::TerminatedByBel,
            byte_offset(3),
            5,
        );
        assert_eq!(
            partial,
            OscDrainResult::Partial {
                bytes_consumed: byte_offset(3),
                reason: OscDrainReason::TerminatedByBel,
            }
        );
        assert_eq!(partial.bytes_consumed(), byte_offset(3));
        assert_eq!(partial.reason(), OscDrainReason::TerminatedByBel);
    }

    #[test]
    fn test_drain_chunk_terminated_by_bel() {
        let mut breaker = OscCircuitBreaker::default();
        breaker.trip(100);
        assert!(matches!(breaker, OscCircuitBreaker::Open { .. }));
        assert_ne!(breaker, OscCircuitBreaker::Closed);

        let chunk = b"payload\x07trailing";
        let result = breaker.drain_chunk(chunk);
        assert_eq!(
            result,
            OscDrainResult::Partial {
                bytes_consumed: byte_offset(8),
                reason: OscDrainReason::TerminatedByBel,
            }
        );
        assert_eq!(breaker, OscCircuitBreaker::Closed);
    }

    #[test]
    fn test_drain_chunk_terminated_by_st() {
        let mut breaker = OscCircuitBreaker::default();
        breaker.trip(100);

        let chunk = b"payload\x1b\\trailing";
        let result = breaker.drain_chunk(chunk);
        assert_eq!(
            result,
            OscDrainResult::Partial {
                bytes_consumed: byte_offset(9),
                reason: OscDrainReason::TerminatedBySt,
            }
        );
        assert_eq!(breaker, OscCircuitBreaker::Closed);
    }

    #[test]
    fn test_drain_chunk_partial_esc_across_boundaries() {
        let mut breaker = OscCircuitBreaker::default();
        breaker.trip(100);

        // Chunk 1 ends in lone ESC.
        let chunk1 = b"payload\x1b";
        let result1 = breaker.drain_chunk(chunk1);
        assert_eq!(
            result1,
            OscDrainResult::Full {
                bytes_consumed: byte_offset(8),
                reason: OscDrainReason::LoneEscAtBoundary,
            }
        );
        assert_eq!(
            breaker,
            OscCircuitBreaker::Open {
                saw_partial_esc: true,
                drained_bytes: 108,
            }
        );

        // Chunk 2 starts with '\', completing ST.
        let chunk2 = b"\\trailing";
        let result2 = breaker.drain_chunk(chunk2);
        assert_eq!(
            result2,
            OscDrainResult::Partial {
                bytes_consumed: byte_offset(1),
                reason: OscDrainReason::TerminatedAcrossBoundary,
            }
        );
        assert_eq!(breaker, OscCircuitBreaker::Closed);
    }

    #[test]
    fn test_drain_chunk_partial_esc_aborted_by_other_char() {
        let mut breaker = OscCircuitBreaker::default();
        breaker.trip(100);

        // Chunk 1 ends in lone ESC.
        let chunk1 = b"payload\x1b";
        let result1 = breaker.drain_chunk(chunk1);
        assert_eq!(
            result1,
            OscDrainResult::Full {
                bytes_consumed: byte_offset(8),
                reason: OscDrainReason::LoneEscAtBoundary,
            }
        );

        // Chunk 2 starts with '[', aborting OSC.
        let chunk2 = b"[A";
        let result2 = breaker.drain_chunk(chunk2);
        assert_eq!(
            result2,
            OscDrainResult::Partial {
                bytes_consumed: byte_offset(0),
                reason: OscDrainReason::AbortedByNewEsc,
            }
        );
        assert_eq!(breaker, OscCircuitBreaker::Closed);
    }

    #[test]
    fn test_drain_chunk_aborted_by_newline() {
        let mut breaker = OscCircuitBreaker::default();
        breaker.trip(100);

        let chunk = b"payload\nrest";
        let result = breaker.drain_chunk(chunk);
        assert_eq!(
            result,
            OscDrainResult::Partial {
                bytes_consumed: byte_offset(7),
                reason: OscDrainReason::AbortedByNewline,
            }
        );
        assert_eq!(breaker, OscCircuitBreaker::Closed);
    }

    #[test]
    fn test_drain_chunk_safety_ceiling() {
        let mut breaker = OscCircuitBreaker::default();
        breaker.trip(MAX_OSC_DRAIN_BYTES);

        let chunk = b"more_data";
        let result = breaker.drain_chunk(chunk);
        assert_eq!(
            result,
            OscDrainResult::Partial {
                bytes_consumed: byte_offset(0),
                reason: OscDrainReason::ExceededSafetyCeiling,
            }
        );
        assert_eq!(breaker, OscCircuitBreaker::Closed);
    }

    #[test]
    fn test_drain_chunk_runaway_payload_full_chunk() {
        let mut breaker = OscCircuitBreaker::default();
        breaker.trip(100);

        let chunk = b"pure_payload_data";
        let result = breaker.drain_chunk(chunk);
        assert_eq!(
            result,
            OscDrainResult::Full {
                bytes_consumed: byte_offset(chunk.len()),
                reason: OscDrainReason::RunawayPayloadOngoing,
            }
        );
        assert_eq!(
            breaker,
            OscCircuitBreaker::Open {
                saw_partial_esc: false,
                drained_bytes: 100 + chunk.len(),
            }
        );
    }

    #[test]
    fn test_drain_chunk_full_chunk_terminated_by_bel() {
        let mut breaker = OscCircuitBreaker::default();
        breaker.trip(100);

        let chunk = b"payload\x07";
        let result = breaker.drain_chunk(chunk);
        assert_eq!(
            result,
            OscDrainResult::Full {
                bytes_consumed: byte_offset(8),
                reason: OscDrainReason::TerminatedByBel,
            }
        );
        assert_eq!(breaker, OscCircuitBreaker::Closed);
    }
}
