/*
 *   Copyright (c) 2024-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use std::{fmt::Debug, future::Future, pin::Pin};

use super::{main_event_loop_impl, BoxedSafeApp, GlobalData};
use crate::{get_size, CommonResult, FlexBoxId, InputDevice, InputEvent, OutputDevice};

#[derive(Debug)]
pub struct TerminalWindow;

/// Type alias for the boxed future returned by the [`TerminalWindow::main_event_loop`]
/// function.
///
/// This represents a [`Pin`]ned [`Box`]ed future that resolves to a [`CommonResult`]
/// containing the main event loop's output: global data, input device, and output device.
pub type MainEventLoopFuture<'a, S, AS> = Pin<
    Box<
        dyn Future<
                Output = CommonResult<(
                    /* global_data */ GlobalData<S, AS>,
                    /* event stream */ InputDevice,
                    /* stdout */ OutputDevice,
                )>,
            > + 'a,
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
    /// This is the public API, and not the internal API
    /// [`super::main_event_loop_impl()`]. This separation exists to allow for
    /// testing using dependency injection.
    ///
    /// This function handles the synchronous initialization (getting terminal size,
    /// creating input/output devices) and then returns a [`MainEventLoopFuture`] that
    /// contains the actual async event loop implementation.
    ///
    /// The main event loop is responsible for handling all input events, and dispatching
    /// them to the [`crate::App`] for processing. It is also responsible for rendering
    /// the [`crate::App`] after each input event. It is also responsible for handling
    /// all signals sent from the [`crate::App`] to the main event loop (eg:
    /// `request_shutdown`, re-render, apply app signal, etc).
    ///
    /// # Arguments
    ///
    /// * `app` - The [`BoxedSafeApp`] instance that will handle input events and signals.
    /// * `exit_keys` - A slice of [`InputEvent`]s that will trigger application exit.
    /// * `state` - The initial application state.
    ///
    /// # Returns
    ///
    /// Returns a [`miette::Result`] containing a [`MainEventLoopFuture`] that resolves
    /// to a [`CommonResult`] with:
    /// * `global_data` - The final [`GlobalData`] state after the event loop exits.
    /// * `event_stream` - The [`InputDevice`] used for input events.
    /// * `stdout` - The [`OutputDevice`] used for output.
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
    pub fn main_event_loop<'a, S, AS>(
        app: BoxedSafeApp<S, AS>,
        exit_keys: &'a [InputEvent],
        state: S,
    ) -> miette::Result<MainEventLoopFuture<'a, S, AS>>
    where
        S: Debug + Default + Clone + Sync + Send + 'a,
        AS: Debug + Default + Clone + Sync + Send + 'static,
    {
        let initial_size = get_size()?;
        let input_device = InputDevice::new_event_stream();
        let output_device = OutputDevice::new_stdout();

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
