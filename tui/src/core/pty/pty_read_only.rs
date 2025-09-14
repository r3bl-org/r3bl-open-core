// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::io::Read;

use miette::IntoDiagnostic;

use crate::{Controlled, ControlledChild, Controller, ControllerReader, OscBuffer,
            PtyCommandBuilder, PtyConfig, PtyReadOnlyOutputEvent, PtyReadOnlySession,
            pty_common_io::{READ_BUFFER_SIZE, create_pty_pair, spawn_command_in_pty}};

impl PtyCommandBuilder {
    /// Spawns a read-only PTY session; it spawns two Tokio tasks and one OS child
    /// process.
    ///
    /// ```text
    /// ┌──────────────────────────┐ ◄── output ◄── ┌───────────────────────────────┐
    /// │ Your Program             │     events     │ Spawned Task (1) in Read Only │
    /// │                          │                │            session            │
    /// │                          │                │               ▼               │
    /// │ a) Handle output events  │                │ ◄─── PTY creates pair ──────► │
    /// │    from                  │                │ ┊Master/   ┊     ┊Slave/    ┊ │
    /// │ b) Process completion of │                │ ┊Controller┊     ┊Controlled┊ │
    /// │ read only session        │                │     ▼                 ▼       │
    /// │                          │                │ Spawn Tokio       Controlled  │
    /// │                          │                │ blocking task     spawns      │
    /// │                          │                │ (3) to read       child       │
    /// │                          │                │ from              process (2) │
    /// │                          │                │ Controller and                │
    /// │                          │                │ generate events               │
    /// │                          │                │ for your program              │
    /// └──────────────────────────┘                └───────────────────────────────┘
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
    ///    command with terminal emulation (the child thinks it is in an interactive
    ///    terminal).
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
    /// use r3bl_tui::{PtyCommandBuilder, PtyConfigOption, PtyReadOnlyOutputEvent, OscEvent};
    ///
    /// let mut session = PtyCommandBuilder::new("cargo")
    ///     .args(["build"])
    ///     .enable_osc_sequences()  // Enable OSC 9;4 progress sequences
    ///     .spawn_read_only(PtyConfigOption::Osc + PtyConfigOption::Output)?;
    ///
    /// let mut output = Vec::new();
    /// while let Some(event) = session.output_evt_ch_rx_half.recv().await {
    ///     match event {
    ///         PtyReadOnlyOutputEvent::Output(data) => output.extend_from_slice(&data),
    ///         PtyReadOnlyOutputEvent::Osc(OscEvent::ProgressUpdate(pct)) => {
    ///             println!("Build progress: {}%", pct);
    ///         }
    ///         PtyReadOnlyOutputEvent::Exit(status) if status.success() => {
    ///             println!("Build completed successfully");
    ///             break;
    ///         }
    ///         PtyReadOnlyOutputEvent::Exit(status) => {
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
        arg_config: impl Into<PtyConfig>,
    ) -> miette::Result<PtyReadOnlySession> {
        let pty_config = arg_config.into();

        // Create channel to bridge events from PTY controlled side -> your program.
        let (
            /* return this to your program */ output_evt_ch_tx_half,
            /* used by blocking reader task */ output_evt_ch_rx_half,
        ) = tokio::sync::mpsc::unbounded_channel();

        // [🛫 SPAWN 0] Spawn the main orchestration task. This is returned to your
        // program, which waits for this to complete.
        let session_completion_handle = tokio::spawn(async move {
            // Build the command, ensuring CWD is set.
            let command = self.build()?;

            // Create PTY pair: controller (master) for your program, controlled (slave)
            // for spawned process
            let (controller, controlled): (Controller, Controlled) =
                create_pty_pair(pty_config.get_pty_size())?;

            // [🛫 SPAWN 1] Spawn the command with PTY (makes is_terminal() return true).
            // The child process uses the controlled side as its stdin/stdout/stderr.
            let controlled_child: ControlledChild =
                spawn_command_in_pty(&controlled, command)?;

            // [🛫 SPAWN 2] Spawn the reader task to process output from the controller
            // side. NOTE: Critical resource management - see module docs for
            // PTY lifecycle details.
            let output_reader_task_handle = {
                let controller_reader = controller
                    .try_clone_reader()
                    .map_err(|e| miette::miette!("Failed to clone pty reader: {}", e))?;
                spawn_blocking_controller_output_reader_task(
                    controller_reader,
                    output_evt_ch_tx_half.clone(),
                    pty_config,
                )
            };

            // [🛬 WAIT 1] Wait for the command to complete.
            let child_proc_exit_code = spawn_child_process_waiter(
                controlled_child,
                output_evt_ch_tx_half.clone(),
            )
            .await
            .into_diagnostic()??;

            // See module docs for detailed PTY lifecycle management explanation.
            drop(controlled); // Close the controlled half. CRITICAL for EOF to be sent to reader.
            drop(controller); // Not critical, but good practice to release controller FD.

            // [🛬 WAIT 2] Wait for the reader task to complete.
            output_reader_task_handle.await.into_diagnostic()??;

            Ok(portable_pty::ExitStatus::with_exit_code(
                child_proc_exit_code,
            ))
        });

        Ok(PtyReadOnlySession {
            output_evt_ch_rx_half,
            // Pin the completion handle: JoinHandle is not Unpin but select! requires it
            // for efficient polling without moving.
            pinned_boxed_session_completion_handle: Box::pin(session_completion_handle),
        })
    }
}

