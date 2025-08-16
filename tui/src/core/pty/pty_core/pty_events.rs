// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Event types for PTY input and output communication.

use portable_pty::PtySize;

use crate::OscEvent;
use super::pty_control::ControlChar;

/// Output event types received from a PTY child process.
///
/// # Summary
/// - Bidirectional communication API for receiving output from PTY child processes
/// - Event types: `Output` (stdout/stderr data), `Exit` (process completion), `Osc`
///   (terminal sequences), `UnexpectedExit` (crashes), `WriteError` (I/O failures)
/// - Handles real-time streaming with combined stdout/stderr output for terminal
///   emulation
/// - Used with [`super::pty_sessions::PtyReadOnlySession`] and [`super::pty_sessions::PtyReadWriteSession`] to monitor process
///   output and lifecycle events
/// - Integrates with [`portable_pty`] for cross-platform terminal compatibility
#[derive(Debug)]
pub enum PtyOutputEvent {
    /// OSC sequence event (if OSC capture is enabled).
    Osc(OscEvent),
    /// Raw output data (stdout/stderr combined).
    Output(Vec<u8>),
    /// Process exited with status.
    Exit(portable_pty::ExitStatus),
    /// Child process crashed or terminated unexpectedly.
    UnexpectedExit(String),
    /// Write operation failed - session will terminate.
    WriteError(std::io::Error),
}

/// Input event types that can be sent to a child process through PTY.
///
/// # Summary
/// - Bidirectional communication API for sending commands to PTY child processes
/// - Event types: `Write` (raw data), `WriteLine` (text), `SendControl` (key sequences),
///   `Resize`, `Flush`, `Close`
/// - Supports terminal control sequences, window resizing, and process lifecycle
///   management
/// - Used with [`super::pty_sessions::PtyReadWriteSession`] for interactive terminal applications
#[derive(Debug, Clone)]
pub enum PtyInputEvent {
    /// Send raw bytes to child's stdin.
    Write(Vec<u8>),
    /// Send text with automatic newline.
    WriteLine(String),
    /// Send control sequences (Ctrl-C, Ctrl-D, etc.).
    SendControl(ControlChar),
    /// Resize the PTY window.
    Resize(PtySize),
    /// Explicit flush without writing data.
    /// Forces any buffered data to be sent to the child immediately.
    Flush,
    /// Close stdin (EOF).
    Close,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pty_event_debug() {
        let event = PtyOutputEvent::Output(b"test".to_vec());
        let debug_str = format!("{event:?}");
        assert!(debug_str.contains("Output"));
    }

    #[test]
    fn test_pty_input_debug_and_clone() {
        let input = PtyInputEvent::Write(b"test".to_vec());
        let cloned = input.clone();
        assert_eq!(format!("{input:?}"), format!("{cloned:?}"));
    }
}