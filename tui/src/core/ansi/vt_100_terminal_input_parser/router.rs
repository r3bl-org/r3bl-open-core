// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module exports [`try_parse_input_event`], the VT-100 terminal input parser entry
//! point for converting raw bytes into terminal input events. See the function
//! documentation for full details.

use super::{VT100InputEventIR, VT100KeyCodeIR, VT100KeyModifiersIR, keyboard, mouse,
            terminal_events, utf8};
use crate::{ByteOffset, byte_offset,
            core::ansi::constants::{ANSI_CSI_BRACKET, ANSI_ESC, ANSI_SS3_O}};

/// Parse a complete input event from a byte buffer.
///
/// This is the main entry point for VT-100 terminal input parsing. It analyzes the
/// buffer and routes to specialized parsers ([`keyboard`], [`mouse`],
/// [`terminal_events`], [`utf8`]) based on content analysis.
///
/// # Parameters
///
/// - `buffer`: The accumulated bytes to parse.
/// - `input_available`: Whether more input is likely available in the kernel buffer.
///   Computed by the caller as `read_count == TTY_BUFFER_SIZE` (crossterm pattern).
///   - When `true` and buffer is `[ESC]`: Return `None` (wait for more bytes).
///   - When `false` and buffer is `[ESC]`: Emit ESC key immediately.
///   - For all other inputs: This flag has no effect.
///
/// # Where You Are in the Pipeline
///
/// For the full data flow, see the [parent module documentation]. This diagram shows
/// where this function fits:
///
/// ```text
/// DirectToAnsiInputDevice (async I/O layer)
///    │
///    │ Reads from tokio::io::stdin(), calls try_parse_input_event()
///    ▼
/// ┌──────────────────────────────────────────┐  ┌──────────────────┐
/// │  try_parse_input_event()                 ◀──┤ **YOU ARE HERE** │
/// │  • Smart routing & `ESC` detection       │  └──────────────────┘
/// │  • Zero-latency `ESC` key handling       │
/// └──────────────────────────────────────────┘
///    │ (routes to specialized parsers)
///    ├─→ keyboard.rs (`CSI`/`SS3` keyboard sequences)
///    ├─→ mouse.rs (mouse protocols)
///    ├─→ terminal_events.rs (resize/focus/paste)
///    └─→ utf8.rs (text input)
///    │
///    ▼
/// VT100InputEventIR
///    │
///    ▼
/// convert_input_event() → InputEvent (returned to application)
/// ```
///
/// **Navigate**:
/// - ⬆️ **Up**: [`DirectToAnsiInputDevice`] - Async I/O layer that calls this
/// - ⬇️ **Down**: [`keyboard`], [`mouse`], [`terminal_events`], [`utf8`] - Specialized
///   parsers
/// - 📚 **Types**: [`VT100InputEventIR`] - Output event type
///
/// # ESC Key Detection (Crossterm Pattern)
///
/// ## The Problem
///
/// Both `ESC` key presses and escape sequences (e.g., Up Arrow = `ESC [ A`) start with
/// [`ANSI_ESC`] (`0x1B`), so when we read that byte, is it a standalone `ESC` or the
/// start of a multi-byte sequence?
///
/// ## Crossterm's Solution: The `input_available` Flag
///
/// Instead of using a timeout (which adds latency), crossterm uses the `input_available`
/// flag to disambiguate. This flag is computed as `read_count == TTY_BUFFER_SIZE`:
///
/// - If the read filled the entire buffer, more data is likely waiting in the kernel.
/// - If the read returned fewer bytes, we've drained all available input.
///
/// This works because:
/// - **Over SSH**: Bytes may arrive in fragments, but if we read fewer bytes than the
///   buffer size, we know there's no more data waiting right now.
/// - **Locally**: Terminal emulators send escape sequences atomically, so they arrive
///   complete in a single read.
///
/// ## Algorithm
///
/// ```text
/// if buffer == [ESC] {
///     if input_available {
///         return None  // Wait for more bytes (might be escape sequence)
///     } else {
///         return ESC key  // No more input, user pressed ESC
///     }
/// }
/// ```
///
/// ## Why This Avoids Fixed Timeouts
///
/// Unlike a fixed 150ms timeout approach, the `input_available` flag provides
/// **adaptive waiting**:
///
/// - **Local terminals**: Escape sequences arrive atomically, so `input_available`
///   is usually `false` after reading—we emit ESC immediately when appropriate.
/// - **SSH/high-latency**: If bytes arrive separately, `input_available` tells us
///   when more data is pending—we wait correctly without a fixed timeout.
///
/// **Benefits**: Vim-style modal editors, `ESC`-heavy workflows, dialog dismissal.
/// **SSH compatibility**: Works correctly because we wait for more bytes when available.
///
/// # Smart Lookahead Logic
///
/// The parser uses intelligent 1-2 byte lookahead to determine routing:
///
/// | Input Pattern            | `input_available` | Routing                             |
/// |:-------------------------|:------------------|:------------------------------------|
/// | `[ 0x1B ]` alone         | `false`           | Emit `ESC` key immediately          |
/// | `[ 0x1B ]` alone         | `true`            | Return `None` (wait for more bytes) |
/// | `[ 0x1B, b'[', .. ]`     | (ignored)         | `CSI` → keyboard/mouse/terminal     |
/// | `[ 0x1B, b'O', .. ]`     | (ignored)         | `SS3` → F1-F4, Home, End, arrows    |
/// | `[ 0x1B, other ]`        | (ignored)         | Alt+letter or emit standalone `ESC` |
/// | Other bytes              | (ignored)         | control char → UTF-8                |
///
/// - `CSI` (Control Sequence Introducer):
///   - The most common escape sequence format, starting with `ESC [`. Used for arrow
///     keys, function keys, mouse events, and terminal queries.
///   - Example: `ESC [ A` is Up arrow, `ESC [ 1 ; 5 C` is Ctrl+Right.
/// - `SS3` (Single Shift 3) / Application mode:
///   - Terminals can switch between "normal" and "application" mode. Programs like vim,
///     less, and emacs enable this mode.
///   - In application mode, arrow keys and F1-F4 send `ESC O x` (`SS3`) instead of `ESC [
///     x` (`CSI`).
/// - Alt+letter fallback:
///   - Terminals historically couldn't send a dedicated Alt modifier, so they send `ESC`
///     followed by the letter (e.g., `ESC b` for Alt+B).
///   - When we see `ESC` + unknown byte, we first try to parse it as Alt+letter. If that
///     fails, we emit a standalone `ESC` and leave the next byte for the next parse
///     cycle.
///
/// # Routing Algorithm
///
/// ```text
/// try_parse_input_event(buffer, input_available):
/// ┌────────────────────────────────────────────────────┐
/// │ First byte check                                   │
/// ├────────────────────────────────────────────────────┤
/// │ 0x1B (`ESC`)?                                      │
/// │  ├─ buf.len() == 1?                                │
/// │  │  ├─ input_available == true?                    │
/// │  │  │  └─ Return None (wait for more bytes)        │
/// │  │  └─ input_available == false?                   │
/// │  │     └─ Emit `ESC` key immediately               │
/// │  └─ buf.len() >= 2?                                │
/// │     ├─ Second byte = b'['?                         │
/// │     │  └─ `CSI` → keyboard/mouse/terminal_events   │
/// │     ├─ Second byte = b'O'?                         │
/// │     │  └─ `SS3` → application mode keys            │
/// │     └─ Second byte = other?                        │
/// │        └─ Alt+letter or emit `ESC`                 │
/// ├────────────────────────────────────────────────────┤
/// │ Not ESC?                                           │
/// │  └─ Raw byte: control_char → UTF-8                 │
/// └────────────────────────────────────────────────────┘
/// ```
///
/// # Returns
///
/// `Some((event, bytes_consumed))` if a complete event was successfully parsed.
/// Returns the protocol-level [`VT100InputEventIR`] with the number of bytes consumed
/// as a [`ByteOffset`].
///
/// `None` if the buffer contains an incomplete sequence (more bytes needed), or if
/// `input_available` is `true` and buffer is `[ESC]` (waiting for potential escape
/// sequence).
///
/// # Examples
///
/// ```
/// use r3bl_tui::core::ansi::vt_100_terminal_input_parser::{try_parse_input_event,
///                                                           VT100InputEventIR,
///                                                           VT100KeyCodeIR};
/// use r3bl_tui::byte_offset;
///
/// // Parse `ESC` key - no more input available, emit immediately.
/// let buffer = &[0x1B];
/// if let Some((event, consumed)) = try_parse_input_event(buffer, false) {
///     assert!(matches!(event, VT100InputEventIR::Keyboard {
///         code: VT100KeyCodeIR::Escape, ..
///     }));
///     assert_eq!(consumed, byte_offset(1));
/// }
///
/// // Lone ESC with more input available - wait for more bytes.
/// let buffer = &[0x1B];
/// assert!(try_parse_input_event(buffer, true).is_none());
///
/// // Parse Up Arrow (`CSI` sequence) - input_available doesn't matter.
/// let buffer = &[0x1B, b'[', b'A'];
/// if let Some((event, consumed)) = try_parse_input_event(buffer, false) {
///     assert!(matches!(event, VT100InputEventIR::Keyboard {
///         code: VT100KeyCodeIR::Up, ..
///     }));
///     assert_eq!(consumed, byte_offset(3));
/// }
///
/// // Parse regular text.
/// let buffer = b"Hello";
/// if let Some((event, consumed)) = try_parse_input_event(buffer, false) {
///     assert!(matches!(event, VT100InputEventIR::Keyboard {
///         code: VT100KeyCodeIR::Char('H'), ..
///     }));
///     assert_eq!(consumed, byte_offset(1));
/// }
/// ```
///
/// [`ANSI_ESC`]: crate::ANSI_ESC
/// [`ByteOffset`]: crate::ByteOffset
/// [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
/// [`TERMINAL_LIB_BACKEND`]: crate::TERMINAL_LIB_BACKEND
/// [`VT100InputEventIR`]: super::VT100InputEventIR
/// [`keyboard`]: mod@super::keyboard
/// [`mouse`]: mod@super::mouse
/// [`terminal_events`]: mod@super::terminal_events
/// [`utf8`]: mod@super::utf8
/// [parent module documentation]: mod@super#primary-consumer
/// [write-syscall]: https://man7.org/linux/man-pages/man2/write.2.html
#[must_use]
pub fn try_parse_input_event(
    buffer: &[u8],
    input_available: bool,
) -> Option<(VT100InputEventIR, ByteOffset)> {
    // Routing table.
    match buffer {
        // Empty buffer.
        [] => None,

        // Single ESC byte - check input_available flag (crossterm pattern).
        // - input_available == true: Wait for more bytes (might be escape sequence).
        // - input_available == false: Emit ESC key immediately (no more input).
        [ANSI_ESC] if input_available => None,
        [ANSI_ESC] => Some((esc_key_event(), byte_offset(1))),

        // CSI sequence (ESC [) - keyboard/mouse/terminal events.
        [ANSI_ESC, ANSI_CSI_BRACKET, ..] => keyboard::parse_keyboard_sequence(buffer)
            .or_else(|| mouse::parse_mouse_sequence(buffer))
            .or_else(|| terminal_events::parse_terminal_event(buffer)),

        // SS3 sequence (ESC O) - application mode keys (F1-F4, Home, End, arrows).
        [ANSI_ESC, ANSI_SS3_O, ..] => keyboard::parse_ss3_sequence(buffer),

        // ESC + other byte - try Alt+letter (e.g., Alt+B, Alt+F), else emit standalone
        // ESC.
        [ANSI_ESC, _, ..] => keyboard::parse_alt_letter(buffer)
            .or_else(|| Some((esc_key_event(), byte_offset(1)))),

        // Not ESC - raw byte input (control characters or UTF-8 text).
        // Control characters (0x00-0x1F) must be tried before UTF-8 because they are
        // technically valid UTF-8 but should be parsed as Ctrl+letter instead.
        _ => keyboard::parse_control_character(buffer)
            .or_else(|| utf8::parse_utf8_text(buffer)),
    }
}

