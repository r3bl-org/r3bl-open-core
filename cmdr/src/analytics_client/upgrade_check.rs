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
          io::{Error, stderr},
          process::{Command, ExitStatus, Stdio},
          sync::{Arc, atomic::AtomicBool},
          time::Duration};

use r3bl_tui::{DefaultIoDevices,
               HowToChoose,
               InlineString,
               ReadlineAsync,
               SpinnerStyle,
               StdMutex,
               StyleSheet,
               choose,
               height,
               inline_string,
               spinner::Spinner,
               try_get_latest_release_version_from_crates_io,
               width};
use tokio::task;

use super::ui_str;
use crate::DEBUG_ANALYTICS_CLIENT_MOD;

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
pub fn get_self_version() -> &'static str { env!("CARGO_PKG_VERSION") }

/// Returns the crate name from `Cargo.toml` (at compile time).
pub fn get_self_crate_name() -> &'static str { env!("CARGO_PKG_NAME") }

/// Get the filename of currently running executable (at run time).
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
        // 1. Show the “upgrade available” text.
        println!("{}", ui_str::upgrade_check::upgrade_is_required_msg());

        // 2. Ask the user.
        let options = &[
            super::ui_str::upgrade_check::yes_msg(),
            super::ui_str::upgrade_check::no_msg(),
        ];
        let mut io = DefaultIoDevices::default();
        let picked = choose(
            super::ui_str::upgrade_check::ask_user_msg(),
            options,
            Some(height(2)),
            Some(width(0)),
            HowToChoose::Single,
            StyleSheet::default(),
            io.as_mut_tuple(),
        )
        .await
        .ok()
        .and_then(|v| v.into_iter().next());

        // 3. If they chose “Yes, upgrade now”, run `cargo install …`.
        if let Some(choice) = picked
            && choice == options[0]
        {
            // With spinner.
            install_upgrade_command_with_spinner().await;

            // Without spinner.
            // install_without_spinner().await;
        }
    }

    // Print goodbye message.
    println!("{}", ui_str::goodbye_greetings::thanks_msg());
}

// XMARK: how to use long running potentially blocking synchronous code in Tokio, with spinner

/// Just like [install_upgrade_command_without_spinner] but **with** the spinner.
async fn install_upgrade_command_with_spinner() {
    let crate_name = get_self_crate_name();

    // 1) Create readline async.
    let res_readline_async = ReadlineAsync::try_new(
        /* does not matter what this is, as it isn't displayed */
        None::<&str>,
    )
    .await;

    let mut maybe_spinner: Option<Spinner> = None;

    // 2) Spawn spinner task.
    if let Ok(Some(readline_async)) = &res_readline_async {
        // Configure the spinner.
        let res = Spinner::try_start(
            ui_str::upgrade_spinner::indeterminate_progress_msg(),
            Duration::from_millis(100),
            SpinnerStyle::default(),
            Arc::new(StdMutex::new(stderr())),
            readline_async.clone_shared_writer(),
        )
        .await;

        if let Ok(Some(spinner)) = res {
            maybe_spinner = Some(spinner);
        }
    }

    // 3) Run the install process via spawn_blocking() so that it does not gum up the
    //    Tokio runtime for async tasks. `cargo install {crate_name}` is a long running,
    //    synchronous, and potentially blocking operation. This is run in a native thread
    //    from Tokio's thread pool, and not as a green thread.
    let blocking_task_join_handle = task::spawn_blocking(|| {
        // Run external process, and block current thread, until external process
        // finishes.
        Command::new("cargo")
            .args(["install", crate_name])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
    });
    // Asynchronously wait for the potentially blocking, long running, synchronous task
    // (on the **other** thread pool) to complete, by calling
    // `blocking_task_join_handle.await`. This does **not block** the current async worker
    // thread.
    let res_join_handle = blocking_task_join_handle.await;
    let res = match res_join_handle {
        // Task completed successfully (the command may have succeeded or failed).
        Ok(it) => it,
        // Task failed (panic or cancellation). Convert JoinError -> io::Error.
        Err(join_err) => {
            let err_msg = ui_str::upgrade_install::tokio_blocking_task_failed_msg(
                join_err.to_string(),
            );
            let io_error = Error::other(err_msg);
            Err(io_error)
        }
    };

    // 4) Stop the spinner & wait.
    if let Some(spinner) = maybe_spinner.as_mut()
        && !spinner.is_shutdown()
    {
        let _ = spinner.stop(ui_str::upgrade_spinner::stop_msg()).await;
    };

    // 5) Exit the readline_async.
    if let Ok(Some(readline_async)) = res_readline_async {
        _ = readline_async
            .exit(Some(&ui_str::upgrade_spinner::readline_async_exit_msg()))
            .await;
    }

    // 6) Report result.
    report_upgrade_install_result(res);
}

/// Just like [install_upgrade_command_with_spinner] but **without** the spinner.
#[allow(dead_code)]
async fn install_upgrade_command_without_spinner() {
    let crate_name = get_self_crate_name();
    let res = Command::new("cargo").args(["install", crate_name]).status();
    report_upgrade_install_result(res);
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
