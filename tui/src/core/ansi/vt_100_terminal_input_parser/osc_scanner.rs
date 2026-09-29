// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Lexical scanning of Operating System Command ([`OSC`]) sequences.
//!
//! [`OSC`]: crate::osc_codes::OscSequence

use crate::{ByteOffset, byte_offset,
            core::ansi::constants::{ANSI_BEL, ANSI_ESC, ANSI_PARAM_SEPARATOR,
                                    ANSI_ST_FINAL, ASCII_DIGIT_0, ASCII_DIGIT_9,
                                    ASCII_QUESTION_MARK, CARRIAGE_RETURN, LINE_FEED,
                                    MAX_OSC_SEQUENCE_LENGTH, OSC_PREFIX, OSC_PREFIX_LEN}};

/// Result of scanning an input buffer for an Operating System Command ([`OSC`])
/// sequence.
///
/// Returned by [`Self::scan()`] to guide the parser in distinguishing between human
/// keystrokes (`Alt+]`), terminal emulator query responses, and unparsed byte
/// classifications.
///
/// [`OSC`]: crate::osc_codes::OscSequence
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OscScanResult {
    /// Found a complete, well-formed [`OSC`] sequence starting with `ESC ]` and
    /// terminating with either [`ANSI_BEL`] (`0x07`) or 7-bit
    /// [`ANSI_ST_7BIT_TRANSPORT_ENCODING`] (`0x1B 0x5C`).
    ///
    /// The associated [`ByteOffset`] indicates the total number of bytes consumed
    /// through the terminator (prefix + payload + terminator).
    ///
    /// [`ANSI_BEL`]: crate::ANSI_BEL
    /// [`ANSI_ST_7BIT_TRANSPORT_ENCODING`]: crate::ANSI_ST_7BIT_TRANSPORT_ENCODING
    /// [`ByteOffset`]: crate::ByteOffset
    /// [`OSC`]: crate::osc_codes::OscSequence
    Complete(ByteOffset),

    /// The sequence begins with `ESC ]` and follows valid [`OSC`] command syntax, but
    /// is still scanning decimal command digits (no `;` or `?` delimiter
    /// arrived yet). If more input is anticipated
    /// ([`MaybeMore::KernelMayHaveMore`]), the parser waits. If the stream
    /// has drained ([`MaybeMore::KernelDrained`]), this indicates human
    /// typing (e.g., `Alt+]` followed by digits), and the parser falls back
    /// to `Alt+]`.
    ///
    /// [`MaybeMore::KernelDrained`]: super::maybe_more::MaybeMore::KernelDrained
    /// [`MaybeMore::KernelMayHaveMore`]: super::maybe_more::MaybeMore::KernelMayHaveMore
    /// [`OSC`]: crate::osc_codes::OscSequence
    IncompleteDigits,

    /// The sequence has seen the parameter delimiter (`;` or `?`) and is scanning
    /// payload content. This is guaranteed to be an in-flight [`OSC`]
    /// sequence. The parser always waits for the remaining payload across
    /// read boundaries (bounded by [`MAX_OSC_SEQUENCE_LENGTH`]).
    ///
    /// [`MAX_OSC_SEQUENCE_LENGTH`]: crate::MAX_OSC_SEQUENCE_LENGTH
    /// [`OSC`]: crate::osc_codes::OscSequence
    IncompletePayload,

    /// The sequence begins with `ESC ]`, but violates [`OSC`] command syntax (such as
    /// non-digit characters before the parameter delimiter `;`, or embedded
    /// newline/carriage return characters). This cannot be a valid [`OSC`] sequence;
    /// the parser immediately rejects it and falls back to emitting `Alt+]`.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    InvalidSyntax,

    /// The candidate [`OSC`] sequence exceeded [`MAX_OSC_SEQUENCE_LENGTH`] without
    /// encountering a terminator. The buffer is purged to prevent unbounded memory
    /// growth and input freezes.
    ///
    /// [`MAX_OSC_SEQUENCE_LENGTH`]: crate::MAX_OSC_SEQUENCE_LENGTH
    /// [`OSC`]: crate::osc_codes::OscSequence
    Runaway,
}

