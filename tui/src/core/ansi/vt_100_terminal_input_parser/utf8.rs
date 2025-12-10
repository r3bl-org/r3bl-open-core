// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words fullwidth multibyte

//! UTF-8 text parsing between ANSI sequences.
//!
//! This module handles conversion of raw UTF-8 bytes (received as regular text input
//! between ANSI escape sequences) into keyboard events representing typed characters.
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
//! │  utf8.rs                                 ◀──┤ **YOU ARE HERE** │
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
//! - Single-byte UTF-8 characters (ASCII)
//! - Multi-byte UTF-8 sequences (2-4 bytes)
//! - Incomplete UTF-8 sequences (buffering)
//! - Invalid UTF-8 sequences (graceful error handling)
//!
//! ## UTF-8 Encoding Explained
//!
//! UTF-8 uses **bit pattern matching** (not arithmetic) to identify byte types
//! and extract data. The high bits are structural markers; remaining bits carry
//! the Unicode code point.
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
//! ### Continuation Bytes: 10xxxxxx
//!
//! Each continuation byte:
//! - Prefix `10` marks it as "not a start byte" (enables self-synchronization)
//! - Remaining 6 bits carry data
//! - Extract with: `byte & 0x3F`
//!
//! ### Worked Example: 'é' (U+00E9 = 233)
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
//!         = 0xE9 = 233 ✓
//! ```
//!
//! ### Why Bit Patterns (Not Arithmetic)?
//!
//! - **Fast**: Detection is just bitwise AND + compare
//! - **Self-synchronizing**: Jump anywhere in a stream, scan for `0xxxxxxx` or `11xxxxxx`
//! - **Unambiguous**: Continuation bytes (`10xxxxxx`) can never be confused with start bytes
//! - **ASCII-compatible**: Single-byte chars unchanged (the `0` prefix means "complete")
//!
//! # Important: UTF-8 Byte Length vs Display Width
//!
//! This module handles **UTF-8 byte-level parsing only** - converting raw bytes
//! from terminal input into Unicode characters. It does NOT handle display width.
//!
//! ## Two Separate Concerns
//!
//! | Concern                                | What it measures            | Example: '😀'    |
//! |:---------------------------------------|:----------------------------|:-----------------|
//! | **UTF-8 byte length** (this module)    | Memory size in bytes        | 4 bytes          |
//! | **Display width** (graphemes module)   | Terminal columns occupied   | 2 columns        |
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
//!
//! [`VT100KeyCodeIR::Char`]: super::VT100KeyCodeIR::Char
//! [`keyboard`]: mod@super::keyboard
//! [`mouse`]: mod@super::mouse
//! [`router`]: mod@super::router
//! [`terminal_events`]: mod@super::terminal_events
//! [parent module documentation]: mod@super#primary-consumer
//! [`convert_input_event()`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::protocol_conversion::convert_input_event

