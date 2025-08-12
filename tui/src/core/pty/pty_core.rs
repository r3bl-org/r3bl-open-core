// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::{borrow::Cow, path::PathBuf, pin::Pin};

use portable_pty::{CommandBuilder, MasterPty, PtySize, SlavePty};
use tokio::{sync::mpsc::{UnboundedReceiver, UnboundedSender},
            task::JoinHandle};

use super::{OscEvent, PtyConfig};

/// Buffer size for reading PTY output (4KB stack allocation).
///
/// This is used for the read buffer in PTY operations. The performance bottleneck
/// is not this buffer size but the `Vec<u8>` allocations in `PtyEvent::Output`.
pub const READ_BUFFER_SIZE: usize = 4096;

/// Type alias for the controlled side of a PTY (slave).
///
/// This represents the process-side of the PTY that the child process
/// will use for stdin/stdout/stderr.
pub type Controlled = Box<dyn SlavePty + Send>;

/// Type alias for the controlling side of a PTY (master).
///
/// This represents the controlling side that the parent process uses
/// to read from and write to the child process.
pub type Controller = Box<dyn MasterPty + Send>;

/// Type alias for a spawned child process in a PTY.
pub type ControlledChild = Box<dyn portable_pty::Child + Send + Sync>;

/// Type alias for a validated PTY command ready for execution.
///
/// This enhances readability by making the flow clear: [`PtyCommandBuilder`] `-> build()
/// ->` [`PtyCommand`]. This is a validated [`CommandBuilder`] returned by
/// [`PtyCommandBuilder::build`].
pub type PtyCommand = CommandBuilder;

/// Unified event type for PTY output that can contain both OSC sequences and raw output
/// data.
#[derive(Debug)]
pub enum PtyEvent {
    /// OSC sequence event (if OSC capture is enabled).
    Osc(OscEvent),
    /// Raw output data (stdout/stderr combined).
    Output(Vec<u8>),
    /// Process exited with status.
    Exit(portable_pty::ExitStatus),
    /// Child process crashed or terminated unexpectedly.
    UnexpectedExit(String),
    /// Write operation failed - session will terminate.
    WriteError(std::io::Error),
}

/// Input types that can be sent to a child process through PTY.
#[derive(Debug, Clone)]
pub enum PtyInput {
    /// Send raw bytes to child's stdin.
    Write(Vec<u8>),
    /// Send text with automatic newline.
    WriteLine(String),
    /// Send control sequences (Ctrl-C, Ctrl-D, etc.).
    SendControl(ControlChar),
    /// Resize the PTY window.
    Resize(PtySize),
    /// Explicit flush without writing data.
    /// Forces any buffered data to be sent to the child immediately.
    Flush,
    /// Close stdin (EOF).
    Close,
}

/// Control characters and special keys that can be sent to PTY.
#[derive(Debug, Clone)]
pub enum ControlChar {
    // Common control characters
    CtrlC, // SIGINT (interrupt)
    CtrlD, // EOF (end of file)
    CtrlZ, // SIGTSTP (suspend)
    CtrlL, // Clear screen
    CtrlU, // Clear line
    CtrlA, // Move to beginning of line
    CtrlE, // Move to end of line
    CtrlK, // Kill to end of line

    // Common keys
    Tab,    // Autocomplete
    Enter,  // Newline
    Escape, // ESC key
    Backspace,
    Delete,

    // Arrow keys
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,

    // Navigation keys
    Home,
    End,
    PageUp,
    PageDown,

    // Function keys (F1-F12)
    F(u8), // F(1) for F1, F(2) for F2, etc.

    // Raw escape sequence for advanced use cases
    RawSequence(Vec<u8>),
}

/// Session handle for read-only PTY communication.
///
/// Provides access to both the output stream and completion status of a PTY process:
/// - The `event_receiver_half` channel receives combined stdout/stderr from the child
///   process, along with optional OSC sequences and process exit events.
/// - The `completion_handle` can be awaited to know when the process completes and get
///   the final exit status.
#[derive(Debug)]
pub struct PtyReadOnlySession {
    /// Receives output events from the child process (combined stdout/stderr).
    pub event_receiver_half: UnboundedReceiver<PtyEvent>,
    /// Await this `completion_handle` for process completion.
    pub completion_handle: Pin<Box<JoinHandle<miette::Result<portable_pty::ExitStatus>>>>,
}

/// Session handle for interactive PTY communication.
///
/// Provides bidirectional communication with a child process running in a PTY.
/// The `event_receiver_half` channel receives combined stdout/stderr from the child
/// process.
#[derive(Debug)]
pub struct PtySession {
    /// Send input TO the child process.
    pub input_sender_half: UnboundedSender<PtyInput>,
    /// Receive output FROM the child process (combined stdout/stderr).
    pub event_receiver_half: UnboundedReceiver<PtyEvent>,
    /// Await this `completion_handle` for process completion.
    pub completion_handle: Pin<Box<JoinHandle<miette::Result<portable_pty::ExitStatus>>>>,
}

