// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Upgrade execution engine: streaming binary archive download from
//! `github.com/r3bl-org/r3bl-binaries`, `.sha256` checksum verification, archive
//! extraction, atomic `self_replace` binary replacement (including sibling binaries in
//! the archive), and automatic fallback to `crates.io` source compilation when
//! `open_source == true`.

use super::{
    CrateDistribution, CrateVersionMetadata, UpgradeConfig, UpgradeError,
    UpgradeOutputTarget, UpgradePolicy, UpgradeSource, archive, check_version_against_sot,
    get_cached_crate_metadata, is_upgrade_available, resolve_source_fallback,
    resolve_upgrade_strategy, run_source_build_with_spinner,
    try_get_crate_metadata_from_r3bl_versions_json, ui_strings, upgrade_reasons,
};
use crate::{DefaultIoDevices, HowToChoose, InlineString, OutputDevice, Spinner,
            SpinnerStyle, StdMutex, StyleSheet, TerminalInteractiveStatus,
            TuiAvailability, TuiAvailabilityChooseExt, check_is_terminal_interactive,
            choose, cli_text_inline, cli_text_line, inline_string, new_style, ok,
            script::{http_client::create_client_with_user_agent,
                     permissions::try_set_file_executable,
                     temp_dir::try_create_temp_dir},
            vp_height};
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use smallvec::smallvec;
use std::{env::current_exe,
          fs::{self, File},
          io::{self, Write},
          path::{Path, PathBuf},
          sync::Arc,
          time::Duration};
use tokio::signal;

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
        .unwrap_or_else(|| inline_string!("r3bl-binary"))
}

/// Constructs an [`OutputDevice`] matching the requested [`UpgradeOutputTarget`].
#[must_use]
pub fn create_output_device_for_target(target: UpgradeOutputTarget) -> OutputDevice {
    match target {
        UpgradeOutputTarget::Stdout => OutputDevice::new_stdout(),
        UpgradeOutputTarget::Stderr => OutputDevice {
            resource: Arc::new(StdMutex::new(std::io::stderr())),
            paint_mode: crate::PaintMode::Real,
        },
        UpgradeOutputTarget::Quiet => OutputDevice {
            resource: Arc::new(StdMutex::new(std::io::sink())),
            paint_mode: crate::PaintMode::Mock,
        },
    }
}

/// Parses the expected hex SHA-256 digest from the contents of a `.sha256` checksum file
/// (handles both `<hex>  <filename>` and plain `<hex>` formats).
#[must_use]
pub fn parse_sha256_checksum_text(text: &str) -> Option<String> {
    let token = text.split_whitespace().next()?.trim();
    if token.len() == 64 && token.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(token.to_ascii_lowercase())
    } else {
        None
    }
}

/// Computes the lower-case hex SHA-256 digest of a byte slice.
#[must_use]
pub fn compute_sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hex_str = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _unused = write!(hex_str, "{byte:02x}");
    }
    hex_str
}

/// Verifies that the SHA-256 digest of `file_path` matches `expected_hex`.
///
/// # Errors
///
/// Returns [`UpgradeError::ChecksumMismatch`] if the computed digest differs from
/// `expected_hex`, or [`UpgradeError::Other`] on file read failure.
pub fn verify_file_sha256(
    file_path: &Path,
    expected_hex: &str,
) -> Result<(), UpgradeError> {
    let bytes = fs::read(file_path).map_err(|e| UpgradeError::Other(e.to_string()))?;
    let actual_hex = compute_sha256_hex(&bytes);
    let expected_lower = expected_hex.trim().to_ascii_lowercase();
    if actual_hex != expected_lower {
        return Err(UpgradeError::ChecksumMismatch {
            expected: expected_lower,
            actual: actual_hex,
        });
    }
    Ok(())
}

