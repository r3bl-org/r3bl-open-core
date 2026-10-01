// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Lexical scanning and parameter extraction from [`CSI`] sequences.
//!
//! [`CSI`]: crate::CsiSequence

use crate::{ByteOffset, WideningCastToU16, WideningCastToU32, byte_offset,
            core::ansi::constants::{ANSI_CSI_BRACKET, ANSI_ESC,
                                    ANSI_FUNCTION_KEY_TERMINATOR, ANSI_PARAM_SEPARATOR,
                                    ASCII_DIGIT_0, ASCII_DIGIT_9, ASCII_LOWER_A,
                                    ASCII_LOWER_Z, ASCII_UPPER_A, ASCII_UPPER_Z,
                                    CSI_PREFIX_LEN}};

/// If `chunk` starts with `ESC [` ([`ANSI_ESC`], [`ANSI_CSI_BRACKET`]) followed by an
/// [`ASCII`] digit (`'0'`..=`'9'`), strips the `ESC [` prefix and returns the payload
/// slice starting at the first digit.
///
/// Returns `None` if `chunk` is too short, does not begin with `ESC [`, or the byte
/// following `ESC [` is not an [`ASCII`] digit.
///
/// [`ANSI_CSI_BRACKET`]: crate::ANSI_CSI_BRACKET
/// [`ANSI_ESC`]: crate::ANSI_ESC
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
#[must_use]
pub fn strip_csi_numeric_prefix(chunk: &[u8]) -> Option<&[u8]> {
    match *chunk {
        [ANSI_ESC, ANSI_CSI_BRACKET, first_byte, ..]
            if (ASCII_DIGIT_0..=ASCII_DIGIT_9).contains(&first_byte) =>
        {
            chunk.get(CSI_PREFIX_LEN..)
        }
        _ => None,
    }
}

