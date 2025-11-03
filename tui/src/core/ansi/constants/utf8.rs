// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! UTF-8 encoding constants for byte-level text parsing.
//!
//! This module provides bit masks, byte ranges, and validation constants used
//! when parsing UTF-8 encoded text from terminal input streams.
//!
//! ## UTF-8 Encoding Structure
//!
//! UTF-8 uses variable-length encoding (1-4 bytes per character):
//!
//! | Bytes | First byte  | Continuation bytes | Bit pattern                               |
//! |-------|-------------|--------------------|-------------------------------------------|
//! | 1     | 0x00-0x7F   | -                  | 0xxxxxxx                                  |
//! | 2     | 0xC0-0xDF   | 0x80-0xBF          | 110xxxxx 10xxxxxx                         |
//! | 3     | 0xE0-0xEF   | 0x80-0xBF (x2)     | 1110xxxx 10xxxxxx 10xxxxxx                |
//! | 4     | 0xF0-0xF7   | 0x80-0xBF (x3)     | 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx       |
//!
//! ## Continuation Byte Validation
//!
//! All bytes after the first must match the pattern `10xxxxxx` (0x80-0xBF).
//! This is validated using [`UTF8_CONTINUATION_MASK`] and [`UTF8_CONTINUATION_PATTERN`]:
//!
//! ```rust
//! # use r3bl_tui::{UTF8_CONTINUATION_MASK, UTF8_CONTINUATION_PATTERN};
//! let byte = 0x9F; // Example continuation byte
//! assert_eq!(byte & UTF8_CONTINUATION_MASK, UTF8_CONTINUATION_PATTERN);
//! ```
//!
//! ## Decoding Process
//!
//! 1. Read first byte to determine sequence length (1-4 bytes)
//! 2. Validate all continuation bytes match `10xxxxxx` pattern
//! 3. Extract data bits using the appropriate mask
//! 4. Combine bits to form the Unicode codepoint
//!
//! ## Usage Example
//!
//! ```rust
//! # use r3bl_tui::{UTF8_1BYTE_MAX, UTF8_2BYTE_MIN, UTF8_2BYTE_MAX};
//! let first_byte = 0xC2; // Start of 2-byte sequence
//!
//! // Determine sequence length
//! let is_2byte = (UTF8_2BYTE_MIN..=UTF8_2BYTE_MAX).contains(&first_byte);
//! assert!(is_2byte);
//! ```

// ============================================================================
// UTF-8 Start Byte Ranges (First byte of sequence)
// ============================================================================

/// ASCII range: single-byte UTF-8 (0x00-0x7F).
///
/// Pattern: `0xxxxxxx`
pub const UTF8_1BYTE_MIN: u8 = 0x00;

/// ASCII range maximum: single-byte UTF-8 (0x7F).
///
/// Pattern: `0xxxxxxx`
pub const UTF8_1BYTE_MAX: u8 = 0x7F;

/// 2-byte UTF-8 sequence start range minimum (0xC0).
///
/// Pattern: `110xxxxx 10xxxxxx`
pub const UTF8_2BYTE_MIN: u8 = 0xC0;

/// 2-byte UTF-8 sequence start range maximum (0xDF).
///
/// Pattern: `110xxxxx 10xxxxxx`
pub const UTF8_2BYTE_MAX: u8 = 0xDF;

/// 3-byte UTF-8 sequence start range minimum (0xE0).
///
/// Pattern: `1110xxxx 10xxxxxx 10xxxxxx`
pub const UTF8_3BYTE_MIN: u8 = 0xE0;

/// 3-byte UTF-8 sequence start range maximum (0xEF).
///
/// Pattern: `1110xxxx 10xxxxxx 10xxxxxx`
pub const UTF8_3BYTE_MAX: u8 = 0xEF;

/// 4-byte UTF-8 sequence start range minimum (0xF0).
///
/// Pattern: `11110xxx 10xxxxxx 10xxxxxx 10xxxxxx`
pub const UTF8_4BYTE_MIN: u8 = 0xF0;

/// 4-byte UTF-8 sequence start range maximum (0xF7).
///
/// Pattern: `11110xxx 10xxxxxx 10xxxxxx 10xxxxxx`
pub const UTF8_4BYTE_MAX: u8 = 0xF7;

// ============================================================================
// UTF-8 Continuation Bytes (Second, third, fourth bytes)
// ============================================================================

/// Continuation byte range minimum (0x80).
///
/// All continuation bytes must match pattern `10xxxxxx` (0x80-0xBF).
pub const UTF8_CONTINUATION_MIN: u8 = 0x80;

/// Continuation byte range maximum (0xBF).
///
/// All continuation bytes must match pattern `10xxxxxx` (0x80-0xBF).
pub const UTF8_CONTINUATION_MAX: u8 = 0xBF;

/// Continuation byte validation mask (0xC0).
///
/// Used to extract the high 2 bits: `byte & 0xC0` should equal `0x80`
/// for valid continuation bytes.
///
/// Example:
/// ```rust
/// # use r3bl_tui::{UTF8_CONTINUATION_MASK, UTF8_CONTINUATION_PATTERN};
/// let valid = 0x9F;
/// assert_eq!(valid & UTF8_CONTINUATION_MASK, UTF8_CONTINUATION_PATTERN);
///
/// let invalid = 0x00;
/// assert_ne!(invalid & UTF8_CONTINUATION_MASK, UTF8_CONTINUATION_PATTERN);
/// ```
pub const UTF8_CONTINUATION_MASK: u8 = 0xC0;

