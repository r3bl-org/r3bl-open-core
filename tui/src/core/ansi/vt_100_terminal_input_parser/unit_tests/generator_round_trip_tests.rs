// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Round-trip validation tests for input event generator and parser.
//!
//! These tests ensure that every input event can be serialized to an [`ANSI`] sequence
//! via [`generate_keyboard_sequence`] and then parsed back to the exact same event via
//! [`try_parse_input_event`].
//!
//! # Round-Trip Invariant
//!
//! ```text
//! VT100InputEventIR ──► generate_keyboard_sequence() ──► ANSI Bytes ──► try_parse_input_event() ──► VT100InputEventIR
//! ```
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`generate_keyboard_sequence`]: crate::core::ansi::generator::generate_keyboard_sequence
//! [`try_parse_input_event`]: crate::core::ansi::vt_100_terminal_input_parser::try_parse_input_event

use crate::{RgbValue, TermPos, VPHeight, VPWidth,
            core::ansi::{generator::generate_keyboard_sequence,
                         vt_100_terminal_input_parser::{MaybeMore, ParsedInputEventIR,
                                                        TerminalColorReport,
                                                        TerminalColorRole,
                                                        VT100FocusStateIR,
                                                        VT100InputEventIR,
                                                        VT100KeyCodeIR,
                                                        VT100KeyModifiersIR,
                                                        VT100MouseActionIR,
                                                        VT100MouseButtonIR,
                                                        VT100PasteModeIR,
                                                        VT100ScrollDirectionIR,
                                                        try_parse_input_event}}};

/// Helper function to assert round-trip symmetry: $\text{Event} \to \text{Bytes} \to
/// \text{Event}$.
fn assert_round_trip(event: &VT100InputEventIR) {
    let bytes = generate_keyboard_sequence(event)
        .expect("Failed to generate ANSI bytes for event");

    let ParsedInputEventIR {
        event: parsed_event,
        bytes_consumed,
    } = try_parse_input_event(&bytes, MaybeMore::KernelDrained)
        .expect("Failed to parse generated ANSI bytes back to event");

    assert_eq!(&parsed_event, event);
    assert_eq!(bytes_consumed.as_usize(), bytes.len());
}

// ==================== Terminal Events Round-Trip ====================

#[test]
fn test_roundtrip_resize_events() {
    assert_round_trip(&VT100InputEventIR::Resize {
        row_height: VPHeight::from(24),
        col_width: VPWidth::from(80),
    });
    assert_round_trip(&VT100InputEventIR::Resize {
        row_height: VPHeight::from(30),
        col_width: VPWidth::from(120),
    });
    assert_round_trip(&VT100InputEventIR::Resize {
        row_height: VPHeight::from(60),
        col_width: VPWidth::from(200),
    });
}

#[test]
fn test_roundtrip_focus_events() {
    assert_round_trip(&VT100InputEventIR::Focus(VT100FocusStateIR::Gained));
    assert_round_trip(&VT100InputEventIR::Focus(VT100FocusStateIR::Lost));
}

#[test]
fn test_roundtrip_paste_events() {
    assert_round_trip(&VT100InputEventIR::Paste(VT100PasteModeIR::Start));
    assert_round_trip(&VT100InputEventIR::Paste(VT100PasteModeIR::End));
}

#[test]
fn test_roundtrip_color_report_events() {
    assert_round_trip(&VT100InputEventIR::ColorReport(TerminalColorReport {
        role: TerminalColorRole::Foreground,
        color: RgbValue::from_u8(0x1e, 0x2a, 0x3b),
    }));
    assert_round_trip(&VT100InputEventIR::ColorReport(TerminalColorReport {
        role: TerminalColorRole::Background,
        color: RgbValue::from_u8(0x00, 0xff, 0x7f),
    }));
    assert_round_trip(&VT100InputEventIR::ColorReport(TerminalColorReport {
        role: TerminalColorRole::Cursor,
        color: RgbValue::from_u8(0xff, 0xff, 0xff),
    }));
    assert_round_trip(&VT100InputEventIR::ColorReport(TerminalColorReport {
        role: TerminalColorRole::MouseForeground,
        color: RgbValue::from_u8(0x12, 0x34, 0x56),
    }));
    assert_round_trip(&VT100InputEventIR::ColorReport(TerminalColorReport {
        role: TerminalColorRole::MouseBackground,
        color: RgbValue::from_u8(0x78, 0x9a, 0xbc),
    }));
    assert_round_trip(&VT100InputEventIR::ColorReport(TerminalColorReport {
        role: TerminalColorRole::Highlight,
        color: RgbValue::from_u8(0x23, 0x45, 0x67),
    }));
    assert_round_trip(&VT100InputEventIR::ColorReport(TerminalColorReport {
        role: TerminalColorRole::HighlightForeground,
        color: RgbValue::from_u8(0x89, 0xab, 0xcd),
    }));
}

