// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Upgrade strategy resolution (`GithubReleaseArtifact` vs. open-source `crates.io`
//! fallback).

use super::{CrateDistribution, CrateName, CrateVersion, UpgradeError, UpgradeSource,
            check_github_release_asset_available, format_github_release_asset_urls,
            get_host_target_triple, toolchain_args, toolchain_binaries, upgrade_reasons};
use crate::CommandOutputResult;

/// Checks whether a binary command is available on `$PATH` by invoking `<cmd> --version`.
///
/// This cross-platform check works identically on Linux, macOS, and Windows without
/// depending on Unix-specific `which` or Windows-specific `where.exe`.
#[must_use]
pub fn is_toolchain_binary_on_path(binary_name: &str) -> bool {
    let result = std::process::Command::new(binary_name)
        .arg(toolchain_args::VERSION)
        .output();
    matches!(
        CommandOutputResult::from(result),
        CommandOutputResult::Success(_)
    )
}

/// Pure decision function that resolves the [`UpgradeSource`] given pre-evaluated
/// environment and network conditions.
///
/// # Errors
///
/// Returns [`UpgradeError::NoViableUpgradeSource`] if `github_asset_available` is `false`
/// and either `distribution` is [`CrateDistribution::ClosedSource`] or neither `rustup`
/// nor `cargo` is available.
pub fn resolve_upgrade_strategy_pure(
    crate_name: &str,
    pinned_version: &str,
    distribution: CrateDistribution,
    github_asset_urls: Option<(String, String)>,
    has_rustup: bool,
    has_cargo: bool,
) -> Result<UpgradeSource, UpgradeError> {
    if let Some((archive_url, sha256_url)) = github_asset_urls {
        return Ok(UpgradeSource::GithubReleaseArtifact {
            archive_url,
            sha256_url,
            pinned_version: CrateVersion::from(pinned_version),
        });
    }

    if distribution == CrateDistribution::ClosedSource {
        return Err(UpgradeError::NoViableUpgradeSource {
            crate_name: CrateName::from(crate_name),
            crate_distribution: CrateDistribution::ClosedSource,
            reason: upgrade_reasons::BINARY_RELEASE_UNREACHABLE.to_string(),
        });
    }

    // Open-source fallback: prefer rustup + cargo, then standalone cargo.
    resolve_source_fallback_pure(crate_name, pinned_version, has_rustup, has_cargo)
}

/// Resolves the open-source `crates.io` compilation fallback strategy (`rustup` + `cargo`
/// or standalone `cargo`).
///
/// # Errors
///
/// Returns [`UpgradeError::NoViableUpgradeSource`] if neither `rustup` nor `cargo` is on
/// `$PATH`.
pub fn resolve_source_fallback_pure(
    crate_name: &str,
    pinned_version: &str,
    has_rustup: bool,
    has_cargo: bool,
) -> Result<UpgradeSource, UpgradeError> {
    if has_rustup {
        Ok(UpgradeSource::BuildFromSourceRustup {
            pinned_version: CrateVersion::from(pinned_version),
        })
    } else if has_cargo {
        Ok(UpgradeSource::BuildFromSourceCargo {
            pinned_version: CrateVersion::from(pinned_version),
        })
    } else {
        Err(UpgradeError::NoViableUpgradeSource {
            crate_name: CrateName::from(crate_name),
            crate_distribution: CrateDistribution::OpenSource,
            reason: upgrade_reasons::NO_LOCAL_TOOLCHAIN.to_string(),
        })
    }
}

/// Resolves the best [`UpgradeSource`] for upgrading `crate_name` to `pinned_version`.
///
/// 1. Detects the host target triple and checks if a pre-compiled binary archive is
///    available on `https://github.com/r3bl-org/r3bl-binaries` via an HTTP `HEAD`
///    request.
/// 2. If the pre-compiled archive is available, returns
///    [`UpgradeSource::GithubReleaseArtifact`].
/// 3. If unavailable and `distribution == CrateDistribution::ClosedSource`, returns
///    `Err(UpgradeError::NoViableUpgradeSource)`.
/// 4. If unavailable and `distribution == CrateDistribution::OpenSource`, inspects
///    `$PATH` for `rustup` (returning [`UpgradeSource::BuildFromSourceRustup`]) or
///    `cargo` (returning [`UpgradeSource::BuildFromSourceCargo`]).
///
/// # Errors
///
/// Returns [`UpgradeError::NoViableUpgradeSource`] if no viable upgrade strategy exists.
pub async fn resolve_upgrade_strategy(
    crate_name: &str,
    pinned_version: &str,
    distribution: CrateDistribution,
) -> Result<UpgradeSource, UpgradeError> {
    let github_asset_urls = if let Ok(target_triple) = get_host_target_triple() {
        let (archive_url, sha256_url) =
            format_github_release_asset_urls(crate_name, pinned_version, target_triple);
        if check_github_release_asset_available(&archive_url).await {
            Some((archive_url, sha256_url))
        } else {
            None
        }
    } else {
        None
    };

    if github_asset_urls.is_some() {
        return resolve_upgrade_strategy_pure(
            crate_name,
            pinned_version,
            distribution,
            github_asset_urls,
            false,
            false,
        );
    }

    if distribution == CrateDistribution::ClosedSource {
        return resolve_upgrade_strategy_pure(
            crate_name,
            pinned_version,
            CrateDistribution::ClosedSource,
            None,
            false,
            false,
        );
    }

    let has_rustup = is_toolchain_binary_on_path(toolchain_binaries::RUSTUP);
    let has_cargo = is_toolchain_binary_on_path(toolchain_binaries::CARGO);

    resolve_upgrade_strategy_pure(
        crate_name,
        pinned_version,
        CrateDistribution::OpenSource,
        None,
        has_rustup,
        has_cargo,
    )
}

