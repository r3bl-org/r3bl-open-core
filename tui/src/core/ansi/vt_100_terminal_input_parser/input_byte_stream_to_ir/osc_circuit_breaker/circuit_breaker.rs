// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{OscDrainReason, OscDrainResult};
use crate::{ANSI_BEL, ANSI_ESC, ANSI_ST_7BIT_TRANSPORT_ENCODING_LEN, ANSI_ST_FINAL,
            ByteOffset, CARRIAGE_RETURN, DEBUG_TUI_SHOW_DIRECT_TO_ANSI, LINE_FEED,
            MAX_OSC_DRAIN_BYTES, byte_offset};
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
/// [`InputByteStreamToIrParser::process_incoming_bytes()`]. It calls [`trip()`] when a
/// runaway sequence is detected, and [`try_drain()`] on all subsequent read chunks while
/// the circuit breaker is open.
///
/// | From State         | Event / Condition                   | Transition Method | To State           |
/// | :----------------- | :---------------------------------- | :---------------- | :----------------- |
/// | [`Closed`]         | Accumulator exceeds 1 MiB runaway   | [`trip()`]        | [`Open`]           |
/// | [`Open`]           | Lone [`ANSI_ESC`] at chunk boundary | [`try_drain()`]   | [`OpenAwaitingSt`] |
/// | [`Open`]           | Terminator or syntax abort          | [`try_drain()`]   | [`Closed`]         |
/// | [`OpenAwaitingSt`] | Resolving byte completes or aborts  | [`try_drain()`]   | [`Closed`]         |
///
/// [`ANSI_ESC`]: crate::ANSI_ESC
/// [`Closed`]: Self::Closed
/// [`InputByteStreamToIrParser::process_incoming_bytes()`]:
///     crate::core::ansi::vt_100_terminal_input_parser::input_byte_stream_to_ir::InputByteStreamToIrParser::process_incoming_bytes
/// [`InputByteStreamToIrParser`]:
///     crate::core::ansi::vt_100_terminal_input_parser::input_byte_stream_to_ir::InputByteStreamToIrParser
/// [`MAX_OSC_SEQUENCE_LENGTH`]: crate::MAX_OSC_SEQUENCE_LENGTH
/// [`Open`]: Self::Open
/// [`OpenAwaitingSt`]: Self::OpenAwaitingSt
/// [`OSC`]: crate::osc_codes::OscSequence
/// [`trip()`]: Self::trip
/// [`try_drain()`]: Self::try_drain
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OscCircuitBreaker {
    /// Normal parsing state (circuit closed). Bytes are accumulated into
    /// [`InputByteStreamToIrParser`]'s internal buffer.
    ///
    /// Initial state, or reset by [`Self::try_drain()`]. See [State Machine Lifecycle]
    /// for transitions.
    ///
    /// [`InputByteStreamToIrParser`]:
    ///     crate::core::ansi::vt_100_terminal_input_parser::input_byte_stream_to_ir::InputByteStreamToIrParser
    /// [State Machine Lifecycle]: Self#state-machine-lifecycle
    #[default]
    Closed,

    /// Circuit breaker tripped (open). Discarding bytes of an in-flight runaway [`OSC`]
    /// sequence until a terminator ([`ANSI_BEL`] `0x07` or 7-bit
    /// [`ANSI_ST_7BIT_TRANSPORT_ENCODING`] `ESC \`, `0x1B 0x5C`) or abort condition is
    /// encountered.
    ///
    /// Tripped from [`Self::Closed`] via [`Self::trip()`] (called by
    /// [`InputByteStreamToIrParser::process_incoming_bytes()`]). See [State Machine
    /// Lifecycle] for transitions.
    ///
    /// [`ANSI_BEL`]: crate::ANSI_BEL
    /// [`ANSI_ST_7BIT_TRANSPORT_ENCODING`]: crate::ANSI_ST_7BIT_TRANSPORT_ENCODING
    /// [`InputByteStreamToIrParser::process_incoming_bytes()`]:
    ///     crate::core::ansi::vt_100_terminal_input_parser::input_byte_stream_to_ir::InputByteStreamToIrParser::process_incoming_bytes
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
        already_drained_byte_count: ByteOffset,
    },

    /// Circuit breaker tripped (open) and awaiting resolution of a chunk-boundary escape.
    ///
    /// The immediately preceding chunk ended in an isolated [`ANSI_ESC`] (`0x1B`). The
    /// very next incoming byte must be inspected to determine if it completes a 7-bit
    /// string terminator [`ANSI_ST_7BIT_TRANSPORT_ENCODING`] (`ESC \`, bytes `0x1B 0x5C`)
    /// or aborts the [`OSC`] sequence.
    ///
    /// Split across chunk boundaries by [`Self::try_drain()`]. See [State Machine
    /// Lifecycle] for transitions.
    ///
    /// [`ANSI_ESC`]: crate::ANSI_ESC
    /// [`ANSI_ST_7BIT_TRANSPORT_ENCODING`]: crate::ANSI_ST_7BIT_TRANSPORT_ENCODING
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [State Machine Lifecycle]: Self#state-machine-lifecycle
    OpenAwaitingSt {
        /// Total bytes drained so far across chunks (bounded by
        /// [`MAX_OSC_DRAIN_BYTES`]).
        ///
        /// [`MAX_OSC_DRAIN_BYTES`]: crate::MAX_OSC_DRAIN_BYTES
        already_drained_byte_count: ByteOffset,
    },
}

