// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Round-trip validation tests for input event generator.
//!
//! These tests validate that the generator produces valid sequences that the parser
//! can correctly parse back to the original event:
//!
//! `InputEvent → generate() → bytes → parse() → InputEvent`
//!
//! This ensures generator and parser are compatible and speak the same protocol.

use crate::core::ansi::vt_100_terminal_input_parser::{
    InputEvent, KeyCode, KeyModifiers, FocusState, PasteMode,
    parse_keyboard_sequence, parse_terminal_event,
};
use crate::core::ansi::vt_100_terminal_input_parser::test_fixtures::*;

// ==================== Terminal Events ====================

#[test]
fn test_generate_resize_event() {
    let event = InputEvent::Resize { rows: 24, cols: 80 };
    let bytes = generate_keyboard_sequence(&event).unwrap();
    assert_eq!(bytes, b"\x1b[8;24;80t");
}

#[test]
fn test_generate_focus_gained() {
    let event = InputEvent::Focus(FocusState::Gained);
    let bytes = generate_keyboard_sequence(&event).unwrap();
    assert_eq!(bytes, b"\x1b[I");
}

#[test]
fn test_generate_focus_lost() {
    let event = InputEvent::Focus(FocusState::Lost);
    let bytes = generate_keyboard_sequence(&event).unwrap();
    assert_eq!(bytes, b"\x1b[O");
}

#[test]
fn test_generate_paste_start() {
    let event = InputEvent::Paste(PasteMode::Start);
    let bytes = generate_keyboard_sequence(&event).unwrap();
    assert_eq!(bytes, b"\x1b[200~");
}

#[test]
fn test_generate_paste_end() {
    let event = InputEvent::Paste(PasteMode::End);
    let bytes = generate_keyboard_sequence(&event).unwrap();
    assert_eq!(bytes, b"\x1b[201~");
}

#[test]
fn test_roundtrip_resize_event() {
    let original_event = InputEvent::Resize { rows: 30, cols: 120 };
    let bytes = generate_keyboard_sequence(&original_event).unwrap();
    let (parsed_event, bytes_consumed) = parse_terminal_event(&bytes).expect("Should parse");

    assert_eq!(parsed_event, original_event);
    assert_eq!(bytes_consumed, bytes.len());
}

#[test]
fn test_roundtrip_focus_events() {
    let original_gained = InputEvent::Focus(FocusState::Gained);
    let bytes_gained = generate_keyboard_sequence(&original_gained).unwrap();
    let (parsed_gained, bytes_consumed) = parse_terminal_event(&bytes_gained).expect("Should parse");

    assert_eq!(parsed_gained, original_gained);
    assert_eq!(bytes_consumed, bytes_gained.len());

    let original_lost = InputEvent::Focus(FocusState::Lost);
    let bytes_lost = generate_keyboard_sequence(&original_lost).unwrap();
    let (parsed_lost, bytes_consumed) = parse_terminal_event(&bytes_lost).expect("Should parse");

    assert_eq!(parsed_lost, original_lost);
    assert_eq!(bytes_consumed, bytes_lost.len());
}

#[test]
fn test_roundtrip_paste_events() {
    let original_start = InputEvent::Paste(PasteMode::Start);
    let bytes_start = generate_keyboard_sequence(&original_start).unwrap();
    let (parsed_start, bytes_consumed) = parse_terminal_event(&bytes_start).expect("Should parse");

    assert_eq!(parsed_start, original_start);
    assert_eq!(bytes_consumed, bytes_start.len());

    let original_end = InputEvent::Paste(PasteMode::End);
    let bytes_end = generate_keyboard_sequence(&original_end).unwrap();
    let (parsed_end, bytes_consumed) = parse_terminal_event(&bytes_end).expect("Should parse");

    assert_eq!(parsed_end, original_end);
    assert_eq!(bytes_consumed, bytes_end.len());
}

// ==================== Arrow Keys ====================

#[test]
fn test_generate_arrow_up() {
    let event = InputEvent::Keyboard {
        code: KeyCode::Up,
        modifiers: KeyModifiers::default(),
    };
    let bytes = generate_keyboard_sequence(&event).unwrap();
    assert_eq!(bytes, b"\x1b[A");
}

#[test]
fn test_generate_arrow_down() {
    let event = InputEvent::Keyboard {
        code: KeyCode::Down,
        modifiers: KeyModifiers::default(),
    };
    let bytes = generate_keyboard_sequence(&event).unwrap();
    assert_eq!(bytes, b"\x1b[B");
}

