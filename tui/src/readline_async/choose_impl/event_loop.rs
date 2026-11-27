// Copyright (c) 2023-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::KeyPressReader;
use crate::{CalculateResizeHint, ChooseError, FunctionComponent, InputDevice,
            InputEvent, ItemsOwned, TTYResult, TerminalModeController,
            emit_stderr_redirection_disclaimer, is_output_interactive};

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
pub async fn try_enter_event_loop_async<S: CalculateResizeHint>(
    state: &mut S,
    function_component: &mut impl FunctionComponent<S>,
    mut on_keypress_fn: impl FnMut(&mut S, InputEvent) -> EventLoopResult,
    input_device: &mut InputDevice,
) -> Result<EventLoopResult, ChooseError> {
    use EventLoopResult::ExitWithError;

    if let TTYResult::IsNotInteractive = is_output_interactive() {
        return Ok(ExitWithError);
    }

    emit_stderr_redirection_disclaimer();

    try_run_before_event_loop(state, function_component)?;

    let return_this: EventLoopResult;

    loop {
        let maybe_ie = input_device.next().await;
        if let Some(ie) = maybe_ie {
            let event_loop_result = on_keypress_fn(state, ie);
            if let Some(result) =
                handle_event_loop_result(function_component, event_loop_result, state)
            {
                return_this = result;
                break;
            }
        } else {
            return_this = ExitWithError;
            function_component
                .clear_viewport(state)
                .map_err(ChooseError::Io)?;
            break;
        }
    }

    try_run_after_event_loop(function_component)?;

    Ok(return_this)
}

/// # Errors
///
/// Returns an error if the event loop encounters a terminal I/O error.
pub fn try_enter_event_loop_sync<S: CalculateResizeHint>(
    state: &mut S,
    function_component: &mut impl FunctionComponent<S>,
    mut on_keypress_fn: impl FnMut(&mut S, InputEvent) -> EventLoopResult,
    key_press_reader: &mut impl KeyPressReader,
) -> Result<EventLoopResult, ChooseError> {
    use EventLoopResult::ExitWithError;

    if let TTYResult::IsNotInteractive = is_output_interactive() {
        return Ok(ExitWithError);
    }

    emit_stderr_redirection_disclaimer();

    try_run_before_event_loop(state, function_component)?;

    let return_this: EventLoopResult;

    loop {
        let maybe_ie = key_press_reader.read_key_press();
        if let Some(ie) = maybe_ie {
            if let Some(result) = handle_event_loop_result(
                function_component,
                on_keypress_fn(state, ie),
                state,
            ) {
                return_this = result;
                break;
            }
        } else {
            return_this = ExitWithError;
            function_component
                .clear_viewport(state)
                .map_err(ChooseError::Io)?;
            break;
        }
    }

    try_run_after_event_loop(function_component)?;

    Ok(return_this)
}

fn try_run_before_event_loop<S: CalculateResizeHint>(
    state: &mut S,
    function_component: &mut impl FunctionComponent<S>,
) -> Result<(), ChooseError> {
    function_component
        .get_output_device()
        .hide_cursor()
        .map_err(ChooseError::TerminalIo)?;
    crate::try_enable_raw_mode().map_err(ChooseError::RawMode)?;

    // First render before blocking the main thread for user input.
    function_component.render(state).map_err(ChooseError::Io)?;

    ok!()
}

fn try_run_after_event_loop<S: CalculateResizeHint>(
    function_component: &mut impl FunctionComponent<S>,
) -> Result<(), ChooseError> {
    function_component
        .get_output_device()
        .show_cursor()
        .map_err(ChooseError::TerminalIo)?;
    crate::try_disable_raw_mode().map_err(ChooseError::RawMode)?;
    ok!()
}

fn handle_event_loop_result<S: CalculateResizeHint>(
    function_component: &mut impl FunctionComponent<S>,
    result: EventLoopResult,
    state: &mut S,
) -> Option<EventLoopResult> {
    use EventLoopResult::{Continue, ContinueAndRerender, ContinueAndRerenderAndClear,
                          ExitWithError, ExitWithResult, ExitWithoutResult, Select};

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