/// Unpacks a `.tar.gz` archive into `extract_dir` and returns the list of extracted
/// regular file paths.
///
/// # Errors
///
/// Returns [`UpgradeError::Other`] if decompression or extraction fails.
pub fn extract_archive_files(
    archive_path: &Path,
    extract_dir: &Path,
) -> Result<Vec<PathBuf>, UpgradeError> {
    let tar_gz =
        File::open(archive_path).map_err(|e| UpgradeError::Other(e.to_string()))?;
    let tar_decoder = GzDecoder::new(tar_gz);
    let mut archive = tar::Archive::new(tar_decoder);

    let mut extracted_files = Vec::new();
    let entries = archive
        .entries()
        .map_err(|e| UpgradeError::Other(e.to_string()))?;

    for entry_res in entries {
        let mut entry = entry_res.map_err(|e| UpgradeError::Other(e.to_string()))?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let entry_path = entry
            .path()
            .map_err(|e| UpgradeError::Other(e.to_string()))?;
        let Some(base_name) = entry_path.file_name() else {
            continue;
        };
        let dest_path = extract_dir.join(base_name);
        entry
            .unpack(&dest_path)
            .map_err(|e| UpgradeError::Other(e.to_string()))?;
        try_set_file_executable(&dest_path)
            .map_err(|e| UpgradeError::Other(e.to_string()))?;
        extracted_files.push(dest_path);
    }

    Ok(extracted_files)
}

fn is_non_binary_metadata_file(path: &Path) -> bool {
    path.extension().is_some_and(|ext| {
        archive::NON_BINARY_EXTENSIONS
            .iter()
            .any(|&ignored| ext.eq_ignore_ascii_case(ignored))
    })
}

/// Replaces the currently running executable via [`self_replace::self_replace`] and
/// updates any sibling binaries included in the extracted archive (e.g. when `giti`
/// upgrades `r3bl-cmdr`, it also updates `edi` and `env-source` in the same directory).
///
/// # Errors
///
/// Returns [`UpgradeError::Other`] if the current executable path cannot be resolved or
/// if `self_replace` fails.
pub fn apply_extracted_binaries(extracted_files: &[PathBuf]) -> Result<(), UpgradeError> {
    let current_exe_path =
        current_exe().map_err(|e| UpgradeError::Other(e.to_string()))?;
    let current_exe_name = current_exe_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| {
            UpgradeError::Other("Could not determine current binary name".to_string())
        })?;
    let install_dir = current_exe_path.parent().ok_or_else(|| {
        UpgradeError::Other("Could not determine binary parent directory".to_string())
    })?;

    let mut replaced_self = false;

    for extracted_file in extracted_files {
        let Some(file_name) = extracted_file.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        if is_non_binary_metadata_file(extracted_file) {
            continue;
        }

        let current_stem = current_exe_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(current_exe_name);
        let extracted_stem = extracted_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(file_name);

        if file_name.eq_ignore_ascii_case(current_exe_name)
            || extracted_stem.eq_ignore_ascii_case(current_stem)
        {
            self_replace::self_replace(extracted_file)
                .map_err(|e| UpgradeError::Other(format!("self_replace failed: {e}")))?;
            replaced_self = true;
        } else {
            let sibling_dest = install_dir.join(file_name);
            let sibling_tmp =
                install_dir.join(format!(".{file_name}.{}", archive::UPGRADE_TMP_SUFFIX));
            if fs::copy(extracted_file, &sibling_tmp).is_ok() {
                drop(try_set_file_executable(&sibling_tmp));
                if fs::rename(&sibling_tmp, &sibling_dest).is_err() {
                    drop(fs::copy(&sibling_tmp, &sibling_dest));
                    drop(fs::remove_file(&sibling_tmp));
                }
            }
        }
    }

    if !replaced_self && extracted_files.len() == 1 {
        self_replace::self_replace(&extracted_files[0])
            .map_err(|e| UpgradeError::Other(format!("self_replace failed: {e}")))?;
    }

    Ok(())
}

