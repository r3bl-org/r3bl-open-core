// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::edi::{AppMain, constructor};
use r3bl_tui::{InputEvent, MainEventLoopError, ModifierKeysMask, TerminalWindow,
               key_press};

/// Runs the editor application with an optional file to open.
///
/// # Errors
///
/// Returns [`MainEventLoopError`] if the terminal is non-interactive or the terminal
/// window fails to initialize or enter full-screen raw mode.
pub async fn try_run_app(
    maybe_file_path: Option<&str>,
) -> Result<(), MainEventLoopError> {
    let state = constructor::new(maybe_file_path);
    let app = AppMain::new_boxed();
    let exit_keys = &[InputEvent::Keyboard(
        key_press! { @char ModifierKeysMask::new().with_ctrl(), 'q' },
    )];

    TerminalWindow::try_main_event_loop(app, exit_keys, state).await?;

    Ok(())
}
