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

use r3bl_ansi_color::{AnsiStyledText, Style};
use r3bl_rs_utils_core::{throws, CommonResult};
use r3bl_tuify::LIGHT_GRAY_COLOR;

// 00: copy ex_editor/launcher.rs
pub async fn run_app(file_path: Option<String>) -> CommonResult<()> {
    throws!({
        AnsiStyledText {
            text: &format!(
                "🔅 TODO implement edi_launcher::open_editor({:?}) 🔅",
                file_path
            ),
            style: &[Style::Foreground(LIGHT_GRAY_COLOR)],
        }
        .println();
    })
}
