// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Terminal event parsing from [`ANSI`] sequences.
//!
//! This module handles terminal-level events like window resize, focus changes, and
//! bracketed paste mode notifications.
//!
//! ## Where You Are in the Pipeline
//!
//! For the full data flow, see the [parent module documentation]. This diagram shows
//! where `terminal_events.rs` fits:
//!
//! ```text
//! DirectToAnsiInputDevice (async I/O layer)
//!    │
//!    ▼
//! router.rs (routing & `ESC` detection)
//!    │ (routes terminal event sequences here)
//! ┌──▼──────────────────────────────────────────┐  ┌──────────────────┐
//! │  terminal_events.rs                         ◄──┤ **YOU ARE HERE** │
//! │  • Parse window resize events               │  └──────────────────┘
//! │  • Parse focus gained/lost                  │
//! │  • Parse bracketed paste markers            │
//! │  • Scan and discard unhandled OSC responses │
//! └─────────────────────────────────────────────┘
//!    │
//!    ▼
//! VT100InputEventIR::{ Resize | Focus | Paste }
//!    │
//!    ▼
//! convert_input_event() → InputEvent (returned to application)
//! ```
//!
//! **Navigate**:
//! - ⬆️ **Up**: [`router`] - Main routing entry point
//! - ➡️ **Peer**: [`keyboard`], [`mouse`], [`utf8`] - Other specialized parsers
//! - 📚 **Types**: [`VT100FocusStateIR`], [`VT100PasteModeIR`]
//! - 📤 **Converted by**: [`convert_input_event()`] in `protocol_conversion.rs` (not this
//!   module)
//!
//! ## Supported Events
//! - **Window Resize**: `ESC [ 8 ; rows ; cols t`
//! - **Focus Gained**: `ESC [ I`
//! - **Focus Lost**: `ESC [ O`
//! - **Bracketed Paste Start**: `ESC [ 2 0 0 ~`
//! - **Bracketed Paste End**: `ESC [ 2 0 1 ~`
//! - **[`OSC`] Responses (Framed & Discarded)**: `ESC ] ... (BEL | ST)`
//!
//! ## Bidirectional Terminal Communication & [`OSC`] Handling
//!
//! Modern terminal emulators write responses to queries (e.g. background color,
//! clipboard contents) directly into `stdin`. These responses begin with `ESC ]`
//! (`0x1B 0x5D`, [`OSC`] prefix).
//!
//! This module houses [`scan_osc_sequence()`], [`try_disambiguate_osc_or_alt_bracket()`],
//! and [`OscScanResult`], which inspect input buffers using a dedicated state
//! machine to distinguish between human keystrokes (such as `Alt+]`) and incoming
//! terminal responses, ensuring complete [`OSC`] payloads are consumed and ignored
//! without leaking text to the screen.
//!
//! For the comprehensive architectural mental model and [`ASCII`] data flow diagram, see
//! the [Bidirectional Communication section in the parent module].
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
//! [`convert_input_event()`]:
//!     crate::direct_to_ansi::input::protocol_conversion::convert_input_event
//! [`keyboard`]: mod@super::keyboard
//! [`mouse`]: mod@super::mouse
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`router`]: mod@super::router
//! [`utf8`]: mod@super::utf8
//! [`VT100FocusStateIR`]: super::VT100FocusStateIR
//! [`VT100PasteModeIR`]: super::VT100PasteModeIR
//! [Bidirectional Communication section in the parent module]:
//!     mod@super#bidirectional-communication-user-input-vs-terminal-responses
//! [parent module documentation]: mod@super#primary-consumer

use super::{ir_event_types::{VT100FocusStateIR, VT100InputEventIR, VT100KeyCodeIR,
                             VT100KeyModifiersIR, VT100PasteModeIR},
            maybe_more::MaybeMore};
use crate::{ByteOffset, KeyState, byte_offset,
            core::ansi::constants::{ANSI_BEL, ANSI_CSI_BRACKET, ANSI_ESC,
                                    ANSI_FUNCTION_KEY_TERMINATOR, ANSI_PARAM_SEPARATOR,
                                    ANSI_ST_7BIT, ANSI_ST_7BIT_LEN, ASCII_DIGIT_0,
                                    ASCII_DIGIT_9, ASCII_QUESTION_MARK,
                                    CARRIAGE_RETURN, FOCUS_GAINED_FINAL,
                                    FOCUS_LOST_FINAL, LINE_FEED,
                                    MAX_OSC_SEQUENCE_LENGTH, OSC_PREFIX,
                                    OSC_PREFIX_LEN, PASTE_END_PARSE_PARAM,
                                    PASTE_START_PARSE_PARAM, RESIZE_EVENT_PARSE_PARAM,
                                    RESIZE_TERMINATOR}};

