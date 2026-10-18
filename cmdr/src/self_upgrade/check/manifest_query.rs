// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::super::{JsoncManifest, UpgradeStrategy, manifest_timeouts};
use crate::DEBUG_SELF_UPGRADE;
use miette::{Context, IntoDiagnostic};
use r3bl_tui::{create_client_with_user_agent, fg_magenta, ok};
use std::time::Duration;

impl UpgradeStrategy {
    /// Returns the manifest HTTP request timeout for this upgrade strategy
    /// ([`IMMEDIATE_MANIFEST_TIMEOUT`] for [`Self::Immediate`],
    /// [`DEDICATED_THREAD_MANIFEST_TIMEOUT`] for [`Self::DedicatedThread`]).
    ///
    /// [`DEDICATED_THREAD_MANIFEST_TIMEOUT`]: crate::self_upgrade::manifest_timeouts::DEDICATED_THREAD_MANIFEST_TIMEOUT
    /// [`IMMEDIATE_MANIFEST_TIMEOUT`]: crate::self_upgrade::manifest_timeouts::IMMEDIATE_MANIFEST_TIMEOUT
    #[must_use]
    pub const fn manifest_timeout(self) -> Duration {
        match self {
            Self::Immediate => manifest_timeouts::IMMEDIATE_MANIFEST_TIMEOUT,
            Self::DedicatedThread(_) => {
                manifest_timeouts::DEDICATED_THREAD_MANIFEST_TIMEOUT
            }
        }
    }
}

/// Fetches `versions.jsonc` from a URL (typically [`R3BL_VERSIONS_JSON_URL`]) within
/// `timeout` and parses it into a [`JsoncManifest`].
///
/// # Errors
///
/// Returns an error if the HTTP request fails, times out after `timeout`, returns a
/// non-2xx status, or if the manifest content cannot be parsed as valid JSONC.
///
/// [`JsoncManifest`]: crate::self_upgrade::JsoncManifest
/// [`R3BL_VERSIONS_JSON_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSON_URL
pub async fn try_get_jsonc_manifest_from_url(
    url: &str,
    timeout: Duration,
) -> miette::Result<JsoncManifest> {
    DEBUG_SELF_UPGRADE.then(|| {
        // % is Display, ? is Debug.
        tracing::debug!(
            message = "Fetching versions manifest",
            url = %fg_magenta(url),
            timeout_ms = timeout.as_millis()
        );
    });

    // Make the HTTP GET request to fetch the `versions.jsonc` file as a string.
    let http_client = create_client_with_user_agent(None)?;
    // Closure prevents having to chain `.into_diagnostic()?` multiple times.
    let fetch_fn = async || -> Result<String, reqwest::Error> {
        http_client
            .get(url)
            .timeout(timeout)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await
    };
    let text = fetch_fn()
        .await
        .into_diagnostic()
        .with_context(|| "Problem getting manifest")?;

    // Parse the `versions.jsonc` string into a JsoncManifest.
    let manifest = JsoncManifest::try_parse_jsonc(&text)?;

    ok!(manifest)
}

#[cfg(test)]
mod tests {
    use super::{super::super::UpgradeActionOnExit, *};
    use r3bl_tui::assert_eq2;

    #[test]
    fn test_upgrade_strategy_manifest_timeout() {
        assert_eq2!(
            UpgradeStrategy::Immediate.manifest_timeout(),
            manifest_timeouts::IMMEDIATE_MANIFEST_TIMEOUT
        );
        assert_eq2!(
            UpgradeStrategy::DedicatedThread(UpgradeActionOnExit::Auto)
                .manifest_timeout(),
            manifest_timeouts::DEDICATED_THREAD_MANIFEST_TIMEOUT
        );
        assert_eq2!(
            UpgradeStrategy::DedicatedThread(UpgradeActionOnExit::PromptToUpgrade)
                .manifest_timeout(),
            manifest_timeouts::DEDICATED_THREAD_MANIFEST_TIMEOUT
        );
        assert_eq2!(
            UpgradeStrategy::DedicatedThread(UpgradeActionOnExit::NotifyOnly)
                .manifest_timeout(),
            manifest_timeouts::DEDICATED_THREAD_MANIFEST_TIMEOUT
        );
    }

    #[tokio::test]
    async fn test_try_get_jsonc_manifest_from_invalid_url() {
        let result = try_get_jsonc_manifest_from_url(
            "http://127.0.0.1:0/nonexistent",
            manifest_timeouts::DEDICATED_THREAD_MANIFEST_TIMEOUT,
        )
        .await;
        assert!(result.is_err());
    }
}
