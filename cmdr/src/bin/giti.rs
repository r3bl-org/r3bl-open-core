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

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_config::*;
use giti::{branch::delete::try_delete_branch,
           giti_ui_templates::ask_user_to_select_from_list,
           *};
use r3bl_cmdr::giti;
use r3bl_rs_utils_core::{call_if_true,
                         log_debug,
                         log_error,
                         try_to_set_log_level,
                         CommonResult};
use r3bl_tuify::SelectionMode;

fn main() {
    // If no args are passed, the following line will fail, and help will be printed
    // thanks to `arg_required_else_help(true)` in the `CliArgs` struct.
    let cli_arg = CLIArg::parse();

    let enable_logging = cli_arg.global_options.enable_logging;
    call_if_true!(enable_logging, {
        try_to_set_log_level(log::LevelFilter::Trace).ok();
        log_debug("Start logging...".to_string());
        log_debug(format!("cli_args {:?}", cli_arg));
    });

    if let Err(error) = try_run_program(cli_arg) {
        log_error(format!("Error running program, error: {:?}", error));
        println!("Error running program! 🤦‍♀️");
    }

    call_if_true!(enable_logging, {
        log_debug("Stop logging...".to_string());
    });
}

fn try_run_program(giti_app_args: CLIArg) -> CommonResult<()> {
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
        _ => {
            unimplemented!()
        }
    }
    Ok(())
}

pub fn get_giti_command_subcommand_names(arg: CLICommand) -> Vec<String> {
    match arg {
        CLICommand::Branch { .. } => BranchSubcommand::value_variants()
            .iter()
            .map(|subcommand| {
                let lower_case_subcommand =
                    format!("{:?}", subcommand).to_ascii_lowercase();
                lower_case_subcommand
            })
            .collect(),
        _ => unimplemented!(),
    }
}

/// More info:
/// - <https://docs.rs/clap/latest/clap/_derive/#overview>
/// - <https://developerlife.com/2023/09/17/tuify-clap/>
mod clap_config {
    use super::*;

    #[derive(Debug, Parser)]
    #[command(bin_name = "giti")]
    #[command(
        about = "Version control with confidence 💪\n\x1b[38;5;206mEarly access preview \x1b[0m🐣"
    )]
    #[command(version)]
    #[command(next_line_help = true)]
    #[command(arg_required_else_help(true))]
    /// More info: <https://docs.rs/clap/latest/clap/struct.Command.html#method.help_template>
    #[command(
        help_template = "{about}\nVersion: {bin} {version} 💻\n\nUSAGE 📓:\n  giti [\x1b[32mCommand\x1b[0m] [\x1b[34mOptions\x1b[0m]\n\n{all-args}\n",
        subcommand_help_heading("Command")
    )]
    pub struct CLIArg {
        #[command(subcommand)]
        pub command: CLICommand,

        #[command(flatten)]
        pub global_options: GlobalOption,
    }

    #[derive(Debug, Args)]
    pub struct GlobalOption {
        #[arg(
            global = true,
            long,
            short = 'l',
            help = "Log app output to a file named `log.txt` for debugging."
        )]
        pub enable_logging: bool,
    }

    #[derive(Debug, Subcommand)]
    pub enum CLICommand {
        #[clap(
            about = "🌱 Manage your git branches with commands: `delete`, `checkout`, and `new`\n💡 Eg: `giti branch delete`"
        )]
        /// More info: <https://docs.rs/clap/latest/clap/struct.Command.html#method.help_template>
        #[command(
            help_template = "{about} \n\nUSAGE 📓:\n  giti branch [\x1b[34mcommand\x1b[0m] [\x1b[32moptions\x1b[0m]\n\n{positionals}\n\n  [options]\n{options}"
        )]
        Branch {
            #[arg(
                value_name = "command",
                help = "In your shell, this command will execute, taking each selected item as an argument."
            )]
            command_to_run_with_each_selection: Option<BranchSubcommand>,
        },

        #[clap(about = "TODO Commit help")]
        Commit {},

        #[clap(about = "TODO Remote help")]
        Remote {},
    }

    #[derive(Clone, Debug, ValueEnum)]
    pub enum BranchSubcommand {
        #[clap(help = "Delete one or more selected branches")]
        Delete,
        #[clap(help = "TODO Checkout a selected branch")]
        Checkout,
        #[clap(help = "TODO Create a new branch")]
        New,
    }
}
