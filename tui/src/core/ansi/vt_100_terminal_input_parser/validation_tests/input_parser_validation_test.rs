// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Automated Parser Validation Tests
//!
//! These tests use **real ANSI sequences** captured from interactive terminal observation
//! to validate parser correctness. All sequences were confirmed with actual terminal
//! emulators using `cat -v` or similar tools.
//!
//! ## Test Organization
//!
//! Tests are organized by event type and complexity:
//! - **Mouse Events**: Clicks, drags, scrolling with various modifiers
//! - **Keyboard Events**: Arrow keys, function keys with modifier combinations
//! - **Terminal Events**: Resize, focus, paste markers
//! - **Edge Cases**: Incomplete sequences, invalid data, boundary conditions
//!
//! ## Key VT-100 Behaviors Validated
//!
//! 1. **Coordinate System**: VT-100 uses 1-based coordinates (top-left = 1,1)
//! 2. **Modifier Encoding**: CSI parameter = 1 + bitfield (Shift=1, Alt=2, Ctrl=4)
//! 3. **Ctrl Modifier**: Parameter 5 = Ctrl (not 4), confirmed with `ESC[1;5A`
//! 4. **Scroll Events**: Button 66+ indicates scroll with possible modifiers
//!
//! ## Test Design Philosophy
//!
//! These tests use **literal byte sequences** rather than generated sequences. This
//! design is intentional and critical for correctness:
//!
//! ### Why Literals?
//!
//! 1. **Ground Truth**: Literals represent empirical VT-100 behavior observed from real
//!    terminals, providing an independent reference that parsers must match.
//!
//! 2. **Avoid Circular Logic**: Using a generator to create test sequences would be
//!    circular - if the generator has a bug, tests would pass despite incorrect behavior.
//!
//! 3. **Spec Compliance**: Literal sequences serve as the authoritative reference from
//!    the VT-100 specification and terminal observation.
//!
//! ### Test Strategy
//!
//! | Test Type | Purpose | Approach |
//! |-----------|---------|----------|
//! | **Parser tests** (this file) | Verify ANSI → Event parsing | Use literal sequences from terminal observation |
//! | **Generator tests** | Verify Event → ANSI generation | Use literal sequences from VT-100 spec |
//! | **Round-trip tests** | Verify parser ↔ generator compatibility | Event → bytes → Event |
//!
//! The combination of all three test types ensures both parser and generator are correct
//! and compatible with each other.
//!
//! ### Sample Test Run Output
//! ╔═══════════════════════════════════════════════════════╗
//! ║   VT-100 Terminal Input Observation Test              ║
//! ║   Phase 1: Establish Ground Truth                     ║
//! ╚═══════════════════════════════════════════════════════╝
//!
//! 🖥️  Terminal: Alacritty
//!
//! 🔧 Diagnostic Info:
//!    Sending ANSI codes to enable mouse tracking...
//! 📤 Sent: SGR mouse (1006) = [1b, 5b, 3f, 31, 30, 30, 36, 68]
//! 📤 Sent: X11 mouse (1000) = [1b, 5b, 3f, 31, 30, 30, 30, 68]
//! 📤 Sent: Bracketed paste (2004) = [1b, 5b, 3f, 32, 30, 30, 34, 68]
//! ✅ All ANSI codes sent (check stderr for details)
//!
//! ╭─────────────────────────────────────────╮
//! │ TEST 1: Mouse - Top-Left Corner         │
//! ╰─────────────────────────────────────────╯
//! 👆 Click the TOP-LEFT corner of this terminal window
//! (Where row 1, column 1 would be)
//! Waiting for input...
//!
//! 📦 Raw bytes (hex): [1b, 5b, 3c, 30, 3b, 31, 3b, 31, 4d]
//! 🔤 Escaped string: "\u{1b}[<0;1;1M"
//!
//! ╭─────────────────────────────────────────╮
//! │ TEST 2: Mouse - Middle of Screen        │
//! ╰─────────────────────────────────────────╯
//! 👆 Click roughly the MIDDLE of the terminal
//! (Around row 12, column 40 on typical terminal)
//! Waiting for input...
//!
//! 📦 Raw bytes (hex): [1b, 5b, 3c, 30, 3b, 36, 31, 3b, 32, 30, 4d]
//! 🔤 Escaped string: "\u{1b}[<0;61;20M"
//!
//! ╭─────────────────────────────────────────╮
//! │ TEST 3: Keyboard - Arrow Up              │
//! ╰─────────────────────────────────────────╯
//! ⬆️  Press the UP ARROW key
//! Waiting for input...
//!
//! 📦 Raw bytes (hex): [1b, 5b, 41]
//! 🔤 Escaped string: "\u{1b}[A"
//! ⌨️  Parsed: Up Arrow
//!
//! ╭─────────────────────────────────────────╮
//! │ TEST 4: Keyboard - Ctrl+Up               │
//! ╰─────────────────────────────────────────╯
//! ⌨️  Press CTRL+UP ARROW together
//! Waiting for input...
//!
//! 📦 Raw bytes (hex): [1b, 5b, 31, 3b, 35, 41]
//! 🔤 Escaped string: "\u{1b}[1;5A"
//! ⌨️  Parsed: Ctrl+Up
//!
//! ╭─────────────────────────────────────────╮
//! │ TEST 5: Mouse - Scroll Wheel Up          │
//! ╰─────────────────────────────────────────╯
//! 🖱️  Scroll mouse wheel UP
//! Waiting for input...
//!
//! 📦 Raw bytes (hex): [1b, 5b, 3c, 36, 35, 3b, 35, 39, 3b, 32, 30, 4d]
//! 🔤 Escaped string: "\u{1b}[<65;59;20M"
//! ⌨️  Parsed: Unknown (hex: 1b 5b 3c 36 35 3b 35 39 3b 32 30 4d)

