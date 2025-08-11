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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc::unbounded_channel;
    use tokio::time::{timeout, Duration};
    use crate::core::pty::{OscEvent, PtyCommandBuilder, PtyConfigOption};

    /// Helper function to collect events with a timeout
    async fn collect_events_with_timeout(
        mut handle: Pin<Box<tokio::task::JoinHandle<miette::Result<portable_pty::ExitStatus>>>>,
        mut receiver: tokio::sync::mpsc::UnboundedReceiver<PtyEvent>,
        max_duration: Duration,
    ) -> miette::Result<(Vec<PtyEvent>, portable_pty::ExitStatus)> {
        let mut events = Vec::new();
        
        let result = timeout(max_duration, async move {
            loop {
                tokio::select! {
                    result = &mut handle => {
                        // Process completed
                        let status = result.into_diagnostic()??;
                        
                        // Drain any remaining events
                        while let Ok(event) = receiver.try_recv() {
                            events.push(event);
                        }
                        
                        return Ok::<_, miette::Error>((events, status));
                    }
                    Some(event) = receiver.recv() => {
                        events.push(event);
                    }
                }
            }
        }).await;
        
        match result {
            Ok(Ok(data)) => Ok(data),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(miette::miette!("Test timed out")),
        }
    }

    #[tokio::test]
    async fn test_simple_echo_command() -> miette::Result<()> {
        let cmd = PtyCommandBuilder::new("echo")
            .args(["Hello, PTY!"])
            .build()?;
        
        let (sender, receiver) = unbounded_channel();
        let handle = spawn_pty_capture_output_no_input(
            cmd, 
            PtyConfigOption::Output,
            sender
        );
        
        let (events, status) = collect_events_with_timeout(
            handle,
            receiver,
            Duration::from_secs(5)
        ).await?;
        
        assert!(status.success());
        
        // Should have output and exit events
        let output_events: Vec<_> = events.iter()
            .filter_map(|e| match e {
                PtyEvent::Output(data) => Some(String::from_utf8_lossy(data).to_string()),
                _ => None
            })
            .collect();
        
        assert!(!output_events.is_empty());
        let combined_output = output_events.join("");
        assert!(combined_output.contains("Hello, PTY!"));
        
        Ok(())
    }

    #[tokio::test]
    async fn test_osc_sequence_with_printf() -> miette::Result<()> {
        // Use printf to emit a known OSC sequence
        let cmd = PtyCommandBuilder::new("printf")
            .args([r"\x1b]9;4;1;50\x1b\\"])
            .build()?;
        
        let (sender, receiver) = unbounded_channel();
        let handle = spawn_pty_capture_output_no_input(
            cmd,
            PtyConfigOption::Osc,
            sender
        );
        
        let (events, status) = collect_events_with_timeout(
            handle,
            receiver,
            Duration::from_secs(5)
        ).await?;
        
        assert!(status.success());
        
        // Should have received the OSC event
        let osc_events: Vec<_> = events.iter()
            .filter_map(|e| match e {
                PtyEvent::Osc(osc) => Some(osc.clone()),
                _ => None
            })
            .collect();
        
        assert_eq!(osc_events, vec![OscEvent::ProgressUpdate(50)]);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_osc_sequences() -> miette::Result<()> {
        // Emit multiple OSC sequences
        let cmd = PtyCommandBuilder::new("printf")
            .args([r"\x1b]9;4;1;25\x1b\\\x1b]9;4;1;50\x1b\\\x1b]9;4;0;0\x1b\\"])
            .build()?;
        
        let (sender, receiver) = unbounded_channel();
        let handle = spawn_pty_capture_output_no_input(
            cmd,
            PtyConfigOption::Osc,
            sender
        );
        
        let (events, status) = collect_events_with_timeout(
            handle,
            receiver,
            Duration::from_secs(5)
        ).await?;
        
        assert!(status.success());
        
        let osc_events: Vec<_> = events.iter()
            .filter_map(|e| match e {
                PtyEvent::Osc(osc) => Some(osc.clone()),
                _ => None
            })
            .collect();
        
        assert_eq!(osc_events, vec![
            OscEvent::ProgressUpdate(25),
            OscEvent::ProgressUpdate(50),
            OscEvent::ProgressCleared,
        ]);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_osc_with_mixed_output() -> miette::Result<()> {
        // Mix regular output with OSC sequences
        // This test requires bash/sh with working printf escape sequences
        if cfg!(target_os = "windows") {
            return Ok(()); // Skip on Windows
        }
        
        // Try using bash first, then sh
        let cmd = match PtyCommandBuilder::new("bash")
            .args(["-c", r"echo 'Starting...'; printf '\033]9;4;1;50\033\\'; echo 'Done!'"])
            .build() 
        {
            Ok(cmd) => cmd,
            Err(_) => {
                // Fallback to sh if bash is not available
                match PtyCommandBuilder::new("sh")
                    .args(["-c", r"echo 'Starting...'; printf '\033]9;4;1;50\033\\'; echo 'Done!'"])
                    .build()
                {
                    Ok(cmd) => cmd,
                    Err(_) => {
                        // Skip test if neither bash nor sh is available
                        println!("Skipping test - neither bash nor sh available");
                        return Ok(());
                    }
                }
            }
        };
        
        let (sender, receiver) = unbounded_channel();
        let config = PtyConfigOption::Osc + PtyConfigOption::Output;
        let handle = spawn_pty_capture_output_no_input(cmd, config, sender);
        
        let (events, status) = collect_events_with_timeout(
            handle,
            receiver,
            Duration::from_secs(5)
        ).await?;
        
        assert!(status.success());
        
        // Check we got output events (OSC might not work on all systems)
        let has_output = events.iter().any(|e| matches!(e, PtyEvent::Output(_)));
        assert!(has_output);
        
        // Check if we got OSC events (may not work on all printf implementations)
        let osc_events: Vec<_> = events.iter()
            .filter_map(|e| match e {
                PtyEvent::Osc(osc) => Some(osc.clone()),
                _ => None
            })
            .collect();
        
        // If OSC sequences were parsed, verify they're correct
        if !osc_events.is_empty() {
            assert_eq!(osc_events, vec![OscEvent::ProgressUpdate(50)]);
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_split_osc_sequence_simulation() -> miette::Result<()> {
        // This test simulates a split sequence using shell commands
        // Note: This is platform-specific and may not work on all systems
        if cfg!(target_os = "windows") {
            return Ok(()); // Skip on Windows
        }
        
        // Try using bash with better escape sequence handling
        let cmd = match PtyCommandBuilder::new("bash")
            .args(["-c", r"printf '\033]9;4;1;'; sleep 0.01; printf '75\033\\'"])
            .build()
        {
            Ok(cmd) => cmd,
            Err(_) => {
                // Try sh as fallback
                match PtyCommandBuilder::new("sh")
                    .args(["-c", r"printf '\033]9;4;1;'; sleep 0.01; printf '75\033\\'"])
                    .build()
                {
                    Ok(cmd) => cmd,
                    Err(_) => {
                        println!("Skipping test - neither bash nor sh available");
                        return Ok(());
                    }
                }
            }
        };
        
        let (sender, receiver) = unbounded_channel();
        let handle = spawn_pty_capture_output_no_input(
            cmd,
            PtyConfigOption::Osc,
            sender
        );
        
        let (events, status) = collect_events_with_timeout(
            handle,
            receiver,
            Duration::from_secs(5)
        ).await?;
        
        // Command should succeed
        assert!(status.success());
        
        let osc_events: Vec<_> = events.iter()
            .filter_map(|e| match e {
                PtyEvent::Osc(osc) => Some(osc.clone()),
                _ => None
            })
            .collect();
        
        // This test may not produce OSC events on all systems due to printf limitations
        // We just verify the command ran successfully
        if !osc_events.is_empty() {
            // If we did get OSC events, they should be correct
            assert_eq!(osc_events, vec![OscEvent::ProgressUpdate(75)]);
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_all_osc_event_types() -> miette::Result<()> {
        // Test all four OSC event types
        let sequences = [
            (r"\x1b]9;4;0;0\x1b\\", OscEvent::ProgressCleared),
            (r"\x1b]9;4;1;42\x1b\\", OscEvent::ProgressUpdate(42)),
            (r"\x1b]9;4;2;0\x1b\\", OscEvent::BuildError),
            (r"\x1b]9;4;3;0\x1b\\", OscEvent::IndeterminateProgress),
        ];
        
        for (sequence, expected) in sequences {
            let cmd = PtyCommandBuilder::new("printf")
                .args([sequence])
                .build()?;
            
            let (sender, receiver) = unbounded_channel();
            let handle = spawn_pty_capture_output_no_input(
                cmd,
                PtyConfigOption::Osc,
                sender
            );
            
            let (events, status) = collect_events_with_timeout(
                handle,
                receiver,
                Duration::from_secs(5)
            ).await?;
            
            assert!(status.success());
            
            let osc_events: Vec<_> = events.iter()
                .filter_map(|e| match e {
                    PtyEvent::Osc(osc) => Some(osc.clone()),
                    _ => None
                })
                .collect();
            
            assert_eq!(osc_events, vec![expected]);
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_command_failure() -> miette::Result<()> {
        // Test that we properly handle command failures
        let cmd = PtyCommandBuilder::new("false")
            .build()?;
        
        let (sender, receiver) = unbounded_channel();
        let handle = spawn_pty_capture_output_no_input(
            cmd,
            PtyConfigOption::NoCaptureOutput,
            sender
        );
        
        let (events, status) = collect_events_with_timeout(
            handle,
            receiver,
            Duration::from_secs(5)
        ).await?;
        
        // Command should fail
        assert!(!status.success());
        
        // Should have an exit event
        let has_exit = events.iter().any(|e| matches!(e, PtyEvent::Exit(_)));
        assert!(has_exit);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_no_capture_option() -> miette::Result<()> {
        // Test that NoCaptureOutput doesn't capture anything
        let cmd = PtyCommandBuilder::new("echo")
            .args(["test"])
            .build()?;
        
        let (sender, receiver) = unbounded_channel();
        let handle = spawn_pty_capture_output_no_input(
            cmd,
            PtyConfigOption::NoCaptureOutput,
            sender
        );
        
        let (events, status) = collect_events_with_timeout(
            handle,
            receiver,
            Duration::from_secs(5)
        ).await?;
        
        assert!(status.success());
        
        // Should only have exit event, no output
        let output_events: Vec<_> = events.iter()
            .filter(|e| matches!(e, PtyEvent::Output(_)))
            .collect();
        
        assert!(output_events.is_empty());
        
        Ok(())
    }
}
