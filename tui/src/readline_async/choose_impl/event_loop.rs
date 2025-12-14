// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crossterm::cursor::{Hide, Show};

use super::KeyPressReader;
use crate::{execute_commands,
            is_output_interactive,
            CalculateResizeHint,
            CommonResult,
            FunctionComponent,
            InputDevice,
            InputEvent,
            ItemsOwned,
            TTYResult};

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum EventLoopResult {
    Continue,
    ContinueAndRerender,
    ContinueAndRerenderAndClear,
    ExitWithResult(ItemsOwned),
    ExitWithoutResult,
    ExitWithError,
    Select,
}

/// # Errors
///
/// Returns an error if the event loop encounters a terminal I/O error.
pub async fn enter_event_loop_async<S: CalculateResizeHint>(
    state: &mut S,
    function_component: &mut impl FunctionComponent<S>,
    on_keypress: impl Fn(&mut S, InputEvent) -> EventLoopResult,
    input_device: &mut InputDevice,
) -> CommonResult<EventLoopResult> {
    use EventLoopResult::ExitWithError;

    if let TTYResult::IsNotInteractive = is_output_interactive() {
        return Ok(ExitWithError);
    }

    run_before_event_loop(state, function_component)?;

    let return_this: EventLoopResult;

    loop {
        let maybe_ie = input_device.next().await;
        if let Some(ie) = maybe_ie {
            let event_loop_result = on_keypress(state, ie);
            if let Some(result) =
                handle_event_loop_result(function_component, event_loop_result, state)
            {
                return_this = result;
                break;
            }
        } else {
            return_this = ExitWithError;
            function_component.clear_viewport(state)?;
            break;
        }
    }

    run_after_event_loop(function_component)?;

    Ok(return_this)
}

/// # Errors
///
/// Returns an error if the event loop encounters a terminal I/O error.
pub fn enter_event_loop_sync<S: CalculateResizeHint>(
    state: &mut S,
    function_component: &mut impl FunctionComponent<S>,
    on_keypress: impl Fn(&mut S, InputEvent) -> EventLoopResult,
    key_press_reader: &mut impl KeyPressReader,
) -> CommonResult<EventLoopResult> {
    use EventLoopResult::ExitWithError;

    if let TTYResult::IsNotInteractive = is_output_interactive() {
        return Ok(ExitWithError);
    }

    run_before_event_loop(state, function_component)?;

    let return_this: EventLoopResult;

    loop {
        let maybe_ie = key_press_reader.read_key_press();
        if let Some(ie) = maybe_ie {
            if let Some(result) = handle_event_loop_result(
                function_component,
                on_keypress(state, ie),
                state,
            ) {
                return_this = result;
                break;
            }
        } else {
            return_this = ExitWithError;
            function_component.clear_viewport(state)?;
            break;
        }
    }

    run_after_event_loop(function_component)?;

    Ok(return_this)
}

fn run_before_event_loop<S: CalculateResizeHint>(
    state: &mut S,
    function_component: &mut impl FunctionComponent<S>,
) -> CommonResult<()> {
    execute_commands!(function_component.get_output_device(), Hide);
    crate::enable_raw_mode()?;

    // First render before blocking the main thread for user input.
    function_component.render(state)?;

    Ok(())
}

fn run_after_event_loop<S: CalculateResizeHint>(
    function_component: &mut impl FunctionComponent<S>,
) -> CommonResult<()> {
    execute_commands!(function_component.get_output_device(), Show);
    crate::disable_raw_mode()?;
    Ok(())
}

fn handle_event_loop_result<S: CalculateResizeHint>(
    function_component: &mut impl FunctionComponent<S>,
    result: EventLoopResult,
    state: &mut S,
) -> Option<EventLoopResult> {
    use EventLoopResult::{ContinueAndRerenderAndClear, ContinueAndRerender, Continue, Select, ExitWithResult, ExitWithoutResult, ExitWithError};

    match result {
        ContinueAndRerenderAndClear => {
            function_component.clear_viewport_for_resize(state).ok()?;
            function_component.render(state).ok()?;
            None
        }
        ContinueAndRerender => {
            function_component.render(state).ok()?;
            None
        }
        Continue | Select => None,
        ExitWithResult(it) => {
            function_component.clear_viewport(state).ok()?;
            Some(ExitWithResult(it))
        }
        ExitWithoutResult => {
            function_component.clear_viewport(state).ok()?;
            Some(ExitWithoutResult)
        }
        ExitWithError => {
            function_component.clear_viewport(state).ok()?;
            Some(ExitWithError)
        }
    }
}
