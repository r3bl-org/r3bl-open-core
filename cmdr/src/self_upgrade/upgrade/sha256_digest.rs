// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use sha2::{Digest, Sha256};
use std::fmt::{Display, Formatter};

/// A stack-allocated 32-byte (256-bit) SHA-256 digest, valid upon construction.
///
/// Once constructed, it doesn't need to be re-validated. You can construct it in two
/// ways:
/// 1. Hash a byte slice via [`Self::from_bytes()`].
/// 2. Parse a string slice containing a 64-character hex SHA-256 digest (e.g., from a
///    `.sha256` file) via [`Self::try_parse_hex()`].
///
/// Equality comparisons operate directly on the 32 bytes on the stack (`inner` field)
/// without heap allocation.
///
/// Hex formatting is done via [`Display`], which is used when rendering output (e.g.,
/// errors or tests), not during comparisons.
///
/// [`Display`]: std::fmt::Display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sha256Digest {
    inner: [u8; constants::DIGEST_BYTES],
}

impl Sha256Digest {
    /// Computes the SHA-256 digest of `bytes` on the stack.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            inner: Sha256::digest(bytes).into(),
        }
    }

    /// Parses a bare 64-character hexadecimal SHA-256 digest from a `.sha256` file.
    ///
    /// Expects `<64-hex>` with optional surrounding [`ASCII`] whitespace (for example a
    /// trailing newline), parsed case-insensitively. Extra tokens such as trailing
    /// filenames are rejected.
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    #[must_use]
    pub fn try_parse_hex(checksum_text: &str) -> Option<Self> {
        // Trim leading and trailing whitespace (including `\n` and `\r\n`).
        let hex = checksum_text.trim_ascii();
        let (pairs, remainder) = hex.as_bytes().as_chunks::<2>();
        if pairs.len() != constants::DIGEST_BYTES || !remainder.is_empty() {
            return None;
        }
        let mut digest = [0u8; constants::DIGEST_BYTES];
        for (dst, &[hi_hex, lo_hex]) in digest.iter_mut().zip(pairs) {
            let hi = Self::decode_hex_nibble(hi_hex)?;
            let lo = Self::decode_hex_nibble(lo_hex)?;
            *dst = (hi << constants::HIGH_NIBBLE_SHIFT) | lo;
        }
        Some(Self { inner: digest })
    }

    /// Decodes a single [`ASCII`] hexadecimal byte into its 4-bit value (`0..=15`).
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    #[must_use]
    const fn decode_hex_nibble(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + constants::HEX_ALPHA_OFFSET),
            b'A'..=b'F' => Some(byte - b'A' + constants::HEX_ALPHA_OFFSET),
            _ => None,
        }
    }

    /// Converts a 4-bit value, `0..=15` (decimal), into its lowercase [`ASCII`] hex byte
    /// representation: `'0'..='9'` or `'a'..='f'`.
    ///
    /// In the [`ASCII`] table, the decimal `u8` code values for `'0'..='9'` are `48..=57`,
    /// followed by 39 other characters (punctuation and uppercase letters, `58..=96`)
    /// before `'a'..='f'` at `97..=102`. So for any masked value in `0..=15`:
    /// - Add `b'0'` (`48`): values `0..=9` land directly on `'0'..='9'` (`48..=57`).
    /// - If the value is `10..=15`, add `39` more so `10` lands on `'a'` (`10 + 48 + 39 =
    ///   97`) through `'f'` (`15 + 48 + 39 = 102`).
    ///
    /// Multiplying `u8::from(masked >= 10)` by `39` performs that jump without an
    /// `if`/`match` branch or lookup table.
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    #[must_use]
    #[inline]
    fn encode_hex_nibble(nibble: u8) -> u8 {
        let masked = nibble & constants::LOW_NIBBLE_MASK;
        masked
            + b'0'
            + u8::from(masked >= constants::HEX_ALPHA_OFFSET)
                * constants::HEX_ALPHA_ASCII_SHIFT
    }
}

