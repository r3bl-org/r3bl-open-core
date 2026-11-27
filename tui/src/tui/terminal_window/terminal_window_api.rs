// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{BoxedSafeApp, GlobalData, try_main_event_loop_impl};
use crate::{FlexBoxId, InputDevice, InputEvent, OutputDevice, RawModeError,
            TerminalIoError, TerminalNotInteractiveReason, TerminalSizeError,
            try_ensure_terminal_interactive, try_get_size};
use std::{fmt::{Debug, Display},
          pin::Pin};

#[derive(Debug)]
pub struct TerminalWindow;

/// Errors that can occur while initializing or running [`TerminalWindow::try_main_event_loop`].
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum MainEventLoopError {
    /// Terminal streams ([`stdin`] / [`stdout`]) are not interactive.
    ///
    /// [`stdin`]: std::io::stdin
    /// [`stdout`]: std::io::stdout
    #[error(transparent)]
    #[diagnostic(transparent)]
    NotInteractive(TerminalNotInteractiveReason),

    /// Failed to query the initial terminal window dimensions.
    #[error(transparent)]
    #[diagnostic(transparent)]
    TerminalSize(TerminalSizeError),

    /// Failed to enable terminal raw mode.
    #[error(transparent)]
    #[diagnostic(transparent)]
    RawMode(RawModeError),

    /// Failed to enter full-screen alternate-screen TUI mode.
    #[error(transparent)]
    #[diagnostic(transparent)]
    FullScreenSetup(TerminalIoError),
}

/// Type alias for the pinned boxed future returned by the
/// [`TerminalWindow::try_main_event_loop`] function.
///
/// This represents a pinned boxed future that resolves to a [`Result`] containing
/// the main event loop's output: global data, input device, and output device, or a
/// [`MainEventLoopError`].
///
/// See [Core Async Concepts] for more information on the async concepts used here.
///
/// [Core Async Concepts]: crate::try_main_event_loop_impl#core-async-concepts-pin-and-unpin
pub type MainEventLoopFuture<S, AS> = Pin<
    Box<
        dyn Future<
            Output = Result<
                (
                    /* global_data */ GlobalData<S, AS>,
                    /* event stream */ InputDevice,
                    /* stdout */ OutputDevice,
                ),
                MainEventLoopError,
            >,
        >,
    >,
>;

#[derive(Debug)]
pub enum TerminalWindowMainThreadSignal<AS>
where
    AS: Debug + Default + Clone + Sync + Send,
{
    /// Exit the main event loop.
    Exit,
    /// Render the app.
    Render(Option<FlexBoxId>),
    /// Apply an app signal to the app.
    ApplyAppSignal(AS),
}

impl TerminalWindow {
    /// This is the public API for the main event loop of the terminal window.
    ///
    /// This is the public API, and not the internal API [`try_main_event_loop_impl()`].
    /// This separation exists to allow for testing using dependency injection.
    ///
    /// Returns a [`MainEventLoopFuture`] that checks terminal interactivity, queries the
    /// initial terminal size, creates input/output devices, and runs the async event loop.
    ///
    /// The main event loop is responsible for handling all input events, and dispatching
    /// them to the [`App`] for processing. It is also responsible for rendering the
    /// [`App`] after each input event. It is also responsible for handling all signals
    /// sent from the [`App`] to the main event loop (eg: `request_shutdown`, re-render,
    /// apply app signal, etc).
    ///
    /// # Arguments
    ///
    /// * `app` - The [`BoxedSafeApp`] instance that will handle input events and signals.
    /// * `exit_keys` - A slice of [`InputEvent`]s that will trigger application exit.
    /// * `state` - The initial application state.
    ///
    /// # Returns
    ///
    /// Returns a [`MainEventLoopFuture`] that resolves to `Ok(...)` with:
    /// * `global_data` - The final [`GlobalData`] state after the event loop exits.
    /// * `event_stream` - The [`InputDevice`] used for input events.
    /// * [`stdout`] - The [`OutputDevice`] used for output.
    ///
    /// # Performance Note
    ///
    /// The state type `S` must implement [`Display`] for telemetry logging. This
    /// implementation is called after EVERY render cycle in the main event loop, so it
    /// must be lightweight and efficient. Avoid expensive operations like:
    /// * Deep recursive traversal of data structures.
    /// * Memory size calculations.
    /// * Complex string formatting.
    ///
    /// Instead, implement a simple summary format that shows only essential metrics. For
    /// example:
    ///
    /// ```no_run
    /// use std::fmt::Display;
    ///
    /// struct MyState {
    ///     buffers: Vec<String>,
    ///     active_id: usize,
    /// }
    ///
    /// impl Display for MyState {
    ///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         write!(f, "State[buffers={}, active={}]", self.buffers.len(), self.active_id)
    ///     }
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// The returned future resolves to [`MainEventLoopError`] if:
    /// * Terminal streams are not interactive ([`MainEventLoopError::NotInteractive`]).
    /// * Terminal dimensions cannot be queried ([`MainEventLoopError::TerminalSize`]).
    /// * Terminal raw mode cannot be enabled ([`MainEventLoopError::RawMode`]).
    /// * Full-screen TUI mode setup fails ([`MainEventLoopError::FullScreenSetup`]).
    ///
    /// # Other entry points for interactive terminal apps
    ///
    /// See [interactive terminal application entry points].
    ///
    /// [`App`]: crate::tui::App
    /// [`stdout`]: std::io::stdout
    /// [`try_main_event_loop_impl()`]: crate::try_main_event_loop_impl()
    /// [interactive terminal application entry points]: crate#interactive-terminal-application-entry-points
    #[must_use]
    pub fn try_main_event_loop<S, AS>(
        app: BoxedSafeApp<S, AS>,
        exit_keys: &[InputEvent],
        state: S,
    ) -> MainEventLoopFuture<S, AS>
    where
        S: Display + Debug + Default + Clone + Sync + Send + 'static,
        AS: Debug + Default + Clone + Sync + Send + 'static,
    {
        let exit_keys = exit_keys.to_vec();
        Box::pin(async move {
            try_ensure_terminal_interactive()
                .map_err(MainEventLoopError::NotInteractive)?;
            let initial_size = try_get_size().map_err(MainEventLoopError::TerminalSize)?;
            let input_device = InputDevice::default();
            let output_device = OutputDevice::new_stdout();

            try_main_event_loop_impl(
                app,
                exit_keys,
                state,
                initial_size,
                input_device,
                output_device,
            )
            .await
        })
    }
}
