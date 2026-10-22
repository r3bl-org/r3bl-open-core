// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CrateDistribution, CrateName, CrateVersion, UpgradeError,
            check_github_release_asset_available, format_github_release_asset_urls,
            get_host_target_triple, toolchain_bin, toolchain_cli_args, upgrade_reasons};
use crate::CommandOutputResult;

/// Resolved execution source for performing the upgrade.
///
/// "Resolved" signifies that the upgrade engine has evaluated runtime environment
/// conditions (host target architecture, remote asset availability on GitHub, crate
/// distribution model, and local Rust toolchain availability on `$PATH`) to determine
/// the exact execution path before initiating downloads or builds.
///
/// Evaluated by [`UpgradeSource::try_resolve`] prior to calling
/// [`execute_upgrade_with_metadata`].
///
/// [`execute_upgrade_with_metadata`]: crate::core::script::upgrade::upgrade_execution::execute_upgrade_with_metadata
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpgradeSource {
    /// Pre-compiled binary archive hosted on [`R3BL_BINARIES_RELEASE_BASE_URL`].
    ///
    /// [`R3BL_BINARIES_RELEASE_BASE_URL`]: crate::core::script::upgrade::R3BL_BINARIES_RELEASE_BASE_URL
    GithubReleaseArtifact {
        /// Direct download URL for the `.tar.gz` archive.
        archive_url: String,

        /// Direct download URL for the `.sha256` checksum file.
        sha256_url: String,

        /// Pinned target version string from [`R3BL_VERSIONS_JSON_URL`].
        ///
        /// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
        pinned_version: CrateVersion,
    },

    /// Build from `crates.io` source using `rustup toolchain install nightly --force`
    /// followed by `cargo +nightly install <crate> --version <pinned_version>`.
    /// Only eligible when [`CrateDistribution::OpenSource`].
    ///
    /// [`CrateDistribution::OpenSource`]: super::CrateDistribution::OpenSource
    /// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
    BuildFromSourceRustup {
        /// Pinned target version string from [`R3BL_VERSIONS_JSON_URL`].
        ///
        /// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
        pinned_version: CrateVersion,
    },
}

mod impl_upgrade_source {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl UpgradeSource {
        /// Resolves the best [`UpgradeSource`] for upgrading `crate_name` to
        /// `pinned_version`.
        ///
        /// 1. Detects the host target triple and checks if a pre-compiled binary archive is
        ///    available on `https://github.com/r3bl-org/r3bl-binaries` via an HTTP `HEAD`
        ///    request.
        /// 2. If the pre-compiled archive is available, returns
        ///    [`UpgradeSource::GithubReleaseArtifact`].
        /// 3. If unavailable and `distribution == CrateDistribution::ClosedSource`,
        ///    returns `Err(UpgradeError::NoViableUpgradeSource)`.
        /// 4. If unavailable and `distribution == CrateDistribution::OpenSource`,
        ///    inspects `$PATH` for `rustup` (returning
        ///    [`UpgradeSource::BuildFromSourceRustup`]).
        ///
        /// # Errors
        ///
        /// Returns [`UpgradeError::NoViableUpgradeSource`] if no viable upgrade strategy
        /// exists.
        pub async fn try_resolve(
            crate_name: &CrateName,
            pinned_version: CrateVersion,
            distribution: CrateDistribution,
        ) -> Result<Self, UpgradeError> {
            let pinned_version_str = pinned_version.to_string();
            let github_asset_urls = if let Ok(target_triple) = get_host_target_triple() {
                let (archive_url, sha256_url) = format_github_release_asset_urls(
                    crate_name.as_str(),
                    &pinned_version_str,
                    target_triple,
                );
                if check_github_release_asset_available(&archive_url).await {
                    Some((archive_url, sha256_url))
                } else {
                    None
                }
            } else {
                None
            };

            if github_asset_urls.is_some() {
                return pure_evaluator::evaluate_upgrade_source_pure(
                    crate_name,
                    pinned_version,
                    distribution,
                    github_asset_urls,
                    false,
                );
            }

            if distribution == CrateDistribution::ClosedSource {
                return pure_evaluator::evaluate_upgrade_source_pure(
                    crate_name,
                    pinned_version,
                    CrateDistribution::ClosedSource,
                    None,
                    false,
                );
            }

            let has_rustup =
                toolchain_probe::is_toolchain_binary_on_path(toolchain_bin::RUSTUP);

            pure_evaluator::evaluate_upgrade_source_pure(
                crate_name,
                pinned_version,
                CrateDistribution::OpenSource,
                None,
                has_rustup,
            )
        }

        /// Resolves the open-source source compilation fallback (`rustup` on `$PATH`)
        /// when a binary download fails mid-stream.
        ///
        /// # Errors
        ///
        /// Returns [`UpgradeError::NoViableUpgradeSource`] if `distribution` is
        /// [`CrateDistribution::ClosedSource`] or `rustup` is not on `$PATH`.
        pub fn try_fallback_to_source(
            crate_name: &CrateName,
            pinned_version: CrateVersion,
            distribution: CrateDistribution,
        ) -> Result<Self, UpgradeError> {
            if distribution == CrateDistribution::ClosedSource {
                return Err(UpgradeError::NoViableUpgradeSource {
                    crate_name: crate_name.clone(),
                    crate_distribution: CrateDistribution::ClosedSource,
                    reason: upgrade_reasons::BINARY_DOWNLOAD_FAILED.to_string(),
                });
            }

            let has_rustup =
                toolchain_probe::is_toolchain_binary_on_path(toolchain_bin::RUSTUP);
            pure_evaluator::evaluate_source_fallback_pure(
                crate_name,
                pinned_version,
                has_rustup,
            )
        }
    }
}

