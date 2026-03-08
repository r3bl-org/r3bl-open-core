// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::tasks::orchestrator::spawn_orchestrator_task;
use crate::{CaptureFlag, ControlledChildTerminationHandle, DefaultPtySize, DefaultSize,
            DetectFlag, InputEventSenderHalf, OutputEventReceiverHalf, PtyCommand,
            PtyInputEvent, PtyOrchestratorHandle, PtyOutputEvent, PtyPair, Size};
use miette::{IntoDiagnostic, miette};
use std::{collections::HashMap,
          ops::{Add, AddAssign},
          path::PathBuf};

/// Builder for configuring and spawning [`PTY`] sessions.
///
/// This provides a clean interface for building terminal commands with arguments,
/// environment variables, and current working directory.
///
/// In order to use this, you must:
/// 1. Create a [`PtySessionBuilder`] using [`new()`].
/// 2. (Optional) Configure the session using [`with_config()`].
/// 3. Call [`start()`] in order to get a [`PtySession`] struct, which you can use to wire
///    up your app.
///
/// # Examples
///
/// ```no_run
/// use r3bl_tui::{DefaultPtySessionConfig, PtyOutputEvent, ok,
///                PtySessionBuilder, PtySessionConfigOption};
///
/// # #[tokio::main]
/// # async fn main() -> miette::Result<()> {
/// let mut session = PtySessionBuilder::new("ls")
///     .cli_args(["-la", "--color=auto"])
///     .env_var("TERM", "xterm-256color")
///     .cwd("/tmp")
///     .with_config(
///         DefaultPtySessionConfig
///             + PtySessionConfigOption::NoCaptureOutput,
///     )
///     .start()?;
///
/// // Drive the session with a select! loop.
/// loop {
///     tokio::select! {
///         Some(event) = session.rx_output_event.recv() => {
///             match event {
///                 PtyOutputEvent::Output(bytes) => { /* render */ }
///                 PtyOutputEvent::Exit(_) => break,
///                 _ => {}
///             }
///         }
///         status = &mut session.orchestrator_task_handle => {
///             break;
///         }
///     }
/// }
/// # ok!()
/// # }
/// ```
///
/// [`new()`]: Self::new()
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`start()`]: Self::start()
/// [`with_config()`]: Self::with_config()
#[derive(Debug, Clone)]
pub struct PtySessionBuilder {
    /// The executable command to run (e.g., [`bash`] or [`ls`]).
    ///
    /// [`bash`]: https://en.wikipedia.org/wiki/Bash_(Unix_shell)
    /// [`ls`]: https://en.wikipedia.org/wiki/Ls_(command)
    pub command: String,

    /// Command-line arguments to pass to the executable.
    pub cli_args: Vec<String>,

    /// Environment variables to set for the child process.
    pub env_vars: HashMap<String, String>,

    /// Optional working directory for the child process.
    ///
    /// See [`Self::build()`] for implementation details and rationale regarding
    /// current working directory management.
    ///
    /// [`Self::build()`]: Self#current-working-directory
    pub maybe_cwd: Option<PathBuf>,

    /// Configuration for the [`PTY`] session.
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub config: PtySessionConfig,
}

mod pty_session_builder_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl PtySessionBuilder {
        /// Creates a new builder for the specified command.
        pub fn new(command: impl Into<String>) -> Self {
            Self {
                command: command.into(),
                cli_args: Vec::new(),
                env_vars: HashMap::new(),
                maybe_cwd: None,
                config: DefaultPtySessionConfig.into(),
            }
        }

        /// Adds a list of arguments to the command.
        #[must_use]
        pub fn cli_args(
            mut self,
            cli_args: impl IntoIterator<Item = impl Into<String>>,
        ) -> Self {
            self.cli_args.extend(cli_args.into_iter().map(Into::into));
            self
        }