/// Parses a terminal event sequence and returns a [`VT100InputEventIR`] with bytes
/// consumed if recognized.
///
/// # Returns
///
/// - The parsed event and byte count on success.
/// - Nothing if the sequence is incomplete or unrecognized.
///
/// # Handled Sequences
///
/// - `ESC [ 8 ; 2 4 ; 8 0 t` - Window resize to 24 rows × 80 columns
/// - `ESC [ I` - Terminal gained focus
/// - `ESC [ O` - Terminal lost focus
/// - `ESC [ 2 0 0 ~` - Bracketed paste start
/// - `ESC [ 2 0 1 ~` - Bracketed paste end
#[must_use]
pub fn parse_terminal_event(buffer: &[u8]) -> Option<(VT100InputEventIR, ByteOffset)> {
    // Check minimum length: ESC [ + final byte
    if buffer.len() < 3 {
        return None;
    }

    // Check for ESC [ sequence start
    if buffer[0] != ANSI_ESC || buffer[1] != ANSI_CSI_BRACKET {
        return None;
    }

    // Handle simple focus events (single character after ESC[)
    if buffer.len() == 3 {
        match buffer[2] {
            FOCUS_GAINED_FINAL => {
                return Some((
                    VT100InputEventIR::Focus(VT100FocusStateIR::Gained),
                    byte_offset(3),
                ));
            }
            FOCUS_LOST_FINAL => {
                return Some((
                    VT100InputEventIR::Focus(VT100FocusStateIR::Lost),
                    byte_offset(3),
                ));
            }
            _ => {}
        }
    }

    // Parse parameters and final byte for multi-character sequences
    parse_csi_terminal_parameters(buffer)
}

/// Parse [`CSI`] sequences with parameters for terminal events.
///
/// [`CSI`]: crate::CsiSequence
fn parse_csi_terminal_parameters(
    buffer: &[u8],
) -> Option<(VT100InputEventIR, ByteOffset)> {
    // Extract parameters and final byte
    // Format: ESC [ [param;param;...] final_byte
    let mut params = Vec::new();
    let mut current_number_str = String::new();
    let mut final_byte = 0u8;
    let mut bytes_scanned = 0;

    for (slice_index, &byte) in buffer[2..].iter().enumerate() {
        bytes_scanned = slice_index + 1; // Track position relative to buffer[2..]

        // IMPORTANT: We use if/else chains instead of match arms because Rust treats
        // constants in match patterns as variable bindings, not value comparisons.
        // See keyboard.rs for detailed explanation of this pattern.

        if (ASCII_DIGIT_0..=ASCII_DIGIT_9).contains(&byte) {
            // Digit: accumulate in current_number_str
            current_number_str.push(char::from(byte));
        } else if byte == ANSI_PARAM_SEPARATOR {
            // Semicolon: parameter separator
            if !current_number_str.is_empty() {
                params.push(current_number_str.parse::<u16>().unwrap_or(0));
                current_number_str.clear();
            }
        } else if byte == ANSI_FUNCTION_KEY_TERMINATOR || byte == RESIZE_TERMINATOR {
            // Terminal character: '~' for paste events, 't' for resize events
            if !current_number_str.is_empty() {
                params.push(current_number_str.parse::<u16>().unwrap_or(0));
            }
            final_byte = byte;
            break;
        } else {
            return None; // Invalid byte in sequence
        }
    }

    if final_byte == 0 {
        return None; // No final byte found
    }

    // Total bytes consumed: ESC [ (2 bytes) + scanned bytes (includes final)
    let total_consumed = 2 + bytes_scanned;

    // Parse based on parameters and final byte
    // Using if/else for consistency - avoiding all match statements when using constants
    if params.len() == 3
        && final_byte == RESIZE_TERMINATOR
        && params[0] == RESIZE_EVENT_PARSE_PARAM
    {
        // Window resize: CSI 8 ; rows ; cols t
        let rows = params[1];
        let columns = params[2];
        Some((
            VT100InputEventIR::Resize {
                col_width: crate::VPWidth::from(columns),
                row_height: crate::VPHeight::from(rows),
            },
            byte_offset(total_consumed),
        ))
    } else if params.len() == 1 && final_byte == ANSI_FUNCTION_KEY_TERMINATOR {
        // Bracketed paste: CSI 200 ~ or CSI 201 ~
        if params[0] == PASTE_START_PARSE_PARAM {
            Some((
                VT100InputEventIR::Paste(VT100PasteModeIR::Start),
                byte_offset(total_consumed),
            ))
        } else if params[0] == PASTE_END_PARSE_PARAM {
            Some((
                VT100InputEventIR::Paste(VT100PasteModeIR::End),
                byte_offset(total_consumed),
            ))
        } else {
            None
        }
    } else {
        None
    }
}

