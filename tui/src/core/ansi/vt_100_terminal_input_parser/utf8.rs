// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`UTF-8`] text parsing between [`ANSI`] sequences.
//!
//! This module handles conversion of raw [`UTF-8`] bytes (received as regular text input
//! between [`ANSI`] escape sequences) into keyboard events representing typed characters.
//!
//! ## Where You Are in the Pipeline
//!
//! For the full data flow, see the [parent module documentation]. This diagram shows
//! where `utf8.rs` fits:
//!
//! ```text
//! DirectToAnsiInputDevice (async I/O layer)
//!    │
//!    ▼
//! router.rs (routing & `ESC` detection)
//!    │ (routes non-escape bytes here)
//! ┌──▼───────────────────────────────────────┐  ┌──────────────────┐
//! │  utf8.rs                                 ◄──┤ **YOU ARE HERE** │
//! │  • Parse UTF-8 multi-byte sequences      │  └──────────────────┘
//! │  • Generate character events             │
//! │  • Handle incomplete sequences           │
//! └──────────────────────────────────────────┘
//!    │
//!    ▼
//! VT100InputEventIR::Keyboard { code: Char(ch), .. }
//!    │
//!    ▼
//! convert_input_event() → InputEvent (returned to application)
//! ```
//!
//! **Navigate**:
//! - ⬆️ **Up**: [`router`] - Main routing entry point
//! - ➡️ **Peer**: [`keyboard`], [`mouse`], [`terminal_events`] - Other specialized
//!   parsers
//! - 📚 **Types**: [`VT100KeyCodeIR::Char`]
//! - 📤 **Converted by**: [`convert_input_event()`] in `protocol_conversion.rs` (not this
//!   module)
//!
//! ## Handles:
//! - Single-byte [`UTF-8`] characters ([`ASCII`])
//! - Multi-byte [`UTF-8`] sequences (2-4 bytes)
//! - Incomplete [`UTF-8`] sequences (buffering)
//! - Invalid [`UTF-8`] sequences (graceful error handling)
//!
//! ## Rust Character And Byte Types
//!
//! The distinctions between [`u8`], [`char`], and `&`[`str`] are core Rust language
//! concepts, but understanding them is essential for parsing terminal streams (which mix
//! raw [`ASCII`] bytes, numbers sent as text, and multi-byte [`UTF-8`] characters):
//!
//! 1. `b'9'` is an 8-bit integer ([`u8`]): It is simply syntactic sugar for the [`ASCII`]
//!    integer value `57u8`. It cannot hold multi-byte Unicode characters (e.g. `b'🦀'` is
//!    a compiler error).
//! 2. `'9'` is a 32-bit Unicode character ([`char`]): In Rust, all [`char`] types are 4
//!    bytes wide (`size_of::<char>() == 4`) to represent any valid Unicode scalar value.
//! 3. `&`[`str`] / [`String`] is a variable-length [`UTF-8`] byte slice: Each character
//!    takes between 1 and 4 bytes in memory ([`ASCII`] digits take 1 byte, emojis take 4
//!    bytes).
//! 4. **Relationship**: `b'9'` is the [`ASCII`] byte representation of the digit
//!    character `'9'`, but its concrete Rust type is [`u8`], not [`char`].
//!
//! | Syntax | Name              | Type                 | Size              | Value in Memory                     |
//! | :----- | :---------------- | :------------------- | :---------------- | :---------------------------------- |
//! | `b'9'` | Byte literal      | [`u8`]               | 1 byte (8 bits)   | `57` (`0x39`)                       |
//! | `'9'`  | Character literal | [`char`]             | 4 bytes (32 bits) | `'\u{0039}'` (Unicode scalar value) |
//! | `"9"`  | String literal    | `&`[`str`] (`&[u8]`) | 1 byte payload    | `[0x39]` ([`UTF-8`] encoded)        |
//!
//! Contrast this with numeric [`ANSI`] parameter parsing in [`parse_decimal_digits()`],
//! which parses text digit bytes (`&[u8]`) directly into integers without allocating or
//! converting to `&`[`str`].
//!
//! ## [`UTF-8`] Encoding Explained
//!
//! [`UTF-8`] uses **bit pattern matching** (not arithmetic) to identify byte types and
//! extract data. The high bits are structural markers; remaining bits carry the `Unicode`
//! code point.
//!
//! ### Byte Type Detection
//!
//! ```text
//! Byte Pattern   Meaning              Detection Mask
//! ──────────────────────────────────────────────────
//! 0xxxxxxx       ASCII (1-byte)       byte & 0x80 == 0x00
//! 110xxxxx       2-byte start         byte & 0xE0 == 0xC0
//! 1110xxxx       3-byte start         byte & 0xF0 == 0xE0
//! 11110xxx       4-byte start         byte & 0xF8 == 0xF0
//! 10xxxxxx       Continuation         byte & 0xC0 == 0x80
//! ```
//!
//! The leading 1s before the first 0 indicate total byte count:
//! - `0xxxxxxx` → 0 leading 1s → 1 byte total
//! - `110xxxxx` → 2 leading 1s → 2 bytes total
//! - `1110xxxx` → 3 leading 1s → 3 bytes total
//! - `11110xxx` → 4 leading 1s → 4 bytes total
//!
//! ### First Byte: Pattern + Data
//!
//! The first byte carries both the length marker AND the most significant data bits:
//!
//! ```text
//! Sequence   Pattern Bits   Data Bits   Total Data Available
//! ────────────────────────────────────────────────────────────
//! 1-byte     1 (the 0)      7 bits      7 bits
//! 2-byte     3 (110)        5 bits      5 + 6 = 11 bits
//! 3-byte     4 (1110)       4 bits      4 + 6 + 6 = 16 bits
//! 4-byte     5 (11110)      3 bits      3 + 6 + 6 + 6 = 21 bits
//! ```
//!
//! ### Continuation Bytes: `10xxxxxx`
//!
//! Each continuation byte:
//! - Prefix `10` marks it as "not a start byte" (enables self-synchronization)
//! - Remaining 6 bits carry data
//! - Extract with: `byte & 0x3F`
//!
//! ### Worked Example: `'é'` (`U+00E9` = 233)
//!
//! ```text
//! Step 1: 233 in binary = 11101001 (needs 8 bits, won't fit in 7-bit ASCII)
//!
//! Step 2: Use 2-byte encoding (11 data bits available)
//!         Pad to 11 bits: 000_11_101001
//!                         └─┬─┘└──┬───┘
//!                         5 bits  6 bits
//!
//! Step 3: Insert into templates:
//!         First byte:  110_00011  (pattern 110 + 5 MSB data bits)
//!         Second byte: 10_101001  (pattern 10  + 6 LSB data bits)
//!
//! Result: 0xC3 0xA9
//!
//! Verification: Extract data bits and recombine:
//!         (0xC3 & 0x1F) << 6 | (0xA9 & 0x3F)
//!         = 0x03 << 6 | 0x29
//!         = 0xC0 | 0x29
//!         = 0xE9
//!         = 233 (matches the original codepoint)
//! ```
//!
//! ### Why Bit Patterns (Not Arithmetic)?
//!
//! - **Fast**: Detection is just bitwise AND + compare
//! - **Self-synchronizing**: Jump anywhere in a stream, scan for `0xxxxxxx` or `11xxxxxx`
//! - **Unambiguous**: Continuation bytes (`10xxxxxx`) can never be confused with start
//!   bytes
//! - **[`ASCII`]-compatible**: Single-byte chars unchanged (the `0` prefix means
//!   "complete")
//!
//! # Important: [`UTF-8`] Byte Length vs Display Width
//!
//! This module handles **[`UTF-8`] byte-level parsing only**, converting raw bytes from
//! terminal input into Unicode characters. It does NOT handle display width.
//!
//! ## Two Separate Concerns
//!
//! | Concern                                 | What it measures          | Example: '😀' |
//! | :-------------------------------------- | :------------------------ | :------------ |
//! | **[`UTF-8`] byte length** (this module) | Memory size in bytes      | 4 bytes       |
//! | **Display width** (graphemes module)    | Terminal columns occupied | 2 columns     |
//!
//! - **This module**: Returns [`ParsedInputEventIR`] where `bytes_consumed` is the number
//!   of bytes to advance in the input buffer (1-4 bytes for [`UTF-8`]).
//!
//! - **Display rendering**: Calculated separately using the [`unicode_width`] crate. See
//!   [`mod@crate::graphemes`] for comprehensive documentation on Unicode display width,
//!   grapheme clusters, and the three types of indices ([`ByteIndex`], [`SegIndex`],
//!   [`VPCol`]).
//!
//! ## Why This Matters
//!
//! A common mistake is assuming that `bytes_consumed` relates to how many terminal
//! columns the character occupies. This is incorrect:
//!
//! | Character | [`UTF-8`] Bytes | Display Width | Category           |
//! | --------- | --------------- | ------------- | ------------------ |
//! | `H`       | 1 byte          | 1 column      | [`ASCII`]          |
//! | `©`       | 2 bytes         | 1 column      | Latin-1 supplement |
//! | `€`       | 3 bytes         | 1 column      | Currency symbol    |
//! | `你`      | 3 bytes         | 2 columns     | CJK fullwidth      |
//! | `😀`      | 4 bytes         | 2 columns     | Emoji              |
//!
//! If you need to position the cursor or calculate line lengths, you need display width
//! calculation, not byte length. See [`crate::graphemes::GCStringOwned`] for text
//! rendering utilities.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
//! [`ByteIndex`]: crate::ByteIndex
//! [`ByteOffset`]: crate::ByteOffset
//! [`convert_input_event()`]:
//!     crate::direct_to_ansi::input::protocol_conversion::convert_input_event
//! [`keyboard`]: mod@super::keyboard
//! [`mouse`]: mod@super::mouse
//! [`parse_decimal_digits()`]: super::csi_scanner::parse_decimal_digits
//! [`ParsedInputEventIR`]: super::ParsedInputEventIR
//! [`router`]: mod@super::router
//! [`SegIndex`]: crate::SegIndex
//! [`terminal_events`]: mod@super::terminal_events
//! [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
//! [`VPCol`]: crate::VPCol
//! [`VT100InputEventIR`]: super::VT100InputEventIR
//! [`VT100KeyCodeIR::Char`]: super::VT100KeyCodeIR::Char
//! [parent module documentation]: mod@super#primary-consumer

