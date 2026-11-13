// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use r3bl_tui::{fmt_lower_hex_bytes, try_parse_hex_bytes};
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
        try_parse_hex_bytes(hex, [0u8; constants::DIGEST_BYTES])
            .map(|inner| Self { inner })
    }
}

/// Formats the 32 byte digest as 64 lowercase hex chars without heap allocation.
impl Display for Sha256Digest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        fmt_lower_hex_bytes(&self.inner, f)
    }
}

/// Constants.
mod constants {
    /// Byte length of a raw SHA-256 digest (`256` bits / `8` bits per byte = `32`).
    pub const DIGEST_BYTES: usize = 32;
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