/// Resolves the open-source source compilation fallback (`rustup` or `cargo` on `$PATH`)
/// when a binary download fails mid-stream.
///
/// # Errors
///
/// Returns [`UpgradeError::NoViableUpgradeSource`] if `distribution` is
/// [`CrateDistribution::ClosedSource`] or neither `rustup` nor `cargo` is on `$PATH`.
pub fn resolve_source_fallback(
    crate_name: &str,
    pinned_version: &str,
    distribution: CrateDistribution,
) -> Result<UpgradeSource, UpgradeError> {
    if distribution == CrateDistribution::ClosedSource {
        return Err(UpgradeError::NoViableUpgradeSource {
            crate_name: CrateName::from(crate_name),
            crate_distribution: CrateDistribution::ClosedSource,
            reason: upgrade_reasons::BINARY_DOWNLOAD_FAILED.to_string(),
        });
    }

    let has_rustup = is_toolchain_binary_on_path(toolchain_binaries::RUSTUP);
    let has_cargo = is_toolchain_binary_on_path(toolchain_binaries::CARGO);
    resolve_source_fallback_pure(crate_name, pinned_version, has_rustup, has_cargo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_strategy_prefers_github_release_when_available() {
        let urls = (
            "https://example.com/archive.tar.gz".to_string(),
            "https://example.com/archive.tar.gz.sha256".to_string(),
        );

        // Open source with GitHub asset available -> GithubReleaseArtifact.
        let res_open = resolve_upgrade_strategy_pure(
            "r3bl-cmdr",
            "0.0.25",
            CrateDistribution::OpenSource,
            Some(urls.clone()),
            true,
            true,
        )
        .unwrap();
        assert_eq2!(
            res_open,
            UpgradeSource::GithubReleaseArtifact {
                archive_url: urls.0.clone(),
                sha256_url: urls.1.clone(),
                pinned_version: CrateVersion::from("0.0.25"),
            }
        );

        // Closed source with GitHub asset available -> GithubReleaseArtifact.
        let res_closed = resolve_upgrade_strategy_pure(
            "r3bl-closed",
            "1.0.0",
            CrateDistribution::ClosedSource,
            Some(urls.clone()),
            false,
            false,
        )
        .unwrap();
        assert_eq2!(
            res_closed,
            UpgradeSource::GithubReleaseArtifact {
                archive_url: urls.0,
                sha256_url: urls.1,
                pinned_version: CrateVersion::from("1.0.0"),
            }
        );
    }

    #[test]
    fn test_strategy_open_source_fallback_to_rustup_then_cargo() {
        // Both rustup and cargo on PATH -> BuildFromSourceRustup.
        let res_rustup = resolve_upgrade_strategy_pure(
            "r3bl-cmdr",
            "0.0.25",
            CrateDistribution::OpenSource,
            None,
            true,
            true,
        )
        .unwrap();
        assert_eq2!(
            res_rustup,
            UpgradeSource::BuildFromSourceRustup {
                pinned_version: CrateVersion::from("0.0.25"),
            }
        );

        // Only cargo on PATH (no rustup) -> BuildFromSourceCargo.
        let res_cargo = resolve_upgrade_strategy_pure(
            "r3bl-cmdr",
            "0.0.25",
            CrateDistribution::OpenSource,
            None,
            false,
            true,
        )
        .unwrap();
        assert_eq2!(
            res_cargo,
            UpgradeSource::BuildFromSourceCargo {
                pinned_version: CrateVersion::from("0.0.25"),
            }
        );

        // Neither rustup nor cargo -> Err(NoViableUpgradeSource).
        let res_none = resolve_upgrade_strategy_pure(
            "r3bl-cmdr",
            "0.0.25",
            CrateDistribution::OpenSource,
            None,
            false,
            false,
        );
        assert!(matches!(
            res_none,
            Err(UpgradeError::NoViableUpgradeSource {
                crate_distribution: CrateDistribution::OpenSource,
                ..
            })
        ));
    }

    #[test]
    fn test_strategy_closed_source_never_falls_back_to_cargo() {
        // Closed source even with rustup and cargo installed ->
        // Err(NoViableUpgradeSource).
        let res = resolve_upgrade_strategy_pure(
            "r3bl-closed",
            "1.0.0",
            CrateDistribution::ClosedSource,
            None,
            true,
            true,
        );
        assert!(matches!(
            res,
            Err(UpgradeError::NoViableUpgradeSource {
                crate_distribution: CrateDistribution::ClosedSource,
                ..
            })
        ));
    }
}
