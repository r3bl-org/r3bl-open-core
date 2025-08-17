// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use miette::IntoDiagnostic;

use crate::{PtyCommandBuilder, PtyConfig, PtyInputEvent, PtyOutputEvent,
            PtyReadWriteSession,
            pty_common_io::{create_input_handler_task, create_pty_pair,
                            spawn_blocking_controller_reader_task, spawn_command_in_pty}};

impl PtyCommandBuilder {
    /// Spawns a read-write PTY session; it spawns three Tokio tasks and one OS child
    /// process with bidirectional communication.
    ///
    /// ```text
    /// ┌──────────────────────────┐ ◄── output ◄── ┌───────────────────────────────┐
    /// │ Your Program             │     events     │ Spawned Task (1) in Read      │
    /// │                          │                │          Write session        │
    /// │                          │ ──► input ───► │               ▼               │
    /// │ a) Handle output events  │     events     │ ◄─── PTY creates pair ──────► │
    /// │    from                  │                │ ┊Master/   ┊     ┊Slave/    ┊ │
    /// │ b) Send input events to  │                │ ┊Controller┊     ┊Controlled┊ │
    /// │ c) Process completion of │                │     ▼                 ▼       │
    /// │ read/write session       │                │ Spawn Tokio       Controlled  │
    /// │                          │                │ blocking task     spawns      │
    /// │                          │                │ (3) to read       child       │
    /// │                          │                │ from              process (2) │
    /// │                          │                │ Controller and    + Spawn     │
    /// │                          │                │ generate events   bridge      │
    /// │                          │                │ for your program  task (4)    │
    /// │                          │                │                   for input   │
    /// └──────────────────────────┘                └───────────────────────────────┘
    /// ```
    ///
    /// # Why 4 Tasks?
    ///
    /// 1. **Background orchestration task [`tokio::spawn`]** -> Required because this
    ///    function needs to return immediately with a session handle, allowing the caller
    ///    to start processing events and sending input while the PTY command runs in the
    ///    background. Without this, the function would block until the entire PTY session
    ///    completes.
    ///
    /// 2. **OS child process [`crate::ControlledChild`]** -> The actual command being
    ///    executed in the PTY. This is not a Tokio task but a system process that runs
    ///    your command with terminal emulation (the child thinks it is in an interactive
    ///    terminal).
    ///
    /// 3. **Blocking reader task [`tokio::task::spawn_blocking`]** -> Required because
    ///    PTY file descriptors only provide synchronous [`std::io::Read`] APIs, not async
    ///    [`tokio::io::AsyncRead`]. Using regular [`tokio::spawn`] with blocking reads
    ///    would block the entire async runtime. [`spawn_blocking`] runs these synchronous
    ///    reads on a dedicated thread pool.
    ///
    /// 4. **Bridge task [`tokio::spawn`]** -> Unique to read-write mode (the 3 above are
    ///    the same for read-only mode). Converts async input from your program to sync
    ///    channel for the blocking input handler. This enables bidirectional
    ///    communication while maintaining proper async/sync boundaries. The bridge task
    ///    serves as an async-to-sync adapter, necessary because `portable_pty` only
    ///    provides synchronous I/O APIs, while the input handler must run in
    ///    `spawn_blocking` context to avoid blocking the tokio runtime. Without this
    ///    bridge, async code couldn't send input to the synchronous PTY writer.
    ///
    /// # Design Decisions
    ///
    /// ## Dumb Pipes Approach
    ///
    /// The API treats input and output channels as dumb pipes of events, making no
    /// assumptions about the child process. The child determines terminal modes
    /// (cooked/raw), interprets environment variables, and handles all
    /// terminal-specific behavior. We simply provide the transport layer.
    ///
    /// ## Single Input Handler Architecture
    ///
    /// A single task owns the [`portable_pty::MasterPty`] and handles all input
    /// operations including resize. This avoids complex synchronization and ensures
    /// clean resource management.
    ///
    /// ## Error Handling
    ///
    /// Write errors terminate the session (no automatic retry). Errors are reported via
    /// [`PtyOutputEvent::WriteError`] before termination. Three termination scenarios:
    /// 1. Child process self-terminates (normal or crash)
    /// 2. Explicit session termination via [`PtyInputEvent::Close`]
    /// 3. Unexpected termination (reported as [`PtyOutputEvent::UnexpectedExit`] event)
    ///
    /// # Returns
    ///
    /// A session with:
    /// - `input_event_sender_half` for sending input events to the PTY
    /// - `output_event_receiver_half` combined stdout/stderr of child process -> events
    /// - `completion_handle` to await spawned child process completion
    ///
    /// # Example: Interactive shell session with input/output
    ///
    /// ```rust,no_run
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use r3bl_tui::{PtyCommandBuilder, PtyConfigOption, PtyOutputEvent, PtyInputEvent, ControlChar};
    /// use tokio::time::{sleep, Duration};
    ///
    /// // Start an interactive shell
    /// let mut session = PtyCommandBuilder::new("sh")
    ///     .spawn_read_write(PtyConfigOption::Output)?;
    ///
    /// // Send commands to the shell
    /// session.input_event_sender_half.send(PtyInputEvent::WriteLine("echo 'Hello from shell'".into()))?;
    /// session.input_event_sender_half.send(PtyInputEvent::WriteLine("date".into()))?;
    /// session.input_event_sender_half.send(PtyInputEvent::WriteLine("exit".into()))?;
    ///
    /// // Process output events
    /// let mut output = Vec::new();
    /// while let Some(event) = session.output_event_receiver_half.recv().await {
    ///     match event {
    ///         PtyOutputEvent::Output(data) => {
    ///             output.extend_from_slice(&data);
    ///             print!("{}", String::from_utf8_lossy(&data));
    ///         }
    ///         PtyOutputEvent::Exit(status) if status.success() => {
    ///             println!("Shell session completed successfully");
    ///             break;
    ///         }
    ///         PtyOutputEvent::Exit(status) => {
    ///             eprintln!("Shell exited with: {:?}", status);
    ///             break;
    ///         }
    ///         PtyOutputEvent::WriteError(err) => {
    ///             eprintln!("Write error: {}", err);
    ///             break;
    ///         }
    ///         _ => {}
    ///     }
    /// }
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
    /// [`crate::ControlledChild`]: crate::ControlledChild
    /// [`portable_pty::MasterPty`]: portable_pty::MasterPty
    pub fn spawn_read_write(
        self,
        arg_config: impl Into<PtyConfig>,
    ) -> miette::Result<PtyReadWriteSession> {
        let config = arg_config.into();

        // Create channels for bidirectional communication
        // Input: Your program → spawned process
        let (input_event_sender_half, mut input_receiver_half) =
            tokio::sync::mpsc::unbounded_channel::<PtyInputEvent>();
        // Output: Your program ← spawned process
        let (event_sender_half, output_event_receiver_half) =
            tokio::sync::mpsc::unbounded_channel::<PtyOutputEvent>();

        // Create a sync channel for the input handler task (spawn_blocking needs sync
        // channel). This bridges the async input channel to blocking I/O operations.
        let (input_handler_sender, input_handler_receiver) =
            std::sync::mpsc::channel::<PtyInputEvent>();

        // Clone senders for various tasks
        let reader_event_sender = event_sender_half.clone();
        let input_handler_event_sender = event_sender_half.clone();

        // [🛫 SPAWN 0] Spawn the main orchestration task.
        // Pin the completion handle: JoinHandle is not Unpin but select! requires it for
        // efficient polling without moving.
        let handle = Box::pin(tokio::spawn(async move {
            // Build the command, ensuring CWD is set
            let command = self.build()?;

            // Create PTY pair: controller (master) for bidirectional I/O, controlled
            // (slave) for spawned process
            let (controller, controlled) = create_pty_pair(&config)?;

            // Spawn the command in the controlled PTY (slave side)
            // The child process will use controlled as its stdin/stdout/stderr
            let mut child = spawn_command_in_pty(&controlled, command)?;

            // Clone a reader from the controller for the reader task
            // NOTE: Critical resource management - see module docs for PTY lifecycle
            // details.
            let reader = controller
                .try_clone_reader()
                .map_err(|e| miette::miette!("Failed to clone reader: {}", e))?;

            // Start the reader task with a controller reader clone to handle output from
            // spawned process
            let reader_handle = spawn_blocking_controller_reader_task(
                reader,
                reader_event_sender,
                config,
            );

            // The input handler task owns the controller and handles all input operations
            // This task writes input from your program to the spawned process via
            // controller
            let input_handler_handle = create_input_handler_task(
                controller,
                input_handler_receiver,
                input_handler_event_sender,
            );

            // Spawn a bridge task to convert async input channel to sync channel for
            // input handler This allows async input from your program to be
            // processed by the blocking input handler
            let bridge_handle = tokio::spawn(async move {
                while let Some(input) = input_receiver_half.recv().await {
                    if input_handler_sender.send(input).is_err() {
                        // Input handler task has exited
                        break;
                    }
                }

                // Ensure input handler gets Close signal
                let _unused = input_handler_sender.send(PtyInputEvent::Close);
            });

            // Wait for the child process to complete
            let status = tokio::task::spawn_blocking(move || child.wait())
                .await
                .into_diagnostic()?
                .into_diagnostic()?;

            // Store exit code before moving status
            let exit_code = status.exit_code();

            // Send exit event
            let _unused = event_sender_half.send(PtyOutputEvent::Exit(status));

            // CRITICAL: Drop the controlled half to signal EOF to reader.
            // See module docs for detailed PTY lifecycle management explanation.
            drop(controlled);

            // input_sender will be dropped when this task completes

            // Wait for all tasks to complete in proper order
            let _unused = bridge_handle.await;
            let _unused = input_handler_handle.await;
            let _unused = reader_handle.await;

            // Return the exit status
            Ok(portable_pty::ExitStatus::with_exit_code(exit_code))
        }));

        Ok(PtyReadWriteSession {
            input_event_sender_half,
            output_event_receiver_half,
            completion_handle: handle,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ControlChar, PtyConfigOption};

    #[tokio::test]
    async fn test_simple_command_lifecycle() -> miette::Result<()> {
        use PtyConfigOption::*;
        use tokio::time::{Duration, timeout};

        // Create a temporary directory for the test
        let temp_dir = std::env::temp_dir();

        let mut session = PtyCommandBuilder::new("echo")
            .args(["Hello, PTY!"])
            .cwd(temp_dir)
            .spawn_read_write(Output)
            .unwrap();

        // Give the echo command a moment to start and produce output
        tokio::time::sleep(Duration::from_millis(10)).await;

        let mut output = String::new();
        let mut events_received = Vec::new();
        let mut saw_exit = false;

        // Add timeout to prevent hanging
        let result = timeout(Duration::from_secs(10), async {
            while let Some(event) = session.output_event_receiver_half.recv().await {
                match &event {
                    PtyOutputEvent::Output(data) => {
                        let data_str = String::from_utf8_lossy(data);
                        output.push_str(&data_str);
                        events_received.push(format!(
                            "Output({} bytes): '{}'",
                            data.len(),
                            data_str
                        ));
                    }
                    PtyOutputEvent::Exit(status) => {
                        saw_exit = true;
                        events_received.push(format!("Exit({status:?})"));
                        assert!(
                            status.success(),
                            "Command should succeed with status: {status:?}"
                        );
                        break;
                    }
                    other => {
                        events_received.push(format!("{other:?}"));
                    }
                }
            }
        })
        .await;

        assert!(
            result.is_ok(),
            "Test timed out after 10 seconds. Events received: {events_received:?}, Output so far: '{output}'"
        );

        assert!(
            saw_exit,
            "Should see exit event. Events received: {events_received:?}, Output: '{output}'"
        );

        assert!(
            output.contains("Hello, PTY!"),
            "Output should contain 'Hello, PTY!'. Events received: {events_received:?}, Full output was: '{output}'"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_cat_with_input() -> miette::Result<()> {
        use PtyConfigOption::*;
        use tokio::time::{Duration, timeout};

        // Create a temporary directory for the test
        let temp_dir = std::env::temp_dir();

        let mut session = PtyCommandBuilder::new("cat")
            .cwd(temp_dir)
            .spawn_read_write(Output)
            .unwrap();

        // Send some text
        session
            .input_event_sender_half
            .send(PtyInputEvent::WriteLine("test input".into()))
            .unwrap();

        // Send EOF to make cat exit
        session
            .input_event_sender_half
            .send(PtyInputEvent::SendControl(ControlChar::CtrlD))
            .unwrap();

        let mut output = String::new();
        let mut events_received = Vec::new();
        let mut saw_exit = false;

        let result = timeout(Duration::from_secs(10), async {
            while let Some(event) = session.output_event_receiver_half.recv().await {
                match &event {
                    PtyOutputEvent::Output(data) => {
                        let data_str = String::from_utf8_lossy(data);
                        output.push_str(&data_str);
                        events_received.push(format!(
                            "Output({} bytes): '{}'",
                            data.len(),
                            data_str
                        ));
                    }
                    PtyOutputEvent::Exit(status) => {
                        saw_exit = true;
                        events_received.push(format!("Exit({status:?})"));
                        assert!(
                            status.success(),
                            "Cat should succeed with status: {status:?}"
                        );
                        break;
                    }
                    other => {
                        events_received.push(format!("{other:?}"));
                    }
                }
            }
        })
        .await;

        assert!(
            result.is_ok(),
            "Test timed out after 10 seconds. Events received: {events_received:?}, Output so far: '{output}'"
        );

        assert!(
            saw_exit,
            "Should see exit event. Events received: {events_received:?}, Output: '{output}'"
        );

        assert!(
            output.contains("test input"),
            "Output should contain 'test input'. Events received: {events_received:?}, Full output was: '{output}'"
        );

        Ok(())
    }

    #[tokio::test]
    #[cfg(not(target_os = "windows"))]
    async fn test_shell_calculation() -> miette::Result<()> {
        use PtyConfigOption::*;
        use tokio::time::{Duration, timeout};

        // Create a temporary directory for the test
        let temp_dir = std::env::temp_dir();

        // Use sh for calculation - POSIX compliant and always available on Unix
        let mut session = PtyCommandBuilder::new("sh")
            .args(["-c", "echo $((2+3)); echo 'Hello from Shell'"])
            .cwd(temp_dir)
            .spawn_read_write(Output)
            .unwrap();

        // Collect output with timeout
        let mut output = String::new();
        let mut events_received = Vec::new();
        let mut saw_exit = false;

        let result = timeout(Duration::from_secs(10), async {
            while let Some(event) = session.output_event_receiver_half.recv().await {
                match &event {
                    PtyOutputEvent::Output(data) => {
                        let data_str = String::from_utf8_lossy(data);
                        output.push_str(&data_str);
                        events_received.push(format!(
                            "Output({} bytes): '{}'",
                            data.len(),
                            data_str
                        ));
                    }
                    PtyOutputEvent::Exit(status) => {
                        saw_exit = true;
                        events_received.push(format!("Exit({status:?})"));
                        break;
                    }
                    other => {
                        events_received.push(format!("{other:?}"));
                    }
                }
            }
        })
        .await;

        assert!(
            result.is_ok(),
            "Shell session timed out after 10 seconds. Events received: {events_received:?}, Output so far: '{output}'"
        );

        assert!(
            saw_exit,
            "Should see exit event. Events received: {events_received:?}, Output: '{output}'"
        );

        // Verify we got expected output
        assert!(
            output.contains('5'),
            "Should see result of 2+3. Events received: {events_received:?}, Full output was: '{output}'"
        );
        assert!(
            output.contains("Hello from Shell"),
            "Should see hello message. Events received: {events_received:?}, Full output was: '{output}'"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_shell_echo_output() -> miette::Result<()> {
        use PtyConfigOption::*;
        use tokio::time::{Duration, timeout};

        let mut session = PtyCommandBuilder::new("sh")
            .args(["-c", "echo 'Test output from shell'"])
            .spawn_read_write(Output)
            .unwrap();

        // Give the shell command a moment to start and produce output
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Collect output
        let mut output = String::new();
        let mut events_received = Vec::new();
        let mut saw_exit = false;

        let result = timeout(Duration::from_secs(10), async {
            while let Some(event) = session.output_event_receiver_half.recv().await {
                match &event {
                    PtyOutputEvent::Output(data) => {
                        let data_str = String::from_utf8_lossy(data);
                        output.push_str(&data_str);
                        events_received.push(format!(
                            "Output({} bytes): '{}'",
                            data.len(),
                            data_str
                        ));
                    }
                    PtyOutputEvent::Exit(status) => {
                        saw_exit = true;
                        events_received.push(format!("Exit({status:?})"));
                        assert!(
                            status.success(),
                            "Shell should succeed with status: {status:?}"
                        );
                        break;
                    }
                    other => {
                        events_received.push(format!("{other:?}"));
                    }
                }
            }
        })
        .await;

        assert!(
            result.is_ok(),
            "Test timed out after 10 seconds. Events received: {events_received:?}, Output so far: '{output}'"
        );

        assert!(
            saw_exit,
            "Should see exit event. Events received: {events_received:?}, Output: '{output}'"
        );

        assert!(
            output.contains("Test output from shell"),
            "Should see shell output. Events received: {events_received:?}, Full output was: '{output}'"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_control_characters() -> miette::Result<()> {
        use PtyConfigOption::*;
        use tokio::time::{Duration, timeout};

        let mut session = PtyCommandBuilder::new("cat")
            .spawn_read_write(Output)
            .unwrap();

        // Test various control characters with delays for PTY processing
        session
            .input_event_sender_half
            .send(PtyInputEvent::WriteLine("Test line".into()))
            .unwrap();

        tokio::time::sleep(Duration::from_millis(10)).await;

        // Check if the session is still alive before sending
        if session
            .input_event_sender_half
            .send(PtyInputEvent::SendControl(ControlChar::Enter))
            .is_err()
        {
            // Session ended early, check output
            let mut output = String::new();
            while let Some(event) = session.output_event_receiver_half.recv().await {
                match event {
                    PtyOutputEvent::Output(data) => {
                        output.push_str(&String::from_utf8_lossy(&data));
                    }
                    PtyOutputEvent::Exit(status) => {
                        panic!(
                            "PTY exited early with status: {status:?}, output: '{output}'"
                        );
                    }
                    _ => {}
                }
            }
            panic!("Failed to send Enter control character, output so far: '{output}'");
        }

        tokio::time::sleep(Duration::from_millis(10)).await;

        session
            .input_event_sender_half
            .send(PtyInputEvent::Write(b"No newline".to_vec()))
            .unwrap();

        tokio::time::sleep(Duration::from_millis(10)).await;

        session
            .input_event_sender_half
            .send(PtyInputEvent::SendControl(ControlChar::Tab))
            .unwrap();

        tokio::time::sleep(Duration::from_millis(10)).await;

        session
            .input_event_sender_half
            .send(PtyInputEvent::Write(b"After tab".to_vec()))
            .unwrap();

        tokio::time::sleep(Duration::from_millis(10)).await;

        session
            .input_event_sender_half
            .send(PtyInputEvent::SendControl(ControlChar::Enter))
            .unwrap();

        // Allow time for all output to be processed before EOF
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Send EOF to exit
        session
            .input_event_sender_half
            .send(PtyInputEvent::SendControl(ControlChar::CtrlD))
            .unwrap();

        let mut output = String::new();

        // Add timeout to prevent hanging
        let result = timeout(Duration::from_secs(5), async {
            while let Some(event) = session.output_event_receiver_half.recv().await {
                match event {
                    PtyOutputEvent::Output(data) => {
                        output.push_str(&String::from_utf8_lossy(&data));
                    }
                    PtyOutputEvent::Exit(_) => break,
                    _ => {}
                }
            }
        })
        .await;

        assert!(result.is_ok(), "Test timed out. Output: '{output}'");
        assert!(
            output.contains("Test line"),
            "Output should contain 'Test line' but was: '{output}'"
        );
        assert!(
            output.contains("No newline"),
            "Output should contain 'No newline' but was: '{output}'"
        );
        assert!(
            output.contains("After tab"),
            "Output should contain 'After tab' but was: '{output}'"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_raw_escape_sequences() -> miette::Result<()> {
        use PtyConfigOption::*;
        use tokio::time::{Duration, timeout};

        // Skip this test in CI environments due to terminal emulation differences
        if is_ci::uncached() {
            eprintln!("Skipping test_raw_escape_sequences in CI environment");
            return Ok(());
        }

        // Create a temporary directory for the test
        let temp_dir = std::env::temp_dir();

        let mut session = PtyCommandBuilder::new("cat")
            .cwd(temp_dir)
            .spawn_read_write(Output)
            .unwrap();

        // Send some text with ANSI color codes using raw sequences
        let red_text = b"\x1b[31mRed Text\x1b[0m";
        session
            .input_event_sender_half
            .send(PtyInputEvent::Write(red_text.to_vec()))
            .unwrap();
        session
            .input_event_sender_half
            .send(PtyInputEvent::SendControl(ControlChar::Enter))
            .unwrap();

        // Add a delay to ensure the first line is processed
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Send using RawSequence variant
        let blue_seq = vec![0x1b, b'[', b'3', b'4', b'm']; // Blue color
        session
            .input_event_sender_half
            .send(PtyInputEvent::SendControl(ControlChar::RawSequence(
                blue_seq,
            )))
            .unwrap();
        session
            .input_event_sender_half
            .send(PtyInputEvent::Write(b"Blue Text".to_vec()))
            .unwrap();

        // Send reset sequence after blue text
        let reset_seq = vec![0x1b, b'[', b'0', b'm']; // Reset color
        session
            .input_event_sender_half
            .send(PtyInputEvent::SendControl(ControlChar::RawSequence(
                reset_seq,
            )))
            .unwrap();

        session
            .input_event_sender_half
            .send(PtyInputEvent::SendControl(ControlChar::Enter))
            .unwrap();

        // Add a delay to ensure all input is processed before EOF
        tokio::time::sleep(Duration::from_millis(100)).await;

        // EOF to exit
        session
            .input_event_sender_half
            .send(PtyInputEvent::SendControl(ControlChar::CtrlD))
            .unwrap();

        let mut output = Vec::new();
        let mut events_received = Vec::new();
        let mut saw_exit = false;

        let result = timeout(Duration::from_secs(10), async {
            while let Some(event) = session.output_event_receiver_half.recv().await {
                match &event {
                    PtyOutputEvent::Output(data) => {
                        output.extend_from_slice(data);
                        events_received.push(format!("Output({} bytes)", data.len()));
                    }
                    PtyOutputEvent::Exit(status) => {
                        saw_exit = true;
                        events_received.push(format!("Exit({status:?})"));
                        break;
                    }
                    other => {
                        events_received.push(format!("{other:?}"));
                    }
                }
            }
        })
        .await;

        let output_str = String::from_utf8_lossy(&output);

        assert!(
            result.is_ok(),
            "Test timed out after 10 seconds. Events received: {events_received:?}, Output so far: '{output_str}'"
        );

        assert!(
            saw_exit,
            "Should see exit event. Events received: {events_received:?}, Output: '{output_str}'"
        );

        // Check we got the ANSI sequences back
        assert!(
            output_str.contains("Red Text"),
            "Output should contain 'Red Text'. Events received: {events_received:?}, Full output was: '{output_str}'"
        );
        assert!(
            output_str.contains("Blue Text"),
            "Output should contain 'Blue Text'. Events received: {events_received:?}, Full output was: '{output_str}'"
        );
        // The actual ANSI codes might be echoed back
        // Note: cat may not preserve exact ANSI sequences depending on terminal settings

        Ok(())
    }
}
