// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Operating System Command ([`OSC`]) codes for terminal control.
//!
//! [`OSC`] sequences allow child processes to send commands to the terminal emulator
//! for features that affect the terminal's operating system integration, such as
//! window titles, notifications, hyperlinks, and clipboard management.
//!
//! # Data Flow
//!
//! **Bidirectional (Child Process <-> [`PTY`] <-> Terminal Emulator)**:
//!
//! Historically, [`OSC`] sequences were considered unidirectional (the child process
//! sends commands to the terminal to set titles, hyperlinks, or notifications). In modern
//! terminals, however, [`OSC`] sequences are frequently **bidirectional**:
//!
//! - **Queries to [`stdout`]**: A TUI app writes queries to the terminal (e.g., color
//!   queries `OSC 10`/`11`, clipboard queries `OSC 52`).
//! - **Responses to [`stdin`]**: The terminal emulator synthesizes responses and writes
//!   them back into [`stdin`].
//!
//! For a comprehensive explanation of bidirectional terminal communication and how
//! incoming [`OSC`] responses are safely parsed and framed on [`stdin`], see the
//! [`vt_100_terminal_input_parser`] module.
//!
//! # Structure
//! [`OSC`] sequences follow the pattern: `ESC ] code ; parameters ST`
//! - Start with [`ESC`] (0x1B) followed by `]`
//! - Numeric code identifying the command type
//! - Parameters separated by `;`
//! - End with String Terminator (ST): `ESC \` or BEL (0x07)
//!
//! # Common Uses
//! - **Window Management**: Set window title and tab names
//! - **Hyperlinks**: Create clickable links in terminal output
//! - **Notifications**: Send desktop notifications (terminal-dependent)
//! - **Clipboard**: Access system clipboard (security-restricted)
//!
//! # Examples
//! - `ESC ] 0 ; My Title ESC \` - Set both window title and tab name
//! - `ESC ] 2 ; Window Title ESC \` - Set window title only
//! - `ESC ] 8 ; ; https://example.com ESC \ Link Text ESC ] 8 ; ; ESC \` - Create
//!   hyperlink
//! - `ESC ] 52 ; c ; SGVsbG8= BEL` - Set system clipboard content
//!
//! [`DSR`]: crate::DsrSequence
//! [`ESC`]: crate::EscSequence
//! [`OSC`]: crate::core::ansi::osc::OscSequence
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`ST`]: OSC_TERMINATOR_ST
//! [`stdin`]: std::io::stdin
//! [`stdout`]: std::io::stdout
//! [`vt_100_terminal_input_parser`]: crate::core::ansi::vt_100_terminal_input_parser

use super::osc_color::try_parse_color_spec;
use crate::{OSC_PREFIX, Pc, TerminalColorReport, TerminalColorRole,
            core::{ansi::constants::{ANSI_BEL, ANSI_ESC, ANSI_ST_FINAL,
                                     CLIPBOARD_TARGET_CLIPBOARD,
                                     CLIPBOARD_TARGET_PRIMARY, OSC_CODE_CLIPBOARD,
                                     OSC_CODE_HYPERLINK, OSC_CODE_ICON,
                                     OSC_CODE_PROGRESS, OSC_CODE_TITLE,
                                     OSC_CODE_TITLE_AND_ICON,
                                     OSC_COLOR_SPEC_CHANNEL_SEPARATOR,
                                     OSC_COLOR_SPEC_RGB_PREFIX, OSC_DELIMITER,
                                     OSC_HYPERLINK_END, OSC_PROGRESS_END,
                                     OSC_PROGRESS_PERCENT_CLEAR,
                                     OSC_PROGRESS_STATE_CLEAR,
                                     OSC_PROGRESS_STATE_UPDATE,
                                     OSC_PROGRESS_SUBCOMMAND, OSC_QUERY,
                                     OSC_QUERY_STR, OSC_START, OSC_TERMINATOR_BEL,
                                     OSC_TERMINATOR_ST, OSC_TITLE_END},
                   common::fast_stringify::{BufTextStorage, FastStringify}},
            generate_impl_display_for_fast_stringify, ok};
use base64::prelude::*;
use std::{fmt::{self, Write as _},
          str::FromStr};

