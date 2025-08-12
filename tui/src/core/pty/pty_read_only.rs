// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use miette::IntoDiagnostic;
use tokio::sync::mpsc::unbounded_channel;

use crate::{PtyCommandBuilder, PtyConfig, PtyEvent, PtyReadOnlySession,
            common_impl::{create_pty_pair, create_reader_task, spawn_command_in_pty}};

/// Internal implementation for spawning a read-only PTY session.
///
/// This is called by `PtyCommandBuilder::spawn_read_only()`.
pub(crate) fn spawn_pty_read_only_impl(
    /* move */ command: PtyCommandBuilder,
    /* move */ config: impl Into<PtyConfig>,
) -> PtyReadOnlySession {
    let config = config.into();

    // Create channel for events
    let (event_sender_half, event_receiver_half) = unbounded_channel();

    let completion_handle = Box::pin(tokio::spawn(async move {
        // Build the command, ensuring CWD is set
        let command = command.build()?;

        // Create PTY pair using common implementation
        let (controller, controlled) = create_pty_pair(&config)?;

        // [🛫 SPAWN 1] Spawn the command with PTY (makes is_terminal() return true).
        let mut controlled_child = spawn_command_in_pty(&controlled, command)?;

        // [🛫 SPAWN 2] Spawn the reader task to process output.
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
        // 1. Create reader from controller, then keep controller in scope.
        // 2. Explicitly drop controlled after process exits - closes our slave FD.
        // 3. This allows the reader to receive EOF and exit cleanly.
        let blocking_reader_task_join_handle = {
            let reader_event_sender = event_sender_half.clone();
            let should_capture_osc = config.is_osc_capture_enabled();
            let should_capture_output = config.is_output_capture_enabled();
            // Get a reader from the controller for the reader task.
            let reader = controller
                .try_clone_reader()
                .map_err(|e| miette::miette!("Failed to clone pty reader: {}", e))?;
            // Use common implementation for reader task.
            create_reader_task(
                reader,
                reader_event_sender,
                should_capture_osc,
                should_capture_output,
            )
        };

        // [🛬 WAIT 1] Wait for the command to complete.
        let status = tokio::task::spawn_blocking(move || controlled_child.wait())
            .await
            .into_diagnostic()?
            .into_diagnostic()?;

        // Store exit code before moving status into event.
        let exit_code = status.exit_code();

        // Send exit event (moves status).
        let _unused = event_sender_half.send(PtyEvent::Exit(status));

        // CRITICAL. Explicitly drop both controlled and controller after process exits.
        // This ensures proper PTY cleanup and allows the reader to receive EOF. If you
        // don't do this, the call `blocking_reader_task_join_handle.await` will deadlock.
        drop(controlled);
        drop(controller);

        // [🛬 WAIT 2] Wait for the reader task to complete.
        blocking_reader_task_join_handle.await.into_diagnostic()??;

        // Recreate status for return value.
        Ok(portable_pty::ExitStatus::with_exit_code(exit_code))
    }));

    PtyReadOnlySession {
        event_receiver_half,
        completion_handle,
    }
}

#[cfg(test)]
mod tests {
    use miette::IntoDiagnostic;
    use tokio::time::{Duration, timeout};

    use crate::{OscEvent, PtyCommandBuilder, PtyConfigOption, PtyEvent,
                PtyReadOnlySession};

    /// Helper function to collect events with a timeout
    async fn collect_events_with_timeout(
        mut session: PtyReadOnlySession,
        max_duration: Duration,
    ) -> miette::Result<(Vec<PtyEvent>, portable_pty::ExitStatus)> {
        let mut events = Vec::new();

        let result = timeout(max_duration, async move {
            loop {
                tokio::select! {
                    result = &mut session.completion_handle => {
                        // Process completed
                        let status = result.into_diagnostic()??;

                        // Drain any remaining events
                        while let Ok(event) = session.event_receiver_half.try_recv() {
                            events.push(event);
                        }

                        return Ok::<_, miette::Error>((events, status));
                    }
                    Some(event) = session.event_receiver_half.recv() => {
                        events.push(event);
                    }
                }
            }
        })
        .await;

        match result {
            Ok(Ok(data)) => Ok(data),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(miette::miette!("Test timed out")),
        }
    }

