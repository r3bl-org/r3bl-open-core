// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Zero-allocation hexadecimal encoding and decoding namespace ([`HexSlice`]) for fixed
//! byte containers and slices.

use crate::ok;
use std::fmt::{Display, Formatter};

/// Zero-cost [`Display`] wrapper and namespace for stack-only hexadecimal encoding and
/// decoding.
///
/// Wrapping a byte slice formats it as compact lowercase hexadecimal characters directly
/// into a [`Formatter`] in 32-byte stack chunks without heap allocation. Stateless
/// nibble, byte, and fixed-buffer parsing helpers live directly on [`HexSlice<'static>`]
/// so callers invoke `HexSlice::encode_byte(b)` or `HexSlice::try_parse_str(s, [0u8;
/// 32])` without unconstrained lifetime inference issues.
///
/// [`Display`]: std::fmt::Display
/// [`Formatter`]: std::fmt::Formatter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HexSlice<'a>(pub &'a [u8]);

impl HexSlice<'static> {
    /// Parses a hexadecimal string directly into a caller-provided byte container
    /// `out_buf` (such as `[u8; N]` or `Vec<u8>`) and returns `Some(out_buf)` on success.
    ///
    /// Since every byte (`u8`) requires 2 [`ASCII`] hex characters (`"00"`..=`"ff"`),
    /// `hex_str` must be **exactly `2 * out_buf.len()` characters long** (for example,
    /// `8` characters for `[0u8; 4]`, or `64` characters for `[0u8; 32]`). Both lowercase
    /// (`'a'..='f'`) and uppercase (`'A'..='F'`) digits are accepted.
    ///
    /// Each 2-character chunk (`hi_hex`, `lo_hex`) decodes its [`ASCII`] nibbles
    /// independently into 4-bit values in bits `3..=0`, then shifts `hi` into the upper
    /// half of the output byte before combining them with bitwise OR, e.g. `"2f"` becomes
    /// `47` (`0x2f`):
    ///
    /// ```text
    /// Input str "2f":    hi_hex = b'2' (50)          lo_hex = b'f' (102)
    ///                          │                           │
    ///                          ▼ decode_nibble             ▼ decode_nibble
    ///                    hi = 0b0000_0010 (2)        lo = 0b0000_1111 (15)
    ///                          │                           │
    ///                          ▼ hi << 4                   │ (already in bits 3..=0)
    ///                         0b0010_0000 (32)            0b0000_1111 (15)
    ///                          └────────────┬──────────────┘
    ///                                       ▼ bitwise OR (|)
    ///                         *out_byte = 0b0010_1111 (47 / 0x2f)
    /// ```
    ///
    /// Returns `None` if `hex_str` does not contain exact valid [`ASCII`] hex pairs
    /// matching the byte length of `out_buf`. Callers that allow surrounding whitespace
    /// should call [`str::trim_ascii()`] before calling this function.
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    #[must_use]
    pub fn try_parse_str<T: AsMut<[u8]>>(
        in_hex_str: &str,
        mut out_buf: T,
    ) -> Option<T> {
        let out_buf_mut = out_buf.as_mut();

        // Expect 2 ASCII hex bytes per output byte (e.g., 64 chars -> 32 raw bytes).
        // Early return if this isn't the case.
        if in_hex_str.len() != out_buf_mut.len().checked_mul(2)? {
            return None;
        }

        let (in_hex_pairs, _rem) = in_hex_str.as_bytes().as_chunks::<2>();

        for (out_byte, &[in_hi_hex, in_lo_hex]) in
            out_buf_mut.iter_mut().zip(in_hex_pairs)
        {
            // Decode each hex digit pair, returning None if either byte is invalid ASCII
            // hex.
            let hi_nibble = codec::decode_nibble(in_hi_hex)?;
            let lo_nibble = codec::decode_nibble(in_lo_hex)?;
            // Pack hi_nibble into upper bits 7..=4 and lo_nibble into lower bits 3..=0 of
            // out_byte.
            *out_byte = (hi_nibble << constants::HIGH_NIBBLE_SHIFT) | lo_nibble;
        }

        Some(out_buf)
    }

    /// Decodes a single [`ASCII`] hexadecimal character byte into its 4-bit nibble value
    /// (`0..=15`), case-insensitively (`'0'..='9'`, `'a'..='f'`, `'A'..='F'`).
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    #[must_use]
    #[inline]
    pub const fn decode_nibble(ascii_byte: u8) -> Option<u8> {
        codec::decode_nibble(ascii_byte)
    }

    /// Encodes a single byte into its two lowercase [`ASCII`] hex character bytes
    /// `[hi, lo]`.
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    #[must_use]
    #[inline]
    pub const fn encode_byte(byte: u8) -> [u8; 2] {
        let high_nibble = byte >> constants::HIGH_NIBBLE_SHIFT;
        let low_nibble = byte & constants::LOW_NIBBLE_MASK;
        [
            codec::encode_nibble(high_nibble),
            codec::encode_nibble(low_nibble),
        ]
    }
}

