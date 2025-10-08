// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::edi::{AppMain, constructor};
use r3bl_tui::{CommonResult, GlobalData, InputDevice, InputEvent, ModifierKeysMask,
               OutputDevice, TerminalWindow, key_press, ok};

/// Runs the editor application with an optional file to open.
///
/// # Errors
///
/// Returns an error if the terminal window fails to initialize or run.
pub async fn run_app(maybe_file_path: Option<&str>) -> CommonResult<()> {
    // Create a new state from the file path.
    let state = constructor::new(maybe_file_path);

    // Create a new app.
    let app = AppMain::new_boxed();

    // Exit if these keys are pressed.
    let exit_keys = &[InputEvent::Keyboard(
        key_press! { @char ModifierKeysMask::new().with_ctrl(), 'q' },
    )];

    // Create a window.
    let _unused: (GlobalData<_, _>, InputDevice, OutputDevice) =
        TerminalWindow::main_event_loop(app, exit_keys, state)?.await?;

    ok!()
}
