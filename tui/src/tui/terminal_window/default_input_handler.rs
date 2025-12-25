// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{Continuation, InputEvent};

#[derive(Debug)]
pub struct DefaultInputEventHandler;

impl DefaultInputEventHandler {
    /// This function does **not** consume the `input_event` argument. [`InputEvent`]
    /// implements [Copy] (no need to pass references into this function).
    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn no_consume(input_event: InputEvent, exit_keys: &[InputEvent]) -> Continuation {
        // Early return if any request_shutdown key sequence is pressed.
        if input_event.matches(exit_keys) {
            return Continuation::Stop;
        }
        Continuation::Continue
    }
}