/// Converts a control character to its byte representation.
///
/// Returns a `Cow` to avoid unnecessary allocations for static sequences.
#[must_use]
pub fn control_char_to_bytes(ctrl: &ControlChar) -> Cow<'static, [u8]> {
    match ctrl {
        // Control characters
        ControlChar::CtrlC => Cow::Borrowed(&[0x03]),
        ControlChar::CtrlD => Cow::Borrowed(&[0x04]),
        ControlChar::CtrlZ => Cow::Borrowed(&[0x1A]),
        ControlChar::CtrlL => Cow::Borrowed(&[0x0C]),
        ControlChar::CtrlU => Cow::Borrowed(&[0x15]),
        ControlChar::CtrlA => Cow::Borrowed(&[0x01]),
        ControlChar::CtrlE => Cow::Borrowed(&[0x05]),
        ControlChar::CtrlK => Cow::Borrowed(&[0x0B]),

        // Common keys
        ControlChar::Tab => Cow::Borrowed(&[0x09]),
        ControlChar::Enter => Cow::Borrowed(&[0x0A]),
        ControlChar::Escape => Cow::Borrowed(&[0x1B]),
        ControlChar::Backspace => Cow::Borrowed(&[0x7F]),
        ControlChar::Delete => Cow::Borrowed(&[0x1B, 0x5B, 0x33, 0x7E]), // ESC[3~

        // Arrow keys (ANSI escape sequences)
        ControlChar::ArrowUp => Cow::Borrowed(&[0x1B, 0x5B, 0x41]), // ESC[A
        ControlChar::ArrowDown => Cow::Borrowed(&[0x1B, 0x5B, 0x42]), // ESC[B
        ControlChar::ArrowRight => Cow::Borrowed(&[0x1B, 0x5B, 0x43]), // ESC[C
        ControlChar::ArrowLeft => Cow::Borrowed(&[0x1B, 0x5B, 0x44]), // ESC[D

        // Navigation keys
        ControlChar::Home => Cow::Borrowed(&[0x1B, 0x5B, 0x48]), // ESC[H
        ControlChar::End => Cow::Borrowed(&[0x1B, 0x5B, 0x46]),  // ESC[F
        ControlChar::PageUp => Cow::Borrowed(&[0x1B, 0x5B, 0x35, 0x7E]), // ESC[5~
        ControlChar::PageDown => Cow::Borrowed(&[0x1B, 0x5B, 0x36, 0x7E]), // ESC[6~

        // Function keys
        ControlChar::F(n) => {
            match n {
                1 => Cow::Borrowed(&[0x1B, 0x4F, 0x50]), // ESCOP
                2 => Cow::Borrowed(&[0x1B, 0x4F, 0x51]), // ESCOQ
                3 => Cow::Borrowed(&[0x1B, 0x4F, 0x52]), // ESCOR
                4 => Cow::Borrowed(&[0x1B, 0x4F, 0x53]), // ESCOS
                5 => Cow::Borrowed(&[0x1B, 0x5B, 0x31, 0x35, 0x7E]), // ESC[15~
                6 => Cow::Borrowed(&[0x1B, 0x5B, 0x31, 0x37, 0x7E]), // ESC[17~
                7 => Cow::Borrowed(&[0x1B, 0x5B, 0x31, 0x38, 0x7E]), // ESC[18~
                8 => Cow::Borrowed(&[0x1B, 0x5B, 0x31, 0x39, 0x7E]), // ESC[19~
                9 => Cow::Borrowed(&[0x1B, 0x5B, 0x32, 0x30, 0x7E]), // ESC[20~
                10 => Cow::Borrowed(&[0x1B, 0x5B, 0x32, 0x31, 0x7E]), // ESC[21~
                11 => Cow::Borrowed(&[0x1B, 0x5B, 0x32, 0x33, 0x7E]), // ESC[23~
                12 => Cow::Borrowed(&[0x1B, 0x5B, 0x32, 0x34, 0x7E]), // ESC[24~
                _ => Cow::Borrowed(&[0x1B]),             /* Just ESC for unknown
                                                           * function keys */
            }
        }

        // Raw sequence - pass through as-is (requires owned data)
        ControlChar::RawSequence(bytes) => Cow::Owned(bytes.clone()),
    }
}

