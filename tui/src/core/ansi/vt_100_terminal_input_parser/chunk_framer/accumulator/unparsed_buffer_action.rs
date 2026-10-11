// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Recovery action evaluation for unparsed residual bytes in [`ChunkAccumulator`].
//!
//! [`ChunkAccumulator`]: super::ChunkAccumulator

use super::constants::MAX_ESCAPE_SEQUENCE_LENGTH;
use crate::{ANSI_CSI_BRACKET, ANSI_ESC, ANSI_OSC_CLOSE_BRACKET, ANSI_SS3_O,
            CSI_FINAL_BYTE_MAX, CSI_FINAL_BYTE_MIN,
            core::ansi::vt_100_terminal_input_parser::chunk_decoder::OscScanResult};

/// Action required by [`ChunkFramer`] for unparsed residual bytes in
/// [`ChunkAccumulator`].
///
/// # Parser Pipeline: Happy Path vs. Unhappy Path
///
/// 1. **Happy Path (Primary Sequence Decoder)**: When incoming bytes form a valid,
///    supported escape sequence (such as arrow keys `\x1b[A`, mouse clicks
///    `\x1b[<0;10;20M`, or `Shift+Home` `\x1b[1;2H`):
///    - [`try_decode_input_event()`] successfully decodes the slice into
///      `Some(ParsedInputEventIR)`.
///    - The event is queued and parsed bytes are consumed from the accumulator.
///    - **This action evaluator is never called on valid sequences.**
///
/// 2. **Unhappy / Recovery Path (Decoder Returned `None`)**: This evaluator is
///    **exclusively invoked** when [`try_decode_input_event()`] returns `None` (the
///    sequence decoder failed to recognize any event from the accumulated bytes). This
///    enum diagnoses the residual unparsed buffer to tell [`ChunkFramer`] what recovery
///    action must be taken:
///    - [`KeepAndAwaitMore`][Self::KeepAndAwaitMore]: Normal stream fragmentation; keep
///      buffer intact and await subsequent reads.
///    - [`PurgeMalformed`][Self::PurgeMalformed]: The sequence structurally completed
///      (e.g., an ECMA-48 final byte in `0x40..=0x7E` arrived), but was unrecognized.
///      Because waiting for more bytes can never make an already-finished sequence valid,
///      the buffer must be purged immediately to prevent accumulator poisoning.
///    - [`TripCircuitBreaker`][Self::TripCircuitBreaker]: Oversized [`OSC`] payload; trip
///      circuit breaker into streaming drain mode to swallow the runaway stream without
///      allocations.
///
/// [`ChunkAccumulator`]: super::ChunkAccumulator
/// [`ChunkFramer`]: crate::core::ansi::vt_100_terminal_input_parser::chunk_framer::ChunkFramer
/// [`OSC`]: crate::osc_codes::OscSequence
/// [`try_decode_input_event()`]: crate::core::ansi::vt_100_terminal_input_parser::chunk_decoder::try_decode_input_event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnparsedBufferAction {
    /// Incomplete sequence awaiting additional bytes from subsequent reads.
    ///
    /// The framer leaves the accumulator intact and waits for the next read from
    /// [`stdin`].
    ///
    /// [`stdin`]: std::io::stdin
    KeepAndAwaitMore,

    /// Completed but malformed, unsupported, or overflowing non-[`OSC`] sequence
    /// (e.g., completed [`CSI`] sequence whose final byte is in `0x40..=0x7E`,
    /// unrecognized [`SS3`] sequence, or >64 bytes of escape syntax).
    ///
    /// The framer purges the accumulator immediately to prevent stream lockup.
    ///
    /// [`CSI`]: crate::CsiSequence
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [`SS3`]: https://en.wikipedia.org/wiki/ANSI_escape_code#SS3
    PurgeMalformed,

    /// Oversized [`OSC`] sequence exceeding [`MAX_OSC_SEQUENCE_LENGTH`] (1 MiB) without a
    /// terminator.
    ///
    /// The framer trips the circuit breaker: purges the 1 MiB accumulator and enters
    /// streaming drain mode to swallow the remainder without allocations.
    ///
    /// [`MAX_OSC_SEQUENCE_LENGTH`]: crate::MAX_OSC_SEQUENCE_LENGTH
    /// [`OSC`]: crate::osc_codes::OscSequence
    TripCircuitBreaker,
}