async fn fetch_expected_sha256(
    client: &reqwest::Client,
    sha256_url: &str,
) -> Result<String, UpgradeError> {
    let sha_resp = tokio::select! {
        _ = signal::ctrl_c() => return Err(UpgradeError::Cancelled),
        res = client.get(sha256_url).send() => {
            res.map_err(|e| UpgradeError::Other(e.to_string()))?
        }
    };
    let sha_resp = sha_resp
        .error_for_status()
        .map_err(|e| UpgradeError::Other(e.to_string()))?;
    let sha_text = tokio::select! {
        _ = signal::ctrl_c() => return Err(UpgradeError::Cancelled),
        res = sha_resp.text() => {
            res.map_err(|e| UpgradeError::Other(e.to_string()))?
        }
    };
    parse_sha256_checksum_text(&sha_text).ok_or_else(|| {
        UpgradeError::Other(format!("Invalid .sha256 file format: {sha_text}"))
    })
}

async fn stream_archive_to_disk(
    client: &reqwest::Client,
    archive_url: &str,
    archive_path: &Path,
    crate_name: &str,
    pinned_version: &str,
    maybe_spinner: Option<&Spinner>,
) -> Result<(), UpgradeError> {
    let archive_resp = tokio::select! {
        _ = signal::ctrl_c() => return Err(UpgradeError::Cancelled),
        res = client.get(archive_url).send() => {
            res.map_err(|e| UpgradeError::Other(e.to_string()))?
        }
    };
    let archive_resp = archive_resp
        .error_for_status()
        .map_err(|e| UpgradeError::Other(e.to_string()))?;
    let total_bytes = archive_resp.content_length();

    let mut dest_file =
        File::create(archive_path).map_err(|e| UpgradeError::Other(e.to_string()))?;
    let mut downloaded_bytes: u64 = 0;
    let mut stream = archive_resp.bytes_stream();

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                return Err(UpgradeError::Cancelled);
            }
            maybe_chunk = stream.next() => {
                match maybe_chunk {
                    Some(Ok(chunk)) => {
                        dest_file
                            .write_all(&chunk)
                            .map_err(|e| UpgradeError::Other(e.to_string()))?;
                        downloaded_bytes = downloaded_bytes.saturating_add(
                            u64::try_from(chunk.len()).unwrap_or(u64::MAX),
                        );
                        if let Some(spinner) = maybe_spinner {
                            spinner.update_message(
                                ui_strings::format_download_progress_msg(
                                    crate_name,
                                    pinned_version,
                                    downloaded_bytes,
                                    total_bytes,
                                ),
                            );
                        }
                    }
                    Some(Err(err)) => {
                        return Err(UpgradeError::Other(err.to_string()));
                    }
                    None => break,
                }
            }
        }
    }

    dest_file
        .flush()
        .map_err(|e| UpgradeError::Other(e.to_string()))?;
    Ok(())
}

/// Streams the binary archive and `.sha256` checksum from
/// `github.com/r3bl-org/r3bl-binaries`, verifies the checksum, extracts the archive, and
/// atomically swaps the binary via [`self_replace`].
///
/// Supports cooperative `Ctrl+C` cancellation and live spinner progress (`%` and `MB`).
///
/// # Errors
///
/// Returns [`UpgradeError`] if the download fails, is cancelled, fails SHA-256
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
        let client = create_client_with_user_agent(None)
            .map_err(|e| UpgradeError::Other(e.to_string()))?;

        let expected_sha256 = fetch_expected_sha256(&client, sha256_url).await?;
        stream_archive_to_disk(
            &client,
            archive_url,
            &archive_path,
            crate_name,
            pinned_version,
            maybe_spinner.as_ref(),
        )
        .await?;

        if let Some(ref spinner) = maybe_spinner {
            spinner.update_message(ui_strings::format_extract_and_replace_msg(
                crate_name,
                pinned_version,
            ));
        }

        verify_file_sha256(&archive_path, &expected_sha256)?;

        let extract_dir = temp_dir.join(archive::EXTRACTED_DIR_NAME);
        fs::create_dir_all(&extract_dir)
            .map_err(|e| UpgradeError::Other(e.to_string()))?;
        let extracted_files = extract_archive_files(&archive_path, &extract_dir)?;
        if extracted_files.is_empty() {
            return Err(UpgradeError::Other(
                "Downloaded archive contained no files".to_string(),
            ));
        }

        apply_extracted_binaries(&extracted_files)?;
        Ok(())
    }
    .await;

    if let Some(mut spinner) = maybe_spinner.take() {
        spinner.request_shutdown();
        spinner.await_shutdown().await;
    }

    step_result
}