// ==================== OSC Scanning & Disambiguation ====================

/// Result of scanning an input buffer for an Operating System Command ([`OSC`]) sequence.
///
/// Returned by [`scan_osc_sequence()`] to guide the parser in distinguishing between
/// human keystrokes (`Alt+]`) and terminal emulator query responses.
///
/// [`OSC`]: crate::osc_codes::OscSequence
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum OscScanResult {
    /// A complete [`OSC`] sequence was recognized and terminated by either [`ANSI_BEL`]
    /// (`0x07`) or 7-bit [`ANSI_ST_7BIT`] (`0x1B 0x5C`). The wrapped [`ByteOffset`]
    /// indicates the total number of bytes consumed from the buffer (prefix + payload
    /// + terminator).
    ///
    /// [`ANSI_BEL`]: crate::ANSI_BEL
    /// [`ANSI_ST_7BIT`]: crate::ANSI_ST_7BIT
    /// [`OSC`]: crate::osc_codes::OscSequence
    Complete(ByteOffset),

    /// The sequence begins with `ESC ]` and follows valid [`OSC`] command syntax, but is
    /// still scanning decimal command digits (no `;` or `?` delimiter arrived yet).
    /// If more input is anticipated ([`MaybeMore::KernelMayHaveMore`]), the parser
    /// waits. If the stream has drained ([`MaybeMore::KernelDrained`]), this
    /// indicates human typing (e.g., `Alt+]` followed by digits), and the parser
    /// falls back to `Alt+]`.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    IncompleteDigits,

    /// The sequence has seen the parameter delimiter (`;` or `?`) and is scanning payload
    /// content. This is guaranteed to be an in-flight [`OSC`] sequence. The parser always
    /// waits for the remaining payload across read boundaries (bounded by
    /// [`MAX_OSC_SEQUENCE_LENGTH`]).
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    IncompletePayload,

    /// The sequence begins with `ESC ]`, but violates [`OSC`] command syntax (such as
    /// non-digit characters before the parameter delimiter `;`, or embedded
    /// newline/carriage return characters). This cannot be a valid [`OSC`] sequence;
    /// the parser immediately rejects it and falls back to emitting `Alt+]`.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    InvalidSyntax,

    /// The candidate [`OSC`] sequence exceeded [`MAX_OSC_SEQUENCE_LENGTH`] without
    /// encountering a terminator. The buffer is purged to prevent unbounded memory
    /// growth and input freezes.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    Runaway,
}