use super::ir_event_types::{VT100InputEventIR, VT100KeyCodeIR, VT100KeyModifiersIR};
use crate::{ByteOffset, UTF8_1BYTE_MAX, UTF8_1BYTE_MIN, UTF8_2BYTE_FIRST_MASK,
            UTF8_2BYTE_MAX, UTF8_2BYTE_MIN, UTF8_3BYTE_FIRST_MASK, UTF8_3BYTE_MAX,
            UTF8_3BYTE_MIN, UTF8_4BYTE_FIRST_MASK, UTF8_4BYTE_MAX, UTF8_4BYTE_MIN,
            UTF8_CONTINUATION_DATA_MASK, UTF8_CONTINUATION_MASK,
            UTF8_CONTINUATION_PATTERN, byte_offset};

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
/// # Important: `bytes_consumed` ≠ display width
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
pub fn parse_utf8_text(buffer: &[u8]) -> Option<(VT100InputEventIR, ByteOffset)> {
    // Check if we have a complete UTF-8 sequence
    let bytes_consumed = is_utf8_complete(buffer)?;

    // Decode the complete UTF-8 sequence
    let ch = decode_utf8(buffer)?;

    // Return keyboard event with the decoded character
    Some((
        VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Char(ch),
            modifiers: VT100KeyModifiersIR::default(),
        },
        byte_offset(bytes_consumed),
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
        if (byte & UTF8_CONTINUATION_MASK) != UTF8_CONTINUATION_PATTERN {
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
        UTF8_1BYTE_MIN..=UTF8_1BYTE_MAX => u32::from(first_byte),

        // 2-byte sequence: 110xxxxx 10xxxxxx
        UTF8_2BYTE_MIN..=UTF8_2BYTE_MAX => {
            if buffer.len() < 2 {
                return None;
            }
            let b1 = u32::from(first_byte & UTF8_2BYTE_FIRST_MASK);
            let b2 = u32::from(buffer[1] & UTF8_CONTINUATION_DATA_MASK);
            (b1 << 6) | b2
        }

        // 3-byte sequence: 1110xxxx 10xxxxxx 10xxxxxx
        UTF8_3BYTE_MIN..=UTF8_3BYTE_MAX => {
            if buffer.len() < 3 {
                return None;
            }
            let b1 = u32::from(first_byte & UTF8_3BYTE_FIRST_MASK);
            let b2 = u32::from(buffer[1] & UTF8_CONTINUATION_DATA_MASK);
            let b3 = u32::from(buffer[2] & UTF8_CONTINUATION_DATA_MASK);
            (b1 << 12) | (b2 << 6) | b3
        }

        // 4-byte sequence: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
        UTF8_4BYTE_MIN..=UTF8_4BYTE_MAX => {
            if buffer.len() < 4 {
                return None;
            }
            let b1 = u32::from(first_byte & UTF8_4BYTE_FIRST_MASK);
            let b2 = u32::from(buffer[1] & UTF8_CONTINUATION_DATA_MASK);
            let b3 = u32::from(buffer[2] & UTF8_CONTINUATION_DATA_MASK);
            let b4 = u32::from(buffer[3] & UTF8_CONTINUATION_DATA_MASK);
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
        UTF8_1BYTE_MIN..=UTF8_1BYTE_MAX => Some(1),
        // Start byte for 2-byte sequence (110xxxxx)
        UTF8_2BYTE_MIN..=UTF8_2BYTE_MAX => Some(2),
        // Start byte for 3-byte sequence (1110xxxx)
        UTF8_3BYTE_MIN..=UTF8_3BYTE_MAX => Some(3),
        // Start byte for 4-byte sequence (11110xxx)
        UTF8_4BYTE_MIN..=UTF8_4BYTE_MAX => Some(4),
        // Continuation byte (10xxxxxx) - invalid as start byte
        // Reserved/invalid bytes (11111xxx)
        _ => None,
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
        // Test parsing multiple ASCII characters sequentially
        let buffer = b"hello";

        // Parse 'h'
        let (event, consumed) = parse_utf8_text(buffer).expect("Should parse first char");
        assert_eq!(consumed, byte_offset(1));
        match event {
            VT100InputEventIR::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCodeIR::Char('h'));
            }
            _ => panic!("Expected Keyboard event"),
        }

        // Parse 'e' from remainder
        let (event, consumed) =
            parse_utf8_text(&buffer[1..]).expect("Should parse second char");
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
        // Two-byte character: '©' (0xC2 0xA9)
        let buffer = b"\xC2\xA9";
        let (event, consumed) =
            parse_utf8_text(buffer).expect("Should parse 2-byte UTF-8");

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
        // Three-byte character: '€' (0xE2 0x82 0xAC)
        let buffer = b"\xE2\x82\xAC";
        let (event, consumed) =
            parse_utf8_text(buffer).expect("Should parse 3-byte UTF-8");

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
        // Four-byte character: '😀' (0xF0 0x9F 0x98 0x80)
        let buffer = b"\xF0\x9F\x98\x80";
        let (event, consumed) =
            parse_utf8_text(buffer).expect("Should parse 4-byte UTF-8");

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
        assert_eq!(consumed, byte_offset(1));
        match event {
            VT100InputEventIR::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCodeIR::Char('a'));
            }
            _ => panic!("Expected Keyboard event"),
        }

        // Parse 2-byte '©'
        let (event, consumed) =
            parse_utf8_text(&buffer[1..]).expect("Should parse 2-byte");
        assert_eq!(consumed, byte_offset(2));
        match event {
            VT100InputEventIR::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCodeIR::Char('©'));
            }
            _ => panic!("Expected Keyboard event"),
        }

        // Parse ASCII 'b'
        let (event, consumed) =
            parse_utf8_text(&buffer[3..]).expect("Should parse ASCII");
        assert_eq!(consumed, byte_offset(1));
        match event {
            VT100InputEventIR::Keyboard { code, .. } => {
                assert_eq!(code, VT100KeyCodeIR::Char('b'));
            }
            _ => panic!("Expected Keyboard event"),
        }
    }
}