        /// Adds a single argument to the command.
        #[must_use]
        pub fn cli_arg(mut self, cli_arg: impl Into<String>) -> Self {
            self.cli_args.push(cli_arg.into());
            self
        }

        /// Adds multiple environment variables to the command.
        #[must_use]
        pub fn env_vars(
            mut self,
            env_vars: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
        ) -> Self {
            for (key, value) in env_vars {
                self.env_vars.insert(key.into(), value.into());
            }
            self
        }

        /// Adds a single environment variable to the command.
        #[must_use]
        pub fn env_var(
            mut self,
            key: impl Into<String>,
            value: impl Into<String>,
        ) -> Self {
            self.env_vars.insert(key.into(), value.into());
            self
        }

        /// Sets the current working directory for the command.
        ///
        /// See [`Self::build()`] for implementation details and rationale regarding
        /// current working directory management.
        ///
        /// [`Self::build()`]: Self#current-working-directory
        #[must_use]
        pub fn cwd(mut self, path: impl Into<PathBuf>) -> Self {
            self.maybe_cwd = Some(path.into());
            self
        }

        /// Sets the configuration for the [`PTY`] session.
        ///
        /// Note that if [`PtySessionConfigOption::CaptureOsc`] is provided, then
        /// [`PtySessionBuilder::env_vars`] will be updated to include environment
        /// variables that trigger [`OSC`] emission from [`cargo`] and [`rustup`].
        ///
        /// [`cargo`]: https://github.com/rust-lang/cargo
        /// [`OSC`]: crate::osc_codes::OscSequence
        /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
        /// [`rustup`]: https://rust-lang.github.io/rustup/
        #[must_use]
        pub fn with_config(mut self, arg_config: impl Into<PtySessionConfig>) -> Self {
            // Replace the default config w/ the one that's provided here.
            let config = arg_config.into();
            self.config = config;

            // Use the config to enable OSC sequences on this builder.
            if config.capture_osc == CaptureFlag::Capture {
                start_impl::enable_osc_sequences(&mut self);
            }

            // Consume the builder.
            self
        }

        /// Builds a [`PtyCommand`] ready for execution.
        ///
        /// # Current Working Directory
        ///
        /// This method ensures the current working directory is valid if one was
        /// specified. If no [`CWD`] is provided, it defaults to the current
        /// process's [`CWD`]. This is a critical safeguard for cross-platform
        /// reliability (especially on Windows) where the child process might not
        /// reliably inherit the parent's [`CWD`] unless explicitly specified,
        /// which can cause tools like [`cargo`] or [`rustup`] to fail.
        ///
        /// This institutional knowledge of [`CWD`]-related flakiness (especially when
        /// running tests in parallel) is captured in the [`test_fixtures`]
        /// module, which uses explicit process isolation (via
        /// [`new_isolated_test_command()`]) to manage the per-process nature of
        /// [`CWD`].
        ///
        /// # Errors
        ///
        /// Returns an error if the specified working directory does not exist or the
        /// current directory cannot be determined.
        ///
        /// [`cargo`]: https://github.com/rust-lang/cargo
        /// [`CWD`]: std::env::current_dir
        /// [`new_isolated_test_command()`]: crate::core::test_fixtures::new_isolated_test_command
        /// [`rustup`]: https://rust-lang.github.io/rustup/
        /// [`test_fixtures`]: crate::core::test_fixtures
        pub fn build(&self) -> miette::Result<PtyCommand> {
            let mut builder = portable_pty::CommandBuilder::new(self.command.clone());
            builder.args(&self.cli_args);

            for (key, value) in &self.env_vars {
                builder.env(key, value);
            }

            if let Some(cwd) = &self.maybe_cwd {
                if !cwd.exists() {
                    return Err(miette!("CWD does not exist: {:?}", cwd));
                }
                builder.cwd(cwd);
            } else {
                let current_dir = std::env::current_dir().into_diagnostic()?;
                builder.cwd(current_dir);
            }

            Ok(builder)
        }

