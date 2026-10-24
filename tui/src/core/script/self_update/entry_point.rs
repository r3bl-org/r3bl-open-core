// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Top-level entry points and orchestration for self-upgrade operations.
//!
//! Provides the public APIs and error types invoked by consumer binaries:
//! - [`run_upgrade_cli_arg_interceptor`] / [`run_upgrade_cli_arg_interceptor_sync`]:
//!   Standalone `--upgrade` handler.
//! - [`handle_upgrade_at_exit`] / [`handle_upgrade_at_exit_sync`]: Background update
//!   checker exit handler.
//! - [`ensure_version_checked_and_handle_at_exit`] /
//!   [`ensure_version_checked_and_handle_at_exit_sync`]: Foreground check and exit
//!   handler.
//! - [`run_upgrade`] / [`run_upgrade_sync`]: Direct programmatic upgrade.
//! - [`UpgradeError`]: Errors that can occur during strategy resolution or upgrade
//!   execution.

use super::{CrateDistribution, CrateName, CrateVersion, CrateVersionMetadata,
            SelfBinaryUpgradeConfig, UpdateBehavior, UpgradeCheckCache,
            UpgradeCheckStatus, UpgradeSource, UserFeedbackChannel, archive, bin_names,
            check_version_against_manifest, download_and_verify_archive,
            extract_and_apply_archive, manifest_query, prompt_user_to_upgrade,
            run_source_build_with_spinner, ui_strings, upgrade_cli_args,
            upgrade_reasons, version_compare};
use crate::{InlineString, OutputDevice, Spinner, SpinnerStyle, StdMutex,
            TuiAvailability, inline_string, ok, script::temp_dir::try_create_temp_dir};
use std::{env::current_exe, io, sync::Arc, time::Duration};

// -------------------------------------------------------------------------------------------------
// Top-level Entry Points (Runtime Call-Site Adapters)
// -------------------------------------------------------------------------------------------------

/// Spawns a non-blocking background Tokio task to check [`R3BL_VERSIONS_JSON_URL`]
/// for a newer version of `config.crate_name`.
///
/// Designed for asynchronous binaries (such as `giti` and `edi`) with an active
/// `#[tokio::main]` runtime.
///
/// If [`SelfBinaryUpgradeConfig::update_behavior`] is [`UpdateBehavior::Disable`], this
/// function returns immediately without spawning any background task or making network
/// requests.
///
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::self_update::R3BL_VERSIONS_JSON_URL
pub fn start_background_version_check(config: SelfBinaryUpgradeConfig) {
    if config.update_behavior == UpdateBehavior::Disable {
        return;
    }

    tokio::spawn(async move {
        check_version_against_manifest(&config).await;
    });
}

/// Spawns a detached OS thread with a lightweight `current_thread` Tokio runtime to check
/// [`R3BL_VERSIONS_JSON_URL`].
///
/// Designed for synchronous binaries (such as `rust-analyzer-mcp-server`) that
/// intentionally run without a global `#[tokio::main]` runtime.
///
/// If [`SelfBinaryUpgradeConfig::update_behavior`] is [`UpdateBehavior::Disable`], this
/// function returns immediately without spawning any thread or making network
/// requests.
///
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::self_update::R3BL_VERSIONS_JSON_URL
pub fn start_background_version_check_sync(config: SelfBinaryUpgradeConfig) {
    if config.update_behavior == UpdateBehavior::Disable {
        return;
    }

    std::thread::spawn(move || {
        let Ok(rt) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            UpgradeCheckCache::set(UpgradeCheckStatus::CheckFailed);
            return;
        };
        rt.block_on(async move {
            check_version_against_manifest(&config).await;
        });
    });
}

/// Standalone CLI argument interceptor for `--upgrade` in asynchronous binaries.
///
/// If `--upgrade` is passed in CLI arguments, runs the upgrade flow and terminates the
/// process (`std::process::exit(0)` on success, `exit(1)` on error).
///
/// Returns immediately without side effects if `--upgrade` is not in CLI arguments.
///
/// # Errors
///
/// Returns [`miette::Result`] with an error message on upgrade failure.
pub async fn run_upgrade_cli_arg_interceptor(
    config: &SelfBinaryUpgradeConfig,
) -> miette::Result<()> {
    let has_upgrade_arg = std::env::args().any(|arg| arg == upgrade_cli_args::UPGRADE);
    if !has_upgrade_arg {
        return ok!();
    }

    match run_upgrade(config).await {
        Ok(()) => std::process::exit(0),
        Err(_e) => std::process::exit(1),
    }
}