use super::ir_event_types::{ParsedInputEventIR, VT100InputEventIR, VT100KeyCodeIR,
                            VT100KeyModifiersIR};
use crate::{ArrayBoundsCheck, ArrayOverflowResult, ByteOffset, UTF8_1BYTE_START_MAX,
            UTF8_1BYTE_START_MIN, UTF8_2BYTE_START_MAX, UTF8_2BYTE_START_MIN,
            UTF8_3BYTE_START_MAX, UTF8_3BYTE_START_MIN, UTF8_4BYTE_START_MAX,
            UTF8_4BYTE_START_MIN, UTF8_CONTINUATION_MASK, UTF8_CONTINUATION_PATTERN,
            byte_index, byte_len, byte_offset};

/// Parses [`UTF-8`] text and returns a single [`VT100InputEventIR`] for the first
/// complete character.
///
/// Converts raw [`UTF-8`] bytes into character input events. Handles multi-byte
/// [`UTF-8`] sequences (1-4 bytes).
///
/// # Returns
///
/// - The parsed character event and byte count on success.
/// - Nothing if the buffer contains an incomplete or invalid [`UTF-8`] sequence.
///
/// The caller ([`DirectToAnsiInputDevice`]) can call this repeatedly to parse multiple
/// characters from the buffer.
///
/// # Important: `bytes_consumed` ≠ display width
///
/// The returned `bytes_consumed` indicates how many bytes to advance in the input buffer.
/// This is **NOT** the display width (terminal columns) of the character.
///
/// Example:
/// - '😀' returns `bytes_consumed = 4` ([`UTF-8`] encoding is 4 bytes)
/// - But '😀' occupies **2 terminal columns** (display width)
///
/// For display width calculation and cursor positioning, see [`mod@crate::graphemes`].
///
/// [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::input::DirectToAnsiInputDevice
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
#[must_use]
pub fn parse_utf8_text(buffer: &[u8]) -> Option<ParsedInputEventIR> {
    // Check if we have a complete UTF-8 sequence.
    let utf8_sequence_len = try_get_complete_utf8_len(buffer)?;

    // Decode the complete UTF-8 sequence slice.
    let complete_utf8_slice = &buffer[..utf8_sequence_len.as_usize()];
    let utf8_char = decode_utf8(complete_utf8_slice)?;

    // Return keyboard event with the decoded character.
    Some(ParsedInputEventIR::new(
        VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Char(utf8_char),
            modifiers: VT100KeyModifiersIR::default(),
        },
        utf8_sequence_len,
    ))
}

