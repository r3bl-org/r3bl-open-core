// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Constant values used in [`OSC`] (Operating System Command) sequences, organized by
//! functional category.
//!
//! These constants are used by [`OscSequence`] and [`OscSender`] for formatting
//! [`OSC`] sequences, by [`PtyOscProgressScanner`] for stream parsing, and by
//! [`vt_100_terminal_input_parser`] for framing and decoding terminal responses.
//!
//! See [constants module design] for the three-tier architecture.
//!
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`OscSender`]: crate::OscSender
//! [`OscSequence`]: crate::osc_codes::OscSequence
//! [`PtyOscProgressScanner`]: crate::core::ansi::osc::PtyOscProgressScanner
//! [`vt_100_terminal_input_parser`]: crate::core::ansi::vt_100_terminal_input_parser
//! [constants module design]: mod@crate::constants#design

use crate::{core::ansi, define_ansi_const};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Group 1: Enclosures, Delimiters, Queries & Terminators (Foundational Protocol Tokens)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Closing Bracket byte (`']'`, `0x5D`) following [`ESC`] to introduce an [`OSC`]
/// sequence.
///
/// Value: `93` dec, `5D` hex.
///
/// [`ESC`]: crate::EscSequence
/// [`OSC`]: crate::osc_codes::OscSequence
pub const ANSI_OSC_CLOSE_BRACKET: u8 = b']';

/// [`OSC`] Prefix: Byte slice starting an [`OSC`] sequence: `ESC ]` (`0x1B 0x5D`).
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_PREFIX: &[u8] = b"\x1b]";

/// [`OSC`] Start: Sequence start as byte slice: `ESC ]` (`0x1B 0x5D`).
///
/// Alias for [`OSC_PREFIX`].
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_START_BYTES: &[u8] = OSC_PREFIX;

/// [`OSC`] Prefix Length: Number of bytes in the [`OSC_PREFIX`] (2 bytes).
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_PREFIX_LEN: usize = OSC_PREFIX.len();

/// [`OSC`] Start: Sequence start as string slice: `ESC ]` (`\x1b]`).
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_START: &str =
    const_format::formatcp!("{ESC_STR}]", ESC_STR = ansi::constants::ESC_STR);

/// Parameter Delimiter: Semicolon `;` separating [`OSC`] parameters.
///
/// Value: `';'` (`3B` hex).
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_DELIMITER: char = ';';

/// Parameter Delimiter byte: Semicolon `b';'` (`0x3B`) separating [`OSC`] parameters.
///
/// Value: `59` dec, `3B` hex.
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_DELIMITER_BYTE: u8 = b';';

/// Query Parameter: Question mark `?` indicating a query in [`OSC`] sequences
/// (such as color queries `OSC 10..19` or clipboard queries `OSC 52`).
///
/// Value: `'?'` (`3F` hex).
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_QUERY: char = '?';

/// Query Parameter string: `"?"` indicating a query in [`OSC`] sequences.
///
/// Value: `"?"`.
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_QUERY_STR: &str = "?";

/// Query Parameter byte slice: `b"?"` indicating a query in [`OSC`] sequences.
///
/// Value: `b"?"`.
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_QUERY_BYTES: &[u8] = b"?";

define_ansi_const!(@esc_str : OSC_TERMINATOR_ST = ["\\"] =>
    "String Terminator (ST)" : "Standard ANSI String Terminator: `ESC \\`."
);

/// 7-bit String Terminator (`ST`) byte slice: `ESC \` (`0x1B 0x5C`).
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_TERMINATOR_ST_BYTES: &[u8] = b"\x1b\\";

/// BEL Terminator: De-facto standard [`OSC`] terminator string (`\x07`).
///
/// Value: `\x07` (`07` hex).
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_TERMINATOR_BEL: &str = "\x07";

/// BEL Terminator byte: De-facto standard [`OSC`] terminator byte (`0x07`).
///
/// Value: `7` dec, `07` hex.
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_TERMINATOR_BEL_BYTE: u8 = 0x07;

