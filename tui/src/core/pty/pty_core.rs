// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::{borrow::Cow, path::PathBuf, pin::Pin};

use portable_pty::{CommandBuilder, MasterPty, PtySize, SlavePty};
use tokio::{sync::mpsc::{UnboundedReceiver, UnboundedSender},
            task::JoinHandle};

use super::OscEvent;

/// Buffer size for reading PTY output (4KB stack allocation).
///
/// This is used for the read buffer in PTY operations. The performance bottleneck
/// is not this buffer size but the `Vec<u8>` allocations in `PtyOutputEvent::Output`.
pub const READ_BUFFER_SIZE: usize = 4096;

/// Type alias for the controlled half of a PTY (slave).
///
/// This represents the process-side of the PTY that the child process
/// will use for stdin/stdout/stderr.
pub type Controlled = Box<dyn SlavePty + Send>;

/// Type alias for the controller half of a PTY (master).
///
/// This represents the controller half that the parent process uses
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

/// Type alias for a pinned completion handle used in PTY sessions.
///
/// This simplifies the verbose
/// `Pin<Box<JoinHandle<miette::Result<portable_pty::ExitStatus>>>>` type used for
/// awaiting PTY session completion. The pinning satisfies Tokio's Unpin requirement for
/// select! macro usage. The `JoinHandle` returned by `tokio::spawn`
/// doesn't implement Unpin by default, but select! requires all futures to be
/// Unpin for efficient polling without moving them.
pub type PtyCompletionHandle =
    Pin<Box<JoinHandle<miette::Result<portable_pty::ExitStatus>>>>;

pub type OutputEventReceiverHalf = UnboundedReceiver<PtyOutputEvent>;
pub type InputEventSenderHalf = UnboundedSender<PtyInputEvent>;

/// Unified output event type for PTY that can contain both OSC sequences and raw output
/// data.
#[derive(Debug)]
pub enum PtyOutputEvent {
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

/// Input event types that can be sent to a child process through PTY.
#[derive(Debug, Clone)]
pub enum PtyInputEvent {
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
/// Provides access to both the output stream and completion status of a child process
/// running in the PTY controlled half (slave half).
/// - The `output_event_receiver_half` channel receives combined stdout/stderr from the
///   child process, along with optional OSC sequences and process exit events.
/// - The `completion_handle` can be awaited to know when the process completes and get
///   the final exit status.
#[derive(Debug)]
pub struct PtyReadOnlySession {
    /// Receives output events from the child process (combined stdout/stderr).
    pub output_event_receiver_half: OutputEventReceiverHalf,
    /// Await this `completion_handle` for process completion.
    ///
    /// Pinned to satisfy Tokio's Unpin requirement for select! macro usage in tests and
    /// other async coordination patterns. The `JoinHandle` returned by `tokio::spawn`
    /// doesn't implement Unpin by default, but select! requires all futures to be
    /// Unpin for efficient polling without moving them.
    pub completion_handle: PtyCompletionHandle,
}

/// Session handle for read-write PTY communication.
///
/// Provides bidirectional communication with a child process running in a PTY.
/// The `event_receiver_half` channel receives combined stdout/stderr from the child
/// process.
#[derive(Debug)]
pub struct PtyReadWriteSession {
    /// Send input TO the child process.
    pub input_event_sender_half: InputEventSenderHalf,
    /// Receive output FROM the child process (combined stdout/stderr).
    pub output_event_receiver_half: OutputEventReceiverHalf,
    /// Await this `completion_handle` for process completion.
    ///
    /// Pinned to satisfy Tokio's Unpin requirement for select! macro usage in tests and
    /// other async coordination patterns. The `JoinHandle` returned by `tokio::spawn`
    /// doesn't implement Unpin by default, but select! requires all futures to be
    /// Unpin for efficient polling without moving them.
    pub completion_handle: PtyCompletionHandle,
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
                // cspell:disable
                1 => Cow::Borrowed(&[0x1B, 0x4F, 0x50]), // ESCOP
                2 => Cow::Borrowed(&[0x1B, 0x4F, 0x51]), // ESCOQ
                3 => Cow::Borrowed(&[0x1B, 0x4F, 0x52]), // ESCOR
                4 => Cow::Borrowed(&[0x1B, 0x4F, 0x53]), // ESCOS
                // cspell:enable
                5 => Cow::Borrowed(&[0x1B, 0x5B, 0x31, 0x35, 0x7E]), // ESC[15~
                6 => Cow::Borrowed(&[0x1B, 0x5B, 0x31, 0x37, 0x7E]), // ESC[17~
                7 => Cow::Borrowed(&[0x1B, 0x5B, 0x31, 0x38, 0x7E]), // ESC[18~
                8 => Cow::Borrowed(&[0x1B, 0x5B, 0x31, 0x39, 0x7E]), // ESC[19~
                9 => Cow::Borrowed(&[0x1B, 0x5B, 0x32, 0x30, 0x7E]), // ESC[20~
                10 => Cow::Borrowed(&[0x1B, 0x5B, 0x32, 0x31, 0x7E]), // ESC[21~
                11 => Cow::Borrowed(&[0x1B, 0x5B, 0x32, 0x33, 0x7E]), // ESC[23~
                12 => Cow::Borrowed(&[0x1B, 0x5B, 0x32, 0x34, 0x7E]), // ESC[24~
                // Unknown function keys
                _ => Cow::Borrowed(&[0x1B]), // Just ESC
            }
        }

