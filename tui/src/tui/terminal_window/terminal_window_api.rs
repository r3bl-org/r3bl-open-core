// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{BoxedSafeApp, GlobalData, main_event_loop_impl};
use crate::{CommonResult, FlexBoxId, InputDevice, InputEvent, OutputDevice, get_size};
use std::{fmt::{Debug, Display},
          pin::Pin};

#[derive(Debug)]
pub struct TerminalWindow;

/// Type alias for the pinned boxed future returned by the
/// [`TerminalWindow::main_event_loop`] function.
///
/// This represents a pinned boxed future that resolves to a [`CommonResult`] containing
/// the main event loop's output: global data, input device, and output device.
///
/// See [Core Async Concepts] for more information on the async concepts used here.
///
/// [Core Async Concepts]: crate::main_event_loop_impl#core-async-concepts-pin-and-unpin
pub type MainEventLoopFuture<S, AS> = Pin<
    Box<
        dyn Future<
            Output = CommonResult<(
                /* global_data */ GlobalData<S, AS>,
                /* event stream */ InputDevice,
                /* stdout */ OutputDevice,
            )>,
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
    /// This is the public API, and not the internal API [`main_event_loop_impl()`]. This
    /// separation exists to allow for testing using dependency injection.
    ///
    /// This function handles the synchronous initialization (getting terminal size,
    /// creating input/output devices) and then returns a [`MainEventLoopFuture`] that
    /// contains the actual async event loop implementation.
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
    /// Returns a [`miette::Result`] containing a [`MainEventLoopFuture`] that resolves to
    /// a [`CommonResult`] with:
    /// * `global_data` - The final [`GlobalData`] state after the event loop exits.
    /// * `event_stream` - The [`InputDevice`] used for input events.
    /// * `stdout` - The [`OutputDevice`] used for output.
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
    /// Returns [`miette::Error`] if there are errors during:
    /// * Terminal initialization (getting initial size).
    ///
    /// The returned future may produce [`miette::Error`] during:
    /// * Input/output device creation.
    /// * Event loop execution (input processing, rendering, signal handling).
    /// * Terminal cleanup and restoration.
    ///
    /// [`App`]: crate::App
    /// [`main_event_loop_impl()`]: crate::main_event_loop_impl()
    pub fn main_event_loop<S, AS>(
        app: BoxedSafeApp<S, AS>,
        exit_keys: &[InputEvent],
        state: S,
    ) -> miette::Result<MainEventLoopFuture<S, AS>>
    where
        S: Display + Debug + Default + Clone + Sync + Send + 'static,
        AS: Debug + Default + Clone + Sync + Send + 'static,
    {
        let initial_size = get_size()?;
        let input_device = InputDevice::default();
        let output_device = OutputDevice::new_stdout();
        let exit_keys = exit_keys.to_vec();

        Ok(main_event_loop_impl(
            app,
            exit_keys,
            state,
            initial_size,
            input_device,
            output_device,
        ))
    }
}
