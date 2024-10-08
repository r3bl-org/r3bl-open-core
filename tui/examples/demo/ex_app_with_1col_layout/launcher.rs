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

use r3bl_core::{throws, CommonResult};
use r3bl_tui::{keypress, InputEvent, TerminalWindow};

use super::{AppMain, State};

pub async fn run_app() -> CommonResult<()> {
    throws!({
        // Create an App (renders & responds to user input).
        let app = AppMain::new_boxed();

        // Exit if these keys are pressed.
        let exit_keys: Vec<InputEvent> =
            vec![InputEvent::Keyboard(keypress! { @char 'x' })];

        // Create a window.
        _ = TerminalWindow::main_event_loop(app, exit_keys, State::default()).await?
    });
}
