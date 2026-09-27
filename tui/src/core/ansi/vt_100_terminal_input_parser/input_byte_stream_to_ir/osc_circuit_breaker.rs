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
/// Prevents framing desynchronization and text leakage (where in-flight payload bytes
/// lose their escape framing and generate spurious input events into the event stream)
/// when an [`OSC`] sequence exceeds [`MAX_OSC_SEQUENCE_LENGTH`] (1 MiB).
///
/// # State Machine Lifecycle
///
/// This enum is stored inside a private field in the [`InputByteStreamToIrParser`]
/// struct.
///
/// The state machine transitions are driven entirely by
/// [`InputByteStreamToIrParser::advance()`]. It calls [`trip()`] when a runaway sequence
/// is detected, and [`drain_chunk()`] on all subsequent read chunks while the circuit
/// breaker is open.
///
/// | From State         | Event / Condition                   | Transition Method | To State           |
/// | :----------------- | :---------------------------------- | :---------------- | :----------------- |
/// | [`Closed`]         | Accumulator exceeds 1 MiB runaway   | [`trip()`]        | [`Open`]           |
/// | [`Open`]           | Lone [`ANSI_ESC`] at chunk boundary | [`drain_chunk()`] | [`OpenAwaitingSt`] |
/// | [`Open`]           | Terminator or syntax abort          | [`drain_chunk()`] | [`Closed`]         |
/// | [`OpenAwaitingSt`] | Resolving byte completes or aborts  | [`drain_chunk()`] | [`Closed`]         |
///
/// [`ANSI_ESC`]: crate::ANSI_ESC
/// [`Closed`]: Self::Closed
/// [`drain_chunk()`]: Self::drain_chunk
/// [`InputByteStreamToIrParser::advance()`]: super::InputByteStreamToIrParser::advance
/// [`InputByteStreamToIrParser`]: super::InputByteStreamToIrParser
/// [`MAX_OSC_SEQUENCE_LENGTH`]: crate::MAX_OSC_SEQUENCE_LENGTH
/// [`Open`]: Self::Open
/// [`OpenAwaitingSt`]: Self::OpenAwaitingSt
/// [`OSC`]: crate::osc_codes::OscSequence
/// [`trip()`]: Self::trip
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OscCircuitBreaker {
    /// Normal parsing state (circuit closed). Bytes are accumulated into
    /// [`InputByteStreamToIrParser`]'s internal buffer.
    ///
    /// Initial state, or reset by [`Self::drain_chunk()`]. See [State Machine Lifecycle]
    /// for transitions.
    ///
    /// [`InputByteStreamToIrParser`]: super::InputByteStreamToIrParser
    /// [State Machine Lifecycle]: Self#state-machine-lifecycle
    #[default]
    Closed,

    /// Circuit breaker tripped (open). Discarding bytes of an in-flight runaway [`OSC`]
    /// sequence until a terminator ([`ANSI_BEL`] `0x07` or 7-bit [`ANSI_ST_7BIT`] `ESC
    /// \`, `0x1B 0x5C`) or abort condition is encountered.
    ///
    /// Tripped from [`Self::Closed`] via [`Self::trip()`] (called by
    /// [`InputByteStreamToIrParser::advance()`]). See [State Machine Lifecycle] for
    /// transitions.
    ///
    /// [`ANSI_BEL`]: crate::ANSI_BEL
    /// [`ANSI_ST_7BIT`]: crate::ANSI_ST_7BIT
    /// [`InputByteStreamToIrParser::advance()`]: super::InputByteStreamToIrParser::advance
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [State Machine Lifecycle]: Self#state-machine-lifecycle
    Open {
        /// Total bytes drained so far across chunks (bounded by
        /// [`MAX_OSC_DRAIN_BYTES`]).
        ///
        /// Uses [`ByteOffset`] rather than [`ByteLength`] because this value tracks
        /// cumulative scanner cursor displacement across the stream rather than a
        /// container capacity. See [`ByteOffset` section on distance vs
        /// capacity][distance-vs-capacity] for the architectural distinction between
        /// displacement distance and container capacity.
        ///
        /// [`ByteLength`]: crate::ByteLength
        /// [`ByteOffset`]: crate::ByteOffset
        /// [`MAX_OSC_DRAIN_BYTES`]: crate::MAX_OSC_DRAIN_BYTES
        /// [distance-vs-capacity]:
        ///     crate::ByteOffset#distance-vs-capacity-byteoffset-vs-bytelength
        drained_bytes: ByteOffset,
    },

    /// Circuit breaker tripped (open) and awaiting resolution of a chunk-boundary escape.
    ///
    /// The immediately preceding chunk ended in an isolated [`ANSI_ESC`] (`0x1B`). The
    /// very next incoming byte must be inspected to determine if it completes a 7-bit
    /// string terminator [`ANSI_ST_7BIT`] (`ESC \`, bytes `0x1B 0x5C`) or aborts the
    /// [`OSC`] sequence.
    ///
    /// Split across chunk boundaries by [`Self::drain_chunk()`]. See [State Machine
    /// Lifecycle] for transitions.
    ///
    /// [`ANSI_ESC`]: crate::ANSI_ESC
    /// [`ANSI_ST_7BIT`]: crate::ANSI_ST_7BIT
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [State Machine Lifecycle]: Self#state-machine-lifecycle
    OpenAwaitingSt {
        /// Total bytes drained so far across chunks (bounded by
        /// [`MAX_OSC_DRAIN_BYTES`]).
        ///
        /// [`MAX_OSC_DRAIN_BYTES`]: crate::MAX_OSC_DRAIN_BYTES
        drained_bytes: ByteOffset,
    },
}