/// Lexical scanner that parses a byte buffer for an Operating System Command ([`OSC`])
/// sequence.
///
/// Terminal query responses (e.g., background color [`OSC`] 11, clipboard [`OSC`] 52)
/// start with `ESC ]` (`0x1B 0x5D`), have a numeric command identifier, parameters, and
/// terminate with either [`ANSI_BEL`] (`0x07`) or 7-bit [`ANSI_ST_7BIT`] (`0x1B 0x5C`).
///
/// # Grammar & Validation Rules
///
/// 1. **Strict [`OSC`] Syntax Validation**: All standard [`OSC`] sequences follow the
///    strict grammar: `ESC ] <command_digits> ; <payload> (BEL | ST)`. If non-digit
///    characters appear before the parameter delimiter `;`, or if raw carriage returns
///    (`\r`) or newlines (`\n`) are encountered, the scanner immediately halts and
///    returns [`OscScanResult::InvalidSyntax`]. This prevents human keystrokes like
///    `Alt+] 5 a` from being falsely treated as candidate [`OSC`].
///
/// 2. **[`UTF-8`] Safety Note**: ECMA-48 specifies `0x9C` as an 8-bit String Terminator
///    (`ST`). However, in [`UTF-8`], `0x9C` is a common continuation byte (`0b1001_1100`,
///    used in characters like `£`, `œ`, and `✓`). Modern terminal emulators in [`UTF-8`]
///    mode exclusively send 7-bit [`ANSI_ST_7BIT`] (`0x1B 0x5C`) or [`ANSI_BEL`]
///    (`0x07`). This scanner intentionally does not match `0x9C` to prevent truncating
///    [`OSC`] payloads containing valid [`UTF-8`] text.
///
/// [`ANSI_BEL`]: crate::ANSI_BEL
/// [`ANSI_ST_7BIT`]: crate::ANSI_ST_7BIT
/// [`OSC`]: crate::osc_codes::OscSequence
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
#[must_use]
pub fn scan_osc_sequence(buffer: &[u8]) -> OscScanResult {
    if !buffer.starts_with(OSC_PREFIX) {
        return OscScanResult::InvalidSyntax;
    }

    let mut byte_index = OSC_PREFIX_LEN;

    // Phase 1: Scan decimal command digits until delimiter or terminator.
    loop {
        if byte_index >= buffer.len() {
            return if buffer.len() >= MAX_OSC_SEQUENCE_LENGTH {
                OscScanResult::Runaway
            } else {
                OscScanResult::IncompleteDigits
            };
        }

        let byte = buffer[byte_index];
        if (ASCII_DIGIT_0..=ASCII_DIGIT_9).contains(&byte) {
            byte_index += 1;
        } else if byte == ANSI_PARAM_SEPARATOR || byte == ASCII_QUESTION_MARK {
            byte_index += 1;
            break; // Transition to payload scanning.
        } else if byte == ANSI_BEL {
            return OscScanResult::Complete(byte_offset(byte_index + 1));
        } else if byte == ANSI_ESC {
            return check_st_terminator(
                buffer,
                byte_index,
                OscScanResult::IncompleteDigits,
            );
        } else {
            return OscScanResult::InvalidSyntax;
        }
    }

    // Phase 2: Scan payload content until terminator.
    while byte_index < buffer.len() {
        let byte = buffer[byte_index];
        if byte == ANSI_BEL {
            return OscScanResult::Complete(byte_offset(byte_index + 1));
        } else if byte == ANSI_ESC {
            return check_st_terminator(
                buffer,
                byte_index,
                OscScanResult::IncompletePayload,
            );
        } else if byte == CARRIAGE_RETURN || byte == LINE_FEED {
            return OscScanResult::InvalidSyntax;
        }
        byte_index += 1;
    }

    if buffer.len() >= MAX_OSC_SEQUENCE_LENGTH {
        OscScanResult::Runaway
    } else {
        OscScanResult::IncompletePayload
    }
}

/// Helper to scan for 7-bit String Terminator ([`ANSI_ST_7BIT`]) following an
/// [`ANSI_ESC`] byte.
///
/// Returns:
/// - [`OscScanResult::Complete`] if the sequence matches [`ANSI_ST_7BIT`].
/// - [`OscScanResult::InvalidSyntax`] if unexpected bytes follow [`ANSI_ESC`].
/// - `incomplete_result` if [`ANSI_ESC`] is the trailing byte in the buffer.
///
/// [`ANSI_ESC`]: crate::ANSI_ESC
/// [`ANSI_ST_7BIT`]: crate::ANSI_ST_7BIT
#[inline]
fn check_st_terminator(
    buffer: &[u8],
    byte_index: usize,
    incomplete_result: OscScanResult,
) -> OscScanResult {
    if buffer[byte_index..].starts_with(ANSI_ST_7BIT) {
        OscScanResult::Complete(byte_offset(byte_index + ANSI_ST_7BIT_LEN))
    } else if byte_index + 1 < buffer.len() {
        OscScanResult::InvalidSyntax
    } else {
        incomplete_result
    }
}