/// Helper to create an ESC key event.
fn esc_key_event() -> VT100InputEventIR {
    VT100InputEventIR::Keyboard {
        code: VT100KeyCodeIR::Escape,
        modifiers: VT100KeyModifiersIR::default(),
    }
}

/// Tests for CSI/SS3 sequence routing using generators for round-trip validation.
#[cfg(test)]
mod tests_csi_routing {
    use super::*;
    use crate::{TermPos,
                core::ansi::vt_100_terminal_input_parser::{VT100FocusStateIR,
                                                           VT100MouseActionIR,
                                                           VT100MouseButtonIR,
                                                           VT100PasteModeIR,
                                                           test_fixtures::generate_keyboard_sequence}};

    #[test]
    fn keyboard_arrow_key() {
        let expected = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Up,
            modifiers: VT100KeyModifiersIR::default(),
        };
        let buffer = generate_keyboard_sequence(&expected).unwrap();
        let (event, consumed) =
            try_parse_input_event(&buffer, false).expect("Should parse Up Arrow");

        assert_eq!(event, expected);
        assert_eq!(consumed.as_usize(), buffer.len());
    }

    #[test]
    fn mouse_sgr_format() {
        let expected = VT100InputEventIR::Mouse {
            button: VT100MouseButtonIR::Left,
            pos: TermPos::from_one_based(10, 20),
            action: VT100MouseActionIR::Press,
            modifiers: VT100KeyModifiersIR::default(),
        };
        let buffer = generate_keyboard_sequence(&expected).unwrap();
        let (event, consumed) =
            try_parse_input_event(&buffer, false).expect("Should parse mouse event");

        assert_eq!(event, expected);
        assert_eq!(consumed.as_usize(), buffer.len());
    }

    #[test]
    fn ss3_function_key() {
        // SS3 format (ESC O P) for F1 in application mode.
        // Note: Generator produces CSI format; we test SS3 directly.
        let expected = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Function(1),
            modifiers: VT100KeyModifiersIR::default(),
        };
        let buffer = &[0x1B, b'O', b'P']; // ESC O P
        let (event, consumed) =
            try_parse_input_event(buffer, false).expect("Should parse F1");

        assert_eq!(event, expected);
        assert_eq!(consumed, byte_offset(3));
    }

    #[test]
    fn terminal_event_focus() {
        // Focus gained.
        let focus_gained = VT100InputEventIR::Focus(VT100FocusStateIR::Gained);
        let buffer = generate_keyboard_sequence(&focus_gained).unwrap();
        let (event, consumed) =
            try_parse_input_event(&buffer, false).expect("Should parse focus gained");

        assert_eq!(event, focus_gained);
        assert_eq!(consumed.as_usize(), buffer.len());

        // Focus lost.
        let focus_lost = VT100InputEventIR::Focus(VT100FocusStateIR::Lost);
        let buffer = generate_keyboard_sequence(&focus_lost).unwrap();
        let (event, consumed) =
            try_parse_input_event(&buffer, false).expect("Should parse focus lost");

        assert_eq!(event, focus_lost);
        assert_eq!(consumed.as_usize(), buffer.len());
    }

    #[test]
    fn terminal_event_paste() {
        // Bracketed paste start.
        let paste_start = VT100InputEventIR::Paste(VT100PasteModeIR::Start);
        let buffer = generate_keyboard_sequence(&paste_start).unwrap();
        let (event, consumed) =
            try_parse_input_event(&buffer, false).expect("Should parse paste start");

        assert_eq!(event, paste_start);
        assert_eq!(consumed.as_usize(), buffer.len());

        // Bracketed paste end.
        let paste_end = VT100InputEventIR::Paste(VT100PasteModeIR::End);
        let buffer = generate_keyboard_sequence(&paste_end).unwrap();
        let (event, consumed) =
            try_parse_input_event(&buffer, false).expect("Should parse paste end");

        assert_eq!(event, paste_end);
        assert_eq!(consumed.as_usize(), buffer.len());
    }
}