        /// Starts a [`PTY`] session by orchestrating the setup of the OS-level
        /// [`PtyPair`] and the [Background Tasks] needed for full bidirectional
        /// I/O.
        ///
        /// This function handles the entire initialization sequence:
        /// 1. Creates a [`bounded MPSC channel`] for output events (sized to
        ///    [`DefaultSize::PtyChannelBufferSize`]).
        /// 2. Creates a [`bounded MPSC channel`] for input events (sized to
        ///    [`DefaultSize::PtyChannelBufferSize`]).
        /// 3. Spawns the child process in a [`PtyPair`] (Engine Layer).
        /// 4. Spawns a [Reader Task] (blocking) to drain output from the process.
        /// 5. Spawns a [Writer Task] (blocking) to pump input events to the process.
        /// 6. Spawns an [Orchestrator Task] (async) to monitor the child process's
        ///    lifecycle.
        ///
        /// # Errors
        ///
        /// Returns an error if:
        /// - The command cannot be built from the [`PtySessionBuilder`].
        /// - The [`PtyPair`] fails to open or the child process fails to spawn.
        /// - The controller reader cannot be cloned for the background task.
        ///
        /// [`bounded MPSC channel`]: tokio::sync::mpsc::channel
        /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
        /// [Background Tasks]: crate::core::pty#the-task-trio
        /// [Orchestrator Task]: crate::tasks::spawn_orchestrator_task
        /// [Reader Task]: crate::tasks::spawn_blocking_reader_task
        /// [Writer Task]: crate::tasks::spawn_blocking_writer_task
        pub fn start(self) -> miette::Result<PtySession> { start_impl::start(self) }
    }
}

