/*
 *   Copyright (c) 2025 R3BL LLC
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
                       CommandRunDetails,
                       branch,
                       ui_str,
                       ui_templates::{self}},
                report_analytics,
                upgrade_check};
use r3bl_tui::{CommandRunResult,
               CommonResult,
               log::try_initialize_logging_global,
               ok,
               set_jemalloc_in_main};

#[tokio::main]
#[allow(clippy::needless_return)]
async fn main() -> CommonResult<()> {
    set_jemalloc_in_main!();

    // If no args are passed, the following line will fail, and help will be printed
    // thanks to `arg_required_else_help(true)` in the `CliArgs` struct.
    let cli_arg = CLIArg::parse();

    let should_log = cli_arg.global_options.enable_logging;

    should_log.then(|| {
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

    should_log.then(|| {
        tracing::debug!(message = "Stop logging...");
    });

    ok!()
}

pub async fn launch_giti(cli_arg: CLIArg) {
    // Figure out which control path to take. Then execute the command for that path.
    let res = match cli_arg.command {
        CLICommand::Branch {
            sub_cmd,
            maybe_branch_name,
        } => branch::handle_branch_command(sub_cmd, maybe_branch_name).await,
        CLICommand::Commit {} => unimplemented!(),
        CLICommand::Remote {} => unimplemented!(),
    };

    // Handle the result of the command execution.
    match res {
        // This branch is for both successful and unsuccessful command executions. Even
        // though the `res` is not `Err` it does not mean that the command ran
        // successfully, it may have failed gracefully.
        Ok(cmd_run_result) => {
            display_command_run_result(cmd_run_result).await;
        }
        // This branch is for strange errors like terminal not interactive.
        Err(error) => {
            report_unrecoverable_errors(error);
        }
    }
}

/// Unknown and unrecoverable errors: readline_async or choose not working.
pub fn report_unrecoverable_errors(report: miette::Report) {
    report_analytics::start_task_to_generate_event(
        "".to_string(),
        AnalyticsAction::GitiFailedToRun,
    );

    // % is Display, ? is Debug.
    tracing::error!(
        message = "Could not run giti due to the following problem",
        error = ?report
    );

    println!("{}", ui_str::unrecoverable_error_message(report));
}

/// Command ran and produced result: success, not success, fail, no-op.
pub async fn display_command_run_result(
    cmd_run_result: CommandRunResult<CommandRunDetails>,
) {
    println!("{cmd_run_result}");
    ui_templates::show_exit_message();
}
