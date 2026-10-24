// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::{CrateVersion, CrateVersionMetadata, JsoncManifest,
                    R3BL_VERSIONS_JSON_URL},
            SelfBinaryUpgradeConfig, UpgradeCheckCache, UpgradeCheckStatus};
use crate::{SCRIPT_MOD_DEBUG, fg_magenta, ok, script::http_client};
use miette::IntoDiagnostic;
use std::time::Duration;

// -------------------------------------------------------------------------------------------------
// Check Engine
// -------------------------------------------------------------------------------------------------

/// Performs a version check against [`R3BL_VERSIONS_JSON_URL`] and updates
/// [`UpgradeCheckCache`].
///
/// # Error Handling
///
/// This check is opportunistic and fails silently when offline or when the manifest
/// is unreachable. See the [`self_update`] module documentation for details on
/// invocation modes and error handling.
///
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::self_update::R3BL_VERSIONS_JSON_URL
/// [`self_update`]: mod@crate::core::script::self_update
/// [`UpgradeCheckCache`]: crate::core::script::self_update::UpgradeCheckCache
pub async fn check_version_against_manifest(
    config: &SelfBinaryUpgradeConfig,
) -> UpgradeCheckStatus {
    UpgradeCheckCache::set(UpgradeCheckStatus::Checking);

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

            let state = if is_newer {
                SCRIPT_MOD_DEBUG.then(|| {
                    tracing::info!(
                        message = "Newer version available in versions.jsonc",
                        crate_name = %config.crate_name,
                        current_version = %config.current_version,
                        remote_version = %metadata.version
                    );
                });
                UpgradeCheckStatus::UpgradeAvailable(metadata)
            } else {
                UpgradeCheckStatus::UpToDate(metadata)
            };
            UpgradeCheckCache::set(state.clone());
            state
        }
        Err(err) => {
            SCRIPT_MOD_DEBUG.then(|| {
                tracing::debug!(
                    message = "Failed to check version against versions manifest",
                    crate_name = %config.crate_name,
                    url = %fg_magenta(R3BL_VERSIONS_JSON_URL),
                    error = ?err
                );
            });
            UpgradeCheckCache::set(UpgradeCheckStatus::CheckFailed);
            UpgradeCheckStatus::CheckFailed
        }
    }
}

/// Network query and manifest parsing for [`R3BL_VERSIONS_JSON_URL`].
///
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::self_update::R3BL_VERSIONS_JSON_URL
pub mod manifest_query {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Network timeout for background version check requests.
    const VERSION_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

    /// Parses a JSONC string matching the [`R3BL_VERSIONS_JSON_URL`] schema
    /// ([`JsoncManifest`]) and extracts the [`CrateVersionMetadata`] for `crate_name`.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSONC is malformed or if `crate_name` is not present in
    /// `crates`.
    ///
    /// [`CrateVersionMetadata`]: crate::core::script::self_update::CrateVersionMetadata
    /// [`JsoncManifest`]: crate::core::script::self_update::JsoncManifest
    /// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::self_update::R3BL_VERSIONS_JSON_URL
    pub fn parse_crate_metadata_from_versions_json_str(
        json_str: &str,
        crate_name: &str,
    ) -> miette::Result<CrateVersionMetadata> {
        let manifest = JsoncManifest::try_parse_jsonc(json_str)?;

        manifest.crates.get(crate_name).cloned().ok_or_else(|| {
            miette::miette!(
                "Crate '{}' not found in versions.jsonc (available: {:?})",
                crate_name,
                manifest.crates.keys().collect::<Vec<_>>()
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
    /// [`CrateVersionMetadata`]: crate::core::script::self_update::CrateVersionMetadata
    /// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::self_update::R3BL_VERSIONS_JSON_URL
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
    /// [`CrateVersionMetadata`]: crate::core::script::self_update::CrateVersionMetadata
    /// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::self_update::R3BL_VERSIONS_JSON_URL
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

/// Strict [`SemVer`] comparison ensuring local development builds are never downgraded.
///
/// [`SemVer`]: https://semver.org/
pub mod version_compare {
    #[allow(clippy::wildcard_imports)]
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
    use super::{super::super::CrateDistribution, *};
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
}