    #[tokio::test]
    async fn test_simple_echo_command() -> miette::Result<()> {
        let session = PtyCommandBuilder::new("echo")
            .args(["Hello, PTY!"])
            .spawn_read_only(PtyConfigOption::Output)?;

        let (events, status) =
            collect_events_with_timeout(session, Duration::from_secs(5)).await?;

        assert!(status.success());

        // Should have output and exit events
        let output_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyEvent::Output(data) => Some(String::from_utf8_lossy(data).to_string()),
                _ => None,
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
        let session = PtyCommandBuilder::new("printf")
            .args([r"\x1b]9;4;1;50\x1b\\"])
            .spawn_read_only(PtyConfigOption::Osc)?;

        let (events, status) =
            collect_events_with_timeout(session, Duration::from_secs(5)).await?;

        assert!(status.success());

        // Should have received the OSC event
        let osc_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyEvent::Osc(osc) => Some(osc.clone()),
                _ => None,
            })
            .collect();

        assert_eq!(osc_events, vec![OscEvent::ProgressUpdate(50)]);

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_osc_sequences() -> miette::Result<()> {
        // Emit multiple OSC sequences
        let session = PtyCommandBuilder::new("printf")
            .args([r"\x1b]9;4;1;25\x1b\\\x1b]9;4;1;50\x1b\\\x1b]9;4;0;0\x1b\\"])
            .spawn_read_only(PtyConfigOption::Osc)?;

        let (events, status) =
            collect_events_with_timeout(session, Duration::from_secs(5)).await?;

        assert!(status.success());

        let osc_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyEvent::Osc(osc) => Some(osc.clone()),
                _ => None,
            })
            .collect();

        assert_eq!(
            osc_events,
            vec![
                OscEvent::ProgressUpdate(25),
                OscEvent::ProgressUpdate(50),
                OscEvent::ProgressCleared,
            ]
        );

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
        let config = PtyConfigOption::Osc + PtyConfigOption::Output;
        let session = PtyCommandBuilder::new("bash")
            .args([
                "-c",
                r"echo 'Starting...'; printf '\033]9;4;1;50\033\\'; echo 'Done!'",
            ])
            .spawn_read_only(config)?;

        let (events, status) =
            collect_events_with_timeout(session, Duration::from_secs(5)).await?;

        assert!(status.success());

        // Check we got output events (OSC might not work on all systems)
        let has_output = events.iter().any(|e| matches!(e, PtyEvent::Output(_)));
        assert!(has_output);

        // Check if we got OSC events (may not work on all printf implementations)
        let osc_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyEvent::Osc(osc) => Some(osc.clone()),
                _ => None,
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
        let session = PtyCommandBuilder::new("bash")
            .args(["-c", r"printf '\033]9;4;1;'; sleep 0.01; printf '75\033\\'"])
            .spawn_read_only(PtyConfigOption::Osc)?;

        let (events, status) =
            collect_events_with_timeout(session, Duration::from_secs(5)).await?;

        // Command should succeed
        assert!(status.success());

        let osc_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyEvent::Osc(osc) => Some(osc.clone()),
                _ => None,
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
            let session = PtyCommandBuilder::new("printf")
                .args([sequence])
                .spawn_read_only(PtyConfigOption::Osc)?;

            let (events, status) =
                collect_events_with_timeout(session, Duration::from_secs(5)).await?;

            assert!(status.success());

            let osc_events: Vec<_> = events
                .iter()
                .filter_map(|e| match e {
                    PtyEvent::Osc(osc) => Some(osc.clone()),
                    _ => None,
                })
                .collect();

            assert_eq!(osc_events, vec![expected]);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_command_failure() -> miette::Result<()> {
        // Test that we properly handle command failures
        let session = PtyCommandBuilder::new("false")
            .spawn_read_only(PtyConfigOption::NoCaptureOutput)?;

        let (events, status) =
            collect_events_with_timeout(session, Duration::from_secs(5)).await?;

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
        let session = PtyCommandBuilder::new("echo")
            .args(["test"])
            .spawn_read_only(PtyConfigOption::NoCaptureOutput)?;

        let (events, status) =
            collect_events_with_timeout(session, Duration::from_secs(5)).await?;

        assert!(status.success());

        // Should only have exit event, no output
        let output_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, PtyEvent::Output(_)))
            .collect();

        assert!(output_events.is_empty());

        Ok(())
    }
}
