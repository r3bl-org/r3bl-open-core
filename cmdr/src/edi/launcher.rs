// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::edi::{AppMain, constructor};
use r3bl_tui::{CommonResult, InputEvent, IntoErr, ModifierKeysMask, TerminalWindow,
               TuiAvailability, key_press, ok};

/// Runs the editor application with an optional file to open.
///
/// # Errors
///
/// Returns an error if the terminal window fails to initialize or run.
pub async fn run_app(maybe_file_path: Option<&str>) -> CommonResult {
    let state = constructor::new(maybe_file_path);
    let app = AppMain::new_boxed();
    let exit_keys = &[InputEvent::Keyboard(
        key_press! { @char ModifierKeysMask::new().with_ctrl(), 'q' },
    )];

    match TerminalWindow::main_event_loop(app, exit_keys, state) {
        TuiAvailability::Available(future) => {
            future.await?;
        }
        it => return it.into_err(),
    }

    ok!()
}