impl UnparsedBufferAction {
    /// Evaluates an unparsed byte slice into [`Self::KeepAndAwaitMore`],
    /// [`Self::PurgeMalformed`], or [`Self::TripCircuitBreaker`].
    ///
    /// Exclusively invoked when [`try_decode_input_event()`] returns `None`.
    ///
    /// Performs a single-pass classification across 4 criteria:
    /// 1. **[`OSC`] sequence**: Scans for runaway (> 1 MiB) or incomplete state via
    ///    [`OscScanResult::scan()`].
    /// 2. **Completed [`CSI`]**: Reached a final byte in `0x40..=0x7E`
    ///    ([`CSI_FINAL_BYTE_MIN`] through [`CSI_FINAL_BYTE_MAX`]) but could not be
    ///    parsed.
    /// 3. **Completed [`SS3`]**: Reached [`SS3_SEQ_LEN`] (3 bytes) but could not be
    ///    parsed.
    /// 4. **Safety overflow**: Non-[`OSC`] sequence reached or exceeded
    ///    [`MAX_ESCAPE_SEQUENCE_LENGTH`] (64 bytes).
    ///
    /// [`CSI_FINAL_BYTE_MAX`]: crate::CSI_FINAL_BYTE_MAX
    /// [`CSI_FINAL_BYTE_MIN`]: crate::CSI_FINAL_BYTE_MIN
    /// [`CSI`]: crate::CsiSequence
    /// [`MAX_ESCAPE_SEQUENCE_LENGTH`]: MAX_ESCAPE_SEQUENCE_LENGTH
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [`OscScanResult::scan()`]: crate::core::ansi::vt_100_terminal_input_parser::chunk_decoder::osc_scanner::OscScanResult::scan
    /// [`SS3_SEQ_LEN`]: crate::SS3_SEQ_LEN
    /// [`SS3`]: https://en.wikipedia.org/wiki/ANSI_escape_code#SS3
    /// [`try_decode_input_event()`]: crate::core::ansi::vt_100_terminal_input_parser::chunk_decoder::try_decode_input_event
    #[must_use]
    pub fn determine_action(unparsed_bytes: &[u8]) -> Self {
        match unparsed_bytes {
            // OSC sequence check (1 MiB threshold).
            [ANSI_ESC, ANSI_OSC_CLOSE_BRACKET, ..] => {
                Self::determine_osc_action(unparsed_bytes)
            }

            // Completed CSI sequence that could not be parsed (`ESC [ ...` followed by a
            // final byte in `0x40..=0x7E`).
            [ANSI_ESC, ANSI_CSI_BRACKET, bytes_after_prefix @ ..] => {
                Self::determine_csi_action(bytes_after_prefix, unparsed_bytes.len())
            }

            // Completed SS3 sequence that could not be parsed (`ESC O <key>` reaching
            // SS3_SEQ_LEN = 3 bytes).
            [ANSI_ESC, ANSI_SS3_O, _, ..] => Self::PurgeMalformed,

            // Safety fallback for non-OSC sequences exceeding 64 bytes.
            _ if unparsed_bytes.len() >= MAX_ESCAPE_SEQUENCE_LENGTH => {
                Self::PurgeMalformed
            }

            // Incomplete sequence waiting for subsequent bytes.
            _ => Self::KeepAndAwaitMore,
        }
    }

    /// Evaluates an [`OSC`] sequence by delegating to [`OscScanResult::scan()`].
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [`OscScanResult::scan()`]: crate::core::ansi::vt_100_terminal_input_parser::chunk_decoder::osc_scanner::OscScanResult::scan
    #[must_use]
    fn determine_osc_action(unparsed_bytes: &[u8]) -> Self {
        let scan_result = OscScanResult::scan(unparsed_bytes);
        match scan_result {
            OscScanResult::Runaway => Self::TripCircuitBreaker,
            OscScanResult::IncompleteDigits | OscScanResult::IncompletePayload => {
                Self::KeepAndAwaitMore
            }
            OscScanResult::Complete(_) | OscScanResult::InvalidSyntax => {
                Self::PurgeMalformed
            }
        }
    }