/// Formats `self.0` as lowercase hexadecimal characters into `f` without heap
/// allocations.
///
/// Each raw `u8` byte (8 bits) splits into two 4-bit nibbles, producing 2 hex characters
/// per input byte, e.g. `10` (dec) becomes `"0a"`:
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
///       │  '0'  │           │  'a'  │   ──► writes "0a"
///       └───────┘           └───────┘
/// ```
///
/// Processes input in 32-byte chunks into a 64-byte stack buffer and writes each chunk
/// with a single [`Formatter::write_str`] call (one call total for `<= 32` bytes), so
/// LLVM can auto-vectorize the branchless nibble loop without `write!()` overhead.
impl Display for HexSlice<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut output_bytes = [0u8; constants::CHUNK_OUTPUT_BYTES];
        for input_chunk in self.0.chunks(constants::CHUNK_INPUT_BYTES) {
            let char_len = input_chunk.len() * 2;
            let out_slice = &mut output_bytes[..char_len];
            let (pairs, _) = out_slice.as_chunks_mut::<2>();
            for ([hi_dst, lo_dst], &byte) in pairs.iter_mut().zip(input_chunk) {
                let [hi_ascii, lo_ascii] = HexSlice::encode_byte(byte);
                *hi_dst = hi_ascii;
                *lo_dst = lo_ascii;
            }
            // SAFETY: Every byte written by `HexSlice::encode_byte` is 7-bit ASCII
            // (`'0'..='9'` or `'a'..='f'`), so all `char_len` bytes are valid UTF-8.
            let hex_str = unsafe { std::str::from_utf8_unchecked(out_slice) };
            f.write_str(hex_str)?;
        }
        ok!()
    }
}

/// Low-level [`ASCII`] hex nibble codec helpers.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
pub mod codec {
    use super::constants;

    /// Converts a single [`ASCII`] hexadecimal character byte into its 4-bit nibble value
    /// (`0..=15`), case-insensitively. This is the inverse of [`encode_nibble`].
    ///
    /// Valid hexadecimal characters live in three separate contiguous [`ASCII`] byte
    /// ranges (decimal digits, uppercase letters, and lowercase letters):
    ///
    /// ```text
    /// ASCII decimal:   48 ... 57  │ 58 ... 64 │  65 ... 70  │ 71 ... 96 │ 97 ... 102
    ///                ─────────────┼───────────┼─────────────┼───────────┼─────────────
    /// Character:      '0' ... '9' │  invalid  │ 'A' ... 'F' │  invalid  │ 'a' ... 'f'
    ///                      ▼                         ▼                         ▼
    /// Decoded u8:      0  ...  9                10  ... 15                10  ... 15
    /// ```
    ///
    /// Subtracting the start of the matched range normalizes the offset to `0..=9` for
    /// digits and `0..=5` for letters (plus `10` to yield `10..=15`). Any byte outside
    /// those three ranges (`< 48`, `58..=64`, `71..=96`, or `> 102`) returns `None`:
    ///
    /// | Input `ascii_byte` | [`ASCII`] Decimal | Formula                  | Output          |
    /// | :----------------- | :---------------- | :----------------------- | :-------------- |
    /// | `b'0'..=b'9'`      | `48..=57`         | `ascii_byte - b'0'`      | `Some(0..=9)`   |
    /// | `b'a'..=b'f'`      | `97..=102`        | `ascii_byte - b'a' + 10` | `Some(10..=15)` |
    /// | `b'A'..=b'F'`      | `65..=70`         | `ascii_byte - b'A' + 10` | `Some(10..=15)` |
    /// | Any other byte     | All other `u8`    | `_ => None`              | `None`          |
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    #[must_use]
    #[inline]
    pub(super) const fn decode_nibble(ascii_byte: u8) -> Option<u8> {
        match ascii_byte {
            b'0'..=b'9' => Some(ascii_byte - b'0'),
            b'a'..=b'f' => Some(ascii_byte - b'a' + constants::HEX_ALPHA_OFFSET),
            b'A'..=b'F' => Some(ascii_byte - b'A' + constants::HEX_ALPHA_OFFSET),
            _ => None,
        }
    }

