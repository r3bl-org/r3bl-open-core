// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Lexical scanning and parameter extraction from [`CSI`] sequences.
//!
//! [`CSI`]: crate::CsiSequence

use crate::{ByteOffset, CSI_MIN_LEN, WideningCastToU16, WideningCastToU32, byte_offset,
            core::ansi::constants::{ANSI_FUNCTION_KEY_TERMINATOR, ANSI_PARAM_SEPARATOR,
                                    ASCII_DIGIT_0, CSI_PREFIX, CSI_PREFIX_LEN}};

/// If `chunk` starts with [`CSI_PREFIX`] (`ESC [`) followed by an [`ASCII`] digit
/// (`'0'`..=`'9'`), strips the [`CSI_PREFIX`] and returns the payload slice starting at
/// the first digit.
///
/// ```text
/// Input chunk: &[u8] (e.g. b"\x1b[91;3u")
///
/// Index:     0       1         2      3      4      5      6
///        ┌───────┬───────┐ ┌──────┬──────┬──────┬──────┬──────┐
/// Byte:  │  ESC  │  '['  │ │ '9'  │ '1'  │ ';'  │ '3'  │ 'u'  │
///        └───────┴───────┘ └──────┴──────┴──────┴──────┴──────┘
///        │◄ CSI_PREFIX  ►│ │◄   Returned payload subslice    ►│
///           (stripped)        (must begin with ASCII digit)
/// ```
///
/// Returns `None` if any of this is true:
/// - `chunk` is too short,
/// - `chunk` does not begin with [`CSI_PREFIX`],
/// - byte after [`CSI_PREFIX`] in `chunk` isn't an [`ASCII`] digit, e.g. `ESC [ A`.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`CSI_PREFIX`]: crate::CSI_PREFIX
#[must_use]
pub fn strip_csi_numeric_prefix(chunk: &[u8]) -> Option<&[u8]> {
    // Early return if `chunk` is too short.
    if chunk.len() < CSI_MIN_LEN {
        return None;
    }

    let payload_slice = chunk.strip_prefix(CSI_PREFIX)?;
    let first_byte = payload_slice.first()?;

    if first_byte.is_ascii_digit() {
        // Strip CSI_PREFIX from chunk and return the rest.
        Some(payload_slice)
    } else {
        // The byte following CSI_PREFIX in chunk is not an ASCII digit.
        None
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
/// directly from the raw byte stream into an integer ([`u32`]).
///
/// See [`Rust Character And Byte Types`] in [`utf8`] for a detailed breakdown of how Rust
/// differentiates between 1-byte [`ASCII`] byte literals (`b'9'`), 4-byte Unicode
/// characters (`'9'`), and [`UTF-8`] strings (`"9"`).
///
/// ```text
/// The byte slice b"91" contains ASCII bytes, where each byte is a text-formatted digit:
///
/// Index:      0      1
///         ┌──────┬──────┐
/// Slice:  │ b'9' │ b'1' │ <- &[u8] byte slice, i.e. &[b'9', b'1']
///         └──────┴──────┘
///             │      │
///             ▼      ▼
///            57     49    <- actual u8 decimal values in memory
///            9      1     <- ASCII character digit
///
/// Resulting u32 integer: 91 (we need u32 to be spec compliant).
/// ```
///
/// # Arguments
///
/// - `chunk`: Chunk containing the text-formatted number, e.g. `&[b'9', b'1']`
///
/// # Returns
///
/// - `Some(u32)`: If the text contains only valid [`ASCII`] digits (`'0'`..=`'9'`).
/// - `None`: If `chunk` is empty or contains non-digit text (e.g. `;`, `:`, or letters).
///
/// # Examples
///
/// ```rust,ignore
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
/// [`Rust Character And Byte Types`]: mod@super::utf8#rust-character-and-byte-types
/// [`str::parse::<u32>()`]: str::parse
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
/// [`utf8`]: mod@super::utf8
#[must_use]
pub fn parse_decimal_digits(chunk: &[u8]) -> Option<u32> {
    const DECIMAL_RADIX: u32 = 10;

    // Early return if the chunk is empty.
    let chunk_is_empty = chunk.is_empty();
    if chunk_is_empty {
        return None;
    }

    // Early return if the chunk contains anything other than ASCII digits.
    let chunk_contains_only_text_formatted_numbers =
        chunk.iter().all(|byte| (*byte).is_ascii_digit());
    if !chunk_contains_only_text_formatted_numbers {
        return None;
    }

    // Invariant established: All bytes are guaranteed to be text formatted numbers.
    let text_formatted_number_chunk = chunk;
    let mut accumulated_value: u32 = 0;

    for byte in text_formatted_number_chunk.iter().copied() {
        // Convert ASCII character byte to its numeric digit value.
        // E.g., `b'5'` (53 dec) - `b'0'` (48 dec) = 5.
        let numeric_value = byte - ASCII_DIGIT_0;
        accumulated_value = accumulated_value
            .saturating_mul(DECIMAL_RADIX)
            .saturating_add(numeric_value.as_u32_widening());
    }

    Some(accumulated_value)
}

/// Result of extracting parameters and command terminator from a [`CSI`] byte slice.
///
/// ```text
/// Chunk:       ESC     '['      '1'    ';'    '2'    'H'
/// Index:        0       1        2      3      4      5
///           ┌───────┬───────┐ ┌──────┬──────┬──────┬──────┐
/// Byte:     │  ESC  │  '['  │ │ '1'  │ ';'  │ '2'  │ 'H'  │
///           └───────┴───────┘ └──────┴──────┴──────┴──────┘
///           │◄ CSI_PREFIX  ►│ │◄─── parameter body ──────►│
///           │   (2 bytes)   │ │         (4 bytes)         │
///           │                                             │
///           │◄───────────── total_consumed ──────────────►│
///           │                  (6 bytes)                  │
/// ```
///
/// [`CSI`]: crate::CsiSequence
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsiParams {
    /// Parsed numeric arguments (e.g. `[1, 2]` from `ESC [ 1 ; 2 H`).
    pub params: Vec<u16>,

    /// Command terminator character (e.g. `b'H'`, `b'~'`).
    pub final_byte: u8,

    /// Total bytes consumed from the chunk including the `ESC [` prefix
    /// ([`CSI_PREFIX_LEN`]).
    ///
    /// [`CSI_PREFIX_LEN`]: crate::CSI_PREFIX_LEN
    pub total_consumed: ByteOffset,
}

impl CsiParams {
    /// Extracts numeric parameters, final byte, and scanned byte count from a [`CSI`]
    /// chunk.
    ///
    /// This is a single-pass, zero-allocation scanner that parses parameter numbers and
    /// identifies the terminating command character in an incoming byte stream.
    ///
    /// # Streaming Behavior
    ///
    /// The input `chunk` may contain data that extends beyond the [`CSI`] sequence (e.g.
    /// subsequent keystrokes or text in a terminal stream buffer). This function scans
    /// only up through the first [`CsiTerminator`], leaving any trailing bytes intact for
    /// subsequent parsers.
    ///
    /// The caller can advance its stream cursor by [`Self::total_consumed`], which equals
    /// [`CSI_PREFIX_LEN`] (2 bytes for `ESC [`) + scanned parameter body bytes.
    ///
    /// # State Machine Transitions
    ///
    /// 1. **Digits (`'0'`..=`'9'`)**: Multiplies the running parameter accumulator by 10
    ///    and adds the digit value using saturating arithmetic.
    /// 2. **Separator (`';'`)**: Flushes the accumulated parameter into `params` via
    ///    [`std::mem::take()`] and resets the accumulator to `0`.
    /// 3. **Terminator (`'~'`, `'A'`..=`'Z'`, `'a'`..=`'z'`)**: Flushes the final
    ///    accumulated parameter, records the terminator byte, and halts scanning.
    /// 4. **Invalid byte**: If any unexpected byte is encountered, returns `None`.
    ///
    /// # Arguments
    ///
    /// - `chunk`: Byte slice starting with `ESC [` ([`CSI_PREFIX`]).
    ///
    /// # Returns
    ///
    /// - `Some(CsiParams)`: If the prefix is valid, contains only legal parameter
    ///   characters, and ends with a recognized command terminator.
    /// - `None`: If `chunk` is too short, missing `ESC [`, malformed, or missing a
    ///   terminator.
    ///
    /// [`CSI_PREFIX_LEN`]: crate::CSI_PREFIX_LEN
    /// [`CSI_PREFIX`]: crate::CSI_PREFIX
    /// [`CSI`]: crate::CsiSequence
    /// [`CsiTerminator`]: super::CsiTerminator
    #[must_use]
    pub fn try_extract(chunk: &[u8]) -> Option<Self> {
        const DECIMAL_RADIX: u16 = 10;

        // Early return if chunk cannot possibly contain CSI prefix + terminator.
        if chunk.len() < CSI_MIN_LEN {
            return None;
        }

        let payload = chunk.strip_prefix(CSI_PREFIX)?;

        let mut params = Vec::new();
        let mut acc_numeric_param: u16 = 0;
        let mut maybe_final_byte: Option<u8> = None;
        let mut bytes_scanned = byte_offset(0);

        // Single-pass state machine: Scans payload bytes until the first command
        // terminator character (`~` or ASCII letter) is reached. Any subsequent trailing
        // stream bytes in `chunk` remain unconsumed.
        for byte in payload.iter().copied() {
            bytes_scanned += byte_offset(1);

            match CsiByteKind::classify(byte) {
                CsiByteKind::Digit(digit) => {
                    acc_numeric_param = acc_numeric_param
                        .saturating_mul(DECIMAL_RADIX)
                        .saturating_add(digit.numeric_value().as_u16_widening());
                }
                CsiByteKind::Separator => {
                    params.push(std::mem::take(&mut acc_numeric_param));
                }
                CsiByteKind::Terminator(terminator) => {
                    params.push(std::mem::take(&mut acc_numeric_param));
                    maybe_final_byte = Some(terminator.as_u8());
                    break;
                }
                CsiByteKind::Invalid => return None,
            }
        }

        let final_byte = maybe_final_byte?;
        let total_consumed = byte_offset(CSI_PREFIX_LEN) + bytes_scanned;

        Some(Self {
            params,
            final_byte,
            total_consumed,
        })
    }
}

/// Represents the role of a byte inside a [`CSI`] sequence.
///
/// [`CSI`]: crate::CsiSequence
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CsiByteKind {
    /// Decimal digit with guaranteed numeric value `0..=9`.
    Digit(AsciiNumeric),

    /// Parameter separator `;` ([`ANSI_PARAM_SEPARATOR`]).
    ///
    /// [`ANSI_PARAM_SEPARATOR`]: crate::core::ansi::constants::ANSI_PARAM_SEPARATOR
    Separator,

    /// Terminating character ([`CsiTerminator`]).
    Terminator(CsiTerminator),

    /// Any byte that is invalid in a numeric [`CSI`] sequence.
    ///
    /// [`CSI`]: crate::CsiSequence
    Invalid,
}

impl CsiByteKind {
    /// Classifies a raw byte in a [`CSI`] parameter sequence into a [`CsiByteKind`].
    ///
    /// [`CSI`]: crate::CsiSequence
    #[must_use]
    pub fn classify(byte: u8) -> Self {
        if let Some(digit) = AsciiNumeric::try_from_ascii_byte(byte) {
            return Self::Digit(digit);
        }

        if byte == ANSI_PARAM_SEPARATOR {
            return Self::Separator;
        }

        if let Some(terminator) = CsiTerminator::try_from_u8(byte) {
            return Self::Terminator(terminator);
        }

        Self::Invalid
    }
}

/// Single [`ASCII`] numeric character byte guaranteed to be in `'0'..='9'`.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct AsciiNumeric(u8);

impl AsciiNumeric {
    /// Attempts to construct an [`AsciiNumeric`] from an [`ASCII`] character byte
    /// (`b'0'`..=`b'9'`).
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    #[must_use]
    pub const fn try_from_ascii_byte(byte: u8) -> Option<Self> {
        if byte.is_ascii_digit() {
            Some(Self(byte))
        } else {
            None
        }
    }

    /// Returns the underlying [`ASCII`] byte (e.g. `b'5'`).
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    #[must_use]
    #[allow(dead_code)]
    pub const fn as_u8(self) -> u8 { self.0 }

    /// Numeric value in range `0..=9`.
    #[must_use]
    pub const fn numeric_value(self) -> u8 { self.0 - ASCII_DIGIT_0 }
}

/// Single [`ASCII`] alphabetic character byte guaranteed to be in `'A'..='Z'` or
/// `'a'..='z'`.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct AsciiAlpha(u8);

impl AsciiAlpha {
    /// Attempts to construct an [`AsciiAlpha`] from a raw byte.
    #[must_use]
    pub const fn try_from_ascii_byte(byte: u8) -> Option<Self> {
        if byte.is_ascii_alphabetic() {
            Some(Self(byte))
        } else {
            None
        }
    }

    /// Returns the underlying [`ASCII`] byte.
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    #[must_use]
    pub const fn as_u8(self) -> u8 { self.0 }
}

/// Valid terminating character for a [`CSI`] sequence.
///
/// Can be either the function key terminator `~` ([`ANSI_FUNCTION_KEY_TERMINATOR`])
/// or an [`ASCII`] alphabetic command character (`A`..=`Z`, `a`..=`z`).
///
/// [`ANSI_FUNCTION_KEY_TERMINATOR`]: crate::core::ansi::constants::ANSI_FUNCTION_KEY_TERMINATOR
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`CSI`]: crate::CsiSequence
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CsiTerminator {
    /// Function key terminator `~` ([`ANSI_FUNCTION_KEY_TERMINATOR`]).
    ///
    /// [`ANSI_FUNCTION_KEY_TERMINATOR`]: crate::core::ansi::constants::ANSI_FUNCTION_KEY_TERMINATOR
    Tilde,

    /// Alphabetic command character (`A`..=`Z`, `a`..=`z`).
    Alpha(AsciiAlpha),
}

impl CsiTerminator {
    /// Attempts to construct a [`CsiTerminator`] from a raw byte.
    #[must_use]
    pub fn try_from_u8(byte: u8) -> Option<Self> {
        if byte == ANSI_FUNCTION_KEY_TERMINATOR {
            Some(Self::Tilde)
        } else {
            AsciiAlpha::try_from_ascii_byte(byte).map(Self::Alpha)
        }
    }

    /// Returns the raw byte representation of the terminator.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Tilde => ANSI_FUNCTION_KEY_TERMINATOR,
            Self::Alpha(alpha) => alpha.as_u8(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_csi_byte() {
        // Digits.
        assert_eq!(
            CsiByteKind::classify(b'0'),
            CsiByteKind::Digit(AsciiNumeric::try_from_ascii_byte(b'0').unwrap())
        );
        assert_eq!(
            CsiByteKind::classify(b'9'),
            CsiByteKind::Digit(AsciiNumeric::try_from_ascii_byte(b'9').unwrap())
        );

        // Separator.
        assert_eq!(CsiByteKind::classify(b';'), CsiByteKind::Separator);

        // Terminators.
        assert_eq!(
            CsiByteKind::classify(b'~'),
            CsiByteKind::Terminator(CsiTerminator::Tilde)
        );
        assert_eq!(
            CsiByteKind::classify(b'A'),
            CsiByteKind::Terminator(CsiTerminator::Alpha(
                AsciiAlpha::try_from_ascii_byte(b'A').unwrap()
            ))
        );
        assert_eq!(
            CsiByteKind::classify(b'Z'),
            CsiByteKind::Terminator(CsiTerminator::Alpha(
                AsciiAlpha::try_from_ascii_byte(b'Z').unwrap()
            ))
        );
        assert_eq!(
            CsiByteKind::classify(b'a'),
            CsiByteKind::Terminator(CsiTerminator::Alpha(
                AsciiAlpha::try_from_ascii_byte(b'a').unwrap()
            ))
        );
        assert_eq!(
            CsiByteKind::classify(b'u'),
            CsiByteKind::Terminator(CsiTerminator::Alpha(
                AsciiAlpha::try_from_ascii_byte(b'u').unwrap()
            ))
        );
        assert_eq!(
            CsiByteKind::classify(b'z'),
            CsiByteKind::Terminator(CsiTerminator::Alpha(
                AsciiAlpha::try_from_ascii_byte(b'z').unwrap()
            ))
        );

        // Invalid ASCII boundaries and control characters.
        assert_eq!(CsiByteKind::classify(b'@'), CsiByteKind::Invalid); // Before 'A'.
        assert_eq!(CsiByteKind::classify(b'['), CsiByteKind::Invalid); // After 'Z'.
        assert_eq!(CsiByteKind::classify(b'`'), CsiByteKind::Invalid); // Before 'a'.
        assert_eq!(CsiByteKind::classify(b'{'), CsiByteKind::Invalid); // After 'z'.
        assert_eq!(CsiByteKind::classify(b'?'), CsiByteKind::Invalid);
        assert_eq!(CsiByteKind::classify(b' '), CsiByteKind::Invalid);
    }

    #[test]
    fn test_try_extract_csi_params() {
        // Multi-parameter sequence: `ESC [ 1 ; 2 H`.
        let chunk = b"\x1b[1;2H";
        let csi_params =
            CsiParams::try_extract(chunk).expect("Should extract CSI params");
        assert_eq!(csi_params.params, vec![1, 2]);
        assert_eq!(csi_params.final_byte, b'H');
        assert_eq!(csi_params.total_consumed, byte_offset(6));

        // Single parameter sequence: ESC [ 5 ~.
        let chunk_tilde = b"\x1b[5~";
        let csi_params_tilde =
            CsiParams::try_extract(chunk_tilde).expect("Should extract CSI params");
        assert_eq!(csi_params_tilde.params, vec![5]);
        assert_eq!(csi_params_tilde.final_byte, b'~');
        assert_eq!(csi_params_tilde.total_consumed, byte_offset(4));

        // Lowercase terminator (e.g. Kitty `CSI u`: `ESC [ 91 ; 3 u)`.
        let chunk_kitty = b"\x1b[91;3u";
        let csi_params_kitty =
            CsiParams::try_extract(chunk_kitty).expect("Should extract CSI u params");
        assert_eq!(csi_params_kitty.params, vec![91, 3]);
        assert_eq!(csi_params_kitty.final_byte, b'u');
        assert_eq!(csi_params_kitty.total_consumed, byte_offset(7));

        // Invalid byte in parameter body.
        let chunk_invalid = b"\x1b[1;?H";
        assert!(CsiParams::try_extract(chunk_invalid).is_none());

        // Missing ESC [ prefix.
        assert!(CsiParams::try_extract(b"1;2H").is_none());
        assert!(CsiParams::try_extract(b"\x1b").is_none());
        assert!(CsiParams::try_extract(b"").is_none());

        // Truncated chunk / missing terminator.
        assert!(CsiParams::try_extract(b"\x1b[1;2").is_none());
        assert!(CsiParams::try_extract(b"\x1b[").is_none());
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

    #[test]
    fn test_ascii_numeric() {
        let digit_0 = AsciiNumeric::try_from_ascii_byte(b'0').unwrap();
        assert_eq!(digit_0.as_u8(), b'0');
        assert_eq!(digit_0.numeric_value(), 0);

        let digit_9 = AsciiNumeric::try_from_ascii_byte(b'9').unwrap();
        assert_eq!(digit_9.as_u8(), b'9');
        assert_eq!(digit_9.numeric_value(), 9);

        assert!(AsciiNumeric::try_from_ascii_byte(b'/').is_none());
        assert!(AsciiNumeric::try_from_ascii_byte(b':').is_none());
        assert!(AsciiNumeric::try_from_ascii_byte(b'a').is_none());
    }

    #[test]
    fn test_ascii_alpha() {
        let alpha_a = AsciiAlpha::try_from_ascii_byte(b'A').unwrap();
        assert_eq!(alpha_a.as_u8(), b'A');

        let alpha_z = AsciiAlpha::try_from_ascii_byte(b'z').unwrap();
        assert_eq!(alpha_z.as_u8(), b'z');

        assert!(AsciiAlpha::try_from_ascii_byte(b'@').is_none());
        assert!(AsciiAlpha::try_from_ascii_byte(b'[').is_none());
        assert!(AsciiAlpha::try_from_ascii_byte(b'`').is_none());
        assert!(AsciiAlpha::try_from_ascii_byte(b'{').is_none());
        assert!(AsciiAlpha::try_from_ascii_byte(b'1').is_none());
    }

    #[test]
    fn test_csi_terminator() {
        let tilde = CsiTerminator::try_from_u8(b'~').unwrap();
        assert_eq!(tilde, CsiTerminator::Tilde);
        assert_eq!(tilde.as_u8(), b'~');

        let letter_h = CsiTerminator::try_from_u8(b'H').unwrap();
        assert_eq!(letter_h.as_u8(), b'H');

        assert!(CsiTerminator::try_from_u8(b'0').is_none());
        assert!(CsiTerminator::try_from_u8(b';').is_none());
        assert!(CsiTerminator::try_from_u8(b' ').is_none());
    }
}