/// Configuration builder for PTY commands with sensible defaults.
///
/// This builder ensures critical settings are not forgotten when creating PTY commands:
/// - Automatically sets the current working directory if not specified
/// - Provides methods for common terminal environment variables
/// - Ensures commands spawn in the correct context (not in `$HOME`)
///
/// # Examples
///
/// Basic cargo command with OSC sequences:
///
/// ```rust
/// # use r3bl_tui::PtyCommandBuilder;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let cmd = PtyCommandBuilder::new("cargo")
///     .args(["build", "--release"])
///     .enable_osc_sequences()
///     .build()?;
/// # Ok(())
/// # }
/// ```
///
/// Command with custom working directory:
///
/// ```rust
/// # use r3bl_tui::PtyCommandBuilder;
/// # use std::env;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let cmd = PtyCommandBuilder::new("npm")
///     .args(["install"])
///     .cwd(env::temp_dir()) // Use temp dir instead of "/path/to/project"
///     .env("NODE_ENV", "production")
///     .build()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct PtyCommandBuilder {
    command: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
    env_vars: Vec<(String, String)>,
}

impl PtyCommandBuilder {
    /// Creates a new PTY command builder for the specified command.
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            cwd: None,
            env_vars: Vec::new(),
        }
    }

    /// Adds arguments to the command.
    #[must_use]
    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Sets the working directory.
    ///
    /// If not called, defaults to the current directory when [`build()`](Self::build) is
    /// invoked.
    #[must_use]
    pub fn cwd(mut self, path: impl Into<PathBuf>) -> Self {
        self.cwd = Some(path.into());
        self
    }

    /// Adds an environment variable to the command's environment.
    #[must_use]
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.push((key.into(), value.into()));
        self
    }

    /// Enables OSC sequence emission by setting appropriate environment variables.
    ///
    /// Cargo requires specific terminal environment variables to emit OSC 9;4 progress
    /// sequences. This method automatically detects and configures the appropriate
    /// environment based on the current terminal:
    ///
    /// - **Windows Terminal**: Detected via `WT_SESSION` (no additional config needed)
    /// - **`ConEmu`**: Detected via `ConEmuANSI=ON` (no additional config needed)
    /// - **`WezTerm`**: Set via `TERM_PROGRAM=WezTerm` (fallback for all platforms)
    ///
    /// This approach ensures maximum compatibility across different terminals and
    /// operating systems, particularly on Windows where Windows Terminal is the
    /// default in Windows 11.
    ///
    /// Here is a link to the Cargo source code that emits these sequences:
    /// - <https://github.com/rust-lang/cargo/blob/master/src/cargo/core/shell.rs#L594-L600>
    #[must_use]
    pub fn enable_osc_sequences(self) -> Self {
        // Windows Terminal sets WT_SESSION automatically, so we don't need to override
        // it.
        if std::env::var("WT_SESSION").is_ok() {
            // Already in Windows Terminal, no need to set anything.
            self
        } else if std::env::var("ConEmuANSI").ok() == Some("ON".into()) {
            // Already in ConEmu with ANSI enabled.
            self
        } else {
            // Fall back to WezTerm which works on all platforms.
            self.env("TERM_PROGRAM", "WezTerm")
        }
    }

    /// Builds the final [`PtyCommand`] with all configurations applied.
    ///
    /// Always sets a working directory - uses the provided one or defaults to current
    /// directory. This is critical to ensure the PTY starts in the expected location,
    /// since by default it uses `$HOME`.
    ///
    /// # Returns
    /// * `Ok(PtyCommand)` - Configured command ready for PTY execution
    /// * `Err(miette::Error)` - If current directory cannot be determined
    ///
    /// # Errors
    /// Returns an error if the current directory cannot be determined when no
    /// working directory was explicitly provided.
    ///
    /// # Panics
    /// Panics if `cwd` is `None` after attempting to set it to the current directory,
    /// which should be impossible in practice.
    pub fn build(mut self) -> miette::Result<PtyCommand> {
        // CRITICAL - Ensure working directory is always set - use current if not
        // specified. This prevents PTY from spawning in an unexpected location.
        if self.cwd.is_none() {
            let current_dir = std::env::current_dir()
                .map_err(|e| miette::miette!("Failed to get current directory: {}", e))?;
            self = self.cwd(current_dir);
        }

        // Create the PtyCommand to return.
        let mut cmd_to_return = PtyCommand::new(&self.command);

        // Add all arguments.
        for arg in &self.args {
            cmd_to_return.arg(arg);
        }

        // Set the working directory. This is guaranteed to be Some at this point because
        // we ensure it's set above. Using unwrap_or_else with unreachable!() makes the
        // invariant explicit while avoiding clippy warnings.
        let cwd = self.cwd.unwrap_or_else(|| {
            unreachable!("Working directory must be set - we ensure this above")
        });
        cmd_to_return.cwd(cwd);

        // Apply all environment variables.
        for (key, value) in &self.env_vars {
            cmd_to_return.env(key, value);
        }

        Ok(cmd_to_return)
    }

    /// Spawns a read-only PTY session.
    ///
    /// Returns a session with an output receiver for events and a `completion_handle` to
    /// await completion. The output channel receives combined stdout/stderr from the
    /// child process.
    ///
    /// # Example: Capturing command output
    /// ```rust
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use r3bl_tui::{PtyCommandBuilder, PtyConfigOption, PtyEvent};
    ///
    /// let mut session = PtyCommandBuilder::new("ls")
    ///     .args(["-la"])
    ///     .spawn_read_only(PtyConfigOption::Output)?;
    ///
    /// let mut output = Vec::new();
    /// while let Some(event) = session.event_receiver_half.recv().await {
    ///     match event {
    ///         PtyEvent::Output(data) => output.extend_from_slice(&data),
    ///         PtyEvent::Exit(status) if status.success() => {
    ///             println!("Command completed successfully");
    ///             break;
    ///         }
    ///         PtyEvent::Exit(status) => {
    ///             eprintln!("Command failed with: {:?}", status);
    ///             break;
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// println!("Output: {}", String::from_utf8_lossy(&output));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Example: Capturing OSC sequences from cargo build
    /// ```rust,no_run
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use r3bl_tui::{PtyCommandBuilder, PtyConfigOption, PtyEvent, OscEvent};
    ///
    /// let mut session = PtyCommandBuilder::new("cargo")
    ///     .args(["build"])
    ///     .enable_osc_sequences()  // Enable OSC 9;4 progress sequences
    ///     .spawn_read_only(PtyConfigOption::Osc)?;
    ///
    /// while let Some(event) = session.event_receiver_half.recv().await {
    ///     match event {
    ///         PtyEvent::Osc(OscEvent::ProgressUpdate(pct)) => {
    ///             println!("Build progress: {}%", pct);
    ///         }
    ///         PtyEvent::Exit(_) => break,
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
    pub fn spawn_read_only(
        self,
        config: impl Into<PtyConfig>,
    ) -> miette::Result<PtyReadOnlySession> {
        // Implementation will use spawn_pty_read_only_impl from spawn_pty_read_channel.rs
        Ok(crate::spawn_pty_read_only_impl(self, config))
    }

    /// Spawns a PTY session with bidirectional communication (read-write).
    ///
    /// Returns a session with input sender, output receiver, and `completion_handle`.
    /// The output channel receives combined stdout/stderr from the child process.
    ///
    /// # Example: Interactive command session
    /// ```rust
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use r3bl_tui::{PtyCommandBuilder, PtyConfigOption, PtyEvent, PtyInput};
    ///
    /// let mut session = PtyCommandBuilder::new("cat")
    ///     .spawn_read_write(PtyConfigOption::Output)?;
    ///
    /// // Send input to the process
    /// session.input_sender_half.send(PtyInput::WriteLine("Hello, PTY!".into()))?;
    /// session.input_sender_half.send(PtyInput::WriteLine("This is interactive".into()))?;
    /// session.input_sender_half.send(PtyInput::SendControl(r3bl_tui::ControlChar::CtrlD))?; // EOF
    ///
    /// // Collect output
    /// let mut output = String::new();
    /// while let Some(event) = session.event_receiver_half.recv().await {
    ///     match event {
    ///         PtyEvent::Output(data) => {
    ///             output.push_str(&String::from_utf8_lossy(&data));
    ///         }
    ///         PtyEvent::Exit(status) => {
    ///             println!("Process exited: {:?}", status);
    ///             break;
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// assert!(output.contains("Hello, PTY!"));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Example: Python REPL interaction
    /// ```rust,no_run
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use r3bl_tui::{PtyCommandBuilder, PtyConfigOption, PtyEvent, PtyInput, ControlChar};
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
    /// session.input_sender_half.send(PtyInput::WriteLine("x = 2 + 3".into()))?;
    /// session.input_sender_half.send(PtyInput::WriteLine("print(f'Result: {x}')".into()))?;
    /// session.input_sender_half.send(PtyInput::SendControl(ControlChar::CtrlD))?; // Exit
    ///
    /// // Process output
    /// while let Some(event) = session.event_receiver_half.recv().await {
    ///     match event {
    ///         PtyEvent::Output(data) => print!("{}", String::from_utf8_lossy(&data)),
    ///         PtyEvent::Exit(_) => break,
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
    ) -> miette::Result<PtySession> {
        // Implementation will use spawn_pty_read_write_impl from
        // spawn_pty_read_write_channels.rs
        Ok(crate::spawn_pty_read_write_impl(self, config))
    }
}
