// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! UTF-8 text parsing between ANSI sequences.
//!
//! This module handles conversion of raw UTF-8 bytes (received as regular text input
//! between ANSI escape sequences) into keyboard events representing typed characters.
//!
//! Handles:
//! - Single-byte UTF-8 characters (ASCII)
//! - Multi-byte UTF-8 sequences (2-4 bytes)
//! - Incomplete UTF-8 sequences (buffering)
//! - Invalid UTF-8 sequences (graceful error handling)
//!
//! # Important: UTF-8 Byte Length vs Display Width
//!
//! This module handles **UTF-8 byte-level parsing only** - converting raw bytes
//! from terminal input into Unicode characters. It does NOT handle display width.
//!
//! ## Two Separate Concerns
//!
//! | Concern                              | What it measures          | Example: '😀'  |
//! |--------------------------------------|---------------------------|----------------|
//! | **UTF-8 byte length** (this module)  | Memory size in bytes      | 4 bytes        |
//! | **Display width** (graphemes module) | Terminal columns occupied | 2 columns      |
//!
//! - **This module**: Returns `(InputEvent, bytes_consumed)` where `bytes_consumed` is
//!   the number of bytes to advance in the input buffer (1-4 bytes for UTF-8).
//! - **Display rendering**: Calculated separately using the `unicode_width` crate. See
//!   [`mod@crate::graphemes`] for comprehensive documentation on Unicode display width,
//!   grapheme clusters, and the three types of indices (`ByteIndex`, `SegIndex`,
//!   `ColIndex`).
//!
//! ## Why This Matters
//!
//! A common mistake is assuming that `bytes_consumed` relates to how many terminal
//! columns the character occupies. This is incorrect:
//!
//! ```text
//! Character  UTF-8 Bytes  Display Width
//! ---------  -----------  -------------
//! 'H'        1 byte       1 column  (ASCII)
//! '©'        2 bytes      1 column  (Latin-1 supplement)
//! '€'        3 bytes      1 column  (currency symbol)
//! '你'       3 bytes      2 columns (CJK fullwidth)
//! '😀'       4 bytes      2 columns (emoji)
//! ```
//!
//! If you need to position the cursor or calculate line lengths, you need display
//! width calculation, not byte length. See [`crate::graphemes::GCStringOwned`] for
//! text rendering utilities.

use super::types::{VT100InputEvent, VT100KeyCode, VT100KeyModifiers};

/// Parse UTF-8 text and return a single `InputEvent` for the first complete character.
///
/// Returns `Some((event, bytes_consumed))` if a complete UTF-8 character is parsed,
/// or `None` if the sequence is incomplete or invalid.
///
/// Converts raw UTF-8 bytes into character input events. Handles multi-byte
/// UTF-8 sequences (1-4 bytes). If the buffer starts with incomplete UTF-8,
/// returns `None` to indicate more bytes are needed.
///
/// The caller (`DirectToAnsiInputDevice`) can call this repeatedly to parse
/// multiple characters from the buffer.
///
/// # Important: bytes_consumed ≠ display width
///
/// The returned `bytes_consumed` indicates how many bytes to advance in the input
/// buffer. This is **NOT** the display width (terminal columns) of the character.
///
/// Example:
/// - '😀' returns `bytes_consumed = 4` (UTF-8 encoding is 4 bytes)
/// - But '😀' occupies **2 terminal columns** (display width)
///
/// For display width calculation and cursor positioning, see [`mod@crate::graphemes`].
#[must_use]
pub fn parse_utf8_text(buffer: &[u8]) -> Option<(VT100InputEvent, usize)> {
    // Check if we have a complete UTF-8 sequence
    let bytes_consumed = is_utf8_complete(buffer)?;

    // Decode the complete UTF-8 sequence
    let ch = decode_utf8(buffer)?;

    // Return keyboard event with the decoded character
    Some((
        VT100InputEvent::Keyboard {
            code: VT100KeyCode::Char(ch),
            modifiers: VT100KeyModifiers::default(),
        },
        bytes_consumed,
    ))
}

/// Check if a UTF-8 byte sequence is complete.
///
/// Returns:
/// - `Some(len)` if complete, where `len` is the byte length of the character
/// - `None` if the sequence is incomplete and needs more bytes
fn is_utf8_complete(buffer: &[u8]) -> Option<usize> {
    if buffer.is_empty() {
        return None;
    }

    let first_byte = buffer[0];
    let required_len = get_utf8_length(first_byte)?;

    // Check if we have enough bytes in the buffer
    if buffer.len() < required_len {
        return None; // Incomplete sequence
    }

    // Verify all continuation bytes are correctly formatted
    for byte in buffer.iter().skip(1).take(required_len - 1) {
        // Continuation bytes must be 10xxxxxx (0x80-0xBF)
        if (byte & 0xC0) != 0x80 {
            return None; // Invalid continuation byte
        }
    }

    Some(required_len)
}

