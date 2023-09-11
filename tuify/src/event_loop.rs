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
}

// TODO: add performance using output buffer
pub fn enter_event_loop<W: Write, S>(
    state: &mut S,
    function_component: &mut impl FunctionComponent<W, S>,
    on_keypress: impl Fn(&mut S, KeyPress) -> EventLoopResult,
) -> Result<EventLoopResult> {
    execute!(function_component.get_write(), Hide)?;
    enable_raw_mode()?;

    // Use to handle clean up.
    #[allow(unused_assignments)]
    let mut maybe_return_this: Option<EventLoopResult> = None;

    loop {
        function_component.render(state)?;

        let key_event = read_key_press();
        match on_keypress(state, key_event) {
            EventLoopResult::ContinueAndRerender => {
                // Continue the loop.
                function_component.render(state)?;
            }
            EventLoopResult::Continue => {
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