pub mod start_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Start a [`PTY`] session from the given [`PtySessionBuilder`] configuration.
    ///
    /// Opens a [`PTY`] pair, spawns the child process, and wires up input/output
    /// channels.
    ///
    /// # Errors
    ///
    /// Returns [`miette::Error`] if:
    /// - The command cannot be built from the builder configuration.
    /// - The [`PTY`] pair fails to open or the child process fails to spawn.
    /// - The controller reader cannot be cloned.
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub fn start(this: PtySessionBuilder) -> miette::Result<PtySession> {
        // Channel for output events (child process → app).
        let (output_event_ch_tx_half, output_event_ch_rx_half) =
            tokio::sync::mpsc::channel::<PtyOutputEvent>(
                DefaultSize::PtyChannelBufferSize.into(),
            );

        // Channel for input events (app → child process).
        let (input_event_ch_tx_half, input_event_ch_rx_half) =
            tokio::sync::mpsc::channel::<PtyInputEvent>(
                DefaultSize::PtyChannelBufferSize.into(),
            );

        let command = this.build()?;

        let (pty_pair, controlled_child) =
            PtyPair::open_and_spawn(this.config.pty_size, command)?;

        let child_process_termination_handle = controlled_child.clone_killer();

        let controller_reader = pty_pair
            .controller()
            .try_clone_reader()
            .map_err(|e| miette!("Failed to clone reader: {}", e))?;

        let controller = pty_pair.into_controller();

        let orchestrator_task_handle = spawn_orchestrator_task(
            controlled_child,
            controller_reader,
            controller,
            input_event_ch_tx_half.clone(),
            input_event_ch_rx_half,
            output_event_ch_tx_half,
            this.config,
        );

        Ok(PtySession {
            tx_input_event: input_event_ch_tx_half,
            rx_output_event: output_event_ch_rx_half,
            orchestrator_task_handle,
            child_process_termination_handle,
        })
    }

    /// Enable capture of **[`OSC`]** (Operating System Command) escape sequences.
    ///
    /// This is required to receive structured events like **`OSC 9;4`** for progress
    /// reporting or **`OSC 0`** for terminal title updates. When enabled, the [`PTY`]
    /// parser intercepts these sequences and emits them as [`PtyOutputEvent::Osc`].
    ///
    /// # Real-time build progress
    ///
    /// When this option is enabled via [`PtySessionBuilder::with_config()`], it also
    /// ensures that [`cargo`] and [`rustup`] commands emit **`OSC 9;4`** (`ESC ] 9 ; 4`)
    /// sequences by setting the following environment variables on the
    ///
    /// - **[`CARGO_TERM_PROGRESS_WHEN=always`]** — forces cargo to always emit its
    ///   progress bar, bypassing its own heuristics about whether a progress bar is
    ///   appropriate. Without this, cargo may suppress progress output even inside a
    ///   [`PTY`], for example when it detects a non-interactive session or a dumb
    ///   terminal.
    ///
    /// - **[`TERM=xterm-256color`]** — signals that the terminal supports modern escape
    ///   sequences, including [`OSC`] and hyperlinks. Notably, this value passes the
    ///   terminal capability [exclusion list] in
    ///   [`examine_env_vars_to_determine_hyperlink_support()`], where plain `"xterm"` is
    ///   excluded. It also ensures cargo trusts the terminal enough to emit [`OSC`]
    ///   sequences rather than falling back to plain text output.
    ///
    /// The emitted [`OSC`] bytes are parsed downstream by [`OscBuffer`] into
    /// [`OscEvent`] variants such as [`ProgressUpdate(u8)`], and delivered to
    /// the caller via the [`PTY`] session's MPSC channel.
    ///
    /// [`CARGO_TERM_PROGRESS_WHEN=always`]:
    ///     https://doc.rust-lang.org/cargo/reference/config.html#termprogresswhen
    /// [`cargo`]: https://github.com/rust-lang/cargo
    /// [`examine_env_vars_to_determine_hyperlink_support()`]:
    ///     crate::examine_env_vars_to_determine_hyperlink_support
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [`OscBuffer`]: crate::OscBuffer
    /// [`OscEvent`]: crate::OscEvent
    /// [`ProgressUpdate(u8)`]: crate::OscEvent::ProgressUpdate
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [`rustup`]: https://rust-lang.github.io/rustup/
    /// [`TERM=xterm-256color`]: https://en.wikipedia.org/wiki/Xterm#256-color_mode
    /// [exclusion list]: https://inclusivenaming.org/word-lists/tier-1/
    pub fn enable_osc_sequences(builder: &mut PtySessionBuilder) {
        builder
            .env_vars
            .insert("CARGO_TERM_PROGRESS_WHEN".to_string(), "always".to_string());
        builder
            .env_vars
            .insert("TERM".to_string(), "xterm-256color".to_string());
    }
}

/// Handle for a [`PTY`] session.
///
/// This is returned by [`PtySessionBuilder::start()`].
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`PtySessionBuilder::start()`]: PtySessionBuilder::start
#[derive(Debug)]
pub struct PtySession {
    /// Send [`PtyInputEvent`] events to the child process.
    pub tx_input_event: InputEventSenderHalf,

    /// Receive [`PtyOutputEvent`] events from the child process.
    pub rx_output_event: OutputEventReceiverHalf,

    /// Handle to await spawned process orchestration and completion. Returns the final
    /// exit status.
    pub orchestrator_task_handle: PtyOrchestratorHandle,

    /// Handle to explicitly terminate the child process if needed.
    pub child_process_termination_handle: ControlledChildTerminationHandle,
}

// XMARK: Clever Rust, use of `impl Into<PtySessionConfig>` for elegant constructor config
// options.

