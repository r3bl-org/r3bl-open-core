// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Source compilation fallback engine using [`PtySessionBuilder`] and `OSC 9;4` progress
//! tracking.
//!
//! Provides the primary build entry point [`run_source_build_with_spinner`] and its
//! synchronous variant [`run_source_build_with_spinner_sync`].
//!
//! When `crate_distribution` is [`CrateDistribution::OpenSource`] and pre-compiled GitHub
//! Release binaries are unreachable, this module compiles the exact version pinned in
//! [`R3BL_VERSIONS_JSON_URL`] from `crates.io`:
//! 1. Optional toolchain update: `rustup toolchain install nightly --force`
//! 2. Pinned crate installation: `cargo +nightly install <crate_name> --version
//!    <pinned_version> --force`
//!
//! Underlying logic is encapsulated into inner modules:
//! - [`toolchain_setup`]: Running `rustup` toolchain installation and parsing terminal
//!   progress.
//! - [`cargo_installer`]: Executing `cargo install` inside a [`PTY`] with [`OSC`]
//!   progress capture.
//!
//! [`CrateDistribution::OpenSource`]: crate::core::script::self_update::CrateDistribution::OpenSource
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::self_update::R3BL_VERSIONS_JSON_URL

use super::{super::{rustup_progress, toolchain_bin, toolchain_cli_args}, ui_strings};
use crate::{OscPtyEvent, OutputDevice, Spinner, SpinnerStyle, TuiAvailability,
            core::pty::{DefaultPtySessionConfig, PtyOutputEvent, PtySessionBuilder,
                        PtySessionConfigToken}};
use std::{io::{Error, ErrorKind},
          process::ExitStatus,
          time::Duration};
use tokio::signal;

// -------------------------------------------------------------------------------------------------
// Top-level Entry Points (Runtime Call-Site Adapters)
// -------------------------------------------------------------------------------------------------

/// Runs the full source build workflow (`rustup` optional + pinned `cargo +nightly
/// install`) with a live [`Spinner`] and `Ctrl+C` cancellation support.
///
/// # Errors
///
/// Returns an [`std::io::Error`] if either `rustup` or `cargo` fails to spawn or exits
/// with an error.
pub async fn run_source_build_with_spinner(
    crate_name: &str,
    pinned_version: &str,
    use_rustup: bool,
    output_device: OutputDevice,
) -> Result<ExitStatus, Error> {
    let initial_msg = if use_rustup {
        ui_strings::rustup_update_initial_msg().to_string()
    } else {
        ui_strings::format_cargo_installing_msg(crate_name, pinned_version)
    };

    let res_spinner = Spinner::try_start(
        initial_msg,
        ui_strings::installation_ended_msg(),
        Duration::from_millis(100),
        SpinnerStyle::default(),
        output_device,
        None,
    )
    .await;

    let mut maybe_spinner = match res_spinner {
        TuiAvailability::Available(spinner) => Some(spinner),
        _ => None,
    };

    if use_rustup {
        let rustup_result =
            toolchain_setup::run_rustup_update(maybe_spinner.as_ref()).await;
        match rustup_result {
            Ok(status) if status.success() => {}
            Ok(status) => {
                if let Some(mut spinner) = maybe_spinner.take() {
                    spinner.request_shutdown();
                    spinner.await_shutdown().await;
                }
                return Ok(status);
            }
            Err(e) => {
                if let Some(mut spinner) = maybe_spinner.take() {
                    spinner.request_shutdown();
                    spinner.await_shutdown().await;
                }
                return Err(e);
            }
        }
    }

    if let Some(ref spinner) = maybe_spinner {
        spinner.update_message(ui_strings::format_cargo_installing_msg(
            crate_name,
            pinned_version,
        ));
    }

    let install_result = cargo_installer::run_cargo_install_with_progress(
        crate_name,
        pinned_version,
        maybe_spinner.as_ref(),
    )
    .await;

    if let Some(mut spinner) = maybe_spinner.take() {
        spinner.request_shutdown();
        spinner.await_shutdown().await;
    }

    install_result
}

/// Synchronous variant of [`run_source_build_with_spinner`].
///
/// Builds a private `current_thread` Tokio runtime to drive the build workflow.
///
/// # Errors
///
/// Returns an [`std::io::Error`] if either `rustup` or `cargo` fails to spawn or exits
/// with an error.
pub fn run_source_build_with_spinner_sync(
    crate_name: &str,
    pinned_version: &str,
    use_rustup: bool,
    output_device: OutputDevice,
) -> Result<ExitStatus, Error> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(Error::other)?;
    rt.block_on(run_source_build_with_spinner(
        crate_name,
        pinned_version,
        use_rustup,
        output_device,
    ))
}

