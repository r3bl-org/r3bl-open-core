// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Terminal event parsing from ANSI sequences.
//!
//! This module handles terminal-level events like window resize, focus changes,
//! and bracketed paste mode notifications.
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
//! ┌──▼───────────────────────────────────────┐  ┌──────────────────┐
//! │  terminal_events.rs                      ◀──┤ **YOU ARE HERE** │
//! │  • Parse window resize events            │  └──────────────────┘
//! │  • Parse focus gained/lost               │
//! │  • Parse bracketed paste markers         │
//! └──────────────────────────────────────────┘
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
//! - **Window Resize**: `CSI 8 ; rows ; cols t`
//! - **Focus Gained**: `CSI I`
//! - **Focus Lost**: `CSI O`
//! - **Bracketed Paste Start**: `ESC [ 200 ~`
//! - **Bracketed Paste End**: `ESC [ 201 ~`
//!
//! [`VT100FocusStateIR`]: super::VT100FocusStateIR
//! [`VT100PasteModeIR`]: super::VT100PasteModeIR
//! [`convert_input_event()`]: crate::direct_to_ansi::input::protocol_conversion::convert_input_event
//! [`keyboard`]: mod@super::keyboard
//! [`mouse`]: mod@super::mouse
//! [`router`]: mod@super::router
//! [`utf8`]: mod@super::utf8
//! [parent module documentation]: mod@super#primary-consumer

use super::ir_event_types::{VT100FocusStateIR, VT100InputEventIR, VT100PasteModeIR};
use crate::{ByteOffset, byte_offset,
            core::ansi::constants::{ANSI_CSI_BRACKET, ANSI_ESC,
                                    ANSI_FUNCTION_KEY_TERMINATOR, ANSI_PARAM_SEPARATOR,
                                    ASCII_DIGIT_0, ASCII_DIGIT_9, FOCUS_GAINED_FINAL,
                                    FOCUS_LOST_FINAL, PASTE_END_PARSE_PARAM,
                                    PASTE_START_PARSE_PARAM, RESIZE_EVENT_PARSE_PARAM,
                                    RESIZE_TERMINATOR}};

/// Parse a terminal event sequence and return an `InputEvent` with bytes consumed if
/// recognized.
///
/// Returns `Some((event, bytes_consumed))` if a complete sequence is parsed,
/// or `None` if the sequence is incomplete or invalid.
///
/// # Handled Sequences
///
/// - `CSI 8;24;80t` - Window resize to 24 rows × 80 columns
/// - `CSI I` - Terminal gained focus
/// - `CSI O` - Terminal lost focus
/// - `ESC [200~` - Bracketed paste start
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

/// Parse `CSI` sequences with parameters for terminal events.
fn parse_csi_terminal_parameters(
    buffer: &[u8],
) -> Option<(VT100InputEventIR, ByteOffset)> {
    // Extract parameters and final byte
    // Format: ESC [ [param;param;...] final_byte
    let mut params = Vec::new();
    let mut current_num = String::new();
    let mut final_byte = 0u8;
    let mut bytes_scanned = 0;

    for (idx, &byte) in buffer[2..].iter().enumerate() {
        bytes_scanned = idx + 1; // Track position relative to buffer[2..]

        // IMPORTANT: We use if/else chains instead of match arms because Rust treats
        // constants in match patterns as variable bindings, not value comparisons.
        // See keyboard.rs for detailed explanation of this pattern.

        if (ASCII_DIGIT_0..=ASCII_DIGIT_9).contains(&byte) {
            // Digit: accumulate in current_num
            current_num.push(byte as char);
        } else if byte == ANSI_PARAM_SEPARATOR {
            // Semicolon: parameter separator
            if !current_num.is_empty() {
                params.push(current_num.parse::<u16>().unwrap_or(0));
                current_num.clear();
            }
        } else if byte == ANSI_FUNCTION_KEY_TERMINATOR || byte == RESIZE_TERMINATOR {
            // Terminal character: '~' for paste events, 't' for resize events
            if !current_num.is_empty() {
                params.push(current_num.parse::<u16>().unwrap_or(0));
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
        let cols = params[2];
        Some((
            VT100InputEventIR::Resize {
                col_width: crate::ColWidth::from(cols),
                row_height: crate::RowHeight::from(rows),
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

/// Unit tests for terminal event parsing (focus, resize, bracketed paste).
///
/// These tests use generator functions instead of hardcoded magic strings to ensure
/// consistency between sequence generation and parsing.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ansi::generator::generate_keyboard_sequence;

    #[test]
    fn test_resize_event() {
        // Round-trip test: Generate sequence from VT100InputEventIR, then parse it back
        let original_event = VT100InputEventIR::Resize {
            row_height: crate::RowHeight::from(24),
            col_width: crate::ColWidth::from(80),
        };
        let seq = generate_keyboard_sequence(&original_event)
            .expect("Failed to generate resize sequence");

        let (parsed_event, bytes_consumed) =
            parse_terminal_event(&seq).expect("Should parse resize");

        assert_eq!(bytes_consumed.as_usize(), seq.len());
        assert_eq!(parsed_event, original_event);
    }

    #[test]
    fn test_focus_events() {
        // Round-trip test: Focus gained
        let original_gained = VT100InputEventIR::Focus(VT100FocusStateIR::Gained);
        let seq_gained = generate_keyboard_sequence(&original_gained)
            .expect("Failed to generate focus gained sequence");

        let (parsed, bytes_consumed) =
            parse_terminal_event(&seq_gained).expect("Should parse focus gained");

        assert_eq!(bytes_consumed.as_usize(), seq_gained.len());
        assert_eq!(parsed, original_gained);

        // Round-trip test: Focus lost
        let original_lost = VT100InputEventIR::Focus(VT100FocusStateIR::Lost);
        let seq_lost = generate_keyboard_sequence(&original_lost)
            .expect("Failed to generate focus lost sequence");

        let (parsed, bytes_consumed) =
            parse_terminal_event(&seq_lost).expect("Should parse focus lost");

        assert_eq!(bytes_consumed.as_usize(), seq_lost.len());
        assert_eq!(parsed, original_lost);
    }

    #[test]
    fn test_bracketed_paste() {
        // Round-trip test: Paste start
        let original_start = VT100InputEventIR::Paste(VT100PasteModeIR::Start);
        let seq_start = generate_keyboard_sequence(&original_start)
            .expect("Failed to generate paste start sequence");

        let (parsed, bytes_consumed) =
            parse_terminal_event(&seq_start).expect("Should parse paste start");

        assert_eq!(bytes_consumed.as_usize(), seq_start.len());
        assert_eq!(parsed, original_start);

        // Round-trip test: Paste end
        let original_end = VT100InputEventIR::Paste(VT100PasteModeIR::End);
        let seq_end = generate_keyboard_sequence(&original_end)
            .expect("Failed to generate paste end sequence");

        let (parsed, bytes_consumed) =
            parse_terminal_event(&seq_end).expect("Should parse paste end");

        assert_eq!(bytes_consumed.as_usize(), seq_end.len());
        assert_eq!(parsed, original_end);
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
}
