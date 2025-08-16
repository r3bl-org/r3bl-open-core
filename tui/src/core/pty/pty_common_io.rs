// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Common I/O implementation shared between PTY session types.
//!
//! This module provides the core I/O functionality used by both read-only and
//! read-write PTY sessions:
//! - PTY pair creation and configuration
//! - Async task spawning for I/O operations
//! - Input/output event handling
//! - Resource management and cleanup

use std::{io::{Read, Write},
          sync::mpsc::{Receiver, RecvTimeoutError},
          time::Duration};

use portable_pty::{Child, MasterPty, SlavePty, native_pty_system};
use tokio::sync::mpsc::UnboundedSender;

use crate::{OscBuffer, PtyCommand, PtyConfig, PtyInputEvent, PtyOutputEvent,
            control_char_to_bytes};

/// Buffer size for reading from PTY.
pub const READ_BUFFER_SIZE: usize = 4096;

/// Type alias for the controller half of the PTY (master).
///
/// The controller is the "master" side that your program interacts with.
/// It can read output from and write input to the spawned process.
pub type Controller = Box<dyn MasterPty + Send>;

/// Type alias for the controlled half of the PTY (slave).
///
/// The controlled is the "slave" side that the spawned process uses as its terminal.
/// The spawned process reads from and writes to this side, believing it has a real
/// terminal.
pub type Controlled = Box<dyn SlavePty + Send>;

/// Type alias for the child process spawned in the PTY.
pub type ControlledChild = Box<dyn Child + Send + Sync>;

/// Creates a PTY pair with the specified configuration.
///
/// # Errors
/// Returns an error if the PTY system fails to open a PTY pair.
pub fn create_pty_pair(config: &PtyConfig) -> miette::Result<(Controller, Controlled)> {
    let pty_system = native_pty_system();
    let pty_pair = pty_system
        .openpty(config.get_pty_size())
        .map_err(|e| miette::miette!("Failed to open PTY: {}", e))?;

    Ok((pty_pair.master, pty_pair.slave))
}

/// Spawns a command in the PTY.
///
/// # Errors
/// Returns an error if the command fails to spawn in the PTY.
pub fn spawn_command_in_pty(
    controlled: &Controlled,
    command: PtyCommand,
) -> miette::Result<ControlledChild> {
    controlled
        .spawn_command(command)
        .map_err(|e| miette::miette!("Failed to spawn command: {}", e))
}

