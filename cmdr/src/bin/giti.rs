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

use std::process::Command;

use clap::Parser;
use r3bl_cmdr::{AnalyticsAction,
                giti::{CLIArg,
                       CLICommand,
                       CommandRunDetails,
                       CommandRunResult::{self,
                                          DidNotRun,
                                          FailedToRun,
                                          RanSuccessfully,
                                          RanUnsuccessfully},
                       UIStrings,
                       branch,
                       ui_templates::{self}},
                report_analytics,
                upgrade_check};
use r3bl_core::{CommonError,
                CommonErrorType,
                CommonResult,
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
    let res = match cli_arg.command {
        CLICommand::Branch {
            command_to_run_with_each_selection,
            maybe_branch_name,
        } => {
            branch::try_main(command_to_run_with_each_selection, maybe_branch_name).await
        }
        CLICommand::Commit {} | CLICommand::Remote {} => unimplemented!(),
    };

    // Handle the result of the command execution.
    match res {
        Ok(cmd_run_result) => {
            display_command_run_result(cmd_run_result).await;
        }
        Err(error) => {
            report_unrecoverable_errors(error);
        }
    }
}

/// Unknown and unrecoverable errors: readline_async or choose not working.
pub fn report_unrecoverable_errors(error: miette::Report) {
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
    fg_guards_red(
        &UIStrings::UnrecoverableErrorEncountered {
            report: error.to_string(),
        }
        .to_string(),
    )
    .println();
}

/// Call this function when you can't execute [Command::output] and something unknown has
/// gone wrong. Propagate the error to the caller since it is not recoverable and can't be
/// handled.
// 01: use parts of this in report_unrecoverable_errors()
pub fn report_error_and_propagate<T>(
    command: &mut Command,
    command_output_error: miette::Report,
) -> CommonResult<T> {
    let program_name_to_string: String =
        command.get_program().to_string_lossy().to_string();

    let command_args_to_string: String = {
        let mut it = vec![];
        for item in command.get_args() {
            it.push(item.to_string_lossy().to_string());
        }
        it.join(" ")
    };

    let error_msg = UIStrings::ErrorExecutingCommand {
        program_name_to_string,
        command_args_to_string,
        command_output_error,
    }
    .to_string();

    // % is Display, ? is Debug.
    tracing::error!(
        message = "report_unknown_error_and_propagate",
        error_msg = %error_msg
    );

    CommonError::new_error_result::<T>(CommonErrorType::CommandExecutionError, &error_msg)
}

/// Command ran and produced result: success, not success, fail, no-op.
pub async fn display_command_run_result(cmd_run_result: CommandRunResult) {
    // 01: handle the output of the command execution, since no output has been printed yet
    match cmd_run_result {
        DidNotRun(_maybe_message, command_run_details) => match command_run_details {
            CommandRunDetails::BranchDelete(details) => {
                if details.maybe_deleted_branches.is_none() {
                    println!("{}", UIStrings::NoBranchGotDeleted);
                }
            }
            CommandRunDetails::BranchNew(_branch_new_details) => todo!(),
            CommandRunDetails::BranchCheckout(_branch_checkout_details) => todo!(),
            CommandRunDetails::Commit => todo!(),
            CommandRunDetails::Remote => todo!(),
        },
        RanSuccessfully(_success_message, _command_run_details) => todo!(),
        RanUnsuccessfully(_error_message, _command, _output) => todo!(),
        FailedToRun(_error_message, _command, _report) => todo!(),
    }

    ui_templates::show_exit_message();
}