/// Tests for non-CSI input: single bytes and ESC+byte sequences.
/// Validates parsing of ESC key, Alt+letter, control characters, and UTF-8 text.
#[cfg(test)]
mod tests_non_csi_input {
    use super::*;
    use crate::{KeyState,
                core::ansi::vt_100_terminal_input_parser::test_fixtures::generate_keyboard_sequence};

    #[test]
    fn esc_key_immediate_when_no_more_input() {
        // Single ESC byte emits immediately when input_available == false.
        let expected = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Escape,
            modifiers: VT100KeyModifiersIR::default(),
        };
        let buffer = generate_keyboard_sequence(&expected).unwrap();
        let (event, consumed) =
            try_parse_input_event(&buffer, false).expect("Should parse ESC key");

        assert_eq!(event, expected);
        assert_eq!(consumed.as_usize(), buffer.len());
    }

    #[test]
    fn esc_key_waits_when_more_input_available() {
        // Single ESC byte returns None when input_available == true.
        let buffer = &[0x1B]; // ESC
        assert!(
            try_parse_input_event(buffer, true).is_none(),
            "Should return None when more input might be coming"
        );
    }

    #[test]
    fn alt_letter() {
        // ESC + printable ASCII = Alt+letter.
        let expected = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Char('b'),
            modifiers: VT100KeyModifiersIR {
                alt: KeyState::Pressed,
                ..Default::default()
            },
        };
        let buffer = generate_keyboard_sequence(&expected).unwrap();
        let (event, consumed) =
            try_parse_input_event(&buffer, false).expect("Should parse Alt+b");

        assert_eq!(event, expected);
        assert_eq!(consumed.as_usize(), buffer.len());
    }

    #[test]
    fn control_character() {
        // Control character (Ctrl+A = 0x01).
        let expected = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Char('a'),
            modifiers: VT100KeyModifiersIR {
                ctrl: KeyState::Pressed,
                ..Default::default()
            },
        };
        let buffer = generate_keyboard_sequence(&expected).unwrap();
        let (event, consumed) =
            try_parse_input_event(&buffer, false).expect("Should parse Ctrl+A");

        assert_eq!(event, expected);
        assert_eq!(consumed.as_usize(), buffer.len());
    }

    #[test]
    fn utf8_char() {
        // Regular ASCII character.
        let expected = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Char('H'),
            modifiers: VT100KeyModifiersIR::default(),
        };
        let buffer = generate_keyboard_sequence(&expected).unwrap();
        let (event, consumed) =
            try_parse_input_event(&buffer, false).expect("Should parse 'H'");

        assert_eq!(event, expected);
        assert_eq!(consumed.as_usize(), buffer.len());
    }
}

/// Tests for invalid and incomplete input sequences.
/// Uses hardcoded bytes intentionally - generators only produce valid sequences.
#[cfg(test)]
mod tests_invalid_input {
    use super::*;

    #[test]
    fn empty_buffer_returns_none() {
        let buffer: &[u8] = &[];
        assert!(try_parse_input_event(buffer, false).is_none());
    }

    #[test]
    fn incomplete_csi_sequence_returns_none() {
        // ESC [ without final byte - waiting for more input.
        let buffer = &[0x1B, b'['];
        assert!(try_parse_input_event(buffer, false).is_none());
    }

    #[test]
    fn unknown_esc_sequence_emits_standalone_esc() {
        // ESC + invalid byte → emit standalone ESC, leave invalid byte for next cycle.
        let buffer = &[0x1B, 0xFF];
        let (event, consumed) =
            try_parse_input_event(buffer, false).expect("Should emit standalone ESC");

        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Escape,
                modifiers: VT100KeyModifiersIR::default(),
            }
        );
        // Only consume 1 byte (ESC), leave 0xFF for next parse.
        assert_eq!(consumed, byte_offset(1));
    }
}