/// Disambiguates an incoming `ESC ]` (`0x1B 0x5D`) buffer between an `Alt+]` human
/// keystroke and a terminal emulator [`OSC`] query response.
///
/// # Disambiguation Rules
///
/// 1. **Rule 1: The [`MaybeMore`] Heuristic**: Terminal emulators emit [`OSC`] responses
///    in high-speed single-burst writes, whereas human keystrokes have tens of
///    milliseconds between them. If all available input was drained, the sequence is
///    incomplete, and it is guaranteed to be human input (e.g., `Alt+]` alone or `Alt+]
///    5`), so the function emits `Alt+]` (2 bytes consumed) and preserves trailing bytes
///    for the next parse cycle. If `maybe_more == MaybeMore::KernelMayHaveMore`, the
///    function defers parsing to allow the rest of the burst to arrive.
///
/// 2. **Rule 2: Strict [`OSC`] Syntax Validation**: If [`scan_osc_sequence()`] returns
///    [`OscScanResult::InvalidSyntax`], the sequence cannot be an [`OSC`] response;
///    `Alt+]` is emitted immediately.
///
/// # Why Inbound [`OSC`] Sequences are Quarantined and Absorbed
///
/// When [`scan_osc_sequence()`] detects a complete [`OSC`] sequence on `stdin`, it is
/// mapped to [`VT100InputEventIR::Ignored`] and absorbed.
///
/// **Terminal Applications Do Not Rely on [`OSC`] 52 Queries**:
/// - Almost no CLI or TUI programs rely on `OSC 52 ; c ; ?` queries to function. In fact,
///   most programs never query [`OSC`] 52 because:
///   - Standard terminals (like [`Kitty`], [`Alacritty`], Foot) disable [`OSC`] 52
///     queries by default for security reasons (clipboard snooping vulnerabilities).
///   - Programs that want clipboard integration either use system CLI helpers (`xclip`,
///     `wl-copy`, `pbcopy`), their own internal registers (like Vim/Neovim's default
///     unnamed registers), or Bracketed Paste.
/// - If a program (like Neovim configured with an [`OSC`] 52 plugin) emits `OSC 52 ; c ;
///   ?` and receives no response, it simply falls back to its internal register without
///   hanging.
///
/// **Preventing Spurious Keystroke Leaks**: If inbound [`OSC`] sequences were not
/// cleanly quarantined and absorbed, their raw bytes (such as command digits,
/// delimiters, and Base64 text) would be interpreted as physical keystrokes, leaking
/// directly into the application event stream.
///
/// **Diagnostics**: When [`DEBUG_TUI_SHOW_DIRECT_TO_ANSI`] is enabled, absorbing an
/// [`OSC`] sequence logs a structured warning with both raw hex bytes and string
/// representation.
///
/// [`Alacritty`]: https://alacritty.org/
/// [`DEBUG_TUI_SHOW_DIRECT_TO_ANSI`]: crate::DEBUG_TUI_SHOW_DIRECT_TO_ANSI
/// [`Kitty`]: https://sw.kovidgoyal.net/kitty/
/// [`MaybeMore`]: crate::core::ansi::vt_100_terminal_input_parser::MaybeMore
/// [`OSC`]: crate::osc_codes::OscSequence
#[must_use]
pub fn try_disambiguate_osc_or_alt_bracket(
    buffer: &[u8],
    maybe_more: MaybeMore,
) -> Option<(VT100InputEventIR, ByteOffset)> {
    if !buffer.starts_with(OSC_PREFIX) {
        return None;
    }

    // Route based on lexical scanner outcome.
    // Note: When buffer is lone `ESC ]` (len == 2), `scan_osc_sequence` performs 0
    // iterations and returns `IncompleteDigits`, seamlessly evaluating the
    // `maybe_more` check below.
    match scan_osc_sequence(buffer) {
        OscScanResult::Complete(consumed) => {
            crate::DEBUG_TUI_SHOW_DIRECT_TO_ANSI.then(|| {
                let len = consumed.as_usize();
                // % is Display, ? is Debug.
                tracing::warn! {
                    message = "try_disambiguate_osc_or_alt_bracket - absorbed OSC sequence from stdin",
                    raw_osc_hex = %format!("{:02X?}", &buffer[..len]),
                    raw_osc_str = %String::from_utf8_lossy(&buffer[..len]),
                    consumed_bytes = len,
                };
            });
            Some((VT100InputEventIR::Ignored, consumed))
        }
        OscScanResult::InvalidSyntax => {
            // Violated OSC syntax; cannot be OSC. Emit Alt+] (2 bytes)
            // and leave trailing bytes in buffer for next cycle.
            Some((alt_bracket_event(), byte_offset(OSC_PREFIX_LEN)))
        }
        OscScanResult::IncompleteDigits => match maybe_more {
            MaybeMore::KernelMayHaveMore => None, /* In-flight burst; wait for */
            // possible delimiter/payload.
            MaybeMore::KernelDrained => {
                // Stream drained before delimiter arrived. Human typed Alt+] (alone or
                // with digits). Emit Alt+] (2 bytes) and leave any
                // trailing digits in buffer.
                Some((alt_bracket_event(), byte_offset(OSC_PREFIX_LEN)))
            }
        },
        OscScanResult::IncompletePayload => {
            // Delimiter was already parsed. This is guaranteed to be an in-flight OSC
            // sequence. Always wait for the rest of the payload across reads
            // (bounded by MAX_OSC_SEQUENCE_LENGTH).
            None
        }
        OscScanResult::Runaway => {
            // Defer to classify_unparsed_buffer to trip circuit breaker and purge buffer.
            None
        }
    }
}

/// Helper to construct an `Alt+]` key event.
#[must_use]
pub fn alt_bracket_event() -> VT100InputEventIR {
    VT100InputEventIR::Keyboard {
        code: VT100KeyCodeIR::Char(']'),
        modifiers: VT100KeyModifiersIR {
            shift: KeyState::NotPressed,
            ctrl: KeyState::NotPressed,
            alt: KeyState::Pressed,
        },
    }
}

