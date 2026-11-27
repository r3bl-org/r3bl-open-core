// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::super::{CrateMetadata, CrateName, EndpointUrl, JsoncManifest,
                   JsoncParseError, VERSIONS_JSONC_FILE};
use crate::DEBUG_SELF_UPGRADE;
use const_format::formatcp;
use r3bl_tui::{HttpClientError, create_http_client, fg_magenta, ok};
use std::time::Duration;

/// Fetches [`VERSIONS_JSONC_FILE`] from `manifest_url` within `timeout` duration and
/// extracts [`FetchManifestOutput::metadata`] and
/// [`FetchManifestOutput::binaries_release_base_url`] for the given `crate_name`.
///
/// This function is called by the [`Checking`] typestate in the [Control Plane] to
/// resolve the transition into [`UpToDate`], [`UpgradeAvailable`], [`Upgrading`], or
/// [`CheckFailed`].
///
/// This function is part of the [Execution Plane]. While it performs its duties, if it
/// encounters any fetch, parse, or lookup failures, then it logs them via
/// [`DEBUG_SELF_UPGRADE`]. It never imports or touches [`UpgradeJourney`].
///
/// # Errors
///
/// 1. Returns [`FetchManifestError::HttpClientCreateFailed`] if building the HTTP client
///    fails.
/// 2. Returns [`FetchManifestError::HttpRequestFailed`] if the HTTP request fails, times
///    out after `timeout` duration, returns a non-2xx status, or fails reading the
///    response body.
/// 3. Returns [`FetchManifestError::JsoncParseFailed`] if the manifest body fails JSONC
///    syntax or schema parsing.
/// 4. Returns [`FetchManifestError::CrateNotFoundInManifest`] if `crate_name` is absent
///    from [`JsoncManifest::crates`].
///
/// [`CheckFailed`]: crate::self_upgrade::CheckFailed
/// [`Checking`]: crate::self_upgrade::Checking
/// [`DEBUG_SELF_UPGRADE`]: crate::DEBUG_SELF_UPGRADE
/// [`JsoncManifest::crates`]: crate::self_upgrade::JsoncManifest::crates
/// [`UpgradeAvailable`]: crate::self_upgrade::UpgradeAvailable
/// [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
/// [`Upgrading`]: crate::self_upgrade::Upgrading
/// [`UpToDate`]: crate::self_upgrade::UpToDate
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [Execution Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
pub async fn try_fetch_manifest(
    manifest_url: &EndpointUrl,
    crate_name: &CrateName,
    timeout: Duration,
) -> Result<FetchManifestOutput, FetchManifestError> {
    let manifest =
        http_req_helpers::try_get_jsonc_manifest_from_url(manifest_url, timeout).await?;

    http_req_helpers::try_get_crate_metadata_from_jsonc_manifest(
        manifest,
        crate_name,
        manifest_url,
    )
}

/// Extracted crate metadata and shared release base URL returned by
/// [`try_fetch_manifest`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchManifestOutput {
    /// Parsed version, distribution, and release date for the queried crate.
    pub metadata: CrateMetadata,

    /// Top-level [`JsoncManifest::binaries_release_base_url`][1] shared by all crates in
    /// [`VERSIONS_JSONC_FILE`].
    ///
    /// [1]: crate::self_upgrade::JsoncManifest::binaries_release_base_url
    pub binaries_release_base_url: EndpointUrl,
}