/// Resolves the [`CrateVersionMetadata`] for `config.crate_name` (reading from the cache
/// if already populated, or querying `https://r3bl.com/versions.json` on demand).
///
/// # Errors
///
/// Returns [`UpgradeError::Other`] if `versions.json` cannot be reached or parsed.
pub async fn resolve_metadata_for_upgrade(
    config: &UpgradeConfig,
) -> Result<CrateVersionMetadata, UpgradeError> {
    if let Some(cached) = get_cached_crate_metadata() {
        return Ok(cached);
    }

    try_get_crate_metadata_from_r3bl_versions_json(config.crate_name.as_str())
        .await
        .map_err(|e| UpgradeError::Other(e.to_string()))
}

/// Executes the resolved upgrade flow for `config`:
/// 1. Resolves [`UpgradeSource`] via [`resolve_upgrade_strategy`].
/// 2. If [`UpgradeSource::GithubReleaseArtifact`], streams the binary archive, verifies
///    `.sha256`, and swaps via `self_replace`. If the download fails mid-stream and
///    [`CrateDistribution::OpenSource`], automatically transitions to the `crates.io`
///    source compilation fallback (`--version <pinned_ver>`).
/// 3. If [`UpgradeSource::BuildFromSourceRustup`] or
///    [`UpgradeSource::BuildFromSourceCargo`], runs the [`PTY`]/[`OSC`] source build
///    pinned to `--version <pinned_ver>`.
///
/// # Errors
///
/// Returns [`UpgradeError`] if all viable upgrade paths fail or are cancelled.
///
/// [`OSC`]: crate::osc_codes::OscSequence
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub async fn execute_upgrade_with_metadata(
    config: &UpgradeConfig,
    metadata: &CrateVersionMetadata,
) -> Result<(), UpgradeError> {
    let bin_name = get_current_bin_name();
    let strategy = match resolve_upgrade_strategy(
        config.crate_name.as_str(),
        metadata.version.as_str(),
        metadata.crate_distribution,
    )
    .await
    {
        Ok(strategy) => strategy,
        Err(err) => {
            report_strategy_error(config, metadata.crate_distribution, &bin_name, &err);
            return Err(err);
        }
    };

    let output_device = create_output_device_for_target(config.output_target);

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
                pinned_version.as_str(),
                output_device,
                &bin_name,
            )
            .await
        }
        UpgradeSource::BuildFromSourceRustup { pinned_version } => {
            config
                .output_target
                .println(ui_strings::format_fallback_to_source_notice(
                    config.crate_name.as_str(),
                    pinned_version.as_str(),
                ));
            execute_source_build_step(
                config,
                pinned_version.as_str(),
                true,
                output_device,
                &bin_name,
            )
            .await
        }
        UpgradeSource::BuildFromSourceCargo { pinned_version } => {
            config
                .output_target
                .println(ui_strings::format_fallback_to_source_notice(
                    config.crate_name.as_str(),
                    pinned_version.as_str(),
                ));
            execute_source_build_step(
                config,
                pinned_version.as_str(),
                false,
                output_device,
                &bin_name,
            )
            .await
        }
    }
}