/// Bell (BEL) ([`ANSI`]): Control character `0x07` used to terminate [`OSC`] sequences.
///
/// Canonical alias for [`OSC_TERMINATOR_BEL_BYTE`].
///
/// Value: `7` dec, `07` hex.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`OSC`]: crate::osc_codes::OscSequence
pub const ANSI_BEL: u8 = OSC_TERMINATOR_BEL_BYTE;

/// Final byte of the 7-bit String Terminator transport encoding
/// ([`ANSI_ST_7BIT_TRANSPORT_ENCODING`]).
///
/// Value: `92` dec, `5C` hex.
///
/// Sequence: `ESC \` second byte ([`ASCII`] backslash `\`).
///
/// Note that this byte is **not** a terminator on its own; it only completes a String
/// Terminator sequence when immediately preceded by [`ANSI_ESC`] (`0x1B`).
///
/// [`ANSI_ESC`]: crate::core::ansi::constants::ANSI_ESC
/// [`ANSI_ST_7BIT_TRANSPORT_ENCODING`]: ANSI_ST_7BIT_TRANSPORT_ENCODING
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
pub const ANSI_ST_FINAL: u8 = b'\\';

/// 7-bit transport encoding of the String Terminator (ST) ([`ANSI`]): Two-byte escape
/// sequence `ESC \` (`0x1B 0x5C`) used to terminate [`OSC`] sequences.
///
/// Canonical alias for [`OSC_TERMINATOR_ST_BYTES`].
///
/// Value: `ESC \` (`1B 5C` hex).
///
/// Sequence: `ESC \` (two 7-bit [`ASCII`] bytes).
///
/// # Why is it called "7-bit" when it is 2 bytes long?
///
/// In terminal standards ([ECMA-48] / [ISO 6429]), the term **"7-bit"** does not refer to
/// the sequence length or size in bits; it refers to the **communication channel /
/// transport encoding**:
///
/// 1. **Historical Origin (7-Bit Serial Hardware Constraints)**: In early computing
///    (1960s-1980s), serial lines (RS-232) and teletypes frequently used 7 data bits with
///    parity (such as `7E1`). The 8th bit was overwritten by [`UART`] hardware for parity
///    checks or stripped to `0`. [ECMA-48] defined the primary C1 control code for String
///    Terminator (`ST`) as the single byte `0x9C` (`1001 1100` in binary). Because its
///    8th bit is set, `0x9C` could not travel across 7-bit links without corruption. To
///    solve this, [ECMA-48] defined an escape mapping: any C1 code `0x80 + X` maps to
///    [`ANSI_ESC`] (`0x1B`) followed by `0x40 + X`. For `ST` (`0x9C` = `0x80 + 0x1C`),
///    this yielded [`ANSI_ESC`] followed by `0x5C` (`\`): `0x1B 0x5C` (`ESC \`). Both
///    bytes are `<= 0x7F` and 7-bit clean.
///
/// 2. **The Modern Reality (8-Bit Clean Channels)**: Today, hardware serial lines are
///    largely obsolete. Modern operating systems, Linux PTYs, Unix domain sockets, and
///    TCP streams are fully 8-bit clean (they never strip bit 7).
///
/// 3. **Why Modern Terminals Never Reverted to Single-Byte `0x9C` ([`UTF-8`] Safety)**:
///    Why didn't modern terminal emulators ditch the 2-byte `ESC \` sequence and return
///    to the single-byte `0x9C`? Because of [`UTF-8`]:
///    - In [`UTF-8`], every byte in the range `0x80`..=`0xBF` is a continuation byte
///      (`0b10xx_xxxx`).
///    - `0x9C` (`0b1001_1100`) falls directly in the middle of this continuation range.
///    - Common Unicode characters like `✓` (`0xE2 0x9C 0x93`) and `£` (`0xC2 0x9C`)
///      contain `0x9C` as their continuation byte.
///    - If modern terminal emulators treated `0x9C` as a control character, any [`OSC`]
///      payload (such as clipboard text in [`OSC`] 52 or window titles in [`OSC`] 0/2)
///      containing `✓` or `£` would be prematurely truncated or cause parser corruption.
///    - By contrast, 7-bit [`ASCII`] characters (`0x00`..=`0x7F`) have the unique
///      property in [`UTF-8`] that they **never** appear inside multi-byte sequences.
///      They are unconditionally unambiguous.
///
/// # The Only Two Valid Terminators in Modern Terminals
///
/// Standard [`OSC`] sequences terminate in only one of two ways:
/// - [`ANSI_BEL`] (`0x07`, 1 byte).
/// - 7-bit [`ANSI_ST_7BIT_TRANSPORT_ENCODING`] (`ESC \`, 2 bytes).
///
/// [`ANSI_BEL`]: ANSI_BEL
/// [`ANSI_ESC`]: crate::core::ansi::constants::ANSI_ESC
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`OSC`]: crate::osc_codes::OscSequence
/// [`UART`]: https://en.wikipedia.org/wiki/UART
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
/// [ECMA-48]: https://en.wikipedia.org/wiki/ECMA-48
/// [ISO 6429]: https://en.wikipedia.org/wiki/ISO/IEC_6429
pub const ANSI_ST_7BIT_TRANSPORT_ENCODING: &[u8] = OSC_TERMINATOR_ST_BYTES;

