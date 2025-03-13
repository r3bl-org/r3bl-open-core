/*
 *   Copyright (c) 2023-2025 R3BL LLC
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

//! For more information on how to use CLAP and Tuify, please read this tutorial:
//! <https://developerlife.com/2023/09/17/tuify-clap/>

use std::{io::{stdin, BufRead, Result},
          process::Command};

use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use miette::IntoDiagnostic;
use r3bl_ansi_color::{blue, lizard_green, pink};
use r3bl_core::{get_size,
                get_terminal_width,
                inline_string,
                is_stdin_piped,
                is_stdout_piped,
                throws,
                usize,
                StdinIsPipedResult,
                StdoutIsPipedResult};
use r3bl_log::try_initialize_logging_global;
use r3bl_tuify::{select_from_list, SelectionMode, StyleSheet, DEVELOPMENT_MODE};
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};
use StdinIsPipedResult::{StdinIsNotPiped, StdinIsPiped};
use StdoutIsPipedResult::{StdoutIsNotPiped, StdoutIsPiped};

const SELECTED_ITEM_SYMBOL: char = '%';

#[derive(Debug, Parser)]
#[command(bin_name = "rt")]
#[command(about = "Easily add lightweight TUI capabilities to any CLI apps using pipes", long_about = None)]
#[command(version)]
#[command(next_line_help = true)]
#[command(arg_required_else_help(true))]
pub struct AppArgs {
    #[clap(subcommand)]
    command: CLICommand,

    #[clap(flatten)]
    global_opts: GlobalOpts,
}

#[derive(Debug, Args)]
struct GlobalOpts {
    /// Enables logging to a file named `log.txt`.
    #[arg(long, short = 'l')]
    enable_logging: bool,

    /// Sets the maximum height of the Tuify component (rows).
    /// If height is not provided, it defaults to the terminal height.
    #[arg(value_name = "height", long, short = 'r')]
    tui_height: Option<usize>,

    /// Sets the maximum width of the Tuify component (columns).
    /// If width is not provided, it defaults to the terminal width.
    #[arg(value_name = "width", long, short = 'c')]
    tui_width: Option<usize>,
}

#[derive(Debug, Subcommand)]
enum CLICommand {
    /// Show TUI to allow you to select one or more options from a list, piped in via stdin 👉
    SelectFromList {
        /// Would you like to select one or more items?
        #[arg(value_name = "mode", long, short = 's')]
        selection_mode: Option<SelectionMode>,

        /// Each selected item is passed to this command as `%` and executed in your shell.
        /// For eg: "echo %". Please wrap the command in quotes 💡
        #[arg(value_name = "command", long, short = 'c')]
        command_to_run_with_each_selection: Option<String>,
    },
}

fn get_bin_name() -> String {
    let cmd = AppArgs::command();
    cmd.get_bin_name().unwrap_or("this command").to_string()
}

fn main() -> miette::Result<()> {
    throws!({
        // If no args are passed, the following line will fail, and help will be printed
        // thanks to `arg_required_else_help(true)` in the `CliArgs` struct.
        let cli_args = AppArgs::parse();

        let enable_logging = DEVELOPMENT_MODE | cli_args.global_opts.enable_logging;

        enable_logging.then(|| {
            try_initialize_logging_global(tracing_core::LevelFilter::DEBUG).ok();
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "Start logging",
                window_size = ?get_size(),
                cli_args = ?cli_args,
            );
        });

        match cli_args.command {
            CLICommand::SelectFromList {
                selection_mode,
                command_to_run_with_each_selection: command_to_run_with_selection,
            } => {
                // macos has issues w/ stdin piped in.
                // https://github.com/crossterm-rs/crossterm/issues/396
                if cfg!(target_os = "macos") {
                    match (is_stdin_piped(), is_stdout_piped()) {
                        (StdinIsPiped, _) => {
                            show_error_stdin_pipe_does_not_work_on_macos();
                        }
                        (_, StdoutIsPiped) => {
                            show_error_do_not_pipe_stdout(get_bin_name().as_ref());
                        }
                        (StdinIsNotPiped, StdoutIsNotPiped) => {
                            print_help()?;
                        }
                    }
                }
                // Linux works fine.
                else {
                    match (is_stdin_piped(), is_stdout_piped()) {
                        (StdinIsPiped, StdoutIsNotPiped) => {
                            let tui_height = cli_args.global_opts.tui_height;
                            let tui_width = cli_args.global_opts.tui_width;
                            show_tui(
                                selection_mode,
                                command_to_run_with_selection,
                                tui_height,
                                tui_width,
                                enable_logging,
                            );
                        }
                        (StdinIsPiped, StdoutIsPiped) => {
                            show_error_do_not_pipe_stdout(get_bin_name().as_ref());
                        }
                        (StdinIsNotPiped, StdoutIsPiped) => {
                            show_error_need_to_pipe_stdin(get_bin_name().as_ref());
                            show_error_do_not_pipe_stdout(get_bin_name().as_ref());
                        }
                        (StdinIsNotPiped, StdoutIsNotPiped) => {
                            show_error_need_to_pipe_stdin(get_bin_name().as_ref());
                        }
                    }
                }
            }
        }
        enable_logging.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(message = "Stop logging...");
        });
    });
}

fn show_error_stdin_pipe_does_not_work_on_macos() {
    let msg = "Unfortunately at this time macOS `stdin` pipe does not work on macOS.
https://github.com/crossterm-rs/crossterm/issues/396";
    println!("{}", blue(msg).bg_dark_grey());
}

fn show_error_need_to_pipe_stdin(bin_name: &str) {
    let msg = format!(
        "Please pipe the output of another command into {bin_name}.
✅ For example: `ls -l | {bin_name} -s single-select`"
    );
    println!("{}", lizard_green(&msg).bg_dark_grey());
}

fn show_error_do_not_pipe_stdout(bin_name: &str) {
    let msg = format!(
        "Please do *not* pipe the output of {bin_name} to another command.
❎ For eg, don't do this: `ls -l | {bin_name} -s single-select | cat`"
    );
    println!("{}", pink(&msg).bg_dark_grey());
}

fn show_tui(
    maybe_selection_mode: Option<SelectionMode>,
    maybe_command_to_run_with_each_selection: Option<String>,
    tui_height: Option<usize>,
    tui_width: Option<usize>,
    enable_logging: bool,
) {
    let lines: Vec<String> = stdin()
        .lock()
        .lines()
        .map_while(Result::ok)
        .collect::<Vec<String>>();

    enable_logging.then(|| {
        // % is Display, ? is Debug.
        tracing::debug!(
            message = "lines",
            lines = ?lines,
        );
    });

    // Early return, nothing to do. No content found in stdin.
    if lines.is_empty() {
        return;
    }

    // Get display size.
    let max_width_col_count = tui_width.unwrap_or_else(|| usize(*get_terminal_width()));
    let max_height_row_count: usize = tui_height.unwrap_or(5);

    // Handle `selection-mode` is not passed in.
    let selection_mode = if let Some(selection_mode) = maybe_selection_mode {
        selection_mode
    } else {
        let possible_values_for_selection_mode =
            get_possible_values_for_subcommand_and_option(
                "select-from-list",
                "selection-mode",
            );
        print_help_for_subcommand_and_option("select-from-list", "selection-mode").ok();

        let user_selection = select_from_list(
            "Choose selection-mode".to_string(),
            possible_values_for_selection_mode,
            max_height_row_count,
            max_width_col_count,
            SelectionMode::Single,
            StyleSheet::default(),
        );

        let it = if let Some(user_selection) = user_selection {
            if let Some(it) = user_selection.first() {
                println!("selection-mode: {}", it);
                SelectionMode::from_str(it, true).unwrap_or(SelectionMode::Single)
            } else {
                print_help_for("select-from-list").ok();
                return;
            }
        } else {
            print_help_for("select-from-list").ok();
            return;
        };

        it
    };

    // Handle `command-to-run-with-each-selection` is not passed in.
    let command_to_run_with_each_selection =
        match maybe_command_to_run_with_each_selection {
            Some(it) => it,
            None => {
                print_help_for_subcommand_and_option(
                    "select-from-list",
                    "command-to-run-with-each-selection",
                )
                .ok();
                let mut line_editor = Reedline::create();
                let prompt = DefaultPrompt {
                    left_prompt: DefaultPromptSegment::Basic(
                        "Enter command to run w/ each selection `%`: ".to_string(),
                    ),
                    right_prompt: DefaultPromptSegment::Empty,
                };

                let sig = line_editor.read_line(&prompt);
                match sig {
                    Ok(Signal::Success(buffer)) => {
                        if buffer.is_empty() {
                            print_help_for("select-from-list").ok();
                            return;
                        }
                        println!("Command to run w/ each selection: {}", buffer);
                        buffer
                    }
                    _ => {
                        print_help_for("select-from-list").ok();
                        return;
                    }
                }
            }
        };

    // Actually get input from the user.
    let selected_items = {
        let it = select_from_list(
            "Select one line".to_string(),
            lines,
            max_height_row_count,
            max_width_col_count,
            selection_mode,
            StyleSheet::default(),
        );
        convert_user_input_into_vec_of_strings(it)
    };

    enable_logging.then(|| {
        // % is Display, ? is Debug.
        tracing::debug!(
            message = "selected_items",
            selected_items = ?selected_items,
        );
    });

    for selected_item in selected_items {
        let actual_command_to_run = &command_to_run_with_each_selection
            .replace(SELECTED_ITEM_SYMBOL, &selected_item);
        execute_command(actual_command_to_run);
    }
}

fn convert_user_input_into_vec_of_strings(
    user_input: Option<Vec<String>>,
) -> Vec<String> {
    user_input.unwrap_or_default()
}

/// More info: <https://docs.rs/execute/latest/execute/#run-a-command-string-in-the-current-shell>
fn execute_command(cmd_str: &str) {
    // This let binding is required to make the code below work.
    let mut command_binding = if cfg!(target_os = "windows") {
        Command::new("cmd")
    } else {
        Command::new("sh")
    };

    let command = if cfg!(target_os = "windows") {
        command_binding.arg("/C").arg(cmd_str)
    } else {
        command_binding.arg("-c").arg(cmd_str)
    };

    let output = command.output().expect("failed to execute process");

    let result_output_str = String::from_utf8(output.stdout);

    match result_output_str {
        Ok(it) => {
            print!("{}", it);
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}

/// Programmatically prints out help.
pub fn print_help() -> miette::Result<()> {
    throws!({
        let mut cmd = AppArgs::command();
        cmd.print_help().into_diagnostic()?;
    });
}

fn print_help_for(subcommand: &str) -> Result<()> {
    throws!({
        let app_args_binding = AppArgs::command();
        if let Some(it) = app_args_binding.find_subcommand(subcommand) {
            it.clone().print_help()?;
        }
    });
}

fn print_help_for_subcommand_and_option(subcommand: &str, option: &str) -> Result<()> {
    throws!({
        let app_args_binding = AppArgs::command();
        if let Some(it) = app_args_binding.find_subcommand(subcommand) {
            for arg in it.get_arguments() {
                if arg.get_long() == Some(option) {
                    let help = arg.get_help();
                    if let Some(help) = help {
                        let output = format!("{}", help);
                        println!("{}", output);
                    }
                }
            }
        }
    });
}

fn get_possible_values_for_subcommand_and_option(
    subcommand: &str,
    option: &str,
) -> Vec<String> {
    let app_args_binding = AppArgs::command();

    if let Some(it) = app_args_binding.find_subcommand(subcommand) {
        for arg in it.get_arguments() {
            if arg.get_long() == Some(option) {
                let possible_values = arg.get_possible_values();
                let possible_values = possible_values
                    .iter()
                    .map(|it| it.get_name().to_string())
                    .collect::<Vec<_>>();

                DEVELOPMENT_MODE.then(|| {
                    // % is Display, ? is Debug.
                    tracing::debug!(
                        message = %inline_string!("{subcommand}, {option}"),
                        possible_values = ?possible_values,
                    );
                });

                return possible_values;
            }
        }
    }

    vec![]
}