impl OscScanResult {
    /// Lexical scanner that parses a byte buffer for an Operating System Command
    /// ([`OSC`]) sequence.
    ///
    /// Terminal query responses (e.g., background color [`OSC`] 11, clipboard [`OSC`] 52)
    /// start with `ESC ]` (`0x1B 0x5D`), have a numeric command identifier, parameters,
    /// and terminate with either [`ANSI_BEL`] (`0x07`) or 7-bit
    /// [`ANSI_ST_7BIT_TRANSPORT_ENCODING`] (`0x1B 0x5C`).
    ///
    /// # Grammar & Validation Rules
    ///
    /// 1. **Strict [`OSC`] Syntax Validation**: All standard [`OSC`] sequences follow the
    ///    strict grammar: `ESC ] <command_digits> ; <payload> (BEL | ST)`. If non-digit
    ///    characters appear before the parameter delimiter `;`, or if raw carriage
    ///    returns (`\r`) or newlines (`\n`) are encountered, the scanner immediately
    ///    halts and returns [`OscScanResult::InvalidSyntax`]. This prevents human
    ///    keystrokes like `Alt+] 5 a` from being falsely treated as candidate [`OSC`].
    ///
    /// 2. **[`UTF-8`] Safety Note**: ECMA-48 specifies `0x9C` as an 8-bit String
    ///    Terminator (`ST`). However, in [`UTF-8`], `0x9C` is a common continuation byte
    ///    (`0b1001_1100`, used in characters like `£`, `œ`, and `✓`). Modern terminal
    ///    emulators in [`UTF-8`] mode exclusively send 7-bit
    ///    [`ANSI_ST_7BIT_TRANSPORT_ENCODING`] (`0x1B 0x5C`) or [`ANSI_BEL`] (`0x07`).
    ///    This scanner intentionally does not match `0x9C` to prevent truncating [`OSC`]
    ///    payloads containing valid [`UTF-8`] text. See
    ///    [`ANSI_ST_7BIT_TRANSPORT_ENCODING`] for full historical and transport encoding
    ///    details.
    ///
    /// [`ANSI_BEL`]: crate::ANSI_BEL
    /// [`ANSI_ST_7BIT_TRANSPORT_ENCODING`]: crate::ANSI_ST_7BIT_TRANSPORT_ENCODING
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
    #[must_use]
    pub fn scan(chunk: &[u8]) -> Self {
        if !chunk.starts_with(OSC_PREFIX) {
            return Self::InvalidSyntax;
        }

        let mut remaining = &chunk[OSC_PREFIX_LEN..];

        // Phase 1: Scan decimal command digits until delimiter or terminator.
        let mut delimiter_found = false;
        while !remaining.is_empty() {
            match remaining {
                [ASCII_DIGIT_0..=ASCII_DIGIT_9, rest @ ..] => {
                    // Advance slice cursor and continue while loop.
                    remaining = rest;
                }
                [ANSI_PARAM_SEPARATOR | ASCII_QUESTION_MARK, rest @ ..] => {
                    remaining = rest;
                    delimiter_found = true;
                    break;
                }
                [ANSI_BEL, rest @ ..] | [ANSI_ESC, ANSI_ST_FINAL, rest @ ..] => {
                    return Self::Complete(consumed_offset(chunk, rest));
                }
                [ANSI_ESC] => return Self::IncompleteDigits,
                _ => return Self::InvalidSyntax,
            }
        }

        // Buffer ended before delimiter arrived (e.g. lone `ESC ]` or `ESC ] 1 1`).
        if !delimiter_found {
            return if chunk.len() >= MAX_OSC_SEQUENCE_LENGTH {
                Self::Runaway
            } else {
                Self::IncompleteDigits
            };
        }

        // Phase 2: Scan payload content until terminator.
        while !remaining.is_empty() {
            match remaining {
                [ANSI_BEL, rest @ ..] | [ANSI_ESC, ANSI_ST_FINAL, rest @ ..] => {
                    return Self::Complete(consumed_offset(chunk, rest));
                }
                [ANSI_ESC] => return Self::IncompletePayload,
                [CARRIAGE_RETURN | LINE_FEED, ..] | [ANSI_ESC, _, ..] => {
                    return Self::InvalidSyntax;
                }
                [_, rest @ ..] => {
                    // Advance slice cursor and continue while loop.
                    remaining = rest;
                }
                [] => break,
            }
        }

        if chunk.len() >= MAX_OSC_SEQUENCE_LENGTH {
            Self::Runaway
        } else {
            Self::IncompletePayload
        }
    }
}