impl OscCircuitBreaker {
    /// Drains bytes from an incoming chunk while in [`Self::Open`].
    ///
    /// Scans for:
    /// 1. `BEL` (`\x07`) -> cleanly terminates [`OSC`].
    /// 2. `ST` (`ESC \`, `0x1B 0x5C`) -> cleanly terminates [`OSC`] (including across
    ///    chunk boundary if previous chunk ended in lone [`ANSI_ESC`]).
    /// 3. Abort conditions:
    ///    - Raw `\r` or `\n` (ECMA-48 / [`OSC`] payloads never contain raw CR/LF).
    ///    - [`ANSI_ESC`] followed by any byte other than backslash (`\`) (starts a new
    ///      escape sequence, aborting [`OSC`]).
    /// 4. Safety upper bound: cumulative drained bytes reaching [`MAX_OSC_DRAIN_BYTES`].
    ///
    /// Returns an [`OscDrainResult`] classifying whether the chunk was fully or partially
    /// consumed, or not consumed at all, along with the consumed [`ByteOffset`] (the
    /// displacement of the scanner cursor across the chunk slice) and domain reason.
    ///
    /// [`ANSI_ESC`]: crate::ANSI_ESC
    /// [`MAX_OSC_DRAIN_BYTES`]: crate::MAX_OSC_DRAIN_BYTES
    /// [`OSC`]: crate::osc_codes::OscSequence
    pub fn drain_chunk(&mut self, chunk: &[u8]) -> OscDrainResult {
        use OscCircuitBreaker::{Closed, Open, OpenAwaitingSt};

        let chunk_len = byte_offset(chunk.len());

        match *self {
            Closed => OscDrainResult::None {
                reason: OscDrainReason::CircuitClosed,
            },

            // Resolve partial ESC from previous chunk boundary.
            OpenAwaitingSt { drained_bytes } => {
                self.resolve_partial_esc(chunk, drained_bytes)
            }

            Open { mut drained_bytes } => {
                let mut byte_index_in_chunk = 0;

                while byte_index_in_chunk < chunk.len() {
                    // Check safety ceiling.
                    if drained_bytes >= byte_offset(MAX_OSC_DRAIN_BYTES) {
                        return self.reset_and_log(
                            Level::WARN,
                            OscDrainReason::ExceededSafetyCeiling,
                            byte_offset(byte_index_in_chunk),
                            chunk_len,
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
                                chunk_len,
                                drained_bytes + byte_offset(1),
                            );
                        }

                        // Terminated by 7-bit ST (ESC \).
                        [ANSI_ESC, ANSI_ST_FINAL, ..] => {
                            return self.reset_and_log(
                                Level::INFO,
                                OscDrainReason::TerminatedBySt,
                                byte_offset(byte_index_in_chunk + ANSI_ST_7BIT_LEN),
                                chunk_len,
                                drained_bytes + byte_offset(ANSI_ST_7BIT_LEN),
                            );
                        }

                        // ESC followed by non-backslash: aborts OSC string! Leave ESC for
                        // normal parsing.
                        [ANSI_ESC, _, ..] => {
                            return self.reset_and_log(
                                Level::INFO,
                                OscDrainReason::AbortedByNewEsc,
                                byte_offset(byte_index_in_chunk),
                                chunk_len,
                                drained_bytes,
                            );
                        }

                        // Lone ESC at end of chunk: remember across chunk boundaries.
                        [ANSI_ESC] => {
                            drained_bytes += byte_offset(1);
                            *self = Self::OpenAwaitingSt { drained_bytes };
                            return OscDrainResult::Full {
                                bytes_consumed: chunk_len,
                                reason: OscDrainReason::LoneEscAtBoundary,
                            };
                        }

                        // Raw newline/CR aborts OSC syntax. Leave newline for normal
                        // parsing.
                        [CARRIAGE_RETURN | LINE_FEED, ..] => {
                            return self.reset_and_log(
                                Level::INFO,
                                OscDrainReason::AbortedByNewline,
                                byte_offset(byte_index_in_chunk),
                                chunk_len,
                                drained_bytes,
                            );
                        }

                        // Regular payload byte: consume and continue draining.
                        _ => {
                            drained_bytes += byte_offset(1);
                            byte_index_in_chunk += 1;
                        }
                    }
                }

                // Entire chunk consumed without encountering terminator or abort.
                *self = Self::Open { drained_bytes };
                OscDrainResult::Full {
                    bytes_consumed: chunk_len,
                    reason: OscDrainReason::RunawayPayloadOngoing,
                }
            }
        }
    }

    /// Returns `true` if the circuit breaker is currently open (actively draining runaway
    /// bytes).
    #[must_use]
    pub fn is_open(&self) -> bool {
        matches!(self, Self::Open { .. } | Self::OpenAwaitingSt { .. })
    }

    /// Trips the circuit breaker to [`Self::Open`] with the specified initial count of
    /// drained bytes (typically the byte displacement of the purged accumulator).
    ///
    /// `initial_bytes` seeds the cumulative stream displacement vector with the offset
    /// reached by the accumulator scanner when the runaway sequence was detected.
    pub fn trip(&mut self, initial_bytes: ByteOffset) {
        *self = Self::Open {
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
        chunk_len: ByteOffset,
        total_drained_bytes: ByteOffset,
    ) -> OscDrainResult {
        DEBUG_TUI_SHOW_DIRECT_TO_ANSI.then(|| {
            // % is Display, ? is Debug.
            match level {
                Level::WARN => {
                    tracing::warn! {
                        message = "OscCircuitBreaker::drain_chunk",
                        status = %reason,
                        ?total_drained_bytes,
                    };
                }
                _ => {
                    tracing::info! {
                        message = "OscCircuitBreaker::drain_chunk",
                        status = %reason,
                        ?total_drained_bytes,
                    };
                }
            }
        });
        *self = Self::Closed;
        OscDrainResult::classify_drain(reason, bytes_consumed, chunk_len)
    }

    /// Resolves a lone [`ANSI_ESC`] that occurred at the end of the previous chunk.
    ///
    /// [`ANSI_ESC`]: crate::ANSI_ESC
    fn resolve_partial_esc(
        &mut self,
        chunk: &[u8],
        drained_bytes: ByteOffset,
    ) -> OscDrainResult {
        let chunk_len = byte_offset(chunk.len());
        match chunk.first() {
            None => OscDrainResult::Full {
                bytes_consumed: byte_offset(0),
                reason: OscDrainReason::LoneEscAtBoundary,
            },
            Some(&ANSI_ST_FINAL) => self.reset_and_log(
                Level::INFO,
                OscDrainReason::TerminatedAcrossBoundary,
                byte_offset(1),
                chunk_len,
                drained_bytes + byte_offset(1),
            ),
            Some(_) => self.reset_and_log(
                Level::INFO,
                OscDrainReason::AbortedByNewEsc,
                byte_offset(0),
                chunk_len,
                drained_bytes,
            ),
        }
    }
}