/// Unit tests for terminal event parsing (focus, resize, bracketed paste).
///
/// These tests use generator functions instead of hardcoded magic strings to ensure
/// consistency between sequence generation and parsing.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClipboardTarget, OscSequence,
                core::{ansi::{constants::{CARRIAGE_RETURN,
                                          CLIPBOARD_TARGET_CLIPBOARD, LINE_FEED,
                                          OSC_CODE_CLIPBOARD},
                              generator::generate_keyboard_sequence},
                       osc::osc_codes::{OSC_DELIMITER, OSC_START, OSC_TERMINATOR_BEL,
                                        OSC_TERMINATOR_ST}}};

    #[test]
    fn test_resize_event() {
        // Round-trip test: Generate sequence from VT100InputEventIR, then parse it back
        let original_event = VT100InputEventIR::Resize {
            row_height: crate::VPHeight::from(24),
            col_width: crate::VPWidth::from(80),
        };
        let sequence = generate_keyboard_sequence(&original_event)
            .expect("Failed to generate resize sequence");

        let (parsed_event, bytes_consumed) =
            parse_terminal_event(&sequence).expect("Should parse resize");

        assert_eq!(bytes_consumed.as_usize(), sequence.len());
        assert_eq!(parsed_event, original_event);
    }

    #[test]
    fn test_focus_events() {
        // Round-trip test: Focus gained
        let original_gained = VT100InputEventIR::Focus(VT100FocusStateIR::Gained);
        let sequence_gained = generate_keyboard_sequence(&original_gained)
            .expect("Failed to generate focus gained sequence");

        let (parsed_event, bytes_consumed) =
            parse_terminal_event(&sequence_gained).expect("Should parse focus gained");

        assert_eq!(bytes_consumed.as_usize(), sequence_gained.len());
        assert_eq!(parsed_event, original_gained);

        // Round-trip test: Focus lost
        let original_lost = VT100InputEventIR::Focus(VT100FocusStateIR::Lost);
        let sequence_lost = generate_keyboard_sequence(&original_lost)
            .expect("Failed to generate focus lost sequence");

        let (parsed_event, bytes_consumed) =
            parse_terminal_event(&sequence_lost).expect("Should parse focus lost");

        assert_eq!(bytes_consumed.as_usize(), sequence_lost.len());
        assert_eq!(parsed_event, original_lost);
    }

    #[test]
    fn test_bracketed_paste() {
        // Round-trip test: Paste start
        let original_start = VT100InputEventIR::Paste(VT100PasteModeIR::Start);
        let sequence_start = generate_keyboard_sequence(&original_start)
            .expect("Failed to generate paste start sequence");

        let (parsed_event, bytes_consumed) =
            parse_terminal_event(&sequence_start).expect("Should parse paste start");

        assert_eq!(bytes_consumed.as_usize(), sequence_start.len());
        assert_eq!(parsed_event, original_start);

        // Round-trip test: Paste end
        let original_end = VT100InputEventIR::Paste(VT100PasteModeIR::End);
        let sequence_end = generate_keyboard_sequence(&original_end)
            .expect("Failed to generate paste end sequence");

        let (parsed_event, bytes_consumed) =
            parse_terminal_event(&sequence_end).expect("Should parse paste end");

        assert_eq!(bytes_consumed.as_usize(), sequence_end.len());
        assert_eq!(parsed_event, original_end);
    }

    #[test]
    fn test_invalid_sequences() {
        // Test: incomplete sequence (too short)
        assert_eq!(parse_terminal_event(&[ANSI_ESC]), None);

        // Test: sequence without CSI start
        assert_eq!(parse_terminal_event(b"abc"), None);

        // Test: empty buffer
        assert_eq!(parse_terminal_event(b""), None);
    }

    #[test]
    fn test_scan_osc_sequence_bel() {
        let buffer = format!("{OSC_START}11;rgb:0000/0000/0000{OSC_TERMINATOR_BEL}");
        let buffer = buffer.as_bytes();
        assert_eq!(
            scan_osc_sequence(buffer),
            OscScanResult::Complete(byte_offset(buffer.len()))
        );
    }

    #[test]
    fn test_scan_osc_sequence_st() {
        let buffer = format!("{OSC_START}11;rgb:ffff/ffff/ffff{OSC_TERMINATOR_ST}");
        let buffer = buffer.as_bytes();
        assert_eq!(
            scan_osc_sequence(buffer),
            OscScanResult::Complete(byte_offset(buffer.len()))
        );
    }

    #[test]
    fn test_scan_osc_sequence_incomplete_digits() {
        let seq = format!("{OSC_START}12");
        assert_eq!(
            scan_osc_sequence(seq.as_bytes()),
            OscScanResult::IncompleteDigits
        );
        assert_eq!(
            scan_osc_sequence(OSC_PREFIX),
            OscScanResult::IncompleteDigits
        );
    }

    #[test]
    fn test_scan_osc_sequence_incomplete_payload() {
        let seq_data = format!("{OSC_START}12{OSC_DELIMITER}data");
        assert_eq!(
            scan_osc_sequence(seq_data.as_bytes()),
            OscScanResult::IncompletePayload
        );
        let seq_empty = format!("{OSC_START}12{OSC_DELIMITER}");
        assert_eq!(
            scan_osc_sequence(seq_empty.as_bytes()),
            OscScanResult::IncompletePayload
        );
    }

    #[test]
    fn test_scan_osc_sequence_partial_st_payload() {
        let seq = [OSC_PREFIX, b"12;data", &[ANSI_ESC]].concat();
        assert_eq!(scan_osc_sequence(&seq), OscScanResult::IncompletePayload);
    }

    #[test]
    fn test_scan_osc_sequence_partial_st_digits() {
        let seq = [OSC_PREFIX, b"0", &[ANSI_ESC]].concat();
        assert_eq!(scan_osc_sequence(&seq), OscScanResult::IncompleteDigits);
    }

    #[test]
    fn test_scan_osc_sequence_invalid_syntax_letters_in_digits() {
        let seq = format!("{OSC_START}12a{OSC_DELIMITER}");
        assert_eq!(
            scan_osc_sequence(seq.as_bytes()),
            OscScanResult::InvalidSyntax
        );
    }

    #[test]
    fn test_scan_osc_sequence_invalid_syntax_embedded_newline() {
        let seq_lf = [OSC_PREFIX, b"12;data", &[LINE_FEED, ANSI_BEL]].concat();
        assert_eq!(scan_osc_sequence(&seq_lf), OscScanResult::InvalidSyntax);
        let seq_cr = [OSC_PREFIX, b"12;data", &[CARRIAGE_RETURN, ANSI_BEL]].concat();
        assert_eq!(scan_osc_sequence(&seq_cr), OscScanResult::InvalidSyntax);
    }

    #[test]
    fn test_scan_osc_sequence_invalid_syntax_unexpected_esc_in_payload() {
        let seq = [OSC_PREFIX, b"12;data", &[ANSI_ESC], b"x"].concat();
        assert_eq!(scan_osc_sequence(&seq), OscScanResult::InvalidSyntax);
    }

    #[test]
    fn test_scan_osc_sequence_utf8_continuation_byte_0x9c() {
        let target_char = char::from(CLIPBOARD_TARGET_CLIPBOARD);
        let buffer_incomplete = format!(
            "{OSC_START}{OSC_CODE_CLIPBOARD}{OSC_DELIMITER}{target_char}{OSC_DELIMITER}\u{2713}"
        );
        let buffer_incomplete = buffer_incomplete.as_bytes();
        assert_eq!(
            scan_osc_sequence(buffer_incomplete),
            OscScanResult::IncompletePayload
        );

        let buffer_complete = format!(
            "{OSC_START}{OSC_CODE_CLIPBOARD}{OSC_DELIMITER}{target_char}{OSC_DELIMITER}\u{2713}{OSC_TERMINATOR_BEL}"
        );
        let buffer_complete = buffer_complete.as_bytes();
        assert_eq!(
            scan_osc_sequence(buffer_complete),
            OscScanResult::Complete(byte_offset(buffer_complete.len()))
        );
    }

    #[test]
    fn test_scan_osc_sequence_runaway() {
        let mut runaway = Vec::with_capacity(MAX_OSC_SEQUENCE_LENGTH + 10);
        let prefix = format!("{OSC_START}{OSC_CODE_CLIPBOARD}{OSC_DELIMITER}");
        runaway.extend_from_slice(prefix.as_bytes());
        runaway.resize(MAX_OSC_SEQUENCE_LENGTH + 1, b'a');
        assert_eq!(scan_osc_sequence(&runaway), OscScanResult::Runaway);
    }

    #[test]
    fn test_try_disambiguate_osc_or_alt_bracket() {
        // Lone Alt+] with KernelDrained: emits Alt+]
        let (event, consumed) =
            try_disambiguate_osc_or_alt_bracket(OSC_PREFIX, MaybeMore::KernelDrained)
                .expect("Should emit Alt+]");
        assert_eq!(event, alt_bracket_event());
        assert_eq!(consumed.as_usize(), 2);

        // Lone Alt+] with KernelMayHaveMore: waits
        assert_eq!(
            try_disambiguate_osc_or_alt_bracket(OSC_PREFIX, MaybeMore::KernelMayHaveMore),
            None
        );

        // Alt+] followed by invalid syntax: emits Alt+] (2 bytes)
        let invalid_syntax_seq = [OSC_PREFIX, b"a"].concat();
        let (event, consumed) = try_disambiguate_osc_or_alt_bracket(
            &invalid_syntax_seq,
            MaybeMore::KernelMayHaveMore,
        )
        .expect("Should emit Alt+] on invalid syntax");
        assert_eq!(event, alt_bracket_event());
        assert_eq!(consumed.as_usize(), 2);

        // Alt+] followed by digits with KernelDrained: emits Alt+] (2 bytes)
        let digits_seq = [OSC_PREFIX, b"5"].concat();
        let (event, consumed) =
            try_disambiguate_osc_or_alt_bracket(&digits_seq, MaybeMore::KernelDrained)
                .expect("Should emit Alt+] when drained");
        assert_eq!(event, alt_bracket_event());
        assert_eq!(consumed.as_usize(), 2);

        // Alt+] followed by digits with KernelMayHaveMore: waits
        assert_eq!(
            try_disambiguate_osc_or_alt_bracket(
                &digits_seq,
                MaybeMore::KernelMayHaveMore
            ),
            None
        );

        // Candidate OSC complete with BEL: emits Ignored
        let complete_bel = format!("{OSC_START}11;rgb:00/00/00{OSC_TERMINATOR_BEL}");
        let complete_bel_bytes = complete_bel.as_bytes();
        let (event, consumed) = try_disambiguate_osc_or_alt_bracket(
            complete_bel_bytes,
            MaybeMore::KernelDrained,
        )
        .expect("Should parse complete OSC");
        assert_eq!(event, VT100InputEventIR::Ignored);
        assert_eq!(consumed.as_usize(), complete_bel_bytes.len());

        // In-flight payload with KernelDrained: waits
        let inflight_payload = format!("{OSC_START}11;rgb:00/00/00");
        assert_eq!(
            try_disambiguate_osc_or_alt_bracket(
                inflight_payload.as_bytes(),
                MaybeMore::KernelDrained
            ),
            None
        );

        // Complete OSC 52 clipboard with BEL: generated using Tier 3 OscSequence
        // generator
        let osc52_bel = OscSequence::ClipboardSet {
            target: ClipboardTarget::System,
            data: "Hello".to_string(),
        }
        .to_string();
        let osc52_bel_bytes = osc52_bel.as_bytes();
        let (event, consumed) = try_disambiguate_osc_or_alt_bracket(
            osc52_bel_bytes,
            MaybeMore::KernelDrained,
        )
        .expect("Should parse complete OSC 52 with BEL");
        assert_eq!(event, VT100InputEventIR::Ignored);
        assert_eq!(consumed.as_usize(), osc52_bel_bytes.len());

        // Complete OSC 52 clipboard with 7-bit ST: emits Ignored
        let target_char = char::from(CLIPBOARD_TARGET_CLIPBOARD);
        let osc52_st = format!(
            "{OSC_START}{OSC_CODE_CLIPBOARD}{OSC_DELIMITER}{target_char}{OSC_DELIMITER}SGVsbG8={OSC_TERMINATOR_ST}"
        );
        let osc52_st_bytes = osc52_st.as_bytes();
        let (event, consumed) = try_disambiguate_osc_or_alt_bracket(
            osc52_st_bytes,
            MaybeMore::KernelMayHaveMore,
        )
        .expect("Should parse complete OSC 52 with ST");
        assert_eq!(event, VT100InputEventIR::Ignored);
        assert_eq!(consumed.as_usize(), osc52_st_bytes.len());

        // Complete OSC 52 with UTF-8 checkmark continuation byte 0x9C: emits Ignored
        let osc52_checkmark = format!(
            "{OSC_START}{OSC_CODE_CLIPBOARD}{OSC_DELIMITER}{target_char}{OSC_DELIMITER}\u{2713}{OSC_TERMINATOR_BEL}"
        );
        let osc52_checkmark_bytes = osc52_checkmark.as_bytes();
        let (event, consumed) = try_disambiguate_osc_or_alt_bracket(
            osc52_checkmark_bytes,
            MaybeMore::KernelDrained,
        )
        .expect("Should parse complete OSC 52 with UTF-8 continuation byte");
        assert_eq!(event, VT100InputEventIR::Ignored);
        assert_eq!(consumed.as_usize(), osc52_checkmark_bytes.len());
    }
}
