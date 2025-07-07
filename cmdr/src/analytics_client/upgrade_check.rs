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
use std::{env::current_exe,
          io::{Error, ErrorKind},
          process::{ExitStatus, Stdio},
          sync::atomic::AtomicBool,
          time::Duration};

use r3bl_tui::{DefaultIoDevices, HowToChoose, InlineString, OutputDevice, SpinnerStyle,
               StyleSheet, ast, ast_line, choose, command, height, inline_string,
               script::command_impl::TokioCommand, spinner::Spinner,
               try_get_latest_release_version_from_crates_io};
use smallvec::smallvec;
use tokio::signal;

use super::ui_str;
use crate::{DEBUG_ANALYTICS_CLIENT_MOD, prefix_single_select_instruction_header};

static UPGRADE_REQUIRED: AtomicBool = AtomicBool::new(false);

/// To trigger an update message to be displayed to the user here are the details:
///
/// - The value of `GET_LATEST_VERSION_ENDPOINT` needs to be different,
/// - from the value of `UPDATE_IF_NOT_THIS_VERSION` in the `r3bl_base` repo.
pub fn start_task_to_check_if_upgrade_is_needed() {
    tokio::spawn(async move {
        let version_self = get_self_version();
        if let Ok(version_crates) =
            try_get_latest_release_version_from_crates_io(get_self_crate_name()).await
        {
            DEBUG_ANALYTICS_CLIENT_MOD.then(|| {
                // % is Display, ? is Debug.
                tracing::info!(
                    message = "📦📦📦 Latest version of cmdr",
                    version = %version_crates
                );
            });

            if version_self != version_crates {
                UPGRADE_REQUIRED.store(true, std::sync::atomic::Ordering::Relaxed);
                DEBUG_ANALYTICS_CLIENT_MOD.then(|| {
                    // % is Display, ? is Debug.
                    tracing::info!(
                        message = "💿💿💿 There is a new version of cmdr available",
                        version = %version_crates
                    );
                });
            }
        }
    });
}

/// Gets version number from `Cargo.toml` (at compile time).
#[must_use]
pub fn get_self_version() -> &'static str { env!("CARGO_PKG_VERSION") }

/// Returns the crate name from `Cargo.toml` (at compile time).
#[must_use]
pub fn get_self_crate_name() -> &'static str { env!("CARGO_PKG_NAME") }

/// Get the filename of currently running executable (at run time).
#[must_use]
pub fn get_self_bin_name() -> InlineString {
    current_exe()
        .ok()
        .and_then(|path_buf| {
            path_buf
                .file_name()?
                .to_str()
                .map(|f_name| inline_string!("{f_name}"))
        })
        .unwrap_or_else(|| inline_string!("unknown"))
}

/// Get the emoji representing the currently running executable (at run time).
#[must_use]
pub fn get_self_bin_emoji() -> String {
    match get_self_bin_name().as_str() {
        "giti" => "🐱".to_string(),
        "edi" => "🦜".to_string(),
        _ => "👾".to_string(),
    }
}

pub fn is_upgrade_required() -> bool {
    UPGRADE_REQUIRED.load(std::sync::atomic::Ordering::Relaxed)
}

