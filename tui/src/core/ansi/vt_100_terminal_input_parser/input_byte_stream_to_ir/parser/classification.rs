// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Classification of unparsed bytes in [`InputByteStreamToIrParser`].
//!
//! [`InputByteStreamToIrParser`]: super::InputByteStreamToIrParser

use super::constants::MAX_ESCAPE_SEQUENCE_LENGTH;
use crate::{ANSI_CSI_BRACKET, ANSI_ESC, ANSI_OSC_CLOSE_BRACKET, ANSI_SS3_O,
            CSI_FINAL_BYTE_MAX, CSI_FINAL_BYTE_MIN,
            core::ansi::vt_100_terminal_input_parser::{OscScanResult, scan_osc_sequence}};

/// Classification of unparsed bytes accumulated in [`InputByteStreamToIrParser`].
///
/// When [`try_parse_input_event()`] returns `None`, this enum categorizes why the
/// bytes could not be parsed and guides the parser's disposition.
///
/// [`InputByteStreamToIrParser`]: super::InputByteStreamToIrParser
/// [`try_parse_input_event()`]:
///     crate::core::ansi::vt_100_terminal_input_parser::try_parse_input_event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnparsedBufferClassification {
    /// Incomplete sequence awaiting additional bytes from subsequent reads.
    ///
    /// The parser leaves the accumulator intact and waits for the next read from
    /// [`stdin`].
    ///
    /// [`stdin`]: std::io::stdin
    Incomplete,

    /// Completed but malformed, unsupported, or overflowing non-[`OSC`] sequence (e.g.,
    /// invalid [`CSI`] final byte, unrecognized [`SS3`] sequence, or >64 bytes of
    /// escape syntax).
    ///
    /// The parser purges the sequence to prevent stream lockup.
    ///
    /// [`CSI`]: crate::CsiSequence
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [`SS3`]: https://en.wikipedia.org/wiki/ANSI_escape_code#SS3
    MalformedSequence,

    /// Oversized [`OSC`] sequence exceeding [`MAX_OSC_SEQUENCE_LENGTH`] (1 MiB) without a
    /// terminator.
    ///
    /// The parser trips the circuit breaker: purges the 1 MiB accumulator and enters
    /// streaming drain mode to swallow the remainder without allocations.
    ///
    /// [`MAX_OSC_SEQUENCE_LENGTH`]: crate::MAX_OSC_SEQUENCE_LENGTH
    /// [`OSC`]: crate::osc_codes::OscSequence
    RunawayOsc,
}

impl UnparsedBufferClassification {
    /// Inspects and classifies an unparsed byte slice into [`Self::Incomplete`],
    /// [`Self::MalformedSequence`], or [`Self::RunawayOsc`].
    ///
    /// Performs a single-pass classification across 4 criteria:
    /// 1. **[`OSC`] sequence**: Scans for runaway (> 1 MiB) or incomplete state via
    ///    [`scan_osc_sequence()`].
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
    /// [`scan_osc_sequence()`]: crate::core::ansi::vt_100_terminal_input_parser::scan_osc_sequence
    /// [`SS3_SEQ_LEN`]: crate::SS3_SEQ_LEN
    /// [`SS3`]: https://en.wikipedia.org/wiki/ANSI_escape_code#SS3
    #[must_use]
    pub fn classify(chunk: &[u8]) -> Self {
        match chunk {
            // 1. OSC sequence check (1 MiB threshold).
            [ANSI_ESC, ANSI_OSC_CLOSE_BRACKET, ..] => match scan_osc_sequence(chunk) {
                OscScanResult::Runaway => Self::RunawayOsc,
                OscScanResult::IncompleteDigits | OscScanResult::IncompletePayload => {
                    Self::Incomplete
                }
                OscScanResult::Complete(_) | OscScanResult::InvalidSyntax => {
                    Self::MalformedSequence
                }
            },

            // 2. Completed CSI sequence that could not be parsed (ESC [ ... followed
            // by a final byte in 0x40..=0x7E).
            [ANSI_ESC, ANSI_CSI_BRACKET, rest @ ..]
                if Self::contains_csi_final_byte(rest) =>
            {
                Self::MalformedSequence
            }

            // 3. Completed SS3 sequence that could not be parsed (ESC O <key> reaching
            // SS3_SEQ_LEN = 3 bytes).
            [ANSI_ESC, ANSI_SS3_O, _, ..] => Self::MalformedSequence,

            // 4. Safety fallback for non-OSC sequences exceeding 64 bytes.
            _ if chunk.len() >= MAX_ESCAPE_SEQUENCE_LENGTH => Self::MalformedSequence,

            // 5. Incomplete sequence waiting for subsequent bytes.
            _ => Self::Incomplete,
        }
    }