/// Synchronous variant of [`run_upgrade_cli_arg_interceptor`] for synchronous binaries
/// (e.g., `env-source`, `rust-analyzer-mcp-server`).
///
/// If `--upgrade` is passed in CLI arguments, runs the upgrade flow and terminates the
/// process (`std::process::exit(0)` on success, `exit(1)` on error).
///
/// Returns immediately without side effects if `--upgrade` is not in CLI arguments.
///
/// # Errors
///
/// Returns [`miette::Result`] with an error message on upgrade failure.
pub fn run_upgrade_cli_arg_interceptor_sync(
    config: &SelfBinaryUpgradeConfig,
) -> miette::Result<()> {
    let has_upgrade_arg = std::env::args().any(|arg| arg == upgrade_cli_args::UPGRADE);
    if !has_upgrade_arg {
        return ok!();
    }

    match run_upgrade_sync(config) {
        Ok(()) => std::process::exit(0),
        Err(_e) => std::process::exit(1),
    }
}

/// Programmatic entry point for upgrading the binary governed by `config`.
///
/// # Errors
///
/// Returns [`miette::Result`] with an error message on upgrade failure.
pub async fn run_upgrade(config: &SelfBinaryUpgradeConfig) -> miette::Result<()> {
    let metadata = match metadata_resolver::resolve_metadata_for_upgrade(config).await {
        Ok(m) => m,
        Err(e) => {
            let bin_name = metadata_resolver::get_current_bin_name();
            let msg =
                ui_strings::format_upgrade_failed_retry_msg(&bin_name, &e.to_string());
            config.user_feedback_channel.eprintln(msg);
            miette::bail!("{e}");
        }
    };

    if !version_compare::is_remote_version_strictly_newer(
        metadata.version,
        config.current_version,
    ) {
        config
            .user_feedback_channel
            .println(ui_strings::format_already_up_to_date_msg(
                config.crate_name.as_str(),
                &config.current_version.to_string(),
            ));
        return ok!();
    }

    upgrade_execution::execute_upgrade_with_metadata(config, &metadata)
        .await
        .map_err(|e| miette::miette!("{e}"))
}

/// Synchronous variant of [`run_upgrade`] for synchronous binaries (e.g., `env-source`,
/// `rust-analyzer-mcp-server`).
///
/// Builds a private `current_thread` Tokio runtime on the calling thread.
///
/// # Errors
///
/// Returns [`miette::Result`] with an error message on upgrade failure.
pub fn run_upgrade_sync(config: &SelfBinaryUpgradeConfig) -> miette::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| miette::miette!("Failed to create tokio runtime: {e}"))?;
    rt.block_on(run_upgrade(config))
}

/// Handles upgrade actions at application exit based on
/// [`SelfBinaryUpgradeConfig::update_behavior`].
///
/// Reads cached metadata (populated during startup by background check). If no update is
/// available, returns immediately with 0 ms latency.
///
/// If an update is available:
/// - [`UpdateBehavior::Disable`][]: No-op.
/// - [`UpdateBehavior::NotifyOnly`]: Prints notification notice.
/// - [`UpdateBehavior::Auto`]: Downloads and installs update automatically.
/// - [`UpdateBehavior::PromptToUpgrade`]: Prompts the user interactively via
///   [`prompt_user_to_upgrade`].
pub async fn handle_upgrade_at_exit(config: &SelfBinaryUpgradeConfig) {
    if config.update_behavior == UpdateBehavior::Disable {
        return;
    }

    let UpgradeCheckStatus::UpgradeAvailable(metadata) = UpgradeCheckCache::get() else {
        return;
    };

    let bin_name = metadata_resolver::get_current_bin_name();
    let current_version_str = config.current_version.to_string();
    let remote_version_str = metadata.version.to_string();

    match config.update_behavior {
        UpdateBehavior::Disable => {}
        UpdateBehavior::NotifyOnly => {
            config.user_feedback_channel.println(
                ui_strings::format_upgrade_available_notice(
                    &bin_name,
                    &current_version_str,
                    &remote_version_str,
                ),
            );
        }
        UpdateBehavior::Auto => {
            config.user_feedback_channel.println(
                ui_strings::format_upgrade_available_notice(
                    &bin_name,
                    &current_version_str,
                    &remote_version_str,
                ),
            );
            drop(
                upgrade_execution::execute_upgrade_with_metadata(config, &metadata).await,
            );
        }
        UpdateBehavior::PromptToUpgrade => {
            config.user_feedback_channel.println(
                ui_strings::format_upgrade_available_notice(
                    &bin_name,
                    &current_version_str,
                    &remote_version_str,
                ),
            );

            if prompt_user_to_upgrade(config.crate_name.as_str(), &remote_version_str)
                .await
            {
                drop(
                    upgrade_execution::execute_upgrade_with_metadata(config, &metadata)
                        .await,
                );
            }
        }
    }
}