/// Parses a text-formatted number in a byte slice into an integer ([`u32`]).
///
/// This is a zero-allocation, byte-level equivalent of [`str::parse::<u32>()`] (or C's
/// [`atoi`]).
///
/// # Context: Numbers as Text in Terminal Streams
///
/// The terminal transmits escape sequences as human-readable [`ASCII`] text over the
/// wire. For example, in the [`Kitty`] sequence `ESC [ 9 1 ; 3 u`, the number `91` is
/// sent as the text characters `'9'` and `'1'`.
///
/// Rather than allocating or converting the slice into a `&str` (which requires a
/// [`UTF-8`] validation pass), this function parses the text representation of the number
/// directly from the raw byte stream:
///
/// ```text
/// Text string in stream:     "91"
///                             │└───┐
///                             ▼    ▼
/// ASCII character bytes:    ['9', '1']  (decimal byte values: [57, 49])
///
/// Resulting u32 integer:      91
/// ```
///
/// `b"91"` is simply string-syntax shorthand for the byte slice `&[b'9', b'1']`.
///
/// # Arguments
///
/// - `digit_bytes`: Slice containing the text-formatted number, e.g. `&[b'9', b'1']`
///
/// # Returns
///
/// - `Some(u32)`: If the text contains only valid [`ASCII`] digits (`'0'`..=`'9'`).
/// - `None`: If `digit_bytes` is empty or contains non-digit text (e.g. `;`, `:`, or
///   letters).
///
/// # Examples
///
/// ```rust
/// use r3bl_tui::vt_100_terminal_input_parser::parse_decimal_digits;
/// // Text "91" parses to integer 91:
/// assert_eq!(parse_decimal_digits(b"91"), Some(91));
///
/// // Text "57366" (Home key) parses to integer 57366:
/// assert_eq!(parse_decimal_digits(b"57366"), Some(57366));
///
/// // Empty text slice returns None:
/// assert_eq!(parse_decimal_digits(b""), None);
///
/// // Non-digit characters (e.g. ';' in "12;3") return None:
/// assert_eq!(parse_decimal_digits(b"12;3"), None);
/// ```
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`atoi`]: https://man7.org/linux/man-pages/man3/atoi.3.html
/// [`Kitty`]: https://sw.kovidgoyal.net/kitty/
/// [`str::parse::<u32>()`]: str::parse
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
#[must_use]
pub fn parse_decimal_digits(digit_bytes: &[u8]) -> Option<u32> {
    const DECIMAL_RADIX: u32 = 10;

    if digit_bytes.is_empty() {
        return None;
    }
    let mut accumulated_value: u32 = 0;
    for byte in digit_bytes.iter().copied() {
        if !(ASCII_DIGIT_0..=ASCII_DIGIT_9).contains(&byte) {
            return None;
        }
        let digit = (byte - ASCII_DIGIT_0).as_u32_widening();
        accumulated_value = accumulated_value
            .saturating_mul(DECIMAL_RADIX)
            .saturating_add(digit);
    }
    Some(accumulated_value)
}

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

    let [ANSI_ESC, ANSI_CSI_BRACKET, ..] = *buffer else {
        return None;
    };
    let payload = buffer.get(CSI_PREFIX_LEN..)?;

    let mut params = Vec::new();
    let mut acc_numeric_param: u16 = 0;
    let mut final_byte: Option<u8> = None;
    let mut bytes_scanned = byte_offset(0);

    for byte in payload.iter().copied() {
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
    fn test_classify_csi_byte() {
        // Digits.
        assert_eq!(classify_csi_byte(b'0'), CsiByteToken::Digit(0));
        assert_eq!(classify_csi_byte(b'9'), CsiByteToken::Digit(9));

        // Separator.
        assert_eq!(classify_csi_byte(b';'), CsiByteToken::Separator);

        // Terminators.
        assert_eq!(classify_csi_byte(b'~'), CsiByteToken::Terminator(b'~'));
        assert_eq!(classify_csi_byte(b'A'), CsiByteToken::Terminator(b'A'));
        assert_eq!(classify_csi_byte(b'Z'), CsiByteToken::Terminator(b'Z'));
        assert_eq!(classify_csi_byte(b'a'), CsiByteToken::Terminator(b'a'));
        assert_eq!(classify_csi_byte(b'u'), CsiByteToken::Terminator(b'u'));
        assert_eq!(classify_csi_byte(b'z'), CsiByteToken::Terminator(b'z'));

        // Invalid ASCII boundaries and control characters.
        assert_eq!(classify_csi_byte(b'@'), CsiByteToken::Invalid); // Before 'A'.
        assert_eq!(classify_csi_byte(b'['), CsiByteToken::Invalid); // After 'Z'.
        assert_eq!(classify_csi_byte(b'`'), CsiByteToken::Invalid); // Before 'a'.
        assert_eq!(classify_csi_byte(b'{'), CsiByteToken::Invalid); // After 'z'.
        assert_eq!(classify_csi_byte(b'?'), CsiByteToken::Invalid);
        assert_eq!(classify_csi_byte(b' '), CsiByteToken::Invalid);
    }

    #[test]
    fn test_extract_csi_params() {
        // Multi-parameter sequence: ESC [ 1 ; 2 H.
        let buffer = b"\x1b[1;2H";
        let extracted = extract_csi_params(buffer).expect("Should extract CSI params");
        assert_eq!(extracted.params, vec![1, 2]);
        assert_eq!(extracted.final_byte, b'H');
        assert_eq!(extracted.bytes_scanned, byte_offset(4));
        assert_eq!(extracted.total_consumed(), byte_offset(6));

        // Single parameter sequence: ESC [ 5 ~.
        let buffer_tilde = b"\x1b[5~";
        let extracted_tilde =
            extract_csi_params(buffer_tilde).expect("Should extract CSI params");
        assert_eq!(extracted_tilde.params, vec![5]);
        assert_eq!(extracted_tilde.final_byte, b'~');
        assert_eq!(extracted_tilde.bytes_scanned, byte_offset(2));
        assert_eq!(extracted_tilde.total_consumed(), byte_offset(4));

        // Lowercase terminator (e.g. Kitty CSI u: ESC [ 91 ; 3 u).
        let buffer_kitty = b"\x1b[91;3u";
        let extracted_kitty =
            extract_csi_params(buffer_kitty).expect("Should extract CSI u params");
        assert_eq!(extracted_kitty.params, vec![91, 3]);
        assert_eq!(extracted_kitty.final_byte, b'u');
        assert_eq!(extracted_kitty.bytes_scanned, byte_offset(5));
        assert_eq!(extracted_kitty.total_consumed(), byte_offset(7));

        // Invalid byte in parameter body.
        let buffer_invalid = b"\x1b[1;?H";
        assert!(extract_csi_params(buffer_invalid).is_none());

        // Missing ESC [ prefix.
        assert!(extract_csi_params(b"1;2H").is_none());
        assert!(extract_csi_params(b"\x1b").is_none());
        assert!(extract_csi_params(b"").is_none());

        // Truncated buffer / missing terminator.
        assert!(extract_csi_params(b"\x1b[1;2").is_none());
        assert!(extract_csi_params(b"\x1b[").is_none());
    }

    #[test]
    fn test_parse_decimal_digits_additional_edge_cases() {
        assert_eq!(parse_decimal_digits(b"0"), Some(0));
        assert_eq!(parse_decimal_digits(b"05"), Some(5));
        assert_eq!(parse_decimal_digits(b"a12"), None);
        assert_eq!(parse_decimal_digits(b"12a"), None);
    }

    #[test]
    fn test_strip_csi_numeric_prefix() {
        assert_eq!(
            strip_csi_numeric_prefix(b"\x1b[1;2H"),
            Some(b"1;2H".as_slice())
        );
        assert_eq!(
            strip_csi_numeric_prefix(b"\x1b[91;3u"),
            Some(b"91;3u".as_slice())
        );
        assert_eq!(
            strip_csi_numeric_prefix(b"\x1b[0;1;1M"),
            Some(b"0;1;1M".as_slice())
        );

        // Non-digit immediately after ESC [.
        assert_eq!(strip_csi_numeric_prefix(b"\x1b[A"), None);
        assert_eq!(strip_csi_numeric_prefix(b"\x1b[<0;1;1M"), None);
        assert_eq!(strip_csi_numeric_prefix(b"\x1b[;3u"), None);

        // Incomplete / invalid prefixes.
        assert_eq!(strip_csi_numeric_prefix(b"\x1b["), None);
        assert_eq!(strip_csi_numeric_prefix(b""), None);
        assert_eq!(strip_csi_numeric_prefix(b"1;2H"), None);
    }
}