#[test]
fn test_generate_arrow_right() {
    let event = InputEvent::Keyboard {
        code: KeyCode::Right,
        modifiers: KeyModifiers::default(),
    };
    let bytes = generate_keyboard_sequence(&event).unwrap();
    assert_eq!(bytes, b"\x1b[C");
}

#[test]
fn test_generate_arrow_left() {
    let event = InputEvent::Keyboard {
        code: KeyCode::Left,
        modifiers: KeyModifiers::default(),
    };
    let bytes = generate_keyboard_sequence(&event).unwrap();
    assert_eq!(bytes, b"\x1b[D");
}

// ==================== Arrow Keys with Modifiers ====================

#[test]
fn test_generate_shift_up() {
    let event = InputEvent::Keyboard {
        code: KeyCode::Up,
        modifiers: KeyModifiers {
            shift: true,
            alt: false,
            ctrl: false,
        },
    };
    let bytes = generate_keyboard_sequence(&event).unwrap();
    // Shift modifier: parameter = 1 + 1 = 2
    assert_eq!(bytes, b"\x1b[1;2A");
}

#[test]
fn test_generate_alt_right() {
    let event = InputEvent::Keyboard {
        code: KeyCode::Right,
        modifiers: KeyModifiers {
            shift: false,
            alt: true,
            ctrl: false,
        },
    };
    let bytes = generate_keyboard_sequence(&event).unwrap();
    // Alt modifier: parameter = 1 + 2 = 3
    assert_eq!(bytes, b"\x1b[1;3C");
}

#[test]
fn test_generate_ctrl_down() {
    let event = InputEvent::Keyboard {
        code: KeyCode::Down,
        modifiers: KeyModifiers {
            shift: false,
            alt: false,
            ctrl: true,
        },
    };
    let bytes = generate_keyboard_sequence(&event).unwrap();
    // Ctrl modifier: parameter = 1 + 4 = 5
    assert_eq!(bytes, b"\x1b[1;5B");
}

#[test]
fn test_generate_ctrl_alt_shift_left() {
    let event = InputEvent::Keyboard {
        code: KeyCode::Left,
        modifiers: KeyModifiers {
            shift: true,
            alt: true,
            ctrl: true,
        },
    };
    let bytes = generate_keyboard_sequence(&event).unwrap();
    // Shift+Alt+Ctrl modifiers: parameter = 1 + 7 = 8
    assert_eq!(bytes, b"\x1b[1;8D");
}

// ==================== Special Keys ====================

#[test]
fn test_generate_home_key() {
    let event = InputEvent::Keyboard {
        code: KeyCode::Home,
        modifiers: KeyModifiers::default(),
    };
    let bytes = generate_keyboard_sequence(&event).unwrap();
    assert_eq!(bytes, b"\x1b[H");
}

#[test]
fn test_generate_end_key() {
    let event = InputEvent::Keyboard {
        code: KeyCode::End,
        modifiers: KeyModifiers::default(),
    };
    let bytes = generate_keyboard_sequence(&event).unwrap();
    assert_eq!(bytes, b"\x1b[F");
}

#[test]
fn test_generate_insert_key() {
    let event = InputEvent::Keyboard {
        code: KeyCode::Insert,
        modifiers: KeyModifiers::default(),
    };
    let bytes = generate_keyboard_sequence(&event).unwrap();
    assert_eq!(bytes, b"\x1b[2~");
}

#[test]
fn test_generate_delete_key() {
    let event = InputEvent::Keyboard {
        code: KeyCode::Delete,
        modifiers: KeyModifiers::default(),
    };
    let bytes = generate_keyboard_sequence(&event).unwrap();
    assert_eq!(bytes, b"\x1b[3~");
}

#[test]
fn test_generate_page_up() {
    let event = InputEvent::Keyboard {
        code: KeyCode::PageUp,
        modifiers: KeyModifiers::default(),
    };
    let bytes = generate_keyboard_sequence(&event).unwrap();
    assert_eq!(bytes, b"\x1b[5~");
}

#[test]
fn test_generate_page_down() {
    let event = InputEvent::Keyboard {
        code: KeyCode::PageDown,
        modifiers: KeyModifiers::default(),
    };
    let bytes = generate_keyboard_sequence(&event).unwrap();
    assert_eq!(bytes, b"\x1b[6~");
}

// ==================== Function Keys ====================