impl OscCircuitBreaker {
    /// Trips the circuit breaker to [`Self::Open`] with the specified initial count of
    /// drained bytes (typically the byte displacement of the purged accumulator).
    ///
    /// # Arguments
    ///
    /// - `already_drained_byte_count`: Seeds the cumulative stream displacement vector
    ///   with the [`ByteOffset`] reached by the accumulator scanner when the runaway
    ///   sequence was detected and purged.
    ///
    /// [`ByteOffset`]: crate::ByteOffset
    pub fn trip(&mut self, already_drained_byte_count: ByteOffset) {
        *self = Self::Open {
            already_drained_byte_count,
        };
    }

    /// Resets the circuit breaker to [`Self::Closed`], logs the transition at the
    /// specified [`Level`], and returns the resulting [`OscDrainResult`].
    ///
    /// # Arguments
    ///
    /// - `chunk`: Current chunk byte slice being evaluated and classified upon resetting
    ///   the circuit breaker.
    /// - `level`: Tracing [`Level`] used for logging the circuit breaker reset.
    /// - `reason`: Specific [`OscDrainReason`] for closing the circuit breaker.
    /// - `bytes_consumed`: Scanner cursor displacement ([`ByteOffset`]) into `chunk`
    ///   consumed by this termination or abort sequence.
    /// - `already_drained_byte_count`: Cumulative count of bytes ([`ByteOffset`]) drained
    ///   across chunks so far, recorded in structured diagnostic logs.
    ///
    /// [`ByteOffset`]: crate::ByteOffset
    /// [`Level`]: tracing::Level
    /// [`OscDrainReason`]: OscDrainReason
    /// [`OscDrainResult`]: OscDrainResult
    fn reset<'a>(
        &mut self,
        chunk: &'a [u8],
        level: Level,
        reason: OscDrainReason,
        bytes_consumed: ByteOffset,
        already_drained_byte_count: ByteOffset,
    ) -> OscDrainResult<'a> {
        DEBUG_TUI_SHOW_DIRECT_TO_ANSI.then(|| {
            // % is Display, ? is Debug.
            match level {
                Level::WARN => {
                    tracing::warn! {
                        message = "OscCircuitBreaker::try_drain",
                        status = %reason,
                        ?already_drained_byte_count,
                    };
                }
                _ => {
                    tracing::info! {
                        message = "OscCircuitBreaker::try_drain",
                        status = %reason,
                        ?already_drained_byte_count,
                    };
                }
            }
        });

        *self = Self::Closed;

        OscDrainResult::classify_drain(chunk, reason, bytes_consumed)
    }

    /// Drains bytes from an incoming chunk of bytes while in [`Self::Open`] state.
    ///
    /// Scans for the two valid [`OSC`] terminators in modern terminals:
    /// 1. `BEL` (`\x07`) -> cleanly terminates [`OSC`].
    /// 2. 7-bit `ST` (`ESC \`, `0x1B 0x5C`) -> cleanly terminates [`OSC`] (including
    ///    across chunk boundary if previous chunk ended in lone [`ANSI_ESC`]).
    /// 3. Abort conditions:
    ///    - Raw `\r` or `\n` (ECMA-48 / [`OSC`] payloads never contain raw CR/LF).
    ///    - [`ANSI_ESC`] followed by any byte other than backslash (`\`) (starts a new
    ///      escape sequence, aborting [`OSC`]).
    /// 4. Safety upper bound: cumulative drained bytes reaching [`MAX_OSC_DRAIN_BYTES`].
    ///
    /// Note that 8-bit ST (`0x9C`) is intentionally ignored for [`UTF-8`] safety; see
    /// [`ANSI_ST_7BIT_TRANSPORT_ENCODING`] for details.
    ///
    /// Returns an [`OscDrainResult`] classifying whether the chunk was fully or partially
    /// consumed, or not consumed at all, along with the consumed [`ByteOffset`] (the
    /// displacement of the scanner cursor across the chunk slice) and domain reason.
    ///
    /// > This [article] has more details on mutable reborrowing. `&mut *self` breaks down
    /// > into:
    /// > - `*self`: Dereference the reference to access the enum in place (in memory).
    /// > - `&mut`: Fresh and temporary reborrow of the enum.
    ///
    /// # Arguments
    ///
    /// - `chunk`: Incoming raw byte slice read from [`stdin`] to evaluate and drain.
    ///
    /// [`ANSI_ESC`]: crate::ANSI_ESC
    /// [`ANSI_ST_7BIT_TRANSPORT_ENCODING`]: crate::ANSI_ST_7BIT_TRANSPORT_ENCODING
    /// [`ByteOffset`]: crate::ByteOffset
    /// [`MAX_OSC_DRAIN_BYTES`]: crate::MAX_OSC_DRAIN_BYTES
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [`stdin`]: std::io::stdin
    /// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
    /// [article]: https://developerlife.com/2026/09/25/rust-reborrowing/
    pub fn try_drain<'a>(&mut self, chunk: &'a [u8]) -> OscDrainResult<'a> {
        let current_state = &mut *self; // Mutable reborrow.
        match *current_state {
            // The circuit breaker is closed, don't drain any bytes, and pass the chunk
            // through intact as valid input bytes in the next stage.
            Self::Closed => OscDrainResult::NotDrained(chunk),

            // Resolve partial ESC from previous chunk boundary.
            Self::OpenAwaitingSt {
                already_drained_byte_count,
            } => current_state.handle_awaiting_st(chunk, already_drained_byte_count),

            // Drain bytes from an incoming chunk while the circuit breaker is open.
            Self::Open {
                already_drained_byte_count,
            } => current_state.handle_open(chunk, already_drained_byte_count),
        }
    }

    /// Handles an incoming chunk while the circuit breaker is in
    /// [`Self::OpenAwaitingSt`].
    ///
    /// # Precondition: How did we get here?
    ///
    /// While draining a runaway [`OSC`] sequence in [`Self::Open`], the immediately
    /// preceding chunk ended in a lone [`ANSI_ESC`] (`0x1B`). Because the chunk boundary
    /// split the stream right after [`ESC`], the circuit breaker transitioned into
    /// [`Self::OpenAwaitingSt`] and consumed the chunk, pausing until the first byte of
    /// the next chunk could be inspected.
    ///
    /// # Resolution: What do we do now?
    ///
    /// Inspects the first byte of `chunk` to resolve the boundary across 3 cases:
    /// 1. `Some(&ANSI_ST_FINAL)` (`\`): Completes the split 7-bit ST (`ESC \`) sequence
    ///    across chunk boundaries. Consumes 1 byte (`\`), logs
    ///    [`OscDrainReason::TerminatedAcrossBoundary`], and resets the circuit breaker to
    ///    [`Self::Closed`].
    /// 2. `Some(_)` (any other byte): The previous [`ESC`] was not an ST terminator, but
    ///    started a new escape sequence or abort condition. Consumes 0 bytes from this
    ///    chunk (`byte_offset(0)`), logs [`OscDrainReason::AbortedByNewEsc`], and resets
    ///    to [`Self::Closed`] so normal input parsing resumes immediately with this
    ///    chunk.
    /// 3. `None` (empty chunk): No bytes are available yet to resolve the boundary.
    ///    Retains [`Self::OpenAwaitingSt`] and returns [`OscDrainResult::FullyDrained`]
    ///    to await subsequent chunk reads.
    ///
    /// # Arguments
    ///
    /// - `chunk`: Incoming byte slice whose first byte is inspected to resolve the chunk
    ///   boundary while in [`Self::OpenAwaitingSt`].
    /// - `already_drained_byte_count`: Cumulative scanner cursor displacement
    ///   ([`ByteOffset`]) drained prior to this chunk, including the trailing
    ///   [`ANSI_ESC`] byte from the previous chunk.
    ///
    /// [`ANSI_ESC`]: crate::ANSI_ESC
    /// [`ANSI_ST_FINAL`]: crate::ANSI_ST_FINAL
    /// [`ByteOffset`]: crate::ByteOffset
    /// [`ESC`]: crate::EscSequence
    /// [`OSC`]: crate::osc_codes::OscSequence
    fn handle_awaiting_st<'a>(
        &mut self,
        chunk: &'a [u8],
        already_drained_byte_count: ByteOffset,
    ) -> OscDrainResult<'a> {
        match chunk.first() {
            // Completes split 7-bit ST (ESC \): consume '\' and close (see case 1 above).
            Some(&ANSI_ST_FINAL) => self.reset(
                chunk,
                Level::INFO,
                OscDrainReason::TerminatedAcrossBoundary,
                byte_offset(1),
                already_drained_byte_count + byte_offset(1),
            ),

            // Aborted by new escape sequence: preserve chunk intact and close (see case 2
            // above).
            Some(_) => self.reset(
                chunk,
                Level::INFO,
                OscDrainReason::AbortedByNewEsc,
                byte_offset(0),
                already_drained_byte_count,
            ),

            // No bytes available yet: retain OpenAwaitingSt for next read (see case 3
            // above).
            None => OscDrainResult::FullyDrained {
                reason: OscDrainReason::LoneEscAtBoundary,
            },
        }
    }

    /// Handles an incoming chunk while the circuit breaker is in [`Self::Open`].
    ///
    /// Scans bytes linearly, checking for the following:
    /// - safety upper bound [`MAX_OSC_DRAIN_BYTES`],
    /// - terminators: [`ANSI_BEL`] or 7-bit [`ANSI_ST_7BIT_TRANSPORT_ENCODING`],
    /// - syntax aborts: [`CARRIAGE_RETURN`], [`LINE_FEED`],
    /// - unexpected escape sequences,
    /// - chunk boundaries ending in a lone [`ANSI_ESC`].
    ///
    /// # Arguments
    ///
    /// - `chunk`: Incoming byte slice to scan and drain while the circuit breaker is in
    ///   [`Self::Open`].
    /// - `already_drained_byte_count`: Cumulative scanner cursor displacement
    ///   ([`ByteOffset`]) already drained prior to scanning this chunk.
    ///
    /// [`ANSI_BEL`]: crate::ANSI_BEL
    /// [`ANSI_ESC`]: crate::ANSI_ESC
    /// [`ANSI_ST_7BIT_TRANSPORT_ENCODING`]: crate::ANSI_ST_7BIT_TRANSPORT_ENCODING
    /// [`ByteOffset`]: crate::ByteOffset
    /// [`CARRIAGE_RETURN`]: crate::CARRIAGE_RETURN
    /// [`LINE_FEED`]: crate::LINE_FEED
    /// [`MAX_OSC_DRAIN_BYTES`]: crate::MAX_OSC_DRAIN_BYTES
    fn handle_open<'a>(
        &mut self,
        chunk: &'a [u8],
        already_drained_byte_count: ByteOffset,
    ) -> OscDrainResult<'a> {
        // Slice-as-cursor tracking unconsumed bytes in the incoming chunk.
        let mut remaining = chunk;

        // Loop until a terminator or abort condition is met, or the chunk is exhausted.
        while !remaining.is_empty() {
            // Relative displacement into chunk and total cumulative bytes drained so far.
            let bytes_consumed = byte_offset(chunk.len() - remaining.len());
            let current_drained = already_drained_byte_count + bytes_consumed;

            // Check safety ceiling.
            if current_drained >= byte_offset(MAX_OSC_DRAIN_BYTES) {
                return self.reset(
                    chunk,
                    Level::WARN,
                    OscDrainReason::ExceededSafetyCeiling,
                    bytes_consumed,
                    current_drained,
                );
            }

            match remaining {
                // Terminated by BEL (0x07).
                [ANSI_BEL, ..] => {
                    let total_consumed = bytes_consumed + byte_offset(1);
                    return self.reset(
                        chunk,
                        Level::INFO,
                        OscDrainReason::TerminatedByBel,
                        total_consumed,
                        already_drained_byte_count + total_consumed,
                    );
                }

                // Terminated by 7-bit ST (ESC \).
                [ANSI_ESC, ANSI_ST_FINAL, ..] => {
                    let total_consumed =
                        bytes_consumed + byte_offset(ANSI_ST_7BIT_TRANSPORT_ENCODING_LEN);
                    return self.reset(
                        chunk,
                        Level::INFO,
                        OscDrainReason::TerminatedBySt,
                        total_consumed,
                        already_drained_byte_count + total_consumed,
                    );
                }

                // ESC followed by non-backslash: aborts OSC string! Leave ESC for
                // normal parsing.
                [ANSI_ESC, _, ..] => {
                    return self.reset(
                        chunk,
                        Level::INFO,
                        OscDrainReason::AbortedByNewEsc,
                        bytes_consumed,
                        current_drained,
                    );
                }

                // Lone ESC is the final byte of the chunk: the 2-byte 7-bit ST (`ESC \`)
                // sequence may be split across read chunk boundaries - we don't know.
                // Transition to OpenAwaitingSt to inspect the first byte of the next
                // chunk.
                [ANSI_ESC] => {
                    *self = Self::OpenAwaitingSt {
                        already_drained_byte_count: already_drained_byte_count
                            + byte_offset(chunk.len()),
                    };
                    return OscDrainResult::FullyDrained {
                        reason: OscDrainReason::LoneEscAtBoundary,
                    };
                }

                // Raw newline/CR aborts OSC syntax. Leave newline for normal
                // parsing.
                [CARRIAGE_RETURN | LINE_FEED, ..] => {
                    return self.reset(
                        chunk,
                        Level::INFO,
                        OscDrainReason::AbortedByNewline,
                        bytes_consumed,
                        current_drained,
                    );
                }

                // This must be the last match arm.
                // Regular payload byte: consume the first byte and advance the slice:
                // - `_` matches and discards the first byte (head).
                // - `rest @ ..` binds the rest of the slice (tail with the first byte
                //   removed).
                // - `remaining = rest` assigns the tail back to advance the slice cursor.
                [_, rest @ ..] => {
                    remaining = rest;
                }

                [] => break,
            }
        }

        // Entire chunk swallowed as ongoing runaway payload without encountering a
        // terminator or abort. Retain Open state with the accumulated displacement to
        // continue draining subsequent chunks.
        *self = Self::Open {
            already_drained_byte_count: already_drained_byte_count
                + byte_offset(chunk.len()),
        };

        OscDrainResult::FullyDrained {
            reason: OscDrainReason::RunawayPayloadOngoing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_drain_terminated_by_bel() {
        let mut breaker = OscCircuitBreaker::default();
        breaker.trip(byte_offset(100));
        assert!(matches!(breaker, OscCircuitBreaker::Open { .. }));
        assert_ne!(breaker, OscCircuitBreaker::Closed);

        let chunk = b"payload\x07trailing";
        let result = breaker.try_drain(chunk);
        assert_eq!(
            result,
            OscDrainResult::PartiallyDrained {
                undrained_bytes: b"trailing",
                bytes_consumed: byte_offset(8),
                reason: OscDrainReason::TerminatedByBel,
            }
        );
        assert_eq!(result.undrained_bytes(), b"trailing");
        assert_eq!(breaker, OscCircuitBreaker::Closed);
    }

    #[test]
    fn test_try_drain_terminated_by_st() {
        let mut breaker = OscCircuitBreaker::default();
        breaker.trip(byte_offset(100));

        let chunk = b"payload\x1b\\trailing";
        let result = breaker.try_drain(chunk);
        assert_eq!(
            result,
            OscDrainResult::PartiallyDrained {
                undrained_bytes: b"trailing",
                bytes_consumed: byte_offset(9),
                reason: OscDrainReason::TerminatedBySt,
            }
        );
        assert_eq!(result.undrained_bytes(), b"trailing");
        assert_eq!(breaker, OscCircuitBreaker::Closed);
    }

    #[test]
    fn test_try_drain_partial_esc_across_boundaries() {
        let mut breaker = OscCircuitBreaker::default();
        breaker.trip(byte_offset(100));

        // Chunk 1 ends in lone ESC.
        let chunk1 = b"payload\x1b";
        let result1 = breaker.try_drain(chunk1);
        assert_eq!(
            result1,
            OscDrainResult::FullyDrained {
                reason: OscDrainReason::LoneEscAtBoundary,
            }
        );
        assert_eq!(result1.undrained_bytes(), b"");
        assert_eq!(
            breaker,
            OscCircuitBreaker::OpenAwaitingSt {
                already_drained_byte_count: byte_offset(108),
            }
        );

        // Chunk 2 starts with '\', completing ST.
        let chunk2 = b"\\trailing";
        let result2 = breaker.try_drain(chunk2);
        assert_eq!(
            result2,
            OscDrainResult::PartiallyDrained {
                undrained_bytes: b"trailing",
                bytes_consumed: byte_offset(1),
                reason: OscDrainReason::TerminatedAcrossBoundary,
            }
        );
        assert_eq!(result2.undrained_bytes(), b"trailing");
        assert_eq!(breaker, OscCircuitBreaker::Closed);
    }

    #[test]
    fn test_try_drain_partial_esc_aborted_by_other_char() {
        let mut breaker = OscCircuitBreaker::default();
        breaker.trip(byte_offset(100));

        // Chunk 1 ends in lone ESC.
        let chunk1 = b"payload\x1b";
        let result1 = breaker.try_drain(chunk1);
        assert_eq!(
            result1,
            OscDrainResult::FullyDrained {
                reason: OscDrainReason::LoneEscAtBoundary,
            }
        );
        assert_eq!(result1.undrained_bytes(), b"");

        // Chunk 2 starts with '[', aborting OSC.
        let chunk2 = b"[A";
        let result2 = breaker.try_drain(chunk2);
        assert_eq!(
            result2,
            OscDrainResult::PartiallyDrained {
                undrained_bytes: b"[A",
                bytes_consumed: byte_offset(0),
                reason: OscDrainReason::AbortedByNewEsc,
            }
        );
        assert_eq!(result2.undrained_bytes(), b"[A");
        assert_eq!(breaker, OscCircuitBreaker::Closed);
    }

    #[test]
    fn test_try_drain_aborted_by_newline() {
        let mut breaker = OscCircuitBreaker::default();
        breaker.trip(byte_offset(100));

        let chunk = b"payload\nrest";
        let result = breaker.try_drain(chunk);
        assert_eq!(
            result,
            OscDrainResult::PartiallyDrained {
                undrained_bytes: b"\nrest",
                bytes_consumed: byte_offset(7),
                reason: OscDrainReason::AbortedByNewline,
            }
        );
        assert_eq!(result.undrained_bytes(), b"\nrest");
        assert_eq!(breaker, OscCircuitBreaker::Closed);
    }

    #[test]
    fn test_try_drain_safety_ceiling() {
        let mut breaker = OscCircuitBreaker::default();
        breaker.trip(byte_offset(MAX_OSC_DRAIN_BYTES));

        let chunk = b"more_data";
        let result = breaker.try_drain(chunk);
        assert_eq!(
            result,
            OscDrainResult::PartiallyDrained {
                undrained_bytes: b"more_data",
                bytes_consumed: byte_offset(0),
                reason: OscDrainReason::ExceededSafetyCeiling,
            }
        );
        assert_eq!(result.undrained_bytes(), b"more_data");
        assert_eq!(breaker, OscCircuitBreaker::Closed);
    }

    #[test]
    fn test_try_drain_runaway_payload_full_chunk() {
        let mut breaker = OscCircuitBreaker::default();
        breaker.trip(byte_offset(100));

        let chunk = b"pure_payload_data";
        let result = breaker.try_drain(chunk);
        assert_eq!(
            result,
            OscDrainResult::FullyDrained {
                reason: OscDrainReason::RunawayPayloadOngoing,
            }
        );
        assert_eq!(result.undrained_bytes(), b"");
        assert_eq!(
            breaker,
            OscCircuitBreaker::Open {
                already_drained_byte_count: byte_offset(100 + chunk.len()),
            }
        );
    }

    #[test]
    fn test_try_drain_full_chunk_terminated_by_bel() {
        let mut breaker = OscCircuitBreaker::default();
        breaker.trip(byte_offset(100));

        let chunk = b"payload\x07";
        let result = breaker.try_drain(chunk);
        assert_eq!(
            result,
            OscDrainResult::FullyDrained {
                reason: OscDrainReason::TerminatedByBel,
            }
        );
        assert_eq!(result.undrained_bytes(), b"");
        assert_eq!(breaker, OscCircuitBreaker::Closed);
    }

    #[test]
    fn test_try_drain_when_closed() {
        let mut breaker = OscCircuitBreaker::Closed;

        // Non-empty chunk preserves all bytes for normal parsing.
        let chunk = b"regular_input_bytes";
        let result = breaker.try_drain(chunk);
        assert_eq!(result, OscDrainResult::NotDrained(chunk));
        assert_eq!(result.undrained_bytes(), b"regular_input_bytes");
        assert_eq!(breaker, OscCircuitBreaker::Closed);

        // Empty chunk also returns NotDrained with zero bytes consumed.
        let empty_chunk = b"";
        let result_empty = breaker.try_drain(empty_chunk);
        assert_eq!(result_empty, OscDrainResult::NotDrained(empty_chunk));
        assert_eq!(result_empty.undrained_bytes(), b"");
        assert_eq!(breaker, OscCircuitBreaker::Closed);
    }
}

// cspell:words byteoffset bytelength
