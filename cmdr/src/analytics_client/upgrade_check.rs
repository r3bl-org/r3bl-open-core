// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Upgrade checking and installation functionality for r3bl-cmdr.
//!
//! This module handles checking for newer versions of the crate on crates.io and manages
//! the upgrade process with progress display and user interaction.
//!
//! # Upgrade Process Details
//!
//! The upgrade process uses two main commands to ensure reliable installation with
//! progress display:
//!
//! ## 1. Rust Toolchain Update: `rustup toolchain install nightly --force`
//! - **What it does**: Downloads and installs the latest nightly Rust toolchain
//! - **Why `--force`**: Forces reinstallation even if nightly is already installed,
//!   guaranteeing progress output
//! - **Progress display**: Shows real-time download/installation messages (e.g.,
//!   "downloading component 'rust-std'")
//! - **Benefits**: Ensures the latest Rust features and bug fixes are available for
//!   compilation
//!
//! ## 2. Crate Installation: `cargo +nightly install r3bl-cmdr`
//! - **What it does**: Downloads, compiles, and installs the latest version of r3bl-cmdr
//!   from crates.io
//! - **Why `+nightly`**: Explicitly uses the nightly toolchain for compilation, ensuring
//!   consistency
//! - **Progress display**: Emits OSC escape sequences showing compilation percentage
//!   (0-100%)
//! - **Benefits**: Uses the latest nightly features for optimal performance and newest
//!   language capabilities
//!
//! ## Testing the Progress Display
//!
//! To see the upgrade progress in action:
//! 1. Run [`../remove_toolchains.sh`] to remove existing toolchains (optional, for
//!    testing)
//! 2. Run `cargo run --bin edi` or `cargo run --bin giti`
//! 3. When you exit, if an upgrade is available, you'll see:
//!    - Spinner with real-time rustup installation messages (using output)
//!    - Progress percentages during cargo compilation (using OSC)
//!    - Both processes can be cancelled with Ctrl+C
//!
//! [`../remove_toolchains.sh`]: https://github.com/r3bl-org/r3bl-open-core/blob/main/remove_toolchains.sh

use super::ui_str;
use crate::{DEBUG_ANALYTICS_CLIENT_MOD, prefix_single_select_instruction_header};
use r3bl_tui::{DefaultIoDevices, HowToChoose, InlineString, OscEvent, OutputDevice,
               SpinnerStyle, StyleSheet, choose, cli_text, cli_text_line,
               core::pty::{PtyCommandBuilder, PtyConfigOption, PtyReadOnlyOutputEvent,
                           pty_to_std_exit_status},
               height, inline_string,
               spinner::Spinner,
               try_get_latest_release_version_from_crates_io};
use smallvec::smallvec;
use std::{env::current_exe,
          io::{Error, ErrorKind},
          process::ExitStatus,
          sync::atomic::AtomicBool,
          time::Duration};
use tokio::signal;

pub static UPGRADE_REQUIRED: AtomicBool = AtomicBool::new(false);
/// Context for the exit message to determine what to display.
#[derive(Debug, Clone, Copy)]
pub enum ExitContext {
    /// Normal exit - show simple goodbye message
    Normal,
    /// Error exit - show full message with GitHub link
    Error,
    /// Help command - show full message with GitHub link
    Help,
}

/// Checks if a newer version of the crate is available on crates.io.
///
/// If a newer version is found, sets the [`UPGRADE_REQUIRED`] flag which will prompt the
/// user to upgrade when they exit the application.
///
/// This function spawns an async task that runs in the background, so it returns
/// immediately without blocking.
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
/// When adding new binaries make sure to update this function to return the
/// correct emoji for the new binary.
#[must_use]
pub fn get_self_bin_emoji() -> String {
    match get_self_bin_name().as_str() {
        "giti" => "🐱".to_string(),
        "edi" => "🦜".to_string(),
        "ch" => "🔮".to_string(),
        "rc" => "🐒".to_string(),
        _ => "👾".to_string(),
    }
}

pub fn is_upgrade_required() -> bool {
    UPGRADE_REQUIRED.load(std::sync::atomic::Ordering::Relaxed)
}

