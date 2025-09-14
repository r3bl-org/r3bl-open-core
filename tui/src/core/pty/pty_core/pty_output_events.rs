// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Output event types and terminal control sequences for PTY communication.
//!
//! This module defines events that flow FROM the PTY child process TO the application:
//! - [`PtyReadOnlyOutputEvent`] - Events for read-only monitoring sessions
//! - [`PtyReadWriteOutputEvent`] - Events for interactive read-write sessions
//! - [`ControlSequence`] - Terminal control sequences with mode-aware generation
//! - [`CursorKeyMode`] - Cursor key mode detection and management
//!
//! ## Terminal Cursor Key Modes
//!
//! Application Mode vs Normal Mode cursor keys. This is a classic terminal
//! compatibility issue.
//!
//! In terminals, arrow keys and function keys can be sent in two different modes:
//!
//! | Key  | Application Mode (VT52/SS3) | Normal Mode (ANSI/CSI) |
//! |----- |-----------------------------|------------------------|
//! | Up   | `\x1BOA`                    | `\x1B[A`               |
//! | Down | `\x1BOB`                    | `\x1B[B`               |
//! | Right| `\x1BOC`                    | `\x1B[C`               |
//! | Left | `\x1BOD`                    | `\x1B[D`               |
//!
//! - **Application Mode (VT52/SS3/Single Shift 3)** uses ESC O sequences for better
//!   terminal compatibility. This is the default mode that we use.
//! - **Normal Mode (ANSI/CSI/Control Sequence Introducer)** uses standard ANSI escape
//!   sequences.
//! - The difference is `\x1B[` (CSI) vs `\x1BO` (SS3).
//!
//! ## Mode Detection Benefits
//!
//! 1. We use Application Mode as the default mode (when no mode set).
//! 2. **Automatic adaptation** - Send `\x1B[A` (normal) or `\x1BOA` (application) based
//!    on what the app actually expects.
//! 3. **App compatibility** - Different apps have different preferences (vim vs htop vs
//!    bash).
//! 4. **Dynamic switching** - Apps can change modes mid-session.
//! 5. **No hardcoding** - Works with any terminal application without assumptions.
//!
//! ## Implementation
//!
//! ```text
//! // Watch for these sequences in PTY output:
//! // \x1B[?1h - Enable application cursor keys
//! // \x1B[?1l - Disable application cursor keys (back to normal)
//!
//! // Then generate appropriate sequences:
//! match current_mode {
//!     CursorKeyMode::Normal => "\x1B[A",      // Up
//!     CursorKeyMode::Application => "\x1BOA", // Up
//! }
//! ```
//!
//! This way your PTY implementation becomes a proper terminal emulator that responds
//! correctly to whatever the running application expects, rather than forcing one mode or
//! the other.
//!
//! The detection approach is more work upfront but creates a much more robust and
//! compatible system.

use std::borrow::Cow;

use crate::OscEvent;

/// Cursor key mode for terminal compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorKeyMode {
    /// Normal mode (ANSI) - ESC[ sequences
    Normal,
    /// Application mode (VT52) - ESC O sequences
    Application,
}

impl Default for CursorKeyMode {
    fn default() -> Self {
        Self::Application // Most PTY apps expect this
    }
}

/// Terminal control sequences with mode-aware generation.
///
/// This replaces the old `ControlChar` enum and provides mode-aware
/// sequence generation for better terminal application compatibility.
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

/// Cursor mode detector for parsing PTY output streams.
///
/// Scans for terminal mode switching sequences and maintains a buffer
/// for partial sequence detection across read boundaries.
#[derive(Debug)]
pub struct CursorModeDetector {
    buffer: Vec<u8>,
}

impl CursorModeDetector {
    /// Creates a new cursor mode detector.
    #[must_use]
    pub fn new() -> Self { Self { buffer: Vec::new() } }

