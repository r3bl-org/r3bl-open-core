/*
 *   Copyright (c) 2023 R3BL LLC
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
use std::io::{Result, *};

use crossterm::{cursor::*, execute, terminal::*};

use crate::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventLoopResult {
    Continue,
    ContinueAndRerender,
    ExitWithResult(Vec<String>),
    ExitWithoutResult,
    ExitWithError,
    Select,
}

pub fn enter_event_loop<W: Write, S:ViewportHeight>(
    state: &mut S,
    function_component: &mut impl FunctionComponent<W, S>,
    on_keypress: impl Fn(&mut S, KeyPress) -> EventLoopResult,
) -> Result<EventLoopResult> {
    let mut shared_global_data = SharedGlobalData::try_to_create_inline_instance(state).unwrap();
    execute!(function_component.get_write(), Hide)?;
    // Start raw mode
    enable_raw_mode()?;

    // Use to handle clean up.
    #[allow(unused_assignments)]
        let mut maybe_return_this: Option<EventLoopResult> = None;

    // Only required for the first time to clean up the terminal,
    // and place the cursor at the correct position.
    function_component.allocate_viewport_height_space(state)?;

    loop {
        function_component.render(state, &mut shared_global_data)?;
        let key_event = read_key_press();
        match on_keypress(state, key_event) {
            EventLoopResult::ContinueAndRerender => {
                // Continue the loop.
                function_component.render(state, &mut shared_global_data)?;
            }
            EventLoopResult::Continue | EventLoopResult::Select => {
                // Noop. Simply continue the loop.
            }
            EventLoopResult::ExitWithResult(it) => {
                // Break the loop and return the result.
                maybe_return_this = Some(EventLoopResult::ExitWithResult(it));
                function_component.clear_viewport(state)?;
                break;
            }
            EventLoopResult::ExitWithoutResult => {
                // Break the loop and return the result.
                maybe_return_this = Some(EventLoopResult::ExitWithoutResult);
                function_component.clear_viewport(state)?;
                break;
            }
            EventLoopResult::ExitWithError => {
                maybe_return_this = Some(EventLoopResult::ExitWithError);
                function_component.clear_viewport(state)?;
                break;
            }
        }
    }

    // Perform cleanup of raw mode, and show cursor.
    match maybe_return_this {
        Some(it) => {
            execute!(function_component.get_write(), Show)?;
            disable_raw_mode()?;
            Ok(it)
        }
        None => Ok(EventLoopResult::ExitWithoutResult),
    }
}
