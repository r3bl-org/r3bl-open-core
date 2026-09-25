// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module exports [`try_parse_input_event`], the [`VT-100`] terminal input parser
//! entry point for converting raw bytes into terminal input events.
//!
//! ## Parser Dispatch Priority Pipeline
//!
//! When bytes are accumulated, [`try_parse_input_event`] dispatches them across
//! specialized parsers in a **predefined priority order**:
//!
//! ### 1. [`CSI`] Sequences (`ESC [`...)
//!
//! When the buffer begins with `ESC [`:
//! 1. **`parse_keyboard_sequence()`** ([`keyboard`]) - Arrow keys, function keys,
//!    modified keys with [`CSI`] format (e.g., `ESC [ A` for Up, `ESC [ 1 ; 5 A` for
//!    Ctrl+Up, `ESC [ 1 5 ~` for F5).
//! 2. **`parse_mouse_sequence()`** ([`mouse`]) - [`SGR`] mouse protocol for clicks,
//!    drags, scrolling (e.g., `ESC [ < 0 ; 10 ; 20 M` for left click, `ESC [ < 64 ; 10 ;
//!    20 M` for scroll up).
//! 3. **`parse_terminal_event()`** ([`terminal_events`]) - Window resize, focus
//!    gained/lost, paste markers (e.g., `ESC [ 8 ; 24 ; 80 t` for resize, `ESC [ I` for
//!    focus gained).
//!
//! ### 2. SS3 Sequences (`ESC O`...)
//!
//! When the buffer begins with `ESC O`:
//! - **`parse_ss3_sequence()`** ([`keyboard`]) - Application mode keys (F1-F4, Home, End,
//!   arrows, e.g., `ESC O P` for F1, `ESC O A` for Up in app mode).
//!
//! ### 3. [`OSC`] Sequences & `Alt+]` Disambiguation (`ESC ]`...)
//!
//! In [`VT-100`] terminals, both the `Alt+]` key combination and terminal-generated
//! [`OSC`] responses begin with `ESC ]` (`\x1b]`).
//!
//! Routing for `ESC ]` delegates directly to
//! [`terminal_events::try_disambiguate_osc_or_alt_bracket()`], which uses a dedicated
//! state machine implementing Rule 1 (the [`MaybeMore`] stream availability heuristic)
//! and Rule 2 (strict [`OSC`] syntax validation) to safely distinguish between user
//! keystrokes and terminal query responses.
//!
//! ### 4. [`ESC`] + Other Byte
//!
//! When the buffer begins with [`ESC`] + (a byte other than `[`, `O`, or `]`):
//! - **`parse_alt_letter()`** ([`keyboard`]) - Alt+printable character combinations
//!   (e.g., `ESC b` for Alt+B, `ESC 3` for Alt+3). If unrecognized, emits standalone
//!   [`ESC`] and leaves the trailing byte in the buffer for the next parse cycle.
//!
//! ### 5. Non-[`ESC`] Sequences (Regular Input)
//!
//! When the first byte is not [`ESC`]:
//! 1. **`parse_control_character()`** ([`keyboard`]) - Ctrl+A through Ctrl+Z
//!    (`0x00`-`0x1F`, e.g., `0x01` for Ctrl+A, `0x04` for Ctrl+D).
//!    - **Must be tried before [`UTF-8`]**: Bytes `0x00`-`0x1F` are technically valid
//!      single-byte [`UTF-8`] but represent control characters. Without this priority,
//!      Ctrl+A would be misinterpreted as raw text.
//! 2. **`parse_utf8_text()`** ([`utf8`]) - Regular text input and printable multi-byte
//!    characters (e.g., `a`, `ñ`, `日`).
//!
//! [`CSI`]: crate::CsiSequence
//! [`ESC`]: crate::EscSequence
//! [`keyboard`]: mod@super::keyboard
//! [`MaybeMore`]: MaybeMore
//! [`mouse`]: mod@super::mouse
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`SGR`]: crate::SgrCode
//! [`terminal_events::try_disambiguate_osc_or_alt_bracket()`]: terminal_events::try_disambiguate_osc_or_alt_bracket
//! [`terminal_events`]: mod@super::terminal_events
//! [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
//! [`utf8`]: mod@super::utf8
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html

