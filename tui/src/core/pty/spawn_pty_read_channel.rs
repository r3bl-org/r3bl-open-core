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

// TODO: Implement testing strategy using docs/task_test_pty.md

use std::pin::Pin;

use miette::IntoDiagnostic;
use portable_pty::{CommandBuilder, native_pty_system};
use tokio::sync::mpsc::UnboundedSender;

use super::{Controlled, ControlledChild, Controller, OscBuffer, PtyConfig, PtyEvent,
            READ_BUFFER_SIZE};

/// Spawns a command in a PTY to capture output without providing input.
///
/// This is a read-only PTY command spawner that can:
/// 1. Run any command (not just cargo)
/// 2. Optionally capture and parse OSC sequences
/// 3. Optionally capture raw output data
/// 4. Use custom PTY dimensions
///
/// Note: This function does not allow sending input to the spawned process. For
/// interactive PTY sessions, a future `spawn_pty_read_write_channels` function would be
/// needed.
///
/// # Arguments
/// * `command` - The command to execute (configured via [`CommandBuilder`])
/// * `config` - Configuration for what to capture (implements [`Into<PtyConfig>`])
/// * `event_sender` - Channel sender for [`PtyEvent`]s
///
/// # Returns
/// A pinned [`tokio::task::JoinHandle`] that resolves to the process exit
/// status.
///
/// ## Why Pinning is Required
///
/// The `JoinHandle` is pinned to the heap using [`Box::pin`] for two important reasons:
///
/// 1. **[`tokio::select!`] requirement**: The `JoinHandle` doesn't implement [`Unpin`],
///    and `tokio::select!` requires all futures to be `Unpin`. By returning a pre-pinned
///    handle, callers can use it directly in `tokio::select!` blocks without additional
///    pinning.
///
/// 2. **Heap vs Stack pinning**: We use `Box::pin` (heap pinning) rather than the
///    [`std::pin::pin!`] macro (stack pinning) because this function returns the pinned
///    value. Stack pinning with `pin!` creates a value that cannot outlive the function's
///    stack frame, making it impossible to return. Heap pinning ensures the pinned future
///    remains valid after the function returns and can be safely moved between async
///    contexts.
///
/// This design simplifies usage by eliminating the need for manual pinning at the call
/// site while ensuring the future can be safely polled across await points.
///
/// # Example
///
/// This example demonstrates running a command with output capture, OSC sequence parsing,
/// and custom PTY dimensions:
///
/// ```rust
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use r3bl_tui::{PtyCommandBuilder, PtyConfigOption, PtyEvent, OscEvent, spawn_pty_capture_output_no_input};
/// use tokio::sync::mpsc::unbounded_channel;
/// use portable_pty::PtySize;
/// use PtyConfigOption::*;
///
/// // Configure command with OSC sequences enabled
/// let cmd = PtyCommandBuilder::new("cargo")
///     .args(["--version"]) // Use --version for quick test
///     .enable_osc_sequences()
///     .build()?;
///
/// // Configure PTY with custom dimensions, OSC parsing, and output capture
/// let config = Size(PtySize { rows: 40, cols: 120, pixel_width: 0, pixel_height: 0 })
///     + Osc + Output;
///
/// let (sender, mut receiver) = unbounded_channel();
/// let mut handle = spawn_pty_capture_output_no_input(cmd, config, sender);
///
/// // Process events from the PTY
/// loop {
///     tokio::select! {
///         result = &mut handle => {
///             let _status = result??;
///             break;
///         }
///         Some(event) = receiver.recv() => {
///             match event {
///                 PtyEvent::Output(data) => {
///                     print!("{}", String::from_utf8_lossy(&data));
///                 }
///                 PtyEvent::Osc(OscEvent::ProgressUpdate(pct)) => {
///                     println!("Build progress: {}%", pct);
///                 }
///                 PtyEvent::Exit(status) => {
///                     println!("Exited with: {:?}", status);
///                     break;
///                 }
///                 _ => {}
///             }
///         }
///     }
/// }
/// # Ok(())
/// # }
/// ```
///
/// The function handles all PTY lifecycle management internally, including proper
/// cleanup of file descriptors to avoid deadlocks.
pub fn spawn_pty_capture_output_no_input(
    /* move */ command: CommandBuilder,
    /* move */ config: impl Into<PtyConfig>,
    /* move */ event_sender: UnboundedSender<PtyEvent>,
) -> Pin<Box<tokio::task::JoinHandle<miette::Result<portable_pty::ExitStatus>>>> {
    let config = config.into();

    Box::pin(tokio::spawn(async move {
        // Create a pseudo-terminal with configured dimensions.
        let pty_system = native_pty_system();
        let pty_pair = pty_system
            .openpty(config.get_pty_size())
            .map_err(|e| miette::miette!("Failed to open PTY: {}", e))?;

        // Extract the endpoints of the PTY using type aliases.
        let controller: Controller = pty_pair.master;
        let controlled: Controlled = pty_pair.slave;

        // [SPAWN 1] Spawn the command with PTY (makes is_terminal() return true).
        let mut controlled_child: ControlledChild = controlled
            .spawn_command(command)
            .map_err(|e| miette::miette!("Failed to spawn command: {}", e))?;

        // [SPAWN 2] Spawn the reader task to process output.
        //
        // CRITICAL: PTY LIFECYCLE AND FILE DESCRIPTOR MANAGEMENT
        // ========================================================
        // Understanding how PTYs handle EOF is crucial to avoiding deadlocks.
        //
        // ## The PTY File Descriptor Reference Counting Problem
        //
        // A PTY consists of two sides: master (controller) and slave (controlled).
        // The kernel's PTY implementation requires BOTH conditions for EOF:
        //
        // 1. The slave side must be closed (happens when the child process exits)
        // 2. The reader must be the ONLY remaining reference to the master
        //
        // ## Why We Need Explicit Resource Management
        //
        // Even though the child process has exited and closed its slave FD, our
        // `controlled` variable keeps the slave side open. The PTY won't send EOF
        // to the master until ALL slave file descriptors are closed. Without
        // explicitly dropping `controlled`, it would remain open until this
        // entire function returns, causing the reader to block forever waiting
        // for EOF that never comes.
        //
        // ## The Solution
        //
        // 1. Move controller into spawn_blocking - ensures it drops after creating reader
        // 2. Explicitly drop controlled after process exits - closes our slave FD
        // 3. This allows the reader to receive EOF and exit cleanly
        let reader_event_sender = event_sender.clone();
        let should_capture_osc = config.is_osc_capture_enabled();
        let should_capture_output = config.is_output_capture_enabled();
        let blocking_reader_task_join_handle =
            tokio::task::spawn_blocking(move || -> miette::Result<()> {
                // Controller is MOVED into this closure, so it will be dropped
                // when this task completes, allowing proper PTY cleanup.
                let mut controller_reader = controller
                    .try_clone_reader()
                    .map_err(|e| miette::miette!("Failed to clone pty reader: {}", e))?;

                let mut read_buffer = [0u8; READ_BUFFER_SIZE];
                let mut osc_buffer = if should_capture_osc {
                    Some(OscBuffer::new())
                } else {
                    None
                };

                loop {
                    // This is a synchronous blocking read operation.
                    match controller_reader.read(&mut read_buffer) {
                        Ok(0) | Err(_) => break, // EOF or error - PTY closed
                        Ok(n) => {
                            let data = &read_buffer[..n];

                            // Send raw output if configured
                            if should_capture_output {
                                let _unused = reader_event_sender
                                    .send(PtyEvent::Output(data.to_vec()));
                            }

                            // Process OSC sequences if configured
                            if let Some(ref mut osc_buf) = osc_buffer {
                                for event in osc_buf.append_and_extract(data, n) {
                                    let _unused =
                                        reader_event_sender.send(PtyEvent::Osc(event));
                                }
                            }
                        }
                    }
                }

                // Controller drops here automatically when the closure ends.
                drop(controller);

                Ok(())
            });

        // [WAIT 1] Wait for the command to complete.
        let status = tokio::task::spawn_blocking(move || controlled_child.wait())
            .await
            .into_diagnostic()?
            .into_diagnostic()?;

        // Store exit code before moving status into event.
        let exit_code = status.exit_code();

        // Send exit event (moves status).
        let _unused = event_sender.send(PtyEvent::Exit(status));

        // Explicitly drop the controlled (slave) side after process exits. If you don't
        // do this, the PTY will not send EOF to the master, causing the reader to
        // block forever waiting for EOF that never comes (and will cause the
        // `blocking_reader_task_join_handle.await` below to deadlock).
        drop(controlled);

        // [WAIT 2] Wait for the reader task to complete.
        blocking_reader_task_join_handle.await.into_diagnostic()??;

        // Recreate status for return value.
        Ok(portable_pty::ExitStatus::with_exit_code(exit_code))
    }))
}