// ==================== Mouse Events Round-Trip (SGR Protocol) ====================

#[test]
fn test_roundtrip_mouse_clicks_and_releases() {
    let buttons = [
        VT100MouseButtonIR::Left,
        VT100MouseButtonIR::Middle,
        VT100MouseButtonIR::Right,
    ];
    let actions = [VT100MouseActionIR::Press, VT100MouseActionIR::Release];

    for button in buttons {
        for action in actions {
            assert_round_trip(&VT100InputEventIR::Mouse {
                button,
                pos: TermPos::from_one_based(10, 20),
                action,
                modifiers: VT100KeyModifiersIR::default(),
            });
        }
    }
}

#[test]
fn test_roundtrip_mouse_drag_and_motion() {
    // Moving with a button held is a Drag event.
    assert_round_trip(&VT100InputEventIR::Mouse {
        button: VT100MouseButtonIR::Left,
        pos: TermPos::from_one_based(15, 30),
        action: VT100MouseActionIR::Drag,
        modifiers: VT100KeyModifiersIR::default(),
    });

    // Moving without a button held (Unknown button) is a Motion (hover) event.
    assert_round_trip(&VT100InputEventIR::Mouse {
        button: VT100MouseButtonIR::Unknown,
        pos: TermPos::from_one_based(45, 90),
        action: VT100MouseActionIR::Motion,
        modifiers: VT100KeyModifiersIR::default(),
    });
}

#[test]
fn test_roundtrip_mouse_scroll() {
    let directions = [
        VT100ScrollDirectionIR::Up,
        VT100ScrollDirectionIR::Down,
        VT100ScrollDirectionIR::Left,
        VT100ScrollDirectionIR::Right,
    ];

    for dir in directions {
        assert_round_trip(&VT100InputEventIR::Mouse {
            button: VT100MouseButtonIR::Unknown,
            pos: TermPos::from_one_based(5, 8),
            action: VT100MouseActionIR::Scroll(dir),
            modifiers: VT100KeyModifiersIR::default(),
        });
    }
}

#[test]
fn test_roundtrip_mouse_with_modifiers() {
    let modifiers_list = [
        VT100KeyModifiersIR::SHIFT,
        VT100KeyModifiersIR::ALT,
        VT100KeyModifiersIR::CTRL,
        VT100KeyModifiersIR::CTRL.with_shift(),
        VT100KeyModifiersIR::CTRL.with_alt(),
        VT100KeyModifiersIR::CTRL.with_alt().with_shift(),
    ];

    for modifiers in modifiers_list {
        assert_round_trip(&VT100InputEventIR::Mouse {
            button: VT100MouseButtonIR::Left,
            pos: TermPos::from_one_based(25, 40),
            action: VT100MouseActionIR::Press,
            modifiers,
        });
    }
}

// ==================== Arrow Keys Round-Trip ====================

#[test]
fn test_roundtrip_arrow_keys_plain() {
    let keys = [
        VT100KeyCodeIR::Up,
        VT100KeyCodeIR::Down,
        VT100KeyCodeIR::Left,
        VT100KeyCodeIR::Right,
    ];

    for code in keys {
        assert_round_trip(&VT100InputEventIR::Keyboard {
            code,
            modifiers: VT100KeyModifiersIR::default(),
        });
    }
}

#[test]
fn test_roundtrip_arrow_keys_with_modifiers() {
    let keys = [
        VT100KeyCodeIR::Up,
        VT100KeyCodeIR::Down,
        VT100KeyCodeIR::Left,
        VT100KeyCodeIR::Right,
    ];
    let modifiers_list = [
        VT100KeyModifiersIR::SHIFT,
        VT100KeyModifiersIR::ALT,
        VT100KeyModifiersIR::CTRL,
        VT100KeyModifiersIR::CTRL.with_shift(),
        VT100KeyModifiersIR::CTRL.with_alt(),
        VT100KeyModifiersIR::CTRL.with_alt().with_shift(),
    ];

    for code in keys {
        for modifiers in modifiers_list {
            assert_round_trip(&VT100InputEventIR::Keyboard { code, modifiers });
        }
    }
}

// ==================== Navigation & Editing Keys Round-Trip ====================

#[test]
fn test_roundtrip_navigation_keys() {
    let keys = [
        VT100KeyCodeIR::Home,
        VT100KeyCodeIR::End,
        VT100KeyCodeIR::Insert,
        VT100KeyCodeIR::Delete,
        VT100KeyCodeIR::PageUp,
        VT100KeyCodeIR::PageDown,
    ];
    let modifiers_list = [
        VT100KeyModifiersIR::default(),
        VT100KeyModifiersIR::SHIFT,
        VT100KeyModifiersIR::ALT,
        VT100KeyModifiersIR::CTRL,
        VT100KeyModifiersIR::CTRL.with_shift(),
        VT100KeyModifiersIR::CTRL.with_alt(),
        VT100KeyModifiersIR::CTRL.with_alt().with_shift(),
    ];

    for code in keys {
        for modifiers in modifiers_list {
            assert_round_trip(&VT100InputEventIR::Keyboard { code, modifiers });
        }
    }
}