/// Prints any pending upgrade message, then asks the user if they'd like to install
/// the new version now.
pub async fn show_exit_message(context: ExitContext) {
    if is_upgrade_required() {
        // Show the "upgrade available" text.
        println!("{}", ui_str::upgrade_check::upgrade_is_required_msg());

        // Ask the user.
        let yes_no_options = &[
            ui_str::upgrade_check::yes_msg_raw(),
            ui_str::upgrade_check::no_msg_raw(),
        ];
        let header_with_instructions = {
            let last_line = cli_text_line![cli_text(
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

        // If they chose "Yes, upgrade now", run `cargo install …`.
        if let Some(user_choice) = maybe_user_choice
            && user_choice == yes_no_options[0]
        {
            install_upgrade_command_with_spinner_and_ctrl_c().await;
        }
    }

    // Print goodbye message based on context.
    match context {
        ExitContext::Normal => {
            println!("{}", ui_str::goodbye_greetings::thanks_msg_simple());
        }
        ExitContext::Error | ExitContext::Help => {
            println!("{}", ui_str::goodbye_greetings::thanks_msg_with_github());
        }
    }
}

// XMARK: how to use long running potentially blocking synchronous code in Tokio, with
// spinner.

/// Extract meaningful progress information from rustup output.
///
/// Looks for patterns like:
/// - "Updating to 1.75.0"
/// - "Downloading component 'rust-std'"
/// - "Installing component 'cargo'"
///
/// # Returns
///
/// The last meaningful line, truncated if too long for spinner display.
fn extract_rustup_progress(output: &str) -> String {
    let lines: Vec<&str> = output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();

    if let Some(last_line) = lines.last() {
        let trimmed = last_line.trim();
        // Remove any common prefixes that are not informative.
        let cleaned = trimmed.strip_prefix("info: ").unwrap_or(trimmed);

        // Truncate if too long for spinner display.
        if cleaned.len() > 50 {
            format!("{}...", &cleaned[..47])
        } else {
            cleaned.to_string()
        }
    } else {
        String::new()
    }
}

/// Run rustup update with PTY support, output capture, and Ctrl+C handling.
///
/// Unlike cargo install, rustup doesn't emit OSC codes, but it does produce output that
/// can be used to show progress. This function captures that output and updates the
/// spinner message with meaningful progress information.
async fn run_rustup_update(spinner: Option<&Spinner>) -> Result<ExitStatus, Error> {
    // Use Output mode to capture rustup's output for progress display.
    let mut session = PtyCommandBuilder::new("rustup")
        .args(["toolchain", "install", "nightly", "--force"])
        .spawn_read_only(PtyConfigOption::Output)
        .map_err(Error::other)?;

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                // User pressed Ctrl+C.
                return Err(Error::new(ErrorKind::Interrupted, "Update cancelled by user"));
            }
            event = session.output_evt_ch_rx_half.recv() => {
                match event {
                    Some(PtyReadOnlyOutputEvent::Output(data)) => {
                        // Convert bytes to string and extract meaningful info.
                        if let Ok(text) = std::str::from_utf8(&data) {
                            let progress_info = extract_rustup_progress(text);
                            if let Some(spinner) = spinner
                                && !progress_info.is_empty() {
                                    spinner.update_message(format!("Updating Rust toolchain... {progress_info}"));
                                }
                        }
                    }
                    Some(PtyReadOnlyOutputEvent::Exit(status)) => {
                        return Ok(pty_to_std_exit_status(status));
                    }
                    None => {
                        // Channel closed unexpectedly.
                        return Err(Error::other("PTY session ended unexpectedly"));
                    }
                    _ => {} // Ignore other events
                }
            }
        }
    }
}

async fn run_cargo_install_with_progress(
    crate_name: &str,
    spinner: Option<&Spinner>,
) -> Result<ExitStatus, Error> {
    // Use Osc mode to capture OSC progress sequences from cargo.
    let mut session = PtyCommandBuilder::new("cargo")
        .args(["+nightly", "install", crate_name])
        .spawn_read_only(PtyConfigOption::Osc)
        .map_err(Error::other)?;

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                // User pressed Ctrl+C.
                return Err(Error::new(ErrorKind::Interrupted,
                    "Installation cancelled by user"));
            }
            event = session.output_evt_ch_rx_half.recv() => {
                match event {
                    Some(PtyReadOnlyOutputEvent::Osc(osc)) => {
                        handle_osc_event(osc, crate_name, spinner);
                    }
                    Some(PtyReadOnlyOutputEvent::Exit(status)) => {
                        return Ok(pty_to_std_exit_status(status));
                    }
                    None => {
                        // Channel closed unexpectedly.
                        return Err(Error::other("PTY session ended unexpectedly"));
                    }
                    _ => {} // Ignore Output events in Osc mode
                }
            }
        }
    }
}

/// Handle OSC events from cargo install and update the spinner message accordingly.
fn handle_osc_event(event: OscEvent, crate_name: &str, spinner: Option<&Spinner>) {
    if let Some(spinner) = spinner {
        match event {
            OscEvent::ProgressUpdate(percentage) => {
                spinner
                    .update_message(format!("Installing {crate_name}... {percentage}%"));
            }
            OscEvent::IndeterminateProgress => {
                spinner.update_message(format!("Installing {crate_name}... (building)"));
            }
            OscEvent::ProgressCleared => {
                spinner.update_message(format!("Installing {crate_name}..."));
            }
            OscEvent::BuildError => {
                spinner.update_message(format!(
                    "Installing {crate_name}... (error occurred)"
                ));
            }
            OscEvent::Hyperlink { .. } | OscEvent::SetTitleAndTab(_) => {
                // Hyperlinks and title/tab events aren't relevant for cargo install
                // progress, so we ignore them here.
            }
        }
    }
}

async fn install_upgrade_command_with_spinner_and_ctrl_c() {
    let crate_name = get_self_crate_name();

    // Setup spinner with initial message for rustup update.
    let mut maybe_spinner = if let Ok(Some(spinner)) = Spinner::try_start(
        "Updating Rust toolchain...", // Initial message for rustup
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

    // First: Run rustup update (spinner shows with output-based progress).
    let rustup_result = run_rustup_update(maybe_spinner.as_ref()).await;
    if let Err(e) = rustup_result {
        // Handle error, stop spinner.
        if let Some(mut spinner) = maybe_spinner.take() {
            spinner.request_shutdown();
            spinner.await_shutdown().await;
        }
        report_upgrade_install_result(Err(e));
        return;
    }

    // Update spinner message for cargo install.
    if let Some(ref spinner) = maybe_spinner {
        spinner.update_message(format!("Installing {crate_name}..."));
    }

    // Second: Run cargo install with OSC progress tracking.
    let install_result =
        run_cargo_install_with_progress(crate_name, maybe_spinner.as_ref()).await;

    // Stop spinner.
    if let Some(mut spinner) = maybe_spinner.take() {
        spinner.request_shutdown();
        spinner.await_shutdown().await;
    }

    // Report result.
    report_upgrade_install_result(install_result);
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