use super::{MaybeMore, VT100InputEventIR, VT100KeyCodeIR, VT100KeyModifiersIR, keyboard,
            mouse, terminal_events, utf8};
use crate::{ByteOffset, byte_offset,
            core::ansi::constants::{ANSI_CSI_BRACKET, ANSI_ESC, ANSI_OSC_CLOSE_BRACKET,
                                    ANSI_SS3_O}};

/// Parses a complete input event from accumulated input bytes.
///
/// This is the main entry point for [`VT-100`] terminal input parsing. It analyzes the
/// accumulated sequence bytes and routes to specialized parsers ([`keyboard`], [`mouse`],
/// [`terminal_events`], [`utf8`]) based on content analysis.
///
/// # Arguments
///
/// - `accumulated_bytes`: The slice of accumulated unparsed bytes to evaluate.
/// - `maybe_more`: Availability heuristic of subsequent bytes:
///   - When `maybe_more == MaybeMore::KernelMayHaveMore` and `accumulated_bytes` is
///     `[ESC]`: Return `None` (wait for subsequent bytes of the multi-byte sequence).
///   - When `maybe_more == MaybeMore::KernelDrained` and `accumulated_bytes` is `[ESC]`:
///     Emit [`ESC`] key immediately (0ms latency).
///   - For complete multi-byte sequences (such as [`CSI`] or [`SS3`]): This parameter has
///     no effect.
///
/// # Where You Are in the Pipeline
///
/// For the full data flow, see the [parent module documentation]. This diagram shows
/// where this function fits:
///
/// ```text
/// mio_poller thread (reads from stdin into read_buffer)
///    │
///    │ InputByteStreamToIrParser::advance(read_buffer, maybe_more)
///    ▼
/// ┌──────────────────────────────────────────┐  ┌──────────────────┐
/// │  try_parse_input_event()                 ◄──┤ YOU ARE HERE     │
/// │  • Smart routing & ESC detection         │  └──────────────────┘
/// │  • Zero-latency ESC key handling         │
/// └──────────────────────────────────────────┘
///    │ (routes to specialized parsers)
///    ├─► keyboard.rs (CSI/SS3 keyboard sequences)
///    ├─► mouse.rs (mouse protocols)
///    ├─► terminal_events.rs (resize/focus/paste)
///    ├─► utf8.rs (text input)
///    │
///    ▼
/// VT100InputEventIR
///    │
///    ▼
/// convert_input_event() -> InputEvent (returned to application)
/// ```
///
/// **Navigate**:
/// - ⬆️ **Up**: [`DirectToAnsiInputDevice`] - Async I/O layer that drives this pipeline
/// - ⬇️ **Down**: [`keyboard`], [`mouse`], [`terminal_events`], [`utf8`] - Specialized
///   parsers
/// - 📚 **Types**: [`VT100InputEventIR`] - Output event type
///
/// # [`ESC`] Key Detection & Disambiguation
///
/// ## The Ambiguity
///
/// Both a standalone [`ESC`] keypress and every multi-byte [`ANSI`] escape sequence (such
/// as Up Arrow `\x1b[A` or OSC color queries `\x1b]11;...`) begin with the identical byte
/// [`ANSI_ESC`] (`0x1B`). When `accumulated_bytes` contains only `[0x1B]`, the router
/// must decide: is this a standalone [`ESC`] key, or the prefix of an in-flight escape
/// sequence?
///
/// ## The [`MaybeMore`] Stream Availability Heuristic
///
/// Instead of using a fixed 50-150ms timeout (which introduces sluggish latency for modal
/// editors like Vim), the parser inspects stream availability through [`MaybeMore`]. See
/// the [`MaybeMore`] documentation for the full two-level heuristic model and pipeline
/// architecture.
///
/// ## How Disambiguation Works
///
/// When `accumulated_bytes` is `[0x1B]`:
///
/// 1. **Part of a Multi-Byte Sequence (Burst Arrival)**: Terminal emulators emit
///    multi-byte escape sequences in a single write burst (e.g., `\x1b[A`). While
///    processing the first byte (`0x1B`), subsequent bytes are already in-flight or
///    waiting in user space.
///    - `maybe_more == MaybeMore::KernelMayHaveMore`: The router returns `None` so the
///      parser loop continues accumulating the rest of the sequence.
///
/// 2. **Standalone [`ESC`] Key (Human Keystroke)**: A human pressing the physical [`ESC`]
///    key generates a single `0x1B` byte.
///    - `maybe_more == MaybeMore::KernelDrained`: The router immediately returns
///      `Some(VT100KeyCodeIR::Escape)` with **0ms latency**.
///
/// # Smart Lookahead Logic
///
/// The router inspects prefix bytes in `accumulated_bytes` to determine routing:
///
/// | Input Pattern        | `maybe_more`              | Routing                               |
/// | :------------------- | :------------------------ | :------------------------------------ |
/// | `[ 0x1B ]` alone     | [`KernelDrained`]         | Emit [`ESC`] key immediately (0ms)    |
/// | `[ 0x1B ]` alone     | [`KernelMayHaveMore`]     | Return `None` (wait for more bytes)   |
/// | `[ 0x1B, b'[', .. ]` | (ignored)                 | [`CSI`] -> keyboard/mouse/terminal    |
/// | `[ 0x1B, b'O', .. ]` | (ignored)                 | [`SS3`] -> F1-F4, Home, End, arrows   |
/// | `[ 0x1B, other ]`    | (ignored)                 | Alt+letter or emit standalone [`ESC`] |
/// | Other bytes          | (ignored)                 | control char -> [`UTF-8`]             |
///
/// - [`CSI`] (Control Sequence Introducer):
///   - The most common escape sequence format, starting with `ESC [`. Used for arrow
///     keys, function keys, mouse events, and terminal queries.
///   - Example: `ESC [ A` is Up arrow, `ESC [ 1 ; 5 C` is Ctrl+Right.
/// - [`SS3`] (Single Shift 3) / Application mode:
///   - Terminals can switch between "normal" and "application" mode. Programs like vim,
///     less, and emacs enable this mode.
///   - In application mode, arrow keys and F1-F4 send `ESC O x` ([`SS3`]) instead of `ESC
///     [ x` ([`CSI`]).
/// - Alt+letter fallback:
///   - Terminals historically couldn't send a dedicated Alt modifier, so they send
///     [`ESC`] followed by the letter (e.g., `ESC b` for Alt+B).
///   - When we see [`ESC`] + unknown byte, we first try to parse it as Alt+letter. If
///     that fails, we emit a standalone [`ESC`] and leave the next byte for the next
///     parse cycle.
///
/// # Routing Algorithm
///
/// ```text
/// try_parse_input_event(accumulated_bytes, maybe_more):
/// ┌────────────────────────────────────────────────────────┐
/// │ First byte check                                       │
/// ├────────────────────────────────────────────────────────┤
/// │ 0x1B (ESC)?                                            │
/// │  ├─ accumulated_bytes.len() == 1?                      │
/// │  │  ├─ maybe_more == KernelMayHaveMore?                │
/// │  │  │  └─ Return None (wait for more bytes)            │
/// │  │  └─ else: emit ESC key immediately (0ms)            │
/// │  └─ accumulated_bytes.len() >= 2?                      │
/// │     ├─ Second byte = b'['?                             │
/// │     │  └─ CSI -> keyboard/mouse/terminal_events        │
/// │     ├─ Second byte = b'O'?                             │
/// │     │  └─ SS3 -> application mode keys                 │
/// │     └─ Second byte = other?                            │
/// │        └─ Alt+letter or emit ESC                       │
/// ├────────────────────────────────────────────────────────┤
/// │ Not ESC?                                               │
/// │  └─ Raw byte: control_char -> UTF-8                    │
/// └────────────────────────────────────────────────────────┘
/// ```
///
/// # Returns
///
/// - The parsed [`VT100InputEventIR`] and [`ByteOffset`] byte count on success.
/// - Nothing if `accumulated_bytes` contains an incomplete sequence (more bytes needed),
///   or if `maybe_more == MaybeMore::KernelMayHaveMore` and `accumulated_bytes` is
///   `[ESC]` (waiting for a potential escape sequence).
///
/// [`ANSI_ESC`]: crate::ANSI_ESC
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`ByteOffset`]: crate::ByteOffset
/// [`CSI`]: crate::CsiSequence
/// [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
/// [`ESC`]: crate::EscSequence
/// [`KernelDrained`]: super::MaybeMore::KernelDrained
/// [`KernelMayHaveMore`]: super::MaybeMore::KernelMayHaveMore
/// [`keyboard`]: mod@super::keyboard
/// [`MaybeMore::KernelDrained`]: super::MaybeMore::KernelDrained
/// [`MaybeMore::KernelMayHaveMore`]: super::MaybeMore::KernelMayHaveMore
/// [`MaybeMore`]: super::MaybeMore
/// [`mouse`]: mod@super::mouse
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`SS3`]: https://vt100.net/docs/vt510-rm/SS.html
/// [`stdin`]: std::io::stdin
/// [`terminal_events`]: mod@super::terminal_events
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
/// [`utf8`]: mod@super::utf8
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [`VT100InputEventIR`]: super::VT100InputEventIR
/// [parent module documentation]: mod@super#primary-consumer
#[must_use]
pub fn try_parse_input_event(
    accumulated_bytes: &[u8],
    maybe_more: MaybeMore,
) -> Option<(VT100InputEventIR, ByteOffset)> {
    // Routing table.
    match accumulated_bytes {
        // Empty buffer.
        [] => None,

        // Single ESC byte - check maybe_more heuristic.
        // - MaybeMore::KernelMayHaveMore: Wait for more bytes (might be escape sequence).
        // - MaybeMore::KernelDrained: Emit ESC key immediately (no more input).
        [ANSI_ESC] => match maybe_more {
            MaybeMore::KernelMayHaveMore => None,
            MaybeMore::KernelDrained => Some((esc_key_event(), byte_offset(1))),
        },

        // CSI sequence (ESC [) - keyboard/mouse/terminal events.
        [ANSI_ESC, ANSI_CSI_BRACKET, ..] => {
            keyboard::parse_keyboard_sequence(accumulated_bytes)
                .or_else(|| mouse::parse_mouse_sequence(accumulated_bytes))
                .or_else(|| terminal_events::parse_terminal_event(accumulated_bytes))
        }

        // SS3 sequence (ESC O) - application mode keys (F1-F4, Home, End, arrows).
        [ANSI_ESC, ANSI_SS3_O, ..] => keyboard::parse_ss3_sequence(accumulated_bytes),

        // OSC sequence or Alt+] keypress disambiguation.
        [ANSI_ESC, ANSI_OSC_CLOSE_BRACKET, ..] => {
            terminal_events::try_disambiguate_osc_or_alt_bracket(
                accumulated_bytes,
                maybe_more,
            )
        }

        // ESC + other byte - try Alt+letter (e.g., Alt+B, Alt+F), else emit standalone
        // ESC.
        [ANSI_ESC, _, ..] => keyboard::parse_alt_letter(accumulated_bytes)
            .or_else(|| Some((esc_key_event(), byte_offset(1)))),

        // Not ESC - raw byte input (control characters or UTF-8 text).
        // Control characters (0x00-0x1F) must be tried before UTF-8 because they are
        // technically valid UTF-8 but should be parsed as Ctrl+letter instead.
        _ => keyboard::parse_control_character(accumulated_bytes)
            .or_else(|| utf8::parse_utf8_text(accumulated_bytes)),
    }
}