/// Formats the 32 byte digest as 64 lowercase hex chars.
///
/// Each raw `u8` byte (8 bits) splits into two 4-bit nibbles, producing 2 hex characters
/// (`32 bytes * 2 = 64 chars`), e.g. `10` (dec) becomes `0A`:
///
/// ```text
/// ┌───────────────────────────────────────┐
/// │          1 raw byte (8 bits)          │           byte      >> 4 = high nibble
/// │   10 (dec) / 0A (hex) = 0b0000_1010   │           0000_1010 >> 4 = 0000_0000
/// ├───────────────────┬───────────────────┤                          = 0
/// │ High nibble (4b)  │  Low nibble (4b)  │
/// │ bits 7..=4: 0000  │ bits 3..=0: 1010  │           byte & 0x0F      = low nibble
/// │   byte >> 4 = 0   │ byte & 0x0F = 10  │           0000_1010 & 0x0F = 0000_1010
/// └─────────┬─────────┴─────────┬─────────┘                            = 10
///           ▼                   ▼
///       ┌───────┐           ┌───────┐
///       │  '0'  │           │  'a'  │   ──► "{byte:02x}" writes "0a"
///       └───────┘           └───────┘
/// ```
///
/// This impl does not incur costs from heap allocations, formatting, `write!()` macro
/// calls. It just writes the raw bytes to the formatter. And it can be SIMD vectorized by
/// the compiler.
impl Display for Sha256Digest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut output_bytes = [0u8; constants::DIGEST_BYTES * 2];

        let (pairs, _) = output_bytes.as_chunks_mut::<2>();
        for ([hi_dst, lo_dst], byte) in pairs.iter_mut().zip(self.inner) {
            let high_nibble = byte >> constants::HIGH_NIBBLE_SHIFT;
            let low_nibble = byte & constants::LOW_NIBBLE_MASK;
            *hi_dst = Self::encode_hex_nibble(high_nibble);
            *lo_dst = Self::encode_hex_nibble(low_nibble);
        }

        // SAFETY: Every byte written by `encode_hex_nibble` is 7-bit ASCII
        // (`'0'..='9'` or `'a'..='f'`), so all 64 bytes are valid UTF-8.
        let hex_str = unsafe { std::str::from_utf8_unchecked(&output_bytes) };
        f.write_str(hex_str)
    }
}

/// Constants.
mod constants {
    /// Byte length of a raw SHA-256 digest (`256` bits / `8` bits per byte = `32`).
    pub const DIGEST_BYTES: usize = 32;

    /// Bit shift (`4` bits) for extracting or packing the high hexadecimal nibble.
    pub const HIGH_NIBBLE_SHIFT: u8 = 4;

    /// Bitmask (`0b0000_1111` / `0x0F`) for isolating the low 4-bit hexadecimal nibble.
    pub const LOW_NIBBLE_MASK: u8 = 0b0000_1111;

    /// Decimal value offset (`10`) for hexadecimal digits `'a'..='f'` and `'A'..='F'`.
    pub const HEX_ALPHA_OFFSET: u8 = 10;

    /// Extra [`ASCII`] offset (`39`) between `'9' + 1` (`58`) and `'a'` (`97`) for
    /// nibbles `10..=15`.
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    pub const HEX_ALPHA_ASCII_SHIFT: u8 = b'a' - b'0' - HEX_ALPHA_OFFSET;
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3bl_tui::assert_eq2;

    #[test]
    fn test_parse_hex() {
        let expected_hex =
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let with_newline = format!("{expected_hex}\n");
        let parsed = Sha256Digest::try_parse_hex(&with_newline).unwrap();
        assert_eq2!(parsed.to_string(), expected_hex);

        let upper_crlf =
            "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855\r\n";
        let from_upper = Sha256Digest::try_parse_hex(upper_crlf).unwrap();
        assert_eq2!(from_upper, parsed);
        assert_eq2!(from_upper.to_string(), expected_hex);

        // Reject trailing filename or extra tokens.
        let with_filename = format!(
            "{expected_hex}  r3bl-cmdr-v0.0.25-x86_64-unknown-linux-gnu.tar.gz\n"
        );
        assert!(Sha256Digest::try_parse_hex(&with_filename).is_none());

        assert!(Sha256Digest::try_parse_hex("not_a_valid_sha").is_none());
        assert!(Sha256Digest::try_parse_hex("").is_none());
        assert!(Sha256Digest::try_parse_hex("   \n").is_none());
        // Odd length (63 and 65 hex characters).
        assert!(Sha256Digest::try_parse_hex(&expected_hex[..63]).is_none());
        assert!(Sha256Digest::try_parse_hex(&format!("{expected_hex}a")).is_none());
        // 64 characters with a non-hex byte ('g').
        let non_hex_64 =
            "g3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert!(Sha256Digest::try_parse_hex(non_hex_64).is_none());
    }

    #[test]
    fn test_from_bytes() {
        // Known SHA-256 of empty byte array:
        // e3b0c442 98fc1c14 9afbf4c8 996fb924 27ae41e4 649b934c a495991b 7852b855
        #[rustfmt::skip]
        let expected_inner: [u8; constants::DIGEST_BYTES] = [
            0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14,
            0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24,
            0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c,
            0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55,
        ];
        let empty_digest = Sha256Digest::from_bytes(b"");
        assert_eq2!(empty_digest.inner, expected_inner);
    }

    #[test]
    fn test_display() {
        let inner: [u8; constants::DIGEST_BYTES] = [
            0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A,
            0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A,
            0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A,
        ];
        let digest = Sha256Digest { inner };
        let output = format!("Digest: {digest}");
        let expected_output =
            "Digest: 0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a";
        print!("output: {output}\nexpected_output: {expected_output}\n");
        assert_eq2!(output, expected_output);
    }
}