/// Validate and decode a complete UTF-8 sequence.
///
/// Returns the decoded character if valid, or None if the sequence is invalid.
///
/// Decodes UTF-8 by extracting the data bits from each byte:
/// - 1-byte: 0xxxxxxx
/// - 2-byte: 110xxxxx 10xxxxxx
/// - 3-byte: 1110xxxx 10xxxxxx 10xxxxxx
/// - 4-byte: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
fn decode_utf8(buffer: &[u8]) -> Option<char> {
    if buffer.is_empty() {
        return None;
    }

    let first_byte = buffer[0];

    let codepoint = match first_byte {
        // 1-byte sequence: 0xxxxxxx
        0x00..=0x7F => u32::from(first_byte),

        // 2-byte sequence: 110xxxxx 10xxxxxx
        0xC0..=0xDF => {
            if buffer.len() < 2 {
                return None;
            }
            let b1 = u32::from(first_byte & 0x1F);
            let b2 = u32::from(buffer[1] & 0x3F);
            (b1 << 6) | b2
        }

        // 3-byte sequence: 1110xxxx 10xxxxxx 10xxxxxx
        0xE0..=0xEF => {
            if buffer.len() < 3 {
                return None;
            }
            let b1 = u32::from(first_byte & 0x0F);
            let b2 = u32::from(buffer[1] & 0x3F);
            let b3 = u32::from(buffer[2] & 0x3F);
            (b1 << 12) | (b2 << 6) | b3
        }

        // 4-byte sequence: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
        0xF0..=0xF7 => {
            if buffer.len() < 4 {
                return None;
            }
            let b1 = u32::from(first_byte & 0x07);
            let b2 = u32::from(buffer[1] & 0x3F);
            let b3 = u32::from(buffer[2] & 0x3F);
            let b4 = u32::from(buffer[3] & 0x3F);
            (b1 << 18) | (b2 << 12) | (b3 << 6) | b4
        }

        // Invalid start byte
        _ => return None,
    };

    // Validate codepoint and convert to char
    char::from_u32(codepoint)
}

/// Get the expected length of a UTF-8 sequence from its first byte.
///
/// This implements the same logic as the unstable `core::str::utf8_char_width`,
/// but uses `Option<usize>` for type-safe error handling. We maintain this
/// custom implementation because:
/// - The std lib version requires nightly Rust (`str_internals` feature)
/// - Our `Option` return type is more explicit than returning 0 for invalid bytes
/// - Zero external dependencies
///
/// Returns the total byte length of the UTF-8 character, or None if invalid.
///
/// # Important: This is NOT the same as `unicode_width`
///
/// This function calculates **UTF-8 byte length** (how many bytes encode the character),
/// NOT **display width** (how many terminal columns it occupies). These are independent:
///
/// - A 3-byte character like '€' occupies **1 column** (narrow)
/// - A 3-byte character like '你' occupies **2 columns** (wide/fullwidth)
/// - Both return `Some(3)` from this function (same byte length)
///
/// For display width calculation, see the `unicode_width` crate used in
/// [`mod@crate::graphemes`]. See also the module-level documentation for a
/// comprehensive explanation of this distinction.
fn get_utf8_length(first_byte: u8) -> Option<usize> {
    match first_byte {
        // ASCII: single byte (0xxxxxxx)
        0x00..=0x7F => Some(1),
        // Start byte for 2-byte sequence (110xxxxx)
        0xC0..=0xDF => Some(2),
        // Start byte for 3-byte sequence (1110xxxx)
        0xE0..=0xEF => Some(3),
        // Start byte for 4-byte sequence (11110xxx)
        0xF0..=0xF7 => Some(4),
        // Continuation byte (10xxxxxx) - invalid as start byte
        // Reserved/invalid bytes (11111xxx)
        0x80..=0xBF | 0xF8..=0xFF => None,
    }
}

