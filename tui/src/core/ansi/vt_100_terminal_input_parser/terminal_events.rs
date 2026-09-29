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
//! This module houses [`osc::try_disambiguate_or_alt_bracket()`], which inspects
//! input buffers using [`OscScanResult::scan()`] to distinguish between human keystrokes
//! (such as `Alt+]`) and incoming terminal responses, ensuring complete [`OSC`] payloads
//! are consumed and ignored without leaking text to the screen.
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
//! [`osc::try_disambiguate_or_alt_bracket()`]: osc::try_disambiguate_or_alt_bracket
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`OscScanResult::scan()`]: super::osc_scanner::OscScanResult::scan
//! [`router`]: mod@super::router
//! [`utf8`]: mod@super::utf8
//! [`VT100FocusStateIR`]: super::VT100FocusStateIR
//! [`VT100PasteModeIR`]: super::VT100PasteModeIR
//! [Bidirectional Communication section in the parent module]:
//!     mod@super#bidirectional-communication-user-input-vs-terminal-responses
//! [parent module documentation]: mod@super#primary-consumer

use super::{csi_scanner::extract_csi_params,
            ir_event_types::{ParsedInputEventIR, VT100FocusStateIR, VT100InputEventIR,
                             VT100KeyCodeIR, VT100KeyModifiersIR, VT100PasteModeIR},
            maybe_more::MaybeMore,
            osc_scanner::OscScanResult};
use crate::{DEBUG_TUI_SHOW_DIRECT_TO_ANSI, KeyState, byte_offset,
            core::ansi::constants::{ANSI_CSI_BRACKET, ANSI_ESC,
                                    ANSI_FUNCTION_KEY_TERMINATOR, FOCUS_GAINED_FINAL,
                                    FOCUS_LOST_FINAL, OSC_PREFIX, OSC_PREFIX_LEN,
                                    PASTE_END_PARSE_PARAM, PASTE_START_PARSE_PARAM,
                                    RESIZE_EVENT_PARSE_PARAM, RESIZE_TERMINATOR},
            vp_height, vp_width};

pub mod csi {
    #[allow(clippy::wildcard_imports)]
    use super::*;

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
    pub fn parse(buffer: &[u8]) -> Option<ParsedInputEventIR> {
        match buffer {
            [ANSI_ESC, ANSI_CSI_BRACKET, FOCUS_GAINED_FINAL] => {
                Some(ParsedInputEventIR::new(
                    VT100InputEventIR::Focus(VT100FocusStateIR::Gained),
                    byte_offset(3),
                ))
            }
            [ANSI_ESC, ANSI_CSI_BRACKET, FOCUS_LOST_FINAL] => {
                Some(ParsedInputEventIR::new(
                    VT100InputEventIR::Focus(VT100FocusStateIR::Lost),
                    byte_offset(3),
                ))
            }
            [ANSI_ESC, ANSI_CSI_BRACKET, _, ..] => parse_csi_terminal_parameters(buffer),
            _ => None,
        }
    }

    /// Parse [`CSI`] sequences with parameters for terminal events.
    ///
    /// [`CSI`]: crate::CsiSequence
    fn parse_csi_terminal_parameters(buffer: &[u8]) -> Option<ParsedInputEventIR> {
        let extracted = extract_csi_params(buffer)?;
        let event = parse_params(&extracted.params, extracted.final_byte)?;
        Some(ParsedInputEventIR::new(event, extracted.total_consumed()))
    }

    /// Parses extracted [`CSI`] parameters into a terminal [`VT100InputEventIR`].
    ///
    /// [`CSI`]: crate::CsiSequence
    /// [`VT100InputEventIR`]: super::VT100InputEventIR
    fn parse_params(params: &[u16], final_byte: u8) -> Option<VT100InputEventIR> {
        match (params, final_byte) {
            ([RESIZE_EVENT_PARSE_PARAM, rows, columns], RESIZE_TERMINATOR) => {
                // Window resize: CSI 8 ; rows ; cols t.
                Some(VT100InputEventIR::Resize {
                    col_width: vp_width(*columns),
                    row_height: vp_height(*rows),
                })
            }
            ([PASTE_START_PARSE_PARAM], ANSI_FUNCTION_KEY_TERMINATOR) => {
                // Bracketed paste start: CSI 200 ~.
                Some(VT100InputEventIR::Paste(VT100PasteModeIR::Start))
            }
            ([PASTE_END_PARSE_PARAM], ANSI_FUNCTION_KEY_TERMINATOR) => {
                // Bracketed paste end: CSI 201 ~.
                Some(VT100InputEventIR::Paste(VT100PasteModeIR::End))
            }
            _ => None,
        }
    }
}

