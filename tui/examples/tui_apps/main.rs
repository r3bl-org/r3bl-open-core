/*
 *   Copyright (c) 2022-2025 R3BL LLC
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
#![warn(clippy::all)]
#![warn(clippy::unwrap_in_result)]
#![warn(rust_2018_idioms)]

/// Enable debug logging.
pub const ENABLE_TRACE_EXAMPLES: bool = true;

// Attach sources.
mod ex_app_no_layout;
mod ex_app_with_1col_layout;
mod ex_app_with_2col_layout;
mod ex_editor;
mod ex_pitch;
mod ex_rc;

// Use other crates.
use std::str::FromStr as _;

use miette::IntoDiagnostic as _;
use r3bl_tui::{fg_color,
               fg_frozen_blue,
               fg_pink,
               fg_slate_gray,
               get_size,
               inline_string,
               key_press,
               log::try_initialize_logging_global,
               ok,
               readline_async::{ReadlineAsync, ReadlineEvent},
               rla_println,
               throws,
               tui_color,
               ASTColor,
               CommonError,
               CommonResult,
               InputEvent,
               TerminalWindow,
               DEBUG_TUI_MOD};
use strum::IntoEnumIterator as _;
use strum_macros::{AsRefStr, Display, EnumIter, EnumString};

#[tokio::main]
#[allow(clippy::needless_return)]
async fn main() -> CommonResult<()> {
    let args: Vec<String> = std::env::args().collect();
    let no_log_arg_passed = args.contains(&"--no-log".to_string());

    // If the terminal is not fully interactive, then return early.
    let Some(mut readline_async) = ReadlineAsync::try_new({
        // Generate prompt.
        let prompt_seg_1 = fg_slate_gray("╭>╮").bg_moonlight_blue();
        let prompt_seg_2 = " ";
        Some(format!("{}{}", prompt_seg_1, prompt_seg_2))
    })?
    else {
        return CommonError::new_error_result_with_only_msg(
            "Terminal is not fully interactive",
        );
    };

    // Pre-populate the read_line's history with some entries.
    for command in AutoCompleteCommand::iter() {
        readline_async
            .readline
            .add_history_entry(command.to_string());
    }

    let msg = inline_string!("{}", &generate_help_msg());

    let msg_fmt = fg_color(ASTColor::from(tui_color!(lizard_green)), &msg);
    rla_println!(readline_async, "{}", msg_fmt.to_string());

    // Ignore errors: https://doc.rust-lang.org/std/result/enum.Result.html#method.ok
    if no_log_arg_passed {
        try_initialize_logging_global(tracing_core::LevelFilter::OFF).ok();
    } else if ENABLE_TRACE_EXAMPLES | DEBUG_TUI_MOD {
        try_initialize_logging_global(tracing_core::LevelFilter::DEBUG).ok();
    }

    loop {
        let result_readline_event = readline_async.read_line().await;
        match result_readline_event {
            Ok(readline_event) => match readline_event {
                ReadlineEvent::Line(input) => {
                    if run_user_selected_example(input, &mut readline_async)
                        .await
                        .is_err()
                    {
                        break;
                    };
                    crossterm::terminal::enable_raw_mode().into_diagnostic()?;
                }
                ReadlineEvent::Eof | ReadlineEvent::Interrupted => break,
                ReadlineEvent::Resized => { /* continue */ }
            },
            Err(_) => {
                break;
            }
        }
    }

    ok!()
}

/// You can type both "0" or "App with no layout" to run the first example. Here are some
/// details:
/// - `selection` is what the user types in the terminal, eg: "0" or "App with no layout".
/// - result_command is the parsed command from the selection, eg:
///   [AutoCompleteCommand::NoLayout].
///
/// # Raw mode caveat
///
/// This function will take the terminal out of raw mode when it returns. This is because
/// the examples below will use `r3bl_tui` which will put the terminal in raw mode, use
/// alt screen, and then restore it all when it exits.
async fn run_user_selected_example(
    selection: String,
    readline_async: &mut ReadlineAsync,
) -> CommonResult<()> {
    let result_command /* Eg: Ok(Exit) */ =
        AutoCompleteCommand::from_str(&selection /* eg: "0" */);

    use AutoCompleteCommand::*;

    match result_command {
        Ok(command) => match command {
            NoLayout => ex_app_no_layout::launcher::run_app().await,
            OneColLayout => ex_app_with_1col_layout::launcher::run_app().await,
            TwoColLayout => ex_app_with_2col_layout::launcher::run_app().await,
            Editor => ex_editor::launcher::run_app().await,
            Slides => ex_pitch::launcher::run_app().await,
            Commander => ex_rc::launcher::run_app().await,
            Exit => CommonError::new_error_result_with_only_msg("Exiting..."),
        },
        Err(_) => {
            rla_println!(
                readline_async,
                "{a} {b}",
                a = fg_frozen_blue("Invalid selection:"),
                b = fg_pink(&selection).bold(),
            );
            Ok(())
        }
    }
}

#[derive(Debug, PartialEq, EnumString, EnumIter, Display, AsRefStr)]
enum AutoCompleteCommand {
    #[strum(ascii_case_insensitive)]
    #[strum(to_string = "App with no layout")]
    #[strum(serialize = "0")]
    NoLayout,

    #[strum(ascii_case_insensitive)]
    #[strum(to_string = "App with 1 column responsive layout")]
    #[strum(serialize = "1")]
    OneColLayout,

    #[strum(ascii_case_insensitive)]
    #[strum(to_string = "App with 2 column responsive layout")]
    #[strum(serialize = "2")]
    TwoColLayout,

    #[strum(ascii_case_insensitive)]
    #[strum(to_string = "Markdown editor, syntax highlighting, modal dialog, and emoji")]
    #[strum(serialize = "3")]
    Editor,

    #[strum(ascii_case_insensitive)]
    #[strum(to_string = "Why R3BL? Why TUI?")]
    #[strum(serialize = "4")]
    Slides,

    #[strum(ascii_case_insensitive)]
    #[strum(to_string = "R3BL CMDR prototype")]
    #[strum(serialize = "5")]
    Commander,

    #[strum(ascii_case_insensitive)]
    #[strum(to_string = "Exit")]
    #[strum(serialize = "x")]
    Exit,
}

fn generate_help_msg() -> String {
    use AutoCompleteCommand::*;

    let window_size = get_size().unwrap_or_default();

    let it = format!(
        "\
Welcome to the R3BL TUI demo app.
Window size: {window_size:?}
Type a number to run corresponding example:
  0. 📏 {}
  1. 📐 {}
  2. 📐 {}
  3. 🐒 {}
  4. 🦜 {}
  5. 📔 {}

or type Ctrl+C, Ctrl+D, 'exit', or 'x' to exit",
        NoLayout, OneColLayout, TwoColLayout, Editor, Slides, Commander,
    );

    it
}