    /// Scans incoming data for cursor mode change sequences.
    ///
    /// Returns `Some(mode)` if a mode change sequence is detected, `None` otherwise.
    /// Maintains an internal buffer to handle sequences that span multiple reads.
    pub fn scan_for_mode_change(&mut self, data: &[u8]) -> Option<CursorKeyMode> {
        // Add new data to buffer.
        self.buffer.extend_from_slice(data);

        // Look for application mode enable: ESC[?1h
        if let Some(pos) = self.buffer.windows(5).position(|w| w == b"\x1B[?1h") {
            self.buffer.drain(..=pos + 4); // Remove processed bytes
            return Some(CursorKeyMode::Application);
        }

        // Look for application mode disable (normal mode): ESC[?1l
        if let Some(pos) = self.buffer.windows(5).position(|w| w == b"\x1B[?1l") {
            self.buffer.drain(..=pos + 4);
            return Some(CursorKeyMode::Normal);
        }

        // Keep buffer reasonable size (prevent memory growth)
        if self.buffer.len() > 100 {
            self.buffer.drain(..50);
        }

        None
    }
}

impl Default for CursorModeDetector {
    fn default() -> Self { Self::new() }
}

/// Output event types for read-only PTY sessions.
///
/// # Summary
/// - Used with [`super::pty_sessions::PtyReadOnlySession`] for monitoring child processes
/// - Event types: `Output` (optional, based on config), `Osc` (optional, based on
///   config), `Exit`
/// - Supports OSC sequence capture for terminal automation and progress monitoring
/// - Output capture is configurable for selective data processing
/// - Integrates with [`portable_pty`] for cross-platform terminal compatibility
#[derive(Debug)]
pub enum PtyReadOnlyOutputEvent {
    /// OSC sequence event (if OSC capture is enabled in config).
    Osc(OscEvent),
    /// Raw output data (if output capture is enabled in config).
    Output(Vec<u8>),
    /// Process exited with status.
    Exit(portable_pty::ExitStatus),
}

