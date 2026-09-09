// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{config::{CaptureFlag, DefaultPtySessionConfig, PtySessionConfig},
            events::{PtyInputEvent, PtyOutputEvent},
            session::{AsyncPtySession, PtySession},
            threads::orchestrator::spawn_pty_orchestrator_thread};
use crate::{DefaultSize, PtyCommand, PtyPair};
use miette::{IntoDiagnostic, miette};
use rustc_hash::FxHashMap;
use std::path::PathBuf;

/// Builder for configuring and spawning [`PTY`] sessions.
///
/// This provides a clean interface for building terminal commands with arguments,
/// environment variables, and current working directory.
///
/// In order to use this, you must:
/// 1. Create a [`PtySessionBuilder`] using [`new()`].
/// 2. (Optional) Configure the session using [`with_config()`].
/// 3. Call [`start()`] to obtain a synchronous [`PtySession`], or [`start_async()`] for
///    an asynchronous [`AsyncPtySession`] adapter.
///
/// For an architectural overview of how this fits into the [`PTY`] stack, the lifecycle
/// diagram, and the standard [`tokio::select!`] usage pattern, see the [Session Layer]
/// documentation.
///
/// # Examples
///
/// ### Synchronous Session (Default Core)
///
/// ```
/// # #[cfg(not(unix))]
/// # fn main() {}
/// # #[cfg(unix)]
/// use r3bl_tui::{DefaultPtySessionConfig, PtySessionBuilder, PtySessionConfigToken};
///
/// # #[cfg(unix)]
/// fn main() -> miette::Result<()> {
///     let mut session = PtySessionBuilder::new("echo")
///         .cli_arg("hello")
///         .env_var("TERM", "xterm-256color")
///         .cwd("/tmp")
///         .with_config(
///             DefaultPtySessionConfig
///                 + PtySessionConfigToken::NoCaptureOutput,
///         )
///         .start()?;
///     Ok(())
/// }
/// ```
///
/// ### Asynchronous Session (Adapter)
///
/// ```
/// # #[cfg(not(unix))]
/// # fn main() {}
/// # #[cfg(unix)]
/// use r3bl_tui::{DefaultPtySessionConfig, PtySessionBuilder, PtySessionConfigToken};
///
/// # #[cfg(unix)]
/// #[tokio::main]
/// # #[cfg(unix)]
/// async fn main() -> miette::Result<()> {
///     let mut session = PtySessionBuilder::new("echo")
///         .cli_arg("hello")
///         .env_var("TERM", "xterm-256color")
///         .cwd("/tmp")
///         .with_config(
///             DefaultPtySessionConfig
///                 + PtySessionConfigToken::NoCaptureOutput,
///         )
///         .start_async()?;
///     Ok(())
/// }
/// ```
///
/// [`new()`]: Self::new()
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`start()`]: Self::start()
/// [`start_async()`]: Self::start_async()
/// [`with_config()`]: Self::with_config()
/// [Session Layer]: mod@crate::pty_session
#[derive(Debug, Clone)]
pub struct PtySessionBuilder {
    /// The executable command to run (e.g., [`bash`] or [`ls`]).
    ///
    /// [`bash`]: https://en.wikipedia.org/wiki/Bash_(Unix_shell)
    /// [`ls`]: https://en.wikipedia.org/wiki/ls
    pub command: String,

    /// Command-line arguments to pass to the executable.
    pub cli_args: Vec<String>,

