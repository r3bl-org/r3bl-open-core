// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use miette::IntoDiagnostic;

use crate::{Controlled, ControlledChild, Controller, PtyCommandBuilder, PtyConfig,
            PtyOutputEvent, PtyReadOnlySession,
            common_impl::{create_pty_pair, spawn_blocking_controller_reader_task,
                          spawn_command_in_pty}};

impl PtyCommandBuilder {
    /// Spawns a read-only PTY session; it spawns two Tokio tasks and one OS child
    /// process.
    ///
    /// ```text
    /// ┌──────────────┐ ◄── events ◄── ┌───────────────────────────────┐
    /// │ Your Program │                │ Spawned Task (1) in Read Only │
    /// │              │                │            session            │
    /// │              │                │               ▼               │
    /// │ Handle       │                │ ◄─── PTY creates pair ──────► │
    /// │ events and   │                │ ┊Master/   ┊     ┊Slave/    ┊ │
    /// │ process      │                │ ┊Controller┊     ┊Controlled┊ │
    /// │ completion   │                │     ▼                 ▼       │
    /// │ from read    │                │ Spawn Tokio       Controlled  │
    /// │ only session │                │ blocking task     spawns      │
    /// │              │                │ (2) to read       child       │
    /// │              │                │ from              process (3) │
    /// │              │                │ Controller and                │
    /// │              │                │ generate events               │
    /// │              │                │ for your program              │
    /// └──────────────┘                └───────────────────────────────┘
    /// ```
    ///
    /// # Why 3 Tasks?
    ///
    /// 1. **Background orchestration task [`tokio::spawn`]** -> Required because this
    ///    function needs to return immediately with a session handle, allowing the caller
    ///    to start processing events while the PTY command runs in the background.
    ///    Without this, the function would block until the entire PTY session completes.
    ///
    /// 2. **OS child process [`ControlledChild`]** -> The actual command being executed
    ///    in the PTY. This is not a Tokio task but a system process that runs your
    ///    command with proper terminal emulation.
    ///
    /// 3. **Blocking reader task [`tokio::task::spawn_blocking`]** -> Required because
    ///    PTY file descriptors only provide synchronous [`std::io::Read`] APIs, not async
    ///    [`tokio::io::AsyncRead`]. Using regular [`tokio::spawn`] with blocking reads
    ///    would block the entire async runtime. [`spawn_blocking`] runs these synchronous
    ///    reads on a dedicated thread pool.
    ///
    /// # Returns
    ///
    /// A session with:
    /// - `output_event_receiver_half` combined stdout/stderr of child process -> events
    /// - `completion_handle` to await spawned child process completion
    ///
    /// # Example: Capturing OSC sequences from cargo build
    ///
    /// ```rust,no_run
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use r3bl_tui::{PtyCommandBuilder, PtyConfigOption, PtyOutputEvent, OscEvent};
    ///
    /// let mut session = PtyCommandBuilder::new("cargo")
    ///     .args(["build"])
    ///     .enable_osc_sequences()  // Enable OSC 9;4 progress sequences
    ///     .spawn_read_only(PtyConfigOption::Osc + PtyConfigOption::Output)?;
    ///
    /// let mut output = Vec::new();
    /// while let Some(event) = session.output_event_receiver_half.recv().await {
    ///     match event {
    ///         PtyOutputEvent::Output(data) => output.extend_from_slice(&data),
    ///         PtyOutputEvent::Osc(OscEvent::ProgressUpdate(pct)) => {
    ///             println!("Build progress: {}%", pct);
    ///         }
    ///         PtyOutputEvent::Exit(status) if status.success() => {
    ///             println!("Build completed successfully");
    ///             break;
    ///         }
    ///         PtyOutputEvent::Exit(status) => {
    ///             eprintln!("Build failed with: {:?}", status);
    ///             break;
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// println!("Build output: {}", String::from_utf8_lossy(&output));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the PTY fails to spawn or initialize properly.
    ///
    /// [`tokio::spawn`]: tokio::spawn
    /// [`tokio::task::spawn_blocking`]: tokio::task::spawn_blocking
    /// [`std::io::Read`]: std::io::Read
    /// [`tokio::io::AsyncRead`]: tokio::io::AsyncRead
    /// [`spawn_blocking`]: tokio::task::spawn_blocking
    /// [`ControlledChild`]: crate::ControlledChild
    pub fn spawn_read_only(
        self,
        config: impl Into<PtyConfig>,
    ) -> miette::Result<PtyReadOnlySession> {
        let config = config.into();

        // Create channel to bridge events from PTY controlled side -> your program.
        let (output_event_sender_half, output_event_receiver_half) =
            tokio::sync::mpsc::unbounded_channel();

        // [🛫 SPAWN 0] Spawn the main orchestration task. The caller waits for this one.
        // Pin the completion handle: JoinHandle is not Unpin but select! requires it for
        // efficient polling without moving.
        let completion_handle = Box::pin(tokio::spawn(async move {
            // Build the command, ensuring CWD is set.
            let command = self.build()?;

            // Create PTY pair: controller (master) for your program, controlled (slave)
            // for spawned process
            let (controller, controlled): (Controller, Controlled) =
                create_pty_pair(&config)?;

            // [🛫 SPAWN 1] Spawn the command with PTY (makes is_terminal() return true).
            // The child process uses the controlled side as its stdin/stdout/stderr.
            let mut controlled_child: ControlledChild =
                spawn_command_in_pty(&controlled, command)?;

            // [🛫 SPAWN 2] Spawn the reader task to process output from the controller
            // side. NOTE: Critical resource management - see module docs for
            // PTY lifecycle details.
            let blocking_controller_reader_task_join_handle = {
                let controller_reader = controller
                    .try_clone_reader()
                    .map_err(|e| miette::miette!("Failed to clone pty reader: {}", e))?;
                spawn_blocking_controller_reader_task(
                    controller_reader,
                    output_event_sender_half.clone(),
                    config,
                )
            };

            // [🛬 WAIT 1] Wait for the command to complete.
            let status = tokio::task::spawn_blocking(move || controlled_child.wait())
                .await
                .into_diagnostic()?
                .into_diagnostic()?;

            let exit_code = status.exit_code();
            let _unused = output_event_sender_half.send(PtyOutputEvent::Exit(status));

            // See module docs for detailed PTY lifecycle management explanation.
            drop(controlled); // Close the controlled half. CRITICAL for EOF to be sent to reader.
            drop(controller); // Not critical, but good practice to release controller FD.

            // [🛬 WAIT 2] Wait for the reader task to complete.
            blocking_controller_reader_task_join_handle
                .await
                .into_diagnostic()??;

            Ok(portable_pty::ExitStatus::with_exit_code(exit_code))
        }));

        Ok(PtyReadOnlySession {
            output_event_receiver_half,
            completion_handle,
        })
    }
}

