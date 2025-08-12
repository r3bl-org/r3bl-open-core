// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Common implementation details shared between PTY spawning functions.
//!
//! This module contains the shared logic for:
//! - PTY setup and initialization
//! - Reader and writer task creation
//! - Resource management patterns

use std::{io::{Read, Write},
          sync::mpsc::{Receiver, RecvTimeoutError},
          time::Duration};

use portable_pty::{Child, MasterPty, SlavePty, native_pty_system};
use tokio::sync::mpsc::UnboundedSender;

use crate::{OscBuffer, PtyCommand, PtyConfig, PtyEvent, PtyInput, control_char_to_bytes};

/// Buffer size for reading from PTY.
pub const READ_BUFFER_SIZE: usize = 4096;

/// Type alias for the controller (master) side of the PTY.
pub type Controller = Box<dyn MasterPty + Send>;

/// Type alias for the controlled (slave) side of the PTY.
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

/// Creates a reader task that processes output from the PTY.
///
/// This task:
/// - Reads data from the PTY master in a blocking manner
/// - Optionally captures raw output
/// - Optionally processes OSC sequences
/// - Sends events through the provided channel
///
/// Returns a `JoinHandle` for the spawned blocking task.
#[must_use]
pub fn create_reader_task(
    mut reader: Box<dyn Read + Send>,
    event_sender: UnboundedSender<PtyEvent>,
    should_capture_osc: bool,
    should_capture_output: bool,
) -> tokio::task::JoinHandle<miette::Result<()>> {
    tokio::task::spawn_blocking(move || -> miette::Result<()> {
        let mut read_buffer = [0u8; READ_BUFFER_SIZE];
        let mut osc_buffer = if should_capture_osc {
            Some(OscBuffer::new())
        } else {
            None
        };

        loop {
            // This is a synchronous blocking read operation.
            match reader.read(&mut read_buffer) {
                Ok(0) | Err(_) => break, // EOF or error - PTY closed.
                Ok(n) => {
                    let data = &read_buffer[..n];

                    // Send raw output if configured.
                    if should_capture_output {
                        let _unused = event_sender.send(PtyEvent::Output(data.to_vec()));
                    }

                    // Process OSC sequences if configured.
                    if let Some(ref mut osc_buf) = osc_buffer {
                        for event in osc_buf.append_and_extract(data, n) {
                            let _unused = event_sender.send(PtyEvent::Osc(event));
                        }
                    }
                }
            }
        }

        // Reader drops here automatically when the closure ends.
        drop(reader);

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
    input_receiver: Receiver<PtyInput>,
    event_sender: UnboundedSender<PtyEvent>,
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
                        PtyInput::Write(bytes) => {
                            if let Err(e) = writer.write_all(&bytes) {
                                // Send error event before terminating
                                let _unused = event_sender.send(PtyEvent::WriteError(e));
                                return Err(miette::miette!("Failed to write to PTY"));
                            }
                            if let Err(e) = writer.flush() {
                                let _unused = event_sender.send(PtyEvent::WriteError(e));
                                return Err(miette::miette!("Failed to flush PTY"));
                            }
                        }
                        PtyInput::WriteLine(text) => {
                            if let Err(e) = writer.write_all(text.as_bytes()) {
                                let _unused = event_sender.send(PtyEvent::WriteError(e));
                                return Err(miette::miette!(
                                    "Failed to write line to PTY"
                                ));
                            }
                            if let Err(e) = writer.write_all(b"\n") {
                                let _unused = event_sender.send(PtyEvent::WriteError(e));
                                return Err(miette::miette!(
                                    "Failed to write newline to PTY"
                                ));
                            }
                            if let Err(e) = writer.flush() {
                                let _unused = event_sender.send(PtyEvent::WriteError(e));
                                return Err(miette::miette!("Failed to flush PTY"));
                            }
                        }
                        PtyInput::SendControl(ctrl) => {
                            let bytes = control_char_to_bytes(&ctrl);
                            if let Err(e) = writer.write_all(&bytes) {
                                let _unused = event_sender.send(PtyEvent::WriteError(e));
                                return Err(miette::miette!(
                                    "Failed to send control char to PTY"
                                ));
                            }
                            if let Err(e) = writer.flush() {
                                let _unused = event_sender.send(PtyEvent::WriteError(e));
                                return Err(miette::miette!("Failed to flush PTY"));
                            }
                        }
                        PtyInput::Resize(size) => {
                            // Handle resize directly in this task since we own the
                            // controller
                            if let Err(e) = controller.resize(size) {
                                let _unused = event_sender.send(PtyEvent::WriteError(
                                    std::io::Error::other(e.to_string()),
                                ));
                                return Err(miette::miette!("Failed to resize PTY"));
                            }
                        }
                        PtyInput::Flush => {
                            // Explicit flush without writing data
                            if let Err(e) = writer.flush() {
                                let _unused = event_sender.send(PtyEvent::WriteError(e));
                                return Err(miette::miette!("Failed to flush PTY"));
                            }
                        }
                        PtyInput::Close => {
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