async fn execute_github_release_or_fallback(
    config: &UpgradeConfig,
    distribution: CrateDistribution,
    archive_url: &str,
    sha256_url: &str,
    pinned_version: &str,
    output_device: OutputDevice,
    bin_name: &str,
) -> Result<(), UpgradeError> {
    match download_and_replace_github_release(
        config.crate_name.as_str(),
        pinned_version,
        archive_url,
        sha256_url,
        output_device.clone(),
    )
    .await
    {
        Ok(()) => {
            config
                .output_target
                .println(ui_strings::format_upgrade_success_msg(
                    config.crate_name.as_str(),
                    pinned_version,
                ));
            Ok(())
        }
        Err(UpgradeError::Cancelled) => {
            config
                .output_target
                .eprintln(ui_strings::format_upgrade_failed_retry_msg(
                    bin_name,
                    upgrade_reasons::CANCELLED_BY_USER,
                ));
            Err(UpgradeError::Cancelled)
        }
        Err(download_err) => {
            if distribution == CrateDistribution::OpenSource
                && let Ok(fallback_source) = resolve_source_fallback(
                    config.crate_name.as_str(),
                    pinned_version,
                    CrateDistribution::OpenSource,
                )
            {
                config.output_target.println(
                    ui_strings::format_fallback_to_source_notice(
                        config.crate_name.as_str(),
                        pinned_version,
                    ),
                );
                let use_rustup = matches!(
                    fallback_source,
                    UpgradeSource::BuildFromSourceRustup { .. }
                );
                return execute_source_build_step(
                    config,
                    pinned_version,
                    use_rustup,
                    output_device,
                    bin_name,
                )
                .await;
            }

            if distribution == CrateDistribution::OpenSource {
                config.output_target.eprintln(
                    ui_strings::format_open_source_no_toolchain_error(bin_name),
                );
            } else {
                config.output_target.eprintln(
                    ui_strings::format_closed_source_unreachable_error(bin_name),
                );
            }
            Err(download_err)
        }
    }
}

async fn execute_source_build_step(
    config: &UpgradeConfig,
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
            config
                .output_target
                .println(ui_strings::format_upgrade_success_msg(
                    config.crate_name.as_str(),
                    pinned_version,
                ));
            Ok(())
        }
        Ok(status) => {
            let reason = format!("cargo install exited with status {status}");
            config
                .output_target
                .eprintln(ui_strings::format_upgrade_failed_retry_msg(
                    bin_name, &reason,
                ));
            Err(UpgradeError::Other(reason))
        }
        Err(err) if err.kind() == io::ErrorKind::Interrupted => {
            config
                .output_target
                .eprintln(ui_strings::format_upgrade_failed_retry_msg(
                    bin_name,
                    upgrade_reasons::CANCELLED_BY_USER,
                ));
            Err(UpgradeError::Cancelled)
        }
        Err(err) => {
            let reason = err.to_string();
            config
                .output_target
                .eprintln(ui_strings::format_upgrade_failed_retry_msg(
                    bin_name, &reason,
                ));
            Err(UpgradeError::Other(reason))
        }
    }
}

fn report_strategy_error(
    config: &UpgradeConfig,
    distribution: CrateDistribution,
    bin_name: &str,
    err: &UpgradeError,
) {
    match err {
        UpgradeError::NoViableUpgradeSource { .. } => {
            if distribution == CrateDistribution::OpenSource {
                config.output_target.eprintln(
                    ui_strings::format_open_source_no_toolchain_error(bin_name),
                );
            } else {
                config.output_target.eprintln(
                    ui_strings::format_closed_source_unreachable_error(bin_name),
                );
            }
        }
        other => {
            config
                .output_target
                .eprintln(ui_strings::format_upgrade_failed_retry_msg(
                    bin_name,
                    &other.to_string(),
                ));
        }
    }
}

/// Runs an on-demand upgrade (e.g., when invoked via `<binary> --upgrade`).
///
/// Queries `https://r3bl.com/versions.json` for the target version and executes the
/// upgrade engine.
///
/// # Errors
///
/// Returns an error if the version check or upgrade execution fails.
pub async fn run_upgrade(config: &UpgradeConfig) -> miette::Result<()> {
    let bin_name = get_current_bin_name();
    let metadata = match resolve_metadata_for_upgrade(config).await {
        Ok(meta) => meta,
        Err(err) => {
            config
                .output_target
                .eprintln(ui_strings::format_upgrade_failed_retry_msg(
                    &bin_name,
                    &err.to_string(),
                ));
            return Err(miette::miette!("{err}"));
        }
    };

    execute_upgrade_with_metadata(config, &metadata)
        .await
        .map_err(|e| miette::miette!("{e}"))?;
    ok!()
}