/// Helper to calculate bytes consumed from the start of `chunk` up to `unconsumed`.
#[inline]
fn consumed_offset(chunk: &[u8], unconsumed: &[u8]) -> ByteOffset {
    byte_offset(chunk.len() - unconsumed.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ansi::constants::{CLIPBOARD_TARGET_CLIPBOARD, OSC_CODE_CLIPBOARD},
                      osc::osc_codes::{OSC_DELIMITER, OSC_START, OSC_TERMINATOR_BEL}};

    #[test]
    fn test_scan_osc_sequence_bel() {
        let buffer = format!("{OSC_START}11;rgb:00/00/00{OSC_TERMINATOR_BEL}");
        let buffer = buffer.as_bytes();
        assert_eq!(
            OscScanResult::scan(buffer),
            OscScanResult::Complete(byte_offset(buffer.len()))
        );
    }

    #[test]
    fn test_scan_osc_sequence_st() {
        let buffer =
            [OSC_PREFIX, b"11;rgb:00/00/00", &[ANSI_ESC, ANSI_ST_FINAL]].concat();
        assert_eq!(
            OscScanResult::scan(&buffer),
            OscScanResult::Complete(byte_offset(buffer.len()))
        );
    }

    #[test]
    fn test_scan_osc_sequence_incomplete_digits() {
        let seq = format!("{OSC_START}12");
        assert_eq!(
            OscScanResult::scan(seq.as_bytes()),
            OscScanResult::IncompleteDigits
        );
        assert_eq!(
            OscScanResult::scan(OSC_PREFIX),
            OscScanResult::IncompleteDigits
        );
    }

    #[test]
    fn test_scan_osc_sequence_incomplete_payload() {
        let seq_data = format!("{OSC_START}12{OSC_DELIMITER}data");
        assert_eq!(
            OscScanResult::scan(seq_data.as_bytes()),
            OscScanResult::IncompletePayload
        );
        let seq_empty = format!("{OSC_START}12{OSC_DELIMITER}");
        assert_eq!(
            OscScanResult::scan(seq_empty.as_bytes()),
            OscScanResult::IncompletePayload
        );
    }

    #[test]
    fn test_scan_osc_sequence_partial_st_payload() {
        let seq = [OSC_PREFIX, b"12;data", &[ANSI_ESC]].concat();
        assert_eq!(OscScanResult::scan(&seq), OscScanResult::IncompletePayload);
    }

    #[test]
    fn test_scan_osc_sequence_partial_st_digits() {
        let seq = [OSC_PREFIX, b"0", &[ANSI_ESC]].concat();
        assert_eq!(OscScanResult::scan(&seq), OscScanResult::IncompleteDigits);
    }

    #[test]
    fn test_scan_osc_sequence_invalid_syntax_letters_in_digits() {
        let seq = format!("{OSC_START}12a{OSC_DELIMITER}");
        assert_eq!(
            OscScanResult::scan(seq.as_bytes()),
            OscScanResult::InvalidSyntax
        );
    }

    #[test]
    fn test_scan_osc_sequence_invalid_syntax_embedded_newline() {
        let seq_lf = [OSC_PREFIX, b"12;data", &[LINE_FEED, ANSI_BEL]].concat();
        assert_eq!(OscScanResult::scan(&seq_lf), OscScanResult::InvalidSyntax);
        let seq_cr = [OSC_PREFIX, b"12;data", &[CARRIAGE_RETURN, ANSI_BEL]].concat();
        assert_eq!(OscScanResult::scan(&seq_cr), OscScanResult::InvalidSyntax);
    }

    #[test]
    fn test_scan_osc_sequence_invalid_syntax_unexpected_esc_in_payload() {
        let seq = [OSC_PREFIX, b"12;data", &[ANSI_ESC], b"x"].concat();
        assert_eq!(OscScanResult::scan(&seq), OscScanResult::InvalidSyntax);
    }

    #[test]
    fn test_scan_osc_sequence_utf8_continuation_byte_0x9c() {
        let target_char = char::from(CLIPBOARD_TARGET_CLIPBOARD);
        let buffer_incomplete = format!(
            "{OSC_START}{OSC_CODE_CLIPBOARD}{OSC_DELIMITER}{target_char}{OSC_DELIMITER}\u{2713}"
        );
        let buffer_incomplete = buffer_incomplete.as_bytes();
        assert_eq!(
            OscScanResult::scan(buffer_incomplete),
            OscScanResult::IncompletePayload
        );

        let buffer_complete = format!(
            "{OSC_START}{OSC_CODE_CLIPBOARD}{OSC_DELIMITER}{target_char}{OSC_DELIMITER}\u{2713}{OSC_TERMINATOR_BEL}"
        );
        let buffer_complete = buffer_complete.as_bytes();
        assert_eq!(
            OscScanResult::scan(buffer_complete),
            OscScanResult::Complete(byte_offset(buffer_complete.len()))
        );
    }

    #[test]
    fn test_scan_osc_sequence_runaway() {
        let mut runaway = Vec::with_capacity(MAX_OSC_SEQUENCE_LENGTH + 10);
        let prefix = format!("{OSC_START}{OSC_CODE_CLIPBOARD}{OSC_DELIMITER}");
        runaway.extend_from_slice(prefix.as_bytes());
        runaway.resize(MAX_OSC_SEQUENCE_LENGTH + 1, b'a');
        assert_eq!(OscScanResult::scan(&runaway), OscScanResult::Runaway);
    }
}
