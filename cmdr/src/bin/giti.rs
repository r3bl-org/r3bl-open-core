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

use clap::Parser;
use r3bl_ansi_color::{AnsiStyledText, Style};
use r3bl_cmdr::{AnalyticsAction,
                color_constants::DefaultColors::{FrozenBlue, GuardsRed, MoonlightBlue},
                giti::{BranchSubcommand,
                       CLIArg,
                       CLICommand,
                       CommandSuccessfulResponse,
                       get_giti_command_subcommand_names,
                       giti_ui_templates,
                       single_select_instruction_header,
                       try_checkout_branch,
                       try_delete_branch,
                       try_make_new_branch},
                report_analytics,
                upgrade_check};
use r3bl_core::{CommonResult, call_if_true, throws};
use r3bl_log::try_initialize_logging_global;
use r3bl_tuify::{SelectionMode, StyleSheet, select_from_list_with_multi_line_header};

#[tokio::main]
#[allow(clippy::needless_return)]
async fn main() -> CommonResult<()> {
    throws!({
        // If no args are passed, the following line will fail, and help will be printed
        // thanks to `arg_required_else_help(true)` in the `CliArgs` struct.
        let cli_arg = CLIArg::parse();

        let enable_logging = cli_arg.global_options.enable_logging;
        call_if_true!(enable_logging, {
            try_initialize_logging_global(tracing_core::LevelFilter::DEBUG).ok();
            tracing::debug!("Start logging... cli_args {:?}", cli_arg);
        });

        // Check analytics reporting.
        if cli_arg.global_options.no_analytics {
            report_analytics::disable();
        }

        upgrade_check::start_task_to_check_for_updates();
        report_analytics::start_task_to_generate_event(
            "".to_string(),
            AnalyticsAction::GitiAppStart,
        );

        launch_giti(cli_arg);

        call_if_true!(enable_logging, {
            tracing::debug!("Stop logging...");
        });
    })
}

pub fn launch_giti(cli_arg: CLIArg) {
    match try_run_command(&cli_arg) {
        // Command ran successfully.
        Ok(try_run_command_result) => {
            if let CLICommand::Branch { .. } = cli_arg.command {
                // If user selected to delete a branch, then show exit message. If user
                // didn't select any branch, then show message that no branches were
                // deleted.
                match (
                    try_run_command_result.maybe_deleted_branches,
                    try_run_command_result.branch_subcommand,
                ) {
                    (Some(_), Some(BranchSubcommand::Delete)) => {
                        giti_ui_templates::show_exit_message();
                    }
                    (None, Some(BranchSubcommand::Delete)) => {
                        println!(" You chose not to delete any branches.");
                        giti_ui_templates::show_exit_message();
                    }
                    _ => {}
                }
            }
        }
        // Handle unrecoverable / unknown errors here.
        Err(error) => {
            report_analytics::start_task_to_generate_event(
                "".to_string(),
                AnalyticsAction::GitiFailedToRun,
            );

            let err_msg = format!(
                " Could not run giti due to the following problem.\n{:#?}",
                error
            );
            tracing::error!(err_msg);
            AnsiStyledText {
                text: &err_msg.to_string(),
                style: &[Style::Foreground(GuardsRed.as_ansi_color())],
            }
            .println();
        }
    }
}

pub fn try_run_command(
    giti_app_args: &CLIArg,
) -> CommonResult<CommandSuccessfulResponse> {
    match &giti_app_args.command {
        CLICommand::Branch {
            command_to_run_with_each_selection,
            maybe_branch_name,
            ..
        } => match command_to_run_with_each_selection {
            Some(subcommand) => match subcommand {
                BranchSubcommand::Delete => try_delete_branch(),
                BranchSubcommand::Checkout => {
                    try_checkout_branch(maybe_branch_name.clone())
                }
                BranchSubcommand::New => try_make_new_branch(maybe_branch_name.clone()),
            },
            _ => user_typed_giti_branch(),
        },
        CLICommand::Commit {} => unimplemented!(),
        CLICommand::Remote {} => unimplemented!(),
    }
}

fn user_typed_giti_branch() -> CommonResult<CommandSuccessfulResponse> {
    let branch_subcommands = get_giti_command_subcommand_names(CLICommand::Branch {
        command_to_run_with_each_selection: None,
        maybe_branch_name: None,
    });
    let default_header_style = [
        Style::Foreground(FrozenBlue.as_ansi_color()),
        Style::Background(MoonlightBlue.as_ansi_color()),
    ];
    let instructions_and_select_branch_subcommand = {
        let mut instructions_and_select_branch_subcommand =
            single_select_instruction_header();
        let header = AnsiStyledText {
            text: "Please select a branch subcommand",
            style: &default_header_style,
        };
        instructions_and_select_branch_subcommand.push(vec![header]);
        instructions_and_select_branch_subcommand
    };
    let maybe_selected = select_from_list_with_multi_line_header(
        instructions_and_select_branch_subcommand,
        branch_subcommands,
        Some(20),
        None,
        SelectionMode::Single,
        StyleSheet::default(),
    );
    if let Some(selected) = maybe_selected {
        let it = selected[0].as_str();
        match it {
            "delete" => return try_delete_branch(),
            "checkout" => return try_checkout_branch(None),
            "new" => return try_make_new_branch(None),
            _ => unimplemented!(),
        };
    };

    Ok(CommandSuccessfulResponse::default())
}