/// Execution Plane failure returned by [`try_fetch_manifest`] when fetching, parsing, or
/// looking up a crate in [`VERSIONS_JSONC_FILE`] fails.
///
/// Wrapped by [`CheckFailedError::FetchManifest`][1] in the [Control Plane].
///
/// [1]: crate::self_upgrade::CheckFailedError::FetchManifest
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum FetchManifestError {
    /// Failed to build the [`reqwest::Client`] (for example, TLS backend initialization
    /// failed).
    #[error("Failed to create HTTP client for manifest '{manifest_url}': {src_err}")]
    HttpClientCreateFailed {
        /// Manifest endpoint URL targeted by the check.
        manifest_url: EndpointUrl,

        /// Underlying HTTP client initialization error.
        #[source]
        #[diagnostic_source]
        src_err: HttpClientError,
    },

    /// Network request, timeout, non-2xx HTTP status, or body read failed when fetching
    /// [`VERSIONS_JSONC_FILE`].
    #[error("Failed to fetch manifest from '{manifest_url}': {src_err}")]
    HttpRequestFailed {
        /// Manifest endpoint URL that was requested.
        manifest_url: EndpointUrl,

        /// Underlying [`reqwest::Error`] (preserves `.is_timeout()`, `.status()`,
        /// `.is_connect()`, etc.).
        #[source]
        src_err: reqwest::Error,
    },

    /// Downloaded manifest text was not valid JSONC or did not match the
    /// [`JsoncManifest`] schema.
    #[error("Failed to parse manifest from '{manifest_url}': {src_err}")]
    JsoncParseFailed {
        /// Manifest endpoint URL whose payload failed to parse.
        manifest_url: EndpointUrl,

        /// Underlying [`JsoncParseError`] (syntax vs. schema failure).
        #[source]
        #[diagnostic_source]
        src_err: JsoncParseError,
    },

    /// Target crate was not present in the remote [`VERSIONS_JSONC_FILE`] manifest's
    /// [`CRATES`] map ([`JsoncManifest::crates`]).
    ///
    /// [`CRATES`]: crate::self_upgrade::versions_manifest_keys::CRATES
    /// [`JsoncManifest::crates`]: crate::self_upgrade::JsoncManifest::crates
    #[error("Crate '{crate_name}' not found in manifest at '{manifest_url}'")]
    CrateNotFoundInManifest {
        /// Queried crate name.
        crate_name: CrateName,

        /// Manifest endpoint URL where `crate_name` was looked up.
        manifest_url: EndpointUrl,
    },
}