/// Synchronous wrapper around [`run_upgrade`] that runs on a lightweight
/// `current_thread` Tokio runtime.
///
/// Used by synchronous binaries (`rust-analyzer-mcp-server`, `env-source --upgrade`).
///
/// # Errors
///
/// Returns an error if the Tokio runtime cannot be built or if [`run_upgrade`] fails.
pub fn run_upgrade_sync(config: &UpgradeConfig) -> miette::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| miette::miette!("Failed to build Tokio runtime: {e}"))?;

    rt.block_on(run_upgrade(config))
}

/// CLI `--upgrade` flag interceptor for async binaries.
///
/// If `upgrade_flag` is `true`, executes [`run_upgrade`] and exits the process with code
/// `0` on success or `1` on failure. If `false`, returns immediately so normal program
/// execution continues.
pub async fn run_upgrade_cli_arg_interceptor(upgrade_flag: bool, config: &UpgradeConfig) {
    if !upgrade_flag {
        return;
    }
    match run_upgrade(config).await {
        Ok(()) => std::process::exit(0),
        Err(_) => std::process::exit(1),
    }
}

/// CLI `--upgrade` flag interceptor for synchronous binaries (`env-source`,
/// `rust-analyzer-mcp-server`).
///
/// If `upgrade_flag` is `true`, executes [`run_upgrade_sync`] and exits the process with
/// code `0` on success or `1` on failure.
pub fn run_upgrade_cli_arg_interceptor_sync(upgrade_flag: bool, config: &UpgradeConfig) {
    if !upgrade_flag {
        return;
    }
    match run_upgrade_sync(config) {
        Ok(()) => std::process::exit(0),
        Err(_) => std::process::exit(1),
    }
}

/// Handles pending upgrades at application exit based on [`UpgradeConfig::policy`]:
/// - [`UpgradePolicy::Never`]: Suppresses exit upgrade notifications and actions entirely.
/// - [`UpgradePolicy::NotifyOnly`]: Prints a one-line notice if an upgrade is available.
/// - [`UpgradePolicy::AutoUpgrade`]: Automatically runs the upgrade if an upgrade is
///   available.
/// - [`UpgradePolicy::InteractivePrompt`]: Prompts the user interactively (`choose()`)
///   whether to upgrade now.
pub async fn handle_upgrade_at_exit(config: &UpgradeConfig) {
    if config.policy == UpgradePolicy::Never || !is_upgrade_available() {
        return;
    }

    let Some(metadata) = get_cached_crate_metadata() else {
        return;
    };

    let bin_name = get_current_bin_name();

    match config.policy {
        UpgradePolicy::Never => {}
        UpgradePolicy::NotifyOnly => {
            config
                .output_target
                .println(ui_strings::format_upgrade_available_notice(
                    &bin_name,
                    config.local_version.as_str(),
                    metadata.version.as_str(),
                ));
        }
        UpgradePolicy::AutoUpgrade => {
            config
                .output_target
                .println(ui_strings::format_upgrade_available_notice(
                    &bin_name,
                    config.local_version.as_str(),
                    metadata.version.as_str(),
                ));
            drop(execute_upgrade_with_metadata(config, &metadata).await);
        }
        UpgradePolicy::InteractivePrompt => {
            if let TerminalInteractiveStatus::NotAvailable(_) =
                check_is_terminal_interactive()
            {
                return;
            }

            config
                .output_target
                .println(ui_strings::format_upgrade_available_notice(
                    &bin_name,
                    config.local_version.as_str(),
                    metadata.version.as_str(),
                ));

            let yes_no_options = &[
                ui_strings::yes_upgrade_now_raw(),
                ui_strings::no_thanks_raw(),
            ];
            let prompt_header = ui_strings::format_interactive_prompt_header(
                config.crate_name.as_str(),
                metadata.version.as_str(),
            );
            let header_lines = smallvec![cli_text_line![cli_text_inline(
                prompt_header,
                new_style!(bold)
            )]];
            let mut io = DefaultIoDevices::default();

            let maybe_user_choice = choose(
                header_lines,
                yes_no_options,
                Some(vp_height(yes_no_options.len())),
                None,
                HowToChoose::Single,
                StyleSheet::default(),
                io.as_mut_tuple(),
            )
            .get_first_result()
            .await
            .ok()
            .flatten();

            if let Some(user_choice) = maybe_user_choice
                && user_choice == yes_no_options[0]
            {
                drop(execute_upgrade_with_metadata(config, &metadata).await);
            }
        }
    }
}