/// Helper to create an [`ESC`] key event.
///
/// [`ESC`]: crate::EscSequence
fn esc_key_event() -> VT100InputEventIR {
    VT100InputEventIR::Keyboard {
        code: VT100KeyCodeIR::Escape,
        modifiers: VT100KeyModifiersIR::default(),
    }
}

/// Tests for [`CSI`]/SS3 sequence routing using generators for round-trip validation.
///
/// [`CSI`]: crate::CsiSequence
#[cfg(test)]
mod tests_csi_routing {
    use super::*;
    use crate::{TermPos,
                core::ansi::{generator::generate_keyboard_sequence,
                             vt_100_terminal_input_parser::{VT100FocusStateIR,
                                                            VT100MouseActionIR,
                                                            VT100MouseButtonIR,
                                                            VT100PasteModeIR}}};

    #[test]
    fn keyboard_arrow_key() {
        let expected = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Up,
            modifiers: VT100KeyModifiersIR::default(),
        };
        let buffer = generate_keyboard_sequence(&expected).expect("conversion error");
        let (event, consumed) = try_parse_input_event(&buffer, MaybeMore::KernelDrained)
            .expect("Should parse Up Arrow");

        assert_eq!(event, expected);
        assert_eq!(consumed.as_usize(), buffer.len());
    }

    #[test]
    fn raw_csi_arrow_key() {
        let buffer = &[0x1B, b'[', b'A'];
        let (event, consumed) = try_parse_input_event(buffer, MaybeMore::KernelDrained)
            .expect("Should parse Up Arrow");

        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Up,
                modifiers: VT100KeyModifiersIR::default(),
            }
        );
        assert_eq!(consumed, byte_offset(3));
    }

    #[test]
    fn mouse_sgr_format() {
        let expected = VT100InputEventIR::Mouse {
            button: VT100MouseButtonIR::Left,
            pos: TermPos::from_one_based(10, 20),
            action: VT100MouseActionIR::Press,
            modifiers: VT100KeyModifiersIR::default(),
        };
        let buffer = generate_keyboard_sequence(&expected).expect("conversion error");
        let (event, consumed) = try_parse_input_event(&buffer, MaybeMore::KernelDrained)
            .expect("Should parse mouse event");

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
        let (event, consumed) = try_parse_input_event(buffer, MaybeMore::KernelDrained)
            .expect("Should parse F1");

        assert_eq!(event, expected);
        assert_eq!(consumed, byte_offset(3));
    }

    #[test]
    fn terminal_event_focus() {
        // Focus gained.
        let focus_gained = VT100InputEventIR::Focus(VT100FocusStateIR::Gained);
        let buffer = generate_keyboard_sequence(&focus_gained).expect("conversion error");
        let (event, consumed) = try_parse_input_event(&buffer, MaybeMore::KernelDrained)
            .expect("Should parse focus gained");

        assert_eq!(event, focus_gained);
        assert_eq!(consumed.as_usize(), buffer.len());

        // Focus lost.
        let focus_lost = VT100InputEventIR::Focus(VT100FocusStateIR::Lost);
        let buffer = generate_keyboard_sequence(&focus_lost).expect("conversion error");
        let (event, consumed) = try_parse_input_event(&buffer, MaybeMore::KernelDrained)
            .expect("Should parse focus lost");

        assert_eq!(event, focus_lost);
        assert_eq!(consumed.as_usize(), buffer.len());
    }

    #[test]
    fn terminal_event_paste() {
        // Bracketed paste start.
        let paste_start = VT100InputEventIR::Paste(VT100PasteModeIR::Start);
        let buffer = generate_keyboard_sequence(&paste_start).expect("conversion error");
        let (event, consumed) = try_parse_input_event(&buffer, MaybeMore::KernelDrained)
            .expect("Should parse paste start");

        assert_eq!(event, paste_start);
        assert_eq!(consumed.as_usize(), buffer.len());

        // Bracketed paste end.
        let paste_end = VT100InputEventIR::Paste(VT100PasteModeIR::End);
        let buffer = generate_keyboard_sequence(&paste_end).expect("conversion error");
        let (event, consumed) = try_parse_input_event(&buffer, MaybeMore::KernelDrained)
            .expect("Should parse paste end");

        assert_eq!(event, paste_end);
        assert_eq!(consumed.as_usize(), buffer.len());
    }
}