/// Synchronous wrapper around [`handle_upgrade_at_exit`] for binaries that run without a
/// global Tokio runtime (`rust-analyzer-mcp-server`).
pub fn handle_upgrade_at_exit_sync(config: &SelfBinaryUpgradeConfig) {
    if !UpgradeCheckCache::is_upgrade_available() {
        return;
    }

    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return;
    };

    rt.block_on(handle_upgrade_at_exit(config));
}

/// Helper for asynchronous binaries that want to ensure the version check has completed
/// before exit (or run it if not yet checked).
pub async fn ensure_version_checked_and_handle_at_exit(config: &SelfBinaryUpgradeConfig) {
    if matches!(UpgradeCheckCache::get(), UpgradeCheckStatus::NotChecked) {
        check_version_against_manifest(config).await;
    }
    handle_upgrade_at_exit(config).await;
}

/// Synchronous variant of [`ensure_version_checked_and_handle_at_exit`] for synchronous
/// binaries (`env-source`, `rust-analyzer-mcp-server`).
pub fn ensure_version_checked_and_handle_at_exit_sync(config: &SelfBinaryUpgradeConfig) {
    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return;
    };

    rt.block_on(ensure_version_checked_and_handle_at_exit(config));
}

// -------------------------------------------------------------------------------------------------
// Error Types
// -------------------------------------------------------------------------------------------------

/// Errors that can occur during strategy resolution or upgrade execution.
#[derive(Debug, thiserror::Error)]
pub enum UpgradeError {
    /// Neither GitHub release binaries nor a viable local source build toolchain are
    /// available.
    #[error(
        "No viable upgrade source for '{crate_name}' (distribution={crate_distribution}): {reason}"
    )]
    NoViableUpgradeSource {
        crate_name: CrateName,
        crate_distribution: CrateDistribution,
        reason: String,
    },

    /// Host platform or architecture does not have pre-compiled binaries and cannot fall
    /// back.
    #[error("Unsupported host target platform: {0}")]
    UnsupportedPlatform(String),

    /// SHA-256 checksum mismatch on downloaded archive.
    #[error("SHA-256 checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    /// User canceled the operation via `Ctrl+C`.
    #[error("Upgrade cancelled by user")]
    Cancelled,

    /// General I/O, network, or archive error.
    #[error("{0}")]
    Other(String),
}

// -------------------------------------------------------------------------------------------------
// Inner Modules (Underlying Implementation & Domain Logic)
// -------------------------------------------------------------------------------------------------