#[test]
fn test_generate_f1_key() {
    let event = InputEvent::Keyboard {
        code: KeyCode::Function(1),
        modifiers: KeyModifiers::default(),
    };
    let bytes = generate_keyboard_sequence(&event).unwrap();
    assert_eq!(bytes, b"\x1b[11~");
}

#[test]
fn test_generate_f6_key() {
    let event = InputEvent::Keyboard {
        code: KeyCode::Function(6),
        modifiers: KeyModifiers::default(),
    };
    let bytes = generate_keyboard_sequence(&event).unwrap();
    assert_eq!(bytes, b"\x1b[17~");
}

#[test]
fn test_generate_f12_key() {
    let event = InputEvent::Keyboard {
        code: KeyCode::Function(12),
        modifiers: KeyModifiers::default(),
    };
    let bytes = generate_keyboard_sequence(&event).unwrap();
    assert_eq!(bytes, b"\x1b[24~");
}

// ==================== Function Keys with Modifiers ====================

#[test]
fn test_generate_shift_f5() {
    let event = InputEvent::Keyboard {
        code: KeyCode::Function(5),
        modifiers: KeyModifiers {
            shift: true,
            alt: false,
            ctrl: false,
        },
    };
    let bytes = generate_keyboard_sequence(&event).unwrap();
    // Shift modifier: parameter = 1 + 1 = 2
    assert_eq!(bytes, b"\x1b[15;2~");
}

#[test]
fn test_generate_ctrl_alt_f10() {
    let event = InputEvent::Keyboard {
        code: KeyCode::Function(10),
        modifiers: KeyModifiers {
            shift: false,
            alt: true,
            ctrl: true,
        },
    };
    let bytes = generate_keyboard_sequence(&event).unwrap();
    // Ctrl+Alt modifiers: parameter = 1 + 6 = 7
    assert_eq!(bytes, b"\x1b[21;7~");
}

// ==================== Unsupported Keys ====================

#[test]
fn test_generate_unsupported_keys() {
    // Tab, Enter, Escape, Backspace are not generated as CSI sequences
    let tab_event = InputEvent::Keyboard {
        code: KeyCode::Tab,
        modifiers: KeyModifiers::default(),
    };
    assert_eq!(generate_keyboard_sequence(&tab_event), None);

    let enter_event = InputEvent::Keyboard {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::default(),
    };
    assert_eq!(generate_keyboard_sequence(&enter_event), None);

    let escape_event = InputEvent::Keyboard {
        code: KeyCode::Escape,
        modifiers: KeyModifiers::default(),
    };
    assert_eq!(generate_keyboard_sequence(&escape_event), None);

    let backspace_event = InputEvent::Keyboard {
        code: KeyCode::Backspace,
        modifiers: KeyModifiers::default(),
    };
    assert_eq!(generate_keyboard_sequence(&backspace_event), None);
}

// ==================== Round-Trip Tests ====================

#[test]
fn test_roundtrip_arrow_up() {
    let original_event = InputEvent::Keyboard {
        code: KeyCode::Up,
        modifiers: KeyModifiers::default(),
    };

    let bytes = generate_keyboard_sequence(&original_event).unwrap();
    let (parsed_event, bytes_consumed) = parse_keyboard_sequence(&bytes).expect("Should parse");

    assert_eq!(parsed_event, original_event);
    assert_eq!(bytes_consumed, bytes.len());
}

#[test]
fn test_roundtrip_ctrl_alt_f10() {
    let original_event = InputEvent::Keyboard {
        code: KeyCode::Function(10),
        modifiers: KeyModifiers {
            shift: false,
            alt: true,
            ctrl: true,
        },
    };

    let bytes = generate_keyboard_sequence(&original_event).unwrap();
    let (parsed_event, bytes_consumed) = parse_keyboard_sequence(&bytes).expect("Should parse");

    assert_eq!(parsed_event, original_event);
    assert_eq!(bytes_consumed, bytes.len());
}

#[test]
fn test_roundtrip_insert_key_with_shift() {
    let original_event = InputEvent::Keyboard {
        code: KeyCode::Insert,
        modifiers: KeyModifiers {
            shift: true,
            alt: false,
            ctrl: false,
        },
    };

    let bytes = generate_keyboard_sequence(&original_event).unwrap();
    let (parsed_event, bytes_consumed) = parse_keyboard_sequence(&bytes).expect("Should parse");

    assert_eq!(parsed_event, original_event);
    assert_eq!(bytes_consumed, bytes.len());
}
