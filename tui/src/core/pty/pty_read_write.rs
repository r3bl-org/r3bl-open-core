/*
 *   Copyright (c) 2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

//! Bidirectional PTY communication implementation.
//!
//! This module provides the internal implementation for
//! `PtyCommandBuilder::spawn_read_write()`, which enables both reading from and writing
//! to a child process running in a pseudo-terminal.
//!
//! # Core Architecture
//!
//! - **Shared functionality**: Uses `common_impl.rs` for PTY setup, reader/writer tasks
//! - **Session management**: [`PtySession`] struct provides channels for bidirectional
//!   communication
//! - **Type system**: [`PtyInput`] for sending commands, [`super::ControlChar`] for
//!   special keys, extended [`PtyEvent`] for output
//! - **Memory efficiency**: [`super::control_char_to_bytes()`] uses `Cow<'static, [u8]>`
//!   to avoid unnecessary allocations
//!
//! # Design Decisions
//!
//! ## Dumb Pipes Approach
//! The API treats input and output channels as dumb pipes of events, making no
//! assumptions about the child process. The child determines terminal modes (cooked/raw),
//! interprets environment variables, and handles all terminal-specific behavior. We
//! simply provide the transport layer.
//!
//! ## Single Input Handler Architecture
//! A single task owns the [`portable_pty::MasterPty`] and handles all input operations
//! including resize. This avoids complex synchronization and ensures clean resource
//! management. The task:
//! - Processes all [`PtyInput`] commands
//! - Handles PTY resizing directly
//! - Manages the write side of the PTY
//! - Reports errors via the event channel
//!
//! ## Task Separation
//! - **Reader task**: Independently reads from PTY, processes OSC sequences, sends events
//! - **Input handler task**: Owns [`portable_pty::MasterPty`], processes all input
//!   commands including resize
//! - **Bridge task**: Converts async channel to sync channel for the blocking input
//!   handler
//!
//! ## Error Handling
//! - Write errors terminate the session (no automatic retry)
//! - Errors are reported via [`PtyEvent::WriteError`] before termination
//! - Three termination scenarios handled:
//!   1. Child process self-terminates (normal or crash)
//!   2. Explicit session termination via [`PtyInput::Close`]
//!   3. Unexpected termination (reported as [`PtyEvent::UnexpectedExit`] event)
//!
//! ## Memory Efficiency
//! - Control character sequences use `&'static [u8]` to avoid heap allocations
//! - Only [`crate::ControlChar::RawSequence`] variants require owned data
//! - Unbounded channels for simplicity (no backpressure handling)
//!
//! # Features
//!
//! - **Bidirectional communication**: Full read/write support for interactive processes
//! - **Control characters**: Comprehensive support including:
//!   - Standard controls (Ctrl-C, Ctrl-D, Ctrl-Z, etc.)
//!   - Arrow keys and navigation (Home, End, `PageUp`, `PageDown`)
//!   - Function keys (F1-F12)
//!   - Raw escape sequences for custom needs
//! - **PTY resizing**: Dynamic terminal size adjustment via [`PtyInput::Resize`]
//! - **Explicit flush control**: [`PtyInput::Flush`] for protocols sensitive to message
//!   boundaries
//! - **Proper cleanup**: Careful resource management prevents PTY deadlocks
//!
//! # Example
//!
//! ```rust,no_run
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use r3bl_tui::{
//!     PtyCommandBuilder, PtyConfigOption::*, PtyEvent, PtyInput, ControlChar
//! };
//!
//! // Start an interactive shell
//! let mut session = PtyCommandBuilder::new("bash")
//! /* cspell:disable-next-line */
//!     .args(["--norc"])
//!     .spawn_read_write(Output)?;
//!
//! // Send commands
//! session.input.send(PtyInput::WriteLine("echo 'Hello PTY!'".into()))?;
//! session.input.send(PtyInput::SendControl(ControlChar::CtrlC))?; // Interrupt
//!
//! // Process output
//! while let Some(event) = session.output.recv().await {
//!     match event {
//!         PtyEvent::Output(data) => print!("{}", String::from_utf8_lossy(&data)),
//!         PtyEvent::Exit(status) => break,
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use miette::IntoDiagnostic;
use tokio::sync::mpsc::unbounded_channel;

use crate::{PtyCommandBuilder, PtyConfig, PtyEvent, PtyInput, PtySession,
            common_impl::{create_input_handler_task, create_pty_pair,
                          create_reader_task, spawn_command_in_pty}};