/// Pure decision functions that evaluate [`UpgradeSource`] without performing network
/// calls or process executions.
pub mod pure_evaluator {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Pure decision function that resolves the [`UpgradeSource`] given pre-evaluated
    /// environment and network conditions.
    ///
    /// # Errors
    ///
    /// Returns [`UpgradeError::NoViableUpgradeSource`] if `github_asset_urls` is `None`
    /// and either `distribution` is [`CrateDistribution::ClosedSource`] or `rustup` is
    /// unavailable.
    pub fn evaluate_upgrade_source_pure(
        crate_name: &CrateName,
        pinned_version: CrateVersion,
        distribution: CrateDistribution,
        github_asset_urls: Option<(String, String)>,
        has_rustup: bool,
    ) -> Result<UpgradeSource, UpgradeError> {
        if let Some((archive_url, sha256_url)) = github_asset_urls {
            return Ok(UpgradeSource::GithubReleaseArtifact {
                archive_url,
                sha256_url,
                pinned_version,
            });
        }

        if distribution == CrateDistribution::ClosedSource {
            return Err(UpgradeError::NoViableUpgradeSource {
                crate_name: crate_name.clone(),
                crate_distribution: CrateDistribution::ClosedSource,
                reason: upgrade_reasons::BINARY_RELEASE_UNREACHABLE.to_string(),
            });
        }

        // Open-source fallback: requires rustup to install and execute the nightly
        // toolchain.
        evaluate_source_fallback_pure(crate_name, pinned_version, has_rustup)
    }

    /// Resolves the open-source `crates.io` compilation fallback strategy (`rustup`).
    ///
    /// # Errors
    ///
    /// Returns [`UpgradeError::NoViableUpgradeSource`] if `rustup` is not on `$PATH`.
    pub fn evaluate_source_fallback_pure(
        crate_name: &CrateName,
        pinned_version: CrateVersion,
        has_rustup: bool,
    ) -> Result<UpgradeSource, UpgradeError> {
        if has_rustup {
            Ok(UpgradeSource::BuildFromSourceRustup { pinned_version })
        } else {
            Err(UpgradeError::NoViableUpgradeSource {
                crate_name: crate_name.clone(),
                crate_distribution: CrateDistribution::OpenSource,
                reason: upgrade_reasons::NO_LOCAL_TOOLCHAIN.to_string(),
            })
        }
    }
}

/// Host toolchain and binary detection helpers.
pub mod toolchain_probe {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Checks whether a binary command is available on `$PATH` by invoking `<cmd>
    /// --version`.
    ///
    /// This cross-platform check works identically on Linux, macOS, and Windows without
    /// depending on Unix-specific `which` or Windows-specific `where.exe`.
    #[must_use]
    pub fn is_toolchain_binary_on_path(binary_name: &str) -> bool {
        let result = std::process::Command::new(binary_name)
            .arg(toolchain_cli_args::VERSION)
            .output();
        matches!(
            CommandOutputResult::from(result),
            CommandOutputResult::Success(_)
        )
    }
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
        let v_0_0_25 = CrateVersion::try_from("0.0.25").unwrap();
        let v_1_0_0 = CrateVersion::try_from("1.0.0").unwrap();
        let cmdr_name = CrateName::from("r3bl-cmdr");
        let closed_name = CrateName::from("r3bl-closed");

        // Open source with GitHub asset available -> GithubReleaseArtifact.
        let res_open = pure_evaluator::evaluate_upgrade_source_pure(
            &cmdr_name,
            v_0_0_25,
            CrateDistribution::OpenSource,
            Some(urls.clone()),
            true,
        )
        .unwrap();
        assert_eq2!(
            res_open,
            UpgradeSource::GithubReleaseArtifact {
                archive_url: urls.0.clone(),
                sha256_url: urls.1.clone(),
                pinned_version: v_0_0_25,
            }
        );

        // Closed source with GitHub asset available -> GithubReleaseArtifact.
        let res_closed = pure_evaluator::evaluate_upgrade_source_pure(
            &closed_name,
            v_1_0_0,
            CrateDistribution::ClosedSource,
            Some(urls.clone()),
            false,
        )
        .unwrap();
        assert_eq2!(
            res_closed,
            UpgradeSource::GithubReleaseArtifact {
                archive_url: urls.0,
                sha256_url: urls.1,
                pinned_version: v_1_0_0,
            }
        );
    }

    #[test]
    fn test_strategy_open_source_fallback_to_rustup() {
        let v_0_0_25 = CrateVersion::try_from("0.0.25").unwrap();
        let cmdr_name = CrateName::from("r3bl-cmdr");

        // rustup on PATH -> BuildFromSourceRustup.
        let res_rustup = pure_evaluator::evaluate_upgrade_source_pure(
            &cmdr_name,
            v_0_0_25,
            CrateDistribution::OpenSource,
            None,
            true,
        )
        .unwrap();
        assert_eq2!(
            res_rustup,
            UpgradeSource::BuildFromSourceRustup {
                pinned_version: v_0_0_25,
            }
        );

        // No rustup on PATH -> Err(NoViableUpgradeSource).
        let res_none = pure_evaluator::evaluate_upgrade_source_pure(
            &cmdr_name,
            v_0_0_25,
            CrateDistribution::OpenSource,
            None,
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
    fn test_strategy_closed_source_never_falls_back_to_source() {
        let closed_name = CrateName::from("r3bl-closed");

        // Closed source even with rustup installed -> Err(NoViableUpgradeSource).
        let res = pure_evaluator::evaluate_upgrade_source_pure(
            &closed_name,
            CrateVersion::try_from("1.0.0").unwrap(),
            CrateDistribution::ClosedSource,
            None,
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
