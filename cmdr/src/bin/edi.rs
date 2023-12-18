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

use r3bl_ansi_color::{AnsiStyledText, Color, Style};

// TODO: https://github.com/r3bl-org/r3bl-open-core/issues/188
pub fn main() {
    AnsiStyledText {
        text: "Hello, edi! 👋 🦜",
        style: &[Style::Bold, Style::Foreground(Color::Rgb(100, 200, 1))],
    }
    .println();
}