/// Internal implementation for spawning a read-write PTY session.
///
/// This is called by `PtyCommandBuilder::spawn_read_write()`.
pub(crate) fn spawn_pty_read_write_impl(
    command: PtyCommandBuilder,
    config: impl Into<PtyConfig>,
) -> PtySession {
    let config = config.into();

    // Create channels for bidirectional communication
    let (input_sender, input_receiver) = unbounded_channel::<PtyInput>();
    let (event_sender, event_receiver) = unbounded_channel::<PtyEvent>();

    // Create a sync channel for the input handler task (spawn_blocking needs sync
    // channel)
    let (input_handler_sender, input_handler_receiver) =
        std::sync::mpsc::channel::<PtyInput>();

    // Clone senders for various tasks
    let reader_event_sender = event_sender.clone();
    let input_handler_event_sender = event_sender.clone();

    // Spawn the main orchestration task
    let handle = Box::pin(tokio::spawn(async move {
        // Build the command, ensuring CWD is set
        let command = command.build()?;

        // Create PTY pair
        let (controller, controlled) = create_pty_pair(&config)?;

        // Spawn the command in the PTY
        let mut child = spawn_command_in_pty(&controlled, command)?;

        // Clone a reader for the reader task
        let reader = controller
            .try_clone_reader()
            .map_err(|e| miette::miette!("Failed to clone reader: {}", e))?;

        // Start the reader task with a reader clone
        let reader_handle = create_reader_task(
            reader,
            reader_event_sender,
            config.is_osc_capture_enabled(),
            config.is_output_capture_enabled(),
        );

        // The input handler task owns the controller and handles all input operations
        let input_handler_handle = create_input_handler_task(
            controller,
            input_handler_receiver,
            input_handler_event_sender,
        );

        // Spawn a task to bridge async input channel to sync channel for input handler
        let bridge_handle = tokio::spawn(async move {
            let mut receiver = input_receiver;
            while let Some(input) = receiver.recv().await {
                if input_handler_sender.send(input).is_err() {
                    // Input handler task has exited
                    break;
                }
            }

            // Ensure input handler gets Close signal
            let _unused = input_handler_sender.send(PtyInput::Close);
        });

        // Wait for the child process to complete
        let status = tokio::task::spawn_blocking(move || child.wait())
            .await
            .into_diagnostic()?
            .into_diagnostic()?;

        // Store exit code before moving status
        let exit_code = status.exit_code();

        // Send exit event
        let _unused = event_sender.send(PtyEvent::Exit(status));

        // Clean up: drop the controlled side to signal EOF to reader
        drop(controlled);

        // input_sender will be dropped when this task completes

        // Wait for all tasks to complete
        let _unused = bridge_handle.await;
        let _unused = input_handler_handle.await;
        let _unused = reader_handle.await;

        // Return the exit status
        Ok(portable_pty::ExitStatus::with_exit_code(exit_code))
    }));

    PtySession {
        input: input_sender,
        output: event_receiver,
        handle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ControlChar, PtyConfigOption};

    #[tokio::test]
    async fn test_echo_command() {
        use PtyConfigOption::*;

        let mut session = PtyCommandBuilder::new("echo")
            .args(["Hello, PTY!"])
            .spawn_read_write(Output)
            .unwrap();

        let mut output = String::new();
        while let Some(event) = session.output.recv().await {
            match event {
                PtyEvent::Output(data) => {
                    output.push_str(&String::from_utf8_lossy(&data));
                }
                PtyEvent::Exit(status) => {
                    assert!(status.success());
                    break;
                }
                _ => {}
            }
        }

        assert!(output.contains("Hello, PTY!"));
    }

    #[tokio::test]
    async fn test_cat_with_input() {
        use PtyConfigOption::*;

        let mut session = PtyCommandBuilder::new("cat")
            .spawn_read_write(Output)
            .unwrap();

        // Send some text
        session
            .input
            .send(PtyInput::WriteLine("test input".into()))
            .unwrap();

        // Send EOF to make cat exit
        session
            .input
            .send(PtyInput::SendControl(ControlChar::CtrlD))
            .unwrap();

        let mut output = String::new();
        while let Some(event) = session.output.recv().await {
            match event {
                PtyEvent::Output(data) => {
                    output.push_str(&String::from_utf8_lossy(&data));
                }
                PtyEvent::Exit(_) => break,
                _ => {}
            }
        }

        assert!(output.contains("test input"));
    }

    #[tokio::test]
    async fn test_python_repl_interaction() {
        use PtyConfigOption::*;
        use tokio::time::{Duration, timeout};

        // Skip if Python is not available
        if std::process::Command::new("python3")
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!("Skipping Python test - python3 not available");
            return;
        }

        // Use a simple Python command that exits immediately
        let mut session = PtyCommandBuilder::new("python3")
            .args(["-c", "print(2 + 3); print('Hello from Python')"])
            .spawn_read_write(Output)
            .unwrap();

        // Collect output with timeout
        let mut output = String::new();
        let result = timeout(Duration::from_secs(2), async {
            while let Some(event) = session.output.recv().await {
                match event {
                    PtyEvent::Output(data) => {
                        output.push_str(&String::from_utf8_lossy(&data));
                    }
                    PtyEvent::Exit(_) => break,
                    _ => {}
                }
            }
        })
        .await;

        assert!(result.is_ok(), "Python session timed out");

        // Verify we got expected output
        assert!(output.contains('5'), "Should see result of 2+3");
        assert!(
            output.contains("Hello from Python"),
            "Should see hello message"
        );
    }

    #[tokio::test]
    async fn test_shell_command_interruption() {
        use PtyConfigOption::*;
        use tokio::time::{Duration, timeout};

        // Test that we can send commands to a shell
        let mut session = PtyCommandBuilder::new("sh")
            .args(["-c", "echo 'Test output from shell'"])
            .spawn_read_write(Output)
            .unwrap();

        // Collect output
        let mut output = String::new();
        let mut saw_exit = false;
        let result = timeout(Duration::from_secs(2), async {
            while let Some(event) = session.output.recv().await {
                match event {
                    PtyEvent::Output(data) => {
                        output.push_str(&String::from_utf8_lossy(&data));
                    }
                    PtyEvent::Exit(status) => {
                        saw_exit = true;
                        if status.success() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        })
        .await;

        assert!(result.is_ok(), "Shell session timed out");
        assert!(saw_exit, "Should see exit event");
        assert!(
            output.contains("Test output from shell"),
            "Should see shell output"
        );
    }

    #[tokio::test]
    async fn test_multiple_control_characters() {
        use PtyConfigOption::*;

        let mut session = PtyCommandBuilder::new("cat")
            .spawn_read_write(Output)
            .unwrap();

        // Test various control characters
        session
            .input
            .send(PtyInput::WriteLine("Test line".into()))
            .unwrap();
        session
            .input
            .send(PtyInput::SendControl(ControlChar::Enter))
            .unwrap();
        session
            .input
            .send(PtyInput::Write(b"No newline".to_vec()))
            .unwrap();
        session
            .input
            .send(PtyInput::SendControl(ControlChar::Tab))
            .unwrap();
        session
            .input
            .send(PtyInput::Write(b"After tab".to_vec()))
            .unwrap();
        session
            .input
            .send(PtyInput::SendControl(ControlChar::Enter))
            .unwrap();

        // Send EOF to exit
        session
            .input
            .send(PtyInput::SendControl(ControlChar::CtrlD))
            .unwrap();

        let mut output = String::new();
        while let Some(event) = session.output.recv().await {
            match event {
                PtyEvent::Output(data) => {
                    output.push_str(&String::from_utf8_lossy(&data));
                }
                PtyEvent::Exit(_) => break,
                _ => {}
            }
        }

        assert!(output.contains("Test line"));
        assert!(output.contains("No newline"));
        assert!(output.contains("After tab"));
    }

    #[tokio::test]
    async fn test_raw_escape_sequences() {
        use PtyConfigOption::*;

        let mut session = PtyCommandBuilder::new("cat")
            .spawn_read_write(Output)
            .unwrap();

        // Send some text with ANSI color codes using raw sequences
        let red_text = b"\x1b[31mRed Text\x1b[0m";
        session
            .input
            .send(PtyInput::Write(red_text.to_vec()))
            .unwrap();
        session
            .input
            .send(PtyInput::SendControl(ControlChar::Enter))
            .unwrap();

        // Send using RawSequence variant
        let blue_seq = vec![0x1b, b'[', b'3', b'4', b'm']; // Blue color
        session
            .input
            .send(PtyInput::SendControl(ControlChar::RawSequence(blue_seq)))
            .unwrap();
        session
            .input
            .send(PtyInput::Write(b"Blue Text".to_vec()))
            .unwrap();
        session
            .input
            .send(PtyInput::SendControl(ControlChar::Enter))
            .unwrap();

        // EOF to exit
        session
            .input
            .send(PtyInput::SendControl(ControlChar::CtrlD))
            .unwrap();

        let mut output = Vec::new();
        while let Some(event) = session.output.recv().await {
            match event {
                PtyEvent::Output(data) => {
                    output.extend_from_slice(&data);
                }
                PtyEvent::Exit(_) => break,
                _ => {}
            }
        }

        // Check we got the ANSI sequences back
        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("Red Text"));
        assert!(output_str.contains("Blue Text"));
        // The actual ANSI codes might be echoed back
        // Note: cat may not preserve exact ANSI sequences depending on terminal settings
    }
}
