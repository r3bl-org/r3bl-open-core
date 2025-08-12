// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.
use std::{fmt::{Debug, Display, Error, Result as FmtResult},
          result::Result as StdResult};

use tokio::process::Command;

use crate::InlineString;

/// Hold all the possible outcomes of executing a [`tokio::process::Command`]. This is
/// used to report the success or failure to a user (display via stdout output) or
/// operator (debug via log output).
#[derive(Debug)]
pub enum CommandRunResult<T: Debug + Display> {
    /// Command was not run (probably because the command would be a no-op).
    Noop(
        /* no-op message */ InlineString,
        /* command-specific details */ T,
    ),

    /// Command ran, and produced success `request_shutdown` code.
    Run(
        /* success message */ InlineString,
        /* command-specific details */ T,
        /* command */ Command,
    ),

    /// Command ran and produced non-zero `request_shutdown` code. Or it failed to run,
    /// and never got the chance to generate an `request_shutdown` code.
    Fail(
        /* error message */ InlineString,
        /* command */ Command,
        /* error report */ miette::Report,
    ),
}

/// Display impl for [`CommandRunResult`]. This also generates log output.
pub(crate) mod display_impl_for_command_run_result {
    use std::fmt::{Debug, Display, Formatter};

    use super::{Command, CommandRunResult, Error, FmtResult, StdResult};
    use crate::{InlineString, InlineVec, fg_lizard_green, fg_orange, fg_pink,
                fg_slate_gray, inline_string};

    impl<T: Debug + Display> Display for CommandRunResult<T> {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            match self {
                CommandRunResult::Fail(error_msg, cmd, error_report) => {
                    let header = "🗴 Command ran unsuccessfully";
                    // Log output. % is Display, ? is Debug.
                    tracing::error!(
                        message = %header,
                        error_msg = %error_msg,
                        cmd = %fmt_cmd_str(cmd)?,
                        error_report = ?error_report
                    );
                    // Display output.
                    write!(
                        f,
                        "{a}\n{b}",
                        a = fg_pink(header).to_small_str(),
                        b = error_msg,
                    )
                }
                CommandRunResult::Run(success_msg, cmd_details, cmd) => {
                    let header = "🗸 Command ran successfully";
                    // Log output. % is Display, ? is Debug.
                    tracing::info!(
                        message = %header,
                        success_msg = %success_msg,
                        cmd_details = %fmt_details_str(cmd_details),
                        cmd = %fmt_cmd_str(cmd)?
                    );
                    // Display output.
                    write!(
                        f,
                        "{a}\n{b}",
                        a = fg_lizard_green(header).to_small_str(),
                        b = success_msg,
                    )
                }
                CommandRunResult::Noop(noop_msg, cmd_details) => {
                    let header = "❯ No command ran";
                    // Log output. % is Display, ? is Debug.
                    tracing::warn!(
                        message = %header,
                        noop_msg = %noop_msg,
                        cmd_details = %fmt_details_str(cmd_details)
                    );
                    // Display output.
                    write!(
                        f,
                        "{a}\n{b}",
                        a = fg_orange(header).to_small_str(),
                        b = noop_msg
                    )
                }
            }
        }
    }

    pub fn fmt_details_str<T: Display>(details: &T) -> InlineString {
        let details_str = inline_string!("{details}");
        fg_slate_gray(&details_str).to_small_str()
    }

    /// Format the command as a string for display.
    pub fn fmt_cmd_str(cmd: &Command) -> StdResult<InlineString, Error> {
        use std::fmt::Write;

        // Convert the tokio::process::Command to a standard Command.
        let cmd = cmd.as_std();

        let cmd_str = {
            let mut acc = InlineString::new();

            writeln!(acc, "Command {{")?;
            writeln!(acc, "  Program: {},", cmd.get_program().display())?;
            writeln!(
                acc,
                "  Args: {:?},",
                cmd.get_args().collect::<InlineVec<_>>()
            )?;
            writeln!(
                acc,
                "  Env: {:?},",
                cmd.get_envs().collect::<InlineVec<_>>()
            )?;
            writeln!(acc, "  Current Dir: {:?}", cmd.get_current_dir())?;
            writeln!(acc, "}}")?;

            acc
        };

        let fmt_cmd_str = fg_slate_gray(&cmd_str).to_small_str();

        Ok(fmt_cmd_str)
    }
}
