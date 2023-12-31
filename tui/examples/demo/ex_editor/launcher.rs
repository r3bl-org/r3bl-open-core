/*
 *   Copyright (c) 2022 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use r3bl_rs_utils_core::*;
use r3bl_tui::*;

use super::*;

pub async fn run_app() -> CommonResult<()> {
    throws!({
        // Ignore errors: https://doc.rust-lang.org/std/result/enum.Result.html#method.ok
        if DEBUG_TUI_MOD {
            try_to_set_log_level(log::LevelFilter::Debug).ok();
        } else {
            try_to_set_log_level(log::LevelFilter::Off).ok();
        }

        // Create an App (renders & responds to user input).
        let app = AppMain::new_boxed();

        // Exit if these keys are pressed.
        let exit_keys: Vec<InputEvent> = vec![InputEvent::Keyboard(
            keypress! { @char ModifierKeysMask::new().with_ctrl(), 'q' },
        )];

        // Create a window.
        TerminalWindow::main_event_loop(app, exit_keys, State::default()).await?
    });
}