    /// Evaluates a [`CSI`] sequence whose prefix is `ESC [`.
    ///
    /// Evaluates whether the sequence contains an ECMA-48 terminating final byte in
    /// the range `0x40..=0x7E` ([`CSI_FINAL_BYTE_MIN`] through [`CSI_FINAL_BYTE_MAX`])
    /// or exceeds the safety buffer length threshold ([`MAX_ESCAPE_SEQUENCE_LENGTH`]).
    ///
    /// [`CSI_FINAL_BYTE_MAX`]: crate::CSI_FINAL_BYTE_MAX
    /// [`CSI_FINAL_BYTE_MIN`]: crate::CSI_FINAL_BYTE_MIN
    /// [`CSI`]: crate::CsiSequence
    /// [`MAX_ESCAPE_SEQUENCE_LENGTH`]: MAX_ESCAPE_SEQUENCE_LENGTH
    #[must_use]
    fn determine_csi_action(bytes_after_prefix: &[u8], total_len: usize) -> Self {
        const fn is_csi_final_byte(byte: u8) -> bool {
            (byte >= CSI_FINAL_BYTE_MIN) && (byte <= CSI_FINAL_BYTE_MAX)
        }

        let contains_final_byte =
            bytes_after_prefix.iter().copied().any(is_csi_final_byte);

        if contains_final_byte || total_len >= MAX_ESCAPE_SEQUENCE_LENGTH {
            Self::PurgeMalformed
        } else {
            Self::KeepAndAwaitMore
        }
    }
}

#[cfg(test)]
mod tests_unparsed_buffer_action {
    use super::*;
    use crate::{CSI_PREFIX, MAX_OSC_SEQUENCE_LENGTH, OSC_PREFIX};

    #[test]
    fn incomplete_sequences() {
        // Empty buffer.
        assert_eq!(
            UnparsedBufferAction::determine_action(b""),
            UnparsedBufferAction::KeepAndAwaitMore
        );

        // Incomplete CSI sequence.
        assert_eq!(
            UnparsedBufferAction::determine_action(CSI_PREFIX),
            UnparsedBufferAction::KeepAndAwaitMore
        );

        // Incomplete OSC sequence (< 1 MiB).
        assert_eq!(
            UnparsedBufferAction::determine_action(b"\x1b]11;rgb"),
            UnparsedBufferAction::KeepAndAwaitMore
        );
    }

    #[test]
    fn malformed_csi_sequence() {
        // Completed CSI sequence with an unknown final byte (e.g. 'z').
        assert_eq!(
            UnparsedBufferAction::determine_action(b"\x1b[999z"),
            UnparsedBufferAction::PurgeMalformed
        );
    }

    #[test]
    fn malformed_ss3_sequence() {
        // Completed SS3 sequence with an unrecognized key byte.
        assert_eq!(
            UnparsedBufferAction::determine_action(b"\x1bO?"),
            UnparsedBufferAction::PurgeMalformed
        );
    }

    #[test]
    fn safety_buffer_overflow() {
        // Non-OSC sequence exceeding 64 bytes without completing.
        let mut overflow = Vec::with_capacity(65);
        overflow.extend_from_slice(b"\x1b?");
        overflow.resize(65, b'x');
        assert_eq!(
            UnparsedBufferAction::determine_action(&overflow),
            UnparsedBufferAction::PurgeMalformed
        );
    }

    #[test]
    fn runaway_osc_sequence() {
        // Runaway unterminated OSC exceeding MAX_OSC_SEQUENCE_LENGTH.
        let mut runaway = Vec::with_capacity(MAX_OSC_SEQUENCE_LENGTH + 10);
        runaway.extend_from_slice(OSC_PREFIX);
        runaway.extend_from_slice(b"52;");
        runaway.resize(MAX_OSC_SEQUENCE_LENGTH + 1, b'x');
        assert_eq!(
            UnparsedBufferAction::determine_action(&runaway),
            UnparsedBufferAction::TripCircuitBreaker
        );
    }

