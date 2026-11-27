// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! For more information on how to use CLAP, here are some resources:
//! 1. [Tutorial]
//! 2. [Video]
//!
//! [Tutorial]: https://developerlife.com/2023/09/17/tuify-clap/
//! [Video]: https://youtu.be/lzMYDA6St0s

use clap::Parser;
use r3bl_cmdr::{AnalyticsAction,
                giti::{CLIArg, CLICommand, CommandRunDetails, branch, ui_str},
                report_analytics,
                upgrade_check::{self, ExitContext}};
use r3bl_tui::{CommandRunReport, log::try_initialize_logging_global,
               run_in_main_with_safe_stack};

fn main() {
    run_in_main_with_safe_stack!(async => main_impl());
}

async fn main_impl() {
    // If no args are passed, the following line will fail, and help will be printed
    // thanks to `arg_required_else_help(true)` in the `CliArgs` struct.
    let cli_arg = CLIArg::parse();

    let should_log = cli_arg.global_options.enable_logging;

    let _log_guard = should_log.then(|| {
        let guard = try_initialize_logging_global(tracing_core::LevelFilter::DEBUG).ok();
        // % is Display, ? is Debug.
        tracing::debug!(message = "Start logging...", cli_arg = ?cli_arg);
        guard
    });

    // Check analytics reporting.
    if cli_arg.global_options.no_analytics {
        report_analytics::disable();
    }

    upgrade_check::start_task_to_check_if_upgrade_is_needed();
    report_analytics::start_task_to_generate_event(
        String::new(),
        AnalyticsAction::GitiAppStart,
    );

    launch_giti(cli_arg).await;

    should_log.then(|| {
        tracing::debug!(message = "Stop logging...");
    });
}

pub async fn launch_giti(cli_arg: CLIArg) {
    // Figure out which control path to take. Then execute the command for that path.
    let res = match cli_arg.command {
        CLICommand::Branch {
            sub_cmd,
            maybe_branch_name,
        } => branch::try_handle_branch_command(sub_cmd, maybe_branch_name).await,
        CLICommand::Commit {} => unimplemented!(),
        CLICommand::Remote {} => unimplemented!(),
    };

    // Handle the result of the command execution.
    match res {
        // This branch is for both successful and unsuccessful command executions. Even
        // though the `res` is not `Err` it does not mean that the command ran
        // successfully, it may have failed gracefully.
        Ok(report) => {
            display_command_run_report(report).await;
        }
        // This branch is for strange errors like terminal not interactive.
        Err(error) => {
            report_unrecoverable_errors(error.into()).await;
        }
    }
}

/// Unknown and unrecoverable errors: `readline_async` or choose not working.
pub async fn report_unrecoverable_errors(report: miette::Report) {
    report_analytics::start_task_to_generate_event(
        String::new(),
        AnalyticsAction::GitiFailedToRun,
    );

    // % is Display, ? is Debug.
    tracing::error!(
        message = "Could not run giti due to the following problem",
        error = ?report
    );

    println!("{}", ui_str::unrecoverable_error_msg(report));
    upgrade_check::show_exit_message(ExitContext::Error).await;
}

/// Command ran and produced report: success, not success, fail, no-op.
pub async fn display_command_run_report(report: CommandRunReport<CommandRunDetails>) {
    println!("{report}");
    upgrade_check::show_exit_message(ExitContext::Normal).await;
}
