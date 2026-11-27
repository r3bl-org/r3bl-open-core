// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{AppMain, State};
use r3bl_tui::{InputEvent, TerminalWindow, key_press, ok};

pub async fn run_app() -> miette::Result<()> {
    let app = AppMain::new_boxed();
    let exit_keys = &[InputEvent::Keyboard(key_press! { @char 'x' })];

    TerminalWindow::try_main_event_loop(app, exit_keys, State::default()).await?;

    ok!()
}
