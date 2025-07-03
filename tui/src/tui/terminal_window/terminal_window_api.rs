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

use std::fmt::Debug;

use super::{main_event_loop_impl, BoxedSafeApp, GlobalData};
use crate::{get_size, CommonResult, FlexBoxId, InputDevice, InputEvent, OutputDevice};

#[derive(Debug)]
pub struct TerminalWindow;

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
    /// This is the main event loop for the entire application. It is responsible for
    /// handling all input events, and dispatching them to the [`crate::App`] for
    /// processing. It is also responsible for rendering the [`crate::App`] after each
    /// input event. It is also responsible for handling all signals sent from the
    /// [`crate::App`] to the main event loop (eg: `request_shutdown`, re-render,
    /// apply app signal, etc).
    pub async fn main_event_loop<S, AS>(
        app: BoxedSafeApp<S, AS>,
        exit_keys: &[InputEvent],
        state: S,
    ) -> CommonResult<(
        /* global_data */ GlobalData<S, AS>,
        /* event stream */ InputDevice,
        /* stdout */ OutputDevice,
    )>
    where
        S: Debug + Default + Clone + Sync + Send,
        AS: Debug + Default + Clone + Sync + Send + 'static,
    {
        let initial_size = get_size()?;
        let input_device = InputDevice::new_event_stream();
        let output_device = OutputDevice::new_stdout();

        main_event_loop_impl(
            app,
            exit_keys,
            state,
            initial_size,
            input_device,
            output_device,
        )
        .await
    }
}
