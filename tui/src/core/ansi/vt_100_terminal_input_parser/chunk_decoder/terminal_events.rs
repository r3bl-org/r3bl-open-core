// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Terminal event parsing from [`ANSI`] sequences.
//!
//! This module handles terminal-level events like window resize, focus changes, bracketed
//! paste mode notifications, and terminal color query responses.
//!
//! ## Where You Are in the Pipeline
//!
//! For the full data flow, see the [parent module documentation]. This diagram shows
//! where `terminal_events.rs` fits:
//!
//! ```text
//! MioPollWorker (reads stdin into read_buffer)
//!    │
//!    │ ChunkFramer::process_incoming_bytes(read_buffer, maybe_more)
//!    ▼
//! chunk_decoder (try_decode_input_event)
//!    │ (routes terminal event sequences here)
//! ┌──▼──────────────────────────────────────────┐  ┌──────────────────┐
//! │  terminal_events.rs                         ◄──┤ **YOU ARE HERE** │
//! │  • Parse window resize events               │  └──────────────────┘
//! │  • Parse focus gained/lost                  │
//! │  • Parse bracketed paste markers            │
//! │  • Parse color reports (OSC 10-14, 17, 19)  │
//! │  • Discard unhandled OSC sequences          │
//! └─────────────────────────────────────────────┘
//!    │
//!    ▼
//! VT100InputEventIR::{ Resize | Focus | Paste | ColorReport }
//!    │
//!    ▼
//! convert_input_event() → InputEvent (returned to application)
//! ```
//!
//! **Navigate**:
//! - ⬆️ **Up**: [`chunk_decoder`] - Main sequence decoding entry point
//! - ➡️ **Peer**: [`keyboard`], [`mouse`], [`utf8`] - Other specialized decoders
//! - 📚 **Types**: [`VT100FocusStateIR`], [`VT100PasteModeIR`], [`TerminalColorReport`],
//!   [`TerminalColorRole`]
//! - 📤 **Converted by**: [`convert_input_event()`] in `protocol_conversion.rs` (not this
//!   module)
//!
//! ## Supported Events
//! - **Window Resize**: `ESC [ 8 ; rows ; cols t`
//! - **Focus Gained**: `ESC [ I`
//! - **Focus Lost**: `ESC [ O`
//! - **Bracketed Paste Start**: `ESC [ 2 0 0 ~`
//! - **Bracketed Paste End**: `ESC [ 2 0 1 ~`
//! - **Terminal Color Reports**: `ESC ] {10..14,17,19} ; <color-spec> (BEL | ST)`
//! - **Unhandled [`OSC`] Responses (Framed & Discarded)**: `ESC ] ... (BEL | ST)`
//!
//! ## Bidirectional Terminal Communication & [`OSC`] Handling
//!
//! Modern terminal emulators write query responses (e.g., color reports, clipboard
//! contents) directly into `stdin` starting with `ESC ]` (`0x1B 0x5D`, [`OSC`] prefix).
//!
//! This module houses [`osc::try_disambiguate_or_alt_bracket()`], which inspects
//! input buffers using [`OscScanResult::scan()`] to distinguish between human keystrokes
//! (`Alt+]`) and incoming terminal responses. Complete color reports are decoded into
//! [`VT100InputEventIR::ColorReport`], while unhandled responses ([`OSC`] 52 clipboard
//! queries, etc.) are safely absorbed as [`VT100InputEventIR::Ignored`] without leaking
//! bytes into the input stream.
//!
//! For the full architectural mental model, [`ASCII`] data flow diagram, and security
//! rationale on discarded queries, see the [Bidirectional Communication] section in the
//! parent module.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
//! [`chunk_decoder`]: mod@super
//! [`convert_input_event()`]: crate::direct_to_ansi::input::protocol_conversion::convert_input_event
//! [`keyboard`]: mod@super::keyboard
//! [`mouse`]: mod@super::mouse
//! [`osc::try_disambiguate_or_alt_bracket()`]: osc::try_disambiguate_or_alt_bracket
//! [`OSC`]: crate::core::ansi::osc::OscSequence
//! [`OscScanResult::scan()`]: super::osc_scanner::OscScanResult::scan
//! [`TerminalColorReport`]: crate::TerminalColorReport
//! [`TerminalColorRole`]: crate::TerminalColorRole
//! [`utf8`]: mod@super::utf8
//! [`VT100FocusStateIR`]: crate::core::ansi::vt_100_terminal_input_parser::VT100FocusStateIR
//! [`VT100InputEventIR::ColorReport`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR::ColorReport
//! [`VT100InputEventIR::Ignored`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR::Ignored
//! [`VT100PasteModeIR`]: crate::core::ansi::vt_100_terminal_input_parser::VT100PasteModeIR
//! [Bidirectional Communication]: mod@super::super#bidirectional-communication-user-input-vs-terminal-responses
//! [parent module documentation]: mod@crate::vt_100_terminal_input_parser

