// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Zero-allocation hexadecimal encoding and decoding namespace ([`HexSlice`]) for fixed
//! byte containers and slices.

use crate::{NUL, ok};
use std::fmt::{Display, Formatter};

/// Zero-cost [`Display`] wrapper and namespace for stack-only hexadecimal encoding and
/// decoding.
///
/// Wrapping a byte slice formats it as compact lowercase hexadecimal characters directly
/// into a [`Formatter`] without heap allocation. Stateless nibble, byte, and byte buffer
/// parsing are associated functions on [`HexSlice`].
///
/// [`Display`]: std::fmt::Display
/// [`Formatter`]: std::fmt::Formatter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HexSlice<'a> {
    /// Raw byte slice to format as lowercase hexadecimal.
    pub inner: &'a [u8],
}

impl HexSlice<'static> {
    /// Parses a hexadecimal string `in_hex_str` directly into a caller-provided byte
    /// container `out_buf` (such as `[u8; N]` or `Vec<u8>`) and returns `Some(out_buf)`
    /// on success.
    ///
    /// Since every byte (`u8`) requires 2 [`ASCII`] hex characters (`"00"`..=`"ff"`),
    /// `in_hex_str` must be **exactly `2 * out_buf.as_mut().len()` bytes long** (for
    /// example, `8` characters for `[0u8; 4]`, or `64` characters for `[0u8; 32]`). Both
    /// lowercase (`'a'..='f'`) and uppercase (`'A'..='F'`) digits are accepted.
    ///
    /// Splits `in_hex_str` into 2-byte pairs `[in_hi_hex, in_lo_hex]` and decodes each
    /// pair in place via [`Self::decode_byte`]:
    ///
    /// ```text
    /// in_hex_str = "0a1B2cFf"  (len = 8 == 2 * out_buf.len())
    ///                         │
    ///                         ▼ as_chunks::<2>()
    ///      ┌────────────┬─────┴──────┬────────────┐
    ///      ▼            ▼            ▼            ▼
    /// [b'0', b'a'] [b'1', b'B'] [b'2', b'c'] [b'F', b'f']
    ///      │            │            │            │
    ///      ▼            ▼            ▼            ▼  Self::decode_byte
    ///     0x0a         0x1b         0x2c         0xff
    ///      └────────────┴─────┬──────┴────────────┘
    ///                         ▼ written into out_buf
    ///           Some([0x0a, 0x1b, 0x2c, 0xff])
    /// ```
    ///
    /// Returns `None` if `in_hex_str` does not contain exact valid [`ASCII`] hex pairs
    /// matching the byte length of `out_buf`. Callers that allow surrounding whitespace
    /// should call [`str::trim_ascii()`] before calling this function.
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    /// [`Self::decode_byte`]: Self::decode_byte
    #[must_use]
    pub fn try_parse_str<T: AsMut<[u8]>>(in_hex_str: &str, mut out_buf: T) -> Option<T> {
        let out_buf_mut = out_buf.as_mut();

        // Expect 2 ASCII hex bytes per output byte (e.g., 64 chars -> 32 raw bytes).
        // Early return if this isn't the case.
        if in_hex_str.len()
            != out_buf_mut
                .len()
                .checked_mul(constants::HEX_CHARS_PER_BYTE)?
        {
            return None;
        }

        let (in_hex_pairs, _rem) = in_hex_str
            .as_bytes()
            .as_chunks::<{ constants::HEX_CHARS_PER_BYTE }>();

        for (out_byte, &[in_hi_hex, in_lo_hex]) in
            out_buf_mut.iter_mut().zip(in_hex_pairs)
        {
            // Decode each hex digit pair, returning None if either byte is invalid ASCII
            // hex.
            *out_byte = Self::decode_byte(in_hi_hex, in_lo_hex)?;
        }

        Some(out_buf)
    }

    /// Decodes a pair of [`ASCII`] hexadecimal character bytes `(in_hi_hex, in_lo_hex)`
    /// into a single `u8`, case-insensitively (`'0'..='9'`, `'a'..='f'`, `'A'..='F'`).
    /// This is the inverse of [`Self::encode_byte`].
    ///
    /// Each input byte decodes into a 4-bit nibble (`0..=15`), then `hi_nibble` shifts
    /// into upper bits `7..=4` and combines with `lo_nibble` in bits `3..=0`, e.g.
    /// `(b'2', b'f')` becomes `Some(0x2f)`:
    ///
    /// ```text
    /// in_hi_hex = b'2' (50)          in_lo_hex = b'f' (102)
    ///          │                              │
    ///          ▼ decode_nibble                ▼ decode_nibble
    /// hi_nibble = 0b0000_0010 (2)    lo_nibble = 0b0000_1111 (15)
    ///          │                              │
    ///          ▼ hi_nibble << 4               │ (already in bits 3..=0)
    ///         0b0010_0000 (32)               0b0000_1111 (15)
    ///          └──────────────┬───────────────┘
    ///                         ▼ bitwise OR (|)
    ///               Some(0b0010_1111) (0x2f)
    /// ```
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    /// [`Self::encode_byte`]: Self::encode_byte
    #[must_use]
    #[inline]
    pub const fn decode_byte(in_hi_hex: u8, in_lo_hex: u8) -> Option<u8> {
        let Some(hi_nibble) = codec::decode_nibble(in_hi_hex) else {
            return None;
        };
        let Some(lo_nibble) = codec::decode_nibble(in_lo_hex) else {
            return None;
        };
        // Pack hi_nibble into upper bits 7..=4 and lo_nibble into lower bits 3..=0.
        Some((hi_nibble << constants::HIGH_NIBBLE_SHIFT) | lo_nibble)
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

    /// Encodes a single byte into its two lowercase [`ASCII`] hex character bytes `(hi,
    /// lo)`. This is the inverse of [`Self::decode_byte`].
    ///
    /// Splits `byte` into its upper 4 bits (`byte >> 4`) and lower 4 bits
    /// (`byte & 0x0F`), encoding each nibble into `'0'..='9'` or `'a'..='f'`, e.g.
    /// `0x2f` (`47`) becomes `(b'2', b'f')`:
    ///
    /// ```text
    /// ┌─────────────────────────────────────────┐
    /// │             byte = 0x2f (47)            │
    /// │               0b0010_1111               │
    /// ├────────────────────┬────────────────────┤
    /// │  high_nibble (4b)  │   low_nibble (4b)  │
    /// │  bits 7..=4: 0010  │  bits 3..=0: 1111  │
    /// │  byte >> 4 = 2     │  byte & 0x0F = 15  │
    /// └─────────┬──────────┴──────────┬─────────┘
    ///           ▼ encode_nibble       ▼ encode_nibble
    ///       b'2' (50)             b'f' (102)
    ///           └──────────┬──────────┘
    ///                      ▼
    ///                (b'2', b'f')
    /// ```
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    /// [`Self::decode_byte`]: Self::decode_byte
    #[must_use]
    #[inline]
    pub const fn encode_byte(byte: u8) -> (u8, u8) {
        let high_nibble = byte >> constants::HIGH_NIBBLE_SHIFT;
        let low_nibble = byte & constants::LOW_NIBBLE_MASK;
        (
            codec::encode_nibble(high_nibble),
            codec::encode_nibble(low_nibble),
        )
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
/// │          1 raw byte (8 bits)          │      byte        >> 4   = high nibble
/// │   10 (dec) / 0A (hex) = 0b0000_1010   │      0b0000_1010 >> 4   = 0b0000_0000
/// ├───────────────────┬───────────────────┤                         = 0
/// │ High nibble (4b)  │  Low nibble (4b)  │
/// │ bits 7..=4: 0000  │ bits 3..=0: 1010  │      byte        & 0x0F = low nibble
/// │   byte >> 4 = 0   │ byte & 0x0F = 10  │      0b0000_1010 & 0x0F = 0b0000_1010
/// └─────────┬─────────┴─────────┬─────────┘                         = 10
///           ▼                   ▼
///       ┌───────┐           ┌───────┐
///       │  '0'  │           │  'a'  │   ──► writes "0a"
///       └───────┘           └───────┘
/// ```
///
/// Processes `self.inner` in 32-byte chunks into a single reusable 64-byte stack buffer
/// via [`HexSlice::encode_byte`], flushing each chunk with one [`Formatter::write_str`]
/// call (one call total when `self.inner.len() <= 32`):
///
/// ```text
/// self.inner: &[u8]  (arbitrary length, e.g. 40 bytes)
///       │
///       ▼ .chunks(32)               (CHUNK_INPUT_BYTES = 32)
/// input_chunk: &[u8]  (N <= 32 bytes: chunk 1 = 32 B, chunk 2 = 8 B)
///       │
///       ▼ HexSlice::encode_byte     (reuses stack [u8; 64] buffer)
/// out_slice: &mut [u8]  (2 * N ASCII bytes: 64 B, then 16 B)
///       │
///       ▼ from_utf8_unchecked + f.write_str(hex_str)
/// Formatter             (1 write_str call per chunk, 0 heap allocs)
/// ```
///
/// Because `as_chunks_mut::<2>()` eliminates bounds checks, [`HexSlice::encode_byte`]
/// is branchless, and [`Formatter::write_str`] sits outside the loop, LLVM
/// auto-vectorizes the inner loop in [SIMD] registers.
///
/// [`Formatter::write_str`]: std::fmt::Formatter::write_str
/// [`HexSlice::encode_byte`]: HexSlice::encode_byte
/// [SIMD]: https://en.wikipedia.org/wiki/SIMD
impl Display for HexSlice<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Buffer for 64 hex chars which will be written next.
        let mut output_bytes =
            [NUL; constants::CHUNK_INPUT_BYTES * constants::HEX_CHARS_PER_BYTE];

        // Chunk self.inner byte slice into 32-byte chunks. This code works if self.inner
        // is < 32 bytes, or > 32 bytes, or 0 bytes.
        for input_chunk in self.inner.chunks(constants::CHUNK_INPUT_BYTES) {
            let out_char_len = input_chunk.len() * constants::HEX_CHARS_PER_BYTE;
            let out_slice = &mut output_bytes[..out_char_len];

            // output_bytes buffer len is always a multiple of 2 (HEX_CHARS_PER_BYTE).
            let (out_pairs, _) =
                out_slice.as_chunks_mut::<{ constants::HEX_CHARS_PER_BYTE }>();
            for ([out_hi, out_lo], &byte) in out_pairs.iter_mut().zip(input_chunk) {
                let (hi_ascii, lo_ascii) = HexSlice::encode_byte(byte);
                *out_hi = hi_ascii;
                *out_lo = lo_ascii;
            }

            // SAFETY: Every byte written by `HexSlice::encode_byte` is 7-bit ASCII
            // (`'0'..='9'` or `'a'..='f'`), so all `out_char_len` bytes are valid UTF-8.
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
    pub const fn decode_nibble(ascii_byte: u8) -> Option<u8> {
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
    pub const fn encode_nibble(nibble: u8) -> u8 {
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
    /// Number of [`ASCII`] hex characters required to represent a single byte (`2`).
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    pub const HEX_CHARS_PER_BYTE: usize = 2;

    /// Stack chunk input size (`32` bytes -> `64` hex output bytes per write call).
    ///
    /// Chosen because:
    /// - `32` bytes matches a SHA-256 digest, so digests format in a single stack pass
    ///   and one [`Formatter::write_str`] call.
    /// - `64` output bytes fits in a single CPU cache line on the stack with zero heap
    ///   allocation regardless of input length.
    /// - `32` / `64` bytes aligns with [SIMD] register widths so LLVM can vectorize the
    ///   inner nibble loop.
    ///
    /// [`Formatter::write_str`]: std::fmt::Formatter::write_str
    /// [SIMD]: https://en.wikipedia.org/wiki/SIMD
    pub const CHUNK_INPUT_BYTES: usize = 32;

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
    fn test_encode_and_decode_byte() {
        assert_eq2!(HexSlice::encode_byte(0x00), (b'0', b'0'));
        assert_eq2!(HexSlice::encode_byte(0x0a), (b'0', b'a'));
        assert_eq2!(HexSlice::encode_byte(0x1e), (b'1', b'e'));
        assert_eq2!(HexSlice::encode_byte(0xff), (b'f', b'f'));

        assert_eq2!(HexSlice::decode_byte(b'0', b'0'), Some(0x00));
        assert_eq2!(HexSlice::decode_byte(b'2', b'f'), Some(0x2f));
        assert_eq2!(HexSlice::decode_byte(b'F', b'a'), Some(0xfa));
        assert_eq2!(HexSlice::decode_byte(b'g', b'0'), None);
        assert_eq2!(HexSlice::decode_byte(b'0', b'g'), None);
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
        assert_eq2!(HexSlice { inner: b"" }.to_string(), "");
        assert_eq2!(
            HexSlice {
                inner: &[0x00, 0x0a, 0x7f, 0xe3]
            }
            .to_string(),
            "000a7fe3"
        );
        // Exercise > 32 input bytes so multiple 64-byte stack chunks execute.
        let input_40 = [0xab_u8; 40];
        let expected_40 = "ab".repeat(40);
        assert_eq2!(HexSlice { inner: &input_40 }.to_string(), expected_40);
    }
}
