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

use super::types::{InputEvent, KeyCode, KeyModifiers};

/// Parse UTF-8 text and return a single InputEvent for the first complete character.
///
/// Returns `Some((event, bytes_consumed))` if a complete UTF-8 character is parsed,
/// or `None` if the sequence is incomplete or invalid.
///
/// Converts raw UTF-8 bytes into character input events. Handles multi-byte
/// UTF-8 sequences (1-4 bytes). If the buffer starts with incomplete UTF-8,
/// returns `None` to indicate more bytes are needed.
///
/// The caller (DirectToAnsiInputDevice) can call this repeatedly to parse
/// multiple characters from the buffer.
pub fn parse_utf8_text(buffer: &[u8]) -> Option<(InputEvent, usize)> {
    // Check if we have a complete UTF-8 sequence
    let bytes_consumed = is_utf8_complete(buffer)?;

    // Decode the complete UTF-8 sequence
    let ch = decode_utf8(buffer)?;

    // Return keyboard event with the decoded character
    Some((
        InputEvent::Keyboard {
            code: KeyCode::Char(ch),
            modifiers: KeyModifiers::default(),
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
    for i in 1..required_len {
        let byte = buffer[i];
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
        0x00..=0x7F => first_byte as u32,

        // 2-byte sequence: 110xxxxx 10xxxxxx
        0xC0..=0xDF => {
            if buffer.len() < 2 {
                return None;
            }
            let b1 = (first_byte & 0x1F) as u32;
            let b2 = (buffer[1] & 0x3F) as u32;
            (b1 << 6) | b2
        }

        // 3-byte sequence: 1110xxxx 10xxxxxx 10xxxxxx
        0xE0..=0xEF => {
            if buffer.len() < 3 {
                return None;
            }
            let b1 = (first_byte & 0x0F) as u32;
            let b2 = (buffer[1] & 0x3F) as u32;
            let b3 = (buffer[2] & 0x3F) as u32;
            (b1 << 12) | (b2 << 6) | b3
        }

        // 4-byte sequence: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
        0xF0..=0xF7 => {
            if buffer.len() < 4 {
                return None;
            }
            let b1 = (first_byte & 0x07) as u32;
            let b2 = (buffer[1] & 0x3F) as u32;
            let b3 = (buffer[2] & 0x3F) as u32;
            let b4 = (buffer[3] & 0x3F) as u32;
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
/// Returns the total byte length of the UTF-8 character, or None if the byte is invalid.
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
        0x80..=0xBF => None,
        // Reserved/invalid bytes (11111xxx)
        0xF8..=0xFF => None,
    }
}

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
            InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, KeyCode::Char('a'));
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
            InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, KeyCode::Char('h'));
            }
            _ => panic!("Expected Keyboard event"),
        }

        // Parse 'e' from remainder
        let (event, consumed) = parse_utf8_text(&buffer[1..]).expect("Should parse second char");
        assert_eq!(consumed, 1);
        match event {
            InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, KeyCode::Char('e'));
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_two_byte_utf8() {
        // Two-byte character: '©' (0xC2 0xA9)
        let buffer = b"\xC2\xA9";
        let (event, consumed) = parse_utf8_text(buffer).expect("Should parse 2-byte UTF-8");

        assert_eq!(consumed, 2);
        match event {
            InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, KeyCode::Char('©'));
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_three_byte_utf8() {
        // Three-byte character: '€' (0xE2 0x82 0xAC)
        let buffer = b"\xE2\x82\xAC";
        let (event, consumed) = parse_utf8_text(buffer).expect("Should parse 3-byte UTF-8");

        assert_eq!(consumed, 3);
        match event {
            InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, KeyCode::Char('€'));
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_four_byte_utf8() {
        // Four-byte character: '😀' (0xF0 0x9F 0x98 0x80)
        let buffer = b"\xF0\x9F\x98\x80";
        let (event, consumed) = parse_utf8_text(buffer).expect("Should parse 4-byte UTF-8");

        assert_eq!(consumed, 4);
        match event {
            InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, KeyCode::Char('😀'));
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_incomplete_two_byte_sequence() {
        // Incomplete 2-byte sequence: only first byte
        let buffer = b"\xC2";
        let result = parse_utf8_text(buffer);
        assert!(result.is_none(), "Should not parse incomplete 2-byte sequence");
    }

    #[test]
    fn test_incomplete_three_byte_sequence() {
        // Incomplete 3-byte sequence: only first two bytes
        let buffer = b"\xE2\x82";
        let result = parse_utf8_text(buffer);
        assert!(result.is_none(), "Should not parse incomplete 3-byte sequence");
    }

    #[test]
    fn test_incomplete_four_byte_sequence() {
        // Incomplete 4-byte sequence: only first three bytes
        let buffer = b"\xF0\x9F\x98";
        let result = parse_utf8_text(buffer);
        assert!(result.is_none(), "Should not parse incomplete 4-byte sequence");
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
            InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, KeyCode::Char('a'));
            }
            _ => panic!("Expected Keyboard event"),
        }

        // Parse 2-byte '©'
        let (event, consumed) = parse_utf8_text(&buffer[1..]).expect("Should parse 2-byte");
        assert_eq!(consumed, 2);
        match event {
            InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, KeyCode::Char('©'));
            }
            _ => panic!("Expected Keyboard event"),
        }

        // Parse ASCII 'b'
        let (event, consumed) = parse_utf8_text(&buffer[3..]).expect("Should parse ASCII");
        assert_eq!(consumed, 1);
        match event {
            InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, KeyCode::Char('b'));
            }
            _ => panic!("Expected Keyboard event"),
        }
    }
}
