// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Centralized version check for querying [`R3BL_VERSIONS_JSON_URL`].
//!
//! Compares versions strictly via `(major, minor, patch)` semantic version tuples
//! (`remote_version > current_version`) so local development builds ahead of
//! `versions.jsonc` are never downgraded.
//!
//! # Background Version Check & Cache Lifecycle
//!
//! To keep application startup instantaneous (0 ms user-perceived latency) while
//! eliminating redundant network round-trips at shutdown, the upgrade engine uses a
//! two-phase decoupled lifecycle coordinated through [`UPGRADE_CHECK_STATE`] and
//! [`CACHED_CRATE_METADATA`]:
//!
//! ```text
//! 1. Binary Startup (0 ms non-blocking):
//!    ├── start_background_version_check()      ──► Spawns Tokio task (for async binaries)
//!    └── start_background_version_check_sync() ──► Spawns dedicated OS thread (for sync binaries)
//!    Main thread immediately proceeds with user CLI/TUI logic
//!
//! 2. Concurrent Manifest Query:
//!    check_version_against_manifest() ───► HTTP GET versions.jsonc
//!    ├── Parses CrateVersionMetadata
//!    ├── Stores metadata in CACHED_CRATE_METADATA (set_cached_crate_metadata)
//!    └── Updates atomic state UPGRADE_CHECK_STATE (UpgradeAvailable | UpToDate | FailedCheck)
//!
//! 3. Binary Exit (0 ms in-memory read):
//!    handle_upgrade_at_exit()
//!    ├── Checks is_upgrade_available() (atomic load)
//!    └── Reads cached metadata via get_cached_crate_metadata() (no 2nd HTTP request!)
//! ```
//!
//! [`CACHED_CRATE_METADATA`]: crate::core::script::upgrade::version_check_cache::CACHED_CRATE_METADATA
//! [`CrateVersionMetadata`]: crate::core::script::upgrade::CrateVersionMetadata
//! [`get_cached_crate_metadata`]: crate::core::script::upgrade::version_check_cache::get_cached_crate_metadata
//! [`handle_upgrade_at_exit`]: crate::core::script::upgrade::handle_upgrade_at_exit
//! [`is_upgrade_available`]: crate::core::script::upgrade::version_check_cache::is_upgrade_available
//! [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
//! [`set_cached_crate_metadata`]: crate::core::script::upgrade::version_check_cache::set_cached_crate_metadata
//! [`start_background_version_check_sync`]: crate::core::script::upgrade::start_background_version_check_sync
//! [`start_background_version_check`]: crate::core::script::upgrade::start_background_version_check
//! [`UPGRADE_CHECK_STATE`]: crate::core::script::upgrade::version_check_cache::UPGRADE_CHECK_STATE

use super::{CrateVersion, CrateVersionMetadata, R3BL_VERSIONS_JSON_URL,
            SelfBinaryUpgradeConfig, UpdateBehavior, VersionsManifest};
use crate::{AtomicU8Ext as _, DeadlockPreventionPolicy::PanicOnSpecificLockNesting,
            SCRIPT_MOD_DEBUG, ScopedMutex, fg_magenta, ok, scoped_mutex,
            script::http_client};
use miette::IntoDiagnostic;
use std::{sync::{LazyLock, atomic::AtomicU8},
          time::Duration};