/// Result of an [`OSC`] drain operation, categorizing whether the chunk was
/// fully or partially consumed, or not consumed at all.
///
/// # Coordinate Semantics
///
/// `bytes_consumed` is typed as [`ByteOffset`] because it designates **scanner
/// cursor displacement** into the current chunk slice:
/// - In [`OscDrainResult::None`], zero bytes were consumed (e.g. circuit breaker was
///   closed).
/// - In [`OscDrainResult::Full`], the displacement equals the chunk slice boundary
///   (`bytes_consumed == chunk_len`).
/// - In [`OscDrainResult::Partial`], the displacement serves as the slicing offset where
///   the stream cursor stopped draining and where normal input parsing must resume
///   (`&chunk[*bytes_consumed..]`).
///
/// [`ByteOffset`]: crate::ByteOffset
/// [`OSC`]: crate::osc_codes::OscSequence
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OscDrainResult {
    /// Zero bytes were consumed by the circuit breaker (e.g., circuit breaker was
    /// closed). The entire chunk must be processed as normal input.
    None { reason: OscDrainReason },

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
    /// Creates an [`OscDrainResult`] classifying a drain operation as
    /// [`OscDrainResult::Full`] if `bytes_consumed == chunk_len`, or
    /// [`OscDrainResult::Partial`] otherwise.
    #[must_use]
    pub fn classify_drain(
        reason: OscDrainReason,
        bytes_consumed: ByteOffset,
        chunk_len: ByteOffset,
    ) -> Self {
        if bytes_consumed == chunk_len {
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
            Self::None { .. } => byte_offset(0),
            Self::Full { bytes_consumed, .. } | Self::Partial { bytes_consumed, .. } => {
                bytes_consumed
            }
        }
    }

    /// Returns the domain reason explaining the drain outcome.
    #[must_use]
    pub fn reason(&self) -> OscDrainReason {
        match *self {
            Self::None { reason }
            | Self::Full { reason, .. }
            | Self::Partial { reason, .. } => reason,
        }
    }
}