pub mod osc {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Disambiguates an incoming `ESC ]` (`0x1B 0x5D`) buffer between an `Alt+]` human
    /// keystroke and a terminal emulator [`OSC`] query response.
    ///
    /// # Disambiguation Rules
    ///
    /// 1. **Rule 1: The [`MaybeMore`] Heuristic**: Terminal emulators emit [`OSC`]
    ///    responses in high-speed single-burst writes, whereas human keystrokes have tens
    ///    of milliseconds between them. If all available input was drained, the sequence
    ///    is incomplete, and it is guaranteed to be human input (e.g., `Alt+]` alone or
    ///    `Alt+] 5`), so the function emits `Alt+]` (2 bytes consumed) and preserves
    ///    trailing bytes for the next parse cycle. If `maybe_more ==
    ///    MaybeMore::KernelMayHaveMore`, the function defers parsing to allow the rest of
    ///    the burst to arrive.
    ///
    /// 2. **Rule 2: Strict [`OSC`] Syntax Validation**: If [`OscScanResult::scan()`]
    ///    returns [`OscScanResult::InvalidSyntax`], the sequence cannot be an [`OSC`]
    ///    response; `Alt+]` is emitted immediately.
    ///
    /// # Why Inbound [`OSC`] Sequences are Quarantined and Absorbed
    ///
    /// When [`OscScanResult::scan()`] detects a complete [`OSC`] sequence on `stdin`, it
    /// is mapped to [`VT100InputEventIR::Ignored`] and absorbed.
    ///
    /// **Terminal Applications Do Not Rely on [`OSC`] 52 Queries**:
    /// - Almost no CLI or TUI programs rely on `OSC 52 ; c ; ?` queries to function. In
    ///   fact, most programs never query [`OSC`] 52 because:
    ///   - Standard terminals (like [`Kitty`], [`Alacritty`], Foot) disable [`OSC`] 52
    ///     queries by default for security reasons (clipboard snooping vulnerabilities).
    ///   - Programs that want clipboard integration either use system CLI helpers
    ///     (`xclip`, `wl-copy`, `pbcopy`), their own internal registers (like
    ///     Vim/Neovim's default unnamed registers), or Bracketed Paste.
    /// - If a program (like Neovim configured with an [`OSC`] 52 plugin) emits `OSC 52 ;
    ///   c ; ?` and receives no response, it simply falls back to its internal register
    ///   without hanging.
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
    /// [`OscScanResult::InvalidSyntax`]:
    ///     super::super::osc_scanner::OscScanResult::InvalidSyntax
    /// [`OscScanResult::scan()`]: super::super::osc_scanner::OscScanResult::scan
    #[must_use]
    pub fn try_disambiguate_or_alt_bracket(
        buffer: &[u8],
        maybe_more: MaybeMore,
    ) -> Option<ParsedInputEventIR> {
        if !buffer.starts_with(OSC_PREFIX) {
            return None;
        }

        // Route based on lexical scanner outcome.
        // Note: When buffer is lone `ESC ]` (len == 2), `scan` performs 0
        // iterations and returns `IncompleteDigits`, seamlessly evaluating the
        // `maybe_more` check below.
        match OscScanResult::scan(buffer) {
            OscScanResult::Complete(consumed) => {
                DEBUG_TUI_SHOW_DIRECT_TO_ANSI.then(|| {
                    let len = consumed.as_usize();
                    // % is Display, ? is Debug.
                    tracing::warn! {
                        message = "try_disambiguate_or_alt_bracket - absorbed OSC sequence from stdin",
                        raw_osc_hex = %format!("{:02X?}", &buffer[..len]),
                        raw_osc_str = %String::from_utf8_lossy(&buffer[..len]),
                        consumed_bytes = len,
                    };
                });
                Some(ParsedInputEventIR::new(
                    VT100InputEventIR::Ignored,
                    consumed,
                ))
            }
            OscScanResult::InvalidSyntax => {
                // Violated OSC syntax; cannot be OSC. Emit Alt+] (2 bytes)
                // and leave trailing bytes in buffer for next cycle.
                Some(ParsedInputEventIR::new(
                    alt_bracket_event(),
                    byte_offset(OSC_PREFIX_LEN),
                ))
            }
            OscScanResult::IncompleteDigits => match maybe_more {
                // In-flight burst; wait for possible delimiter/payload.
                MaybeMore::KernelMayHaveMore => None,
                // Stream drained before delimiter arrived. Human typed Alt+] (alone
                // or with digits). Emit Alt+] (2 bytes) and leave
                // any trailing digits in buffer.
                MaybeMore::KernelDrained => Some(ParsedInputEventIR::new(
                    alt_bracket_event(),
                    byte_offset(OSC_PREFIX_LEN),
                )),
            },
            OscScanResult::IncompletePayload => {
                // Delimiter was already parsed. This is guaranteed to be an in-flight OSC
                // sequence. Always wait for the rest of the payload across reads
                // (bounded by MAX_OSC_SEQUENCE_LENGTH).
                None
            }
            OscScanResult::Runaway => {
                // Defer to classify_unparsed_buffer to trip circuit breaker and purge
                // buffer.
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
}

/// Unit tests for terminal event parsing (focus, resize, bracketed paste).
///
/// These tests use generator functions instead of hardcoded magic strings to ensure
/// consistency between sequence generation and parsing.
#[cfg(test)]
mod tests {
    use super::{csi::parse as parse_terminal_event,
                osc::{alt_bracket_event,
                      try_disambiguate_or_alt_bracket as try_disambiguate_osc_or_alt_bracket},
                *};
    use crate::{ClipboardTarget, OscSequence,
                core::{ansi::{constants::{CLIPBOARD_TARGET_CLIPBOARD,
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

        let ParsedInputEventIR {
            event: parsed_event,
            bytes_consumed,
        } = parse_terminal_event(&sequence).expect("Should parse resize");

        assert_eq!(bytes_consumed.as_usize(), sequence.len());
        assert_eq!(parsed_event, original_event);
    }

    #[test]
    fn test_focus_events() {
        // Round-trip test: Focus gained
        let original_gained = VT100InputEventIR::Focus(VT100FocusStateIR::Gained);
        let sequence_gained = generate_keyboard_sequence(&original_gained)
            .expect("Failed to generate focus gained sequence");

        let ParsedInputEventIR {
            event: parsed_event,
            bytes_consumed,
        } = parse_terminal_event(&sequence_gained).expect("Should parse focus gained");

        assert_eq!(bytes_consumed.as_usize(), sequence_gained.len());
        assert_eq!(parsed_event, original_gained);

        // Round-trip test: Focus lost
        let original_lost = VT100InputEventIR::Focus(VT100FocusStateIR::Lost);
        let sequence_lost = generate_keyboard_sequence(&original_lost)
            .expect("Failed to generate focus lost sequence");

        let ParsedInputEventIR {
            event: parsed_event,
            bytes_consumed,
        } = parse_terminal_event(&sequence_lost).expect("Should parse focus lost");

        assert_eq!(bytes_consumed.as_usize(), sequence_lost.len());
        assert_eq!(parsed_event, original_lost);
    }

    #[test]
    fn test_bracketed_paste() {
        // Round-trip test: Paste start
        let original_start = VT100InputEventIR::Paste(VT100PasteModeIR::Start);
        let sequence_start = generate_keyboard_sequence(&original_start)
            .expect("Failed to generate paste start sequence");

        let ParsedInputEventIR {
            event: parsed_event,
            bytes_consumed,
        } = parse_terminal_event(&sequence_start).expect("Should parse paste start");

        assert_eq!(bytes_consumed.as_usize(), sequence_start.len());
        assert_eq!(parsed_event, original_start);

        // Round-trip test: Paste end
        let original_end = VT100InputEventIR::Paste(VT100PasteModeIR::End);
        let sequence_end = generate_keyboard_sequence(&original_end)
            .expect("Failed to generate paste end sequence");

        let ParsedInputEventIR {
            event: parsed_event,
            bytes_consumed,
        } = parse_terminal_event(&sequence_end).expect("Should parse paste end");

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
    fn test_try_disambiguate_osc_or_alt_bracket() {
        // Lone Alt+] with KernelDrained: emits Alt+]
        let parsed =
            try_disambiguate_osc_or_alt_bracket(OSC_PREFIX, MaybeMore::KernelDrained)
                .expect("Should emit Alt+]");
        assert_eq!(parsed.event, alt_bracket_event());
        assert_eq!(parsed.consumed_usize(), 2);

        // Lone Alt+] with KernelMayHaveMore: waits
        assert_eq!(
            try_disambiguate_osc_or_alt_bracket(OSC_PREFIX, MaybeMore::KernelMayHaveMore),
            None
        );

        // Alt+] followed by invalid syntax: emits Alt+] (2 bytes)
        let invalid_syntax_seq = [OSC_PREFIX, b"a"].concat();
        let parsed = try_disambiguate_osc_or_alt_bracket(
            &invalid_syntax_seq,
            MaybeMore::KernelMayHaveMore,
        )
        .expect("Should emit Alt+] on invalid syntax");
        assert_eq!(parsed.event, alt_bracket_event());
        assert_eq!(parsed.consumed_usize(), 2);

        // Alt+] followed by digits with KernelDrained: emits Alt+] (2 bytes)
        let digits_seq = [OSC_PREFIX, b"5"].concat();
        let parsed =
            try_disambiguate_osc_or_alt_bracket(&digits_seq, MaybeMore::KernelDrained)
                .expect("Should emit Alt+] when drained");
        assert_eq!(parsed.event, alt_bracket_event());
        assert_eq!(parsed.consumed_usize(), 2);

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
        let parsed = try_disambiguate_osc_or_alt_bracket(
            complete_bel_bytes,
            MaybeMore::KernelDrained,
        )
        .expect("Should parse complete OSC");
        assert_eq!(parsed.event, VT100InputEventIR::Ignored);
        assert_eq!(parsed.consumed_usize(), complete_bel_bytes.len());

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
        let parsed = try_disambiguate_osc_or_alt_bracket(
            osc52_bel_bytes,
            MaybeMore::KernelDrained,
        )
        .expect("Should parse complete OSC 52 with BEL");
        assert_eq!(parsed.event, VT100InputEventIR::Ignored);
        assert_eq!(parsed.consumed_usize(), osc52_bel_bytes.len());

        // Complete OSC 52 clipboard with 7-bit ST: emits Ignored
        let target_char = char::from(CLIPBOARD_TARGET_CLIPBOARD);
        let osc52_st = format!(
            "{OSC_START}{OSC_CODE_CLIPBOARD}{OSC_DELIMITER}{target_char}{OSC_DELIMITER}SGVsbG8={OSC_TERMINATOR_ST}"
        );
        let osc52_st_bytes = osc52_st.as_bytes();
        let parsed = try_disambiguate_osc_or_alt_bracket(
            osc52_st_bytes,
            MaybeMore::KernelMayHaveMore,
        )
        .expect("Should parse complete OSC 52 with ST");
        assert_eq!(parsed.event, VT100InputEventIR::Ignored);
        assert_eq!(parsed.consumed_usize(), osc52_st_bytes.len());

        // Complete OSC 52 with UTF-8 checkmark continuation byte 0x9C: emits Ignored
        let osc52_checkmark = format!(
            "{OSC_START}{OSC_CODE_CLIPBOARD}{OSC_DELIMITER}{target_char}{OSC_DELIMITER}\u{2713}{OSC_TERMINATOR_BEL}"
        );
        let osc52_checkmark_bytes = osc52_checkmark.as_bytes();
        let parsed = try_disambiguate_osc_or_alt_bracket(
            osc52_checkmark_bytes,
            MaybeMore::KernelDrained,
        )
        .expect("Should parse complete OSC 52 with UTF-8 continuation byte");
        assert_eq!(parsed.event, VT100InputEventIR::Ignored);
        assert_eq!(parsed.consumed_usize(), osc52_checkmark_bytes.len());
    }
}
