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

//! For more information on how to use CLAP, here are some resources:
//! 1. [Tutorial](https://developerlife.com/2023/09/17/tuify-clap/)
//! 2. [Video](https://youtu.be/lzMYDA6St0s)

use clap::Parser;
use r3bl_cmdr::{AnalyticsAction,
                giti::{CLIArg,
                       CLICommand,
                       CommandExecutionReport,
                       UIStrings,
                       branch,
                       ui_templates::{self}},
                report_analytics,
                upgrade_check};
use r3bl_core::{CommonResult,
                fg_guards_red,
                log_support::try_initialize_logging_global,
                throws};

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
    // Figure out which control path to take. Then execute the command for that path.
    let res_cmd_exec = match cli_arg.command {
        CLICommand::Branch {
            command_to_run_with_each_selection,
            maybe_branch_name,
        } => {
            branch::try_main(command_to_run_with_each_selection, maybe_branch_name).await
        }
        CLICommand::Commit {} | CLICommand::Remote {} => unimplemented!(),
    };

    // Handle the result of the command execution.
    // 01: handle the output of the command execution, since no output has been printed yet
    match res_cmd_exec {
        // Command ran successfully.
        Ok(cmd_exec_report) => {
            if let CommandExecutionReport::BranchDelete(details) = cmd_exec_report {
                // If user selected to delete a branch, then show exit message. If
                // user didn't select any branch, then show message that no branches
                // were deleted.
                if details.maybe_deleted_branches.is_none() {
                    println!("{}", UIStrings::NoBranchGotDeleted);
                }
            }

            ui_templates::show_exit_message();
        }
        // Handle unrecoverable / unknown errors.
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

            // 01: don't print this to stdout, just log it (since it should have been printed already)
            fg_guards_red(&format!(
                " Could not run giti due to the following problem.\n{:#?}",
                error
            ))
            .println();
        }
    }
}
