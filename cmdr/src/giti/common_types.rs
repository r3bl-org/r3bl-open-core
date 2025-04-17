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

use std::{fmt::{Debug, Display, Error, Formatter, Result as FmtResult},
          result::Result as StdResult};

use r3bl_core::{InlineString,
                InlineVec,
                ItemsOwned,
                fg_lizard_green,
                fg_orange,
                fg_pink,
                fg_slate_gray,
                inline_string};
use tokio::process::Command;

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
pub enum CommandRunDetails {
    BranchDelete(BranchDeleteDetails),
    BranchNew(BranchNewDetails),
    BranchCheckout(BranchCheckoutDetails),
    Commit,
    Remote,
    Noop,
}

/// A command is something that is run by `giti` in the underlying OS. This is meant to
/// hold all the possible outcomes of executing a [tokio::process::Command].
#[derive(Debug)]
pub enum CommandRunResult<T: Debug> {
    /// Command was not run (probably because the command would be a no-op).
    DidNotRun(
        /* no-op message */ String,
        /* command specific details */ T,
    ),

    /// Command ran, and produced success exit code.
    RanSuccessfully(
        /* success message */ String,
        /* command specific details */ T,
        /* command */ Command,
    ),

    /// Command ran and produced non-zero exit code. Or it failed to run, and never got
    /// the chance to generate an exit code.
    RanUnsuccessfullyOrFailedToRun(
        /* error message */ String,
        /* command */ Command,
        /* error report */ miette::Report,
    ),
}

impl Display for CommandRunDetails {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            CommandRunDetails::BranchDelete(details) => {
                write!(
                    f,
                    " BranchDelete: {A:?}",
                    A = details.maybe_deleted_branches
                )
            }
            CommandRunDetails::BranchNew(details) => {
                write!(f, " BranchNew: {A:?}", A = details.maybe_created_branch)
            }
            CommandRunDetails::BranchCheckout(details) => {
                write!(
                    f,
                    " BranchCheckout: {A:?}",
                    A = details.maybe_checked_out_branch
                )
            }
            CommandRunDetails::Commit => write!(f, " Commit"),
            CommandRunDetails::Remote => write!(f, " Remote"),
            CommandRunDetails::Noop => write!(f, " Noop"),
        }
    }
}

/// Format the command as a string for display.
pub fn fmt_cmd_str(cmd: &Command) -> StdResult<InlineString, Error> {
    // Convert the tokio::process::Command to a standard Command.
    let cmd = cmd.as_std();

    use std::fmt::Write as _;

    let cmd_str = {
        let mut acc = InlineString::new();

        writeln!(acc, " Command {{\n")?;
        writeln!(acc, "   Program: {:?},\n", cmd.get_program())?;
        writeln!(
            acc,
            "   Args: {:?},\n",
            cmd.get_args().collect::<InlineVec<_>>()
        )?;
        writeln!(
            acc,
            "   Env: {:?},\n",
            cmd.get_envs().collect::<InlineVec<_>>()
        )?;
        writeln!(acc, "   Current Dir: {:?}\n", cmd.get_current_dir())?;
        writeln!(acc, " }}")?;

        acc
    };

    let fmt_cmd_str = fg_slate_gray(&cmd_str).to_small_str();

    Ok(fmt_cmd_str)
}

pub fn fmt_details_str(details: &CommandRunDetails) -> StdResult<InlineString, Error> {
    let details_str = inline_string!("{details}");
    let fmt_details_str = fg_slate_gray(&details_str).to_small_str();
    Ok(fmt_details_str)
}

impl Display for CommandRunResult<CommandRunDetails> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            CommandRunResult::RanSuccessfully(message, details, cmd) => {
                write!(
                    f,
                    "{a}\n{b}\n{c}\n{d}",
                    a = fg_lizard_green(" 🗸 Command ran successfully"),
                    b = message,
                    c = details,
                    d = fmt_cmd_str(cmd)?
                )?;

                write!(f, "{A:?}", A = cmd)
            }
            CommandRunResult::DidNotRun(message, details) => {
                write!(
                    f,
                    "{a}\n{b}\n{c}",
                    a = fg_orange(" ❯ Command did not run").to_small_str(),
                    b = message,
                    c = fmt_details_str(details)?
                )
            }
            CommandRunResult::RanUnsuccessfullyOrFailedToRun(message, cmd, output) => {
                write!(
                    f,
                    "{a}\n{b}\n{c}\n{D:?}",
                    a = fg_pink(" 🗴 Command ran unsuccessfully").to_small_str(),
                    b = message,
                    c = fmt_cmd_str(cmd)?,
                    D = output
                )
            }
        }
    }
}