/// Output event types for read-write PTY sessions.
///
/// # Summary
/// - Used with [`super::pty_sessions::PtyReadWriteSession`] for interactive terminal
///   sessions
/// - Event types: `Output` (always sent), `CursorModeChange` (when detected), `Exit`,
///   `UnexpectedExit`, `WriteError`
/// - Follows "dumb pipe" philosophy - raw bytes pass through, but mode detection added
/// - Handles write errors and unexpected termination for robust error reporting
/// - Integrates with [`portable_pty`] for cross-platform terminal compatibility
#[derive(Debug)]
pub enum PtyReadWriteOutputEvent {
    /// Raw output data (always sent, no processing).
    Output(Vec<u8>),
    /// Cursor key mode changed (detected from terminal escape sequences).
    CursorModeChange(CursorKeyMode),
    /// Process exited with status.
    Exit(portable_pty::ExitStatus),
    /// Child process crashed or terminated unexpectedly.
    UnexpectedExit(String),
    /// Write operation failed - session will terminate.
    WriteError(miette::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pty_event_debug() {
        let read_only_event = PtyReadOnlyOutputEvent::Output(b"test".to_vec());
        let debug_str = format!("{read_only_event:?}");
        assert!(debug_str.contains("Output"));

        let read_write_event = PtyReadWriteOutputEvent::Output(b"test".to_vec());
        let debug_str = format!("{read_write_event:?}");
        assert!(debug_str.contains("Output"));

        let mode_change_event =
            PtyReadWriteOutputEvent::CursorModeChange(CursorKeyMode::Application);
        let debug_str = format!("{mode_change_event:?}");
        assert!(debug_str.contains("CursorModeChange"));
    }

    #[test]
    fn test_cursor_key_mode() {
        assert_eq!(CursorKeyMode::default(), CursorKeyMode::Application);

        let normal = CursorKeyMode::Normal;
        let application = CursorKeyMode::Application;
        assert_ne!(normal, application);
    }

    #[test]
    fn test_control_sequence_to_bytes_basic() {
        let mode = CursorKeyMode::Application;

        assert_eq!(*ControlSequence::CtrlC.to_bytes(mode), [0x03]);
        assert_eq!(*ControlSequence::CtrlD.to_bytes(mode), [0x04]);
        assert_eq!(*ControlSequence::Tab.to_bytes(mode), [0x09]);
        assert_eq!(*ControlSequence::Enter.to_bytes(mode), [0x0D]);
    }

    #[test]
    fn test_control_sequence_arrow_keys_mode_aware() {
        // Test Normal Mode.
        let normal_mode = CursorKeyMode::Normal;
        assert_eq!(
            *ControlSequence::ArrowUp.to_bytes(normal_mode),
            [0x1B, 0x5B, 0x41]
        ); // ESC[A
        assert_eq!(
            *ControlSequence::ArrowDown.to_bytes(normal_mode),
            [0x1B, 0x5B, 0x42]
        ); // ESC[B
        assert_eq!(
            *ControlSequence::ArrowRight.to_bytes(normal_mode),
            [0x1B, 0x5B, 0x43]
        ); // ESC[C
        assert_eq!(
            *ControlSequence::ArrowLeft.to_bytes(normal_mode),
            [0x1B, 0x5B, 0x44]
        ); // ESC[D

        // Test Application Mode.
        let app_mode = CursorKeyMode::Application;
        assert_eq!(
            *ControlSequence::ArrowUp.to_bytes(app_mode),
            [0x1B, 0x4F, 0x41]
        ); // ESC O A
        assert_eq!(
            *ControlSequence::ArrowDown.to_bytes(app_mode),
            [0x1B, 0x4F, 0x42]
        ); // ESC O B
        assert_eq!(
            *ControlSequence::ArrowRight.to_bytes(app_mode),
            [0x1B, 0x4F, 0x43]
        ); // ESC O C
        assert_eq!(
            *ControlSequence::ArrowLeft.to_bytes(app_mode),
            [0x1B, 0x4F, 0x44]
        ); // ESC O D
    }

    #[test]
    fn test_control_sequence_function_keys() {
        let mode = CursorKeyMode::Application;

        // Test F1-F4
        assert_eq!(*ControlSequence::F(1).to_bytes(mode), [0x1B, 0x4F, 0x50]);
        assert_eq!(*ControlSequence::F(2).to_bytes(mode), [0x1B, 0x4F, 0x51]);
        assert_eq!(*ControlSequence::F(3).to_bytes(mode), [0x1B, 0x4F, 0x52]);
        assert_eq!(*ControlSequence::F(4).to_bytes(mode), [0x1B, 0x4F, 0x53]);

        // Test F5-F12
        assert_eq!(
            *ControlSequence::F(5).to_bytes(mode),
            [0x1B, 0x5B, 0x31, 0x35, 0x7E]
        );
        assert_eq!(
            *ControlSequence::F(12).to_bytes(mode),
            [0x1B, 0x5B, 0x32, 0x34, 0x7E]
        );

        // Test unknown function key.
        assert_eq!(*ControlSequence::F(99).to_bytes(mode), [0x1B]);
    }

    #[test]
    fn test_control_sequence_raw_sequence() {
        let mode = CursorKeyMode::Application;
        let custom_bytes = vec![0x1B, 0x5B, 0x33, 0x31, 0x6D]; // Custom ANSI sequence
        let ctrl = ControlSequence::RawSequence(custom_bytes.clone());
        assert_eq!(*ctrl.to_bytes(mode), *custom_bytes);
    }

    #[test]
    fn test_cursor_mode_detector() {
        let mut detector = CursorModeDetector::new();

        // Test application mode detection.
        let app_mode_data = b"\x1B[?1h";
        assert_eq!(
            detector.scan_for_mode_change(app_mode_data),
            Some(CursorKeyMode::Application)
        );

        // Test normal mode detection.
        let normal_mode_data = b"\x1B[?1l";
        assert_eq!(
            detector.scan_for_mode_change(normal_mode_data),
            Some(CursorKeyMode::Normal)
        );

        // Test no mode change.
        let regular_data = b"Hello world";
        assert_eq!(detector.scan_for_mode_change(regular_data), None);
    }

    #[test]
    fn test_cursor_mode_detector_partial_sequences() {
        let mut detector = CursorModeDetector::new();

        // Test partial sequence across multiple calls.
        assert_eq!(detector.scan_for_mode_change(b"\x1B["), None);
        assert_eq!(detector.scan_for_mode_change(b"?1"), None);
        assert_eq!(
            detector.scan_for_mode_change(b"h"),
            Some(CursorKeyMode::Application)
        );
    }

    #[test]
    fn test_cursor_mode_detector_buffer_management() {
        let mut detector = CursorModeDetector::new();

        // Fill buffer with data that doesn't contain mode sequences.
        let large_data = vec![b'x'; 150];
        assert_eq!(detector.scan_for_mode_change(&large_data), None);

        // Buffer should be trimmed to prevent memory growth.
        assert!(detector.buffer.len() <= 100);
    }
}