use super::{super::{ir_event_types::{ParsedInputEventIR, VT100FocusStateIR,
                                     VT100InputEventIR, VT100KeyCodeIR,
                                     VT100KeyModifiersIR, VT100PasteModeIR},
                    maybe_more::MaybeMore},
            csi_scanner::CsiParams,
            osc_scanner::OscScanResult};
use crate::{DEBUG_TUI_SHOW_DIRECT_TO_ANSI, OscSequence, byte_offset,
            core::ansi::constants::{ANSI_CSI_BRACKET, ANSI_ESC,
                                    ANSI_FUNCTION_KEY_TERMINATOR,
                                    ANSI_OSC_CLOSE_BRACKET, FOCUS_GAINED_FINAL,
                                    FOCUS_LOST_FINAL, OSC_PREFIX, OSC_PREFIX_LEN,
                                    PASTE_END_PARSE_PARAM, PASTE_START_PARSE_PARAM,
                                    RESIZE_EVENT_PARSE_PARAM, RESIZE_TERMINATOR},
            vp_height, vp_width};

pub mod csi {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Parses a terminal event sequence ([`CSI`] focus, resize, or bracketed paste).
    ///
    /// # Returns
    ///
    /// - `Some(ParsedInputEventIR)` on success.
    /// - `None` if the sequence is incomplete or unrecognized.
    ///
    /// # Handled Sequences
    ///
    /// - `ESC [ 8 ; 2 4 ; 8 0 t` - Window resize to 24 rows × 80 columns
    /// - `ESC [ I` - Terminal gained focus
    /// - `ESC [ O` - Terminal lost focus
    /// - `ESC [ 2 0 0 ~` - Bracketed paste start
    /// - `ESC [ 2 0 1 ~` - Bracketed paste end
    ///
    /// [`CSI`]: crate::CsiSequence
    #[must_use]
    pub fn parse(chunk: &[u8]) -> Option<ParsedInputEventIR> {
        match chunk {
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
            [ANSI_ESC, ANSI_CSI_BRACKET, _, ..] => parse_csi_terminal_parameters(chunk),
            _ => None,
        }
    }

    /// Parse [`CSI`] sequences with parameters for terminal events.
    ///
    /// [`CSI`]: crate::CsiSequence
    fn parse_csi_terminal_parameters(chunk: &[u8]) -> Option<ParsedInputEventIR> {
        let csi_params = CsiParams::try_extract(chunk)?;
        let event = parse_params(&csi_params.params, csi_params.final_byte)?;
        Some(ParsedInputEventIR::new(event, csi_params.total_consumed))
    }

