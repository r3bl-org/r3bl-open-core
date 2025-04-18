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
use std::{fmt::{Debug, Display, Error, Result as FmtResult},
          result::Result as StdResult};

use tokio::process::Command;

/// Hold all the possible outcomes of executing a [tokio::process::Command]. This is used
/// to report the success or failure to a user (display via stdout output) or operator
/// (debug via log output).
#[derive(Debug)]
pub enum CommandRunResult<T: Debug + Display> {
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

mod display_impl_for_command_run_result {
    use std::fmt::{Debug, Display, Formatter};

    use super::*;
    use crate::{fg_lizard_green,
                fg_orange,
                fg_pink,
                fg_slate_gray,
                inline_string,
                InlineString,
                InlineVec};

    impl<T: Debug + Display> Display for CommandRunResult<T> {
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
                CommandRunResult::RanUnsuccessfullyOrFailedToRun(
                    message,
                    cmd,
                    output,
                ) => {
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

    pub fn fmt_details_str<T: Display>(details: &T) -> StdResult<InlineString, Error> {
        let details_str = inline_string!("{details}");
        let fmt_details_str = fg_slate_gray(&details_str).to_small_str();
        Ok(fmt_details_str)
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
}