/// Spawn a blocking reader task that processes output from the PTY controller half.
///
/// This function spawns a blocking task that continuously reads data from the PTY
/// controller half and processes it according to the provided configuration options.
///
/// # Why `spawn_blocking`?
///
/// PTY operations are inherently **synchronous** and require `spawn_blocking` for proper
/// async integration:
///
/// ## Synchronous PTY APIs
/// - The `portable_pty` crate and underlying PTY file descriptors only provide
///   synchronous I/O
/// - `controller_reader` implements `std::io::Read` (blocking), not
///   `tokio::io::AsyncRead`
/// - PTY file descriptors are Unix concepts that operate at the kernel level with
///   blocking semantics
///
/// ## No `AsyncRead` Implementation
/// - There is no `AsyncRead` implementation available for PTY file descriptors
/// - `portable_pty::MasterPty::take_reader()` returns `Box<dyn Read + Send>`
///   (synchronous)
/// - PTY operations don't map cleanly to async file I/O patterns
///
/// ## Tokio Integration
/// - Using regular `tokio::spawn()` with blocking `Read::read()` would block the entire
///   async runtime
/// - `spawn_blocking()` runs the blocking operation on a dedicated thread pool
/// - This allows other async tasks to continue running while PTY I/O happens on separate
///   threads
///
/// ## Alternative Approaches (and why they don't work)
/// - **Polling/Non-blocking**: PTY file descriptors don't reliably support non-blocking
///   mode across platforms
/// - **Native async PTY library**: Doesn't exist with required cross-platform support
/// - **File descriptor conversion**: `tokio::fs::File::from_std()` doesn't work with PTY
///   FDs
///
/// # Arguments
///
/// * `controller_reader` - A boxed reader that implements [`Read`] + [`Send`], typically
///   the read end of a PTY master file descriptor
/// * `output_event_sender_half` - An unbounded sender for [`PtyOutputEvent`]s to
///   communicate with other parts of the application
/// * `config` - Configuration settings that determine which events to capture
///
/// # Returns
///
/// A [`tokio::task::JoinHandle`]`<`[`miette::Result`]`<()>>` for the spawned blocking
/// task. The task will complete when the PTY is closed (EOF) or an error occurs during
/// reading. CRITICAL - If the PTY is not closed and this join handle is awaited, it will
/// deadlock.
///
/// [`Read`]: std::io::Read
/// [`Send`]: std::marker::Send
/// [`tokio::task::JoinHandle`]: tokio::task::JoinHandle
/// [`miette::Result`]: miette::Result
#[must_use]
pub fn spawn_blocking_controller_reader_task(
    mut controller_reader: Box<dyn Read + Send>,
    output_event_sender_half: UnboundedSender<PtyOutputEvent>,
    config: impl Into<PtyConfig>,
) -> tokio::task::JoinHandle<miette::Result<()>> {
    let config = config.into();
    tokio::task::spawn_blocking(move || -> miette::Result<()> {
        let mut read_buffer = [0u8; READ_BUFFER_SIZE];
        let mut osc_buffer = if config.is_osc_capture_enabled() {
            Some(OscBuffer::new())
        } else {
            None
        };

        loop {
            // This is a synchronous blocking read operation.
            match controller_reader.read(&mut read_buffer) {
                Ok(0) | Err(_) => break, // EOF or error - PTY closed.
                Ok(n) => {
                    let data = &read_buffer[..n];

                    // Send raw output if configured.
                    if config.is_output_capture_enabled() {
                        let _unused = output_event_sender_half
                            .send(PtyOutputEvent::Output(data.to_vec()));
                    }

                    // Process OSC sequences if configured.
                    if let Some(ref mut osc_buf) = osc_buffer {
                        for event in osc_buf.append_and_extract(data, n) {
                            let _unused =
                                output_event_sender_half.send(PtyOutputEvent::Osc(event));
                        }
                    }
                }
            }
        }

        // Reader drops here automatically when the closure ends.
        drop(controller_reader);

        Ok(())
    })
}