/// Configuration for a [`PTY`] session.
///
/// This struct holds the final resolved state of all configuration options. While this
/// struct is `pub`, it is **not** intended to be constructed manually. Instead, either:
/// 1. Compose the desired [`PtySessionConfigOption`]s using the `+` operator from
///    scratch.
/// 2. Start with [`DefaultPtySessionConfig`] and use `+` operator to override any default
///    options.
///
/// # Examples
///
/// ```rust
/// # use r3bl_tui::{
/// #     DefaultPtySessionConfig, PtySessionConfig, PtySessionConfigOption,
/// #     PtySessionConfigOption::CaptureOsc, PtySessionConfigOption::CaptureOutput
/// # };
/// let config_1: PtySessionConfig = DefaultPtySessionConfig + CaptureOsc;
/// let config_2: PtySessionConfig = CaptureOsc + CaptureOutput;
/// let config_3: PtySessionConfig = DefaultPtySessionConfig + CaptureOsc + CaptureOutput;
/// let config_4: PtySessionConfig = DefaultPtySessionConfig.into();
/// ```
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtySessionConfig {
    /// Whether to capture **[`OSC`]** sequences.
    ///
    /// See [`PtySessionConfigOption::CaptureOsc`] for details.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    pub capture_osc: CaptureFlag,

    /// Whether to capture raw terminal output.
    ///
    /// See [`PtySessionConfigOption::CaptureOutput`] for details.
    pub capture_output: CaptureFlag,

    /// Whether to detect terminal cursor mode changes.
    ///
    /// See [`PtySessionConfigOption::DetectCursorMode`] for details.
    pub detect_cursor_mode: DetectFlag,

    /// The initial window size for the [`PTY`].
    ///
    /// See [`PtySessionConfigOption::Size`] for details.
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub pty_size: Size,
}

/// Marker struct that provides the default [`PtySessionConfig`].
///
/// This zero-sized type acts as the starting point for **composing** configuration
/// options. It implements [`Into<PtySessionConfig>`] and supports the `+` operator for
/// applying [`PtySessionConfigOption`]s.
///
/// # Default Values
///
/// See the [`From<DefaultPtySessionConfig>`] implementation for the default field values.
///
/// [`DefaultPtySessionConfig::default()`]: DefaultPtySessionConfig::default()
#[derive(Debug, Clone, Copy)]
pub struct DefaultPtySessionConfig;

mod impl_default_pty_session_config {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl DefaultPtySessionConfig {
        /// Generate the default [`PtySessionConfig`] with sensible config options.
        #[must_use]
        #[allow(clippy::should_implement_trait)]
        pub fn default() -> PtySessionConfig {
            PtySessionConfig {
                capture_osc: CaptureFlag::NoCapture,
                capture_output: CaptureFlag::Capture,
                detect_cursor_mode: DetectFlag::Detect,
                pty_size: DefaultPtySize.into(),
            }
        }
    }

    /// Convert [`DefaultPtySessionConfig`] marker to [`PtySessionConfig`].
    impl From<DefaultPtySessionConfig> for PtySessionConfig {
        fn from(_: DefaultPtySessionConfig) -> Self { DefaultPtySessionConfig::default() }
    }
}