/// Continuation byte expected pattern (0x80).
///
/// After masking with [`UTF8_CONTINUATION_MASK`], valid continuation bytes
/// should equal this value.
pub const UTF8_CONTINUATION_PATTERN: u8 = 0x80;

// ============================================================================
// UTF-8 Reserved/Invalid Bytes
// ============================================================================

/// Reserved byte range minimum (0xF8).
///
/// Bytes 0xF8-0xFF are invalid UTF-8 start bytes.
pub const UTF8_RESERVED_MIN: u8 = 0xF8;

/// Reserved byte range maximum (0xFF).
///
/// Bytes 0xF8-0xFF are invalid UTF-8 start bytes.
pub const UTF8_RESERVED_MAX: u8 = 0xFF;

// ============================================================================
// UTF-8 Decoding Bit Masks (Extract data bits from each byte)
// ============================================================================

/// 2-byte sequence: first byte data mask (0x1F).
///
/// Extracts lower 5 bits from first byte: `byte & 0x1F`
///
/// Pattern: `110xxxxx` → extract `xxxxx`
pub const UTF8_2BYTE_FIRST_MASK: u8 = 0x1F;

/// Continuation byte data mask (0x3F).
///
/// Extracts lower 6 bits from continuation bytes: `byte & 0x3F`
///
/// Pattern: `10xxxxxx` → extract `xxxxxx`
pub const UTF8_CONTINUATION_DATA_MASK: u8 = 0x3F;

/// 3-byte sequence: first byte data mask (0x0F).
///
/// Extracts lower 4 bits from first byte: `byte & 0x0F`
///
/// Pattern: `1110xxxx` → extract `xxxx`
pub const UTF8_3BYTE_FIRST_MASK: u8 = 0x0F;

/// 4-byte sequence: first byte data mask (0x07).
///
/// Extracts lower 3 bits from first byte: `byte & 0x07`
///
/// Pattern: `11110xxx` → extract `xxx`
pub const UTF8_4BYTE_FIRST_MASK: u8 = 0x07;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1byte_ranges() {
        assert_eq!(UTF8_1BYTE_MIN, 0x00);
        assert_eq!(UTF8_1BYTE_MAX, 0x7F);
    }

    #[test]
    fn test_2byte_ranges() {
        assert_eq!(UTF8_2BYTE_MIN, 0xC0);
        assert_eq!(UTF8_2BYTE_MAX, 0xDF);
    }

    #[test]
    fn test_3byte_ranges() {
        assert_eq!(UTF8_3BYTE_MIN, 0xE0);
        assert_eq!(UTF8_3BYTE_MAX, 0xEF);
    }

    #[test]
    fn test_4byte_ranges() {
        assert_eq!(UTF8_4BYTE_MIN, 0xF0);
        assert_eq!(UTF8_4BYTE_MAX, 0xF7);
    }

    #[test]
    fn test_continuation_byte_validation() {
        // Valid continuation bytes (0x80-0xBF)
        assert_eq!(0x80 & UTF8_CONTINUATION_MASK, UTF8_CONTINUATION_PATTERN);
        assert_eq!(0x9F & UTF8_CONTINUATION_MASK, UTF8_CONTINUATION_PATTERN);
        assert_eq!(0xBF & UTF8_CONTINUATION_MASK, UTF8_CONTINUATION_PATTERN);

        // Invalid continuation bytes
        assert_ne!(0x00 & UTF8_CONTINUATION_MASK, UTF8_CONTINUATION_PATTERN);
        assert_ne!(0x7F & UTF8_CONTINUATION_MASK, UTF8_CONTINUATION_PATTERN);
        assert_ne!(0xC0 & UTF8_CONTINUATION_MASK, UTF8_CONTINUATION_PATTERN);
        assert_ne!(0xFF & UTF8_CONTINUATION_MASK, UTF8_CONTINUATION_PATTERN);
    }

    #[test]
    fn test_continuation_ranges() {
        assert_eq!(UTF8_CONTINUATION_MIN, 0x80);
        assert_eq!(UTF8_CONTINUATION_MAX, 0xBF);
    }

    #[test]
    fn test_reserved_ranges() {
        assert_eq!(UTF8_RESERVED_MIN, 0xF8);
        assert_eq!(UTF8_RESERVED_MAX, 0xFF);
    }

    #[test]
    fn test_decoding_masks() {
        assert_eq!(UTF8_2BYTE_FIRST_MASK, 0x1F);
        assert_eq!(UTF8_CONTINUATION_DATA_MASK, 0x3F);
        assert_eq!(UTF8_3BYTE_FIRST_MASK, 0x0F);
        assert_eq!(UTF8_4BYTE_FIRST_MASK, 0x07);
    }

    #[test]
    fn test_2byte_first_mask_extraction() {
        // 0xC2 = 11000010 → extract lower 5 bits = 00010 = 0x02
        assert_eq!(0xC2 & UTF8_2BYTE_FIRST_MASK, 0x02);
    }

    #[test]
    fn test_continuation_data_mask_extraction() {
        // 0xA9 = 10101001 → extract lower 6 bits = 101001 = 0x29
        assert_eq!(0xA9 & UTF8_CONTINUATION_DATA_MASK, 0x29);
    }

    #[test]
    fn test_3byte_first_mask_extraction() {
        // 0xE2 = 11100010 → extract lower 4 bits = 0010 = 0x02
        assert_eq!(0xE2 & UTF8_3BYTE_FIRST_MASK, 0x02);
    }

    #[test]
    fn test_4byte_first_mask_extraction() {
        // 0xF0 = 11110000 → extract lower 3 bits = 000 = 0x00
        assert_eq!(0xF0 & UTF8_4BYTE_FIRST_MASK, 0x00);
    }
}