/// Checks if a [`UTF-8`] byte sequence is complete and returns its byte length.
///
/// # Returns
///
/// - The scanner cursor displacement ([`ByteOffset`], 1-4 bytes) needed to consume the
///   complete character from the input buffer.
/// - Nothing if more bytes are needed or the sequence is invalid.
///
/// [`ByteOffset`]: crate::ByteOffset
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
fn try_get_complete_utf8_len(buffer: &[u8]) -> Option<ByteOffset> {
    // If the buffer is empty, there is no UTF-8 sequence to parse.
    if buffer.is_empty() {
        return None;
    }

    // Check the first byte to determine the expected length of the sequence.
    let first_byte = buffer[0];
    let required_len = get_utf8_length(first_byte)?;

    // Check if we have enough bytes in the buffer.
    let last_byte_index = byte_index(required_len.as_last_byte_index());
    let byte_len = byte_len(buffer.len());
    if last_byte_index.overflows(byte_len) == ArrayOverflowResult::Overflowed {
        return None; // Incomplete sequence.
    }

    // Verify all continuation bytes are correctly formatted.
    let continuation_bytes = &buffer[1..required_len.as_usize()];
    for byte in continuation_bytes {
        // Continuation bytes must be `10xxxxxx` (0x80-0xBF).
        if (byte & UTF8_CONTINUATION_MASK) != UTF8_CONTINUATION_PATTERN {
            return None; // Invalid continuation byte.
        }
    }

    Some(required_len)
}