    #[test]
    fn csi_final_byte_boundaries() {
        // Lower boundary: 0x40 ('@') is a valid final byte -> PurgeMalformed.
        assert_eq!(
            UnparsedBufferAction::determine_action(b"\x1b[@"),
            UnparsedBufferAction::PurgeMalformed
        );

        // Upper boundary: 0x7E ('~') is a valid final byte -> PurgeMalformed.
        assert_eq!(
            UnparsedBufferAction::determine_action(b"\x1b[~"),
            UnparsedBufferAction::PurgeMalformed
        );

        // Below boundary: 0x3F ('?') is an intermediate parameter byte, not a final byte
        // -> KeepAndAwaitMore.
        assert_eq!(
            UnparsedBufferAction::determine_action(b"\x1b[?"),
            UnparsedBufferAction::KeepAndAwaitMore
        );

        // Above boundary: 0x7F (DEL) is outside 0x40..=0x7E -> KeepAndAwaitMore.
        assert_eq!(
            UnparsedBufferAction::determine_action(b"\x1b[\x7f"),
            UnparsedBufferAction::KeepAndAwaitMore
        );
    }

    #[test]
    fn invalid_osc_syntax() {
        // Non-digit immediately following OSC prefix produces
        // OscScanResult::InvalidSyntax -> PurgeMalformed.
        assert_eq!(
            UnparsedBufferAction::determine_action(b"\x1b]invalid"),
            UnparsedBufferAction::PurgeMalformed
        );
    }

    #[test]
    fn complete_unrecognized_osc_sequence() {
        // A complete OSC sequence that could not be parsed into an input event
        // produces OscScanResult::Complete(_) -> PurgeMalformed.
        assert_eq!(
            UnparsedBufferAction::determine_action(b"\x1b]999;unsupported\x07"),
            UnparsedBufferAction::PurgeMalformed
        );
        assert_eq!(
            UnparsedBufferAction::determine_action(b"\x1b]999;unsupported\x1b\\"),
            UnparsedBufferAction::PurgeMalformed
        );
    }

    #[test]
    fn incomplete_osc_digits() {
        // OSC sequence still scanning command digits without parameter delimiter
        // produces OscScanResult::IncompleteDigits -> KeepAndAwaitMore.
        assert_eq!(
            UnparsedBufferAction::determine_action(b"\x1b]"),
            UnparsedBufferAction::KeepAndAwaitMore
        );
        assert_eq!(
            UnparsedBufferAction::determine_action(b"\x1b]1"),
            UnparsedBufferAction::KeepAndAwaitMore
        );
    }

    #[test]
    fn incomplete_ss3_sequence() {
        // Incomplete SS3 sequence (ESC O) lacking the 3rd key byte -> KeepAndAwaitMore.
        assert_eq!(
            UnparsedBufferAction::determine_action(b"\x1bO"),
            UnparsedBufferAction::KeepAndAwaitMore
        );
    }

    #[test]
    fn csi_buffer_overflow_without_final_byte() {
        // CSI sequence exceeding MAX_ESCAPE_SEQUENCE_LENGTH without reaching
        // a final byte -> PurgeMalformed via determine_csi_action.
        let mut long_csi = CSI_PREFIX.to_vec();
        long_csi.resize(MAX_ESCAPE_SEQUENCE_LENGTH, b'1');
        assert_eq!(
            UnparsedBufferAction::determine_action(&long_csi),
            UnparsedBufferAction::PurgeMalformed
        );
    }

    #[test]
    fn safety_buffer_overflow_boundaries() {
        // Exactly at MAX_ESCAPE_SEQUENCE_LENGTH (64 bytes) -> PurgeMalformed.
        let mut at_boundary = Vec::with_capacity(MAX_ESCAPE_SEQUENCE_LENGTH);
        at_boundary.extend_from_slice(b"\x1b?");
        at_boundary.resize(MAX_ESCAPE_SEQUENCE_LENGTH, b'x');
        assert_eq!(
            UnparsedBufferAction::determine_action(&at_boundary),
            UnparsedBufferAction::PurgeMalformed
        );

        // Just below MAX_ESCAPE_SEQUENCE_LENGTH (63 bytes) -> KeepAndAwaitMore.
        let mut below_boundary = Vec::with_capacity(MAX_ESCAPE_SEQUENCE_LENGTH - 1);
        below_boundary.extend_from_slice(b"\x1b?");
        below_boundary.resize(MAX_ESCAPE_SEQUENCE_LENGTH - 1, b'x');
        assert_eq!(
            UnparsedBufferAction::determine_action(&below_boundary),
            UnparsedBufferAction::KeepAndAwaitMore
        );
    }
}