        // Raw sequence - pass through as-is (requires owned data)
        ControlChar::RawSequence(bytes) => Cow::Owned(bytes.clone()),
    }
}

/// Extension trait for converting portable_pty::ExitStatus to std::process::ExitStatus.
///
/// This trait provides cross-platform compatible conversion from portable_pty's
/// exit status type to the standard library's exit status type, handling platform
/// differences properly.
pub trait ExitStatusConversion {
    /// Converts a portable_pty::ExitStatus to std::process::ExitStatus.
    ///
    /// This method handles cross-platform exit status conversion properly:
    /// - On success: Uses explicit success status (exit code 0)
    /// - On failure: Encodes exit code in Unix wait status format with bounds checking
    /// - Clamps large exit codes to 255 to prevent overflow
    fn to_std_exit_status(self) -> std::process::ExitStatus;
}

impl ExitStatusConversion for portable_pty::ExitStatus {
    fn to_std_exit_status(self) -> std::process::ExitStatus {
        #[cfg(unix)]
        use std::os::unix::process::ExitStatusExt;

        if self.success() {
            // Success case: use explicit success status
            #[cfg(unix)]
            return std::process::ExitStatus::from_raw(0);
            #[cfg(not(unix))]
            return std::process::ExitStatus::from_raw(0);
        } else {
            // Failure case: encode exit code properly
            let code = self.exit_code();

            // Ensure we don't overflow when shifting for Unix wait status format
            let wait_status = if code <= 255 {
                (code as i32) << 8
            } else {
                // If exit code is too large, clamp to 255 and encode
                255_i32 << 8
            };

            #[cfg(unix)]
            return std::process::ExitStatus::from_raw(wait_status);
            #[cfg(not(unix))]
            return std::process::ExitStatus::from_raw(wait_status);
        }
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
}
#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn test_pty_event_debug() {
        let event = PtyOutputEvent::Output(b"test".to_vec());
        let debug_str = format!("{event:?}");
        assert!(debug_str.contains("Output"));
    }

    #[test]
    fn test_pty_input_debug_and_clone() {
        let input = PtyInputEvent::Write(b"test".to_vec());
        let cloned = input.clone();
        assert_eq!(format!("{input:?}"), format!("{cloned:?}"));
    }

    #[test]
    fn test_control_char_debug_and_clone() {
        let ctrl = ControlChar::CtrlC;
        let cloned = ctrl.clone();
        assert_eq!(format!("{ctrl:?}"), format!("{cloned:?}"));
    }

    #[test]
    fn test_control_char_to_bytes_basic() {
        assert_eq!(*control_char_to_bytes(&ControlChar::CtrlC), [0x03]);
        assert_eq!(*control_char_to_bytes(&ControlChar::CtrlD), [0x04]);
        assert_eq!(*control_char_to_bytes(&ControlChar::CtrlZ), [0x1A]);
        assert_eq!(*control_char_to_bytes(&ControlChar::CtrlL), [0x0C]);
        assert_eq!(*control_char_to_bytes(&ControlChar::CtrlU), [0x15]);
        assert_eq!(*control_char_to_bytes(&ControlChar::CtrlA), [0x01]);
        assert_eq!(*control_char_to_bytes(&ControlChar::CtrlE), [0x05]);
        assert_eq!(*control_char_to_bytes(&ControlChar::CtrlK), [0x0B]);
    }

    #[test]
    fn test_control_char_to_bytes_common_keys() {
        assert_eq!(*control_char_to_bytes(&ControlChar::Tab), [0x09]);
        assert_eq!(*control_char_to_bytes(&ControlChar::Enter), [0x0A]);
        assert_eq!(*control_char_to_bytes(&ControlChar::Escape), [0x1B]);
        assert_eq!(*control_char_to_bytes(&ControlChar::Backspace), [0x7F]);
        assert_eq!(
            *control_char_to_bytes(&ControlChar::Delete),
            [0x1B, 0x5B, 0x33, 0x7E]
        );
    }

    #[test]
    fn test_control_char_to_bytes_arrow_keys() {
        assert_eq!(
            *control_char_to_bytes(&ControlChar::ArrowUp),
            [0x1B, 0x5B, 0x41]
        );
        assert_eq!(
            *control_char_to_bytes(&ControlChar::ArrowDown),
            [0x1B, 0x5B, 0x42]
        );
        assert_eq!(
            *control_char_to_bytes(&ControlChar::ArrowRight),
            [0x1B, 0x5B, 0x43]
        );
        assert_eq!(
            *control_char_to_bytes(&ControlChar::ArrowLeft),
            [0x1B, 0x5B, 0x44]
        );
    }

    #[test]
    fn test_control_char_to_bytes_navigation() {
        assert_eq!(
            *control_char_to_bytes(&ControlChar::Home),
            [0x1B, 0x5B, 0x48]
        );
        assert_eq!(
            *control_char_to_bytes(&ControlChar::End),
            [0x1B, 0x5B, 0x46]
        );
        assert_eq!(
            *control_char_to_bytes(&ControlChar::PageUp),
            [0x1B, 0x5B, 0x35, 0x7E]
        );
        assert_eq!(
            *control_char_to_bytes(&ControlChar::PageDown),
            [0x1B, 0x5B, 0x36, 0x7E]
        );
    }

    #[test]
    fn test_control_char_to_bytes_function_keys() {
        // Test F1-F4 (special sequences)
        assert_eq!(
            *control_char_to_bytes(&ControlChar::F(1)),
            [0x1B, 0x4F, 0x50]
        );
        assert_eq!(
            *control_char_to_bytes(&ControlChar::F(2)),
            [0x1B, 0x4F, 0x51]
        );
        assert_eq!(
            *control_char_to_bytes(&ControlChar::F(3)),
            [0x1B, 0x4F, 0x52]
        );
        assert_eq!(
            *control_char_to_bytes(&ControlChar::F(4)),
            [0x1B, 0x4F, 0x53]
        );

        // Test F5-F12 (ESC[nn~ sequences)
        assert_eq!(
            *control_char_to_bytes(&ControlChar::F(5)),
            [0x1B, 0x5B, 0x31, 0x35, 0x7E]
        );
        assert_eq!(
            *control_char_to_bytes(&ControlChar::F(12)),
            [0x1B, 0x5B, 0x32, 0x34, 0x7E]
        );

        // Test unknown function key
        assert_eq!(*control_char_to_bytes(&ControlChar::F(99)), [0x1B]);
    }

    #[test]
    fn test_control_char_to_bytes_raw_sequence() {
        let custom_bytes = vec![0x1B, 0x5B, 0x32, 0x4A]; // Clear screen from cursor
        let ctrl = ControlChar::RawSequence(custom_bytes.clone());
        assert_eq!(*control_char_to_bytes(&ctrl), *custom_bytes);
    }

    #[test]
    fn test_pty_command_builder_new() {
        let builder = PtyCommandBuilder::new("test");
        assert_eq!(builder.command, "test");
        assert!(builder.args.is_empty());
        assert!(builder.cwd.is_none());
        assert!(builder.env_vars.is_empty());
    }

    #[test]
    fn test_pty_command_builder_args() {
        let builder = PtyCommandBuilder::new("test").args(["arg1", "arg2"]);

        assert_eq!(builder.args, vec!["arg1", "arg2"]);
    }

    #[test]
    fn test_pty_command_builder_cwd() {
        let path = env::temp_dir();
        let builder = PtyCommandBuilder::new("test").cwd(&path);

        assert_eq!(builder.cwd, Some(path));
    }

    #[test]
    fn test_pty_command_builder_env() {
        let builder = PtyCommandBuilder::new("test")
            .env("KEY1", "value1")
            .env("KEY2", "value2");

        assert_eq!(
            builder.env_vars,
            vec![
                ("KEY1".to_string(), "value1".to_string()),
                ("KEY2".to_string(), "value2".to_string())
            ]
        );
    }

    #[test]
    fn test_pty_command_builder_build() {
        let builder = PtyCommandBuilder::new("ls")
            .args(["-la", "-h"])
            .env("TEST_VAR", "test_value");

        let result = builder.build();
        assert!(result.is_ok());

        let _pty_command = result.unwrap();
        // PtyCommand doesn't expose get_program(), so we just verify build succeeds
    }

    #[test]
    fn test_pty_command_builder_build_with_cwd() {
        let temp_dir = env::temp_dir();
        let builder = PtyCommandBuilder::new("test").cwd(&temp_dir);

        let result = builder.build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_pty_command_builder_chaining() {
        let builder = PtyCommandBuilder::new("cargo")
            .args(["build", "--release"])
            .cwd(env::current_dir().unwrap())
            .env("CARGO_TERM_COLOR", "always")
            .enable_osc_sequences();

        let result = builder.build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_pty_session_structs_debug() {
        // Test that the session structs have Debug implemented
        // We can't easily test the actual structs without spawning processes,
        // but we can verify the types exist and have the expected fields

        // These will be compile-time checks
        fn check_debug<T: std::fmt::Debug>() {}

        check_debug::<PtyReadOnlySession>();
        check_debug::<PtyReadWriteSession>();
    }

    #[test]
    fn test_read_buffer_size_constant() {
        assert_eq!(READ_BUFFER_SIZE, 4096);
    }

    /// Compile-time validation that PTY type aliases are correctly defined.
    ///
    /// This test ensures that the core PTY type aliases (`Controller`, `Controlled`,
    /// and `ControlledChild`) can be used as function parameters, proving they are
    /// properly defined and usable. If any type alias has incorrect bounds or
    /// missing trait implementations, this test will fail at compile time.
    ///
    /// The functions are marked with `#[allow(dead_code)]` since they are never
    /// called - they only need to compile successfully to validate the type
    /// definitions.
    #[test]
    fn validate_pty_type_aliases_compile() {
        // Verify type aliases exist and are correctly defined
        #[allow(dead_code)]
        fn check_controller(_: Controller) {}
        #[allow(dead_code)]
        fn check_controlled(_: Controlled) {}
        #[allow(dead_code)]
        fn check_controlled_child(_: ControlledChild) {}

        // These are compile-time checks to ensure the types exist
    }
}
