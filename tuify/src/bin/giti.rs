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

//! For more information on how to use CLAP and Tuify, please read this tutorial:
//! <https://developerlife.com/2023/09/17/tuify-clap/>

use std::io::Result;

#[allow(unused_imports)]
use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};
use r3bl_ansi_color::{AnsiStyledText, Color, Style};
use r3bl_rs_utils_core::*;
use r3bl_tuify::*;

#[derive(Debug, Parser)]
#[command(bin_name = "giti")]
#[command(about = "Easy to use, interactive, tuified git", long_about = None)]
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
    /// Print debug output to log file (log.txt)
    #[arg(long, short = 'l')]
    enable_logging: bool,

    /// Optional maximum height of the TUI (rows)
    #[arg(value_name = "height", long, short = 'r')]
    tui_height: Option<usize>,

    /// Optional maximum width of the TUI (columns)
    #[arg(value_name = "width", long, short = 'c')]
    tui_width: Option<usize>,
}

// TODO: What is the UX of this command? https://github.com/r3bl-org/r3bl-open-core/issues/187
#[derive(Debug, Subcommand)]
enum CLICommand {
    /// Show TUI to allow you to select one or more local branches for deletion 🌿
    Branch {
        /// Would you like to select one or more items?
        #[arg(value_name = "mode", long, short = 's')]
        selection_mode: Option<SelectionMode>,

        /// Each selected item is passed to this command as `%` and executed in your shell.
        /// For eg: "echo %". Please wrap the command in quotes 💡
        #[arg(value_name = "command", long, short = 'c')]
        command_to_run_with_each_selection: Option<String>,
    },
}

pub fn get_bin_name() -> String {
    let cmd = AppArgs::command();
    cmd.get_bin_name().unwrap_or("this command").to_string()
}

fn main() -> Result<()> {
    throws!({
        AnsiStyledText {
            text: "Hello, giti! 👋🐈",
            style: &[Style::Bold, Style::Foreground(Color::Rgb(100, 200, 1))],
        }
        .println();

        // If no args are passed, the following line will fail, and help will be printed
        // thanks to `arg_required_else_help(true)` in the `CliArgs` struct.
        let cli_args = AppArgs::parse();

        let enable_logging = TRACE | cli_args.global_opts.enable_logging;

        call_if_true!(enable_logging, {
            try_to_set_log_level(log::LevelFilter::Trace).ok();
            log_debug("Start logging...".to_string());
            log_debug(format!("og_size: {:?}", get_size()?).to_string());
            log_debug(format!("cli_args {:?}", cli_args));
        });

        // TODO: What is the UX of this command? https://github.com/r3bl-org/r3bl-open-core/issues/187
        match cli_args.command {
            CLICommand::Branch {
                selection_mode: _,
                command_to_run_with_each_selection: _,
            } => {
                // TODO: Implement this command
            }
        }

        call_if_true!(enable_logging, {
            log_debug("Stop logging...".to_string());
        });
    });
}