/// Unit tests for UTF-8 text parsing.
///
/// These tests use generator functions instead of hardcoded magic strings to ensure
/// consistency between sequence generation and parsing. For testing strategy details,
/// see the [testing strategy] documentation.
///
/// [testing strategy]: mod@super#testing-strategy
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_character() {
        // Single ASCII character: 'a' (0x61)
        let buffer = b"a";
        let (event, consumed) = parse_utf8_text(buffer).expect("Should parse ASCII");

        assert_eq!(consumed, 1);
        match event {
            VT100InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCode::Char('a'));
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_ascii_multiple_chars() {
        // Test parsing multiple ASCII characters sequentially
        let buffer = b"hello";

        // Parse 'h'
        let (event, consumed) = parse_utf8_text(buffer).expect("Should parse first char");
        assert_eq!(consumed, 1);
        match event {
            VT100InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCode::Char('h'));
            }
            _ => panic!("Expected Keyboard event"),
        }

        // Parse 'e' from remainder
        let (event, consumed) =
            parse_utf8_text(&buffer[1..]).expect("Should parse second char");
        assert_eq!(consumed, 1);
        match event {
            VT100InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCode::Char('e'));
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_two_byte_utf8() {
        // Two-byte character: '©' (0xC2 0xA9)
        let buffer = b"\xC2\xA9";
        let (event, consumed) =
            parse_utf8_text(buffer).expect("Should parse 2-byte UTF-8");

        assert_eq!(consumed, 2);
        match event {
            VT100InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCode::Char('©'));
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_three_byte_utf8() {
        // Three-byte character: '€' (0xE2 0x82 0xAC)
        let buffer = b"\xE2\x82\xAC";
        let (event, consumed) =
            parse_utf8_text(buffer).expect("Should parse 3-byte UTF-8");

        assert_eq!(consumed, 3);
        match event {
            VT100InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCode::Char('€'));
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_four_byte_utf8() {
        // Four-byte character: '😀' (0xF0 0x9F 0x98 0x80)
        let buffer = b"\xF0\x9F\x98\x80";
        let (event, consumed) =
            parse_utf8_text(buffer).expect("Should parse 4-byte UTF-8");

        assert_eq!(consumed, 4);
        match event {
            VT100InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCode::Char('😀'));
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_incomplete_two_byte_sequence() {
        // Incomplete 2-byte sequence: only first byte
        let buffer = b"\xC2";
        let result = parse_utf8_text(buffer);
        assert!(
            result.is_none(),
            "Should not parse incomplete 2-byte sequence"
        );
    }

    #[test]
    fn test_incomplete_three_byte_sequence() {
        // Incomplete 3-byte sequence: only first two bytes
        let buffer = b"\xE2\x82";
        let result = parse_utf8_text(buffer);
        assert!(
            result.is_none(),
            "Should not parse incomplete 3-byte sequence"
        );
    }

    #[test]
    fn test_incomplete_four_byte_sequence() {
        // Incomplete 4-byte sequence: only first three bytes
        let buffer = b"\xF0\x9F\x98";
        let result = parse_utf8_text(buffer);
        assert!(
            result.is_none(),
            "Should not parse incomplete 4-byte sequence"
        );
    }

    #[test]
    fn test_invalid_continuation_byte() {
        // Invalid: 2-byte sequence with wrong continuation byte
        // Expected: 0xC2 0xA9, but provide: 0xC2 0x00 (0x00 is not a valid continuation)
        let buffer = b"\xC2\x00";
        let result = parse_utf8_text(buffer);
        assert!(result.is_none(), "Should reject invalid continuation byte");
    }

    #[test]
    fn test_invalid_start_byte_continuation() {
        // Invalid: continuation byte (0x80) at start of buffer
        let buffer = b"\x80hello";
        let result = parse_utf8_text(buffer);
        assert!(result.is_none(), "Should reject continuation byte as start");
    }

    #[test]
    fn test_reserved_byte_value() {
        // Invalid: reserved byte value (0xFF)
        let buffer = b"\xFF";
        let result = parse_utf8_text(buffer);
        assert!(result.is_none(), "Should reject reserved byte");
    }

    #[test]
    fn test_empty_buffer() {
        // Empty buffer
        let buffer = b"";
        let result = parse_utf8_text(buffer);
        assert!(result.is_none(), "Should not parse empty buffer");
    }

    #[test]
    fn test_mixed_ascii_and_multibyte() {
        // Buffer with ASCII followed by multi-byte
        let buffer = b"a\xC2\xA9b";

        // Parse ASCII 'a'
        let (event, consumed) = parse_utf8_text(buffer).expect("Should parse ASCII");
        assert_eq!(consumed, 1);
        match event {
            VT100InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCode::Char('a'));
            }
            _ => panic!("Expected Keyboard event"),
        }

        // Parse 2-byte '©'
        let (event, consumed) =
            parse_utf8_text(&buffer[1..]).expect("Should parse 2-byte");
        assert_eq!(consumed, 2);
        match event {
            VT100InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCode::Char('©'));
            }
            _ => panic!("Expected Keyboard event"),
        }

        // Parse ASCII 'b'
        let (event, consumed) =
            parse_utf8_text(&buffer[3..]).expect("Should parse ASCII");
        assert_eq!(consumed, 1);
        match event {
            VT100InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCode::Char('b'));
            }
            _ => panic!("Expected Keyboard event"),
        }
    }
}