// ==================== Function Keys Round-Trip ====================

#[test]
fn test_roundtrip_function_keys() {
    let modifiers_list = [
        VT100KeyModifiersIR::default(),
        VT100KeyModifiersIR::SHIFT,
        VT100KeyModifiersIR::ALT,
        VT100KeyModifiersIR::CTRL,
        VT100KeyModifiersIR::CTRL.with_shift(),
        VT100KeyModifiersIR::CTRL.with_alt(),
        VT100KeyModifiersIR::CTRL.with_alt().with_shift(),
    ];

    for f_num in 1..=12 {
        for modifiers in modifiers_list {
            assert_round_trip(&VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Function(f_num),
                modifiers,
            });
        }
    }
}

// ==================== Dedicated & Raw Byte Keys Round-Trip ====================

#[test]
fn test_roundtrip_dedicated_keys() {
    let dedicated_keys = [
        VT100KeyCodeIR::Tab,
        VT100KeyCodeIR::BackTab,
        VT100KeyCodeIR::Enter,
        VT100KeyCodeIR::Escape,
        VT100KeyCodeIR::Backspace,
    ];

    for code in dedicated_keys {
        assert_round_trip(&VT100InputEventIR::Keyboard {
            code,
            modifiers: VT100KeyModifiersIR::default(),
        });
    }
}

#[test]
fn test_roundtrip_ctrl_letters() {
    for c in 'a'..='z' {
        let event = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Char(c),
            modifiers: VT100KeyModifiersIR::CTRL,
        };

        // In ASCII/VT-100, specific control characters are canonical aliases for
        // dedicated keys:
        // - Ctrl+H (0x08) parses as Backspace
        // - Ctrl+I (0x09) parses as Tab
        // - Ctrl+J (0x0A) / Ctrl+M (0x0D) parses as Enter
        let bytes = generate_keyboard_sequence(&event)
            .expect("Failed to generate ANSI bytes for ctrl letter");
        let ParsedInputEventIR {
            event: parsed_event,
            bytes_consumed,
        } = try_parse_input_event(&bytes, MaybeMore::KernelDrained)
            .expect("Failed to parse ctrl letter bytes");

        match c {
            'h' => assert_eq!(
                parsed_event,
                VT100InputEventIR::Keyboard {
                    code: VT100KeyCodeIR::Backspace,
                    modifiers: VT100KeyModifiersIR::default(),
                }
            ),
            'i' => assert_eq!(
                parsed_event,
                VT100InputEventIR::Keyboard {
                    code: VT100KeyCodeIR::Tab,
                    modifiers: VT100KeyModifiersIR::default(),
                }
            ),
            'j' | 'm' => assert_eq!(
                parsed_event,
                VT100InputEventIR::Keyboard {
                    code: VT100KeyCodeIR::Enter,
                    modifiers: VT100KeyModifiersIR::default(),
                }
            ),
            _ => assert_eq!(parsed_event, event),
        }
        assert_eq!(bytes_consumed.as_usize(), bytes.len());
    }
}

// ==================== Characters & Alt Keys Round-Trip ====================

#[test]
fn test_roundtrip_characters() {
    let test_chars = ['a', 'Z', '1', '!', ' ', '🦀', '日', 'é'];

    for c in test_chars {
        assert_round_trip(&VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Char(c),
            modifiers: VT100KeyModifiersIR::default(),
        });
    }
}

#[test]
fn test_roundtrip_alt_letter_combinations() {
    for c in 'a'..='z' {
        assert_round_trip(&VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Char(c),
            modifiers: VT100KeyModifiersIR::ALT,
        });
    }
}

#[test]
fn test_roundtrip_kitty_protocol_alt_bracket() {
    assert_round_trip(&VT100InputEventIR::Keyboard {
        code: VT100KeyCodeIR::Char('['),
        modifiers: VT100KeyModifiersIR::ALT,
    });
}

// ==================== Unsupported Generator Events ====================

#[test]
fn test_unsupported_generator_events_return_none() {
    // Ignored events cannot be serialized to input sequences.
    assert!(generate_keyboard_sequence(&VT100InputEventIR::Ignored).is_none());

    // Out-of-bounds function keys (F0, F13+).
    assert!(
        generate_keyboard_sequence(&VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Function(0),
            modifiers: VT100KeyModifiersIR::default(),
        })
        .is_none()
    );
    assert!(
        generate_keyboard_sequence(&VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Function(13),
            modifiers: VT100KeyModifiersIR::default(),
        })
        .is_none()
    );

    // Ctrl + non-alphabetic character is not a valid terminal control byte.
    assert!(
        generate_keyboard_sequence(&VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Char('1'),
            modifiers: VT100KeyModifiersIR::CTRL,
        })
        .is_none()
    );
}