/// Creates an input handler task that sends input to the PTY and handles resize.
///
/// This task:
/// - Reads input commands from a channel
/// - Writes data to the PTY master
/// - Handles control characters and text input
/// - Handles PTY resize commands
/// - Reports write errors through the event channel
///
/// This single task owns the `MasterPty` and handles all input operations.
///
/// Returns a `JoinHandle` for the spawned blocking task.
#[must_use]
pub fn create_input_handler_task(
    controller: Box<dyn MasterPty + Send>,
    input_receiver: Receiver<PtyInputEvent>,
    event_sender: UnboundedSender<PtyOutputEvent>,
) -> tokio::task::JoinHandle<miette::Result<()>> {
    tokio::task::spawn_blocking(move || -> miette::Result<()> {
        let controller = controller;
        // Get a writer from the controller
        let mut writer = controller
            .take_writer()
            .map_err(|e| miette::miette!("Failed to take PTY writer: {}", e))?;
        // Process input commands until channel closes or Close command received
        loop {
            // Use timeout to periodically check if we should exit
            match input_receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(input) => {
                    match input {
                        PtyInputEvent::Write(bytes) => {
                            if let Err(e) = writer.write_all(&bytes) {
                                // Send error event before terminating
                                let _unused =
                                    event_sender.send(PtyOutputEvent::WriteError(e));
                                return Err(miette::miette!("Failed to write to PTY"));
                            }
                            if let Err(e) = writer.flush() {
                                let _unused =
                                    event_sender.send(PtyOutputEvent::WriteError(e));
                                return Err(miette::miette!("Failed to flush PTY"));
                            }
                        }
                        PtyInputEvent::WriteLine(text) => {
                            if let Err(e) = writer.write_all(text.as_bytes()) {
                                let _unused =
                                    event_sender.send(PtyOutputEvent::WriteError(e));
                                return Err(miette::miette!(
                                    "Failed to write line to PTY"
                                ));
                            }
                            if let Err(e) = writer.write_all(b"\n") {
                                let _unused =
                                    event_sender.send(PtyOutputEvent::WriteError(e));
                                return Err(miette::miette!(
                                    "Failed to write newline to PTY"
                                ));
                            }
                            if let Err(e) = writer.flush() {
                                let _unused =
                                    event_sender.send(PtyOutputEvent::WriteError(e));
                                return Err(miette::miette!("Failed to flush PTY"));
                            }
                        }
                        PtyInputEvent::SendControl(ctrl) => {
                            let bytes = control_char_to_bytes(&ctrl);
                            if let Err(e) = writer.write_all(&bytes) {
                                let _unused =
                                    event_sender.send(PtyOutputEvent::WriteError(e));
                                return Err(miette::miette!(
                                    "Failed to send control char to PTY"
                                ));
                            }
                            if let Err(e) = writer.flush() {
                                let _unused =
                                    event_sender.send(PtyOutputEvent::WriteError(e));
                                return Err(miette::miette!("Failed to flush PTY"));
                            }
                        }
                        PtyInputEvent::Resize(size) => {
                            // Handle resize directly in this task since we own the
                            // controller
                            if let Err(e) = controller.resize(size) {
                                let _unused =
                                    event_sender.send(PtyOutputEvent::WriteError(
                                        std::io::Error::other(e.to_string()),
                                    ));
                                return Err(miette::miette!("Failed to resize PTY"));
                            }
                        }
                        PtyInputEvent::Flush => {
                            // Explicit flush without writing data
                            if let Err(e) = writer.flush() {
                                let _unused =
                                    event_sender.send(PtyOutputEvent::WriteError(e));
                                return Err(miette::miette!("Failed to flush PTY"));
                            }
                        }
                        PtyInputEvent::Close => {
                            // Close command received, exit the task
                            break;
                        }
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    // Timeout is normal, continue checking
                }
                Err(RecvTimeoutError::Disconnected) => {
                    // Channel closed, exit gracefully
                    break;
                }
            }
        }

        // Controller drops here automatically when the closure ends.
        drop(controller);

        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use portable_pty::PtySize;
    use tokio::sync::mpsc::unbounded_channel;

    use super::*;
    use crate::PtyConfigOption;

    #[test]
    fn test_create_pty_pair() {
        let config = PtyConfig::default();
        let result = create_pty_pair(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_pty_pair_with_custom_size() {
        let config = PtyConfig::default()
            + PtyConfigOption::Size(PtySize {
                rows: 30,
                cols: 100,
                pixel_width: 0,
                pixel_height: 0,
            });
        let result = create_pty_pair(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_spawn_command_in_pty() {
        let config = PtyConfig::default();
        let (_controller, controlled) = create_pty_pair(&config).unwrap();

        let mut command = PtyCommand::new("echo");
        command.arg("test");

        let result = spawn_command_in_pty(&controlled, command);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_reader_task_no_capture() {
        let (event_sender, mut event_receiver) = unbounded_channel();

        // Create a mock reader that sends some data then EOF
        let mock_data = b"test data";
        let reader = Box::new(std::io::Cursor::new(mock_data.to_vec()));

        let handle = spawn_blocking_controller_reader_task(
            reader,
            event_sender,
            PtyConfigOption::NoCaptureOutput,
        );

        // Reader should complete successfully
        let result = tokio::time::timeout(Duration::from_millis(100), handle).await;
        assert!(result.is_ok());

        // No events should be sent since capture is disabled
        assert!(event_receiver.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_create_reader_task_with_output_capture() {
        let (event_sender, mut event_receiver) = unbounded_channel();

        let mock_data = b"test data";
        let reader = Box::new(std::io::Cursor::new(mock_data.to_vec()));

        let handle = spawn_blocking_controller_reader_task(
            reader,
            event_sender,
            PtyConfigOption::Output,
        );

        // Wait for task to complete
        let result = tokio::time::timeout(Duration::from_millis(100), handle).await;
        assert!(result.is_ok());

        // Should receive output event
        if let Ok(event) = event_receiver.try_recv() {
            match event {
                PtyOutputEvent::Output(data) => assert_eq!(data, mock_data),
                _ => panic!("Expected Output event"),
            }
        }
    }

    #[tokio::test]
    async fn test_create_reader_task_with_osc_capture() {
        let (event_sender, mut event_receiver) = unbounded_channel();

        // OSC sequence for Cargo progress update (50%) - using actual escape bytes
        let mock_data = b"\x1b]9;4;1;50\x1b\\";
        let reader = Box::new(std::io::Cursor::new(mock_data.to_vec()));

        // This test now uses the new OSC-only test as the comprehensive one
        // This version keeps the old behavior for backward compatibility
        let handle = spawn_blocking_controller_reader_task(
            reader,
            event_sender,
            PtyConfigOption::Osc,
        );

        // Wait for task to complete
        let result = tokio::time::timeout(Duration::from_millis(100), handle).await;
        assert!(result.is_ok());

        // Collect all events - PtyConfigOption::Osc.into() enables both capture_osc and
        // capture_output
        let mut events = Vec::new();
        while let Ok(event) = event_receiver.try_recv() {
            events.push(event);
        }

        assert!(
            !events.is_empty(),
            "Should have received at least one event"
        );

        // Check that we received an OSC event (may also receive raw output due to default
        // behavior)
        let osc_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyOutputEvent::Osc(osc) => Some(osc),
                _ => None,
            })
            .collect();

        assert!(
            !osc_events.is_empty(),
            "Should have received at least one OSC event"
        );

        // Verify we got the correct OSC event
        let has_correct_event = osc_events
            .iter()
            .any(|osc| matches!(osc, crate::OscEvent::ProgressUpdate(50)));

        assert!(
            has_correct_event,
            "Expected OSC progress update event with 50%"
        );
    }

    #[tokio::test]
    async fn test_create_reader_task_with_osc_only_capture() {
        let (event_sender, mut event_receiver) = unbounded_channel();

        // OSC sequence for Cargo progress update (75%)
        let mock_data = b"\x1b]9;4;1;75\x1b\\";
        let reader = Box::new(std::io::Cursor::new(mock_data.to_vec()));

        // Create config with OSC capture only (disable output capture)
        let config = PtyConfigOption::Osc
            + PtyConfigOption::NoCaptureOutput
            + PtyConfigOption::Osc;

        let handle = spawn_blocking_controller_reader_task(reader, event_sender, config);

        // Wait for task to complete
        let result = tokio::time::timeout(Duration::from_millis(100), handle).await;
        assert!(result.is_ok());

        // Collect all events - should only get OSC events, no raw output
        let mut events = Vec::new();
        while let Ok(event) = event_receiver.try_recv() {
            events.push(event);
        }

        assert!(
            !events.is_empty(),
            "Should have received at least one event"
        );

        // Should have OSC events but no output events
        let osc_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyOutputEvent::Osc(osc) => Some(osc),
                _ => None,
            })
            .collect();

        let output_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyOutputEvent::Output(_) => Some(()),
                _ => None,
            })
            .collect();

        assert!(!osc_events.is_empty(), "Should have received OSC events");
        assert!(
            output_events.is_empty(),
            "Should NOT have received output events (OSC-only capture)"
        );

        // Verify we got the correct OSC event
        let has_correct_event = osc_events
            .iter()
            .any(|osc| matches!(osc, crate::OscEvent::ProgressUpdate(75)));

        assert!(
            has_correct_event,
            "Expected OSC progress update event with 75%"
        );
    }

    #[tokio::test]
    async fn test_create_reader_task_with_both_osc_and_output_capture() {
        let (event_sender, mut event_receiver) = unbounded_channel();

        // OSC sequence for Cargo progress update (25%)
        let mock_data = b"\x1b]9;4;1;25\x1b\\";
        let reader = Box::new(std::io::Cursor::new(mock_data.to_vec()));

        // Create config with both output and OSC capture enabled
        let config = PtyConfigOption::Osc + PtyConfigOption::Output;

        let handle = spawn_blocking_controller_reader_task(reader, event_sender, config);

        // Wait for task to complete
        let result = tokio::time::timeout(Duration::from_millis(100), handle).await;
        assert!(result.is_ok());

        // Collect all events - should get both raw output AND OSC events
        let mut events = Vec::new();
        while let Ok(event) = event_receiver.try_recv() {
            events.push(event);
        }

        assert!(
            !events.is_empty(),
            "Should have received at least one event"
        );

        // Should have both OSC events AND output events
        let osc_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyOutputEvent::Osc(osc) => Some(osc),
                _ => None,
            })
            .collect();

        let output_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyOutputEvent::Output(_) => Some(()),
                _ => None,
            })
            .collect();

        assert!(!osc_events.is_empty(), "Should have received OSC events");
        assert!(
            !output_events.is_empty(),
            "Should have received output events (both capture enabled)"
        );

        // Verify we got the correct OSC event
        let has_correct_event = osc_events
            .iter()
            .any(|osc| matches!(osc, crate::OscEvent::ProgressUpdate(25)));

        assert!(
            has_correct_event,
            "Expected OSC progress update event with 25%"
        );
    }

    #[tokio::test]
    async fn test_create_input_handler_task_write() {
        let config = PtyConfig::default();
        let (controller, _controlled) = create_pty_pair(&config).unwrap();

        let (input_sender, input_receiver) = mpsc::channel();
        let (event_sender, _event_receiver) = unbounded_channel();

        let handle = create_input_handler_task(controller, input_receiver, event_sender);

        // Send write command
        let test_data = b"test input";
        input_sender
            .send(PtyInputEvent::Write(test_data.to_vec()))
            .unwrap();

        // Send close to terminate task
        input_sender.send(PtyInputEvent::Close).unwrap();

        // Task should complete successfully
        let result = tokio::time::timeout(Duration::from_millis(500), handle).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_input_handler_task_write_line() {
        let config = PtyConfig::default();
        let (controller, _controlled) = create_pty_pair(&config).unwrap();

        let (input_sender, input_receiver) = mpsc::channel();
        let (event_sender, _event_receiver) = unbounded_channel();

        let handle = create_input_handler_task(controller, input_receiver, event_sender);

        // Send write line command
        input_sender
            .send(PtyInputEvent::WriteLine("test line".to_string()))
            .unwrap();

        // Send close to terminate task
        input_sender.send(PtyInputEvent::Close).unwrap();

        // Task should complete successfully
        let result = tokio::time::timeout(Duration::from_millis(500), handle).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_input_handler_task_control_char() {
        let config = PtyConfig::default();
        let (controller, _controlled) = create_pty_pair(&config).unwrap();

        let (input_sender, input_receiver) = mpsc::channel();
        let (event_sender, _event_receiver) = unbounded_channel();

        let handle = create_input_handler_task(controller, input_receiver, event_sender);

        // Send control character
        input_sender
            .send(PtyInputEvent::SendControl(crate::ControlChar::CtrlC))
            .unwrap();

        // Send close to terminate task
        input_sender.send(PtyInputEvent::Close).unwrap();

        // Task should complete successfully
        let result = tokio::time::timeout(Duration::from_millis(500), handle).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_input_handler_task_resize() {
        let config = PtyConfig::default();
        let (controller, _controlled) = create_pty_pair(&config).unwrap();

        let (input_sender, input_receiver) = mpsc::channel();
        let (event_sender, _event_receiver) = unbounded_channel();

        let handle = create_input_handler_task(controller, input_receiver, event_sender);

        // Send resize command
        let new_size = PtySize {
            rows: 40,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        };
        input_sender.send(PtyInputEvent::Resize(new_size)).unwrap();

        // Send close to terminate task
        input_sender.send(PtyInputEvent::Close).unwrap();

        // Task should complete successfully
        let result = tokio::time::timeout(Duration::from_millis(500), handle).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_input_handler_task_flush() {
        let config = PtyConfig::default();
        let (controller, _controlled) = create_pty_pair(&config).unwrap();

        let (input_sender, input_receiver) = mpsc::channel();
        let (event_sender, _event_receiver) = unbounded_channel();

        let handle = create_input_handler_task(controller, input_receiver, event_sender);

        // Send flush command
        input_sender.send(PtyInputEvent::Flush).unwrap();

        // Send close to terminate task
        input_sender.send(PtyInputEvent::Close).unwrap();

        // Task should complete successfully
        let result = tokio::time::timeout(Duration::from_millis(500), handle).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_input_handler_task_channel_disconnect() {
        let config = PtyConfig::default();
        let (controller, _controlled) = create_pty_pair(&config).unwrap();

        let (input_sender, input_receiver) = mpsc::channel();
        let (event_sender, _event_receiver) = unbounded_channel();

        let handle = create_input_handler_task(controller, input_receiver, event_sender);

        // Drop sender to disconnect channel
        drop(input_sender);

        // Task should complete successfully when channel disconnects
        let result = tokio::time::timeout(Duration::from_millis(500), handle).await;
        assert!(result.is_ok());
    }
}
