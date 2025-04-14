/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

use std::process::{Command, Output};

use r3bl_core::{CommonError, CommonErrorType, CommonResult, ItemsOwned};

use crate::giti::UIStrings;

/// Detailed information about a sub command that has run successfully.
#[derive(Debug, Clone, Default)]
pub struct BranchDeleteDetails {
    pub maybe_deleted_branches: Option<ItemsOwned>,
}

/// Detailed information about a sub command that has run successfully.
#[derive(Debug, Clone, Default)]
pub struct BranchNewDetails {
    pub maybe_created_branch: Option<String>,
}

/// Detailed information about a sub command that has run successfully.
#[derive(Debug, Clone, Default)]
pub struct BranchCheckoutDetails {
    pub maybe_checked_out_branch: Option<String>,
}

/// Information about command and subcommand that has run successfully. Eg: `giti branch
/// delete` or `giti branch checkout` or `giti branch new`.
#[derive(Debug, Clone)]
pub enum CommandExecutionReport {
    BranchDelete(BranchDeleteDetails),
    BranchNew(BranchNewDetails),
    BranchCheckout(BranchCheckoutDetails),
    Commit,
    Report,
}

/// Call this function when you can't execute [Command::output] and something unknown has
/// gone wrong. Propagate the error to the caller since it is not recoverable and can't be
/// handled.
// 01: this should only be called after it has already been printed
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

// 01: apply this to all subcommands and eliminate one-off error and status reporting at the subcommand level
#[rustfmt::skip]
/// A command is something that is run by `giti` in the underlying OS. This is meant to
/// hold all the possible outcomes of executing a [std::process::Command].
pub enum CompletionReport {
    /// Command was not run (probably because the command would be a no-op).
    CommandDidNotRun(
        /* command specific details */ CommandExecutionReport
    ),

    /// Command ran, and produced success exit code.
    CommandRanSuccessfully(
        /* success message */ String,
        /* command specific details */ CommandExecutionReport,
    ),

    /// Command ran, and produced non-zero exit code.
    CommandRanUnsuccessfully(
        /* error message */ String,
        /* command */ Command,
        /* stdout or stderr */ Output,
    ),

    /// Attempt to run the command failed. It never ran.
    CommandFailedToRun(
        /* error message */ String,
        /* command */ Command,
        /* error report */ miette::Report,
    ),
}