/// Synchronous wrapper around [`handle_upgrade_at_exit`] for binaries that run without a
/// global Tokio runtime (`rust-analyzer-mcp-server`).
pub fn handle_upgrade_at_exit_sync(config: &UpgradeConfig) {
    if !is_upgrade_available() {
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

/// Helper for binaries that want to ensure the version check has completed before exit
/// (or run it if not yet checked).
pub async fn ensure_version_checked_and_handle_at_exit(config: &UpgradeConfig) {
    if get_cached_crate_metadata().is_none() {
        check_version_against_sot(config).await;
    }
    handle_upgrade_at_exit(config).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_eq2;
    use flate2::{Compression, write::GzEncoder};

    #[test]
    fn test_parse_sha256_checksum_text() {
        let sha_line = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  r3bl-cmdr-v0.0.25-x86_64-unknown-linux-gnu.tar.gz\n";
        assert_eq2!(
            parse_sha256_checksum_text(sha_line).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        let plain_hex =
            "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855";
        assert_eq2!(
            parse_sha256_checksum_text(plain_hex).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        assert!(parse_sha256_checksum_text("not_a_valid_sha").is_none());
        assert!(parse_sha256_checksum_text("").is_none());
    }

    #[test]
    fn test_compute_and_verify_file_sha256() {
        let temp_dir = try_create_temp_dir().unwrap();
        let file_path = temp_dir.join("empty.bin");
        fs::write(&file_path, b"").unwrap();

        // Known SHA-256 of empty byte array.
        let empty_sha =
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq2!(compute_sha256_hex(b""), empty_sha);
        assert!(verify_file_sha256(&file_path, empty_sha).is_ok());
        assert!(matches!(
            verify_file_sha256(
                &file_path,
                "0000000000000000000000000000000000000000000000000000000000000000"
            ),
            Err(UpgradeError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn test_extract_tar_gz_archive() {
        let temp_dir = try_create_temp_dir().unwrap();
        let archive_path = temp_dir.join("test_suite.tar.gz");

        {
            let tar_gz_file = File::create(&archive_path).unwrap();
            let enc = GzEncoder::new(tar_gz_file, Compression::default());
            let mut tar_builder = tar::Builder::new(enc);

            let data = b"#!/bin/sh\necho hello\n";
            let mut header = tar::Header::new_gnu();
            header.set_size(u64::try_from(data.len()).unwrap());
            header.set_mode(0o755);
            header.set_cksum();
            tar_builder
                .append_data(&mut header, "giti", &data[..])
                .unwrap();
            tar_builder.finish().unwrap();
        }

        let extract_dir = temp_dir.join("out");
        fs::create_dir_all(&extract_dir).unwrap();
        let extracted = extract_archive_files(&archive_path, &extract_dir).unwrap();
        assert_eq2!(extracted.len(), 1);
        assert!(extracted[0].ends_with("giti"));
        assert_eq2!(fs::read(&extracted[0]).unwrap(), b"#!/bin/sh\necho hello\n");
    }
}
