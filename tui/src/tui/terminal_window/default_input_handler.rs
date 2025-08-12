// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::Continuation;
use crate::InputEvent;

#[derive(Debug)]
pub struct DefaultInputEventHandler;

impl DefaultInputEventHandler {
    /// This function does **not** consume the `input_event` argument. [`InputEvent`]
    /// implements [Copy] (no need to pass references into this function).
    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn no_consume(
        input_event: InputEvent,
        exit_keys: &[InputEvent],
    ) -> Continuation<String> {
        // Early return if any request_shutdown key sequence is pressed.
        if input_event.matches(exit_keys) {
            return Continuation::Exit;
        }
        Continuation::Continue
    }
}
