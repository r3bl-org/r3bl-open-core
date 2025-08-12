// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use miette::IntoDiagnostic;
use tokio::sync::mpsc::unbounded_channel;

use crate::{PtyCommandBuilder, PtyConfig, PtyInputEvent, PtyOutputEvent,
            PtyReadWriteSession,
            common_impl::{create_input_handler_task, create_pty_pair,
                          spawn_blocking_controller_reader_task, spawn_command_in_pty}};

impl PtyCommandBuilder {
    /// Spawns a PTY session with bidirectional communication (read-write).
    ///
    /// ```text
    /// ┌────────────┐   ┌────────────┐   ┌─────────────────┐
    /// │Your Program│◄─►│    PTY     │   │Spawned Process  │
    /// │            │   │Controller/ │   │                 │
    /// │Reads/writes│   │  Master    │   │stdin/stdout/    │
    /// │through     │   │     ↕      │   │stderr redirected│
    /// │controller/ │   │    PTY     │   │to slave/        │
    /// │master side │   │     ↕      │   │controlled side  │
    /// │            │   │ Slave/     │◄─►│                 │
    /// │            │   │Controlled  │   │                 │
    /// └────────────┘   └────────────┘   └─────────────────┘
    /// ```
    ///
    /// This function provides the internal implementation for
    /// [`PtyCommandBuilder::spawn_read_write()`], which enables both reading from and
    /// writing to a child process running in a pseudo-terminal.
    ///
    /// # Core Architecture
    ///
    /// - **Shared functionality**: Uses `common_impl.rs` for PTY setup, reader/writer
    ///   tasks
    /// - **Session management**: [`PtyReadWriteSession`] struct provides channels for
    ///   bidirectional communication
    /// - **Type system**: [`PtyInputEvent`] for sending commands, [`super::ControlChar`]
    ///   for special keys, extended [`PtyOutputEvent`] for output
    /// - **Memory efficiency**: [`super::control_char_to_bytes()`] uses `Cow<'static,
    ///   [u8]>` to avoid unnecessary allocations
    ///
    /// # Design Decisions
    ///
    /// ## Dumb Pipes Approach
    /// The API treats input and output channels as dumb pipes of events, making no
    /// assumptions about the child process. The child determines terminal modes
    /// (cooked/raw), interprets environment variables, and handles all
    /// terminal-specific behavior. We simply provide the transport layer.
    ///
    /// ## Single Input Handler Architecture
    ///
    /// A single task owns the [`portable_pty::MasterPty`] and handles all input
    /// operations including resize. This avoids complex synchronization and ensures
    /// clean resource management. The task:
    /// - Processes all [`PtyInputEvent`] commands
    /// - Handles PTY resizing directly
    /// - Manages the write side of the PTY
    /// - Reports errors via the event channel
    ///
    /// ## Task Separation
    ///
    /// - **Reader task**: Independently reads from PTY, processes OSC sequences, sends
    ///   events
    /// - **Input handler task**: Owns [`portable_pty::MasterPty`], processes all input
    ///   commands including resize
    /// - **Bridge task**: Converts async channel to sync channel for the blocking input
    ///   handler
    ///
    /// ## Error Handling
    ///
    /// - Write errors terminate the session (no automatic retry)
    /// - Errors are reported via [`PtyOutputEvent::WriteError`] before termination
    /// - Three termination scenarios handled:
    ///   1. Child process self-terminates (normal or crash)
    ///   2. Explicit session termination via [`PtyInputEvent::Close`]
    ///   3. Unexpected termination (reported as [`PtyOutputEvent::UnexpectedExit`] event)
    ///
    /// ## Memory Efficiency
    ///
    /// - Control character sequences use `&'static [u8]` to avoid heap allocations
    /// - Only [`crate::ControlChar::RawSequence`] variants require owned data
    /// - Unbounded channels for simplicity (no backpressure handling)
    ///
    /// # Features
    ///
    /// - **Bidirectional communication**: Full read/write support for interactive
    ///   processes
    /// - **Control characters**: Comprehensive support including:
    ///   - Standard controls (Ctrl-C, Ctrl-D, Ctrl-Z, etc.)
    ///   - Arrow keys and navigation (Home, End, `PageUp`, `PageDown`)
    ///   - Function keys (F1-F12)
    ///   - Raw escape sequences for custom needs
    /// - **PTY resizing**: Dynamic terminal size adjustment via [`PtyInputEvent::Resize`]
    /// - **Explicit flush control**: [`PtyInputEvent::Flush`] for protocols sensitive to
    ///   message boundaries
    /// - **Proper cleanup**: Careful resource management prevents PTY deadlocks
    ///
    /// # Returns
    ///
    /// A session with:
    /// 1. `input_event_sender_half` for sending input events to the PTY
    /// 2. `output_event_receiver_half` combined stdout/stderr of child process -> events
    /// 3. `completion_handle` to await spawned child process completion
    ///
    /// # Example: Python REPL interaction
    ///
    /// ```rust,no_run
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use r3bl_tui::{PtyCommandBuilder, PtyConfigOption, PtyOutputEvent, PtyInputEvent, ControlChar};
    /// use tokio::time::{sleep, Duration};
    ///
    /// let mut session = PtyCommandBuilder::new("python3")
    ///     .args(["-u", "-i"])  // Unbuffered, interactive
    ///     .spawn_read_write(PtyConfigOption::Output)?;
    ///
    /// // Wait for Python to start
    /// sleep(Duration::from_millis(500)).await;
    ///
    /// // Send Python commands
    /// session.input_event_sender_half.send(PtyInputEvent::WriteLine("x = 2 + 3".into()))?;
    /// session.input_event_sender_half.send(PtyInputEvent::WriteLine("print(f'Result: {x}')".into()))?;
    /// session.input_event_sender_half.send(PtyInputEvent::SendControl(ControlChar::CtrlD))?; // Exit
    ///
    /// // Process output
    /// while let Some(event) = session.output_event_receiver_half.recv().await {
    ///     match event {
    ///         PtyOutputEvent::Output(data) => print!("{}", String::from_utf8_lossy(&data)),
    ///         PtyOutputEvent::Exit(_) => break,
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
    pub fn spawn_read_write(
        self,
        config: impl Into<PtyConfig>,
    ) -> miette::Result<PtyReadWriteSession> {
        let config = config.into();

        // Create channels for bidirectional communication
        // Input: Your program → spawned process
        let (input_event_sender_half, input_receiver_half) =
            unbounded_channel::<PtyInputEvent>();
        // Output: Spawned process → your program
        let (event_sender_half, output_event_receiver_half) =
            unbounded_channel::<PtyOutputEvent>();

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
                let mut receiver = input_receiver_half;
                while let Some(input) = receiver.recv().await {
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
    async fn test_echo_command() -> miette::Result<()> {
        use PtyConfigOption::*;
        use tokio::time::{Duration, timeout};

        let mut session = PtyCommandBuilder::new("echo")
            .args(["Hello, PTY!"])
            .spawn_read_write(Output)
            .unwrap();

        // Give the echo command a moment to start and produce output
        tokio::time::sleep(Duration::from_millis(10)).await;

        let mut output = String::new();
        let mut saw_exit = false;

        // Add timeout to prevent hanging
        let result = timeout(Duration::from_secs(3), async {
            while let Some(event) = session.output_event_receiver_half.recv().await {
                match event {
                    PtyOutputEvent::Output(data) => {
                        output.push_str(&String::from_utf8_lossy(&data));
                    }
                    PtyOutputEvent::Exit(status) => {
                        saw_exit = true;
                        assert!(
                            status.success(),
                            "Command should succeed with status: {:?}",
                            status
                        );
                        break;
                    }
                    _ => {}
                }
            }
        })
        .await;

        assert!(
            result.is_ok(),
            "Test timed out after 3 seconds. Output so far: '{}'",
            output
        );
        assert!(saw_exit, "Should see exit event. Output: '{}'", output);
        assert!(
            output.contains("Hello, PTY!"),
            "Output should contain 'Hello, PTY!' but was: '{}'",
            output
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_cat_with_input() -> miette::Result<()> {
        use PtyConfigOption::*;
        use tokio::time::{Duration, timeout};

        let mut session = PtyCommandBuilder::new("cat")
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
        let mut saw_exit = false;

        let result = timeout(Duration::from_secs(3), async {
            while let Some(event) = session.output_event_receiver_half.recv().await {
                match event {
                    PtyOutputEvent::Output(data) => {
                        output.push_str(&String::from_utf8_lossy(&data));
                    }
                    PtyOutputEvent::Exit(status) => {
                        saw_exit = true;
                        assert!(
                            status.success(),
                            "Cat should succeed with status: {:?}",
                            status
                        );
                        break;
                    }
                    _ => {}
                }
            }
        })
        .await;

        assert!(
            result.is_ok(),
            "Test timed out after 3 seconds. Output so far: '{}'",
            output
        );
        assert!(saw_exit, "Should see exit event. Output: '{}'", output);
        assert!(
            output.contains("test input"),
            "Output should contain 'test input' but was: '{}'",
            output
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_python_repl_interaction() -> miette::Result<()> {
        use PtyConfigOption::*;
        use tokio::time::{Duration, timeout};

        // Skip if Python is not available
        if std::process::Command::new("python3")
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!("Skipping Python test - python3 not available");
            return Ok(());
        }

        // Use a simple Python command that exits immediately
        let mut session = PtyCommandBuilder::new("python3")
            .args(["-c", "print(2 + 3); print('Hello from Python')"])
            .spawn_read_write(Output)
            .unwrap();

        // Collect output with timeout
        let mut output = String::new();
        let result = timeout(Duration::from_secs(3), async {
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

        assert!(
            result.is_ok(),
            "Python session timed out. Output so far: '{}'",
            output
        );

        // Verify we got expected output
        assert!(
            output.contains('5'),
            "Should see result of 2+3, but output was: '{}'",
            output
        );
        assert!(
            output.contains("Hello from Python"),
            "Should see hello message, but output was: '{}'",
            output
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_shell_command_interruption() -> miette::Result<()> {
        use PtyConfigOption::*;
        use tokio::time::{Duration, timeout};

        let mut session = PtyCommandBuilder::new("sh")
            .args(["-c", "echo 'Test output from shell'"])
            .spawn_read_write(Output)
            .unwrap();

        // Collect output
        let mut output = String::new();
        let mut saw_exit = false;
        let result = timeout(Duration::from_secs(3), async {
            while let Some(event) = session.output_event_receiver_half.recv().await {
                match event {
                    PtyOutputEvent::Output(data) => {
                        output.push_str(&String::from_utf8_lossy(&data));
                    }
                    PtyOutputEvent::Exit(status) => {
                        saw_exit = true;
                        assert!(
                            status.success(),
                            "Shell should succeed with status: {:?}",
                            status
                        );
                        break;
                    }
                    _ => {}
                }
            }
        })
        .await;

        assert!(
            result.is_ok(),
            "Test timed out after 3 seconds. Output so far: '{}'",
            output
        );
        assert!(saw_exit, "Should see exit event. Output: '{}'", output);
        assert!(
            output.contains("Test output from shell"),
            "Should see shell output, but was: '{}'",
            output
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

        session
            .input_event_sender_half
            .send(PtyInputEvent::SendControl(ControlChar::Enter))
            .unwrap();

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

        assert!(result.is_ok(), "Test timed out. Output: '{}'", output);
        assert!(
            output.contains("Test line"),
            "Output should contain 'Test line' but was: '{}'",
            output
        );
        assert!(
            output.contains("No newline"),
            "Output should contain 'No newline' but was: '{}'",
            output
        );
        assert!(
            output.contains("After tab"),
            "Output should contain 'After tab' but was: '{}'",
            output
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_raw_escape_sequences() -> miette::Result<()> {
        use PtyConfigOption::*;
        use tokio::time::{Duration, timeout};

        let mut session = PtyCommandBuilder::new("cat")
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
        session
            .input_event_sender_half
            .send(PtyInputEvent::SendControl(ControlChar::Enter))
            .unwrap();

        // EOF to exit
        session
            .input_event_sender_half
            .send(PtyInputEvent::SendControl(ControlChar::CtrlD))
            .unwrap();

        let mut output = Vec::new();
        let result = timeout(Duration::from_secs(3), async {
            while let Some(event) = session.output_event_receiver_half.recv().await {
                match event {
                    PtyOutputEvent::Output(data) => {
                        output.extend_from_slice(&data);
                    }
                    PtyOutputEvent::Exit(_) => break,
                    _ => {}
                }
            }
        })
        .await;

        assert!(result.is_ok(), "Test timed out after 3 seconds");

        // Check we got the ANSI sequences back
        let output_str = String::from_utf8_lossy(&output);
        assert!(
            output_str.contains("Red Text"),
            "Output should contain 'Red Text' but was: '{}'",
            output_str
        );
        assert!(
            output_str.contains("Blue Text"),
            "Output should contain 'Blue Text' but was: '{}'",
            output_str
        );
        // The actual ANSI codes might be echoed back
        // Note: cat may not preserve exact ANSI sequences depending on terminal settings

        Ok(())
    }
}