    /// Environment variables to set for the child process.
    pub env_vars: FxHashMap<String, String>,

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

impl PtySessionBuilder {
    /// Creates a new builder for the specified command.
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            cli_args: Vec::new(),
            env_vars: FxHashMap::default(),
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
    pub fn env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
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
    /// Note that if [`PtySessionConfigToken::CaptureOsc`] is provided, then
    /// [`PtySessionBuilder::env_vars`] will be updated to include environment
    /// variables that trigger [`OSC`] emission from [`cargo`] and [`rustup`].
    ///
    /// [`cargo`]: https://github.com/rust-lang/cargo
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [`PtySessionConfigToken::CaptureOsc`]: crate::PtySessionConfigToken::CaptureOsc
    /// [`rustup`]: https://rust-lang.github.io/rustup/
    #[must_use]
    pub fn with_config(mut self, arg_config: impl Into<PtySessionConfig>) -> Self {
        // Replace the default config w/ the one that's provided here.
        let config = arg_config.into();
        self.config = config;

        // Use the config to enable OSC sequences on this builder.
        if config.capture_osc == CaptureFlag::Capture {
            self.enable_osc_sequences();
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
    /// [`PtyPair`] and the [Background Threads] needed for full bidirectional I/O.
    ///
    /// This function handles the entire initialization sequence:
    /// 1. Creates a synchronous bounded channel for output events (sized to
    ///    [`DefaultSize::PtyChannelBufferSize`]).
    /// 2. Creates a synchronous bounded channel for input events (sized to
    ///    [`DefaultSize::PtyChannelBufferSize`]).
    /// 3. Spawns the child process in a [`PtyPair`] (Engine Layer).
    /// 4. Spawns a [Reader Thread] to drain output from the process.
    /// 5. Spawns a [Writer Thread] to pump input events to the process.
    /// 6. Spawns an [Orchestrator Thread] to monitor the child process's lifecycle.
    ///
    /// For details on how synchronous backpressure throttles the child process, see
    /// the [Backpressure Architecture]. For an architectural overview of how this
    /// fits into the [`PTY`] stack and the lifecycle diagram, see the [Session Layer]
    /// documentation.
    ///
    /// # Examples
    ///
    /// Starting a synchronous session, draining output, and waiting for exit:
    ///
    /// ```
    /// # #[cfg(not(unix))]
    /// # fn main() {}
    /// # #[cfg(unix)]
    /// use r3bl_tui::{ok, PtyOutputEvent, PtySessionBuilder};
    ///
    /// # #[cfg(unix)]
    /// fn main() -> miette::Result<()> {
    ///     let mut session = PtySessionBuilder::new("echo")
    ///         .cli_arg("hello")
    ///         .start()?;
    ///
    ///     while let Ok(event) = session.rx_output_event.recv() {
    ///         match event {
    ///             PtyOutputEvent::Output(_bytes) => { /* render bytes */ }
    ///             PtyOutputEvent::Exit(_status) => { break; }
    ///             _ => {}
    ///         }
    ///     }
    ///
    ///     let _status = session.orchestrator_task_handle.join();
    ///     ok!()
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The command cannot be built from the [`PtySessionBuilder`].
    /// - The [`PtyPair`] fails to open or the child process fails to spawn.
    /// - The controller reader cannot be cloned for the background thread.
    ///
    /// [`DefaultSize::PtyChannelBufferSize`]:
    ///     crate::DefaultSize::PtyChannelBufferSize
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [Background Threads]: crate::core::pty#the-thread-trio
    /// [Backpressure Architecture]: crate::core::pty#backpressure-architecture
    /// [Orchestrator Thread]: crate::threads::spawn_pty_orchestrator_thread
    /// [Reader Thread]: crate::threads::spawn_pty_reader_thread
    /// [Session Layer]: mod@crate::pty_session
    /// [Writer Thread]: crate::threads::spawn_pty_writer_thread
    pub fn start(self) -> miette::Result<PtySession> {
        use std::sync::mpsc::sync_channel;

        // Channel for output events (child process → app).
        let (output_event_ch_tx_half, output_event_ch_rx_half) =
            sync_channel::<PtyOutputEvent>(DefaultSize::PtyChannelBufferSize.into());

        // Channel for input events (app → child process).
        let (input_event_ch_tx_half, input_event_ch_rx_half) =
            sync_channel::<PtyInputEvent>(DefaultSize::PtyChannelBufferSize.into());

        let command = self.build()?;

        let (pty_pair, controlled_child) =
            PtyPair::open_and_spawn(self.config.pty_size, command)?;

        let child_process_termination_handle =
            controlled_child.clone_termination_handle();

        let orchestrator_task_handle = spawn_pty_orchestrator_thread(
            controlled_child,
            pty_pair.into_controller(),
            input_event_ch_tx_half.clone(),
            input_event_ch_rx_half,
            output_event_ch_tx_half,
            self.config,
        )?;

        Ok(PtySession {
            tx_input_event: input_event_ch_tx_half,
            rx_output_event: output_event_ch_rx_half,
            orchestrator_task_handle,
            child_process_termination_handle,
        })
    }

    /// Start an asynchronous [`PTY`] session from the given [`PtySessionBuilder`]
    /// configuration.
    ///
    /// This creates an [`AsyncPtySession`] backed by Tokio channels and bridge
    /// tasks, designed for integration with [`tokio::select!`]. Bridge tasks are
    /// spawned to forward events between Tokio channels and the underlying
    /// synchronous session threads.
    ///
    /// # Examples
    ///
    /// Starting an asynchronous session and driving it with a [`tokio::select!`] loop:
    ///
    /// ```
    /// # #[cfg(not(unix))]
    /// # fn main() {}
    /// # #[cfg(unix)]
    /// use r3bl_tui::{ok, PtyOutputEvent, PtySessionBuilder};
    ///
    /// # #[cfg(unix)]
    /// #[tokio::main]
    /// # #[cfg(unix)]
    /// async fn main() -> miette::Result<()> {
    ///     let mut session = PtySessionBuilder::new("echo")
    ///         .cli_arg("hello")
    ///         .start_async()?;
    ///
    ///     loop {
    ///         tokio::select! {
    ///             // 1. Handle output from the PTY.
    ///             Some(event) = session.rx_output_event.recv() => {
    ///                 match event {
    ///                     PtyOutputEvent::Output(_bytes) => { /* render bytes */ }
    ///                     PtyOutputEvent::Exit(_status) => { break; }
    ///                     _ => {}
    ///                 }
    ///             }
    ///             // 2. Await process orchestration and completion.
    ///             _status = &mut session.orchestrator_task_handle => {
    ///                 break;
    ///             }
    ///         }
    ///     }
    ///     ok!()
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`miette::Error`] if starting the underlying synchronous session
    /// fails.
    ///
    /// [`AsyncPtySession`]: crate::AsyncPtySession
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [`tokio::select!`]: tokio::select
    pub fn start_async(self) -> miette::Result<AsyncPtySession> {
        use tokio::{sync::mpsc::channel, task::spawn_blocking};

        let sync_session = self.start()?;

        let (async_input_tx, mut async_input_rx) =
            channel::<PtyInputEvent>(DefaultSize::PtyChannelBufferSize.into());

        let (async_output_tx, async_output_rx) =
            channel::<PtyOutputEvent>(DefaultSize::PtyChannelBufferSize.into());

        // Bridge input: async Tokio channel -> sync sender.
        let _input_bridge_handle = spawn_blocking({
            // Partial move of field (since PtySession does not impl Drop).
            let sync_input_tx = sync_session.tx_input_event;

            move || {
                while let Some(event) = async_input_rx.blocking_recv() {
                    if sync_input_tx.send(event).is_err() {
                        break;
                    }
                }
            }
        });

        // Bridge output: sync receiver -> async Tokio channel.
        let _output_bridge_handle = spawn_blocking({
            // Partial move of field (since PtySession does not impl Drop).
            let sync_output_rx = sync_session.rx_output_event;

            move || {
                while let Ok(event) = sync_output_rx.recv() {
                    if async_output_tx.blocking_send(event).is_err() {
                        break;
                    }
                }
            }
        });

        // Bridge orchestrator: join thread -> tokio task JoinHandle.
        let orchestrator_task_handle = spawn_blocking({
            // Partial move of field (since PtySession does not impl Drop).
            let handle = sync_session.orchestrator_task_handle;

            move || {
                handle
                    .join()
                    .map_err(|e| miette!("Orchestrator thread panicked: {e:?}"))?
            }
        });

        // Final move of remaining field (since PtySession does not impl Drop).
        let child_process_termination_handle =
            sync_session.child_process_termination_handle;

        Ok(AsyncPtySession {
            tx_input_event: async_input_tx,
            rx_output_event: async_output_rx,
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
    /// sequences by setting the following environment variables on the child process:
    ///
    /// - **[`CARGO_TERM_PROGRESS_WHEN=always`]**: forces cargo to always emit its
    ///   progress bar, bypassing its own heuristics about whether a progress bar is
    ///   appropriate. Without this, cargo may suppress progress output even inside a
    ///   [`PTY`], for example when it detects a non-interactive session or a dumb
    ///   terminal.
    ///
    /// - **[`TERM=xterm-256color`]**: signals that the terminal supports modern escape
    ///   sequences, including [`OSC`] and hyperlinks. Notably, this value passes the
    ///   terminal capability [exclusion list] in
    ///   [`examine_env_vars_to_determine_hyperlink_support()`], where plain `"xterm"` is
    ///   excluded. It also ensures cargo trusts the terminal enough to emit [`OSC`]
    ///   sequences rather than falling back to plain text output.
    ///
    /// The emitted [`OSC`] bytes are parsed downstream by [`OscBuffer`] into [`OscEvent`]
    /// variants such as [`ProgressUpdate(u8)`], and delivered to the caller via the
    /// [`PTY`] session's MPSC channel.
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
    pub fn enable_osc_sequences(&mut self) {
        self.env_vars
            .insert("CARGO_TERM_PROGRESS_WHEN".to_string(), "always".to_string());
        self.env_vars
            .insert("TERM".to_string(), "xterm-256color".to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DefaultPtySessionConfig, PtySessionConfigToken};

    #[test]
    fn test_builder_pattern() {
        let sz = crate::vp_width(80) + crate::vp_height(24);
        let builder = PtySessionBuilder::new("bash")
            .cli_args(["-c", "ls"])
            .cli_arg("-la")
            .env_vars([("KEY1", "VAL1"), ("KEY2", "VAL2")])
            .env_var("KEY3", "VAL3")
            .cwd("/tmp")
            .with_config(
                DefaultPtySessionConfig
                    + PtySessionConfigToken::Size(sz)
                    + PtySessionConfigToken::CaptureOsc,
            );

        assert_eq!(builder.command, "bash");
        assert_eq!(builder.cli_args, vec!["-c", "ls", "-la"]);
        assert_eq!(
            builder.env_vars.get("KEY1").expect("conversion error"),
            "VAL1"
        );
        assert_eq!(
            builder.env_vars.get("KEY2").expect("conversion error"),
            "VAL2"
        );
        assert_eq!(
            builder.env_vars.get("KEY3").expect("conversion error"),
            "VAL3"
        );
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
            .with_config(DefaultPtySessionConfig + PtySessionConfigToken::CaptureOsc);

        assert_eq!(builder.maybe_cwd, Some(PathBuf::from("/")));
        assert_eq!(builder.config.capture_osc, CaptureFlag::Capture);
    }

    #[test]
    fn test_enable_osc_sequences() {
        let mut builder = PtySessionBuilder::new("cargo");
        builder.enable_osc_sequences();

        assert_eq!(
            builder
                .env_vars
                .get("CARGO_TERM_PROGRESS_WHEN")
                .expect("conversion error"),
            "always"
        );
        assert_eq!(
            builder.env_vars.get("TERM").expect("conversion error"),
            "xterm-256color"
        );
    }

    #[allow(clippy::unnecessary_get_then_check)]
    #[test]
    fn test_with_config_osc_side_effect() {
        // with_config(CaptureOsc) should trigger enable_osc_sequences side effect.
        let builder = PtySessionBuilder::new("cargo")
            .with_config(DefaultPtySessionConfig + PtySessionConfigToken::CaptureOsc);

        assert_eq!(
            builder
                .env_vars
                .get("CARGO_TERM_PROGRESS_WHEN")
                .expect("conversion error"),
            "always"
        );
        assert_eq!(
            builder.env_vars.get("TERM").expect("conversion error"),
            "xterm-256color"
        );

        // with_config(NoCaptureOsc) should NOT trigger the side effect.
        let builder = PtySessionBuilder::new("cargo")
            .with_config(DefaultPtySessionConfig + PtySessionConfigToken::NoCaptureOsc);

        assert!(builder.env_vars.get("CARGO_TERM_PROGRESS_WHEN").is_none());
        assert!(builder.env_vars.get("TERM").is_none());
    }
}