/// Configuration options for a [`PTY`] session.
///
/// Options exist so they can be composed to build a [`PtySessionConfig`], using the `+`
/// operator. The operator follows a "last write wins" for each field.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtySessionConfigOption {
    /// Enable capture of **[`OSC`]** (Operating System Command) escape sequences.
    ///
    /// This is required to receive structured events like **`OSC 9;4`** for progress
    /// reporting or **`OSC 0`** for terminal title updates. When enabled, the [`PTY`]
    /// parser intercepts these sequences and emits them as [`PtyOutputEvent::Osc`].
    ///
    /// See [`enable_osc_sequences()`] for more details.
    ///
    /// [`enable_osc_sequences()`]: super::start_impl::enable_osc_sequences()
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    CaptureOsc,

    /// Disable **[`OSC`]** sequence capture.
    ///
    /// When disabled, **[`OSC`]** sequences are treated as raw output bytes and
    /// delivered via [`PtyOutputEvent::Output`], rather than being parsed into
    /// structured events.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    NoCaptureOsc,

    /// Enable capture of raw terminal output.
    ///
    /// [`stdout`] and [`stderr`] outputs from the child process are delivered as
    /// [`PtyOutputEvent::Output`] events. This is the default behavior and is
    /// required for displaying the process's output in a terminal.
    ///
    /// [`stderr`]: std::io::stderr
    /// [`stdout`]: std::io::stdout
    CaptureOutput,

    /// Disable raw output capture.
    ///
    /// Useful for background tasks where you only care about structured events
    /// (like [`CaptureOsc`]) or process lifecycle events (like [`Exit`]), and
    /// want to avoid the overhead of processing large volumes of raw text.
    ///
    /// [`CaptureOsc`]: Self::CaptureOsc
    /// [`Exit`]: crate::PtyOutputEvent::Exit
    NoCaptureOutput,

    /// Enable detection of terminal cursor mode changes.
    ///
    /// Intercepts escape sequences that change the cursor's behavior (e.g.,
    /// showing/hiding the cursor, or changing the blinking mode) and emits them
    /// as [`PtyOutputEvent::CursorModeChange`].
    DetectCursorMode,

    /// Disable terminal cursor mode detection.
    ///
    /// When disabled, cursor mode escape sequences are delivered as raw output
    /// bytes via [`PtyOutputEvent::Output`].
    NoDetectCursorMode,

    /// Specify the initial window size ([`rows`] and [`columns`]) for the [`PTY`].
    ///
    /// Correct sizing is essential for **`TUI`** applications like `htop` or
    /// `vim` to render their interface properly within the available terminal
    /// area.
    ///
    /// [`columns`]: crate::ColWidth
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [`rows`]: crate::RowHeight
    Size(Size),
}

/// This module implements the "heavy lifting" for the Elegant Constructor DSL Pattern.
///
/// This enables an elegant, type-safe, and ergonomic DSL for configuring a [`PTY`]
/// session. By leveraging [`impl Into<PtySessionConfig>`] and operator overloading (`+`),
/// callers can progressively disclose their configuration needs.
///
/// It implements [`From`] traits to convert various configuration types
/// ([`DefaultPtySessionConfig`], [`PtySessionConfig`], and [`PtySessionConfigOption`])
/// into [`PtySessionConfig`], and [`Add`] traits to combine them with `+`. This is what
/// allows [`PtySessionBuilder::with_config`] to accept multiple types of inputs via
/// [`impl Into<PtySessionConfig>`].
///
/// See [`PtySessionBuilder`] docs for a full usage example.
///
/// [`impl Into<PtySessionConfig>`]: PtySessionConfig
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
mod impl_elegant_constructor_dsl_pattern {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl PtySessionConfig {
        fn apply(&mut self, option: PtySessionConfigOption) {
            match option {
                PtySessionConfigOption::CaptureOsc => {
                    self.capture_osc = CaptureFlag::Capture;
                }
                PtySessionConfigOption::NoCaptureOsc => {
                    self.capture_osc = CaptureFlag::NoCapture;
                }
                PtySessionConfigOption::CaptureOutput => {
                    self.capture_output = CaptureFlag::Capture;
                }
                PtySessionConfigOption::NoCaptureOutput => {
                    self.capture_output = CaptureFlag::NoCapture;
                }
                PtySessionConfigOption::DetectCursorMode => {
                    self.detect_cursor_mode = DetectFlag::Detect;
                }
                PtySessionConfigOption::NoDetectCursorMode => {
                    self.detect_cursor_mode = DetectFlag::NoDetect;
                }
                PtySessionConfigOption::Size(size) => self.pty_size = size,
            }
        }
    }

    /// Start from [`DefaultPtySessionConfig`] and apply one option with `+`.
    impl Add<PtySessionConfigOption> for DefaultPtySessionConfig {
        type Output = PtySessionConfig;