/// Reason specifying the exact condition, termination, or transition that occurred
/// during an [`OSC`] drain operation in [`OscCircuitBreaker`].
///
/// Each variant represents a distinct parsing outcome (clean termination by `BEL`/`ST`,
/// syntax abort by raw newline or new escape sequence, safety ceiling overflow,
/// ongoing runaway payload consumption, or inactive circuit breaker).
///
/// [`OSC`]: crate::osc_codes::OscSequence
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OscDrainReason {
    /// The circuit breaker is in [`OscCircuitBreaker::Closed`]. No bytes were
    /// drained, and any incoming bytes are preserved for normal input parsing.
    ///
    /// [`OscCircuitBreaker::Closed`]: OscCircuitBreaker::Closed
    CircuitClosed,

    /// Terminated cleanly by [`ANSI_BEL`] (`0x07`).
    ///
    /// [`ANSI_BEL`]: crate::ANSI_BEL
    TerminatedByBel,

    /// Terminated cleanly by 7-bit [`ANSI_ST_7BIT`] (`ESC \`, `0x1B 0x5C`).
    ///
    /// [`ANSI_ST_7BIT`]: crate::ANSI_ST_7BIT
    TerminatedBySt,

    /// Terminated cleanly by final byte of 7-bit [`ANSI_ST_7BIT`] ([`ASCII`] `\`, `0x5C`)
    /// across a chunk boundary.
    ///
    /// [`ANSI_ST_7BIT`]: crate::ANSI_ST_7BIT
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
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
    /// the circuit breaker transitions to [`OscCircuitBreaker::OpenAwaitingSt`]
    /// waiting for the next chunk.
    ///
    /// [`ANSI_ESC`]: crate::ANSI_ESC
    LoneEscAtBoundary,
}