/// [`OSC`] ([`OSC` spec]) sequence builder enum that provides type-safe construction of
/// Operating System Command sequences.
///
/// This enum follows the same pattern as [`CsiSequence`] and [`EscSequence`], providing
/// a structured way to build [`OSC`] sequences instead of manual string formatting.
///
/// [`OSC`] sequences follow the format: `ESC ] code ; parameters ST`
/// where ST is the String Terminator (ST): `ESC \` or BEL.
///
/// [`CsiSequence`]: crate::CsiSequence
/// [`ESC`]: crate::EscSequence
/// [`EscSequence`]: crate::EscSequence
/// [`OSC` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#OSC
/// [`OSC`]: crate::core::ansi::osc::OscSequence
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OscSequence {
    /// `ESC ] 0 ; title ST` - Set Title and Icon
    ///
    /// [`OSC`]: crate::core::ansi::osc::OscSequence
    SetTitleAndIcon(String),

    /// `ESC ] 1 ; icon ST` - Set Icon
    ///
    /// [`OSC`]: crate::core::ansi::osc::OscSequence
    SetIcon(String),

    /// `ESC ] 2 ; title ST` - Set Title
    ///
    /// [`OSC`]: crate::core::ansi::osc::OscSequence
    SetTitle(String),

    /// `ESC ] 8 ; [id] ; uri ST` - Hyperlink Start
    ///
    /// [`OSC`]: crate::core::ansi::osc::OscSequence
    HyperlinkStart {
        uri: String,
        maybe_link_correlation_id: Option<String>,
    },

    /// `ESC ] 8 ; ; ST` - Hyperlink End
    ///
    /// [`OSC`]: crate::core::ansi::osc::OscSequence
    HyperlinkEnd,

    /// `ESC ] 9 ; 4 ; 1 ; percent ST` - Progress Update
    ///
    /// [`OSC`]: crate::core::ansi::osc::OscSequence
    ProgressUpdate(Pc),

    /// `ESC ] 9 ; 4 ; 0 ; 0 ST` - Progress Cleared
    ///
    /// [`OSC`]: crate::core::ansi::osc::OscSequence
    ProgressCleared,

    /// `ESC ] 52 ; target ; <base64_data> ST` - Write text to the system or primary
    /// clipboard.
    ///
    /// [`OSC`]: crate::core::ansi::osc::OscSequence
    ClipboardSet {
        target: ClipboardTarget,
        data: String,
    },

    /// `ESC ] <code> ; ? BEL` - Query terminal color for a specific role (e.g.
    /// [`TerminalColorRole::Background`] -> `OSC 11`, [`TerminalColorRole::Foreground`]
    /// -> `OSC 10`).
    ///
    /// When emitted (e.g. via [`OscSender::send_color_query`]), the terminal emulator
    /// responds asynchronously over `stdin`. That response is framed by
    /// [`vt_100_terminal_input_parser`] and parsed by [`parse_osc_response`] into
    /// [`InputEvent::TerminalColor`].
    ///
    /// > ⚠️ **Platform Availability**: Inbound decoding of terminal color responses is
    /// > Linux-only via [`DirectToAnsiInputDevice`]. On macOS and Windows (Crossterm),
    /// > replies are misparsed as phantom keypresses. For details, see
    /// > [Platform & Backend Availability][osc-platform-support].
    ///
    /// [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
    /// [`InputEvent::TerminalColor`]: crate::InputEvent::TerminalColor
    /// [`OSC`]: crate::core::ansi::osc::OscSequence
    /// [`OscSender::send_color_query`]: crate::core::ansi::osc::OscSender::send_color_query
    /// [`parse_osc_response`]: crate::core::ansi::vt_100_terminal_input_parser::chunk_decoder::terminal_events::osc::parse_osc_response
    /// [`TerminalColorRole::Background`]: crate::TerminalColorRole::Background
    /// [`TerminalColorRole::Foreground`]: crate::TerminalColorRole::Foreground
    /// [`TerminalColorRole`]: crate::TerminalColorRole
    /// [`vt_100_terminal_input_parser`]: crate::core::ansi::vt_100_terminal_input_parser
    /// [osc-platform-support]: mod@crate::core::ansi::osc#platform--backend-availability
    ColorQuery(TerminalColorRole),

    /// `ESC ] <code> ; rgb:rrrr/gggg/bbbb ST` - Terminal color report for a specific
    /// role.
    ///
    /// Emitted when decoding incoming terminal query responses (via
    /// [`parse_osc_response`]) into [`InputEvent::TerminalColor`].
    ///
    /// [`InputEvent::TerminalColor`]: crate::InputEvent::TerminalColor
    /// [`OSC`]: crate::core::ansi::osc::OscSequence
    /// [`parse_osc_response`]: crate::core::ansi::vt_100_terminal_input_parser::chunk_decoder::terminal_events::osc::parse_osc_response
    /// [`TerminalColorReport`]: crate::TerminalColorReport
    ColorReport(TerminalColorReport),
}