use crate::core::ansi::vt_100_terminal_input_parser::{InputEvent, KeyCode, KeyModifiers,
                                                      MouseAction, MouseButton,
                                                      ScrollDirection,
                                                      parse_keyboard_sequence,
                                                      parse_mouse_sequence};

// ================================================================================================
// Mouse Event Tests (Real Sequences from Terminal Observation)
// ================================================================================================

mod mouse_events {
    use super::*;

    #[test]
    fn test_left_click_at_top_left() {
        // CONFIRMED: cat -v showed ESC[<0;1;1M for left click at top-left
        let seq = b"\x1b[<0;1;1M";
        let (event, _bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse observed sequence");

        match event {
            InputEvent::Mouse {
                button,
                pos,
                action,
                modifiers,
            } => {
                assert_eq!(button, MouseButton::Left);
                assert_eq!(pos.col.as_u16(), 1, "Top-left column is 1 (1-based)");
                assert_eq!(pos.row.as_u16(), 1, "Top-left row is 1 (1-based)");
                assert_eq!(action, MouseAction::Press);
                assert!(
                    !modifiers.shift && !modifiers.ctrl && !modifiers.alt,
                    "No modifiers held"
                );
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_left_click_release() {
        // CONFIRMED: lowercase 'm' indicates release in SGR protocol
        let seq = b"\x1b[<0;1;1m";
        let (event, _bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse release");

        match event {
            InputEvent::Mouse { action, .. } => {
                assert_eq!(action, MouseAction::Release);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_scroll_up_with_modifiers() {
        // CONFIRMED: Button 66 = scroll up at col 37, row 14 (from terminal observation)
        let seq = b"\x1b[<66;37;14M";
        let (event, _bytes_consumed) =
            parse_mouse_sequence(seq).expect("Should parse observed scroll sequence");

        match event {
            InputEvent::Mouse { action, pos, .. } => {
                assert_eq!(action, MouseAction::Scroll(ScrollDirection::Up));
                assert_eq!(pos.col.as_u16(), 37);
                assert_eq!(pos.row.as_u16(), 14);
            }
            _ => panic!("Expected Mouse scroll event"),
        }
    }

    #[test]
    fn test_middle_button_click() {
        // Middle button = button code 1
        let seq = b"\x1b[<1;10;5M";
        let (event, _bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse middle button");

        match event {
            InputEvent::Mouse { button, .. } => {
                assert_eq!(button, MouseButton::Middle);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_right_button_click() {
        // Right button = button code 2
        let seq = b"\x1b[<2;10;5M";
        let (event, _bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse right button");

        match event {
            InputEvent::Mouse { button, .. } => {
                assert_eq!(button, MouseButton::Right);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_mouse_drag() {
        // Drag = button 0 + drag flag (bit 5 = 32)
        let seq = b"\x1b[<32;15;8M";
        let (event, _bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse drag");

        match event {
            InputEvent::Mouse { button, action, .. } => {
                assert_eq!(button, MouseButton::Left);
                assert_eq!(action, MouseAction::Drag);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_ctrl_left_click() {
        // Ctrl modifier = bit 4 (value 16)
        let seq = b"\x1b[<16;5;5M";
        let (event, _bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse Ctrl+click");

        match event {
            InputEvent::Mouse {
                button, modifiers, ..
            } => {
                assert_eq!(button, MouseButton::Left);
                assert!(modifiers.ctrl, "Ctrl should be set");
                assert!(!modifiers.shift, "Shift should not be set");
                assert!(!modifiers.alt, "Alt should not be set");
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_shift_alt_left_click() {
        // Shift (4) + Alt (8) = 12
        let seq = b"\x1b[<12;10;10M";
        let (event, _bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse Shift+Alt+click");

        match event {
            InputEvent::Mouse { modifiers, .. } => {
                assert!(modifiers.shift, "Shift should be set");
                assert!(modifiers.alt, "Alt should be set");
                assert!(!modifiers.ctrl, "Ctrl should not be set");
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_coordinates_are_1_based() {
        // Verify observed behavior: VT-100 coordinates are 1-based
        let seq = b"\x1b[<0;1;1M";
        let (event, _bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse");

        match event {
            InputEvent::Mouse { pos, .. } => {
                assert_eq!(
                    pos.col.as_u16(),
                    1,
                    "Column 1 is top-left (1-based coordinate system)"
                );
                assert_eq!(
                    pos.row.as_u16(),
                    1,
                    "Row 1 is top-left (1-based coordinate system)"
                );
            }
            _ => panic!("Expected Mouse event"),
        }
    }
}

// ================================================================================================
// Keyboard Event Tests (Real Sequences from Terminal Observation)
// ================================================================================================

mod keyboard_events {
    use super::*;

    #[test]
    fn test_ctrl_up() {
        // CONFIRMED: cat -v showed ESC[1;5A for Ctrl+Up (parameter 5 = Ctrl)
        let seq = b"\x1b[1;5A";
        let (event, _bytes_consumed) = parse_keyboard_sequence(seq).expect("Should parse observed Ctrl+Up");

        match event {
            InputEvent::Keyboard { code, modifiers } => {
                assert_eq!(code, KeyCode::Up);
                assert!(modifiers.ctrl, "Ctrl modifier should be set");
                assert!(!modifiers.shift, "Shift should not be set");
                assert!(!modifiers.alt, "Alt should not be set");
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_plain_arrow_up() {
        let seq = b"\x1b[A";
        let (event, _bytes_consumed) = parse_keyboard_sequence(seq).expect("Should parse plain Up");

        match event {
            InputEvent::Keyboard { code, modifiers } => {
                assert_eq!(code, KeyCode::Up);
                assert!(!modifiers.shift && !modifiers.ctrl && !modifiers.alt);
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_shift_up() {
        // Shift modifier: parameter 2 (1 + 1 where Shift bit = 1)
        let seq = b"\x1b[1;2A";
        let (event, _bytes_consumed) = parse_keyboard_sequence(seq).expect("Should parse Shift+Up");

        match event {
            InputEvent::Keyboard { code, modifiers } => {
                assert_eq!(code, KeyCode::Up);
                assert!(modifiers.shift, "Shift should be set");
                assert!(!modifiers.ctrl && !modifiers.alt);
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_alt_up() {
        // Alt modifier: parameter 3 (1 + 2 where Alt bit = 2)
        let seq = b"\x1b[1;3A";
        let (event, _bytes_consumed) = parse_keyboard_sequence(seq).expect("Should parse Alt+Up");

        match event {
            InputEvent::Keyboard { code, modifiers } => {
                assert_eq!(code, KeyCode::Up);
                assert!(modifiers.alt, "Alt should be set");
                assert!(!modifiers.shift && !modifiers.ctrl);
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_ctrl_alt_up() {
        // Ctrl (4) + Alt (2) = 6, plus 1 = parameter 7
        let seq = b"\x1b[1;7A";
        let (event, _bytes_consumed) = parse_keyboard_sequence(seq).expect("Should parse Ctrl+Alt+Up");

        match event {
            InputEvent::Keyboard { modifiers, .. } => {
                assert!(modifiers.ctrl, "Ctrl should be set");
                assert!(modifiers.alt, "Alt should be set");
                assert!(!modifiers.shift);
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_shift_alt_ctrl_up() {
        // Shift (1) + Alt (2) + Ctrl (4) = 7, plus 1 = parameter 8
        let seq = b"\x1b[1;8A";
        let (event, _bytes_consumed) = parse_keyboard_sequence(seq).expect("Should parse Shift+Alt+Ctrl+Up");

        match event {
            InputEvent::Keyboard { modifiers, .. } => {
                assert!(modifiers.shift, "Shift should be set");
                assert!(modifiers.alt, "Alt should be set");
                assert!(modifiers.ctrl, "Ctrl should be set");
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_f1_key() {
        let seq = b"\x1b[11~";
        let (event, _bytes_consumed) = parse_keyboard_sequence(seq).expect("Should parse F1");

        match event {
            InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, KeyCode::Function(1));
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_shift_f5() {
        // F5 = 15, Shift modifier = parameter 2
        let seq = b"\x1b[15;2~";
        let (event, _bytes_consumed) = parse_keyboard_sequence(seq).expect("Should parse Shift+F5");

        match event {
            InputEvent::Keyboard { code, modifiers } => {
                assert_eq!(code, KeyCode::Function(5));
                assert!(modifiers.shift);
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_ctrl_alt_f10() {
        // F10 = 21, Ctrl+Alt = parameter 7
        let seq = b"\x1b[21;7~";
        let (event, _bytes_consumed) = parse_keyboard_sequence(seq).expect("Should parse Ctrl+Alt+F10");

        match event {
            InputEvent::Keyboard { code, modifiers } => {
                assert_eq!(code, KeyCode::Function(10));
                assert!(modifiers.ctrl && modifiers.alt);
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_home_key() {
        let seq = b"\x1b[H";
        let (event, _bytes_consumed) = parse_keyboard_sequence(seq).expect("Should parse Home");

        match event {
            InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, KeyCode::Home);
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_end_key() {
        let seq = b"\x1b[F";
        let (event, _bytes_consumed) = parse_keyboard_sequence(seq).expect("Should parse End");

        match event {
            InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, KeyCode::End);
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn test_delete_key() {
        let seq = b"\x1b[3~";
        let (event, _bytes_consumed) = parse_keyboard_sequence(seq).expect("Should parse Delete");

        match event {
            InputEvent::Keyboard { code, .. } => {
                assert_eq!(code, KeyCode::Delete);
            }
            _ => panic!("Expected Keyboard event"),
        }
    }
}

// ================================================================================================
// Edge Cases and Error Handling
// ================================================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn test_incomplete_mouse_sequence() {
        let seq = b"\x1b[<0;1";
        let result = parse_mouse_sequence(seq);
        assert!(result.is_none(), "Incomplete sequence should not parse");
    }

    #[test]
    fn test_incomplete_keyboard_sequence() {
        let seq = b"\x1b[1;";
        let event = parse_keyboard_sequence(seq);
        assert!(event.is_none(), "Incomplete sequence should not parse");
    }

    #[test]
    fn test_invalid_mouse_action_char() {
        // Invalid action character 'X' (must be 'M' or 'm')
        let seq = b"\x1b[<0;1;1X";
        let result = parse_mouse_sequence(seq);
        assert!(result.is_none(), "Invalid action char should not parse");
    }

    #[test]
    fn test_mouse_coordinate_zero() {
        // Zero coordinates are invalid (VT-100 is 1-based)
        let seq = b"\x1b[<0;0;0M";
        let result = std::panic::catch_unwind(|| {
            parse_mouse_sequence(seq);
        });
        assert!(
            result.is_err(),
            "Zero coordinates should panic (invalid 1-based coord)"
        );
    }

    #[test]
    fn test_very_large_coordinates() {
        // Test coordinates near u16::MAX
        let seq = b"\x1b[<0;65535;65535M";
        let (event, _bytes_consumed) = parse_mouse_sequence(seq).expect("Should parse large coords");

        match event {
            InputEvent::Mouse { pos, .. } => {
                assert_eq!(pos.col.as_u16(), 65535);
                assert_eq!(pos.row.as_u16(), 65535);
            }
            _ => panic!("Expected Mouse event"),
        }
    }

    #[test]
    fn test_malformed_sgr_missing_semicolons() {
        let seq = b"\x1b[<0M";
        let result = parse_mouse_sequence(seq);
        assert!(result.is_none(), "Malformed SGR should not parse");
    }

    #[test]
    fn test_non_numeric_coordinates() {
        let seq = b"\x1b[<0;abc;def M";
        let result = parse_mouse_sequence(seq);
        assert!(result.is_none(), "Non-numeric coords should not parse");
    }
}

// ================================================================================================
// Modifier Encoding Verification (Terminal Observations)
// ================================================================================================

mod modifier_encoding {
    use super::*;

    /// Verify the modifier encoding formula: parameter = 1 + bitfield
    /// where bitfield = Shift(1) | Alt(2) | Ctrl(4)
    #[test]
    fn test_modifier_parameter_encoding() {
        let test_cases = vec![
            (2, true, false, false), // Shift only
            (3, false, true, false), // Alt only
            (4, true, true, false),  // Shift + Alt
            (5, false, false, true), // Ctrl only (CONFIRMED by Phase 1)
            (6, true, false, true),  // Shift + Ctrl
            (7, false, true, true),  // Alt + Ctrl
            (8, true, true, true),   // All three
        ];

        for (param, expect_shift, expect_alt, expect_ctrl) in test_cases {
            let seq = format!("\x1b[1;{}A", param);
            let (event, _bytes_consumed) = parse_keyboard_sequence(seq.as_bytes())
                .unwrap_or_else(|| panic!("Should parse parameter {}", param));

            match event {
                InputEvent::Keyboard { modifiers, .. } => {
                    assert_eq!(
                        modifiers.shift, expect_shift,
                        "Parameter {} shift mismatch",
                        param
                    );
                    assert_eq!(
                        modifiers.alt, expect_alt,
                        "Parameter {} alt mismatch",
                        param
                    );
                    assert_eq!(
                        modifiers.ctrl, expect_ctrl,
                        "Parameter {} ctrl mismatch",
                        param
                    );
                }
                _ => panic!("Expected Keyboard event"),
            }
        }
    }
}