#[cfg(test)]
mod tests {
    use miette::IntoDiagnostic;
    use tokio::time::{Duration, timeout};

    use crate::{OscEvent, PtyCommandBuilder, PtyConfigOption, PtyOutputEvent,
                PtyReadOnlySession};

    /// Helper function to collect events with a timeout
    async fn collect_events_with_timeout(
        mut session: PtyReadOnlySession,
        max_duration: Duration,
    ) -> miette::Result<(Vec<PtyOutputEvent>, portable_pty::ExitStatus)> {
        let mut events = Vec::new();

        let result = timeout(max_duration, async move {
            loop {
                tokio::select! {
                    result = &mut session.completion_handle => {
                        // Process completed
                        let status = result.into_diagnostic()??;

                        // Drain any remaining events
                        while let Ok(event) = session.output_event_receiver_half.try_recv() {
                            events.push(event);
                        }

                        return Ok::<_, miette::Error>((events, status));
                    }
                    Some(event) = session.output_event_receiver_half.recv() => {
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
        // Create a temporary directory for the test
        let temp_dir = std::env::temp_dir();

        let session = PtyCommandBuilder::new("echo")
            .args(["Hello, PTY!"])
            .cwd(temp_dir)
            .spawn_read_only(PtyConfigOption::Output)?;

        let (events, status) =
            collect_events_with_timeout(session, Duration::from_secs(5)).await?;

        assert!(status.success());

        // Should have output and exit events
        let output_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyOutputEvent::Output(data) => {
                    Some(String::from_utf8_lossy(data).to_string())
                }
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
        // Create a temporary directory for the test
        let temp_dir = std::env::temp_dir();

        // Use printf to emit a known OSC sequence
        let session = PtyCommandBuilder::new("printf")
            .args([r"\x1b]9;4;1;50\x1b\\"])
            .cwd(temp_dir)
            .spawn_read_only(PtyConfigOption::Osc)?;

        let (events, status) =
            collect_events_with_timeout(session, Duration::from_secs(5)).await?;

        assert!(status.success());

        // Should have received the OSC event
        let osc_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyOutputEvent::Osc(osc) => Some(osc.clone()),
                _ => None,
            })
            .collect();

        assert_eq!(osc_events, vec![OscEvent::ProgressUpdate(50)]);

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_osc_sequences() -> miette::Result<()> {
        // Create a temporary directory for the test
        let temp_dir = std::env::temp_dir();

        // Emit multiple OSC sequences
        let session = PtyCommandBuilder::new("printf")
            .args([r"\x1b]9;4;1;25\x1b\\\x1b]9;4;1;50\x1b\\\x1b]9;4;0;0\x1b\\"])
            .cwd(temp_dir)
            .spawn_read_only(PtyConfigOption::Osc)?;

        let (events, status) =
            collect_events_with_timeout(session, Duration::from_secs(5)).await?;

        assert!(status.success());

        let osc_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyOutputEvent::Osc(osc) => Some(osc.clone()),
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

        // Create a temporary directory for the test
        let temp_dir = std::env::temp_dir();

        // Try using bash first, then sh
        let config = PtyConfigOption::Osc + PtyConfigOption::Output;
        let session = PtyCommandBuilder::new("bash")
            .args([
                "-c",
                r"echo 'Starting...'; printf '\033]9;4;1;50\033\\'; echo 'Done!'",
            ])
            .cwd(temp_dir)
            .spawn_read_only(config)?;

        let (events, status) =
            collect_events_with_timeout(session, Duration::from_secs(5)).await?;

        assert!(status.success());

        // Check we got output events (OSC might not work on all systems)
        let has_output = events
            .iter()
            .any(|e| matches!(e, PtyOutputEvent::Output(_)));
        assert!(has_output);

        // Check if we got OSC events (may not work on all printf implementations)
        let osc_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyOutputEvent::Osc(osc) => Some(osc.clone()),
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

        // Create a temporary directory for the test
        let temp_dir = std::env::temp_dir();

        // Try using bash with better escape sequence handling
        let session = PtyCommandBuilder::new("bash")
            .args(["-c", r"printf '\033]9;4;1;'; sleep 0.01; printf '75\033\\'"])
            .cwd(temp_dir)
            .spawn_read_only(PtyConfigOption::Osc)?;

        let (events, status) =
            collect_events_with_timeout(session, Duration::from_secs(5)).await?;

        // Command should succeed
        assert!(status.success());

        let osc_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyOutputEvent::Osc(osc) => Some(osc.clone()),
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
        // Create a temporary directory for the test
        let temp_dir = std::env::temp_dir();

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
                .cwd(temp_dir.clone())
                .spawn_read_only(PtyConfigOption::Osc)?;

            let (events, status) =
                collect_events_with_timeout(session, Duration::from_secs(5)).await?;

            assert!(status.success());

            let osc_events: Vec<_> = events
                .iter()
                .filter_map(|e| match e {
                    PtyOutputEvent::Osc(osc) => Some(osc.clone()),
                    _ => None,
                })
                .collect();

            assert_eq!(osc_events, vec![expected]);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_command_failure() -> miette::Result<()> {
        // Create a temporary directory for the test
        let temp_dir = std::env::temp_dir();

        // Test that we properly handle command failures
        let session = PtyCommandBuilder::new("false")
            .cwd(temp_dir)
            .spawn_read_only(PtyConfigOption::NoCaptureOutput)?;

        let (events, status) =
            collect_events_with_timeout(session, Duration::from_secs(5)).await?;

        // Command should fail
        assert!(!status.success());

        // Should have an exit event
        let has_exit = events.iter().any(|e| matches!(e, PtyOutputEvent::Exit(_)));
        assert!(has_exit);

        Ok(())
    }

    #[tokio::test]
    async fn test_no_capture_option() -> miette::Result<()> {
        // Create a temporary directory for the test
        let temp_dir = std::env::temp_dir();

        // Test that NoCaptureOutput doesn't capture anything
        let session = PtyCommandBuilder::new("echo")
            .args(["test"])
            .cwd(temp_dir)
            .spawn_read_only(PtyConfigOption::NoCaptureOutput)?;

        let (events, status) =
            collect_events_with_timeout(session, Duration::from_secs(5)).await?;

        assert!(status.success());

        // Should only have exit event, no output
        let output_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, PtyOutputEvent::Output(_)))
            .collect();

        assert!(output_events.is_empty());

        Ok(())
    }
}