impl OscSequence {
    /// Attempts to parse an [`OSC`] sequence from raw bytes.
    ///
    /// Currently parses:
    /// - Color reports (`OSC 10..19` query responses in `rgb:r/g/b` or hex format) ->
    ///   [`OscSequence::ColorReport`]
    /// - Color queries (`OSC 10..19 ; ?`) -> [`OscSequence::ColorQuery`]
    ///
    /// Used by [`parse_osc_response`] to decode incoming terminal responses on `stdin`.
    ///
    /// [`OSC`]: crate::core::ansi::osc::OscSequence
    /// [`OscSequence::ColorQuery`]: crate::core::ansi::osc::OscSequence::ColorQuery
    /// [`OscSequence::ColorReport`]: crate::core::ansi::osc::OscSequence::ColorReport
    /// [`parse_osc_response`]: crate::core::ansi::vt_100_terminal_input_parser::chunk_decoder::terminal_events::osc::parse_osc_response
    #[must_use]
    pub fn try_parse(bytes: &[u8]) -> Option<Self> {
        let body = try_strip_osc_enclosure(bytes)?;
        let body_str = std::str::from_utf8(body).ok()?;

        let (code, payload) = body_str.split_once(
            /* Splits on either delimiter */ [OSC_DELIMITER, OSC_QUERY],
        )?;

        // Try convert code -> color role.
        let role = TerminalColorRole::from_str(code).ok()?;

        // If payload is query string, it is ColorQuery.
        if payload == OSC_QUERY_STR {
            return Some(Self::ColorQuery(role));
        }

        // Otherwise, it is ColorReport.
        let color = try_parse_color_spec(payload.as_bytes())?;
        Some(Self::ColorReport(TerminalColorReport { role, color }))
    }
}

/// Strips the `ESC ]` prefix and the `ST` (`ESC \`) or `BEL` (`\x07`) terminator.
fn try_strip_osc_enclosure(bytes: &[u8]) -> Option<&[u8]> {
    let inner = bytes.strip_prefix(OSC_PREFIX)?;
    match inner {
        [rest @ .., ANSI_ESC, ANSI_ST_FINAL] | [rest @ .., ANSI_BEL] => Some(rest),
        _ => None,
    }
}

