// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{OscEvent, PtyControlledChildExitStatus};
use std::borrow::Cow;

/// Events received from a [`PTY`] process.
///
/// This is a unified event type used by both read-only and read-write sessions.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[derive(Debug, Clone)]
pub enum PtyOutputEvent {
    /// Raw output from the child process.
    Output(Vec<u8>),

    /// [`OSC`] (Operating System Command) sequences.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    Osc(OscEvent),

    /// Child process exited normally.
    Exit(PtyControlledChildExitStatus),

    /// Child process crashed or terminated unexpectedly.
    UnexpectedExit(String),

    /// Write operation failed - session will terminate.
    ///
    /// This gives users a chance to understand why the session ended.
    WriteError(String),
}

/// Cursor key mode for terminal compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorKeyMode {
    /// Normal mode ([`ANSI`][ - `ESC`][ sequences
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    Normal,
    /// Application mode (VT52) - `ESC O` sequences
    #[default]
    Application,
}

/// Terminal control sequences with mode-aware generation.
#[derive(Debug, Clone)]
pub enum ControlSequence {
    // Common control characters.
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

    // Arrow keys (mode-aware)
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

    // Raw escape sequence for advanced use cases.
    RawSequence(Vec<u8>),
}

impl ControlSequence {
    /// Converts a control sequence to its byte representation based on cursor mode.
    ///
    /// Returns a `Cow` to avoid unnecessary allocations for static sequences.
    #[must_use]
    pub fn to_bytes(&self, mode: CursorKeyMode) -> Cow<'static, [u8]> {
        match self {
            // Control characters (mode-independent)
            ControlSequence::CtrlC => Cow::Borrowed(&[0x03]),
            ControlSequence::CtrlD => Cow::Borrowed(&[0x04]),
            ControlSequence::CtrlZ => Cow::Borrowed(&[0x1A]),
            ControlSequence::CtrlL => Cow::Borrowed(&[0x0C]),
            ControlSequence::CtrlU => Cow::Borrowed(&[0x15]),
            ControlSequence::CtrlA => Cow::Borrowed(&[0x01]),
            ControlSequence::CtrlE => Cow::Borrowed(&[0x05]),
            ControlSequence::CtrlK => Cow::Borrowed(&[0x0B]),

            // Common keys (mode-independent)
            ControlSequence::Tab => Cow::Borrowed(&[0x09]),
            ControlSequence::Enter => Cow::Borrowed(&[0x0D]), // CR, not LF
            ControlSequence::Escape => Cow::Borrowed(&[0x1B]),
            ControlSequence::Backspace => Cow::Borrowed(&[0x7F]),
            ControlSequence::Delete => Cow::Borrowed(&[0x1B, 0x5B, 0x33, 0x7E]), /* ESC[3~ */

            // Arrow keys (mode-aware)
            ControlSequence::ArrowUp => match mode {
                CursorKeyMode::Normal => Cow::Borrowed(&[0x1B, 0x5B, 0x41]), // ESC[A
                CursorKeyMode::Application => Cow::Borrowed(&[0x1B, 0x4F, 0x41]), /* ESC O A */
            },
            ControlSequence::ArrowDown => match mode {
                CursorKeyMode::Normal => Cow::Borrowed(&[0x1B, 0x5B, 0x42]), // ESC[B
                CursorKeyMode::Application => Cow::Borrowed(&[0x1B, 0x4F, 0x42]), /* ESC O B */
            },
            ControlSequence::ArrowRight => match mode {
                CursorKeyMode::Normal => Cow::Borrowed(&[0x1B, 0x5B, 0x43]), // ESC[C
                CursorKeyMode::Application => Cow::Borrowed(&[0x1B, 0x4F, 0x43]), /* ESC O C */
            },
            ControlSequence::ArrowLeft => match mode {
                CursorKeyMode::Normal => Cow::Borrowed(&[0x1B, 0x5B, 0x44]), // ESC[D
                CursorKeyMode::Application => Cow::Borrowed(&[0x1B, 0x4F, 0x44]), /* ESC O D */
            },

            // Navigation keys (mode-independent)
            ControlSequence::Home => Cow::Borrowed(&[0x1B, 0x5B, 0x48]), // ESC[H
            ControlSequence::End => Cow::Borrowed(&[0x1B, 0x5B, 0x46]),  // ESC[F
            ControlSequence::PageUp => Cow::Borrowed(&[0x1B, 0x5B, 0x35, 0x7E]), // ESC[5~
            ControlSequence::PageDown => Cow::Borrowed(&[0x1B, 0x5B, 0x36, 0x7E]), /* ESC[6~ */

            // Function keys (mode-independent)
            ControlSequence::F(n) => {
                match n {
                    1 => Cow::Borrowed(&[0x1B, 0x4F, 0x50]), // ESC O P
                    2 => Cow::Borrowed(&[0x1B, 0x4F, 0x51]), // ESC O Q
                    3 => Cow::Borrowed(&[0x1B, 0x4F, 0x52]), // ESC O R
                    4 => Cow::Borrowed(&[0x1B, 0x4F, 0x53]), // ESC O S
                    5 => Cow::Borrowed(&[0x1B, 0x5B, 0x31, 0x35, 0x7E]), // ESC[15~
                    6 => Cow::Borrowed(&[0x1B, 0x5B, 0x31, 0x37, 0x7E]), // ESC[17~
                    7 => Cow::Borrowed(&[0x1B, 0x5B, 0x31, 0x38, 0x7E]), // ESC[18~
                    8 => Cow::Borrowed(&[0x1B, 0x5B, 0x31, 0x39, 0x7E]), // ESC[19~
                    9 => Cow::Borrowed(&[0x1B, 0x5B, 0x32, 0x30, 0x7E]), // ESC[20~
                    10 => Cow::Borrowed(&[0x1B, 0x5B, 0x32, 0x31, 0x7E]), // ESC[21~
                    11 => Cow::Borrowed(&[0x1B, 0x5B, 0x32, 0x33, 0x7E]), // ESC[23~
                    12 => Cow::Borrowed(&[0x1B, 0x5B, 0x32, 0x34, 0x7E]), // ESC[24~
                    // Unknown function keys.
                    _ => Cow::Borrowed(&[0x1B]), // Just ESC
                }
            }

            // Raw sequence - pass through as-is (requires owned data)
            ControlSequence::RawSequence(bytes) => Cow::Owned(bytes.clone()),
        }
    }
}