/// Length of the 7-bit String Terminator transport encoding
/// ([`ANSI_ST_7BIT_TRANSPORT_ENCODING`]).
///
/// Value: `2` bytes (`ESC \`).
///
/// [`ANSI_ST_7BIT_TRANSPORT_ENCODING`]: ANSI_ST_7BIT_TRANSPORT_ENCODING
pub const ANSI_ST_7BIT_TRANSPORT_ENCODING_LEN: usize =
    ANSI_ST_7BIT_TRANSPORT_ENCODING.len();

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Group 2: Semantic Sequence Prefixes & Ends (Tier 2 composed constants)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

define_ansi_const!(@osc_str : OSC_TITLE_AND_ICON_START = ["0;"] =>
    "Title and Icon Start (OSC 0)" : "Title and icon start: `ESC ] 0 ;`."
);

define_ansi_const!(@osc_str : OSC_ICON_START = ["1;"] =>
    "Icon Start (OSC 1)" : "Icon start: `ESC ] 1 ;`."
);

define_ansi_const!(@osc_str : OSC_TITLE_START = ["2;"] =>
    "Title Start (OSC 2)" : "Title start: `ESC ] 2 ;`."
);

define_ansi_const!(@osc_str : OSC_HYPERLINK_START = ["8;;"] =>
    "Hyperlink Start (OSC 8)" : "Hyperlink start: `ESC ] 8 ; ;`."
);

define_ansi_const!(@osc_str : OSC_PROGRESS_START = ["9;4;"] =>
    "Progress Start (OSC 9;4)" : "Progress start: `ESC ] 9 ; 4 ;`."
);

/// Title End: Semantic alias for [`OSC_TERMINATOR_BEL`] (`\x07`).
pub const OSC_TITLE_END: &str = OSC_TERMINATOR_BEL;

/// Hyperlink End: Semantic alias for [`OSC_TERMINATOR_BEL`] (`\x07`).
pub const OSC_HYPERLINK_END: &str = OSC_TERMINATOR_BEL;

/// Progress End: Semantic alias for [`OSC_TERMINATOR_ST`] (`ESC \`).
pub const OSC_PROGRESS_END: &str = OSC_TERMINATOR_ST;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Group 3: Command Identification Codes (&str)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// `0` - Code for Title and Icon
pub const OSC_CODE_TITLE_AND_ICON: &str = "0";

/// `1` - Code for Icon
pub const OSC_CODE_ICON: &str = "1";

/// `2` - Code for Title
pub const OSC_CODE_TITLE: &str = "2";

/// `8` - Code for Hyperlink
pub const OSC_CODE_HYPERLINK: &str = "8";

/// `9` - Code for Progress
pub const OSC_CODE_PROGRESS: &str = "9";

/// Progress Subcommand: Parameter `4` identifying the progress reporting extension in
/// [`OSC`] 9.
///
/// Value: `"4"`.
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_PROGRESS_SUBCOMMAND: &str = "4";

/// Progress State Update: State `1` indicating active progress in [`OSC`] 9;4.
///
/// Value: `"1"`.
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_PROGRESS_STATE_UPDATE: &str = "1";

/// Progress State Clear: State `0` indicating progress cleared or removed in [`OSC`] 9;4.
///
/// Value: `"0"`.
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_PROGRESS_STATE_CLEAR: &str = "0";