/// Core upgrade execution engine handling strategy resolution, binary download/swap,
/// and source compilation.
pub mod upgrade_execution {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Executes the resolved upgrade flow for `config`:
    /// 1. Resolves [`UpgradeSource`] via [`UpgradeSource::try_resolve`].
    /// 2. If [`UpgradeSource::GithubReleaseArtifact`], streams the binary archive,
    ///    verifies `.sha256`, and swaps via `self_replace`. If the download fails
    ///    mid-stream and [`CrateDistribution::OpenSource`], automatically transitions to
    ///    the `crates.io` source compilation fallback (`--version <pinned_ver>`).
    /// 3. If [`UpgradeSource::BuildFromSourceRustup`], runs the [`PTY`]/[`OSC`] source
    ///    build pinned to `--version <pinned_ver>`.
    ///
    /// # Errors
    ///
    /// Returns [`UpgradeError`] if all viable upgrade paths fail or are canceled.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub async fn execute_upgrade_with_metadata(
        config: &SelfBinaryUpgradeConfig,
        metadata: &CrateVersionMetadata,
    ) -> Result<(), UpgradeError> {
        let bin_name = metadata_resolver::get_current_bin_name();
        let strategy = match UpgradeSource::try_resolve(
            &config.crate_name,
            metadata.version,
            metadata.crate_distribution,
        )
        .await
        {
            Ok(strategy) => strategy,
            Err(err) => {
                feedback_device::report_strategy_error(
                    config,
                    metadata.crate_distribution,
                    &bin_name,
                    &err,
                );
                return Err(err);
            }
        };

        let output_device = feedback_device::create_output_device_for_feedback_channel(
            config.user_feedback_channel,
        );

        match strategy {
            UpgradeSource::GithubReleaseArtifact {
                archive_url,
                sha256_url,
                pinned_version,
            } => {
                execute_github_release_or_fallback(
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
                config.user_feedback_channel.println(
                    ui_strings::format_fallback_to_source_notice(
                        config.crate_name.as_str(),
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

    /// Attempts GitHub release download and replacement, falling back to source build on
    /// failure if `distribution == CrateDistribution::OpenSource`.
    ///
    /// # Errors
    ///
    /// Returns [`UpgradeError`] if downloading, verifying, replacing the binary, or
    /// falling back to source build fails.
    pub async fn execute_github_release_or_fallback(
        config: &SelfBinaryUpgradeConfig,
        distribution: CrateDistribution,
        archive_url: &str,
        sha256_url: &str,
        pinned_version: CrateVersion,
        output_device: OutputDevice,
        bin_name: &str,
    ) -> Result<(), UpgradeError> {
        let pinned_version_str = pinned_version.to_string();
        match download_and_replace_github_release(
            config.crate_name.as_str(),
            &pinned_version_str,
            archive_url,
            sha256_url,
            output_device.clone(),
        )
        .await
        {
            Ok(()) => {
                config.user_feedback_channel.println(
                    ui_strings::format_upgrade_success_msg(
                        config.crate_name.as_str(),
                        &pinned_version_str,
                    ),
                );
                Ok(())
            }
            Err(UpgradeError::Cancelled) => {
                config.user_feedback_channel.eprintln(
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
                    &config.crate_name,
                    pinned_version,
                    distribution,
                ) {
                    config.user_feedback_channel.println(
                        ui_strings::format_fallback_to_source_notice(
                            config.crate_name.as_str(),
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
                    config.user_feedback_channel.eprintln(
                        ui_strings::format_open_source_no_toolchain_error(bin_name),
                    );
                } else {
                    config.user_feedback_channel.eprintln(
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
        config: &SelfBinaryUpgradeConfig,
        pinned_version: &str,
        use_rustup: bool,
        output_device: OutputDevice,
        bin_name: &str,
    ) -> Result<(), UpgradeError> {
        match run_source_build_with_spinner(
            config.crate_name.as_str(),
            pinned_version,
            use_rustup,
            output_device,
        )
        .await
        {
            Ok(status) if status.success() => {
                config.user_feedback_channel.println(
                    ui_strings::format_upgrade_success_msg(
                        config.crate_name.as_str(),
                        pinned_version,
                    ),
                );
                Ok(())
            }
            Ok(status) => {
                let reason = format!("cargo install exited with status {status}");
                config.user_feedback_channel.eprintln(
                    ui_strings::format_upgrade_failed_retry_msg(bin_name, &reason),
                );
                Err(UpgradeError::Other(reason))
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => {
                config.user_feedback_channel.eprintln(
                    ui_strings::format_upgrade_failed_retry_msg(
                        bin_name,
                        upgrade_reasons::CANCELLED_BY_USER,
                    ),
                );
                Err(UpgradeError::Cancelled)
            }
            Err(err) => {
                let reason = err.to_string();
                config.user_feedback_channel.eprintln(
                    ui_strings::format_upgrade_failed_retry_msg(bin_name, &reason),
                );
                Err(UpgradeError::Other(reason))
            }
        }
    }

    /// Streams the binary archive and `.sha256` checksum from
    /// `github.com/r3bl-org/r3bl-binaries`, verifies the checksum, extracts the archive,
    /// and atomically swaps the binary via [`self_replace`].
    ///
    /// Supports cooperative `Ctrl+C` cancellation and live spinner progress (`%` and
    /// `MB`).
    ///
    /// # Errors
    ///
    /// Returns [`UpgradeError`] if the download fails, is canceled, fails SHA-256
    /// verification, or cannot replace the executable.
    pub async fn download_and_replace_github_release(
        crate_name: &str,
        pinned_version: &str,
        archive_url: &str,
        sha256_url: &str,
        output_device: OutputDevice,
    ) -> Result<(), UpgradeError> {
        let temp_dir =
            try_create_temp_dir().map_err(|e| UpgradeError::Other(e.to_string()))?;

        let archive_file_name = archive_url
            .rsplit('/')
            .next()
            .unwrap_or(archive::DEFAULT_ARCHIVE_FILENAME);
        let archive_path = temp_dir.join(archive_file_name);

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
}

/// Metadata resolution and binary naming helpers.
pub mod metadata_resolver {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Returns the filename of the currently running executable (without directory path).
    #[must_use]
    pub fn get_current_bin_name() -> InlineString {
        current_exe()
            .ok()
            .and_then(|path_buf| {
                path_buf
                    .file_name()?
                    .to_str()
                    .map(|f_name| inline_string!("{f_name}"))
            })
            .unwrap_or_else(|| inline_string!("{}", bin_names::DEFAULT_FALLBACK_BIN_NAME))
    }

    /// Resolves the [`CrateVersionMetadata`] for `config.crate_name` (reading from the
    /// cache if already populated, or querying [`R3BL_VERSIONS_JSON_URL`] on demand).
    ///
    /// # Errors
    ///
    /// Returns [`UpgradeError::Other`] if [`R3BL_VERSIONS_JSON_URL`] cannot be reached or
    /// parsed.
    ///
    /// [`CrateVersionMetadata`]: crate::core::script::self_update::CrateVersionMetadata
    /// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::self_update::R3BL_VERSIONS_JSON_URL
    pub async fn resolve_metadata_for_upgrade(
        config: &SelfBinaryUpgradeConfig,
    ) -> Result<CrateVersionMetadata, UpgradeError> {
        if let Some(meta) = UpgradeCheckCache::try_get_crate_metadata() {
            return Ok(meta);
        }
        manifest_query::try_get_crate_metadata_from_r3bl_versions_json(
            config.crate_name.as_str(),
        )
        .await
        .map_err(|e| UpgradeError::Other(e.to_string()))
    }
}

/// Feedback channel output devices and user-facing error reporting.
pub mod feedback_device {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Constructs an [`OutputDevice`] matching the requested [`UserFeedbackChannel`].
    #[must_use]
    pub fn create_output_device_for_feedback_channel(
        channel: UserFeedbackChannel,
    ) -> OutputDevice {
        match channel {
            UserFeedbackChannel::Stdout => OutputDevice::new_stdout(),
            UserFeedbackChannel::Stderr => OutputDevice {
                resource: Arc::new(StdMutex::new(std::io::stderr())),
                paint_mode: crate::PaintMode::Real,
            },
            UserFeedbackChannel::Quiet => OutputDevice {
                resource: Arc::new(StdMutex::new(std::io::sink())),
                paint_mode: crate::PaintMode::Mock,
            },
        }
    }

    /// Formats and logs errors encountered during strategy resolution.
    pub fn report_strategy_error(
        config: &SelfBinaryUpgradeConfig,
        distribution: CrateDistribution,
        bin_name: &str,
        err: &UpgradeError,
    ) {
        match err {
            UpgradeError::NoViableUpgradeSource { .. } => {
                if distribution == CrateDistribution::ClosedSource {
                    config.user_feedback_channel.eprintln(
                        ui_strings::format_closed_source_unreachable_error(bin_name),
                    );
                } else {
                    config.user_feedback_channel.eprintln(
                        ui_strings::format_open_source_no_toolchain_error(bin_name),
                    );
                }
            }
            other => {
                config.user_feedback_channel.eprintln(
                    ui_strings::format_upgrade_failed_retry_msg(
                        bin_name,
                        &other.to_string(),
                    ),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{super::{UpgradeCheckCache, UpgradeCheckStatus},
                *};
    use chrono::Utc;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_run_upgrade_already_up_to_date() {
        let version = CrateVersion::try_from("0.0.25").unwrap();
        let meta = CrateVersionMetadata {
            version,
            crate_distribution: CrateDistribution::OpenSource,
            released_on: Utc::now(),
        };
        UpgradeCheckCache::set(UpgradeCheckStatus::UpToDate(meta));

        let config = SelfBinaryUpgradeConfig::new(
            "r3bl-cmdr".into(),
            version,
            None,
            UserFeedbackChannel::Quiet,
        );

        let result = run_upgrade(&config).await;
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_run_upgrade_sync_already_up_to_date() {
        let version = CrateVersion::try_from("0.0.25").unwrap();
        let meta = CrateVersionMetadata {
            version,
            crate_distribution: CrateDistribution::OpenSource,
            released_on: Utc::now(),
        };
        UpgradeCheckCache::set(UpgradeCheckStatus::UpToDate(meta));

        let config = SelfBinaryUpgradeConfig::new(
            "r3bl-cmdr".into(),
            version,
            None,
            UserFeedbackChannel::Quiet,
        );

        let result = run_upgrade_sync(&config);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_upgrade_cli_arg_interceptor_no_flag() {
        let version = CrateVersion::try_from("0.0.25").unwrap();
        let config = SelfBinaryUpgradeConfig::new(
            "r3bl-cmdr".into(),
            version,
            None,
            UserFeedbackChannel::Quiet,
        );

        // When `--upgrade` is not in std::env::args(), interceptor returns Ok
        // immediately.
        let result = run_upgrade_cli_arg_interceptor(&config).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_upgrade_cli_arg_interceptor_sync_no_flag() {
        let version = CrateVersion::try_from("0.0.25").unwrap();
        let config = SelfBinaryUpgradeConfig::new(
            "r3bl-cmdr".into(),
            version,
            None,
            UserFeedbackChannel::Quiet,
        );

        // When `--upgrade` is not in std::env::args(), interceptor returns Ok
        // immediately.
        let result = run_upgrade_cli_arg_interceptor_sync(&config);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_handle_upgrade_at_exit_sync_no_upgrade() {
        let version = CrateVersion::try_from("0.0.25").unwrap();
        let meta = CrateVersionMetadata {
            version,
            crate_distribution: CrateDistribution::OpenSource,
            released_on: Utc::now(),
        };
        UpgradeCheckCache::set(UpgradeCheckStatus::UpToDate(meta));
        let config = SelfBinaryUpgradeConfig::new(
            "r3bl-cmdr".into(),
            version,
            None,
            UserFeedbackChannel::Quiet,
        );
        handle_upgrade_at_exit_sync(&config);
    }

    #[tokio::test]
    #[serial]
    async fn test_ensure_version_checked_and_handle_at_exit_no_upgrade() {
        let version = CrateVersion::try_from("0.0.25").unwrap();
        let meta = CrateVersionMetadata {
            version,
            crate_distribution: CrateDistribution::OpenSource,
            released_on: Utc::now(),
        };
        UpgradeCheckCache::set(UpgradeCheckStatus::UpToDate(meta));

        let config = SelfBinaryUpgradeConfig::new(
            "r3bl-cmdr".into(),
            version,
            None,
            UserFeedbackChannel::Quiet,
        );
        ensure_version_checked_and_handle_at_exit(&config).await;
    }

    #[test]
    #[serial]
    fn test_ensure_version_checked_and_handle_at_exit_sync_no_upgrade() {
        let version = CrateVersion::try_from("0.0.25").unwrap();
        let meta = CrateVersionMetadata {
            version,
            crate_distribution: CrateDistribution::OpenSource,
            released_on: Utc::now(),
        };
        UpgradeCheckCache::set(UpgradeCheckStatus::UpToDate(meta));

        let config = SelfBinaryUpgradeConfig::new(
            "r3bl-cmdr".into(),
            version,
            None,
            UserFeedbackChannel::Quiet,
        );
        ensure_version_checked_and_handle_at_exit_sync(&config);
    }

    #[test]
    #[serial]
    fn test_start_background_version_check_disabled() {
        UpgradeCheckCache::set(UpgradeCheckStatus::NotChecked);
        let config = SelfBinaryUpgradeConfig::new(
            "r3bl-cmdr".into(),
            CrateVersion::try_from("0.0.25").unwrap(),
            Some(UpdateBehavior::Disable),
            UserFeedbackChannel::Quiet,
        );
        start_background_version_check(config);
        assert_eq!(UpgradeCheckCache::get(), UpgradeCheckStatus::NotChecked);
    }

    #[test]
    #[serial]
    fn test_start_background_version_check_sync_disabled() {
        UpgradeCheckCache::set(UpgradeCheckStatus::NotChecked);
        let config = SelfBinaryUpgradeConfig::new(
            "r3bl-cmdr".into(),
            CrateVersion::try_from("0.0.25").unwrap(),
            Some(UpdateBehavior::Disable),
            UserFeedbackChannel::Quiet,
        );
        start_background_version_check_sync(config);
        assert_eq!(UpgradeCheckCache::get(), UpgradeCheckStatus::NotChecked);
    }
}
