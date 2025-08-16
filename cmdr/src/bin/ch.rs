// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! For more information on how to use CLAP, here are some resources:
//! 1. [Tutorial](https://developerlife.com/2023/09/17/tuify-clap/)
//! 2. [Video](https://youtu.be/lzMYDA6St0s)

use clap::Parser;
use r3bl_cmdr::{AnalyticsAction,
                ch::{CLIArg, ChResult},
                report_analytics,
                upgrade_check::{self, ExitContext}};
use r3bl_tui::{CommonResult, log::try_initialize_logging_global, ok,
               run_with_safe_stack, set_mimalloc_in_main};

fn main() -> CommonResult<()> { run_with_safe_stack!(main_impl()) }

#[tokio::main]
#[allow(clippy::needless_return)]
async fn main_impl() -> CommonResult<()> {
    set_mimalloc_in_main!();

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

    upgrade_check::start_task_to_check_if_upgrade_is_needed();
    report_analytics::start_task_to_generate_event(
        String::new(),
        AnalyticsAction::ChAppStart,
    );

    launch_ch(cli_arg).await;

    should_log.then(|| {
        tracing::debug!(message = "Stop logging...");
    });

    ok!()
}

pub async fn launch_ch(cli_arg: CLIArg) {
    // Execute the ch command.
    let res = r3bl_cmdr::ch::handle_ch_command(cli_arg).await;

    // Handle the result of the command execution.
    match res {
        // This branch is for both successful and unsuccessful command executions. Even
        // though the `res` is not `Err` it does not mean that the command ran
        // successfully, it may have failed gracefully.
        Ok(ch_result) => {
            display_ch_result(ch_result).await;
        }
        // This branch is for strange errors like terminal not interactive.
        Err(error) => {
            report_unrecoverable_errors(error).await;
        }
    }
}

/// Unknown and unrecoverable errors: `readline_async` or choose not working.
pub async fn report_unrecoverable_errors(report: miette::Report) {
    report_analytics::start_task_to_generate_event(
        String::new(),
        AnalyticsAction::ChFailedToRun,
    );

    // % is Display, ? is Debug.
    tracing::error!(
        message = "Could not run ch due to the following problem",
        error = ?report
    );

    println!("{}", r3bl_cmdr::ch::ui_str::unrecoverable_error_msg(report));
    upgrade_check::show_exit_message(ExitContext::Error).await;
}

/// Display the result of ch command execution.
pub async fn display_ch_result(ch_result: ChResult) {
    println!("{ch_result}");
    upgrade_check::show_exit_message(ExitContext::Normal).await;
}
