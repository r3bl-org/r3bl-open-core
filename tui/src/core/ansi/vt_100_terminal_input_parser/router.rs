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
/// # Zero-Latency `ESC` Key Detection
///
/// ## The Problem
///
/// Both `ESC` key presses and escape sequences (e.g., Up Arrow = `ESC [ A`) start with
/// [`ANSI_ESC`] (`0x1B`), so when we read that byte, is it a standalone `ESC` or the
/// start of a multi-byte sequence?
///
/// ## The Conventional Solution (crossterm)
///
/// When reading [`ANSI_ESC`] alone, wait up to 150ms to see if more bytes arrive. If
/// timeout expires → emit `ESC` key. If bytes arrive → parse escape sequence. This
/// guarantees correctness but adds 150ms latency to every `ESC` key press.
///
/// ## Key Insight: Escape Sequences Arrive Atomically
///
/// On POSIX systems (Linux, macOS), terminal emulators send escape sequences
/// **atomically** in a single [`write()`][write-syscall] syscall, and the kernel buffers
/// all bytes together.
///
/// This parser is currently used only on Linux via [`DirectToAnsiInputDevice`] and
/// [`TERMINAL_LIB_BACKEND`]; macOS and Windows use the crossterm backend.
///
/// ```text
/// User presses Up Arrow
///   ↓
/// Terminal: write(stdout, "\x1B[A", 3)     ← One syscall, 3 bytes
///   ↓
/// Kernel buffer: [0x1B, 0x5B, 0x41]        ← All bytes arrive together
///   ↓
/// stdin.read().await → [0x1B, 0x5B, 0x41]  ← We get all 3 bytes in one read
/// ```
///
/// This holds because:
/// - **Local terminals** (gnome-terminal, xterm, Alacritty, iTerm2): Always send escape
///   sequences atomically in one write.
/// - **Terminal protocol design**: Sequences are designed to be atomic units.
/// - **Kernel buffering**: Even with slight delays, kernel buffers complete sequences
///   before `read()` sees them.
///
/// ## Our Approach
///
/// Given atomic delivery, we immediately emit `ESC` when buffer contains only
/// [`ANSI_ESC`], with no artificial delay. If escape sequences always arrive complete, a
/// lone [`ANSI_ESC`] byte means the user pressed `ESC`.
///
/// ## Trade-off: Edge Cases
///
/// Over high-latency connections (SSH, slow serial), bytes might arrive separately:
///
/// ```text
/// First read:  [0x1B]           → Emits `ESC` immediately
/// Second read: [0x5B, 0x41]     → User gets `ESC` instead of Up Arrow
/// ```
///
/// We accept this rare edge case because:
/// - Over SSH with 200ms latency, UX is already degraded
/// - Getting `ESC` instead of Up Arrow is annoying but not catastrophic
/// - The alternative (150ms timeout for everyone) penalizes 99.9% of users
///
/// ## Performance Summary
///
/// | Input Type          | crossterm Latency | Our Latency | Improvement      |
/// | ------------------- | ----------------- | ----------- | ---------------- |
/// | **`ESC` key press** | 150ms (timeout)   | 0ms         | **150ms faster** |
/// | Arrow keys          | 0ms (immediate)   | 0ms         | Same             |
/// | Regular text        | 0ms (immediate)   | 0ms         | Same             |
/// | Mouse events        | 0ms (immediate)   | 0ms         | Same             |
///
/// **Benefits**: Vim-style modal editors, `ESC`-heavy workflows, dialog dismissal.
///
/// # Smart Lookahead Logic
///
/// The parser uses intelligent 1-2 byte lookahead to determine routing:
///
/// | Input Pattern        | Interpretation      | Routing                                     |
/// | -------------------- | ------------------- | ------------------------------------------- |
/// | `[ 0x1B ]` alone     | `ESC` key           | Emitted immediately, zero-latency           |
/// | `[ 0x1B, b'[', .. ]` | `CSI` sequence      | keyboard/mouse/terminal parsers             |
/// | `[ 0x1B, b'O', .. ]` | `SS3` sequence      | F1-F4, Home, End, arrows (application mode) |
/// | `[ 0x1B, other ]`    | Alt+letter or `ESC` | try Alt+letter, else emit standalone `ESC`  |
/// | Other bytes          | various             | terminal/mouse/control/UTF-8                |
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
///     cycle. This enables zero-latency `ESC` detection without the 150ms timeout other
///     parsers use.
///
/// # Routing Algorithm
///
/// ```text
/// try_parse_input_event() uses smart 1-2 byte lookahead:
/// ┌────────────────────────────────────────────────────┐
/// │ First byte check                                   │
/// ├────────────────────────────────────────────────────┤
/// │ 0x1B (`ESC`)?                                      │
/// │  ├─ buf.len() == 1?                                │
/// │  │  └─ YES → Emit `ESC` immediately ▲              │
/// │  │     (zero-latency `ESC` key!)                   │
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
/// `None` if the buffer contains an incomplete sequence (more bytes needed).
///
/// # Examples
///
/// ```
/// use r3bl_tui::core::ansi::vt_100_terminal_input_parser::{try_parse_input_event,
///                                                           VT100InputEventIR,
///                                                           VT100KeyCodeIR};
/// use r3bl_tui::byte_offset;
///
/// // Parse `ESC` key (single byte, immediate)
/// let buffer = &[0x1B];
/// if let Some((event, consumed)) = try_parse_input_event(buffer) {
///     assert!(matches!(event, VT100InputEventIR::Keyboard {
///         code: VT100KeyCodeIR::Escape, ..
///     }));
///     assert_eq!(consumed, byte_offset(1));
/// }
///
/// // Parse Up Arrow (`CSI` sequence)
/// let buffer = &[0x1B, b'[', b'A'];
/// if let Some((event, consumed)) = try_parse_input_event(buffer) {
///     assert!(matches!(event, VT100InputEventIR::Keyboard {
///         code: VT100KeyCodeIR::Up, ..
///     }));
///     assert_eq!(consumed, byte_offset(3));
/// }
///
/// // Parse regular text
/// let buffer = b"Hello";
/// if let Some((event, consumed)) = try_parse_input_event(buffer) {
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
pub fn try_parse_input_event(buffer: &[u8]) -> Option<(VT100InputEventIR, ByteOffset)> {
    // Routing table.
    match buffer {
        // Empty buffer.
        [] => None,

        // Single ESC byte - emit immediately (zero-latency, no timeout!).
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
            try_parse_input_event(&buffer).expect("Should parse Up Arrow");

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
            try_parse_input_event(&buffer).expect("Should parse mouse event");

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
        let (event, consumed) = try_parse_input_event(buffer).expect("Should parse F1");

        assert_eq!(event, expected);
        assert_eq!(consumed, byte_offset(3));
    }

    #[test]
    fn terminal_event_focus() {
        // Focus gained.
        let focus_gained = VT100InputEventIR::Focus(VT100FocusStateIR::Gained);
        let buffer = generate_keyboard_sequence(&focus_gained).unwrap();
        let (event, consumed) =
            try_parse_input_event(&buffer).expect("Should parse focus gained");

        assert_eq!(event, focus_gained);
        assert_eq!(consumed.as_usize(), buffer.len());

        // Focus lost.
        let focus_lost = VT100InputEventIR::Focus(VT100FocusStateIR::Lost);
        let buffer = generate_keyboard_sequence(&focus_lost).unwrap();
        let (event, consumed) =
            try_parse_input_event(&buffer).expect("Should parse focus lost");

        assert_eq!(event, focus_lost);
        assert_eq!(consumed.as_usize(), buffer.len());
    }

    #[test]
    fn terminal_event_paste() {
        // Bracketed paste start.
        let paste_start = VT100InputEventIR::Paste(VT100PasteModeIR::Start);
        let buffer = generate_keyboard_sequence(&paste_start).unwrap();
        let (event, consumed) =
            try_parse_input_event(&buffer).expect("Should parse paste start");

        assert_eq!(event, paste_start);
        assert_eq!(consumed.as_usize(), buffer.len());

        // Bracketed paste end.
        let paste_end = VT100InputEventIR::Paste(VT100PasteModeIR::End);
        let buffer = generate_keyboard_sequence(&paste_end).unwrap();
        let (event, consumed) =
            try_parse_input_event(&buffer).expect("Should parse paste end");

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
    fn esc_key_immediate() {
        // Single ESC byte emits immediately (zero-latency, no timeout).
        let expected = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Escape,
            modifiers: VT100KeyModifiersIR::default(),
        };
        let buffer = generate_keyboard_sequence(&expected).unwrap();
        let (event, consumed) =
            try_parse_input_event(&buffer).expect("Should parse ESC key");

        assert_eq!(event, expected);
        assert_eq!(consumed.as_usize(), buffer.len());
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
            try_parse_input_event(&buffer).expect("Should parse Alt+b");

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
            try_parse_input_event(&buffer).expect("Should parse Ctrl+A");

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
        let (event, consumed) = try_parse_input_event(&buffer).expect("Should parse 'H'");

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
        assert!(try_parse_input_event(buffer).is_none());
    }

    #[test]
    fn incomplete_csi_sequence_returns_none() {
        // ESC [ without final byte - waiting for more input.
        let buffer = &[0x1B, b'['];
        assert!(try_parse_input_event(buffer).is_none());
    }

    #[test]
    fn unknown_esc_sequence_emits_standalone_esc() {
        // ESC + invalid byte → emit standalone ESC, leave invalid byte for next cycle.
        let buffer = &[0x1B, 0xFF];
        let (event, consumed) =
            try_parse_input_event(buffer).expect("Should emit standalone ESC");

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