/// Gets the expected length of a [`UTF-8`] sequence from its first byte.
///
/// This implements the same logic as the unstable [`core::str::utf8_char_width`], but
/// uses [`Option<ByteOffset>`] for type-safe error handling. We maintain this custom
/// implementation because:
///
/// - The [`std`] library version requires nightly Rust ([`str_internals`] feature)
/// - Our `Option` return type is more explicit than returning `0` for invalid bytes
/// - Zero external dependencies
///
/// # Returns
///
/// - The scanner cursor displacement ([`ByteOffset`], 1-4) required to consume the
///   [`UTF-8`] character.
/// - Nothing if the first byte is invalid (continuation byte or reserved).
///
/// # Important: This is NOT the same as [`unicode_width`]
///
/// This function calculates **[`UTF-8`] byte length** (how many bytes encode the
/// character), NOT **display width** (how many terminal columns it occupies). These are
/// independent:
///
/// - A 3-byte character like '€' occupies **1 column** (narrow)
/// - A 3-byte character like '你' occupies **2 columns** (wide/fullwidth)
/// - Both return `Some(byte_offset(3))` from this function (same byte length)
///
/// For display width calculation, see the [`unicode_width`] crate used in
/// [`mod@crate::graphemes`]. See also the [module-level documentation] for a
/// comprehensive explanation of this distinction.
///
/// [`ByteOffset`]: crate::ByteOffset
/// [`core::str::utf8_char_width`]: https://en.wikipedia.org/wiki/UTF-8#Encoding
/// [`str_internals`]:
///     https://doc.rust-lang.org/unstable-book/library-features/str-internals.html
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
/// [module-level documentation]: self#important-utf-8-byte-length-vs-display-width
fn get_utf8_length(first_byte: u8) -> Option<ByteOffset> {
    match first_byte {
        // ASCII: single byte (`0xxxxxxx`).
        UTF8_1BYTE_START_MIN..=UTF8_1BYTE_START_MAX => Some(byte_offset(1)),
        // Start byte for 2-byte sequence (`110xxxxx`, RFC 3629: 0xC2..=0xDF).
        UTF8_2BYTE_START_MIN..=UTF8_2BYTE_START_MAX => Some(byte_offset(2)),
        // Start byte for 3-byte sequence (`1110xxxx`).
        UTF8_3BYTE_START_MIN..=UTF8_3BYTE_START_MAX => Some(byte_offset(3)),
        // Start byte for 4-byte sequence (`11110xxx`, RFC 3629: 0xF0..=0xF4).
        UTF8_4BYTE_START_MIN..=UTF8_4BYTE_START_MAX => Some(byte_offset(4)),
        // Continuation byte (`10xxxxxx`): invalid as start byte.
        // Overlong 2-byte start bytes (`0xC0`, `0xC1`).
        // Out-of-range 4-byte start bytes (`0xF5..=0xF7`).
        // Reserved or invalid bytes (`11111xxx`).
        _ => None,
    }
}

