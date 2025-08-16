// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Terminal control characters and conversion functions.

use std::borrow::Cow;

/// Control characters and special keys that can be sent to PTY.
///
/// # Summary
/// - Terminal control sequence API for keyboard input emulation in PTY sessions
/// - Character types: Control keys (Ctrl-C/D/Z), navigation (arrows, Home/End), editing
///   (Tab, Backspace), function keys (F1-F12), raw escape sequences
/// - Converts to ANSI escape sequences via [`control_char_to_bytes`] for terminal
///   compatibility
/// - Used with [`super::pty_events::PtyInputEvent::SendControl`] to send keyboard
///   commands to child processes
/// - Supports both standard terminal operations and custom escape sequences
#[derive(Debug, Clone)]
pub enum ControlChar {
    // Common control characters
    CtrlC, // SIGINT (interrupt)
    CtrlD, // EOF (end of file)
    CtrlZ, // SIGTSTP (suspend)
    CtrlL, // Clear screen
    CtrlU, // Clear line
    CtrlA, // Move to beginning of line
    CtrlE, // Move to end of line
    CtrlK, // Kill to end of line

    // Common keys
    Tab,    // Autocomplete
    Enter,  // Newline
    Escape, // ESC key
    Backspace,
    Delete,

    // Arrow keys
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,

    // Navigation keys
    Home,
    End,
    PageUp,
    PageDown,

    // Function keys (F1-F12)
    F(u8), // F(1) for F1, F(2) for F2, etc.

    // Raw escape sequence for advanced use cases
    RawSequence(Vec<u8>),
}

