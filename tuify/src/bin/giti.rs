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

use crate::giti::branch::delete::try_delete_branch;
use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_config::*;
use r3bl_rs_utils_core::{call_if_true, log_debug, log_error, try_to_set_log_level, CommonResult};
use r3bl_tuify::{giti_ui_templates::ask_user_to_select_from_list, *};

fn main() {
    // If no args are passed, the following line will fail, and help will be printed
    // thanks to `arg_required_else_help(true)` in the `CliArgs` struct.
    let giti_app_args = GitiAppArg::parse();

    let enable_logging = DEVELOPMENT_MODE | giti_app_args.global_options.enable_logging;
    call_if_true!(enable_logging, {
        try_to_set_log_level(log::LevelFilter::Trace).ok();
        log_debug("Start logging...".to_string());
        log_debug(format!("cli_args {:?}", giti_app_args));
    });

    if let Err(error) = try_run_program(giti_app_args) {
        log_error(format!("Error running program, error: {:?}", error));
        println!("Error running program! 🤦‍♀️");
    }

    call_if_true!(enable_logging, {
        log_debug("Stop logging...".to_string());
    });
}

fn try_run_program(giti_app_args: GitiAppArg) -> CommonResult<()> {
    match giti_app_args.command {
        CLICommand::Branch {
            command_to_run_with_each_selection,
            ..
        } => match command_to_run_with_each_selection {
            Some(subcommand) => match subcommand {
                BranchSubcommand::Delete => {
                    try_delete_branch()?;
                }
                _ => unimplemented!(),
            },
            _ => {
                // Show all the branch sub-commands (delete, checkout, new, etc.) in a tuify component.
                giti_ui_templates::single_select_instruction_header();
                let options = get_giti_command_subcommand_names(CLICommand::Branch {
                    selection_mode: Some(SelectionMode::Single),
                    command_to_run_with_each_selection: None,
                });

                let maybe_selected = ask_user_to_select_from_list(
                    options,
                    "Please select a branch subcommand".to_string(),
                    SelectionMode::Single,
                );

                if let Some(selected) = maybe_selected {
                    match selected[0].as_str() {
                        "delete" => {
                            try_delete_branch()?;
                        }
                        _ => unimplemented!(),
                    }
                }
            }
        },
    }
    Ok(())
}

pub fn get_giti_command_subcommand_names(arg: CLICommand) -> Vec<String> {
    match arg {
        CLICommand::Branch { .. } => BranchSubcommand::value_variants()
            .iter()
            .map(|subcommand| {
                let lower_case_subcommand = format!("{:?}", subcommand).to_ascii_lowercase();
                lower_case_subcommand
            })
            .collect(),
    }
}

/// More info:
/// - <https://docs.rs/clap/latest/clap/_derive/#overview>
/// - <https://developerlife.com/2023/09/17/tuify-clap/>
mod clap_config {
    use super::*;

    #[derive(Debug, Parser)]
    #[command(bin_name = "giti")]
    #[command(about = "Easy to use, interactive, tuified git", long_about = None)]
    #[command(version)]
    #[command(next_line_help = true)]
    #[command(arg_required_else_help(true))]
    pub struct GitiAppArg {
        #[clap(subcommand)]
        pub command: CLICommand,

        #[clap(flatten)]
        pub global_options: GlobalOption,
    }

    #[derive(Debug, Args)]
    pub struct GlobalOption {
        /// Enables logging to a file named `log.txt`.
        #[arg(long, short = 'l', help = "Enables logging to a file")]
        pub enable_logging: bool,

        /// Sets the maximum height of the Tuify component (rows).
        /// If height is not provided, it defaults to the terminal height.
        #[arg(
            value_name = "height",
            long,
            short = 'h',
            help = "Sets the maximum height of the Tuify component (rows)"
        )]
        pub tuify_height: Option<usize>,

        /// Sets the maximum width of the Tuify component (columns).
        /// If width is not provided, it defaults to the terminal width.
        #[arg(
            value_name = "width",
            long,
            short = 'w',
            help = "Sets the maximum width of the Tuify component (columns)"
        )]
        pub tuify_width: Option<usize>,
    }

    #[derive(Debug, Subcommand)]
    pub enum CLICommand {
        /// Manages giti branches. This command has subcommands like `delete`, `checkout`, and `new`. 🌿
        Branch {
            /// Select one or more items to operate on. Available modes are: `Single` or `Multi`.
            #[arg(value_name = "mode", long, short = 's')]
            selection_mode: Option<SelectionMode>,

            /// This command will be executed in your shell with each selected item as an argument.
            #[arg(value_name = "command")]
            command_to_run_with_each_selection: Option<BranchSubcommand>,
        },
    }

    #[derive(Clone, Debug, ValueEnum)]
    pub enum BranchSubcommand {
        Delete,
        Checkout,
        New,
    }
}
