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
    /// # Syntax & Validation Rules
    ///
    /// 1. **Strict [`OSC`] Syntax Validation**: All standard [`OSC`] sequences follow the
    ///    strict syntax: `ESC ] <command_digits> ; <payload> (BEL | ST)`. If non-digit
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
    /// # Arguments
    ///
    /// - `chunk`: The raw input byte slice containing candidate [`OSC`] sequence bytes to
    ///   scan.
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

        // Phase 1 - get the scan command.
        let remaining = &chunk[OSC_PREFIX_LEN..];
        let chunk_len = chunk.len();
        let scan_command = Self::scan_command(chunk_len, remaining);

        // Phase 2 - execute the scan command.
        match scan_command {
            CommandScanResult::Concluded(result) => result,
            CommandScanResult::Payload(payload) => Self::scan_payload(chunk_len, payload),
        }
    }

    /// Phase 1: Scan decimal command digits until delimiter, terminator, or error.
    ///
    /// Iterates through characters following [`OSC_PREFIX`] (`ESC ]`) to parse the
    /// decimal command identifier. If a delimiter (`;` or `?`) is reached, returns
    /// [`CommandScanResult::Payload`] with the remaining slice for Phase 2. If an early
    /// terminator ([`ANSI_BEL`] or `ST`) is reached, returns
    /// [`CommandScanResult::Concluded`] with [`OscScanResult::Complete`].
    ///
    /// # Arguments
    ///
    /// - `chunk_len`: Total byte length of the initial chunk passed to [`Self::scan()`],
    ///   used to calculate consumed byte offsets and evaluate runaway bounds against
    ///   [`MAX_OSC_SEQUENCE_LENGTH`].
    /// - `remaining`: Byte slice positioned immediately after [`OSC_PREFIX`].
    ///
    /// [`ANSI_BEL`]: crate::ANSI_BEL
    /// [`MAX_OSC_SEQUENCE_LENGTH`]: crate::MAX_OSC_SEQUENCE_LENGTH
    /// [`OSC_PREFIX`]: crate::core::ansi::constants::OSC_PREFIX
    #[allow(clippy::match_same_arms)]
    fn scan_command(chunk_len: usize, mut remaining: &[u8]) -> CommandScanResult<'_> {
        loop {
            match remaining {
                // Advance slice cursor and continue loop.
                [ASCII_DIGIT_0..=ASCII_DIGIT_9, rest @ ..] => {
                    remaining = rest;
                }

                // Standard parameter delimiter (';'): transition to payload scan.
                [ANSI_PARAM_SEPARATOR, rest @ ..] => {
                    return CommandScanResult::Payload(rest);
                }

                // Query parameter delimiter ('?'): transition to payload scan.
                [ASCII_QUESTION_MARK, rest @ ..] => {
                    return CommandScanResult::Payload(rest);
                }

                // Terminated early by BEL (0x07) without payload.
                [ANSI_BEL, rest @ ..] => {
                    return CommandScanResult::Concluded(Self::complete(chunk_len, rest));
                }

                // Terminated early by 7-bit ST (ESC \) without payload.
                [ANSI_ESC, ANSI_ST_FINAL, rest @ ..] => {
                    return CommandScanResult::Concluded(Self::complete(chunk_len, rest));
                }

                // Lone ESC at end of buffer: wait for possible 2-byte ST (ESC \).
                [ANSI_ESC] => {
                    return CommandScanResult::Concluded(Self::IncompleteDigits);
                }

                // Buffer exhausted: break to evaluate runaway vs incomplete digits.
                [] => break,

                // Any non-digit character before delimiter: invalid syntax.
                _ => return CommandScanResult::Concluded(Self::InvalidSyntax),
            }
        }

        if chunk_len >= MAX_OSC_SEQUENCE_LENGTH {
            CommandScanResult::Concluded(Self::Runaway)
        } else {
            CommandScanResult::Concluded(Self::IncompleteDigits)
        }
    }

    /// Phase 2: Scan payload content until terminator.
    ///
    /// Iterates through bytes following the parameter delimiter (`;` or `?`) until
    /// finding [`ANSI_BEL`] or 7-bit String Terminator (`ST`, `ESC \`). Rejects embedded
    /// newlines or carriage returns as [`OscScanResult::InvalidSyntax`].
    ///
    /// # Arguments
    ///
    /// - `chunk_len`: Total byte length of the initial chunk passed to [`Self::scan()`],
    ///   used to calculate consumed byte offsets and evaluate runaway bounds against
    ///   [`MAX_OSC_SEQUENCE_LENGTH`].
    /// - `remaining`: Byte slice positioned immediately after the parameter delimiter
    ///   (the payload bytes).
    ///
    /// [`ANSI_BEL`]: crate::ANSI_BEL
    /// [`MAX_OSC_SEQUENCE_LENGTH`]: crate::MAX_OSC_SEQUENCE_LENGTH
    #[allow(clippy::match_same_arms)]
    fn scan_payload(chunk_len: usize, mut remaining: &[u8]) -> Self {
        loop {
            match remaining {
                // Terminated by BEL (0x07).
                [ANSI_BEL, rest @ ..] => return Self::complete(chunk_len, rest),

                // Terminated by 7-bit ST (ESC \).
                [ANSI_ESC, ANSI_ST_FINAL, rest @ ..] => {
                    return Self::complete(chunk_len, rest);
                }

                // Lone ESC at end of buffer: wait for possible 2-byte ST across boundary.
                [ANSI_ESC] => return Self::IncompletePayload,

                // Raw carriage return (`\r`) aborts OSC syntax.
                [CARRIAGE_RETURN, ..] => return Self::InvalidSyntax,

                // Raw line feed (`\n`) aborts OSC syntax.
                [LINE_FEED, ..] => return Self::InvalidSyntax,

                // ESC followed by non-backslash: unexpected escape aborts OSC syntax.
                [ANSI_ESC, _, ..] => return Self::InvalidSyntax,

                // Regular payload byte: advance slice cursor and continue loop.
                [_, rest @ ..] => {
                    remaining = rest;
                }

                // Buffer exhausted: break to evaluate runaway vs incomplete payload.
                [] => break,
            }
        }

        if chunk_len >= MAX_OSC_SEQUENCE_LENGTH {
            Self::Runaway
        } else {
            Self::IncompletePayload
        }
    }

    /// Helper to construct [`Self::Complete`] from the total chunk length and unconsumed
    /// bytes.
    #[inline]
    fn complete(chunk_len: usize, unconsumed_slice: &[u8]) -> Self {
        Self::Complete(consumed_offset(chunk_len, unconsumed_slice))
    }
}

