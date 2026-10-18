// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::{CrateDistribution, CrateMetadata, CrateName, CrateVersion,
                    EndpointUrl, UpgradeConfig, archive, upgrade_reasons},
            ReleaseAsset, ReleaseTarget, Sha256Digest, UpgradeSource,
            create_output_device_for_feedback_channel, download_and_verify_archive,
            extract_and_apply_archive, get_current_bin_name,
            report_upgrade_source_error, run_source_build_with_spinner, ui_strings};
use r3bl_tui::{OutputDevice, Spinner, SpinnerStyle, TuiAvailability, try_create_temp_dir};
use std::{io, time::Duration};

/// Executes the resolved upgrade flow for `config`:
/// 1. Resolves [`UpgradeSource`] via [`UpgradeSource::try_resolve`].
/// 2. If [`UpgradeSource::ReleaseArchive`], streams the binary archive, verifies the
///    SHA-256 checksum, and swaps via `self_replace`. If the download fails mid-stream
///    and [`CrateDistribution::OpenSource`], automatically transitions to the `crates.io`
///    source compilation fallback (`--version <pinned_ver>`).
/// 3. If [`UpgradeSource::BuildFromSourceRustup`], runs the [`PTY`]/[`OSC`] source build
///    pinned to `--version <pinned_ver>`.
///
/// This function is called by the [`Upgrading`] typestate in the [Control Plane] (either
/// during an immediate upgrade via [`UpgradeJourney::start`] or during a deferred
/// upgrade on exit via [`StartOutcome::handle_upgrade_on_exit`]) to resolve the
/// transition into [`Completed`]. This is part of the [Execution Plane]: it never
/// imports or touches [`UpgradeJourney`] or [`SINGLETON`].
///
/// # Errors
///
/// Returns [`UpgradeError`] if all viable upgrade paths fail or are canceled.
///
/// [`Completed`]: crate::self_upgrade::Completed
/// [`OSC`]: r3bl_tui::osc_codes::OscSequence
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`SINGLETON`]: crate::self_upgrade::SINGLETON
/// [`StartOutcome::handle_upgrade_on_exit`]: crate::self_upgrade::StartOutcome::handle_upgrade_on_exit
/// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
/// [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
/// [`Upgrading`]: crate::self_upgrade::Upgrading
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [Execution Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
pub async fn perform_upgrade(
    config: &UpgradeConfig,
    metadata: &CrateMetadata,
    binaries_release_base_url: &EndpointUrl,
) -> Result<(), UpgradeError> {
    let bin_name = get_current_bin_name();
    let upgrade_source = match UpgradeSource::try_resolve(
        config.get_crate_name(),
        metadata.version,
        metadata.crate_distribution,
        binaries_release_base_url,
    )
    .await
    {
        Ok(upgrade_source) => upgrade_source,
        Err(err) => {
            report_upgrade_source_error(
                config,
                metadata.crate_distribution,
                &bin_name,
                &err,
            );
            return Err(err);
        }
    };

    let output_device =
        create_output_device_for_feedback_channel(config.get_user_feedback_channel());

    match upgrade_source {
        UpgradeSource::ReleaseArchive {
            archive_url,
            sha256_url,
            pinned_version,
        } => {
            execute_release_archive_or_fallback(
                config,
                metadata.crate_distribution,
                &archive_url,
                sha256_url.as_str(),
                pinned_version,
                output_device,
                &bin_name,
            )
            .await
        }
        UpgradeSource::BuildFromSourceRustup { pinned_version } => {
            let pinned_version_str = pinned_version.to_string();
            config.get_user_feedback_channel().println(
                ui_strings::format_fallback_to_source_notice(
                    config.get_crate_name(),
                    &pinned_version_str,
                ),
            );
            execute_source_build_step(
                config,
                &pinned_version_str,
                true,
                output_device,
                &bin_name,
            )
            .await
        }
    }
}

/// Attempts release archive download and replacement, falling back to source build on
/// failure if `distribution == CrateDistribution::OpenSource`.
///
/// # Errors
///
/// Returns [`UpgradeError`] if downloading, verifying, replacing the binary, or
/// falling back to source build fails.
pub async fn execute_release_archive_or_fallback(
    config: &UpgradeConfig,
    distribution: CrateDistribution,
    archive_url: &str,
    sha256_url: &str,
    pinned_version: CrateVersion,
    output_device: OutputDevice,
    bin_name: &str,
) -> Result<(), UpgradeError> {
    let pinned_version_str = pinned_version.to_string();
    match download_and_replace_release_archive(
        config.get_crate_name(),
        &pinned_version_str,
        archive_url,
        sha256_url,
        output_device.clone(),
    )
    .await
    {
        Ok(()) => {
            config.get_user_feedback_channel().println(
                ui_strings::format_upgrade_success_msg(
                    config.get_crate_name(),
                    &pinned_version_str,
                ),
            );
            Ok(())
        }
        Err(UpgradeError::Cancelled) => {
            config.get_user_feedback_channel().eprintln(
                ui_strings::format_upgrade_failed_retry_msg(
                    bin_name,
                    upgrade_reasons::CANCELLED_BY_USER,
                ),
            );
            Err(UpgradeError::Cancelled)
        }
        Err(download_err) => {
            // If download fails mid-stream and crate is OpenSource, fall back to
            // source build.
            if let Ok(fallback_source) = UpgradeSource::try_fallback_to_source(
                config.get_crate_name(),
                pinned_version,
                distribution,
            ) {
                config.get_user_feedback_channel().println(
                    ui_strings::format_fallback_to_source_notice(
                        config.get_crate_name(),
                        &pinned_version_str,
                    ),
                );
                let use_rustup = matches!(
                    fallback_source,
                    UpgradeSource::BuildFromSourceRustup { .. }
                );
                return execute_source_build_step(
                    config,
                    &pinned_version_str,
                    use_rustup,
                    output_device,
                    bin_name,
                )
                .await;
            }

            if distribution == CrateDistribution::OpenSource {
                config.get_user_feedback_channel().eprintln(
                    ui_strings::format_open_source_no_toolchain_error(bin_name),
                );
            } else {
                config.get_user_feedback_channel().eprintln(
                    ui_strings::format_closed_source_unreachable_error(bin_name),
                );
            }
            Err(download_err)
        }
    }
}

/// Executes the source compilation step with live spinner progress.
///
/// # Errors
///
/// Returns [`UpgradeError`] if the cargo install or rustup toolchain command fails or
/// exits with a non-zero status.
pub async fn execute_source_build_step(
    config: &UpgradeConfig,
    pinned_version: &str,
    use_rustup: bool,
    output_device: OutputDevice,
    bin_name: &str,
) -> Result<(), UpgradeError> {
    match run_source_build_with_spinner(
        config.get_crate_name(),
        pinned_version,
        use_rustup,
        output_device,
    )
    .await
    {
        Ok(status) if status.success() => {
            config.get_user_feedback_channel().println(
                ui_strings::format_upgrade_success_msg(
                    config.get_crate_name(),
                    pinned_version,
                ),
            );
            Ok(())
        }
        Ok(status) => {
            let reason = format!("cargo install exited with status {status}");
            config.get_user_feedback_channel().eprintln(
                ui_strings::format_upgrade_failed_retry_msg(bin_name, &reason),
            );
            Err(UpgradeError::Other(reason))
        }
        Err(err) if err.kind() == io::ErrorKind::Interrupted => {
            config.get_user_feedback_channel().eprintln(
                ui_strings::format_upgrade_failed_retry_msg(
                    bin_name,
                    upgrade_reasons::CANCELLED_BY_USER,
                ),
            );
            Err(UpgradeError::Cancelled)
        }
        Err(err) => {
            let reason = err.to_string();
            config.get_user_feedback_channel().eprintln(
                ui_strings::format_upgrade_failed_retry_msg(bin_name, &reason),
            );
            Err(UpgradeError::Other(reason))
        }
    }
}

/// Streams the binary archive and SHA-256 checksum from the release server,
/// verifies the checksum, extracts the archive, and atomically swaps the binary via
/// [`self_replace`].
///
/// Supports cooperative `Ctrl+C` cancellation and live spinner progress (`%` and
/// MiB).
///
/// # Errors
///
/// Returns [`UpgradeError`] if the download fails, is canceled, fails SHA-256
/// verification, or cannot replace the executable.
pub async fn download_and_replace_release_archive(
    crate_name: &str,
    pinned_version: &str,
    archive_url: &str,
    sha256_url: &str,
    output_device: OutputDevice,
) -> Result<(), UpgradeError> {
    let temp_dir =
        try_create_temp_dir().map_err(|e| UpgradeError::Other(e.to_string()))?;

    let archive_path = temp_dir.join(ReleaseAsset::format_archive_filename(
        crate_name,
        pinned_version,
        ReleaseTarget::try_from_host()?,
    ));

    let res_spinner = Spinner::try_start(
        ui_strings::format_download_initial_msg(crate_name, pinned_version),
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

    let step_result = async {
        download_and_verify_archive(
            crate_name,
            pinned_version,
            archive_url,
            sha256_url,
            &archive_path,
            maybe_spinner.as_ref(),
        )
        .await?;

        let extract_dir = temp_dir.join(archive::EXTRACTED_DIR_NAME);
        extract_and_apply_archive(&archive_path, &extract_dir)?;
        Ok(())
    }
    .await;

    if let Some(mut spinner) = maybe_spinner.take() {
        spinner.request_shutdown();
        spinner.await_shutdown().await;
    }

    step_result
}

/// Errors that can occur during [`UpgradeSource`] resolution or upgrade execution.
///
/// [`UpgradeSource`]: crate::self_upgrade::UpgradeSource
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, miette::Diagnostic)]
pub enum UpgradeError {
    /// Neither release binaries nor a viable local source build toolchain are
    /// available.
    #[error(
        "No viable upgrade source for '{crate_name}' (distribution={crate_distribution}): {reason}"
    )]
    NoViableUpgradeSource {
        crate_name: CrateName,
        crate_distribution: CrateDistribution,
        reason: NoViableSourceReason,
    },

    /// Host platform or architecture does not have pre-compiled binaries and cannot fall
    /// back.
    #[error("Unsupported host target platform: {0}")]
    UnsupportedPlatform(String),

    /// SHA-256 checksum mismatch on downloaded archive.
    #[error("SHA-256 checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch {
        expected: Sha256Digest,
        actual: Sha256Digest,
    },

    /// User canceled the operation via `Ctrl+C`.
    #[error("Upgrade cancelled by user")]
    Cancelled,

    /// General I/O, network, or archive error.
    #[error("{0}")]
    Other(String),
}