// -------------------------------------------------------------------------------------------------
// Inner Modules (Underlying Implementation & Domain Logic)
// -------------------------------------------------------------------------------------------------

/// Rustup toolchain installation and output monitoring.
pub mod toolchain_setup {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Extracts meaningful progress information from `rustup` output.
    ///
    /// Looks for patterns that start with [`info: `], such as:
    /// - `info: syncing channel updates for 'nightly-x86_64-unknown-linux-gnu'`
    /// - `info: downloading component 'rust-std'`
    /// - `info: installing component 'cargo'`
    ///
    /// [`info: `]: rustup_progress::INFO_PREFIX
    #[must_use]
    pub fn extract_rustup_progress(output: &str) -> String {
        use rustup_progress::{ELLIPSIS, INFO_PREFIX, MAX_DISPLAY_LEN, TRUNCATE_LEN};

        let lines: Vec<&str> = output
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();

        if let Some(last_line) = lines.last() {
            let trimmed = last_line.trim();
            let cleaned = trimmed.strip_prefix(INFO_PREFIX).unwrap_or(trimmed);

            if cleaned.len() > MAX_DISPLAY_LEN {
                format!("{}{ELLIPSIS}", &cleaned[..TRUNCATE_LEN])
            } else {
                cleaned.to_string()
            }
        } else {
            String::new()
        }
    }

