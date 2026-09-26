// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Lexical scanning and parameter extraction from [`CSI`] sequences.
//!
//! [`CSI`]: crate::CsiSequence

use crate::{ByteOffset, WideningCastToU16, byte_offset,
            core::ansi::constants::{ANSI_FUNCTION_KEY_TERMINATOR, ANSI_PARAM_SEPARATOR,
                                    ASCII_DIGIT_0, ASCII_DIGIT_9, ASCII_LOWER_A,
                                    ASCII_LOWER_Z, ASCII_UPPER_A, ASCII_UPPER_Z,
                                    CSI_PREFIX_LEN}};

/// Lexical token of a byte scanned inside a [`CSI`] parameter sequence.
///
/// [`CSI`]: crate::CsiSequence
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CsiByteToken {
    /// Decimal digit with its numeric value `0..=9`.
    Digit(u8),
    /// Parameter separator `;` ([`ANSI_PARAM_SEPARATOR`]).
    Separator,
    /// Terminating character (`~` or [`ASCII`] letter).
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    Terminator(u8),
    /// Any byte that is invalid in a numeric [`CSI`] sequence.
    ///
    /// [`CSI`]: crate::CsiSequence
    Invalid,
}

/// Classifies a raw byte in a [`CSI`] parameter sequence into a [`CsiByteToken`].
///
/// [`CSI`]: crate::CsiSequence
#[must_use]
pub fn classify_csi_byte(byte: u8) -> CsiByteToken {
    // IMPORTANT: We use if/else chains instead of match arms because Rust treats
    // constants in match patterns as variable bindings, not value comparisons.
    // This is a Rust language limitation documented in RFC 1445.
    //
    // Using named constants in match arms like:
    //   ASCII_DIGIT_0..=ASCII_DIGIT_9 => { ... }
    // would create new bindings named ASCII_DIGIT_0 and ASCII_DIGIT_9 instead of
    // matching against the constant values. The if/else chain correctly compares
    // against the constant values.
    if (ASCII_DIGIT_0..=ASCII_DIGIT_9).contains(&byte) {
        CsiByteToken::Digit(byte - ASCII_DIGIT_0)
    } else if byte == ANSI_PARAM_SEPARATOR {
        CsiByteToken::Separator
    } else if byte == ANSI_FUNCTION_KEY_TERMINATOR
        || (ASCII_UPPER_A..=ASCII_UPPER_Z).contains(&byte)
        || (ASCII_LOWER_A..=ASCII_LOWER_Z).contains(&byte)
    {
        CsiByteToken::Terminator(byte)
    } else {
        CsiByteToken::Invalid
    }
}

/// Result of extracting parameters and command terminator from a [`CSI`] byte slice.
///
/// [`CSI`]: crate::CsiSequence
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedCsiParams {
    /// Parsed numeric arguments (e.g. `[1, 2]` from `ESC [ 1 ; 2 H`).
    pub params: Vec<u16>,
    /// Command terminator character (e.g. `b'H'`, `b'~'`).
    pub final_byte: u8,
    /// Scanner cursor displacement across the parameter body (from after `ESC [`
    /// through `final_byte`).
    pub bytes_scanned: ByteOffset,
}

impl ExtractedCsiParams {
    /// Total bytes consumed from the buffer including the `ESC [` prefix
    /// ([`CSI_PREFIX_LEN`]).
    ///
    /// [`CSI_PREFIX_LEN`]: crate::CSI_PREFIX_LEN
    #[must_use]
    pub fn total_consumed(&self) -> ByteOffset {
        byte_offset(CSI_PREFIX_LEN) + self.bytes_scanned
    }
}

/// Extracts numeric parameters, final byte, and scanned byte count from a [`CSI`]
/// buffer.
///
/// The returned [`ExtractedCsiParams`] contains the parsed parameters, terminator
/// byte, and scanner cursor displacement across the parameter body (from after
/// `ESC [` through the final byte).
///
/// [`CSI`]: crate::CsiSequence
#[must_use]
pub fn extract_csi_params(buffer: &[u8]) -> Option<ExtractedCsiParams> {
    const DECIMAL_RADIX: u16 = 10;

    let mut params = Vec::new();
    let mut acc_numeric_param: u16 = 0;
    let mut final_byte: Option<u8> = None;
    let mut bytes_scanned = byte_offset(0);

    for &byte in &buffer[CSI_PREFIX_LEN..] {
        bytes_scanned += byte_offset(1);

        match classify_csi_byte(byte) {
            CsiByteToken::Digit(digit) => {
                acc_numeric_param = acc_numeric_param
                    .saturating_mul(DECIMAL_RADIX)
                    .saturating_add(digit.as_u16_widening());
            }
            CsiByteToken::Separator => {
                params.push(acc_numeric_param);
                acc_numeric_param = 0;
            }
            CsiByteToken::Terminator(terminator) => {
                params.push(acc_numeric_param);
                final_byte = Some(terminator);
                break;
            }
            CsiByteToken::Invalid => return None,
        }
    }

    let final_byte = final_byte?;

    Some(ExtractedCsiParams {
        params,
        final_byte,
        bytes_scanned,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_csi_params() {
        // Multi-parameter sequence: ESC [ 1 ; 2 H
        let buffer = b"\x1b[1;2H";
        let extracted = extract_csi_params(buffer).expect("Should extract CSI params");
        assert_eq!(extracted.params, vec![1, 2]);
        assert_eq!(extracted.final_byte, b'H');
        assert_eq!(extracted.bytes_scanned, byte_offset(4));
        assert_eq!(extracted.total_consumed(), byte_offset(6));

        // Single parameter sequence: ESC [ 5 ~
        let buffer_tilde = b"\x1b[5~";
        let extracted_tilde =
            extract_csi_params(buffer_tilde).expect("Should extract CSI params");
        assert_eq!(extracted_tilde.params, vec![5]);
        assert_eq!(extracted_tilde.final_byte, b'~');
        assert_eq!(extracted_tilde.bytes_scanned, byte_offset(2));
        assert_eq!(extracted_tilde.total_consumed(), byte_offset(4));

        // Invalid byte in parameter body
        let buffer_invalid = b"\x1b[1;?H";
        assert!(extract_csi_params(buffer_invalid).is_none());
    }
}