/// Converts a control character to its byte representation.
///
/// Returns a `Cow` to avoid unnecessary allocations for static sequences.
#[must_use]
pub fn control_char_to_bytes(ctrl: &ControlChar) -> Cow<'static, [u8]> {
    match ctrl {
        // Control characters
        ControlChar::CtrlC => Cow::Borrowed(&[0x03]),
        ControlChar::CtrlD => Cow::Borrowed(&[0x04]),
        ControlChar::CtrlZ => Cow::Borrowed(&[0x1A]),
        ControlChar::CtrlL => Cow::Borrowed(&[0x0C]),
        ControlChar::CtrlU => Cow::Borrowed(&[0x15]),
        ControlChar::CtrlA => Cow::Borrowed(&[0x01]),
        ControlChar::CtrlE => Cow::Borrowed(&[0x05]),
        ControlChar::CtrlK => Cow::Borrowed(&[0x0B]),

        // Common keys
        ControlChar::Tab => Cow::Borrowed(&[0x09]),
        ControlChar::Enter => Cow::Borrowed(&[0x0A]),
        ControlChar::Escape => Cow::Borrowed(&[0x1B]),
        ControlChar::Backspace => Cow::Borrowed(&[0x7F]),
        ControlChar::Delete => Cow::Borrowed(&[0x1B, 0x5B, 0x33, 0x7E]), // ESC[3~

        // Arrow keys (ANSI escape sequences)
        ControlChar::ArrowUp => Cow::Borrowed(&[0x1B, 0x5B, 0x41]), // ESC[A
        ControlChar::ArrowDown => Cow::Borrowed(&[0x1B, 0x5B, 0x42]), // ESC[B
        ControlChar::ArrowRight => Cow::Borrowed(&[0x1B, 0x5B, 0x43]), // ESC[C
        ControlChar::ArrowLeft => Cow::Borrowed(&[0x1B, 0x5B, 0x44]), // ESC[D

        // Navigation keys
        ControlChar::Home => Cow::Borrowed(&[0x1B, 0x5B, 0x48]), // ESC[H
        ControlChar::End => Cow::Borrowed(&[0x1B, 0x5B, 0x46]),  // ESC[F
        ControlChar::PageUp => Cow::Borrowed(&[0x1B, 0x5B, 0x35, 0x7E]), // ESC[5~
        ControlChar::PageDown => Cow::Borrowed(&[0x1B, 0x5B, 0x36, 0x7E]), // ESC[6~

        // Function keys
        ControlChar::F(n) => {
            match n {
                // cspell:disable
                1 => Cow::Borrowed(&[0x1B, 0x4F, 0x50]), // ESCOP
                2 => Cow::Borrowed(&[0x1B, 0x4F, 0x51]), // ESCOQ
                3 => Cow::Borrowed(&[0x1B, 0x4F, 0x52]), // ESCOR
                4 => Cow::Borrowed(&[0x1B, 0x4F, 0x53]), // ESCOS
                // cspell:enable
                5 => Cow::Borrowed(&[0x1B, 0x5B, 0x31, 0x35, 0x7E]), // ESC[15~
                6 => Cow::Borrowed(&[0x1B, 0x5B, 0x31, 0x37, 0x7E]), // ESC[17~
                7 => Cow::Borrowed(&[0x1B, 0x5B, 0x31, 0x38, 0x7E]), // ESC[18~
                8 => Cow::Borrowed(&[0x1B, 0x5B, 0x31, 0x39, 0x7E]), // ESC[19~
                9 => Cow::Borrowed(&[0x1B, 0x5B, 0x32, 0x30, 0x7E]), // ESC[20~
                10 => Cow::Borrowed(&[0x1B, 0x5B, 0x32, 0x31, 0x7E]), // ESC[21~
                11 => Cow::Borrowed(&[0x1B, 0x5B, 0x32, 0x33, 0x7E]), // ESC[23~
                12 => Cow::Borrowed(&[0x1B, 0x5B, 0x32, 0x34, 0x7E]), // ESC[24~
                // Unknown function keys
                _ => Cow::Borrowed(&[0x1B]), // Just ESC
            }
        }

        // Raw sequence - pass through as-is (requires owned data)
        ControlChar::RawSequence(bytes) => Cow::Owned(bytes.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_char_debug_and_clone() {
        let ctrl = ControlChar::CtrlC;
        let cloned = ctrl.clone();
        assert_eq!(format!("{ctrl:?}"), format!("{cloned:?}"));
    }

    #[test]
    fn test_control_char_to_bytes_basic() {
        assert_eq!(*control_char_to_bytes(&ControlChar::CtrlC), [0x03]);
        assert_eq!(*control_char_to_bytes(&ControlChar::CtrlD), [0x04]);
        assert_eq!(*control_char_to_bytes(&ControlChar::CtrlZ), [0x1A]);
        assert_eq!(*control_char_to_bytes(&ControlChar::CtrlL), [0x0C]);
        assert_eq!(*control_char_to_bytes(&ControlChar::CtrlU), [0x15]);
        assert_eq!(*control_char_to_bytes(&ControlChar::CtrlA), [0x01]);
        assert_eq!(*control_char_to_bytes(&ControlChar::CtrlE), [0x05]);
        assert_eq!(*control_char_to_bytes(&ControlChar::CtrlK), [0x0B]);
    }

    #[test]
    fn test_control_char_to_bytes_common_keys() {
        assert_eq!(*control_char_to_bytes(&ControlChar::Tab), [0x09]);
        assert_eq!(*control_char_to_bytes(&ControlChar::Enter), [0x0A]);
        assert_eq!(*control_char_to_bytes(&ControlChar::Escape), [0x1B]);
        assert_eq!(*control_char_to_bytes(&ControlChar::Backspace), [0x7F]);
        assert_eq!(
            *control_char_to_bytes(&ControlChar::Delete),
            [0x1B, 0x5B, 0x33, 0x7E]
        );
    }

    #[test]
    fn test_control_char_to_bytes_arrow_keys() {
        assert_eq!(
            *control_char_to_bytes(&ControlChar::ArrowUp),
            [0x1B, 0x5B, 0x41]
        );
        assert_eq!(
            *control_char_to_bytes(&ControlChar::ArrowDown),
            [0x1B, 0x5B, 0x42]
        );
        assert_eq!(
            *control_char_to_bytes(&ControlChar::ArrowRight),
            [0x1B, 0x5B, 0x43]
        );
        assert_eq!(
            *control_char_to_bytes(&ControlChar::ArrowLeft),
            [0x1B, 0x5B, 0x44]
        );
    }

    #[test]
    fn test_control_char_to_bytes_navigation() {
        assert_eq!(
            *control_char_to_bytes(&ControlChar::Home),
            [0x1B, 0x5B, 0x48]
        );
        assert_eq!(
            *control_char_to_bytes(&ControlChar::End),
            [0x1B, 0x5B, 0x46]
        );
        assert_eq!(
            *control_char_to_bytes(&ControlChar::PageUp),
            [0x1B, 0x5B, 0x35, 0x7E]
        );
        assert_eq!(
            *control_char_to_bytes(&ControlChar::PageDown),
            [0x1B, 0x5B, 0x36, 0x7E]
        );
    }

    #[test]
    fn test_control_char_to_bytes_function_keys() {
        // Test F1-F4 (special sequences)
        assert_eq!(
            *control_char_to_bytes(&ControlChar::F(1)),
            [0x1B, 0x4F, 0x50]
        );
        assert_eq!(
            *control_char_to_bytes(&ControlChar::F(2)),
            [0x1B, 0x4F, 0x51]
        );
        assert_eq!(
            *control_char_to_bytes(&ControlChar::F(3)),
            [0x1B, 0x4F, 0x52]
        );
        assert_eq!(
            *control_char_to_bytes(&ControlChar::F(4)),
            [0x1B, 0x4F, 0x53]
        );

        // Test F5-F12 (ESC[nn~ sequences)
        assert_eq!(
            *control_char_to_bytes(&ControlChar::F(5)),
            [0x1B, 0x5B, 0x31, 0x35, 0x7E]
        );
        assert_eq!(
            *control_char_to_bytes(&ControlChar::F(12)),
            [0x1B, 0x5B, 0x32, 0x34, 0x7E]
        );

        // Test unknown function key
        assert_eq!(*control_char_to_bytes(&ControlChar::F(99)), [0x1B]);
    }

    #[test]
    fn test_control_char_to_bytes_raw_sequence() {
        let custom_bytes = vec![0x1B, 0x5B, 0x32, 0x4A]; // Clear screen from cursor
        let ctrl = ControlChar::RawSequence(custom_bytes.clone());
        assert_eq!(*control_char_to_bytes(&ctrl), *custom_bytes);
    }
}
