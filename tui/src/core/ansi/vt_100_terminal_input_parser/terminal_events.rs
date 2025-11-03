// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Terminal event parsing from ANSI sequences.
//!
//! This module handles terminal-level events like window resize, focus changes,
//! and bracketed paste mode notifications.
//!
//! Supported events:
//! - **Window Resize**: `CSI 8 ; rows ; cols t`
//! - **Focus Gained**: `CSI I`
//! - **Focus Lost**: `CSI O`
//! - **Bracketed Paste Start**: `ESC [ 200 ~`
//! - **Bracketed Paste End**: `ESC [ 201 ~`

use super::types::{VT100FocusState, VT100InputEvent, VT100PasteMode};
use crate::core::ansi::constants::{ANSI_ESC, ANSI_CSI_BRACKET,
                                   FOCUS_GAINED_FINAL, FOCUS_LOST_FINAL};

/// Parse a terminal event sequence and return an `InputEvent` with bytes consumed if
/// recognized.
///
/// Returns `Some((event, bytes_consumed))` if a complete sequence is parsed,
/// or `None` if the sequence is incomplete or invalid.
///
/// Handles sequences like:
/// - `CSI 8;24;80t` → Window resize to 24 rows × 80 columns
/// - `CSI I` → Terminal gained focus
/// - `CSI O` → Terminal lost focus
/// - `ESC[200~` → Bracketed paste start
#[must_use]
pub fn parse_terminal_event(buffer: &[u8]) -> Option<(VT100InputEvent, usize)> {
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
            FOCUS_GAINED_FINAL => return Some((VT100InputEvent::Focus(VT100FocusState::Gained), 3)),
            FOCUS_LOST_FINAL => return Some((VT100InputEvent::Focus(VT100FocusState::Lost), 3)),
            _ => {}
        }
    }

    // Parse parameters and final byte for multi-character sequences
    parse_csi_terminal_parameters(buffer)
}

/// Parse CSI sequences with parameters for terminal events.
fn parse_csi_terminal_parameters(buffer: &[u8]) -> Option<(VT100InputEvent, usize)> {
    // Extract parameters and final byte
    // Format: ESC [ [param;param;...] final_byte
    let mut params = Vec::new();
    let mut current_num = String::new();
    let mut final_byte = 0u8;
    let mut bytes_scanned = 0;

    for (idx, &byte) in buffer[2..].iter().enumerate() {
        bytes_scanned = idx + 1; // Track position relative to buffer[2..]
        match byte {
            b'0'..=b'9' => {
                current_num.push(byte as char);
            }
            b';' => {
                if !current_num.is_empty() {
                    params.push(current_num.parse::<u16>().unwrap_or(0));
                    current_num.clear();
                }
            }
            b'~' | b't' => {
                if !current_num.is_empty() {
                    params.push(current_num.parse::<u16>().unwrap_or(0));
                }
                final_byte = byte;
                break;
            }
            _ => return None, // Invalid byte in sequence
        }
    }

    if final_byte == 0 {
        return None; // No final byte found
    }

    // Total bytes consumed: ESC [ (2 bytes) + scanned bytes (includes final)
    let total_consumed = 2 + bytes_scanned;

    // Parse based on parameters and final byte
    match (params.len(), final_byte) {
        // Window resize: CSI 8 ; rows ; cols t
        (3, b't') if params[0] == 8 => {
            let rows = params[1];
            let cols = params[2];
            Some((VT100InputEvent::Resize { rows, cols }, total_consumed))
        }
        // Bracketed paste: CSI 200 ~ or CSI 201 ~
        (1, b'~') => match params[0] {
            200 => Some((
                VT100InputEvent::Paste(VT100PasteMode::Start),
                total_consumed,
            )),
            201 => Some((VT100InputEvent::Paste(VT100PasteMode::End), total_consumed)),
            _ => None,
        },
        _ => None,
    }
}

/// Unit tests for terminal event parsing (focus, resize, bracketed paste).
///
/// These tests use generator functions instead of hardcoded magic strings to ensure
/// consistency between sequence generation and parsing. For testing strategy details,
/// see the [testing strategy] documentation.
///
/// [testing strategy]: mod@super#testing-strategy
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ansi::vt_100_terminal_input_parser::test_fixtures::generate_keyboard_sequence;

    #[test]
    fn test_resize_event() {
        // Round-trip test: Generate sequence from VT100InputEvent, then parse it back
        let original_event = VT100InputEvent::Resize { rows: 24, cols: 80 };
        let seq = generate_keyboard_sequence(&original_event)
            .expect("Failed to generate resize sequence");

        let (parsed_event, bytes_consumed) =
            parse_terminal_event(&seq).expect("Should parse resize");

        assert_eq!(bytes_consumed, seq.len());
        assert_eq!(parsed_event, original_event);
    }

    #[test]
    fn test_focus_events() {
        // Round-trip test: Focus gained
        let original_gained = VT100InputEvent::Focus(VT100FocusState::Gained);
        let seq_gained = generate_keyboard_sequence(&original_gained)
            .expect("Failed to generate focus gained sequence");

        let (parsed, bytes_consumed) =
            parse_terminal_event(&seq_gained).expect("Should parse focus gained");

        assert_eq!(bytes_consumed, seq_gained.len());
        assert_eq!(parsed, original_gained);

        // Round-trip test: Focus lost
        let original_lost = VT100InputEvent::Focus(VT100FocusState::Lost);
        let seq_lost = generate_keyboard_sequence(&original_lost)
            .expect("Failed to generate focus lost sequence");

        let (parsed, bytes_consumed) =
            parse_terminal_event(&seq_lost).expect("Should parse focus lost");

        assert_eq!(bytes_consumed, seq_lost.len());
        assert_eq!(parsed, original_lost);
    }

    #[test]
    fn test_bracketed_paste() {
        // Round-trip test: Paste start
        let original_start = VT100InputEvent::Paste(VT100PasteMode::Start);
        let seq_start = generate_keyboard_sequence(&original_start)
            .expect("Failed to generate paste start sequence");

        let (parsed, bytes_consumed) =
            parse_terminal_event(&seq_start).expect("Should parse paste start");

        assert_eq!(bytes_consumed, seq_start.len());
        assert_eq!(parsed, original_start);

        // Round-trip test: Paste end
        let original_end = VT100InputEvent::Paste(VT100PasteMode::End);
        let seq_end = generate_keyboard_sequence(&original_end)
            .expect("Failed to generate paste end sequence");

        let (parsed, bytes_consumed) =
            parse_terminal_event(&seq_end).expect("Should parse paste end");

        assert_eq!(bytes_consumed, seq_end.len());
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