/// Validates and decodes a complete [`UTF-8`] byte sequence into a [`char`].
///
/// Uses [`core::str::from_utf8`] to decode and validate the character slice. This
/// ensures full compliance with [RFC 3629] (rejecting invalid sequences, overlong
/// encodings, surrogate halves, and codepoints exceeding `0x10_FFFF`).
///
/// # Returns
///
/// Returns the decoded [`char`], or [`None`] if the sequence is invalid.
///
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
/// [RFC 3629]: https://datatracker.ietf.org/doc/html/rfc3629
fn decode_utf8(char_bytes: &[u8]) -> Option<char> {
    core::str::from_utf8(char_bytes).ok()?.chars().next()
}

/// Unit tests for [`UTF-8`] text parsing.
///
/// These tests use generator functions instead of hardcoded magic strings to ensure
/// consistency between sequence generation and parsing. For testing strategy details,
/// see the [testing strategy] documentation.
///
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
/// [testing strategy]: mod@super#testing-strategy
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_character() {
        // Single ASCII character: 'a' (0x61).
        let buffer = b"a";
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_utf8_text(buffer).expect("Should parse ASCII");

        assert_eq!(consumed, byte_offset(1));
        match event {
            VT100InputEventIR::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCodeIR::Char('a'));
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_ascii_multiple_chars() {
        // Test parsing multiple ASCII characters sequentially.
        let buffer = b"hello";

        // Parse 'h'.
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_utf8_text(buffer).expect("Should parse first char");
        assert_eq!(consumed, byte_offset(1));
        match event {
            VT100InputEventIR::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCodeIR::Char('h'));
            }
            _ => panic!("Expected Keyboard event"),
        }

        // Parse 'e' from remainder.
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_utf8_text(&buffer[1..]).expect("Should parse second char");
        assert_eq!(consumed, byte_offset(1));
        match event {
            VT100InputEventIR::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCodeIR::Char('e'));
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_two_byte_utf8() {
        // Two-byte character: '©' (0xC2 0xA9).
        let buffer = b"\xC2\xA9";
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_utf8_text(buffer).expect("Should parse 2-byte UTF-8");

        assert_eq!(consumed, byte_offset(2));
        match event {
            VT100InputEventIR::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCodeIR::Char('©'));
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_three_byte_utf8() {
        // Three-byte character: '€' (0xE2 0x82 0xAC).
        let buffer = b"\xE2\x82\xAC";
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_utf8_text(buffer).expect("Should parse 3-byte UTF-8");

        assert_eq!(consumed, byte_offset(3));
        match event {
            VT100InputEventIR::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCodeIR::Char('€'));
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_four_byte_utf8() {
        // Four-byte character: '😀' (0xF0 0x9F 0x98 0x80).
        let buffer = b"\xF0\x9F\x98\x80";
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_utf8_text(buffer).expect("Should parse 4-byte UTF-8");

        assert_eq!(consumed, byte_offset(4));
        match event {
            VT100InputEventIR::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCodeIR::Char('😀'));
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_incomplete_two_byte_sequence() {
        // Incomplete 2-byte sequence: only first byte.
        let buffer = b"\xC2";
        let result = parse_utf8_text(buffer);
        assert!(
            result.is_none(),
            "Should not parse incomplete 2-byte sequence"
        );
    }

    #[test]
    fn test_incomplete_three_byte_sequence() {
        // Incomplete 3-byte sequence: only first two bytes.
        let buffer = b"\xE2\x82";
        let result = parse_utf8_text(buffer);
        assert!(
            result.is_none(),
            "Should not parse incomplete 3-byte sequence"
        );
    }

    #[test]
    fn test_incomplete_four_byte_sequence() {
        // Incomplete 4-byte sequence: only first three bytes.
        let buffer = b"\xF0\x9F\x98";
        let result = parse_utf8_text(buffer);
        assert!(
            result.is_none(),
            "Should not parse incomplete 4-byte sequence"
        );
    }

    #[test]
    fn test_invalid_continuation_byte() {
        // Invalid: 2-byte sequence with wrong continuation byte.
        // Expected: 0xC2 0xA9, but provide: 0xC2 0x00 (0x00 is not a valid continuation).
        let buffer = b"\xC2\x00";
        let result = parse_utf8_text(buffer);
        assert!(result.is_none(), "Should reject invalid continuation byte");
    }

    #[test]
    fn test_invalid_start_byte_continuation() {
        // Invalid: continuation byte (0x80) at start of buffer.
        let buffer = b"\x80hello";
        let result = parse_utf8_text(buffer);
        assert!(result.is_none(), "Should reject continuation byte as start");
    }

    #[test]
    fn test_reserved_byte_value() {
        // Invalid: reserved byte value (0xFF).
        let buffer = b"\xFF";
        let result = parse_utf8_text(buffer);
        assert!(result.is_none(), "Should reject reserved byte");
    }

    #[test]
    fn test_empty_buffer() {
        // Empty buffer.
        let buffer = b"";
        let result = parse_utf8_text(buffer);
        assert!(result.is_none(), "Should not parse empty buffer");
    }

    #[test]
    fn test_mixed_ascii_and_multibyte() {
        // Buffer with ASCII followed by multi-byte.
        let buffer = b"a\xC2\xA9b";

        // Parse ASCII 'a'.
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_utf8_text(buffer).expect("Should parse ASCII");
        assert_eq!(consumed, byte_offset(1));
        match event {
            VT100InputEventIR::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCodeIR::Char('a'));
            }
            _ => panic!("Expected Keyboard event"),
        }

        // Parse 2-byte '©'.
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_utf8_text(&buffer[1..]).expect("Should parse 2-byte");
        assert_eq!(consumed, byte_offset(2));
        match event {
            VT100InputEventIR::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCodeIR::Char('©'));
            }
            _ => panic!("Expected Keyboard event"),
        }

        // Parse ASCII 'b'.
        let ParsedInputEventIR {
            event,
            bytes_consumed: consumed,
        } = parse_utf8_text(&buffer[3..]).expect("Should parse ASCII");
        assert_eq!(consumed, byte_offset(1));
        match event {
            VT100InputEventIR::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCodeIR::Char('b'));
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_surrogate_codepoint_rejection() {
        // `U+D800` is a surrogate code point (illegal in UTF-8: 0xED 0xA0 0x80).
        // try_get_complete_utf8_len passes structural check, but decode_utf8 rejects via
        // core::str::from_utf8.
        let buffer = &[0xED, 0xA0, 0x80];
        let result = parse_utf8_text(buffer);
        assert!(result.is_none(), "Should reject surrogate codepoints");
    }

    #[test]
    fn test_out_of_range_codepoint_rejection() {
        // Codepoints > `U+10FFFF` are invalid Unicode scalars.
        // U+110000: 0xF4 0x90 0x80 0x80 (valid start byte 0xF4, rejected by decode_utf8).
        let buffer_over_max = &[0xF4, 0x90, 0x80, 0x80];
        assert!(
            parse_utf8_text(buffer_over_max).is_none(),
            "Should reject codepoints > U+10FFFF"
        );

        // Maximum 4-byte bit pattern (0xF7 0xBF 0xBF 0xBF -> `U+1FFFFF`).
        // 0xF7 is rejected early by get_utf8_length (UTF8_4BYTE_START_MAX is 0xF4).
        let buffer_pattern_max = &[0xF7, 0xBF, 0xBF, 0xBF];
        assert!(
            parse_utf8_text(buffer_pattern_max).is_none(),
            "Should reject 4-byte out-of-range bit patterns"
        );
    }

    #[test]
    fn test_invalid_subsequent_continuation_bytes() {
        // 3-byte sequence: valid 1st continuation (0x82), invalid 2nd continuation
        // (0x20).
        let buffer_3byte = &[0xE2, 0x82, 0x20];
        assert!(
            parse_utf8_text(buffer_3byte).is_none(),
            "Should reject 3-byte sequence with invalid 2nd continuation byte"
        );

        // 4-byte sequence: valid 1st (0x9F) and 2nd (0x98), invalid 3rd continuation
        // (0x00).
        let buffer_4byte = &[0xF0, 0x9F, 0x98, 0x00];
        assert!(
            parse_utf8_text(buffer_4byte).is_none(),
            "Should reject 4-byte sequence with invalid 3rd continuation byte"
        );
    }

    #[test]
    fn test_ascii_boundary_values() {
        // Lower boundary: NUL byte (0x00).
        let buffer_nul = b"\x00";
        let ParsedInputEventIR {
            event: event_nul,
            bytes_consumed: consumed_nul,
        } = parse_utf8_text(buffer_nul).expect("Should parse NUL byte");
        assert_eq!(consumed_nul, byte_offset(1));
        match event_nul {
            VT100InputEventIR::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCodeIR::Char('\0'));
            }
            _ => panic!("Expected Keyboard event"),
        }

        // Upper boundary: DEL byte (0x7F).
        let buffer_del = b"\x7F";
        let ParsedInputEventIR {
            event: event_del,
            bytes_consumed: consumed_del,
        } = parse_utf8_text(buffer_del).expect("Should parse DEL byte");
        assert_eq!(consumed_del, byte_offset(1));
        match event_del {
            VT100InputEventIR::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCodeIR::Char('\x7F'));
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_early_rejection_start_bytes() {
        // Overlong 2-byte start bytes (0xC0, 0xC1) rejected early at get_utf8_length.
        assert_eq!(get_utf8_length(0xC0), None);
        assert_eq!(get_utf8_length(0xC1), None);
        // Valid 2-byte start boundary.
        assert_eq!(get_utf8_length(0xC2), Some(byte_offset(2)));

        // Out-of-range 4-byte start bytes (0xF5..=0xF7) rejected early.
        assert_eq!(get_utf8_length(0xF4), Some(byte_offset(4)));
        assert_eq!(get_utf8_length(0xF5), None);
        assert_eq!(get_utf8_length(0xF6), None);
        assert_eq!(get_utf8_length(0xF7), None);
    }

    #[test]
    fn test_overlong_sequence_rejection() {
        // Overlong 2-byte NUL (0xC0 0x80) is illegal in UTF-8 (RFC 3629).
        // 0xC0 is rejected early by try_get_complete_utf8_len / get_utf8_length.
        let buffer_overlong_nul = &[0xC0, 0x80];
        assert!(
            parse_utf8_text(buffer_overlong_nul).is_none(),
            "Should reject overlong 2-byte NUL sequence"
        );

        // Overlong 2-byte '/' (0xC0 0xAF) is illegal in UTF-8.
        let buffer_overlong_slash = &[0xC0, 0xAF];
        assert!(
            parse_utf8_text(buffer_overlong_slash).is_none(),
            "Should reject overlong 2-byte slash sequence"
        );
    }
}
