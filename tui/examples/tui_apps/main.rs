// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

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
use std::str::FromStr;

use miette::IntoDiagnostic;
use r3bl_tui::{ASTColor, CommonError, CommonResult, DEBUG_TUI_MOD, InputEvent,
               TerminalWindow, fg_color, fg_frozen_blue, fg_pink, fg_slate_gray,
               get_size, inline_string, key_press,
               log::try_initialize_logging_global,
               ok,
               readline_async::{ReadlineAsyncContext, ReadlineEvent},
               rla_println, run_with_safe_stack, set_mimalloc_in_main, tui_color};
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, Display, EnumIter, EnumString};

fn main() -> CommonResult<()> { run_with_safe_stack!(main_impl()) }

#[tokio::main]
#[allow(clippy::needless_return)]
async fn main_impl() -> CommonResult<()> {
    set_mimalloc_in_main!();

    let args: Vec<String> = std::env::args().collect();
    let no_log_arg_passed = args.contains(&"--no-log".to_string());

    // If the terminal is not fully interactive, then return early.
    let Some(mut rl_ctx) = ReadlineAsyncContext::try_new({
        // Generate prompt.
        let prompt_seg_1 = fg_slate_gray("╭>╮").bg_moonlight_blue();
        let prompt_seg_2 = " ";
        Some(format!("{prompt_seg_1}{prompt_seg_2}"))
    })
    .await?
    else {
        return CommonError::new_error_result_with_only_msg(
            "Terminal is not fully interactive",
        );
    };

    // Pre-populate the read_line's history with some entries.
    for command in AutoCompleteCommand::iter() {
        rl_ctx.readline.add_history_entry(command.to_string());
    }

    let msg = inline_string!("{}", &generate_help_msg());

    let msg_fmt = fg_color(ASTColor::from(tui_color!(lizard_green)), &msg);
    rla_println!(rl_ctx, "{}", msg_fmt.to_string());

    // Ignore errors: https://doc.rust-lang.org/std/result/enum.Result.html#method.ok
    if no_log_arg_passed {
        try_initialize_logging_global(tracing_core::LevelFilter::OFF).ok();
    } else if ENABLE_TRACE_EXAMPLES | DEBUG_TUI_MOD {
        try_initialize_logging_global(tracing_core::LevelFilter::DEBUG).ok();
    }

    loop {
        let result_readline_event = rl_ctx.read_line().await;
        match result_readline_event {
            Ok(readline_event) => match readline_event {
                ReadlineEvent::Line(input) => {
                    if run_user_selected_example(input, &mut rl_ctx).await.is_err() {
                        break;
                    }
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
/// - `result_command` is the parsed command from the selection, eg:
///   [`AutoCompleteCommand::NoLayout`].
///
/// # Raw mode caveat
///
/// This function will take the terminal out of raw mode when it returns. This is because
/// the examples below will use `r3bl_tui` which will put the terminal in raw mode, use
/// alt screen, and then restore it all when it exits.
async fn run_user_selected_example(
    selection: String,
    readline_async: &mut ReadlineAsyncContext,
) -> CommonResult<()> {
    use AutoCompleteCommand::{Commander, Editor, Exit, NoLayout, OneColLayout, Slides,
                              TwoColLayout};

    let result_command /* Eg: Ok(Exit) */ =
        AutoCompleteCommand::from_str(&selection /* eg: "0" */);

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
    use AutoCompleteCommand::{Commander, Editor, NoLayout, OneColLayout, Slides,
                              TwoColLayout};

    let window_size = get_size().unwrap_or_default();

    let it = format!(
        "\
Welcome to the R3BL TUI demo app.
Window size: {window_size:?}
Type a number to run corresponding example:
  0. 📏 {NoLayout}
  1. 📐 {OneColLayout}
  2. 📐 {TwoColLayout}
  3. 🐒 {Editor}
  4. 🦜 {Slides}
  5. 📔 {Commander}

or type Ctrl+C, Ctrl+D, 'request_shutdown', or 'x' to request_shutdown",
    );

    it
}
