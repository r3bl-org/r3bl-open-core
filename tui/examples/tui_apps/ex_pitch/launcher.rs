// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{AppMain, state::State};
use r3bl_tui::{CommonResult, InputEvent, IntoErr, TerminalWindow, TuiAvailability,
               key_press, ok};

pub async fn run_app() -> CommonResult {
    let app = AppMain::new_boxed();
    let exit_keys = &[InputEvent::Keyboard(key_press! { @char 'x' })];

    match TerminalWindow::main_event_loop(app, exit_keys, State::default()) {
        TuiAvailability::Available(future) => {
            future.await?;
        }
        it => return it.into_err(),
    }

    ok!()
}
