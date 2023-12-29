/*
 *   Copyright (c) 2023 R3BL LLC
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

use r3bl_rs_utils_core::{throws, CommonResult};
use r3bl_tui::*;

use crate::edi::{constructor, AppMain};

pub async fn run_app(maybe_file_path: Option<String>) -> CommonResult<()> {
    throws!({
        // Create a new state from the file path.
        let state = constructor::new(&maybe_file_path);

        // Create a new app.
        let app = AppMain::new_boxed();

        // Exit if these keys are pressed.
        let exit_keys: Vec<InputEvent> = vec![InputEvent::Keyboard(
            keypress! { @char ModifierKeysMask::new().with_ctrl(), 'q' },
        )];

        // Create a window.
        TerminalWindow::main_event_loop(app, exit_keys, state).await?;
    })
}
