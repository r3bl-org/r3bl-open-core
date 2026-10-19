// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Centralized version check querying [`R3BL_VERSIONS_JSON_URL`].
//!
//! Compares versions strictly via `(major, minor, patch)` semantic version tuples
//! (`remote_version > local_version`) so local development builds ahead of
//! `versions.jsonc` are never downgraded.
//!
//! [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL

use super::{CrateVersion, CrateVersionMetadata, R3BL_VERSIONS_JSON_URL,
            UpgradeCheckResult, UpgradeConfig, UpgradePolicy, VersionsManifest};
use crate::{AtomicU8Ext as _, DeadlockPreventionPolicy::PanicOnSpecificLockNesting,
            SCRIPT_MOD_DEBUG, ScopedMutex, fg_magenta, ok, scoped_mutex,
            script::http_client};
use miette::IntoDiagnostic;
use std::{sync::{LazyLock, atomic::AtomicU8},
          time::Duration};

/// Network timeout for background version check requests.
const VERSION_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Global atomic state tracking the result of the background version check.
/// Initialized to `0` ([`UpgradeCheckResult::NotChecked`]).
pub static UPGRADE_CHECK_STATE: AtomicU8 = AtomicU8::new(0);

/// Thread-safe cache storing the resolved [`CrateVersionMetadata`] from
/// `r3bl.com/versions.jsonc`.
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

/// Returns a clone of the cached [`CrateVersionMetadata`] if a version check has
/// resolved it.
#[must_use]
pub fn get_cached_crate_metadata() -> Option<CrateVersionMetadata> {
    CACHED_CRATE_METADATA.read(Clone::clone)
}

/// Stores the resolved [`CrateVersionMetadata`] in the global cache.
pub fn set_cached_crate_metadata(metadata: CrateVersionMetadata) {
    CACHED_CRATE_METADATA.write(|slot| {
        *slot = Some(metadata);
    });
}

/// Strictly compares `remote_version` against `local_version` via [`CrateVersion`]'s
/// `SemVer` [`Ord`] implementation, returning `true` if and only if
/// `remote_version > local_version`.
///
/// If local development version is equal to or ahead of `remote_version`, returns `false`
/// to prevent accidental downgrades.
#[must_use]
pub fn is_remote_version_strictly_newer(
    remote_version: CrateVersion,
    local_version: CrateVersion,
) -> bool {
    remote_version > local_version
}

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
/// Returns an error if the HTTP request fails, times out, returns a non-2xx status, or if
/// the crate is missing from `versions.jsonc`.
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
/// Returns an error if the HTTP request fails, times out, returns a non-2xx status, or if
/// the crate is missing from the manifest.
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
    let response = tokio::time::timeout(VERSION_CHECK_TIMEOUT, client.get(url).send())
        .await
        .into_diagnostic()?
        .into_diagnostic()?;
    let response = response.error_for_status().into_diagnostic()?;
    let body_text = response.text().await.into_diagnostic()?;

    let metadata = parse_crate_metadata_from_versions_json_str(&body_text, crate_name)?;
    ok!(metadata)
}

