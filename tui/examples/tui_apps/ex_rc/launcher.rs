// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{AppMain, State};
use r3bl_tui::{CommonResult, InputEvent, ModifierKeysMask, TerminalWindow,
               TuiAvailability, key_press, ok};

pub async fn run_app() -> CommonResult<()> {
    let app = AppMain::new_boxed();
    let exit_keys = &[InputEvent::Keyboard(
        key_press! { @char ModifierKeysMask::new().with_ctrl(), 'q' },
    )];

    match TerminalWindow::main_event_loop(app, exit_keys, State::default()) {
        TuiAvailability::Available(future) => {
            future.await?;
        }
        TuiAvailability::NotAvailable(reason) => {
            eprintln!("{}", reason.as_err_msg());
        }
        TuiAvailability::Broken(e) => return Err(e),
    }

    ok!()
}
