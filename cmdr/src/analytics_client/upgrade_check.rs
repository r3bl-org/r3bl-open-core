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

use std::{io::stderr,
          os::unix::process::ExitStatusExt as _,
          process::{Command, ExitStatus, Stdio},
          sync::{Arc, atomic::AtomicBool},
          time::Duration};

use r3bl_tui::{DefaultIoDevices,
               HowToChoose,
               ReadlineAsync,
               SpinnerStyle,
               StdMutex,
               StyleSheet,
               choose,
               glyphs,
               height,
               spinner::Spinner,
               try_get_latest_release_version_from_crates_io,
               width};
use tokio::task;

use super::*;
use crate::DEBUG_ANALYTICS_CLIENT_MOD;

static UPDATE_REQUIRED: AtomicBool = AtomicBool::new(false);

/// To trigger an update message to be displayed to the user here are the details:
///
/// - The value of `GET_LATEST_VERSION_ENDPOINT` needs to be different,
/// - from the value of `UPDATE_IF_NOT_THIS_VERSION` in the `r3bl_base` repo.
pub fn start_task_to_check_for_updates() {
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
                UPDATE_REQUIRED.store(true, std::sync::atomic::Ordering::Relaxed);
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
pub fn get_self_bin_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_name()
                .and_then(|os_str| os_str.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn get_self_bin_emoji() -> String {
    let bin_name = get_self_bin_name();
    match bin_name.as_str() {
        "giti" => "🐱".to_string(),
        "edi" => "🦜".to_string(),
        _ => "👾".to_string(),
    }
}

pub fn is_update_required() -> bool {
    UPDATE_REQUIRED.load(std::sync::atomic::Ordering::Relaxed)
}

/// Prints any pending upgrade message, then asks the user if they'd like to install
/// the new version now.
pub async fn show_exit_message() {
    if upgrade_check::is_update_required() {
        let crate_name = get_self_crate_name();
        let upgrade_available_text = upgrade_check::ui_str::upgrade_required_message();

        // 1. Show the “upgrade available” text
        println!("{upgrade_available_text}\n");

        // 2. Ask the user
        let options = &[super::ui_str::upgrade_yes(), super::ui_str::upgrade_no()];
        let mut io = DefaultIoDevices::default();
        let picked = choose(
            super::ui_str::upgrade_available_message(crate_name),
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

        // 3. If they chose “Yes, upgrade now”, run `cargo install …`
        if let Some(choice) = picked
            && choice == options[0]
        {
            // With spinner.
            install_with_spinner().await;

            // Without spinner.
            // install_without_spinner().await;
        }
    }

    // Print goodbye message.
    let exit_msg = upgrade_check::ui_str::goodbye_thanks_for_using_message();
    println!("{exit_msg}");
}

/// Just like [install_without_spinner] but **with** the spinner.
async fn install_with_spinner() {
    let crate_name = upgrade_check::get_self_crate_name();

    // 1) Create readline async.
    let res_readline_async = ReadlineAsync::try_new(Some(format!(
        " {} cargo install {}... ",
        glyphs::PROMPT,
        crate_name
    )))
    .await;

    let mut maybe_spinner: Option<Spinner> = None;

    // 2) Spawn spinner task.
    if let Ok(Some(readline_async)) = &res_readline_async {
        // Configure the spinner.
        let res = Spinner::try_start(
            format!("Installing {crate_name}..."),
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

    // 3) Run the install in a blocking thread.
    let res_install_status = task::spawn_blocking(|| {
        Command::new("cargo")
            .args(["install", crate_name])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
    })
    .await
    .unwrap_or_else(|err| {
        eprintln!(" Failed to spawn cargo install {crate_name}: {err}");
        let it = ExitStatus::from_raw(1);
        Ok(it)
    });

    // 4) Stop the spinner & wait.
    if let Ok(Some(readline_async)) = res_readline_async {
        if let Some(mut spinner) = maybe_spinner
            && !spinner.is_shutdown()
        {
            let _ = spinner.stop(" Finished installation!").await;
        };
        let msg = format!("{crate_name} is installed 🎉.");
        _ = readline_async.exit(Some(&msg)).await;
    }

    // 5) Report result.
    match res_install_status {
        Ok(status) => {
            if status.success() {
                println!("\n ✅ Update installed successfully.");
            } else {
                eprintln!("\n ❌ Update failed (exit code {:?}).", status.code());
            }
        }
        Err(err) => {
            eprintln!(" Failed to run install: {err}");
        }
    }
}

/// Just like [install_with_spinner] but **without** the spinner.
#[allow(dead_code)]
async fn install_without_spinner() {
    let res = Command::new("cargo")
        .args(["install", "r3bl-cmdr"])
        .status();
    match res {
        Ok(status) => {
            if status.success() {
                println!(" Upgrade successful! 🎉");
            } else {
                println!(" Upgrade failed. Please try again.");
            }
        }
        Err(e) => {
            println!(" Error running cargo install: {e}");
        }
    }
}