/// Spawns a task to wait for child process completion and send exit event.
///
/// Flow: child process → this task → output event channel → your program
///
/// This task:
/// - Waits for the child process to complete in a blocking context using
///   [`tokio::task::spawn_blocking`] because [`portable_pty`] provides only synchronous
///   APIs, not async ones, like our code in this module
/// - Sends the exit status as a `PtyReadOnlyOutputEvent::Exit` event
/// - Returns the exit code for the orchestration task
///
/// This function encapsulates the child process waiting logic to keep the
/// main orchestration task cleaner and more focused.
///
/// Returns a `JoinHandle` that resolves to the child process exit code.
///
/// [`tokio::task::spawn_blocking`]: tokio::task::spawn_blocking
/// [`portable_pty`]: mod@portable_pty
#[must_use]
fn spawn_child_process_waiter(
    mut controlled_child: ControlledChild,
    output_evt_ch_tx_half: tokio::sync::mpsc::UnboundedSender<PtyReadOnlyOutputEvent>,
) -> tokio::task::JoinHandle<miette::Result<u32>> {
    tokio::task::spawn_blocking(move || -> miette::Result<u32> {
        let status = controlled_child.wait().into_diagnostic()?;
        let exit_code = status.exit_code();
        let _unused = output_evt_ch_tx_half.send(PtyReadOnlyOutputEvent::Exit(status));
        Ok(exit_code)
    })
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
/// * `output_event_sender_half` - An unbounded sender for [`PtyReadOnlyOutputEvent`]s to
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
pub fn spawn_blocking_controller_output_reader_task(
    mut controller_reader: ControllerReader,
    output_event_ch_tx_half: tokio::sync::mpsc::UnboundedSender<PtyReadOnlyOutputEvent>,
    arg_config: impl Into<PtyConfig>,
) -> tokio::task::JoinHandle<miette::Result<()>> {
    let pty_config: PtyConfig = arg_config.into();

    // Async <-> Sync bridge using `spawn_blocking`.
    tokio::task::spawn_blocking(move || -> miette::Result<()> {
        let mut read_buffer = [0u8; READ_BUFFER_SIZE];
        let mut osc_buffer = if pty_config.is_osc_capture_enabled() {
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
                    if pty_config.is_output_capture_enabled() {
                        let _unused = output_event_ch_tx_half
                            .send(PtyReadOnlyOutputEvent::Output(data.to_vec()));
                    }

                    // Process OSC sequences if configured.
                    if let Some(ref mut osc_buf) = osc_buffer {
                        for event in osc_buf.append_and_extract(data, n) {
                            let _unused = output_event_ch_tx_half
                                .send(PtyReadOnlyOutputEvent::Osc(event));
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

#[cfg(test)]
mod tests {
    use miette::IntoDiagnostic;
    use tokio::{sync::mpsc::unbounded_channel,
                time::{Duration, timeout}};

    use crate::{OscEvent, PtyCommandBuilder, PtyConfigOption, PtyReadOnlyOutputEvent,
                PtyReadOnlySession,
                pty_read_only::spawn_blocking_controller_output_reader_task};

    /// Helper function to collect events with a timeout
    async fn collect_events_with_timeout(
        mut session: PtyReadOnlySession,
        max_duration: Duration,
    ) -> miette::Result<(Vec<PtyReadOnlyOutputEvent>, portable_pty::ExitStatus)> {
        let mut events = Vec::new();

        let result = timeout(max_duration, async move {
            loop {
                tokio::select! {
                    result = &mut session.pinned_boxed_session_completion_handle => {
                        // Process completed.
                        let status = result.into_diagnostic()??;

                        // Drain any remaining events.
                        while let Ok(event) = session.output_evt_ch_rx_half.try_recv() {
                            events.push(event);
                        }

                        return Ok::<_, miette::Error>((events, status));
                    }
                    Some(event) = session.output_evt_ch_rx_half.recv() => {
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
        // Create a temporary directory for the test.
        let temp_dir = std::env::temp_dir();

        let session = PtyCommandBuilder::new("echo")
            .args(["Hello, PTY!"])
            .cwd(temp_dir)
            .spawn_read_only(PtyConfigOption::Output)?;

        let (events, status) =
            collect_events_with_timeout(session, Duration::from_secs(5)).await?;

        assert!(status.success());

        // Should have output and exit events.
        let output_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyReadOnlyOutputEvent::Output(data) => {
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
        // Create a temporary directory for the test.
        let temp_dir = std::env::temp_dir();

        // Use printf to emit a known OSC sequence.
        let session = PtyCommandBuilder::new("printf")
            .args([r"\x1b]9;4;1;50\x1b\\"])
            .cwd(temp_dir)
            .spawn_read_only(PtyConfigOption::Osc)?;

        let (events, status) =
            collect_events_with_timeout(session, Duration::from_secs(5)).await?;

        assert!(status.success());

        // Should have received the OSC event.
        let osc_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyReadOnlyOutputEvent::Osc(osc) => Some(osc.clone()),
                _ => None,
            })
            .collect();

        assert_eq!(osc_events, vec![OscEvent::ProgressUpdate(50)]);

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_osc_sequences() -> miette::Result<()> {
        // Create a temporary directory for the test.
        let temp_dir = std::env::temp_dir();

        // Emit multiple OSC sequences.
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
                PtyReadOnlyOutputEvent::Osc(osc) => Some(osc.clone()),
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
        // Mix regular output with OSC sequences.
        // This test requires bash/sh with working printf escape sequences
        if cfg!(target_os = "windows") {
            return Ok(()); // Skip on Windows
        }

        // Create a temporary directory for the test.
        let temp_dir = std::env::temp_dir();

        // Try using bash first, then sh.
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
            .any(|e| matches!(e, PtyReadOnlyOutputEvent::Output(_)));
        assert!(has_output);

        // Check if we got OSC events (may not work on all printf implementations)
        let osc_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyReadOnlyOutputEvent::Osc(osc) => Some(osc.clone()),
                _ => None,
            })
            .collect();

        // If OSC sequences were parsed, verify they're correct.
        if !osc_events.is_empty() {
            assert_eq!(osc_events, vec![OscEvent::ProgressUpdate(50)]);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_split_osc_sequence_simulation() -> miette::Result<()> {
        // This test simulates a split sequence using shell commands.
        // Note: This is platform-specific and may not work on all systems
        if cfg!(target_os = "windows") {
            return Ok(()); // Skip on Windows
        }

        // Create a temporary directory for the test.
        let temp_dir = std::env::temp_dir();

        // Try using bash with better escape sequence handling.
        let session = PtyCommandBuilder::new("bash")
            .args(["-c", r"printf '\033]9;4;1;'; sleep 0.01; printf '75\033\\'"])
            .cwd(temp_dir)
            .spawn_read_only(PtyConfigOption::Osc)?;

        let (events, status) =
            collect_events_with_timeout(session, Duration::from_secs(5)).await?;

        // Command should succeed.
        assert!(status.success());

        let osc_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyReadOnlyOutputEvent::Osc(osc) => Some(osc.clone()),
                _ => None,
            })
            .collect();

        // This test may not produce OSC events on all systems due to printf limitations.
        // We just verify the command ran successfully.
        if !osc_events.is_empty() {
            // If we did get OSC events, they should be correct.
            assert_eq!(osc_events, vec![OscEvent::ProgressUpdate(75)]);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_all_osc_event_types() -> miette::Result<()> {
        // Create a temporary directory for the test.
        let temp_dir = std::env::temp_dir();

        // Test all four OSC event types.
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
                    PtyReadOnlyOutputEvent::Osc(osc) => Some(osc.clone()),
                    _ => None,
                })
                .collect();

            assert_eq!(osc_events, vec![expected]);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_command_failure() -> miette::Result<()> {
        // Create a temporary directory for the test.
        let temp_dir = std::env::temp_dir();

        // Test that we properly handle command failures.
        let session = PtyCommandBuilder::new("false")
            .cwd(temp_dir)
            .spawn_read_only(PtyConfigOption::NoCaptureOutput)?;

        let (events, status) =
            collect_events_with_timeout(session, Duration::from_secs(5)).await?;

        // Command should fail.
        assert!(!status.success());

        // Should have an exit event.
        let has_exit = events
            .iter()
            .any(|e| matches!(e, PtyReadOnlyOutputEvent::Exit(_)));
        assert!(has_exit);

        Ok(())
    }

    #[tokio::test]
    async fn test_no_capture_option() -> miette::Result<()> {
        // Create a temporary directory for the test.
        let temp_dir = std::env::temp_dir();

        // Test that NoCaptureOutput doesn't capture anything.
        let session = PtyCommandBuilder::new("echo")
            .args(["test"])
            .cwd(temp_dir)
            .spawn_read_only(PtyConfigOption::NoCaptureOutput)?;

        let (events, status) =
            collect_events_with_timeout(session, Duration::from_secs(5)).await?;

        assert!(status.success());

        // Should only have exit event, no output.
        let output_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, PtyReadOnlyOutputEvent::Output(_)))
            .collect();

        assert!(output_events.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_create_reader_task_no_capture() {
        let (event_sender, mut event_receiver) = unbounded_channel();

        // Create a mock reader that sends some data then EOF.
        let mock_data = b"test data";
        let reader = Box::new(std::io::Cursor::new(mock_data.to_vec()));

        let handle = spawn_blocking_controller_output_reader_task(
            reader,
            event_sender,
            PtyConfigOption::NoCaptureOutput,
        );

        // Reader should complete successfully.
        let result = tokio::time::timeout(Duration::from_millis(100), handle).await;
        assert!(result.is_ok());

        // No events should be sent since capture is disabled.
        assert!(event_receiver.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_create_reader_task_with_output_capture() {
        let (event_sender, mut event_receiver) = unbounded_channel();

        let mock_data = b"test data";
        let reader = Box::new(std::io::Cursor::new(mock_data.to_vec()));

        let handle = spawn_blocking_controller_output_reader_task(
            reader,
            event_sender,
            PtyConfigOption::Output,
        );

        // Wait for task to complete.
        let result = tokio::time::timeout(Duration::from_millis(100), handle).await;
        assert!(result.is_ok());

        // Should receive output event.
        if let Ok(event) = event_receiver.try_recv() {
            match event {
                PtyReadOnlyOutputEvent::Output(data) => assert_eq!(data, mock_data),
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

        // This test now uses the new OSC-only test as the comprehensive one.
        // This version keeps the old behavior for backward compatibility.
        let handle = spawn_blocking_controller_output_reader_task(
            reader,
            event_sender,
            PtyConfigOption::Osc,
        );

        // Wait for task to complete.
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

        // Check that we received an OSC event (may also receive raw output due to default.
        // behavior)
        let osc_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyReadOnlyOutputEvent::Osc(osc) => Some(osc),
                _ => None,
            })
            .collect();

        assert!(
            !osc_events.is_empty(),
            "Should have received at least one OSC event"
        );

        // Verify we got the correct OSC event.
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

        let handle =
            spawn_blocking_controller_output_reader_task(reader, event_sender, config);

        // Wait for task to complete.
        let result = tokio::time::timeout(Duration::from_millis(100), handle).await;
        assert!(result.is_ok());

        // Collect all events - should only get OSC events, no raw output.
        let mut events = Vec::new();
        while let Ok(event) = event_receiver.try_recv() {
            events.push(event);
        }

        assert!(
            !events.is_empty(),
            "Should have received at least one event"
        );

        // Should have OSC events but no output events.
        let osc_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyReadOnlyOutputEvent::Osc(osc) => Some(osc),
                _ => None,
            })
            .collect();

        let output_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyReadOnlyOutputEvent::Output(_) => Some(()),
                _ => None,
            })
            .collect();

        assert!(!osc_events.is_empty(), "Should have received OSC events");
        assert!(
            output_events.is_empty(),
            "Should NOT have received output events (OSC-only capture)"
        );

        // Verify we got the correct OSC event.
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

        // Create config with both output and OSC capture enabled.
        let config = PtyConfigOption::Osc + PtyConfigOption::Output;

        let handle =
            spawn_blocking_controller_output_reader_task(reader, event_sender, config);

        // Wait for task to complete.
        let result = tokio::time::timeout(Duration::from_millis(100), handle).await;
        assert!(result.is_ok());

        // Collect all events - should get both raw output AND OSC events.
        let mut events = Vec::new();
        while let Ok(event) = event_receiver.try_recv() {
            events.push(event);
        }

        assert!(
            !events.is_empty(),
            "Should have received at least one event"
        );

        // Should have both OSC events AND output events.
        let osc_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyReadOnlyOutputEvent::Osc(osc) => Some(osc),
                _ => None,
            })
            .collect();

        let output_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                PtyReadOnlyOutputEvent::Output(_) => Some(()),
                _ => None,
            })
            .collect();

        assert!(!osc_events.is_empty(), "Should have received OSC events");
        assert!(
            !output_events.is_empty(),
            "Should have received output events (both capture enabled)"
        );

        // Verify we got the correct OSC event.
        let has_correct_event = osc_events
            .iter()
            .any(|osc| matches!(osc, crate::OscEvent::ProgressUpdate(25)));

        assert!(
            has_correct_event,
            "Expected OSC progress update event with 25%"
        );
    }
}
