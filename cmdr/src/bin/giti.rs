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

//! For more information on how to use CLAP and Tuify, here are some resources:
//! 1. [Tutorial](https://developerlife.com/2023/09/17/tuify-clap/)
//! 2. [Video](https://youtu.be/lzMYDA6St0s)

use clap::{Parser, ValueEnum};
use r3bl_cmdr::{AnalyticsAction,
                giti::{BranchSubcommand,
                       CLIArg,
                       CLICommand,
                       SuccessReport,
                       get_giti_command_subcommand_names,
                       try_checkout_branch,
                       try_delete_branch,
                       try_make_new_branch,
                       ui_templates,
                       ui_templates::single_select_instruction_header},
                report_analytics,
                upgrade_check};
use r3bl_core::{CommonResult,
                ast,
                fg_guards_red,
                height,
                log_support::try_initialize_logging_global,
                new_style,
                throws,
                tui_color};
use r3bl_tui::{DefaultIoDevices,
               choose,
               terminal_async::{HowToChoose, StyleSheet}};
use smallvec::smallvec;

#[tokio::main]
#[allow(clippy::needless_return)]
async fn main() -> CommonResult<()> {
    throws!({
        // If no args are passed, the following line will fail, and help will be printed
        // thanks to `arg_required_else_help(true)` in the `CliArgs` struct.
        let cli_arg = CLIArg::parse();

        let enable_logging = cli_arg.global_options.enable_logging;
        enable_logging.then(|| {
            try_initialize_logging_global(tracing_core::LevelFilter::DEBUG).ok();
            // % is Display, ? is Debug.
            tracing::debug!(message = "Start logging...", cli_arg = ?cli_arg);
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

        launch_giti(cli_arg).await;

        enable_logging.then(|| {
            tracing::debug!(message = "Stop logging...");
        });
    })
}

pub async fn launch_giti(cli_arg: CLIArg) {
    let res = try_run_command(&cli_arg).await;
    match res {
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
                        ui_templates::show_exit_message();
                    }
                    (None, Some(BranchSubcommand::Delete)) => {
                        println!(" You chose not to delete any branches.");
                        ui_templates::show_exit_message();
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

            // % is Display, ? is Debug.
            tracing::error!(
                message = "Could not run giti due to the following problem",
                error = ?error
            );

            fg_guards_red(&format!(
                " Could not run giti due to the following problem.\n{:#?}",
                error
            ))
            .println();
        }
    }
}

pub async fn try_run_command(giti_app_args: &CLIArg) -> CommonResult<SuccessReport> {
    match &giti_app_args.command {
        CLICommand::Branch {
            command_to_run_with_each_selection,
            maybe_branch_name,
            ..
        } => match command_to_run_with_each_selection {
            Some(subcommand) => match subcommand {
                BranchSubcommand::Delete => try_delete_branch().await,
                BranchSubcommand::Checkout => {
                    try_checkout_branch(maybe_branch_name.clone()).await
                }
                BranchSubcommand::New => {
                    try_make_new_branch(maybe_branch_name.clone()).await
                }
            },
            _ => user_typed_giti_branch().await,
        },
        CLICommand::Commit {} => unimplemented!(),
        CLICommand::Remote {} => unimplemented!(),
    }
}

async fn user_typed_giti_branch() -> CommonResult<SuccessReport> {
    let branch_subcommands = get_giti_command_subcommand_names(CLICommand::Branch {
        command_to_run_with_each_selection: None,
        maybe_branch_name: None,
    });

    let default_header_style = new_style!(
        color_fg: {tui_color!(frozen_blue)} color_bg: {tui_color!(moonlight_blue)}
    );
    let instructions_and_select_branch_subcommand = {
        let mut lines = single_select_instruction_header();
        let header_line = ast("Please select a branch subcommand", default_header_style);
        lines.push(smallvec![header_line]);
        lines
    };

    let mut default_io_devices = DefaultIoDevices::default();
    let selected = choose(
        instructions_and_select_branch_subcommand,
        branch_subcommands,
        Some(height(20)),
        None,
        HowToChoose::Single,
        StyleSheet::default(),
        default_io_devices.as_mut_tuple(),
    )
    .await?;

    if let Some(selected) = selected.first() {
        if let Ok(branch_subcommand) = BranchSubcommand::from_str(selected, true) {
            match branch_subcommand {
                BranchSubcommand::Delete => return try_delete_branch().await,
                BranchSubcommand::Checkout => return try_checkout_branch(None).await,
                BranchSubcommand::New => return try_make_new_branch(None).await,
            }
        } else {
            unimplemented!();
        }
    };

    Ok(SuccessReport::default())
}