impl FastStringify for OscSequence {
    fn write_to_buf(&self, acc: &mut BufTextStorage) -> fmt::Result {
        acc.push_str(OSC_START);
        let terminator = match self {
            OscSequence::ProgressUpdate(percent) => {
                acc.push_str(OSC_CODE_PROGRESS);
                acc.push(OSC_DELIMITER);
                acc.push_str(OSC_PROGRESS_SUBCOMMAND);
                acc.push(OSC_DELIMITER);
                acc.push_str(OSC_PROGRESS_STATE_UPDATE);
                acc.push(OSC_DELIMITER);
                let _ = write!(acc, "{}", **percent);
                OSC_PROGRESS_END
            }
            OscSequence::ProgressCleared => {
                acc.push_str(OSC_CODE_PROGRESS);
                acc.push(OSC_DELIMITER);
                acc.push_str(OSC_PROGRESS_SUBCOMMAND);
                acc.push(OSC_DELIMITER);
                acc.push_str(OSC_PROGRESS_STATE_CLEAR);
                acc.push(OSC_DELIMITER);
                acc.push_str(OSC_PROGRESS_PERCENT_CLEAR);
                OSC_PROGRESS_END
            }
            OscSequence::SetTitleAndIcon(title) => {
                acc.push_str(OSC_CODE_TITLE_AND_ICON);
                acc.push(OSC_DELIMITER);
                acc.push_str(title);
                OSC_TITLE_END
            }
            OscSequence::SetIcon(icon) => {
                acc.push_str(OSC_CODE_ICON);
                acc.push(OSC_DELIMITER);
                acc.push_str(icon);
                OSC_TITLE_END
            }
            OscSequence::SetTitle(title) => {
                acc.push_str(OSC_CODE_TITLE);
                acc.push(OSC_DELIMITER);
                acc.push_str(title);
                OSC_TITLE_END
            }
            OscSequence::HyperlinkStart {
                uri,
                maybe_link_correlation_id,
            } => {
                acc.push_str(OSC_CODE_HYPERLINK);
                acc.push(OSC_DELIMITER);
                if let Some(link_id) = maybe_link_correlation_id {
                    acc.push_str(link_id);
                }
                acc.push(OSC_DELIMITER);
                acc.push_str(uri);
                OSC_HYPERLINK_END
            }
            OscSequence::HyperlinkEnd => {
                acc.push_str(OSC_CODE_HYPERLINK);
                acc.push(OSC_DELIMITER);
                acc.push(OSC_DELIMITER);
                OSC_HYPERLINK_END
            }
            OscSequence::ClipboardSet { target, data } => {
                acc.push_str(OSC_CODE_CLIPBOARD);
                acc.push(OSC_DELIMITER);
                acc.push(target.as_char());
                acc.push(OSC_DELIMITER);
                BASE64_STANDARD.encode_string(data, acc);
                OSC_TERMINATOR_BEL
            }
            OscSequence::ColorQuery(role) => {
                acc.push_str(role.as_str());
                acc.push(OSC_DELIMITER);
                acc.push(OSC_QUERY);
                OSC_TERMINATOR_BEL
            }
            OscSequence::ColorReport(report) => {
                acc.push_str(report.role.as_str());
                acc.push(OSC_DELIMITER);
                let r = report.color.red;
                let g = report.color.green;
                let b = report.color.blue;
                let prefix = OSC_COLOR_SPEC_RGB_PREFIX;
                let sep = char::from(OSC_COLOR_SPEC_CHANNEL_SEPARATOR);
                let _ = write!(
                    acc,
                    "{prefix}{r:02x}{r:02x}{sep}{g:02x}{g:02x}{sep}{b:02x}{b:02x}"
                );
                OSC_TERMINATOR_ST
            }
        };
        acc.push_str(terminator);
        ok!()
    }

    fn write_buf_to_fmt(
        &self,
        acc: &BufTextStorage,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(&acc.clone())
    }
}

generate_impl_display_for_fast_stringify!(OscSequence);

/// Target clipboard buffer for [`OscSequence::ClipboardSet`].
///
/// Corresponds to the target parameter in the [`OSC`] 52 sequence:
/// - [`ClipboardTarget::System`] (`'c'`): standard desktop system clipboard.
/// - [`ClipboardTarget::Primary`] (`'p'`): primary selection buffer (primarily
///   Wayland/Linux).
///
/// [`OSC`]: crate::core::ansi::osc::OscSequence
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardTarget {
    /// System clipboard (`'c'`).
    System,
    /// Primary selection buffer (`'p'`).
    Primary,
}