/// Private step helpers for [`try_fetch_manifest`] (fetch/parse and crate metadata
/// extraction), kept separate for direct unit testing.
mod http_req_helpers {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Fetches [`VERSIONS_JSONC_FILE`] from `url` (typically
    /// [`EndpointUrl::default_manifest_url`][1], backed by [`R3BL_VERSIONS_JSONC_URL`])
    /// within `timeout` duration and parses it into a [`JsoncManifest`].
    ///
    /// # Errors
    ///
    /// Returns [`FetchManifestError::HttpClientCreateFailed`],
    /// [`FetchManifestError::HttpRequestFailed`], or
    /// [`FetchManifestError::JsoncParseFailed`] if client creation, the HTTP request,
    /// timeout, status check, or JSONC parsing fails.
    ///
    /// [1]: crate::self_upgrade::EndpointUrl::default_manifest_url
    /// [`JsoncManifest`]: crate::self_upgrade::JsoncManifest
    /// [`R3BL_VERSIONS_JSONC_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSONC_URL
    pub async fn try_get_jsonc_manifest_from_url(
        url: &EndpointUrl,
        timeout: Duration,
    ) -> Result<JsoncManifest, FetchManifestError> {
        let url_str: &str = url;

        DEBUG_SELF_UPGRADE.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message = formatcp!("Fetching {VERSIONS_JSONC_FILE}"),
                url = %fg_magenta(url_str),
                timeout_ms = timeout.as_millis()
            );
        });

        let res_manifest: Result<JsoncManifest, FetchManifestError> = async {
            // Make the HTTP GET request to fetch the manifest file as a string.
            let http_client = create_http_client().map_err(|src_err| {
                FetchManifestError::HttpClientCreateFailed {
                    manifest_url: url.clone(),
                    src_err,
                }
            })?;

            // This closure groups the request, status check, and body read into one
            // `reqwest::Error` boundary.
            let async_fetch_fn = async || -> Result<String, reqwest::Error> {
                http_client
                    .get(url_str)
                    .timeout(timeout)
                    .send()
                    .await?
                    .error_for_status()?
                    .text()
                    .await
            };
            let resp_text = async_fetch_fn().await.map_err(|src_err| {
                FetchManifestError::HttpRequestFailed {
                    manifest_url: url.clone(),
                    src_err,
                }
            })?;

            // Parse the manifest string into a JsoncManifest.
            let manifest =
                JsoncManifest::try_parse_jsonc(&resp_text).map_err(|src_err| {
                    FetchManifestError::JsoncParseFailed {
                        manifest_url: url.clone(),
                        src_err,
                    }
                })?;

            ok!(manifest)
        }
        .await;

        res_manifest.inspect_err(|error| {
            DEBUG_SELF_UPGRADE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = formatcp!("Failed to fetch or parse {VERSIONS_JSONC_FILE}"),
                    url = %fg_magenta(url_str),
                    error = ?error
                );
            });
        })
    }

    /// Extracts [`FetchManifestOutput`] for `crate_name` from an already-fetched
    /// [`JsoncManifest`].
    ///
    /// # Errors
    ///
    /// Returns [`FetchManifestError::CrateNotFoundInManifest`] if `crate_name` is absent
    /// from [`JsoncManifest::crates`].
    ///
    /// [`JsoncManifest::crates`]: crate::self_upgrade::JsoncManifest::crates
    /// [`JsoncManifest`]: crate::self_upgrade::JsoncManifest
    pub fn try_get_crate_metadata_from_jsonc_manifest(
        manifest: JsoncManifest,
        crate_name: &CrateName,
        manifest_url: &EndpointUrl,
    ) -> Result<FetchManifestOutput, FetchManifestError> {
        let metadata = manifest.crates.get(crate_name).cloned().ok_or_else(|| {
            let error = FetchManifestError::CrateNotFoundInManifest {
                crate_name: crate_name.clone(),
                manifest_url: manifest_url.clone(),
            };
            DEBUG_SELF_UPGRADE.then(|| {
                let url_str: &str = manifest_url;
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = formatcp!(
                        "Failed to resolve crate metadata from {VERSIONS_JSONC_FILE}"
                    ),
                    crate_name = %crate_name,
                    url = %fg_magenta(url_str),
                    error = ?error
                );
            });
            error
        })?;
        Ok(FetchManifestOutput {
            metadata,
            binaries_release_base_url: manifest.binaries_release_base_url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{super::super::{CrateDistribution, CrateVersion, JsoncScalar,
                               integration_tests::test_fixtures::{blocks,
                                                                  manifest_server},
                               manifest_timeouts},
                *};
    use r3bl_tui::assert_eq2;
    use reqwest::StatusCode;

    const SAMPLE_MANIFEST_0_0_26: &str = r#"{
        "binaries_release_base_url": "https://github.com/r3bl-org/r3bl-binaries/releases/download",
        "crates": {
            "r3bl-cmdr": {
                "version": "0.0.26",
                "crate_distribution": "open-source",
                "released_on": "2026-04-06T12:00:00Z"
            }
        }
    }"#;

    #[tokio::test]
    async fn test_try_get_jsonc_manifest_from_invalid_url() {
        let invalid_url =
            EndpointUrl::try_parse("http://127.0.0.1:0/nonexistent").unwrap();
        let res_manifest = http_req_helpers::try_get_jsonc_manifest_from_url(
            &invalid_url,
            manifest_timeouts::DEFERRED_CHECK_FETCH_MANIFEST_URL_TIMEOUT,
        )
        .await;
        assert!(matches!(
            res_manifest,
            Err(FetchManifestError::HttpRequestFailed { .. })
        ));
    }

    #[tokio::test]
    async fn test_try_fetch_manifest_jsonc_parse_failed_preserves_source_error() {
        let crate_name: CrateName = "r3bl-cmdr".into();
        let res_server_spawn = manifest_server::spawn_success("{ invalid jsonc");

        let err = try_fetch_manifest(
            &res_server_spawn.endpoint_url,
            &crate_name,
            manifest_timeouts::DEFERRED_CHECK_FETCH_MANIFEST_URL_TIMEOUT,
        )
        .await
        .unwrap_err();
        res_server_spawn.assert_each_mock_called_once();

        match err {
            FetchManifestError::JsoncParseFailed {
                manifest_url,
                src_err,
            } => {
                assert_eq2!(manifest_url, res_server_spawn.endpoint_url);
                assert!(matches!(src_err, JsoncParseError::InvalidSyntax(_)));
            }
            other => panic!("Expected JsoncParseFailed(InvalidSyntax), got {other:?}"),
        }
    }

    #[test]
    fn test_try_get_crate_metadata_missing_crate_and_success() {
        let crate_name = "r3bl-cmdr".into();
        let missing_crate: CrateName = "nonexistent-crate".into();
        let manifest = JsoncManifest::try_parse_jsonc(SAMPLE_MANIFEST_0_0_26).unwrap();
        let example_url =
            EndpointUrl::try_parse("https://example.com/versions.jsonc").unwrap();

        let missing_err = http_req_helpers::try_get_crate_metadata_from_jsonc_manifest(
            manifest.clone(),
            &missing_crate,
            &example_url,
        )
        .unwrap_err();
        match missing_err {
            FetchManifestError::CrateNotFoundInManifest {
                crate_name: ref actual_crate,
                manifest_url: ref actual_url,
            } => {
                assert_eq2!(actual_crate, &missing_crate);
                assert_eq2!(actual_url, &example_url);
            }
            ref other => {
                panic!("Expected CrateNotFoundInManifest, got {other:?}")
            }
        }
        assert!(missing_err.to_string().contains("nonexistent-crate"));
        assert!(
            missing_err
                .to_string()
                .contains("https://example.com/versions.jsonc")
        );

        let FetchManifestOutput {
            metadata: meta,
            binaries_release_base_url: release_base_url,
        } = http_req_helpers::try_get_crate_metadata_from_jsonc_manifest(
            manifest,
            &crate_name,
            &example_url,
        )
        .unwrap();
        assert_eq2!(meta.version, CrateVersion::try_parse("0.0.26").unwrap());
        assert_eq2!(meta.crate_distribution, CrateDistribution::OpenSource);
        assert_eq2!(
            &*release_base_url,
            "https://github.com/r3bl-org/r3bl-binaries/releases/download"
        );
    }

    #[tokio::test]
    async fn test_try_fetch_manifest_success_and_http_error() {
        let crate_name: CrateName = "r3bl-cmdr".into();
        let res_ok_server_spawn = manifest_server::spawn_success(SAMPLE_MANIFEST_0_0_26);
        let FetchManifestOutput {
            metadata: meta,
            binaries_release_base_url: release_url,
        } = try_fetch_manifest(
            &res_ok_server_spawn.endpoint_url,
            &crate_name,
            manifest_timeouts::DEFERRED_CHECK_FETCH_MANIFEST_URL_TIMEOUT,
        )
        .await
        .unwrap();
        res_ok_server_spawn.assert_each_mock_called_once();
        assert_eq2!(meta.version, CrateVersion::try_parse("0.0.26").unwrap());
        assert_eq2!(
            &*release_url,
            "https://github.com/r3bl-org/r3bl-binaries/releases/download"
        );

        let res_err_server_spawn =
            manifest_server::spawn_error("boom", blocks::HTTP_500_INTERNAL_SERVER_ERROR);
        let err = try_fetch_manifest(
            &res_err_server_spawn.endpoint_url,
            &crate_name,
            manifest_timeouts::DEFERRED_CHECK_FETCH_MANIFEST_URL_TIMEOUT,
        )
        .await
        .unwrap_err();
        res_err_server_spawn.assert_each_mock_called_once();
        match err {
            FetchManifestError::HttpRequestFailed { src_err, .. } => {
                assert_eq2!(src_err.status(), Some(StatusCode::INTERNAL_SERVER_ERROR));
            }
            other => panic!("Expected HttpRequestFailed(500), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_try_fetch_manifest_timeout_returns_http_request_failed() {
        let crate_name: CrateName = "r3bl-cmdr".into();
        let res_server_spawn = manifest_server::spawn_delayed_success(
            SAMPLE_MANIFEST_0_0_26,
            Duration::from_millis(100),
        );

        let res_fetch = try_fetch_manifest(
            &res_server_spawn.endpoint_url,
            &crate_name,
            Duration::from_millis(20),
        )
        .await;
        res_server_spawn.assert_each_mock_called_once();

        match res_fetch {
            Err(FetchManifestError::HttpRequestFailed { ref src_err, .. }) => {
                assert!(src_err.is_timeout());
            }
            other => {
                panic!("Expected Err(HttpRequestFailed) on timeout, got {other:?}")
            }
        }
    }
}