impl Display for OscDrainReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::CircuitClosed => "circuit breaker closed; no payload drained",
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
            format!("{}", OscDrainReason::CircuitClosed),
            "circuit breaker closed; no payload drained"
        );
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
    fn test_osc_drain_result_classify_drain() {
        let full = OscDrainResult::classify_drain(
            OscDrainReason::RunawayPayloadOngoing,
            byte_offset(5),
            byte_offset(5),
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

        let partial = OscDrainResult::classify_drain(
            OscDrainReason::TerminatedByBel,
            byte_offset(3),
            byte_offset(5),
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

        let none = OscDrainResult::None {
            reason: OscDrainReason::CircuitClosed,
        };
        assert_eq!(none.bytes_consumed(), byte_offset(0));
        assert_eq!(none.reason(), OscDrainReason::CircuitClosed);
    }

    #[test]
    fn test_drain_chunk_terminated_by_bel() {
        let mut breaker = OscCircuitBreaker::default();
        breaker.trip(byte_offset(100));
        assert!(breaker.is_open());
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
        assert!(!breaker.is_open());
    }

    #[test]
    fn test_drain_chunk_terminated_by_st() {
        let mut breaker = OscCircuitBreaker::default();
        breaker.trip(byte_offset(100));

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
        breaker.trip(byte_offset(100));

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
            OscCircuitBreaker::OpenAwaitingSt {
                drained_bytes: byte_offset(108),
            }
        );
        assert!(breaker.is_open());

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
        breaker.trip(byte_offset(100));

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
        breaker.trip(byte_offset(100));

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
        breaker.trip(byte_offset(MAX_OSC_DRAIN_BYTES));

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
        breaker.trip(byte_offset(100));

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
                drained_bytes: byte_offset(100 + chunk.len()),
            }
        );
    }

    #[test]
    fn test_drain_chunk_full_chunk_terminated_by_bel() {
        let mut breaker = OscCircuitBreaker::default();
        breaker.trip(byte_offset(100));

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

    #[test]
    fn test_drain_chunk_when_closed() {
        let mut breaker = OscCircuitBreaker::Closed;

        // Non-empty chunk preserves all bytes for normal parsing.
        let chunk = b"regular_input_bytes";
        let result = breaker.drain_chunk(chunk);
        assert_eq!(
            result,
            OscDrainResult::None {
                reason: OscDrainReason::CircuitClosed,
            }
        );
        assert_eq!(result.bytes_consumed(), byte_offset(0));
        assert_eq!(result.reason(), OscDrainReason::CircuitClosed);
        assert_eq!(breaker, OscCircuitBreaker::Closed);

        // Empty chunk also returns None with zero bytes consumed.
        let empty_chunk = b"";
        let result_empty = breaker.drain_chunk(empty_chunk);
        assert_eq!(
            result_empty,
            OscDrainResult::None {
                reason: OscDrainReason::CircuitClosed,
            }
        );
        assert_eq!(result_empty.bytes_consumed(), byte_offset(0));
        assert_eq!(result_empty.reason(), OscDrainReason::CircuitClosed);
        assert_eq!(breaker, OscCircuitBreaker::Closed);
    }
}

// cspell:words byteoffset bytelength
