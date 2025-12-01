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
//! | Bytes | First byte | Continuation bytes | Bit pattern                          |
//! | ----- | ---------- | ------------------ | ------------------------------------ |
//! | 1     | 00-7F      | -                  | 0xxxxxxx                             |
//! | 2     | C0-DF      | 80-BF              | 110xxxxx 10xxxxxx                    |
//! | 3     | E0-EF      | 80-BF (x2)         | 1110xxxx 10xxxxxx 10xxxxxx           |
//! | 4     | F0-F7      | 80-BF (x3)         | 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx  |
//!
//! ## Continuation Byte Validation
//!
//! All bytes after the first must match the pattern `10xxxxxx` (80-BF hex).
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

/// ASCII range: single-byte UTF-8 (00-7F hex).
///
/// Pattern: `0xxxxxxx`
pub const UTF8_1BYTE_MIN: u8 = 0b0000_0000;

/// ASCII range maximum: single-byte UTF-8 (7F hex).
///
/// Pattern: `0xxxxxxx`
pub const UTF8_1BYTE_MAX: u8 = 0b0111_1111;

/// 2-byte UTF-8 sequence start range minimum (C0 hex).
///
/// Pattern: `110xxxxx 10xxxxxx`
pub const UTF8_2BYTE_MIN: u8 = 0b1100_0000;

/// 2-byte UTF-8 sequence start range maximum (DF hex).
///
/// Pattern: `110xxxxx 10xxxxxx`
pub const UTF8_2BYTE_MAX: u8 = 0b1101_1111;

/// 3-byte UTF-8 sequence start range minimum (E0 hex).
///
/// Pattern: `1110xxxx 10xxxxxx 10xxxxxx`
pub const UTF8_3BYTE_MIN: u8 = 0b1110_0000;

/// 3-byte UTF-8 sequence start range maximum (EF hex).
///
/// Pattern: `1110xxxx 10xxxxxx 10xxxxxx`
pub const UTF8_3BYTE_MAX: u8 = 0b1110_1111;

/// 4-byte UTF-8 sequence start range minimum (F0 hex).
///
/// Pattern: `11110xxx 10xxxxxx 10xxxxxx 10xxxxxx`
pub const UTF8_4BYTE_MIN: u8 = 0b1111_0000;

/// 4-byte UTF-8 sequence start range maximum (F7 hex).
///
/// Pattern: `11110xxx 10xxxxxx 10xxxxxx 10xxxxxx`
pub const UTF8_4BYTE_MAX: u8 = 0b1111_0111;

// ============================================================================
// UTF-8 Continuation Bytes (Second, third, fourth bytes)
// ============================================================================

/// Continuation byte range minimum (80 hex).
///
/// All continuation bytes must match pattern `10xxxxxx` (80-BF hex).
pub const UTF8_CONTINUATION_MIN: u8 = 0b1000_0000;

/// Continuation byte range maximum (BF hex).
///
/// All continuation bytes must match pattern `10xxxxxx` (80-BF hex).
pub const UTF8_CONTINUATION_MAX: u8 = 0b1011_1111;

/// Continuation byte validation mask (C0 hex).
///
/// Used to extract the high 2 bits: `byte & 0b1100_0000` should equal
/// `0b1000_0000` for valid continuation bytes.
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
pub const UTF8_CONTINUATION_MASK: u8 = 0b1100_0000;

/// Continuation byte expected pattern (80 hex).
///
/// After masking with [`UTF8_CONTINUATION_MASK`], valid continuation bytes
/// should equal this value.
pub const UTF8_CONTINUATION_PATTERN: u8 = 0b1000_0000;

// ============================================================================
// UTF-8 Reserved/Invalid Bytes
// ============================================================================

/// Reserved byte range minimum (F8 hex).
///
/// Bytes F8-FF hex are invalid UTF-8 start bytes.
pub const UTF8_RESERVED_MIN: u8 = 0b1111_1000;

/// Reserved byte range maximum (FF hex).
///
/// Bytes F8-FF hex are invalid UTF-8 start bytes.
pub const UTF8_RESERVED_MAX: u8 = 0b1111_1111;

// ============================================================================
// UTF-8 Decoding Bit Masks (Extract data bits from each byte)
// ============================================================================

/// 2-byte sequence: first byte data mask (1F hex).
///
/// Extracts lower 5 bits from first byte: `byte & 0b0001_1111`
///
/// Pattern: `110xxxxx` → extract `xxxxx`
pub const UTF8_2BYTE_FIRST_MASK: u8 = 0b0001_1111;

/// Continuation byte data mask (3F hex).
///
/// Extracts lower 6 bits from continuation bytes: `byte & 0b0011_1111`
///
/// Pattern: `10xxxxxx` → extract `xxxxxx`
pub const UTF8_CONTINUATION_DATA_MASK: u8 = 0b0011_1111;

/// 3-byte sequence: first byte data mask (0F hex).
///
/// Extracts lower 4 bits from first byte: `byte & 0b0000_1111`
///
/// Pattern: `1110xxxx` → extract `xxxx`
pub const UTF8_3BYTE_FIRST_MASK: u8 = 0b0000_1111;

/// 4-byte sequence: first byte data mask (07 hex).
///
/// Extracts lower 3 bits from first byte: `byte & 0b0000_0111`
///
/// Pattern: `11110xxx` → extract `xxx`
pub const UTF8_4BYTE_FIRST_MASK: u8 = 0b0000_0111;

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
        #[allow(clippy::erasing_op)]
        {
            assert_ne!(0x00 & UTF8_CONTINUATION_MASK, UTF8_CONTINUATION_PATTERN);
        }
        assert_ne!(0x7F & UTF8_CONTINUATION_MASK, UTF8_CONTINUATION_PATTERN);
        assert_ne!(0xC0 & UTF8_CONTINUATION_MASK, UTF8_CONTINUATION_PATTERN);
        #[allow(clippy::identity_op)]
        {
            assert_ne!(UTF8_CONTINUATION_MASK, UTF8_CONTINUATION_PATTERN);
        }
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