    /// Parses extracted [`CSI`] parameters into a terminal [`VT100InputEventIR`].
    ///
    /// [`CSI`]: crate::CsiSequence
    /// [`VT100InputEventIR`]: super::VT100InputEventIR
    fn parse_params(param_slice: &[u16], final_byte: u8) -> Option<VT100InputEventIR> {
        match (param_slice, final_byte) {
            ([RESIZE_EVENT_PARSE_PARAM, rows, columns], RESIZE_TERMINATOR) => {
                // Window resize: `CSI 8 ; rows ; cols t`.
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
    /// # Inbound [`OSC`] Handling: Color Responses vs Discarded Sequences
    ///
    /// When [`OscScanResult::scan()`] detects a complete [`OSC`] sequence on [`stdin`]:
    ///
    /// 1. **Terminal Color Reports ([`OSC`] 10, 11, 12, 13, 14, 17, 19)**: Decoded into
    ///    [`VT100InputEventIR::ColorReport`] containing [`TerminalColorReport`] and
    ///    [`TerminalColorRole`]. This allows the TUI engine and applications to adapt
    ///    palettes, cursor styles, and highlight colors dynamically.
    /// 2. **[`OSC`] 52 (Inbound Clipboard Read) & Other Sequences**: Mapped to
    ///    [`VT100InputEventIR::Ignored`] and absorbed to prevent machine-level sequence
    ///    bytes from leaking into the application event stream as physical keystrokes.
    ///    For detailed security rationale on discarding inbound clipboard queries, see
    ///    [Why are [`OSC`] 52 read queries discarded?][osc-52-discard-rationale].
    ///
    /// **Diagnostics**: When [`DEBUG_TUI_SHOW_DIRECT_TO_ANSI`] is enabled, parsing or
    /// absorbing an [`OSC`] sequence logs structured telemetry with both raw hex bytes
    /// and string representation.
    ///
    /// [`DEBUG_TUI_SHOW_DIRECT_TO_ANSI`]: crate::DEBUG_TUI_SHOW_DIRECT_TO_ANSI
    /// [`MaybeMore`]: crate::core::ansi::vt_100_terminal_input_parser::MaybeMore
    /// [`OSC`]: crate::core::ansi::osc::OscSequence
    /// [`OscScanResult::InvalidSyntax`]: super::super::osc_scanner::OscScanResult::InvalidSyntax
    /// [`OscScanResult::scan()`]: super::super::osc_scanner::OscScanResult::scan
    /// [`stdin`]: std::io::stdin
    /// [`TerminalColorReport`]: crate::TerminalColorReport
    /// [`TerminalColorRole`]: crate::TerminalColorRole
    /// [`VT100InputEventIR::ColorReport`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR::ColorReport
    /// [`VT100InputEventIR::Ignored`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR::Ignored
    /// [osc-52-discard-rationale]: mod@crate::core::ansi::vt_100_terminal_input_parser#why-are-osc-52-read-queries-discarded
    #[must_use]
    pub fn try_disambiguate_or_alt_bracket(
        chunk: &[u8],
        maybe_more: MaybeMore,
    ) -> Option<ParsedInputEventIR> {
        if !chunk.starts_with(OSC_PREFIX) {
            return None;
        }

        // Route based on lexical scanner outcome.
        // Note: When buffer is lone `ESC ]` (len == 2), `scan` performs 0 iterations and
        // returns `IncompleteDigits`, seamlessly evaluating the `maybe_more` check below.
        match OscScanResult::scan(chunk) {
            OscScanResult::Complete(consumed) => {
                let osc_seq_bytes = &chunk[..consumed.as_usize()];
                let event = parse_osc_response(osc_seq_bytes);
                DEBUG_TUI_SHOW_DIRECT_TO_ANSI.then(|| {
                    let len = consumed.as_usize();
                    // % is Display, ? is Debug.
                    tracing::warn! {
                        message = "try_disambiguate_or_alt_bracket - parsed OSC sequence from stdin",
                        event = ?event,
                        raw_osc_hex = %format!("{:02X?}", &chunk[..len]),
                        raw_osc_str = %String::from_utf8_lossy(&chunk[..len]),
                        consumed_bytes = len,
                    };
                });
                Some(ParsedInputEventIR::new(event, consumed))
            }
            OscScanResult::InvalidSyntax => {
                // Violated OSC syntax; cannot be OSC. Emit Alt+] (2 bytes) and leave
                // trailing bytes in buffer for next cycle.
                Some(alt_bracket_parsed_event())
            }
            OscScanResult::IncompleteDigits => match maybe_more {
                // In-flight burst; wait for possible delimiter/payload.
                MaybeMore::KernelMayHaveMore => None,
                // Stream drained before delimiter arrived. Human typed Alt+] (alone or
                // with digits). Emit Alt+] (2 bytes) and leave any trailing digits in
                // buffer.
                MaybeMore::KernelDrained => Some(alt_bracket_parsed_event()),
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

    /// Helper to construct a parsed `Alt+]` input event consuming [`OSC_PREFIX_LEN`] (2
    /// bytes).
    ///
    /// [`OSC_PREFIX_LEN`]: crate::core::ansi::constants::OSC_PREFIX_LEN
    #[must_use]
    pub fn alt_bracket_parsed_event() -> ParsedInputEventIR {
        ParsedInputEventIR::new(alt_bracket_event(), byte_offset(OSC_PREFIX_LEN))
    }

    /// Helper to construct an `Alt+]` key event.
    #[must_use]
    pub fn alt_bracket_event() -> VT100InputEventIR {
        VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Char(char::from(ANSI_OSC_CLOSE_BRACKET)),
            modifiers: VT100KeyModifiersIR::ALT,
        }
    }

    /// Parses a complete [`OSC`] sequence into a [`VT100InputEventIR`].
    ///
    /// - `OSC 10, 11, 12, 13, 14, 17, 19`: Decodes terminal color responses
    ///   ([`TerminalColorRole`]) into [`VT100InputEventIR::ColorReport`]. These responses
    ///   arrive asynchronously on `stdin` in reply to queries constructed via
    ///   [`OscSequence::ColorQuery`] or emitted via [`OscSender::send_color_query`].
    /// - `OSC 52`: Inbound clipboard responses are ignored
    ///   ([`VT100InputEventIR::Ignored`]).
    /// - All other or malformed [`OSC`] sequences: safely ignored
    ///   ([`VT100InputEventIR::Ignored`]).
    ///
    /// [`OSC`]: crate::core::ansi::osc::OscSequence
    /// [`OscSender::send_color_query`]: crate::core::ansi::osc::OscSender::send_color_query
    /// [`OscSequence::ColorQuery`]: crate::core::ansi::osc::OscSequence::ColorQuery
    /// [`TerminalColorRole`]: crate::TerminalColorRole
    /// [`VT100InputEventIR::ColorReport`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR::ColorReport
    /// [`VT100InputEventIR::Ignored`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR::Ignored
    /// [`VT100InputEventIR`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR
    #[must_use]
    pub fn parse_osc_response(osc_seq_bytes: &[u8]) -> VT100InputEventIR {
        match OscSequence::try_parse(osc_seq_bytes) {
            Some(OscSequence::ColorReport(report)) => {
                VT100InputEventIR::ColorReport(report)
            }
            _ => VT100InputEventIR::Ignored,
        }
    }
}

/// Unit tests for terminal event parsing (focus, resize, bracketed paste, color reports).
///
/// These tests use generator functions instead of hardcoded magic strings to ensure
/// consistency between sequence generation and parsing.
#[cfg(test)]
mod tests {
    use super::{csi::parse as parse_terminal_event,
                osc::{alt_bracket_parsed_event,
                      try_disambiguate_or_alt_bracket as try_disambiguate_osc_or_alt_bracket},
                *};
    use crate::{ClipboardTarget, MAX_OSC_SEQUENCE_LENGTH, OscSequence, RgbValue,
                TerminalColorReport, TerminalColorRole,
                core::ansi::{constants::{CLIPBOARD_TARGET_CLIPBOARD,
                                         OSC_CODE_CLIPBOARD,
                                         OSC_CODE_COLOR_REPORT_BACKGROUND,
                                         OSC_CODE_COLOR_REPORT_FOREGROUND,
                                         OSC_CODE_TITLE_AND_ICON,
                                         OSC_COLOR_SPEC_RGB_PREFIX, OSC_DELIMITER,
                                         OSC_START, OSC_TERMINATOR_BEL,
                                         OSC_TERMINATOR_ST},
                             generator::generate_keyboard_sequence}};

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
        // Test: incomplete sequence (too short).
        assert_eq!(parse_terminal_event(&[ANSI_ESC]), None);

        // Test: sequence without CSI start.
        assert_eq!(parse_terminal_event(b"abc"), None);

        // Test: empty buffer.
        assert_eq!(parse_terminal_event(b""), None);

        // Test: incomplete CSI prefix (only ESC [).
        assert_eq!(parse_terminal_event(b"\x1b["), None);

        // Test: CSI sequence with unrecognized function parameter (not 200/201).
        assert_eq!(parse_terminal_event(b"\x1b[999~"), None);

        // Test: malformed resize parameter count (missing column parameter).
        assert_eq!(parse_terminal_event(b"\x1b[8;24t"), None);

        // Test: valid CSI sequence handled by keyboard parser, not terminal_events.
        assert_eq!(parse_terminal_event(b"\x1b[A"), None);
    }

    #[test]
    fn test_try_disambiguate_osc_or_alt_bracket() {
        // Lone Alt+] with KernelDrained: emits Alt+] (2 bytes).
        let parsed =
            try_disambiguate_osc_or_alt_bracket(OSC_PREFIX, MaybeMore::KernelDrained)
                .expect("Should emit Alt+]");
        assert_eq!(parsed, alt_bracket_parsed_event());

        // Lone Alt+] with KernelMayHaveMore: waits.
        assert_eq!(
            try_disambiguate_osc_or_alt_bracket(OSC_PREFIX, MaybeMore::KernelMayHaveMore),
            None
        );

        // Alt+] followed by invalid syntax: emits Alt+] (2 bytes).
        let invalid_syntax_seq = [OSC_PREFIX, b"a"].concat();
        let parsed = try_disambiguate_osc_or_alt_bracket(
            &invalid_syntax_seq,
            MaybeMore::KernelMayHaveMore,
        )
        .expect("Should emit Alt+] on invalid syntax");
        assert_eq!(parsed, alt_bracket_parsed_event());

        // Alt+] followed by digits with KernelDrained: emits Alt+] (2 bytes).
        let digits_seq = [OSC_PREFIX, b"5"].concat();
        let parsed =
            try_disambiguate_osc_or_alt_bracket(&digits_seq, MaybeMore::KernelDrained)
                .expect("Should emit Alt+] when drained");
        assert_eq!(parsed, alt_bracket_parsed_event());

        // Alt+] followed by digits with KernelMayHaveMore: waits.
        assert_eq!(
            try_disambiguate_osc_or_alt_bracket(
                &digits_seq,
                MaybeMore::KernelMayHaveMore
            ),
            None
        );

        // Candidate OSC complete with BEL: emits ColorReport for OSC 11.
        let complete_bel = format!(
            "{OSC_START}{OSC_CODE_COLOR_REPORT_BACKGROUND}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}00/00/00{OSC_TERMINATOR_BEL}"
        );
        let complete_bel_bytes = complete_bel.as_bytes();
        let parsed = try_disambiguate_osc_or_alt_bracket(
            complete_bel_bytes,
            MaybeMore::KernelDrained,
        )
        .expect("Should parse complete OSC");
        assert_eq!(
            parsed.event,
            VT100InputEventIR::ColorReport(TerminalColorReport {
                role: TerminalColorRole::Background,
                color: RgbValue::from_u8(0, 0, 0),
            })
        );
        assert_eq!(parsed.consumed_usize(), complete_bel_bytes.len());

        // In-flight payload with KernelDrained: waits
        let inflight_payload = format!(
            "{OSC_START}{OSC_CODE_COLOR_REPORT_BACKGROUND}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}00/00/00"
        );
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

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_parse_osc_color_reports() {
        let test_cases = [
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}1e1e/2a2a/3b3b{OSC_TERMINATOR_BEL}",
                    TerminalColorRole::Foreground.as_str()
                ),
                TerminalColorRole::Foreground,
                RgbValue::from_u8(0x1e, 0x2a, 0x3b),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}1e/2a/3b{OSC_TERMINATOR_ST}",
                    TerminalColorRole::Foreground.as_str()
                ),
                TerminalColorRole::Foreground,
                RgbValue::from_u8(0x1e, 0x2a, 0x3b),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}1/2/3{OSC_TERMINATOR_BEL}",
                    TerminalColorRole::Background.as_str()
                ),
                TerminalColorRole::Background,
                RgbValue::from_u8(17, 34, 51),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}111/222/333{OSC_TERMINATOR_BEL}",
                    TerminalColorRole::Background.as_str()
                ),
                TerminalColorRole::Background,
                RgbValue::from_u8(0x11, 0x22, 0x33),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}#1e2a3b{OSC_TERMINATOR_ST}",
                    TerminalColorRole::Background.as_str()
                ),
                TerminalColorRole::Background,
                RgbValue::from_u8(0x1e, 0x2a, 0x3b),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}#123{OSC_TERMINATOR_BEL}",
                    TerminalColorRole::Background.as_str()
                ),
                TerminalColorRole::Background,
                RgbValue::from_u8(17, 34, 51),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}ffff/0000/0000{OSC_TERMINATOR_BEL}",
                    TerminalColorRole::Cursor.as_str()
                ),
                TerminalColorRole::Cursor,
                RgbValue::from_u8(255, 0, 0),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}00/ff/00{OSC_TERMINATOR_ST}",
                    TerminalColorRole::MouseForeground.as_str()
                ),
                TerminalColorRole::MouseForeground,
                RgbValue::from_u8(0, 255, 0),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}#0000ff{OSC_TERMINATOR_BEL}",
                    TerminalColorRole::MouseBackground.as_str()
                ),
                TerminalColorRole::MouseBackground,
                RgbValue::from_u8(0, 0, 255),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}3333/4444/5555{OSC_TERMINATOR_ST}",
                    TerminalColorRole::Highlight.as_str()
                ),
                TerminalColorRole::Highlight,
                RgbValue::from_u8(0x33, 0x44, 0x55),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}#ffffff{OSC_TERMINATOR_BEL}",
                    TerminalColorRole::HighlightForeground.as_str()
                ),
                TerminalColorRole::HighlightForeground,
                RgbValue::from_u8(255, 255, 255),
            ),
            (
                format!(
                    "{OSC_START}{}?{OSC_COLOR_SPEC_RGB_PREFIX}00/00/00{OSC_TERMINATOR_BEL}",
                    TerminalColorRole::Background.as_str()
                ),
                TerminalColorRole::Background,
                RgbValue::from_u8(0, 0, 0),
            ),
            (
                format!(
                    "{OSC_START}{}{OSC_DELIMITER}RGB:1e/2a/3b{OSC_TERMINATOR_ST}",
                    TerminalColorRole::Foreground.as_str()
                ),
                TerminalColorRole::Foreground,
                RgbValue::from_u8(0x1e, 0x2a, 0x3b),
            ),
        ];

        for (seq, expected_role, expected_color) in test_cases {
            let parsed = try_disambiguate_osc_or_alt_bracket(
                seq.as_bytes(),
                MaybeMore::KernelDrained,
            )
            .expect("Should parse valid OSC color report");
            assert_eq!(
                parsed.event,
                VT100InputEventIR::ColorReport(TerminalColorReport {
                    role: expected_role,
                    color: expected_color,
                })
            );
            assert_eq!(parsed.consumed_usize(), seq.len());
        }
    }

    #[test]
    fn test_parse_osc_malformed_and_unhandled_emits_ignored() {
        // Malformed color payloads emit Ignored.
        let malformed_cases = [
            format!(
                "{OSC_START}{OSC_CODE_COLOR_REPORT_FOREGROUND}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}xyz/12/34{OSC_TERMINATOR_BEL}"
            ),
            format!(
                "{OSC_START}{OSC_CODE_COLOR_REPORT_BACKGROUND}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}12/34{OSC_TERMINATOR_BEL}"
            ),
            format!(
                "{OSC_START}{OSC_CODE_COLOR_REPORT_FOREGROUND}{OSC_DELIMITER}#12345{OSC_TERMINATOR_BEL}"
            ),
            format!(
                "{OSC_START}{OSC_CODE_COLOR_REPORT_BACKGROUND}{OSC_DELIMITER}notacolor{OSC_TERMINATOR_BEL}"
            ),
            format!(
                "{OSC_START}{OSC_CODE_COLOR_REPORT_BACKGROUND}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}1/2/3/4{OSC_TERMINATOR_BEL}"
            ),
            format!(
                "{OSC_START}{OSC_CODE_COLOR_REPORT_BACKGROUND}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}12345/00/00{OSC_TERMINATOR_BEL}"
            ),
        ];
        for malformed in malformed_cases {
            let parsed = try_disambiguate_osc_or_alt_bracket(
                malformed.as_bytes(),
                MaybeMore::KernelDrained,
            )
            .expect("Should consume malformed OSC");
            assert_eq!(parsed.event, VT100InputEventIR::Ignored);
            assert_eq!(parsed.consumed_usize(), malformed.len());
        }

        // Unhandled OSC commands emit Ignored.
        let unhandled_cases = [
            format!(
                "{OSC_START}{OSC_CODE_TITLE_AND_ICON}{OSC_DELIMITER}Window Title{OSC_TERMINATOR_BEL}"
            ),
            format!(
                "{OSC_START}4{OSC_DELIMITER}0{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}00/00/00{OSC_TERMINATOR_BEL}"
            ),
        ];
        for unhandled in unhandled_cases {
            let parsed = try_disambiguate_osc_or_alt_bracket(
                unhandled.as_bytes(),
                MaybeMore::KernelDrained,
            )
            .expect("Should consume unhandled OSC");
            assert_eq!(parsed.event, VT100InputEventIR::Ignored);
            assert_eq!(parsed.consumed_usize(), unhandled.len());
        }
    }

    #[test]
    fn test_try_disambiguate_non_osc_and_runaway() {
        // Non-OSC buffer: returns None immediately.
        assert_eq!(
            try_disambiguate_osc_or_alt_bracket(b"", MaybeMore::KernelDrained),
            None
        );
        assert_eq!(
            try_disambiguate_osc_or_alt_bracket(b"abc", MaybeMore::KernelDrained),
            None
        );
        assert_eq!(
            try_disambiguate_osc_or_alt_bracket(b"\x1b[", MaybeMore::KernelDrained),
            None
        );

        // Runaway OSC sequence exceeding MAX_OSC_SEQUENCE_LENGTH: returns None
        // (defers to unparsed buffer classification circuit breaker).
        let mut runaway = Vec::with_capacity(MAX_OSC_SEQUENCE_LENGTH + 10);
        let prefix = format!("{OSC_START}{OSC_CODE_CLIPBOARD}{OSC_DELIMITER}");
        runaway.extend_from_slice(prefix.as_bytes());
        runaway.resize(MAX_OSC_SEQUENCE_LENGTH + 1, b'a');
        assert_eq!(
            try_disambiguate_osc_or_alt_bracket(&runaway, MaybeMore::KernelDrained),
            None
        );
    }
}