/// Performs a version check against [`R3BL_VERSIONS_JSON_URL`] and updates
/// [`UPGRADE_CHECK_STATE`] and [`CACHED_CRATE_METADATA`].
///
/// # Error Handling
///
/// This check is opportunistic and fails silently when offline or when the manifest is
/// unreachable. See the [`upgrade`] module documentation for details on invocation modes
/// and error handling.
///
/// [`CACHED_CRATE_METADATA`]: crate::core::script::upgrade::CACHED_CRATE_METADATA
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
/// [`UPGRADE_CHECK_STATE`]: crate::core::script::upgrade::UPGRADE_CHECK_STATE
/// [`upgrade`]: mod@crate::core::script::upgrade
pub async fn check_version_against_manifest(
    config: &UpgradeConfig,
) -> UpgradeCheckResult {
    set_upgrade_check_result(UpgradeCheckResult::Checking);

    match try_get_crate_metadata_from_r3bl_versions_json(config.crate_name.as_str()).await
    {
        Ok(metadata) => {
            let is_newer =
                is_remote_version_strictly_newer(metadata.version, config.local_version);
            set_cached_crate_metadata(metadata.clone());

            let result = if is_newer {
                SCRIPT_MOD_DEBUG.then(|| {
                    tracing::info!(
                        message = "Newer version available in versions.jsonc",
                        crate_name = %config.crate_name,
                        local_version = %config.local_version,
                        remote_version = %metadata.version
                    );
                });
                UpgradeCheckResult::UpgradeAvailable
            } else {
                UpgradeCheckResult::UpToDate
            };
            set_upgrade_check_result(result);
            result
        }
        Err(err) => {
            SCRIPT_MOD_DEBUG.then(|| {
                tracing::debug!(
                    message = "Failed to check version against r3bl.com/versions.jsonc",
                    crate_name = %config.crate_name,
                    error = ?err
                );
            });
            set_upgrade_check_result(UpgradeCheckResult::FailedCheck);
            UpgradeCheckResult::FailedCheck
        }
    }
}

/// Spawns a non-blocking background Tokio task to check [`R3BL_VERSIONS_JSON_URL`]
/// for a newer version of `config.crate_name`.
///
/// If [`UpgradeConfig::policy`] is [`UpgradePolicy::Never`], this function returns
/// immediately without spawning any background task or making network requests.
///
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
pub fn start_background_version_check(config: UpgradeConfig) {
    if config.policy == UpgradePolicy::Never {
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
/// If [`UpgradeConfig::policy`] is [`UpgradePolicy::Never`], this function returns
/// immediately without spawning any thread or making network requests.
///
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
pub fn start_background_version_check_sync(config: UpgradeConfig) {
    if config.policy == UpgradePolicy::Never {
        return;
    }

    std::thread::spawn(move || {
        let Ok(rt) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            set_upgrade_check_result(UpgradeCheckResult::FailedCheck);
            return;
        };
        rt.block_on(async move {
            check_version_against_manifest(&config).await;
        });
    });
}

#[cfg(test)]
mod tests {
    use super::{super::CrateDistribution, *};
    use crate::assert_eq2;

    #[test]
    fn test_is_remote_version_strictly_newer() {
        // Strictly newer -> true
        assert!(is_remote_version_strictly_newer(
            CrateVersion::try_from("0.0.26").unwrap(),
            CrateVersion::try_from("0.0.25").unwrap(),
        ));
        assert!(is_remote_version_strictly_newer(
            CrateVersion::try_from("0.1.0").unwrap(),
            CrateVersion::try_from("0.0.99").unwrap(),
        ));
        assert!(is_remote_version_strictly_newer(
            CrateVersion::try_from("1.0.0").unwrap(),
            CrateVersion::try_from("0.9.9").unwrap(),
        ));

        // Equal -> false
        assert!(!is_remote_version_strictly_newer(
            CrateVersion::try_from("0.0.25").unwrap(),
            CrateVersion::try_from("0.0.25").unwrap(),
        ));

        // Local ahead of remote (dev version bumped) -> false (prevents downgrade!)
        assert!(!is_remote_version_strictly_newer(
            CrateVersion::try_from("0.0.25").unwrap(),
            CrateVersion::try_from("0.0.26").unwrap(),
        ));
        assert!(!is_remote_version_strictly_newer(
            CrateVersion::try_from("0.1.0").unwrap(),
            CrateVersion::try_from("0.2.0").unwrap(),
        ));
        assert!(!is_remote_version_strictly_newer(
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
                    "released_on": "2026-10-17T00:00:00Z",
                },
            }
        }"#;

        let cmdr =
            parse_crate_metadata_from_versions_json_str(jsonc, "r3bl-cmdr").unwrap();
        assert_eq2!(cmdr.version, CrateVersion::try_from("0.0.25").unwrap());
        assert_eq2!(cmdr.crate_distribution, CrateDistribution::OpenSource);

        let missing =
            parse_crate_metadata_from_versions_json_str(jsonc, "nonexistent-crate");
        assert!(missing.is_err());
    }
}