/// Progress Percent Clear: Value `0` indicating 0% progress when clearing in [`OSC`] 9;4.
///
/// Value: `"0"`.
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_PROGRESS_PERCENT_CLEAR: &str = "0";

/// Operating System Command 52 ([`OSC`]): Parameter code for clipboard operations.
///
/// Value: `"52"`.
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_CODE_CLIPBOARD: &str = "52";

/// Operating System Command 10 ([`OSC`]): Parameter code for querying or reporting the
/// text foreground color.
///
/// Value: `"10"`.
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_CODE_COLOR_REPORT_FOREGROUND: &str = "10";

/// Operating System Command 11 ([`OSC`]): Parameter code for querying or reporting the
/// text background color.
///
/// Value: `"11"`.
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_CODE_COLOR_REPORT_BACKGROUND: &str = "11";

/// Operating System Command 12 ([`OSC`]): Parameter code for querying or reporting the
/// text cursor color.
///
/// Value: `"12"`.
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_CODE_COLOR_REPORT_CURSOR: &str = "12";

/// Operating System Command 13 ([`OSC`]): Parameter code for querying or reporting the
/// mouse pointer foreground color.
///
/// Value: `"13"`.
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_CODE_COLOR_REPORT_MOUSE_FOREGROUND: &str = "13";

/// Operating System Command 14 ([`OSC`]): Parameter code for querying or reporting the
/// mouse pointer background color.
///
/// Value: `"14"`.
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_CODE_COLOR_REPORT_MOUSE_BACKGROUND: &str = "14";

/// Operating System Command 17 ([`OSC`]): Parameter code for querying or reporting the
/// highlight / selection background color.
///
/// Value: `"17"`.
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_CODE_COLOR_REPORT_HIGHLIGHT: &str = "17";

/// Operating System Command 19 ([`OSC`]): Parameter code for querying or reporting the
/// highlight / selection foreground color.
///
/// Value: `"19"`.
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_CODE_COLOR_REPORT_HIGHLIGHT_FOREGROUND: &str = "19";

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Group 4: Color Specification Payload Tokens
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Color specification prefix for [`RGB`] color reports in [`OSC`] 10/11 (`"rgb:"`).
///
/// Value: `"rgb:"`.
///
/// [`OSC`]: crate::osc_codes::OscSequence
/// [`RGB`]: crate::RgbValue
pub const OSC_COLOR_SPEC_RGB_PREFIX: &str = "rgb:";

/// Color specification channel separator byte for [`RGB`] color reports in [`OSC`] 10/11
/// (`'/'`).
///
/// Value: `47` dec, `2F` hex.
///
/// Sequence part: `/`.
///
/// [`OSC`]: crate::osc_codes::OscSequence
/// [`RGB`]: crate::RgbValue
pub const OSC_COLOR_SPEC_CHANNEL_SEPARATOR: u8 = b'/';

/// Color specification prefix byte for hex color reports in [`OSC`] (`'#'`).
///
/// Value: `35` dec, `23` hex.
///
/// Sequence part: `#`.
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const OSC_COLOR_SPEC_HASH_PREFIX: &[u8] = b"#";

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Group 5: Clipboard Target Tokens
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// [`OSC`] 52 Target: Standard desktop system clipboard buffer (`'c'`).
///
/// Value: `99` dec, `63` hex.
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const CLIPBOARD_TARGET_CLIPBOARD: u8 = b'c';

/// [`OSC`] 52 Target: Primary selection buffer (`'p'`).
///
/// Value: `112` dec, `70` hex.
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const CLIPBOARD_TARGET_PRIMARY: u8 = b'p';

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Group 6: Framing Limits & Circuit Breaker Ceilings
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Maximum allowed length in bytes for an in-flight [`OSC`] sequence.
///
/// Safely accommodates large payloads such as [`OSC`] 52 clipboard transfers (1 MiB),
/// while acting as a circuit breaker ceiling against malformed unbounded escape
/// sequences.
///
/// Value: `1_048_576` bytes (1 MiB).
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const MAX_OSC_SEQUENCE_LENGTH: usize = 1_048_576;