/// Tests for non-[`CSI`] input: single bytes and [`ESC`]+byte sequences.
/// Validates parsing of [`ESC`] key, Alt+letter, control characters, and [`UTF-8`] text.
///
/// [`CSI`]: crate::CsiSequence
/// [`ESC`]: crate::EscSequence
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
#[cfg(test)]
mod tests_non_csi_input {
    use super::*;
    use crate::{KeyState, core::ansi::generator::generate_keyboard_sequence};

    #[test]
    fn esc_key_immediate_when_no_more_input() {
        // Single ESC byte emits immediately when stream is drained.
        let expected = VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Escape,
            modifiers: VT100KeyModifiersIR::default(),
        };
        let buffer = generate_keyboard_sequence(&expected).expect("conversion error");
        let (event, consumed) = try_parse_input_event(&buffer, MaybeMore::KernelDrained)
            .expect("Should parse ESC key");

        assert_eq!(event, expected);
        assert_eq!(consumed.as_usize(), buffer.len());
    }

    #[test]
    fn esc_key_waits_when_more_input_available() {
        // Single ESC byte returns None when more input is anticipated.
        let buffer = &[0x1B]; // ESC
        assert!(
            try_parse_input_event(buffer, MaybeMore::KernelMayHaveMore).is_none(),
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
        let buffer = generate_keyboard_sequence(&expected).expect("conversion error");
        let (event, consumed) = try_parse_input_event(&buffer, MaybeMore::KernelDrained)
            .expect("Should parse Alt+b");

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
        let buffer = generate_keyboard_sequence(&expected).expect("conversion error");
        let (event, consumed) = try_parse_input_event(&buffer, MaybeMore::KernelDrained)
            .expect("Should parse Ctrl+A");

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
        let buffer = generate_keyboard_sequence(&expected).expect("conversion error");
        let (event, consumed) = try_parse_input_event(&buffer, MaybeMore::KernelDrained)
            .expect("Should parse 'H'");

        assert_eq!(event, expected);
        assert_eq!(consumed.as_usize(), buffer.len());
    }

    #[test]
    fn utf8_text_in_longer_buffer() {
        let buffer = b"Hello";
        let (event, consumed) = try_parse_input_event(buffer, MaybeMore::KernelDrained)
            .expect("Should parse 'H'");

        assert_eq!(
            event,
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char('H'),
                modifiers: VT100KeyModifiersIR::default(),
            }
        );
        assert_eq!(consumed, byte_offset(1));
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
        assert!(try_parse_input_event(buffer, MaybeMore::KernelDrained).is_none());
    }

    #[test]
    fn incomplete_csi_sequence_returns_none() {
        // ESC [ without final byte - waiting for more input.
        let buffer = &[0x1B, b'['];
        assert!(try_parse_input_event(buffer, MaybeMore::KernelDrained).is_none());
    }

    #[test]
    fn unknown_esc_emits_standalone_esc() {
        // ESC + invalid byte -> emit standalone ESC, leave invalid byte for next cycle.
        let buffer = &[0x1B, 0xFF];
        let (event, consumed) = try_parse_input_event(buffer, MaybeMore::KernelDrained)
            .expect("Should emit standalone ESC");

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

/// Tests for [`OSC`] sequence and Alt+] disambiguation routing.
///
/// [`OSC`]: crate::osc_codes::OscSequence
#[cfg(test)]
mod tests_osc_routing {
    use super::*;
    use crate::{KeyState, MAX_OSC_SEQUENCE_LENGTH};

    fn alt_bracket_expected() -> VT100InputEventIR {
        VT100InputEventIR::Keyboard {
            code: VT100KeyCodeIR::Char(']'),
            modifiers: VT100KeyModifiersIR {
                shift: KeyState::NotPressed,
                ctrl: KeyState::NotPressed,
                alt: KeyState::Pressed,
            },
        }
    }

    #[test]
    fn lone_alt_bracket_drained() {
        let (event, consumed) = try_parse_input_event(b"\x1b]", MaybeMore::KernelDrained)
            .expect("Should emit Alt+]");
        assert_eq!(event, alt_bracket_expected());
        assert_eq!(consumed, byte_offset(2));
    }

    #[test]
    fn lone_alt_bracket_more_anticipated() {
        assert!(try_parse_input_event(b"\x1b]", MaybeMore::KernelMayHaveMore).is_none());
    }

    #[test]
    fn alt_bracket_followed_by_non_digit() {
        let (event, consumed) =
            try_parse_input_event(b"\x1b]a", MaybeMore::KernelMayHaveMore)
                .expect("Should emit Alt+]");
        assert_eq!(event, alt_bracket_expected());
        assert_eq!(consumed, byte_offset(2));
    }

    #[test]
    fn alt_bracket_followed_by_digit_drained() {
        let (event, consumed) =
            try_parse_input_event(b"\x1b]5", MaybeMore::KernelDrained)
                .expect("Should emit Alt+] when drained");
        assert_eq!(event, alt_bracket_expected());
        assert_eq!(consumed, byte_offset(2));
    }

    #[test]
    fn alt_bracket_followed_by_digit_more_anticipated() {
        assert!(try_parse_input_event(b"\x1b]5", MaybeMore::KernelMayHaveMore).is_none());
    }

    #[test]
    fn candidate_osc_complete() {
        let seq = b"\x1b]0;my title\x07";
        let (event, consumed) = try_parse_input_event(seq, MaybeMore::KernelDrained)
            .expect("Should consume OSC sequence");
        assert_eq!(event, VT100InputEventIR::Ignored);
        assert_eq!(consumed, byte_offset(seq.len()));
    }

    #[test]
    fn candidate_osc_in_flight_payload_drained() {
        // Delimiter was parsed; in-flight payload must wait even if drained.
        assert!(
            try_parse_input_event(b"\x1b]0;my title", MaybeMore::KernelDrained).is_none()
        );
    }

    #[test]
    fn candidate_osc_invalid_syntax() {
        // Embedded newline in payload violates OSC syntax -> emits Alt+]
        let (event, consumed) =
            try_parse_input_event(b"\x1b]0;line\nbreak\x07", MaybeMore::KernelDrained)
                .expect("Should emit Alt+] on invalid syntax");
        assert_eq!(event, alt_bracket_expected());
        assert_eq!(consumed, byte_offset(2));
    }

    #[test]
    fn candidate_osc_runaway() {
        let mut runaway = Vec::with_capacity(MAX_OSC_SEQUENCE_LENGTH + 10);
        runaway.extend_from_slice(b"\x1b]52;");
        runaway.resize(MAX_OSC_SEQUENCE_LENGTH + 1, b'a');
        assert!(try_parse_input_event(&runaway, MaybeMore::KernelDrained).is_none());
    }
}
