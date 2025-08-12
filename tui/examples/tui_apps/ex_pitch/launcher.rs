// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use r3bl_tui::{CommonResult, GlobalData, InputDevice, InputEvent, ModifierKeysMask,
               OutputDevice, TerminalWindow, key_press, ok};

use super::{AppMain, state::State};

pub async fn run_app() -> CommonResult<()> {
    // Create an App (renders & responds to user input).
    let app = AppMain::new_boxed();

    // Exit if these keys are pressed.
    let exit_keys = &[InputEvent::Keyboard(
        key_press! { @char ModifierKeysMask::new().with_ctrl(), 'q' },
    )];

    // Create a window.
    let _unused: (GlobalData<_, _>, InputDevice, OutputDevice) =
        TerminalWindow::main_event_loop(app, exit_keys, State::default())?.await?;

    ok!()
}
