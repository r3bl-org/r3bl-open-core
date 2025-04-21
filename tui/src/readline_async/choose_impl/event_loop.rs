/*
 *   Copyright (c) 2023-2025 R3BL LLC
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
use std::io::Result;

use crossterm::{cursor::{Hide, Show},
                terminal::{disable_raw_mode, enable_raw_mode}};

use super::KeyPressReader;
use crate::{execute_commands,
            return_if_not_interactive_terminal,
            CalculateResizeHint,
            FunctionComponent,
            InputDevice,
            InputDeviceExt,
            InputEvent,
            ItemsOwned,
            TTYResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventLoopResult {
    Continue,
    ContinueAndRerender,
    ContinueAndRerenderAndClear,
    ExitWithResult(ItemsOwned),
    ExitWithoutResult,
    ExitWithError,
    Select,
}

pub async fn enter_event_loop_async<S: CalculateResizeHint>(
    state: &mut S,
    function_component: &mut impl FunctionComponent<S>,
    on_keypress: impl Fn(&mut S, InputEvent) -> EventLoopResult,
    input_device: &mut InputDevice,
) -> Result<EventLoopResult> {
    return_if_not_interactive_terminal!(Ok(EventLoopResult::ExitWithError));

    run_before_event_loop(state, function_component)?;

    let return_this: EventLoopResult;

    loop {
        let maybe_ie = input_device.next_input_event().await;
        match maybe_ie {
            Some(ie) => {
                let event_loop_result = on_keypress(state, ie);
                if let Some(result) =
                    handle_event_loop_result(function_component, event_loop_result, state)
                {
                    return_this = result;
                    break;
                }
            }
            None => {
                return_this = EventLoopResult::ExitWithError;
                function_component.clear_viewport(state)?;
                break;
            }
        }
    }

    run_after_event_loop(function_component)?;

    Ok(return_this)
}

pub fn enter_event_loop_sync<S: CalculateResizeHint>(
    state: &mut S,
    function_component: &mut impl FunctionComponent<S>,
    on_keypress: impl Fn(&mut S, InputEvent) -> EventLoopResult,
    key_press_reader: &mut impl KeyPressReader,
) -> Result<EventLoopResult> {
    return_if_not_interactive_terminal!(Ok(EventLoopResult::ExitWithError));

    run_before_event_loop(state, function_component)?;

    let return_this: EventLoopResult;

    loop {
        let maybe_ie = key_press_reader.read_key_press();
        match maybe_ie {
            Some(ie) => {
                if let Some(result) = handle_event_loop_result(
                    function_component,
                    on_keypress(state, ie),
                    state,
                ) {
                    return_this = result;
                    break;
                }
            }
            None => {
                return_this = EventLoopResult::ExitWithError;
                function_component.clear_viewport(state)?;
                break;
            }
        }
    }

    run_after_event_loop(function_component)?;

    Ok(return_this)
}

fn run_before_event_loop<S: CalculateResizeHint>(
    state: &mut S,
    function_component: &mut impl FunctionComponent<S>,
) -> Result<()> {
    execute_commands!(function_component.get_output_device(), Hide);
    enable_raw_mode()?;

    // First render before blocking the main thread for user input.
    function_component.render(state)?;

    Ok(())
}

fn run_after_event_loop<S: CalculateResizeHint>(
    function_component: &mut impl FunctionComponent<S>,
) -> Result<()> {
    execute_commands!(function_component.get_output_device(), Show);
    disable_raw_mode()?;
    Ok(())
}

fn handle_event_loop_result<S: CalculateResizeHint>(
    function_component: &mut impl FunctionComponent<S>,
    result: EventLoopResult,
    state: &mut S,
) -> Option<EventLoopResult> {
    match result {
        EventLoopResult::ContinueAndRerenderAndClear => {
            function_component.clear_viewport_for_resize(state).ok()?;
            function_component.render(state).ok()?;
            None
        }
        EventLoopResult::ContinueAndRerender => {
            function_component.render(state).ok()?;
            None
        }
        EventLoopResult::Continue | EventLoopResult::Select => None,
        EventLoopResult::ExitWithResult(it) => {
            function_component.clear_viewport(state).ok()?;
            Some(EventLoopResult::ExitWithResult(it))
        }
        EventLoopResult::ExitWithoutResult => {
            function_component.clear_viewport(state).ok()?;
            Some(EventLoopResult::ExitWithoutResult)
        }
        EventLoopResult::ExitWithError => {
            function_component.clear_viewport(state).ok()?;
            Some(EventLoopResult::ExitWithError)
        }
    }
}
