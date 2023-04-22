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

// https://github.com/rust-lang/rust-clippy
// https://rust-lang.github.io/rust-clippy/master/index.html
// #![warn(clippy::all)]
// #![warn(rust_2018_idioms)]

pub const DEBUG: bool = true;

use r3bl_rs_utils_core::*;
use r3bl_tui::*;

// Attach sources.
mod ex_app_no_layout;
mod ex_app_with_1col_layout;
mod ex_app_with_2col_layout;
mod ex_editor;
mod ex_pitch;
mod ex_rc;

// Use things from sources.
use reedline::*;

const HELP_MSG: &str = "\
Welcome to the R3BL TUI demo app.
Type a number to run corresponding example:
  0. 📏 App w/ no layout
  1. 📐 App w/ 1 column responsive layout
  2. 📐 App w/ 2 column responsive layout
  3. 🐒 Markdown editor, syntax highlighting, modal dialog, and emoji
  4. ⚾ Why R3BL? Why TUI?
  5. 📔 R3BL CMDR prototype

or type Ctrl+C / Ctrl+D / 'x' to exit";

#[tokio::main]
async fn main() -> CommonResult<()> {
    throws!({
        loop {
            let continuation = get_user_selection_from_terminal();
            match continuation {
                Continuation::Exit => break,
                Continuation::Result(user_selection) => {
                    run_user_selected_example(user_selection).await?;
                }
                _ => {}
            }
        }
    })
}

async fn run_user_selected_example(selection: String) -> CommonResult<()> {
    throws!({
        if !selection.is_empty() {
            match selection.as_ref() {
                "0" => throws!(ex_app_no_layout::launcher::run_app().await?),
                "1" => throws!(ex_app_with_1col_layout::launcher::run_app().await?),
                "2" => throws!(ex_app_with_2col_layout::launcher::run_app().await?),
                "3" => throws!(ex_editor::launcher::run_app().await?),
                "4" => throws!(ex_pitch::launcher::run_app().await?),
                "5" => throws!(ex_rc::launcher::run_app().await?),
                _ => eprintln!("{}", style_error("Unknown selection 🤷")),
            }
        }
    })
}

/// This is a single threaded blocking function. The R3BL examples are all async and non-blocking.
fn get_user_selection_from_terminal() -> Continuation<String> {
    println!("{}", style_prompt(HELP_MSG));

    let mut line_editor = Reedline::create();
    let prompt = DefaultPrompt::default();

    loop {
        let maybe_signal = &line_editor.read_line(&prompt);
        if let Ok(Signal::Success(user_input_text)) = maybe_signal {
            match user_input_text.as_str() {
                "x" => break,
                _ => return Continuation::Result(user_input_text.into()),
            }
        } else if let Ok(Signal::CtrlC) | Ok(Signal::CtrlD) = maybe_signal {
            break;
        }
    }

    Continuation::Exit
}