    /// Returns `true` if `bytes` contains an ECMA-48 / [`CSI`] sequence terminating
    /// final byte in the range `0x40..=0x7E` ([`CSI_FINAL_BYTE_MIN`] through
    /// [`CSI_FINAL_BYTE_MAX`]).
    ///
    /// [`CSI_FINAL_BYTE_MAX`]: crate::CSI_FINAL_BYTE_MAX
    /// [`CSI_FINAL_BYTE_MIN`]: crate::CSI_FINAL_BYTE_MIN
    /// [`CSI`]: crate::CsiSequence
    #[must_use]
    fn contains_csi_final_byte(bytes: &[u8]) -> bool {
        const fn is_csi_final_byte(byte: u8) -> bool {
            (byte >= CSI_FINAL_BYTE_MIN) && (byte <= CSI_FINAL_BYTE_MAX)
        }

        bytes.iter().copied().any(is_csi_final_byte)
    }
}

#[cfg(test)]
mod tests_classify_unparsed_buffer {
    use super::*;
    use crate::{CSI_PREFIX, MAX_OSC_SEQUENCE_LENGTH, OSC_PREFIX};

    #[test]
    fn incomplete_sequences() {
        // Empty buffer.
        assert_eq!(
            UnparsedBufferClassification::classify(b""),
            UnparsedBufferClassification::Incomplete
        );

        // Incomplete CSI sequence.
        assert_eq!(
            UnparsedBufferClassification::classify(CSI_PREFIX),
            UnparsedBufferClassification::Incomplete
        );

        // Incomplete OSC sequence (< 1 MiB).
        assert_eq!(
            UnparsedBufferClassification::classify(b"\x1b]11;rgb"),
            UnparsedBufferClassification::Incomplete
        );
    }

    #[test]
    fn malformed_csi_sequence() {
        // Completed CSI sequence with an unknown final byte (e.g. 'z').
        assert_eq!(
            UnparsedBufferClassification::classify(b"\x1b[999z"),
            UnparsedBufferClassification::MalformedSequence
        );
    }

    #[test]
    fn malformed_ss3_sequence() {
        // Completed SS3 sequence with an unrecognized key byte.
        assert_eq!(
            UnparsedBufferClassification::classify(b"\x1bO?"),
            UnparsedBufferClassification::MalformedSequence
        );
    }

    #[test]
    fn safety_buffer_overflow() {
        // Non-OSC sequence exceeding 64 bytes without completing.
        let mut overflow = Vec::with_capacity(65);
        overflow.extend_from_slice(b"\x1b?");
        overflow.resize(65, b'x');
        assert_eq!(
            UnparsedBufferClassification::classify(&overflow),
            UnparsedBufferClassification::MalformedSequence
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
            UnparsedBufferClassification::classify(&runaway),
            UnparsedBufferClassification::RunawayOsc
        );
    }

    #[test]
    fn csi_final_byte_boundaries() {
        // Lower boundary: 0x40 ('@') is a valid final byte -> MalformedSequence.
        assert_eq!(
            UnparsedBufferClassification::classify(b"\x1b[@"),
            UnparsedBufferClassification::MalformedSequence
        );

        // Upper boundary: 0x7E ('~') is a valid final byte -> MalformedSequence.
        assert_eq!(
            UnparsedBufferClassification::classify(b"\x1b[~"),
            UnparsedBufferClassification::MalformedSequence
        );

        // Below boundary: 0x3F ('?') is an intermediate parameter byte, not a final byte
        // -> Incomplete.
        assert_eq!(
            UnparsedBufferClassification::classify(b"\x1b[?"),
            UnparsedBufferClassification::Incomplete
        );

        // Above boundary: 0x7F (DEL) is outside 0x40..=0x7E -> Incomplete.
        assert_eq!(
            UnparsedBufferClassification::classify(b"\x1b[\x7f"),
            UnparsedBufferClassification::Incomplete
        );
    }

    #[test]
    fn invalid_osc_syntax() {
        // Non-digit immediately following OSC prefix produces
        // OscScanResult::InvalidSyntax -> MalformedSequence.
        assert_eq!(
            UnparsedBufferClassification::classify(b"\x1b]invalid"),
            UnparsedBufferClassification::MalformedSequence
        );
    }
}