    /// Converts a 4-bit value, `0..=15` (decimal), into its lowercase [`ASCII`] hex byte
    /// representation: `'0'..='9'` or `'a'..='f'`.
    ///
    /// In [`ASCII`], every character is a `u8` integer, and `'0'..='9'` and `'a'..='f'`
    /// are two separate contiguous blocks with 39 other characters (`58..=96`, uppercase
    /// letters and punctuation) sitting in between:
    ///
    /// ```text
    /// ASCII decimal:  48  49  ...  57 │ 58  59  ...  96 │ 97  98  99  100 101 102
    ///                ─────────────────┼─────────────────┼─────────────────────────
    /// Character:     '0' '1' ... '9'  │ ':' ';' ... '`' │ 'a' 'b' 'c' 'd' 'e' 'f'
    ///                 ▲                ◄── 39 codes ───►  ▲
    ///                 start (+48)                         land here (+39 more)
    /// ```
    ///
    /// How the math works on each byte:
    /// 1. Mask with `0b0000_1111` so `masked` is guaranteed to be in `0..=15` (preventing
    ///    overflow or non-[`ASCII`] output regardless of input).
    /// 2. Add `b'0'` (`48`): values `0..=9` land directly on `'0'..='9'` (`48..=57`),
    ///    while `10..=15` land temporarily on `58..=63`.
    /// 3. Add `39` when `masked >= 10` (and `0` otherwise), shifting letters (`10..=15`)
    ///    across the 39-character gap onto `'a'..='f'` (`97..=102`). Because both arms
    ///    are constants, LLVM lowers the `if`/`else` to a branchless `select` and
    ///    vectorizes across 16 or 32 bytes at once in [SIMD] registers.
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    /// [SIMD]: https://en.wikipedia.org/wiki/SIMD
    #[must_use]
    #[inline]
    pub(super) const fn encode_nibble(nibble: u8) -> u8 {
        let masked = nibble & constants::LOW_NIBBLE_MASK;
        let alpha_jump = if masked >= constants::HEX_ALPHA_OFFSET {
            constants::HEX_ALPHA_ASCII_SHIFT
        } else {
            0
        };
        masked + b'0' + alpha_jump
    }
}

/// Constants.
mod constants {
    /// Stack chunk input size (`32` bytes -> `64` hex output bytes per write call).
    pub const CHUNK_INPUT_BYTES: usize = 32;

    /// Stack buffer size (`64` [`ASCII`] bytes) for a single formatted hex chunk.
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    pub const CHUNK_OUTPUT_BYTES: usize = CHUNK_INPUT_BYTES * 2;

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
    use crate::assert_eq2;

    #[test]
    fn test_nibble_encode_and_decode() {
        for nibble in 0..=15u8 {
            let ascii = codec::encode_nibble(nibble);
            assert_eq2!(HexSlice::decode_nibble(ascii), Some(nibble));
        }
        assert_eq2!(codec::encode_nibble(0xF0), b'0');
        assert_eq2!(codec::encode_nibble(0xFA), b'a');
        assert_eq2!(HexSlice::decode_nibble(b'F'), Some(15));
        assert_eq2!(HexSlice::decode_nibble(b'g'), None);
        assert_eq2!(HexSlice::decode_nibble(b'/'), None);
    }

    #[test]
    fn test_encode_byte() {
        assert_eq2!(HexSlice::encode_byte(0x00), [b'0', b'0']);
        assert_eq2!(HexSlice::encode_byte(0x0a), [b'0', b'a']);
        assert_eq2!(HexSlice::encode_byte(0x1e), [b'1', b'e']);
        assert_eq2!(HexSlice::encode_byte(0xff), [b'f', b'f']);
    }

    #[test]
    fn test_try_parse_str() {
        assert_eq2!(
            HexSlice::try_parse_str("0a1B2cFf", [0u8; 4]),
            Some([0x0a, 0x1b, 0x2c, 0xff])
        );
        assert_eq2!(
            HexSlice::try_parse_str("0a1b", vec![0u8; 2]),
            Some(vec![0x0a, 0x1b])
        );
        assert_eq2!(HexSlice::try_parse_str("", [0u8; 0]), Some([]));
        // Wrong length (too short, too long, odd) or non-hex digits fail.
        assert_eq2!(HexSlice::try_parse_str("0a1b2c", [0u8; 4]), None);
        assert_eq2!(HexSlice::try_parse_str("0a1b2cffe0", [0u8; 4]), None);
        assert_eq2!(HexSlice::try_parse_str("0a1b2cf", [0u8; 4]), None);
        assert_eq2!(HexSlice::try_parse_str("0a1b2cfg", [0u8; 4]), None);
        assert_eq2!(HexSlice::try_parse_str(" 0a1b2cff", [0u8; 4]), None);
    }

    #[test]
    fn test_display_empty_single_chunk_and_multi_chunk() {
        assert_eq2!(HexSlice(b"").to_string(), "");
        assert_eq2!(HexSlice(&[0x00, 0x0a, 0x7f, 0xe3]).to_string(), "000a7fe3");
        // Exercise > 32 input bytes so multiple 64-byte stack chunks execute.
        let input_40 = [0xab_u8; 40];
        let expected_40 = "ab".repeat(40);
        assert_eq2!(HexSlice(&input_40).to_string(), expected_40);
    }
}