    /// Runs `rustup toolchain install nightly --force` in a [`PTY`] session, capturing
    /// output lines to update the spinner and listening for cooperative `Ctrl+C`
    /// cancellation.
    ///
    /// # Errors
    ///
    /// Returns an [`std::io::Error`] if the [`PTY`] session fails to start, terminates
    /// unexpectedly, or is cancelled by the user via `Ctrl+C`
    /// ([`ErrorKind::Interrupted`]).
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub async fn run_rustup_update(
        spinner: Option<&Spinner>,
    ) -> Result<ExitStatus, Error> {
        let mut session = PtySessionBuilder::new(toolchain_bin::RUSTUP)
            .cli_args([
                toolchain_cli_args::TOOLCHAIN,
                toolchain_cli_args::INSTALL,
                toolchain_cli_args::NIGHTLY_TOOLCHAIN,
                toolchain_cli_args::FORCE,
            ])
            .start_async()
            .map_err(Error::other)?;

        loop {
            tokio::select! {
                _ = signal::ctrl_c() => {
                    return Err(Error::new(ErrorKind::Interrupted, "Update cancelled by user"));
                }
                event = session.rx_output_event.recv() => {
                    match event {
                        Some(PtyOutputEvent::Output(data)) => {
                            if let Ok(text) = std::str::from_utf8(&data) {
                                let progress_info = extract_rustup_progress(text);
                                if let Some(spinner) = spinner
                                    && !progress_info.is_empty()
                                {
                                    spinner.update_message(
                                        ui_strings::format_rustup_progress_msg(&progress_info),
                                    );
                                }
                            }
                        }
                        Some(PtyOutputEvent::Exit(status)) => {
                            return Ok(status.into());
                        }
                        None => {
                            return Err(Error::other("PTY session ended unexpectedly"));
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

/// Cargo compilation invocation and [`OSC`] 9;4 progress event processing.
///
/// [`OSC`]: crate::osc_codes::OscSequence
pub mod cargo_installer {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Formats the CLI arguments for `cargo +nightly install` pinned to `pinned_version`.
    #[must_use]
    pub fn format_pinned_cargo_install_args<'a>(
        crate_name: &'a str,
        pinned_version: &'a str,
    ) -> [&'a str; 6] {
        [
            toolchain_cli_args::NIGHTLY_FLAG,
            toolchain_cli_args::INSTALL,
            crate_name,
            toolchain_cli_args::VERSION,
            pinned_version.trim_start_matches('v'),
            toolchain_cli_args::FORCE,
        ]
    }

    /// Runs `cargo +nightly install <crate_name> --version <pinned_version> --force` in a
    /// [`PTY`] with [`OSC`] capture enabled, reporting real-time compilation percentage
    /// to the spinner.
    ///
    /// # Errors
    ///
    /// Returns an [`std::io::Error`] if the [`PTY`] session fails to start, terminates
    /// unexpectedly, or is cancelled by the user via `Ctrl+C`
    /// ([`ErrorKind::Interrupted`]).
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub async fn run_cargo_install_with_progress(
        crate_name: &str,
        pinned_version: &str,
        spinner: Option<&Spinner>,
    ) -> Result<ExitStatus, Error> {
        let args = format_pinned_cargo_install_args(crate_name, pinned_version);
        let mut session = PtySessionBuilder::new(toolchain_bin::CARGO)
            .cli_args(args)
            .with_config(DefaultPtySessionConfig + PtySessionConfigToken::CaptureOsc)
            .start_async()
            .map_err(Error::other)?;

        loop {
            tokio::select! {
                _ = signal::ctrl_c() => {
                    return Err(Error::new(
                        ErrorKind::Interrupted,
                        "Installation cancelled by user",
                    ));
                }
                event = session.rx_output_event.recv() => {
                    match event {
                        Some(PtyOutputEvent::Osc(osc)) => {
                            handle_osc_event(osc, crate_name, pinned_version, spinner);
                        }
                        Some(PtyOutputEvent::Exit(status)) => {
                            return Ok(status.into());
                        }
                        None => {
                            return Err(Error::other("PTY session ended unexpectedly"));
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Handles [`OSC`] 9;4 progress events emitted by Cargo and updates the spinner
    /// message.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    pub fn handle_osc_event(
        event: OscPtyEvent,
        crate_name: &str,
        pinned_version: &str,
        spinner: Option<&Spinner>,
    ) {
        if let Some(spinner) = spinner {
            match event {
                OscPtyEvent::ProgressUpdate(percentage) => {
                    spinner.update_message(ui_strings::format_cargo_progress_msg(
                        crate_name,
                        pinned_version,
                        percentage,
                    ));
                }
                OscPtyEvent::IndeterminateProgress => {
                    spinner.update_message(ui_strings::format_cargo_building_msg(
                        crate_name,
                        pinned_version,
                    ));
                }
                OscPtyEvent::ProgressCleared => {
                    spinner.update_message(ui_strings::format_cargo_installing_msg(
                        crate_name,
                        pinned_version,
                    ));
                }
                OscPtyEvent::BuildError => {
                    spinner.update_message(ui_strings::format_cargo_build_error_msg(
                        crate_name,
                        pinned_version,
                    ));
                }
                OscPtyEvent::Hyperlink { .. } | OscPtyEvent::SetTitleAndTab(_) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_extract_rustup_progress_empty() {
        assert_eq2!(toolchain_setup::extract_rustup_progress(""), "");
        assert_eq2!(toolchain_setup::extract_rustup_progress("   \n\n  "), "");
    }

    #[test]
    fn test_extract_rustup_progress_with_info_prefix() {
        let output = "info: downloading component 'rust-std'";
        assert_eq2!(
            toolchain_setup::extract_rustup_progress(output),
            "downloading component 'rust-std'"
        );
    }

    #[test]
    fn test_extract_rustup_progress_without_info_prefix() {
        let output = "syncing channel updates";
        assert_eq2!(
            toolchain_setup::extract_rustup_progress(output),
            "syncing channel updates"
        );
    }

    #[test]
    fn test_extract_rustup_progress_multiline() {
        let output =
            "info: syncing channel updates\ninfo: downloading component 'cargo'\n\n";
        assert_eq2!(
            toolchain_setup::extract_rustup_progress(output),
            "downloading component 'cargo'"
        );
    }

    #[test]
    fn test_extract_rustup_progress_truncation() {
        let long_line = "info: syncing channel updates for 'nightly-x86_64-unknown-linux-gnu' and extra long text";
        let result = toolchain_setup::extract_rustup_progress(long_line);
        assert_eq2!(result.len(), rustup_progress::MAX_DISPLAY_LEN);
        assert!(result.ends_with(rustup_progress::ELLIPSIS));
    }

    #[test]
    fn test_format_pinned_cargo_install_args() {
        let args =
            cargo_installer::format_pinned_cargo_install_args("r3bl-cmdr", "0.0.25");
        assert_eq2!(
            args,
            [
                "+nightly",
                "install",
                "r3bl-cmdr",
                "--version",
                "0.0.25",
                "--force"
            ]
        );

        let args_with_v =
            cargo_installer::format_pinned_cargo_install_args("r3bl-cmdr", "v0.0.25");
        assert_eq2!(
            args_with_v,
            [
                "+nightly",
                "install",
                "r3bl-cmdr",
                "--version",
                "0.0.25",
                "--force"
            ]
        );
    }
}
