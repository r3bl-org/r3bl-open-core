// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Main VT-100 Terminal Input Parser Entry Point
//!
//! This module provides the primary parsing interface for converting raw bytes
//! into terminal input events. It acts as the main routing layer that dispatches
//! to specialized parsers based on buffer content analysis.
//!
//! ## Where You Are in the Pipeline
//!
//! ```text
//! Raw Terminal Input (stdin)
//!    ↓
//! DirectToAnsiInputDevice (async I/O layer)
//!    ↓
//! ┌──▼───────────────────────────────────────┐
//! │  parser.rs - Main Entry Point            │  ← **YOU ARE HERE**
//! │  • try_parse_input_event()               │
//! │  • Smart routing & ESC detection         │
//! │  • Zero-latency ESC key handling         │
//! └──────────────────────────────────────────┘
//!    │ (routes to specialized parsers)
//!    ├─→ keyboard.rs (CSI/SS3 keyboard sequences)
//!    ├─→ mouse.rs (mouse protocols)
//!    ├─→ terminal_events.rs (resize/focus/paste)
//!    └─→ utf8.rs (text input)
//!    ↓
//! VT100InputEvent
//! ```
//!
//! **Navigate**:
//! - ⬇️ **Down**: [`keyboard`], [`mouse`], [`terminal_events`], [`utf8`] - Specialized parsers
//! - 🔧 **Backend**: [`DirectToAnsiInputDevice`] - Async I/O layer that calls this
//! - 📚 **Types**: [`VT100InputEvent`] - Output event type
//!
//! ## Zero-Latency ESC Key Detection
//!
//! **The Problem**: Distinguishing ESC key presses from escape sequences (e.g., Up Arrow
//! = `ESC [ A`).
//!
//! **Baseline (crossterm)**: When reading `0x1B` alone, wait up to 150ms to see if more
//! bytes arrive. If timeout expires → emit ESC key. If bytes arrive → parse escape
//! sequence.
//!
//! **Our Approach**: Immediately emit ESC when buffer contains only `[0x1B]`, with no
//! artificial delay.
//!
//! ### Performance Comparison
//!
//! | Input Type         | crossterm Latency | Our Latency | Improvement     |
//! |--------------------|-------------------|-------------|-----------------|
//! | **ESC key press**  | 150ms (timeout)   | 0ms         | **150ms faster**|
//! | Arrow keys         | 0ms (immediate)   | 0ms         | Same            |
//! | Regular text       | 0ms (immediate)   | 0ms         | Same            |
//! | Mouse events       | 0ms (immediate)   | 0ms         | Same            |
//!
//! **Benefit applies to**: Vim-style modal editors, ESC-heavy workflows, dialog
//! dismissal.
//!
//! ### How Escape Sequences Arrive in Practice
//!
//! When you press a special key (e.g., Up Arrow), the terminal emulator sends
//! an escape sequence like `ESC [ A` (3 bytes: `[0x1B, 0x5B, 0x41]`).
//!
//! **Key Assumption**: Modern terminal emulators send escape sequences **atomically**
//! in a single `write()` syscall, and the kernel buffers all bytes together.
//!
//! #### Typical Flow (99.9% of cases - local terminals)
//!
//! ```text
//! User presses Up Arrow
//!   ↓
//! Terminal: write(stdout, "\x1B[A", 3)  ← One syscall, 3 bytes
//!   ↓
//! Kernel buffer: [0x1B, 0x5B, 0x41]    ← All bytes arrive together
//!   ↓
//! stdin.read().await → [0x1B, 0x5B, 0x41]  ← We get all 3 bytes in one read
//!   ↓
//! try_parse_input_event() sees complete sequence → Up Arrow event ✓
//! ```
//!
//! #### Edge Case: Slow Byte Arrival (rare - high-latency SSH, slow serial)
//!
//! Over high-latency connections, bytes might arrive separately:
//!
//! ```text
//! First read:  [0x1B]           → Emits ESC immediately
//! Second read: [0x5B, 0x41]     → User gets ESC instead of Up Arrow
//! ```
//!
//! **Trade-off**: We optimize for the common case (local terminals with atomic
//! sequences) to achieve 0ms ESC latency, accepting rare edge cases over forcing
//! 150ms timeout on all users.
//!
//! #### Why This Assumption Holds
//!
//! - **Local terminals** (gnome-terminal, xterm, Alacritty, iTerm2): Always send escape
//!   sequences atomically in one write
//! - **Terminal protocol design**: Sequences are designed to be atomic units
//! - **Kernel buffering**: Even with slight delays, kernel buffers complete sequences
//!   before `read()` sees them
//! - **Network delay case**: Over SSH with 200ms latency, UX is already degraded; getting
//!   ESC instead of Up Arrow is annoying but not catastrophic
//!
//! [`keyboard`]: mod@super::keyboard
//! [`mouse`]: mod@super::mouse
//! [`terminal_events`]: mod@super::terminal_events
//! [`utf8`]: mod@super::utf8
//! [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
//! [`VT100InputEvent`]: super::VT100InputEvent

