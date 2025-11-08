// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use r3bl_tui::{CommonResult, GlobalData, InputDevice, InputEvent, OutputDevice,
               TerminalWindow, key_press, ok};

use super::app_main::TodoListApp;
use super::state::AppState;

pub async fn launch_app() -> CommonResult<()> {
    let app = TodoListApp::new_boxed();

    // Exit if these keys are pressed.
    let exit_keys = &[InputEvent::Keyboard(key_press! { @char 'x' })];

    // Create a window.
    let _unused: (GlobalData<_, _>, InputDevice, OutputDevice) =
        TerminalWindow::main_event_loop(app, exit_keys, AppState::default())?.await?;

    ok!()
}