impl ClipboardTarget {
    /// Returns the single-character representation of the target buffer
    /// ([`CLIPBOARD_TARGET_CLIPBOARD`] or [`CLIPBOARD_TARGET_PRIMARY`]).
    ///
    /// [`CLIPBOARD_TARGET_CLIPBOARD`]: crate::core::ansi::constants::CLIPBOARD_TARGET_CLIPBOARD
    /// [`CLIPBOARD_TARGET_PRIMARY`]: crate::core::ansi::constants::CLIPBOARD_TARGET_PRIMARY
    #[must_use]
    pub fn as_char(&self) -> char {
        match self {
            ClipboardTarget::System => char::from(CLIPBOARD_TARGET_CLIPBOARD),
            ClipboardTarget::Primary => char::from(CLIPBOARD_TARGET_PRIMARY),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::ansi::constants::*, pc};
    use crate::{RgbValue,
                core::ansi::vt_100_terminal_input_parser::{
                    chunk_decoder::terminal_events::osc::parse_osc_response,
                    ir_event_types::VT100InputEventIR}};

    #[test]
    fn test_osc_sequence_set_title_and_icon() {
        let sequence = OscSequence::SetTitleAndIcon("My Title".to_string());
        let result = sequence.to_string();
        let expected = format!(
            "{OSC_START}{OSC_CODE_TITLE_AND_ICON}{OSC_DELIMITER}My Title{OSC_TITLE_END}"
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_set_icon() {
        let sequence = OscSequence::SetIcon("Icon Name".to_string());
        let result = sequence.to_string();
        let expected =
            format!("{OSC_START}{OSC_CODE_ICON}{OSC_DELIMITER}Icon Name{OSC_TITLE_END}");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_set_title() {
        let sequence = OscSequence::SetTitle("Window Title".to_string());
        let result = sequence.to_string();
        let expected = format!(
            "{OSC_START}{OSC_CODE_TITLE}{OSC_DELIMITER}Window Title{OSC_TITLE_END}"
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_hyperlink_start_with_id() {
        let sequence = OscSequence::HyperlinkStart {
            uri: "https://example.com".to_string(),
            maybe_link_correlation_id: Some("link1".to_string()),
        };
        let result = sequence.to_string();
        let expected = format!(
            "{OSC_START}{OSC_CODE_HYPERLINK}{OSC_DELIMITER}link1{OSC_DELIMITER}https://example.com{OSC_HYPERLINK_END}"
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_hyperlink_start_without_id() {
        let sequence = OscSequence::HyperlinkStart {
            uri: "https://example.com".to_string(),
            maybe_link_correlation_id: None,
        };
        let result = sequence.to_string();
        let expected = format!(
            "{OSC_START}{OSC_CODE_HYPERLINK}{OSC_DELIMITER}{OSC_DELIMITER}https://example.com{OSC_HYPERLINK_END}"
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_hyperlink_end() {
        let sequence = OscSequence::HyperlinkEnd;
        let result = sequence.to_string();
        let expected = format!(
            "{OSC_START}{OSC_CODE_HYPERLINK}{OSC_DELIMITER}{OSC_DELIMITER}{OSC_HYPERLINK_END}"
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_empty_strings() {
        let sequence = OscSequence::SetTitle(String::new());
        let result = sequence.to_string();
        let expected =
            format!("{OSC_START}{OSC_CODE_TITLE}{OSC_DELIMITER}{OSC_TITLE_END}");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_special_characters() {
        let sequence = OscSequence::SetTitle("Title with spaces & symbols!".to_string());
        let result = sequence.to_string();
        let expected = format!(
            "{OSC_START}{OSC_CODE_TITLE}{OSC_DELIMITER}Title with spaces & symbols!{OSC_TITLE_END}"
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn test_hyperlink_complete_sequence() {
        let start = OscSequence::HyperlinkStart {
            uri: "https://r3bl.com".to_string(),
            maybe_link_correlation_id: Some("r3bl".to_string()),
        };
        let end = OscSequence::HyperlinkEnd;

        let complete_link = format!("{start}Link Text{end}");
        let expected = format!(
            "{OSC_START}{OSC_CODE_HYPERLINK}{OSC_DELIMITER}r3bl{OSC_DELIMITER}https://r3bl.com{OSC_HYPERLINK_END}Link Text{OSC_START}{OSC_CODE_HYPERLINK}{OSC_DELIMITER}{OSC_DELIMITER}{OSC_HYPERLINK_END}"
        );
        assert_eq!(complete_link, expected);
    }

    #[test]
    fn test_osc_sequence_clipboard_set_ascii() {
        let data = "Hello, World!".to_string();
        let encoded_b64 = BASE64_STANDARD.encode(&data);
        let sequence = OscSequence::ClipboardSet {
            target: ClipboardTarget::System,
            data,
        };
        let result = sequence.to_string();
        let target_char = ClipboardTarget::System.as_char();
        let expected = format!(
            "{OSC_START}{OSC_CODE_CLIPBOARD}{OSC_DELIMITER}{target_char}{OSC_DELIMITER}{encoded_b64}{OSC_TERMINATOR_BEL}"
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_clipboard_set_empty() {
        let sequence = OscSequence::ClipboardSet {
            target: ClipboardTarget::Primary,
            data: String::new(),
        };
        let result = sequence.to_string();
        let target_char = ClipboardTarget::Primary.as_char();
        let expected = format!(
            "{OSC_START}{OSC_CODE_CLIPBOARD}{OSC_DELIMITER}{target_char}{OSC_DELIMITER}{OSC_TERMINATOR_BEL}"
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_clipboard_set_utf8_checkmark() {
        use base64::prelude::*;
        let data = "Task done ✓".to_string();
        let encoded_b64 = BASE64_STANDARD.encode(&data);
        let sequence = OscSequence::ClipboardSet {
            target: ClipboardTarget::System,
            data: data.clone(),
        };
        let result = sequence.to_string();
        let target_char = ClipboardTarget::System.as_char();
        let expected = format!(
            "{OSC_START}{OSC_CODE_CLIPBOARD}{OSC_DELIMITER}{target_char}{OSC_DELIMITER}{encoded_b64}{OSC_TERMINATOR_BEL}"
        );
        assert_eq!(result, expected);

        // Verify base64 decode round-trip.
        let prefix = format!(
            "{OSC_START}{OSC_CODE_CLIPBOARD}{OSC_DELIMITER}{target_char}{OSC_DELIMITER}"
        );
        let payload = result
            .strip_prefix(&prefix)
            .unwrap()
            .strip_suffix(OSC_TERMINATOR_BEL)
            .unwrap();
        let decoded =
            String::from_utf8(BASE64_STANDARD.decode(payload).unwrap()).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_clipboard_target_as_char() {
        assert_eq!(
            ClipboardTarget::System.as_char(),
            char::from(CLIPBOARD_TARGET_CLIPBOARD)
        );
        assert_eq!(
            ClipboardTarget::Primary.as_char(),
            char::from(CLIPBOARD_TARGET_PRIMARY)
        );
    }

    /// Test helper to format an [`OSC`] color query sequence with a specific terminator.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    fn generate_color_query_sequence(
        role: TerminalColorRole,
        terminator: &str,
    ) -> String {
        format!(
            "{OSC_START}{}{OSC_DELIMITER}{OSC_QUERY}{terminator}",
            role.as_str()
        )
    }

    #[test]
    fn test_osc_sequence_color_query_all_roles() {
        let roles = [
            TerminalColorRole::Foreground,
            TerminalColorRole::Background,
            TerminalColorRole::Cursor,
            TerminalColorRole::MouseForeground,
            TerminalColorRole::MouseBackground,
            TerminalColorRole::Highlight,
            TerminalColorRole::HighlightForeground,
        ];

        for role in roles {
            let seq = OscSequence::ColorQuery(role);
            let expected = generate_color_query_sequence(role, OSC_TERMINATOR_BEL);
            assert_eq!(seq.to_string(), expected);
        }
    }

    #[test]
    fn test_osc_sequence_color_report_formatting() {
        let color = RgbValue {
            red: 0x1e,
            green: 0x2a,
            blue: 0x3b,
        };
        let report = TerminalColorReport {
            role: TerminalColorRole::Background,
            color,
        };
        let seq = OscSequence::ColorReport(report);
        let sep = char::from(OSC_COLOR_SPEC_CHANNEL_SEPARATOR);
        let expected = format!(
            "{OSC_START}{}{OSC_DELIMITER}{OSC_COLOR_SPEC_RGB_PREFIX}{:02x}{:02x}{sep}{:02x}{:02x}{sep}{:02x}{:02x}{OSC_TERMINATOR_ST}",
            TerminalColorRole::Background.as_str(),
            color.red,
            color.red,
            color.green,
            color.green,
            color.blue,
            color.blue,
        );
        assert_eq!(seq.to_string(), expected);
    }

    #[test]
    fn test_osc_color_report_parser_roundtrip() {
        let report = TerminalColorReport {
            role: TerminalColorRole::Foreground,
            color: RgbValue {
                red: 0xaa,
                green: 0xbb,
                blue: 0xcc,
            },
        };
        let seq = OscSequence::ColorReport(report);
        let seq_bytes = seq.to_string().into_bytes();

        let parsed = parse_osc_response(&seq_bytes);
        assert_eq!(parsed, VT100InputEventIR::ColorReport(report));
    }

    #[test]
    fn test_osc_sequence_parse_color_report() {
        use crate::RgbValue;

        let report = TerminalColorReport {
            role: TerminalColorRole::Background,
            color: RgbValue::from_u8(0x1e, 0x2a, 0x3b),
        };
        let seq = OscSequence::ColorReport(report);
        let seq_bytes = seq.to_string().into_bytes();

        let parsed = OscSequence::try_parse(&seq_bytes);
        assert_eq!(parsed, Some(seq));
    }

    #[test]
    fn test_osc_sequence_parse_color_query() {
        let seq = OscSequence::ColorQuery(TerminalColorRole::Background);
        let seq_bytes = seq.to_string().into_bytes();

        let parsed = OscSequence::try_parse(&seq_bytes);
        assert_eq!(parsed, Some(seq.clone()));

        // Also test with ST terminator (\x1b\) instead of BEL (\x07).
        let st_seq = generate_color_query_sequence(
            TerminalColorRole::Background,
            OSC_TERMINATOR_ST,
        );
        assert_eq!(OscSequence::try_parse(st_seq.as_bytes()), Some(seq));
    }

    #[test]
    fn test_osc_sequence_parse_invalid() {
        assert_eq!(OscSequence::try_parse(b""), None);
        assert_eq!(OscSequence::try_parse(b"not an osc sequence"), None);

        // Prefix only (no terminator).
        let prefix_only = format!(
            "{OSC_START}{OSC_CODE_COLOR_REPORT_BACKGROUND}{OSC_DELIMITER}{OSC_QUERY}"
        );
        assert_eq!(OscSequence::try_parse(prefix_only.as_bytes()), None);

        // Terminator only (no prefix).
        let terminator_only = format!(
            "{OSC_CODE_COLOR_REPORT_BACKGROUND}{OSC_DELIMITER}{OSC_QUERY}{OSC_TERMINATOR_BEL}"
        );
        assert_eq!(OscSequence::try_parse(terminator_only.as_bytes()), None);

        // No delimiter.
        let no_delimiter =
            format!("{OSC_START}{OSC_CODE_COLOR_REPORT_BACKGROUND}{OSC_TERMINATOR_BEL}");
        assert_eq!(OscSequence::try_parse(no_delimiter.as_bytes()), None);

        // Non-UTF-8 body.
        let non_utf8 = [OSC_START_BYTES, &[0xff], &[OSC_TERMINATOR_BEL_BYTE]].concat();
        assert_eq!(OscSequence::try_parse(&non_utf8), None);

        // Unknown role code.
        let invalid_query =
            format!("{OSC_START}99{OSC_DELIMITER}{OSC_QUERY}{OSC_TERMINATOR_BEL}");
        assert_eq!(OscSequence::try_parse(invalid_query.as_bytes()), None);

        // Invalid color spec payload.
        let invalid_color = format!(
            "{OSC_START}{OSC_CODE_COLOR_REPORT_BACKGROUND}{OSC_DELIMITER}notacolor{OSC_TERMINATOR_BEL}"
        );
        assert_eq!(OscSequence::try_parse(invalid_color.as_bytes()), None);
    }

    #[test]
    fn test_osc_sequence_progress_update() {
        let seq = OscSequence::ProgressUpdate(pc!(50).unwrap());
        let result = seq.to_string();
        let expected = format!(
            "{OSC_START}{OSC_CODE_PROGRESS}{OSC_DELIMITER}{OSC_PROGRESS_SUBCOMMAND}{OSC_DELIMITER}{OSC_PROGRESS_STATE_UPDATE}{OSC_DELIMITER}50{OSC_PROGRESS_END}"
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn test_osc_sequence_progress_cleared() {
        let seq = OscSequence::ProgressCleared;
        let result = seq.to_string();
        let expected = format!(
            "{OSC_START}{OSC_CODE_PROGRESS}{OSC_DELIMITER}{OSC_PROGRESS_SUBCOMMAND}{OSC_DELIMITER}{OSC_PROGRESS_STATE_CLEAR}{OSC_DELIMITER}{OSC_PROGRESS_PERCENT_CLEAR}{OSC_PROGRESS_END}"
        );
        assert_eq!(result, expected);
    }
}
