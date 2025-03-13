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
use std::io::{Result, Write};

use crossterm::{cursor::{Hide, Show},
                execute,
                terminal::{disable_raw_mode, enable_raw_mode}};
use r3bl_core::{is_fully_uninteractive_terminal, TTYResult};

use crate::{CalculateResizeHint, FunctionComponent, KeyPress, KeyPressReader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventLoopResult {
    Continue,
    ContinueAndRerender,
    ContinueAndRerenderAndClear,
    ExitWithResult(Vec<String>),
    ExitWithoutResult,
    ExitWithError,
    Select,
}

pub fn enter_event_loop<W: Write, S: CalculateResizeHint>(
    state: &mut S,
    function_component: &mut impl FunctionComponent<W, S>,
    on_keypress: impl Fn(&mut S, KeyPress) -> EventLoopResult,
    reader: &mut impl KeyPressReader,
) -> Result<EventLoopResult> {
    // Don't block tests.
    if let TTYResult::IsNotInteractive = is_fully_uninteractive_terminal() {
        return Ok(EventLoopResult::ExitWithError);
    }

    execute!(function_component.get_write(), Hide)?;
    enable_raw_mode()?;

    // Use to handle clean up.
    let return_this: EventLoopResult;

    // First render before blocking the main thread for user input.
    function_component.render(state)?;

    loop {
        let key_press = reader.read_key_press();
        let result = on_keypress(state, key_press);
        match result {
            EventLoopResult::ContinueAndRerenderAndClear => {
                // Clear the viewport.
                function_component.clear_viewport_for_resize(state)?;
                // Repaint the viewport.
                function_component.render(state)?;
            }
            EventLoopResult::ContinueAndRerender => {
                // Continue the loop.
                function_component.render(state)?;
            }
            EventLoopResult::Continue | EventLoopResult::Select => {
                // Noop. Simply continue the loop.
            }
            EventLoopResult::ExitWithResult(it) => {
                // Break the loop and return the result.
                return_this = EventLoopResult::ExitWithResult(it);
                function_component.clear_viewport(state)?;
                break;
            }
            EventLoopResult::ExitWithoutResult => {
                // Break the loop and return the result.
                return_this = EventLoopResult::ExitWithoutResult;
                function_component.clear_viewport(state)?;
                break;
            }
            EventLoopResult::ExitWithError => {
                return_this = EventLoopResult::ExitWithError;
                function_component.clear_viewport(state)?;
                break;
            }
        }
    }

    // Perform cleanup of raw mode, and show cursor.
    execute!(function_component.get_write(), Show)?;
    disable_raw_mode()?;
    Ok(return_this)
}