pub use version_check_cache::UpgradeCheckResult;

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
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
pub fn start_background_version_check(config: SelfBinaryUpgradeConfig) {
    if config.update_behavior == UpdateBehavior::Disable {
        return;
    }

    tokio::spawn(async move {
        check_engine::check_version_against_manifest(&config).await;
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
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
pub fn start_background_version_check_sync(config: SelfBinaryUpgradeConfig) {
    if config.update_behavior == UpdateBehavior::Disable {
        return;
    }

    std::thread::spawn(move || {
        let Ok(rt) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            version_check_cache::set_upgrade_check_result(
                UpgradeCheckResult::FailedCheck,
            );
            return;
        };
        rt.block_on(async move {
            check_engine::check_version_against_manifest(&config).await;
        });
    });
}

// -------------------------------------------------------------------------------------------------
// Inner Modules (Underlying Implementation & Domain Logic)
// -------------------------------------------------------------------------------------------------

/// Core version check engine coordinating manifest query, SemVer evaluation, and cache
/// updates.
pub mod check_engine {
    use super::*;

    /// Performs a version check against [`R3BL_VERSIONS_JSON_URL`] and updates
    /// [`UPGRADE_CHECK_STATE`] and [`CACHED_CRATE_METADATA`].
    ///
    /// # Error Handling
    ///
    /// This check is opportunistic and fails silently when offline or when the manifest
    /// is unreachable. See the [`upgrade`] module documentation for details on
    /// invocation modes and error handling.
    ///
    /// [`CACHED_CRATE_METADATA`]: crate::core::script::upgrade::version_check_cache::CACHED_CRATE_METADATA
    /// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
    /// [`UPGRADE_CHECK_STATE`]: crate::core::script::upgrade::version_check_cache::UPGRADE_CHECK_STATE
    /// [`upgrade`]: mod@crate::core::script::upgrade
    pub async fn check_version_against_manifest(
        config: &SelfBinaryUpgradeConfig,
    ) -> UpgradeCheckResult {
        version_check_cache::set_upgrade_check_result(UpgradeCheckResult::Checking);

        match manifest_query::try_get_crate_metadata_from_r3bl_versions_json(
            config.crate_name.as_str(),
        )
        .await
        {
            Ok(metadata) => {
                let is_newer = version_compare::is_remote_version_strictly_newer(
                    metadata.version,
                    config.current_version,
                );
                version_check_cache::set_cached_crate_metadata(metadata.clone());

                let result = if is_newer {
                    SCRIPT_MOD_DEBUG.then(|| {
                        tracing::info!(
                            message = "Newer version available in versions.jsonc",
                            crate_name = %config.crate_name,
                            current_version = %config.current_version,
                            remote_version = %metadata.version
                        );
                    });
                    UpgradeCheckResult::UpgradeAvailable
                } else {
                    UpgradeCheckResult::UpToDate
                };
                version_check_cache::set_upgrade_check_result(result);
                result
            }
            Err(err) => {
                SCRIPT_MOD_DEBUG.then(|| {
                    tracing::debug!(
                        message =
                            "Failed to check version against r3bl.com/versions.jsonc",
                        crate_name = %config.crate_name,
                        error = ?err
                    );
                });
                version_check_cache::set_upgrade_check_result(
                    UpgradeCheckResult::FailedCheck,
                );
                UpgradeCheckResult::FailedCheck
            }
        }
    }
}

/// In-memory cache and atomic status state for background version check results.
pub mod version_check_cache {
    use super::*;

    /// Atomic state representation of the background version check.
    ///
    /// Stored inside an [`AtomicU8`] and manipulated via [`AtomicU8Ext`].
    ///
    /// [`AtomicU8`]: std::sync::atomic::AtomicU8
    /// [`AtomicU8Ext`]: crate::AtomicU8Ext
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    pub enum UpgradeCheckResult {
        /// Background version check has not yet started.
        NotChecked = 0,

        /// Background version check is currently in flight.
        Checking = 1,

        /// Local binary version is greater than or equal to the remote `versions.jsonc`
        /// version.
        UpToDate = 2,

        /// Remote `versions.jsonc` version is strictly greater than local binary version.
        UpgradeAvailable = 3,

        /// Network or parsing error occurred while checking `versions.jsonc`.
        FailedCheck = 4,
    }

    mod impl_upgrade_check_result {
        #[allow(clippy::wildcard_imports)]
        use super::*;

        impl From<u8> for UpgradeCheckResult {
            fn from(value: u8) -> Self {
                match value {
                    0 => Self::NotChecked,
                    1 => Self::Checking,
                    2 => Self::UpToDate,
                    3 => Self::UpgradeAvailable,
                    _ => Self::FailedCheck,
                }
            }
        }

        impl From<UpgradeCheckResult> for u8 {
            fn from(value: UpgradeCheckResult) -> Self {
                match value {
                    UpgradeCheckResult::NotChecked => 0,
                    UpgradeCheckResult::Checking => 1,
                    UpgradeCheckResult::UpToDate => 2,
                    UpgradeCheckResult::UpgradeAvailable => 3,
                    UpgradeCheckResult::FailedCheck => 4,
                }
            }
        }
    }

    /// Global atomic state tracking the result of the background version check.
    /// Initialized to `0` ([`UpgradeCheckResult::NotChecked`]).
    pub static UPGRADE_CHECK_STATE: AtomicU8 = AtomicU8::new(0);

    /// Thread-safe in-memory cache storing the resolved [`CrateVersionMetadata`] from
    /// `r3bl.com/versions.jsonc`.
    ///
    /// # Purpose
    ///
    /// Bridges the gap between startup and shutdown without making redundant HTTP
    /// requests. When the background check completes,
    /// [`check_version_against_manifest`] stores the parsed manifest metadata here
    /// via [`set_cached_crate_metadata`].
    ///
    /// At application exit, [`handle_upgrade_at_exit`] reads the metadata via
    /// [`get_cached_crate_metadata`] in 0 ms without needing to re-fetch or re-parse
    /// `versions.jsonc`.
    ///
    /// [`check_version_against_manifest`]: crate::core::script::upgrade::check_engine::check_version_against_manifest
    /// [`get_cached_crate_metadata`]: crate::core::script::upgrade::version_check_cache::get_cached_crate_metadata
    /// [`handle_upgrade_at_exit`]: crate::core::script::upgrade::handle_upgrade_at_exit
    /// [`set_cached_crate_metadata`]: crate::core::script::upgrade::version_check_cache::set_cached_crate_metadata
    pub static CACHED_CRATE_METADATA: LazyLock<
        ScopedMutex<Option<CrateVersionMetadata>, { PanicOnSpecificLockNesting }>,
    > = LazyLock::new(|| scoped_mutex!(SPECIFIC, None));

    /// Returns the current [`UpgradeCheckResult`] from the atomic state.
    #[must_use]
    pub fn get_upgrade_check_result() -> UpgradeCheckResult {
        UpgradeCheckResult::from(UPGRADE_CHECK_STATE.get())
    }

    /// Sets the current [`UpgradeCheckResult`] in the atomic state.
    pub fn set_upgrade_check_result(result: UpgradeCheckResult) {
        UPGRADE_CHECK_STATE.set(result.into());
    }

    /// Returns `true` if a strictly newer version was found during the version check.
    #[must_use]
    pub fn is_upgrade_available() -> bool {
        get_upgrade_check_result() == UpgradeCheckResult::UpgradeAvailable
    }

    /// Returns a clone of the cached [`CrateVersionMetadata`] if the background version
    /// check has completed successfully.
    ///
    /// # Consumers
    ///
    /// This function is called by:
    /// - [`handle_upgrade_at_exit`]: to display version notices or execute an automatic
    ///   upgrade at shutdown without making a second network call.
    /// - [`resolve_metadata_for_upgrade`]: to reuse existing metadata before falling back
    ///   to a direct network query.
    ///
    /// Returns `None` if the background check is still in flight, has failed, or if
    /// update checks were disabled via [`UpdateBehavior::Disable`].
    ///
    /// [`handle_upgrade_at_exit`]: crate::core::script::upgrade::handle_upgrade_at_exit
    /// [`resolve_metadata_for_upgrade`]: crate::core::script::upgrade::metadata_resolver::resolve_metadata_for_upgrade
    /// [`UpdateBehavior::Disable`]: crate::core::script::upgrade::UpdateBehavior::Disable
    #[must_use]
    pub fn get_cached_crate_metadata() -> Option<CrateVersionMetadata> {
        CACHED_CRATE_METADATA.read(Clone::clone)
    }

    /// Stores the resolved [`CrateVersionMetadata`] in the global
    /// [`CACHED_CRATE_METADATA`].
    ///
    /// # Writers
    ///
    /// This function is called by:
    /// - [`check_version_against_manifest`]: upon successfully fetching and parsing
    ///   `r3bl.com/versions.jsonc`.
    /// - Unit tests: to deterministically inject mock remote version metadata without
    ///   requiring internet access or live HTTP requests.
    ///
    /// [`CACHED_CRATE_METADATA`]: crate::core::script::upgrade::version_check_cache::CACHED_CRATE_METADATA
    /// [`check_version_against_manifest`]: crate::core::script::upgrade::check_engine::check_version_against_manifest
    pub fn set_cached_crate_metadata(metadata: CrateVersionMetadata) {
        CACHED_CRATE_METADATA.write(|slot| {
            *slot = Some(metadata);
        });
    }
}

/// Network query and manifest parsing for `r3bl.com/versions.jsonc`.
pub mod manifest_query {
    use super::*;

    /// Network timeout for background version check requests.
    const VERSION_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

    /// Parses a JSONC string matching the `r3bl.com/versions.jsonc` schema
    /// ([`VersionsManifest`]) and extracts the [`CrateVersionMetadata`] for `crate_name`.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSONC is malformed or if `crate_name` is not present in
    /// `crates_map`.
    pub fn parse_crate_metadata_from_versions_json_str(
        json_str: &str,
        crate_name: &str,
    ) -> miette::Result<CrateVersionMetadata> {
        let manifest = VersionsManifest::try_parse_jsonc(json_str)?;

        manifest.crates_map.get(crate_name).cloned().ok_or_else(|| {
            miette::miette!(
                "Crate '{}' not found in versions.jsonc (available: {:?})",
                crate_name,
                manifest.crates_map.keys().collect::<Vec<_>>()
            )
        })
    }

    /// Fetches [`R3BL_VERSIONS_JSON_URL`] and extracts the [`CrateVersionMetadata`]
    /// for `crate_name`.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails, times out, returns a non-2xx status,
    /// or if the crate is missing from `versions.jsonc`.
    ///
    /// [`CrateVersionMetadata`]: crate::core::script::upgrade::CrateVersionMetadata
    /// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
    pub async fn try_get_crate_metadata_from_r3bl_versions_json(
        crate_name: &str,
    ) -> miette::Result<CrateVersionMetadata> {
        try_get_crate_metadata_from_url(R3BL_VERSIONS_JSON_URL, crate_name).await
    }

    /// Fetches `versions.jsonc` from a custom URL (or [`R3BL_VERSIONS_JSON_URL`]) and
    /// extracts the [`CrateVersionMetadata`] for `crate_name`.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails, times out, returns a non-2xx status,
    /// or if the crate is missing from the manifest.
    ///
    /// [`CrateVersionMetadata`]: crate::core::script::upgrade::CrateVersionMetadata
    /// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
    pub async fn try_get_crate_metadata_from_url(
        url: &str,
        crate_name: &str,
    ) -> miette::Result<CrateVersionMetadata> {
        SCRIPT_MOD_DEBUG.then(|| {
            tracing::debug!(
                message = "Fetching crate version metadata",
                crate_name = %crate_name,
                url = %fg_magenta(url)
            );
        });

        let client = http_client::create_client_with_user_agent(None)?;
        let response =
            tokio::time::timeout(VERSION_CHECK_TIMEOUT, client.get(url).send())
                .await
                .into_diagnostic()?
                .into_diagnostic()?;
        let response = response.error_for_status().into_diagnostic()?;
        let body_text = response.text().await.into_diagnostic()?;

        let metadata =
            parse_crate_metadata_from_versions_json_str(&body_text, crate_name)?;
        ok!(metadata)
    }
}

/// Strict SemVer comparison ensuring local development builds are never downgraded.
pub mod version_compare {
    use super::*;

    /// Strictly compares `remote_version` against `current_version` via the
    /// [`CrateVersion`] `SemVer` [`Ord`] implementation, returning `true` if and only
    /// if `remote_version > current_version`.
    ///
    /// If current version is equal to or ahead of `remote_version`, returns `false`
    /// to prevent accidental downgrades.
    #[must_use]
    pub fn is_remote_version_strictly_newer(
        remote_version: CrateVersion,
        current_version: CrateVersion,
    ) -> bool {
        remote_version > current_version
    }
}

#[cfg(test)]
mod tests {
    use super::{super::CrateDistribution, *};
    use crate::assert_eq2;

    #[test]
    fn test_is_remote_version_strictly_newer() {
        // Strictly newer -> true
        assert!(version_compare::is_remote_version_strictly_newer(
            CrateVersion::try_from("0.0.26").unwrap(),
            CrateVersion::try_from("0.0.25").unwrap(),
        ));
        assert!(version_compare::is_remote_version_strictly_newer(
            CrateVersion::try_from("0.1.0").unwrap(),
            CrateVersion::try_from("0.0.99").unwrap(),
        ));
        assert!(version_compare::is_remote_version_strictly_newer(
            CrateVersion::try_from("1.0.0").unwrap(),
            CrateVersion::try_from("0.9.9").unwrap(),
        ));

        // Equal -> false
        assert!(!version_compare::is_remote_version_strictly_newer(
            CrateVersion::try_from("0.0.25").unwrap(),
            CrateVersion::try_from("0.0.25").unwrap(),
        ));

        // Current version ahead of remote (dev version bumped) -> false (prevents
        // downgrade!)
        assert!(!version_compare::is_remote_version_strictly_newer(
            CrateVersion::try_from("0.0.25").unwrap(),
            CrateVersion::try_from("0.0.26").unwrap(),
        ));
        assert!(!version_compare::is_remote_version_strictly_newer(
            CrateVersion::try_from("0.1.0").unwrap(),
            CrateVersion::try_from("0.2.0").unwrap(),
        ));
        assert!(!version_compare::is_remote_version_strictly_newer(
            CrateVersion::try_from("1.0.0").unwrap(),
            CrateVersion::try_from("2.0.0").unwrap(),
        ));
    }

    #[test]
    fn test_parse_crate_metadata_from_versions_json_str() {
        let jsonc = r#"{
            // R3BL versions.jsonc manifest
            "schema_version": 1,
            "crates": {
                "r3bl-cmdr": {
                    "version": "0.0.25",
                    "crate_distribution": "open-source",
                    "released_on": "2026-10-17T00:00:00Z"
                },
                "r3bl-build-infra": {
                    "version": "0.1.0",
                    "crate_distribution": "open-source",
                    "released_on": "2026-10-17T00:00:00Z"
                },
                "r3bl-rust-analyzer-mcp-server": {
                    "version": "0.1.0",
                    "crate_distribution": "open-source",
                    "released_on": "2026-10-17T00:00:00Z"
                },
            }
        }"#;

        let cmdr = manifest_query::parse_crate_metadata_from_versions_json_str(
            jsonc,
            "r3bl-cmdr",
        )
        .unwrap();
        assert_eq2!(cmdr.version, CrateVersion::try_from("0.0.25").unwrap());
        assert_eq2!(cmdr.crate_distribution, CrateDistribution::OpenSource);

        let missing = manifest_query::parse_crate_metadata_from_versions_json_str(
            jsonc,
            "nonexistent-crate",
        );
        assert!(missing.is_err());
    }

    #[test]
    fn test_upgrade_check_result_roundtrip() {
        let variants = [
            UpgradeCheckResult::NotChecked,
            UpgradeCheckResult::Checking,
            UpgradeCheckResult::UpToDate,
            UpgradeCheckResult::UpgradeAvailable,
            UpgradeCheckResult::FailedCheck,
        ];

        for variant in variants {
            let byte: u8 = variant.into();
            let converted = UpgradeCheckResult::from(byte);
            assert_eq2!(variant, converted);
        }

        // Unknown u8 maps to FailedCheck.
        assert_eq2!(
            UpgradeCheckResult::from(99),
            UpgradeCheckResult::FailedCheck
        );
    }

    #[test]
    fn test_start_background_version_check_disabled() {
        version_check_cache::set_upgrade_check_result(UpgradeCheckResult::NotChecked);
        let config = SelfBinaryUpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_from("0.0.25").unwrap(),
            Some(UpdateBehavior::Disable),
            super::super::UserFeedbackChannel::Quiet,
        );
        start_background_version_check(config);
        assert_eq2!(
            version_check_cache::get_upgrade_check_result(),
            UpgradeCheckResult::NotChecked
        );
    }

    #[test]
    fn test_start_background_version_check_sync_disabled() {
        version_check_cache::set_upgrade_check_result(UpgradeCheckResult::NotChecked);
        let config = SelfBinaryUpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_from("0.0.25").unwrap(),
            Some(UpdateBehavior::Disable),
            super::super::UserFeedbackChannel::Quiet,
        );
        start_background_version_check_sync(config);
        assert_eq2!(
            version_check_cache::get_upgrade_check_result(),
            UpgradeCheckResult::NotChecked
        );
    }
}