/// Maximum allowed cumulative bytes to discard when draining a runaway [`OSC`] sequence.
///
/// Prevents a pathological or adversarial stream that emits a runaway sequence without
/// terminating from hanging the parser in an infinite drain loop.
///
/// Value: `16_777_216` bytes (16 MiB).
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub const MAX_OSC_DRAIN_BYTES: usize = 16_777_216;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc_constants() {
        assert_eq!(ANSI_OSC_CLOSE_BRACKET, b']');
        assert_eq!(OSC_PREFIX, b"\x1b]");
        assert_eq!(OSC_START_BYTES, b"\x1b]");
        assert_eq!(OSC_PREFIX_LEN, 2);
        assert_eq!(OSC_START, "\x1b]");
        assert_eq!(OSC_DELIMITER, ';');
        assert_eq!(OSC_DELIMITER_BYTE, b';');
        assert_eq!(OSC_QUERY, '?');
        assert_eq!(OSC_QUERY_STR, "?");
        assert_eq!(OSC_QUERY_BYTES, b"?");
        assert_eq!(OSC_TERMINATOR_BEL, "\x07");
        assert_eq!(OSC_TERMINATOR_BEL_BYTE, 0x07);
        assert_eq!(OSC_TERMINATOR_ST, "\x1b\\");
        assert_eq!(OSC_TERMINATOR_ST_BYTES, b"\x1b\\");
        assert_eq!(ANSI_BEL, 7);
        assert_eq!(ANSI_ST_FINAL, b'\\');
        assert_eq!(ANSI_ST_7BIT_TRANSPORT_ENCODING, b"\x1b\\");
        assert_eq!(ANSI_ST_7BIT_TRANSPORT_ENCODING_LEN, 2);
        assert_eq!(OSC_TITLE_AND_ICON_START, "\x1b]0;");
        assert_eq!(OSC_ICON_START, "\x1b]1;");
        assert_eq!(OSC_TITLE_START, "\x1b]2;");
        assert_eq!(OSC_HYPERLINK_START, "\x1b]8;;");
        assert_eq!(OSC_PROGRESS_START, "\x1b]9;4;");
        assert_eq!(OSC_CODE_TITLE_AND_ICON, "0");
        assert_eq!(OSC_CODE_ICON, "1");
        assert_eq!(OSC_CODE_TITLE, "2");
        assert_eq!(OSC_CODE_HYPERLINK, "8");
        assert_eq!(OSC_CODE_PROGRESS, "9");
        assert_eq!(OSC_PROGRESS_SUBCOMMAND, "4");
        assert_eq!(OSC_PROGRESS_STATE_UPDATE, "1");
        assert_eq!(OSC_PROGRESS_STATE_CLEAR, "0");
        assert_eq!(OSC_PROGRESS_PERCENT_CLEAR, "0");
        assert_eq!(OSC_CODE_CLIPBOARD, "52");
        assert_eq!(CLIPBOARD_TARGET_CLIPBOARD, b'c');
        assert_eq!(CLIPBOARD_TARGET_PRIMARY, b'p');
        assert_eq!(MAX_OSC_SEQUENCE_LENGTH, 1_048_576);
        assert_eq!(MAX_OSC_DRAIN_BYTES, 16_777_216);
        assert_eq!(OSC_CODE_COLOR_REPORT_FOREGROUND, "10");
        assert_eq!(OSC_CODE_COLOR_REPORT_BACKGROUND, "11");
        assert_eq!(OSC_CODE_COLOR_REPORT_CURSOR, "12");
        assert_eq!(OSC_CODE_COLOR_REPORT_MOUSE_FOREGROUND, "13");
        assert_eq!(OSC_CODE_COLOR_REPORT_MOUSE_BACKGROUND, "14");
        assert_eq!(OSC_CODE_COLOR_REPORT_HIGHLIGHT, "17");
        assert_eq!(OSC_CODE_COLOR_REPORT_HIGHLIGHT_FOREGROUND, "19");
        assert_eq!(OSC_COLOR_SPEC_RGB_PREFIX, "rgb:");
        assert_eq!(OSC_COLOR_SPEC_CHANNEL_SEPARATOR, b'/');
        assert_eq!(OSC_COLOR_SPEC_HASH_PREFIX, b"#");
    }
}