use super::{VT100InputEvent, VT100KeyCode, VT100KeyModifiers,
            parse_alt_letter, parse_control_character, parse_keyboard_sequence,
            parse_mouse_sequence, parse_ss3_sequence, parse_terminal_event,
            parse_utf8_text};
use crate::core::ansi::constants::{ANSI_CSI_BRACKET, ANSI_ESC, ANSI_SS3_O};

/// Try to parse a complete input event from the buffer.
///
/// This is the main entry point for VT-100 terminal input parsing. It analyzes
/// the buffer and routes to the appropriate specialized parser based on the
/// content.
///
/// ## Smart Lookahead Logic
///
/// The parser uses intelligent 1-2 byte lookahead to determine routing:
///
/// - `[0x1B]` alone → ESC key (emitted immediately, zero-latency)
/// - `[0x1B, b'[', ...]` → CSI sequence → keyboard/mouse/terminal parsers
/// - `[0x1B, b'O', ...]` → SS3 sequence → application mode keys
/// - `[0x1B, other]` → Alt+letter or standalone ESC
/// - Other bytes → terminal events, mouse (legacy), control chars, UTF-8 text
///
/// ## Routing Algorithm
///
/// ```text
/// try_parse_input_event() uses smart 1-2 byte lookahead:
/// ┌─────────────────────────────────────────┐
/// │  First byte check                       │
/// ├─────────────────────────────────────────┤
/// │ 0x1B (ESC)?                             │
/// │  ├─ buf.len() == 1?                     │
/// │  │  └─ YES → Emit ESC immediately ▲     │
/// │  │     (zero-latency ESC key!)          │
/// │  └─ buf.len() > 1?                      │
/// │     ├─ Second byte = b'['?              │
/// │     │  └─ CSI → keyboard/mouse/terminal │
/// │     ├─ Second byte = b'O'?              │
/// │     │  └─ SS3 → app mode keys (F1-F4)   │
/// │     └─ Second byte = other?             │
/// │        └─ Alt+letter or emit ESC        │
/// ├─────────────────────────────────────────┤
/// │ Not ESC?                                │
/// │  └─ Try: terminal → mouse → control     │
/// │          characters → UTF-8             │
/// └─────────────────────────────────────────┘
/// ```
///
/// ## Returns
///
/// `Some((event, bytes_consumed))` if a complete event was successfully parsed.
/// Returns the protocol-level [`VT100InputEvent`] with the number of bytes consumed.
///
/// `None` if the buffer contains an incomplete sequence (more bytes needed).
///
/// ## Examples
///
/// ```
/// use r3bl_tui::core::ansi::vt_100_terminal_input_parser::{try_parse_input_event,
///                                                           VT100InputEvent,
///                                                           VT100KeyCode};
///
/// // Parse ESC key (single byte, immediate)
/// let buffer = &[0x1B];
/// if let Some((event, consumed)) = try_parse_input_event(buffer) {
///     assert!(matches!(event, VT100InputEvent::Keyboard {
///         code: VT100KeyCode::Escape, ..
///     }));
///     assert_eq!(consumed, 1);
/// }
///
/// // Parse Up Arrow (CSI sequence)
/// let buffer = &[0x1B, b'[', b'A'];
/// if let Some((event, consumed)) = try_parse_input_event(buffer) {
///     assert!(matches!(event, VT100InputEvent::Keyboard {
///         code: VT100KeyCode::Up, ..
///     }));
///     assert_eq!(consumed, 3);
/// }
///
/// // Parse regular text
/// let buffer = b"Hello";
/// if let Some((event, consumed)) = try_parse_input_event(buffer) {
///     assert!(matches!(event, VT100InputEvent::Keyboard {
///         code: VT100KeyCode::Char('H'), ..
///     }));
///     assert_eq!(consumed, 1);
/// }
/// ```
#[must_use]
pub fn try_parse_input_event(buffer: &[u8]) -> Option<(VT100InputEvent, usize)> {
    // Fast path: empty buffer.
    if buffer.is_empty() {
        return None;
    }

    // Check first byte for routing.
    match buffer.first() {
        Some(&ANSI_ESC) => {
            // ESC sequence or ESC key.
            if buffer.len() == 1 {
                // Just ESC, emit immediately (no timeout!).
                let esc_event = VT100InputEvent::Keyboard {
                    code: VT100KeyCode::Escape,
                    modifiers: VT100KeyModifiers::default(),
                };
                return Some((esc_event, 1));
            }

            // Check second byte.
            match buffer.get(1) {
                Some(&ANSI_CSI_BRACKET) => {
                    // CSI sequence - try keyboard first, then mouse, then terminal
                    // events.
                    parse_keyboard_sequence(buffer)
                        .or_else(|| parse_mouse_sequence(buffer))
                        .or_else(|| parse_terminal_event(buffer))
                }
                Some(&ANSI_SS3_O) => {
                    // SS3 sequence - application mode keys (F1-F4, Home, End,
                    // arrows).
                    parse_ss3_sequence(buffer)
                }
                Some(_) => {
                    // ESC + unknown byte - try Alt+letter before emitting standalone
                    // ESC. This handles Alt+B (ESC+'b'),
                    // Alt+F (ESC+'f'), etc.
                    parse_alt_letter(buffer).or_else(|| {
                        // Not Alt+letter, emit standalone ESC
                        let esc_event = VT100InputEvent::Keyboard {
                            code: VT100KeyCode::Escape,
                            modifiers: VT100KeyModifiers::default(),
                        };
                        Some((esc_event, 1))
                    })
                }
                None => {
                    // Shouldn't reach here (buf.len() > 1 but get(1) is None?).
                    unreachable!()
                }
            }
        }
        Some(_) => {
            // Not ESC - try terminal events, mouse (X10/RXVT), control characters, or
            // UTF-8 text. Control characters (0x00-0x1F like Ctrl+A,
            // Ctrl+D, Ctrl+W) must be tried before UTF-8 because they
            // are technically valid UTF-8 but should be parsed as Ctrl+letter
            // instead.
            parse_terminal_event(buffer)
                .or_else(|| parse_mouse_sequence(buffer))
                .or_else(|| parse_control_character(buffer))
                .or_else(|| parse_utf8_text(buffer))
        }
        None => {
            // Empty buffer (shouldn't reach here due to early return).
            unreachable!()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KeyState;

    #[test]
    fn test_try_parse_esc_key_immediate() {
        // Test: Single ESC byte should emit ESC key immediately (zero-latency)
        let buffer = &[0x1B];
        let (event, consumed) =
            try_parse_input_event(buffer).expect("Should parse ESC key");

        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Escape,
                ..
            }
        ));
        assert_eq!(consumed, 1);
    }

    #[test]
    fn test_try_parse_csi_routing_keyboard() {
        // Test: CSI sequence routes to keyboard parser
        let buffer = &[0x1B, b'[', b'A']; // ESC [ A (Up Arrow)
        let (event, consumed) =
            try_parse_input_event(buffer).expect("Should parse Up Arrow");

        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Up,
                ..
            }
        ));
        assert_eq!(consumed, 3);
    }

    #[test]
    fn test_try_parse_csi_routing_mouse() {
        // Test: CSI mouse sequence routes to mouse parser
        // SGR mouse protocol: ESC [ < 0 ; 10 ; 20 M (left button press at col=10,
        // row=20)
        let buffer = b"\x1b[<0;10;20M";
        let result = try_parse_input_event(buffer);

        assert!(result.is_some(), "Should parse mouse event");
        let (event, consumed) = result.unwrap();
        assert!(matches!(event, VT100InputEvent::Mouse { .. }));
        assert_eq!(consumed, buffer.len());
    }

    #[test]
    fn test_try_parse_ss3_routing() {
        // Test: SS3 sequence routes to SS3 parser
        let buffer = &[0x1B, b'O', b'P']; // ESC O P (F1 in application mode)
        let (event, consumed) =
            try_parse_input_event(buffer).expect("Should parse F1");

        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Function(1),
                ..
            }
        ));
        assert_eq!(consumed, 3);
    }

    #[test]
    fn test_try_parse_alt_letter_routing() {
        // Test: ESC + printable ASCII routes to Alt+letter parser
        let buffer = &[0x1B, b'b']; // ESC b (Alt+b)
        let (event, consumed) =
            try_parse_input_event(buffer).expect("Should parse Alt+b");

        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Char('b'),
                modifiers: VT100KeyModifiers { alt, .. }
            } if alt == KeyState::Pressed
        ));
        assert_eq!(consumed, 2);
    }

    #[test]
    fn test_try_parse_control_character_routing() {
        // Test: Control character (Ctrl+A) routes to control character parser
        let buffer = &[0x01]; // Ctrl+A
        let (event, consumed) =
            try_parse_input_event(buffer).expect("Should parse Ctrl+A");

        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Char('a'),
                modifiers: VT100KeyModifiers { ctrl, .. }
            } if ctrl == KeyState::Pressed
        ));
        assert_eq!(consumed, 1);
    }

    #[test]
    fn test_try_parse_utf8_fallback() {
        // Test: Regular ASCII routes to UTF-8 parser
        let buffer = b"Hello";
        let (event, consumed) =
            try_parse_input_event(buffer).expect("Should parse 'H'");

        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Char('H'),
                ..
            }
        ));
        assert_eq!(consumed, 1);
    }

    #[test]
    fn test_try_parse_empty_buffer() {
        // Test: Empty buffer returns None
        let buffer = &[];
        assert!(try_parse_input_event(buffer).is_none());
    }

    #[test]
    fn test_try_parse_incomplete_csi_sequence() {
        // Test: Incomplete CSI sequence returns None
        let buffer = &[0x1B, b'[']; // Just ESC [ without final byte
        assert!(try_parse_input_event(buffer).is_none());
    }

    #[test]
    fn test_try_parse_unknown_esc_sequence() {
        // Test: ESC + unknown byte emits standalone ESC
        let buffer = &[0x1B, 0xFF]; // ESC + invalid byte
        let (event, consumed) =
            try_parse_input_event(buffer).expect("Should emit standalone ESC");

        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Escape,
                ..
            }
        ));
        assert_eq!(consumed, 1); // Only consume ESC, leave 0xFF for next parse
    }
}
