// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CommandRunError, InlineString, InlineVec, TokioCommand, fg_lizard_green,
            fg_orange, fg_pink, fg_slate_gray};
use std::{fmt::{Debug, Display, Error, Formatter},
          result::Result as StdResult};

/// Hold all the possible outcomes of executing a [`TokioCommand`].
///
/// This is used to report the success or failure to a user (display via [`stdout`]
/// output) or operator (debug via log output).
///
/// # Summary
/// - High-level command execution report for CLI tools
/// - Three states: Noop (command not run), Run (success), Fail (error)
/// - Carries user-friendly messages and command details
/// - Uses [`TokioCommand`] for one-shot command execution
///
/// [`stdout`]: std::io::stdout
#[derive(Debug)]
pub enum CommandRunReport<T: Debug + Display> {
    /// Command was not run (probably because the command would be a no-op).
    Noop(
        /* no-op message */ InlineString,
        /* command-specific details */ T,
    ),

    /// Command ran, and produced success `request_shutdown` code.
    Run(
        /* success message */ InlineString,
        /* command-specific details */ T,
        /* command */ TokioCommand,
    ),

    /// Command ran and produced non-zero `request_shutdown` code. Or it failed to run,
    /// and never got the chance to generate an `request_shutdown` code.
    Fail(
        /* error message */ InlineString,
        /* command */ TokioCommand,
        /* error */ CommandRunError,
    ),
}

/// Display impl for [`CommandRunReport`]. This also generates log output.
mod display_impl_for_command_run_report {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl<T: Debug + Display> Display for CommandRunReport<T> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                CommandRunReport::Fail(error_msg, cmd, error_report) => {
                    let header = "🗴 Command ran unsuccessfully";
                    // Log output. % is Display, ? is Debug.
                    tracing::error!(
                        message = %header,
                        error_msg = %error_msg,
                        cmd = %try_fmt_cmd_str(cmd)?,
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
                CommandRunReport::Run(success_msg, cmd_details, cmd) => {
                    let header = "🗸 Command ran successfully";
                    // Log output. % is Display, ? is Debug.
                    tracing::info!(
                        message = %header,
                        success_msg = %success_msg,
                        cmd_details = %fmt_details_str(cmd_details),
                        cmd = %try_fmt_cmd_str(cmd)?
                    );
                    // Display output.
                    write!(
                        f,
                        "{a}\n{b}",
                        a = fg_lizard_green(header).to_small_str(),
                        b = success_msg,
                    )
                }
                CommandRunReport::Noop(noop_msg, cmd_details) => {
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

    /// Formats the command as a string for display.
    pub fn try_fmt_cmd_str(cmd: &TokioCommand) -> StdResult<InlineString, Error> {
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