/// Internal result of scanning the command identifier (Phase 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandScanResult<'a> {
    /// Delimiter (`;` or `?`) reached; contains remaining slice for payload parsing.
    Payload(&'a [u8]),

    /// Scanning concluded in Phase 1 (early terminator, incomplete digits, syntax error,
    /// or runaway).
    Concluded(OscScanResult),
}

/// Helper to calculate bytes consumed from the start of the chunk up to `unconsumed`.
#[inline]
fn consumed_offset(chunk_len: usize, unconsumed_slice: &[u8]) -> ByteOffset {
    byte_offset(chunk_len - unconsumed_slice.len())
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

    #[test]
    fn test_scan_osc_sequence_invalid_prefix() {
        assert_eq!(OscScanResult::scan(b""), OscScanResult::InvalidSyntax);
        assert_eq!(OscScanResult::scan(b"\x1b"), OscScanResult::InvalidSyntax);
        assert_eq!(OscScanResult::scan(b"\x1b["), OscScanResult::InvalidSyntax);
        assert_eq!(
            OscScanResult::scan(b"plain_text"),
            OscScanResult::InvalidSyntax
        );
    }

    #[test]
    fn test_scan_osc_sequence_question_mark_delimiter() {
        let complete_seq = format!("{OSC_START}11?rgb:00/00/00{OSC_TERMINATOR_BEL}");
        let complete_bytes = complete_seq.as_bytes();
        assert_eq!(
            OscScanResult::scan(complete_bytes),
            OscScanResult::Complete(byte_offset(complete_bytes.len()))
        );

        let incomplete_seq = format!("{OSC_START}11?");
        assert_eq!(
            OscScanResult::scan(incomplete_seq.as_bytes()),
            OscScanResult::IncompletePayload
        );
    }

    #[test]
    fn test_scan_osc_sequence_direct_terminator_without_payload() {
        // Direct BEL termination after digits (e.g., OSC 104 reset color palette).
        let direct_bel = format!("{OSC_START}104{OSC_TERMINATOR_BEL}");
        let direct_bel_bytes = direct_bel.as_bytes();
        assert_eq!(
            OscScanResult::scan(direct_bel_bytes),
            OscScanResult::Complete(byte_offset(direct_bel_bytes.len()))
        );

        // Direct ST termination after digits.
        let direct_st = [OSC_PREFIX, b"104", &[ANSI_ESC, ANSI_ST_FINAL]].concat();
        assert_eq!(
            OscScanResult::scan(&direct_st),
            OscScanResult::Complete(byte_offset(direct_st.len()))
        );
    }

    #[test]
    fn test_scan_osc_sequence_runaway_without_delimiter() {
        let mut runaway = Vec::with_capacity(MAX_OSC_SEQUENCE_LENGTH + 10);
        runaway.extend_from_slice(OSC_PREFIX);
        runaway.resize(MAX_OSC_SEQUENCE_LENGTH, b'1');
        assert_eq!(OscScanResult::scan(&runaway), OscScanResult::Runaway);
    }

    #[test]
    fn test_scan_osc_sequence_consumed_offset_with_trailing_bytes() {
        let buffer =
            format!("{OSC_START}11;rgb:00/00/00{OSC_TERMINATOR_BEL}trailing_keystrokes");
        let buffer_bytes = buffer.as_bytes();
        let expected_consumed = buffer.len() - "trailing_keystrokes".len();
        assert_eq!(
            OscScanResult::scan(buffer_bytes),
            OscScanResult::Complete(byte_offset(expected_consumed))
        );
    }
}