        fn add(self, rhs: PtySessionConfigOption) -> PtySessionConfig {
            let mut config = PtySessionConfig::from(self);
            config.apply(rhs);
            config
        }
    }

    /// Combine two [`PtySessionConfigOption`]s into a [`PtySessionConfig`] using `+`.
    impl Add<PtySessionConfigOption> for PtySessionConfigOption {
        type Output = PtySessionConfig;

        fn add(self, rhs: PtySessionConfigOption) -> PtySessionConfig {
            let mut config = PtySessionConfig::from(DefaultPtySessionConfig);
            config.apply(self);
            config.apply(rhs);
            config
        }
    }

    /// Add an option to an existing [`PtySessionConfig`].
    impl Add<PtySessionConfigOption> for PtySessionConfig {
        type Output = PtySessionConfig;

        fn add(mut self, rhs: PtySessionConfigOption) -> PtySessionConfig {
            self.apply(rhs);
            self
        }
    }

    /// Implement [`AddAssign`] for `+=` operator on [`PtySessionConfig`].
    impl AddAssign<PtySessionConfigOption> for PtySessionConfig {
        fn add_assign(&mut self, rhs: PtySessionConfigOption) { self.apply(rhs); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{height, size, width};

    #[test]
    fn test_default_config() {
        let config = PtySessionConfig::from(DefaultPtySessionConfig);
        assert_eq!(config.capture_osc, CaptureFlag::NoCapture);
        assert_eq!(config.capture_output, CaptureFlag::Capture);
        assert_eq!(config.detect_cursor_mode, DetectFlag::Detect);
    }

    #[test]
    fn test_option_combination() {
        // Option + Option.
        let config =
            PtySessionConfigOption::CaptureOsc + PtySessionConfigOption::NoCaptureOutput;
        assert_eq!(config.capture_osc, CaptureFlag::Capture);
        assert_eq!(config.capture_output, CaptureFlag::NoCapture);

        // DefaultPtySessionConfig + one option.
        let config = DefaultPtySessionConfig + PtySessionConfigOption::CaptureOsc;
        assert_eq!(config.capture_osc, CaptureFlag::Capture);
        assert_eq!(config.capture_output, CaptureFlag::Capture); // Default

        // DefaultPtySessionConfig + two options.
        let config = DefaultPtySessionConfig
            + PtySessionConfigOption::CaptureOsc
            + PtySessionConfigOption::NoCaptureOutput;
        assert_eq!(config.capture_osc, CaptureFlag::Capture);
        assert_eq!(config.capture_output, CaptureFlag::NoCapture);

        // DefaultPtySessionConfig + NoDetectCursorMode.
        let config = DefaultPtySessionConfig + PtySessionConfigOption::NoDetectCursorMode;
        assert_eq!(config.detect_cursor_mode, DetectFlag::NoDetect);

        // DefaultPtySessionConfig + three options.
        let sz = size(width(80) + height(24));
        let config = DefaultPtySessionConfig
            + PtySessionConfigOption::CaptureOsc
            + PtySessionConfigOption::CaptureOutput
            + PtySessionConfigOption::Size(sz);
        assert_eq!(config.capture_osc, CaptureFlag::Capture);
        assert_eq!(config.capture_output, CaptureFlag::Capture);
        assert_eq!(config.pty_size, sz);
    }

    #[test]
    fn test_add_assign_and_chaining() {
        let mut config: PtySessionConfig = DefaultPtySessionConfig.into();
        config += PtySessionConfigOption::CaptureOsc;
        assert_eq!(config.capture_osc, CaptureFlag::Capture);

        let sz = size(width(100) + height(50));
        let config = config
            + PtySessionConfigOption::Size(sz)
            + PtySessionConfigOption::NoCaptureOutput;
        assert_eq!(config.capture_osc, CaptureFlag::Capture);
        assert_eq!(config.capture_output, CaptureFlag::NoCapture);
        assert_eq!(config.pty_size, sz);
    }

    #[test]
    fn test_default_with_size() {
        let sz = size(width(120) + height(60));
        let config = DefaultPtySessionConfig + PtySessionConfigOption::Size(sz);
        assert_eq!(config.pty_size, sz);
        assert_eq!(config.capture_output, CaptureFlag::Capture); // Default
        assert_eq!(config.capture_osc, CaptureFlag::NoCapture); // Default
    }

    #[test]
    fn test_builder_pattern() {
        let sz = size(width(80) + height(24));
        let builder = PtySessionBuilder::new("bash")
            .cli_args(["-c", "ls"])
            .cli_arg("-la")
            .env_vars([("KEY1", "VAL1"), ("KEY2", "VAL2")])
            .env_var("KEY3", "VAL3")
            .cwd("/tmp")
            .with_config(
                DefaultPtySessionConfig
                    + PtySessionConfigOption::Size(sz)
                    + PtySessionConfigOption::CaptureOsc,
            );

        assert_eq!(builder.command, "bash");
        assert_eq!(builder.cli_args, vec!["-c", "ls", "-la"]);
        assert_eq!(builder.env_vars.get("KEY1").unwrap(), "VAL1");
        assert_eq!(builder.env_vars.get("KEY2").unwrap(), "VAL2");
        assert_eq!(builder.env_vars.get("KEY3").unwrap(), "VAL3");
        assert_eq!(builder.maybe_cwd, Some(PathBuf::from("/tmp")));
        assert_eq!(builder.config.pty_size, sz);
        assert_eq!(builder.config.capture_osc, CaptureFlag::Capture);
    }

    #[test]
    fn test_builder_build() {
        let builder = PtySessionBuilder::new("ls").cwd("/");
        let result = builder.build();
        assert!(result.is_ok());

        let builder =
            PtySessionBuilder::new("ls").cwd("/non_existent_directory_r3bl_test");
        let result = builder.build();
        assert!(result.is_err());

        // Test build with default CWD (None).
        let builder = PtySessionBuilder::new("ls");
        let result = builder.build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_overwrites() {
        let builder = PtySessionBuilder::new("bash")
            .cwd("/tmp")
            .cwd("/")
            .with_config(DefaultPtySessionConfig)
            .with_config(DefaultPtySessionConfig + PtySessionConfigOption::CaptureOsc);

        assert_eq!(builder.maybe_cwd, Some(PathBuf::from("/")));
        assert_eq!(builder.config.capture_osc, CaptureFlag::Capture);
    }

    #[test]
    fn test_enable_osc_sequences() {
        let mut builder = PtySessionBuilder::new("cargo");
        start_impl::enable_osc_sequences(&mut builder);

        assert_eq!(
            builder.env_vars.get("CARGO_TERM_PROGRESS_WHEN").unwrap(),
            "always"
        );
        assert_eq!(builder.env_vars.get("TERM").unwrap(), "xterm-256color");
    }

    #[allow(clippy::unnecessary_get_then_check)]
    #[test]
    fn test_with_config_osc_side_effect() {
        // with_config(CaptureOsc) should trigger enable_osc_sequences side effect.
        let builder = PtySessionBuilder::new("cargo")
            .with_config(DefaultPtySessionConfig + PtySessionConfigOption::CaptureOsc);

        assert_eq!(
            builder.env_vars.get("CARGO_TERM_PROGRESS_WHEN").unwrap(),
            "always"
        );
        assert_eq!(builder.env_vars.get("TERM").unwrap(), "xterm-256color");

        // with_config(NoCaptureOsc) should NOT trigger the side effect.
        let builder = PtySessionBuilder::new("cargo")
            .with_config(DefaultPtySessionConfig + PtySessionConfigOption::NoCaptureOsc);

        assert!(builder.env_vars.get("CARGO_TERM_PROGRESS_WHEN").is_none());
        assert!(builder.env_vars.get("TERM").is_none());
    }
}