/// Prints any pending upgrade message, then asks the user if they'd like to install
/// the new version now.
pub async fn show_exit_message() {
    if is_upgrade_required() {
        // Show the “upgrade available” text.
        println!("{}", ui_str::upgrade_check::upgrade_is_required_msg());

        // Ask the user.
        let yes_no_options = &[
            ui_str::upgrade_check::yes_msg_raw(),
            ui_str::upgrade_check::no_msg_raw(),
        ];
        let header_with_instructions = {
            let last_line = ast_line![ast(
                ui_str::upgrade_check::ask_user_msg_raw(),
                crate::common::ui_templates::header_style_default()
            )];
            prefix_single_select_instruction_header(smallvec![last_line])
        };
        let mut io = DefaultIoDevices::default();

        // Get the first item selected by the user.
        let maybe_user_choice = choose(
            header_with_instructions,
            yes_no_options,
            Some(height(yes_no_options.len())),
            None,
            HowToChoose::Single,
            StyleSheet::default(),
            io.as_mut_tuple(),
        )
        .await
        .ok()
        .and_then(|items| items.into_iter().next());

        // If they chose “Yes, upgrade now”, run `cargo install …`.
        if let Some(user_choice) = maybe_user_choice
            && user_choice == yes_no_options[0]
        {
            install_upgrade_command_with_spinner_and_ctrl_c().await;
        }
    }

    // Print goodbye message.
    println!("{}", ui_str::goodbye_greetings::thanks_msg());
}

// XMARK: how to use long running potentially blocking synchronous code in Tokio, with
// spinner

async fn install_upgrade_command_with_spinner_and_ctrl_c() {
    let crate_name = get_self_crate_name();

    // Setup spinner.
    let mut maybe_spinner = if let Ok(Some(spinner)) = Spinner::try_start(
        ui_str::upgrade_install::indeterminate_progress_msg_raw(),
        ui_str::upgrade_install::stop_msg(),
        Duration::from_millis(100),
        SpinnerStyle::default(),
        OutputDevice::default(),
        None,
    )
    .await
    {
        Some(spinner)
    } else {
        None
    };

    // Spawn the command asynchronously.
    let mut cmd = {
        let mut it: TokioCommand = command!(
            program => "cargo",
            args => "install", crate_name
        );
        it.kill_on_drop(true);
        it.stdin(Stdio::null());
        it.stdout(Stdio::piped());
        it.stderr(Stdio::piped());
        it
    };

    let process_status_result = match cmd.spawn() {
        Ok(mut child) => {
            tokio::select! {
                // [Branch]: Wait for Ctrl+C signal.
                _ = signal::ctrl_c() => {
                    // Stop the spinner (if running).
                    if let Some(mut spinner) = maybe_spinner.take()
                        && !spinner.is_shutdown() {
                            spinner.request_shutdown();
                            spinner.await_shutdown().await;
                        }

                    // Try to kill the process, with start_kill() which is non-blocking.
                    match child.start_kill() {
                        Ok(()) => {
                            println!("{}", ui_str::upgrade_install::send_sigint_msg());
                            // We don't care about the result of this operation.
                            child.wait().await.ok();
                        }
                        Err(e) => {
                            eprintln!("{}", ui_str::upgrade_install::fail_send_sigint_msg(e));
                        }
                    }

                    // Return an error indicating cancellation.
                    Err(Error::new(
                        ErrorKind::Interrupted, "Installation cancelled by user",
                    ))
                }
                // [Branch]: Wait for the process to complete.
                status_result = child.wait() => {
                    // Stop the spinner (if running).
                    if let Some(mut spinner) = maybe_spinner.take()
                        && !spinner.is_shutdown() {
                            spinner.request_shutdown();
                            spinner.await_shutdown().await;
                        }

                    // Return the process request_shutdown status.
                    status_result
                }
            }
        }
        Err(e) => {
            // Return the spawn error.
            Err(e)
        }
    };

    // Report the final result (success, failure, cancellation).
    report_upgrade_install_result(process_status_result);
}

/// Reports the result of the installation process.
fn report_upgrade_install_result(res: Result<ExitStatus, Error>) {
    match res {
        Ok(status) => {
            if status.success() {
                println!("{}", ui_str::upgrade_install::install_success_msg());
            } else {
                eprintln!(
                    "{}",
                    ui_str::upgrade_install::install_not_success_msg(status)
                );
            }
        }
        Err(err) => {
            eprintln!(
                "{}",
                ui_str::upgrade_install::install_failed_to_run_command_msg(err)
            );
        }
    }
}