/// Specific reason why [`UpgradeError::NoViableUpgradeSource`] occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NoViableSourceReason {
    /// Pre-compiled release archive server could not be reached for a
    /// [`CrateDistribution::ClosedSource`] crate.
    ///
    /// [`CrateDistribution::ClosedSource`]: crate::self_upgrade::CrateDistribution::ClosedSource
    #[error("{}", upgrade_reasons::BINARY_RELEASE_UNREACHABLE)]
    BinaryReleaseUnreachable,

    /// Pre-compiled release archive server was unreachable and no `rustup` toolchain
    /// was found on `$PATH` for a [`CrateDistribution::OpenSource`] crate.
    ///
    /// [`CrateDistribution::OpenSource`]: crate::self_upgrade::CrateDistribution::OpenSource
    #[error("{}", upgrade_reasons::NO_LOCAL_TOOLCHAIN)]
    NoLocalToolchain,

    /// Pre-compiled release archive download failed mid-stream for a
    /// [`CrateDistribution::ClosedSource`] crate.
    ///
    /// [`CrateDistribution::ClosedSource`]: crate::self_upgrade::CrateDistribution::ClosedSource
    #[error("{}", upgrade_reasons::BINARY_DOWNLOAD_FAILED)]
    BinaryDownloadFailed,
}
