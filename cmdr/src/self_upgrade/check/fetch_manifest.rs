// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::super::{CrateMetadata, CrateName, EndpointUrl, JsoncManifest};
use crate::DEBUG_SELF_UPGRADE;
use miette::{Context, IntoDiagnostic};
use r3bl_tui::{create_client_with_user_agent, fg_magenta, ok};
use std::time::Duration;

/// Fetches `versions.jsonc` from `manifest_url` within `timeout` and extracts
/// `(`[`CrateMetadata`]`, `[`EndpointUrl`]`)` for `crate_name`.
///
/// This function is called by the [`Checking`] typestate in the [Control Plane] to
/// resolve the transition into [`UpToDate`], [`UpgradeAvailable`], [`Upgrading`], or
/// [`CheckFailed`]. This is part of the [Execution Plane]: it logs any fetch or parse
/// failure via [`DEBUG_SELF_UPGRADE`], and never imports or touches [`UpgradeJourney`]
/// or [`SINGLETON`].
///
/// # Errors
///
/// Returns [`ManifestCheckError::FetchOrParseFailed`] if the HTTP request fails, times
/// out after `timeout`, returns a non-2xx status, or fails JSONC deserialization, or
/// [`ManifestCheckError::CrateNotFoundInManifest`] if `crate_name` is absent from
/// [`JsoncManifest::crates`].
///
/// [`CheckFailed`]: crate::self_upgrade::CheckFailed
/// [`Checking`]: crate::self_upgrade::Checking
/// [`CrateMetadata`]: crate::self_upgrade::CrateMetadata
/// [`DEBUG_SELF_UPGRADE`]: crate::DEBUG_SELF_UPGRADE
/// [`EndpointUrl`]: crate::self_upgrade::EndpointUrl
/// [`JsoncManifest::crates`]: crate::self_upgrade::JsoncManifest::crates
/// [`SINGLETON`]: crate::self_upgrade::SINGLETON
/// [`UpgradeAvailable`]: crate::self_upgrade::UpgradeAvailable
/// [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
/// [`Upgrading`]: crate::self_upgrade::Upgrading
/// [`UpToDate`]: crate::self_upgrade::UpToDate
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [Execution Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
pub async fn fetch_crate_metadata_from_manifest(
    manifest_url: &EndpointUrl,
    crate_name: &CrateName,
    timeout: Duration,
) -> Result<(CrateMetadata, EndpointUrl), ManifestCheckError> {
    let res_manifest =
        http_req_helpers::try_get_jsonc_manifest_from_url(manifest_url, timeout).await;

    let res_extracted = http_req_helpers::extract_crate_metadata_from_jsonc_manifest(
        res_manifest,
        crate_name,
        manifest_url,
    );

    if let Err(ref error) = res_extracted {
        DEBUG_SELF_UPGRADE.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "Failed to resolve crate metadata from versions manifest",
                crate_name = %crate_name,
                url = %fg_magenta(&**manifest_url),
                error = %error
            );
        });
    }

    res_extracted
}

/// These helper methods make it easier to write unit tests. Even though ergonomically
/// they are a bit awkward to use.
mod http_req_helpers {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Fetches `versions.jsonc` from `url` (typically
    /// [`EndpointUrl::default_manifest_url`], backed by [`R3BL_VERSIONS_JSONC_URL`])
    /// within `timeout` and parses it into a [`JsoncManifest`].
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails, times out after `timeout`, returns a
    /// non-2xx status, or if the manifest content cannot be parsed as valid JSONC.
    ///
    /// [`EndpointUrl::default_manifest_url`]: crate::self_upgrade::EndpointUrl::default_manifest_url
    /// [`JsoncManifest`]: crate::self_upgrade::JsoncManifest
    /// [`R3BL_VERSIONS_JSONC_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSONC_URL
    pub async fn try_get_jsonc_manifest_from_url(
        url: &EndpointUrl,
        timeout: Duration,
    ) -> miette::Result<JsoncManifest> {
        DEBUG_SELF_UPGRADE.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "Fetching versions manifest",
                url = %fg_magenta(&**url),
                timeout_ms = timeout.as_millis()
            );
        });

        // Make the HTTP GET request to fetch the `versions.jsonc` file as a string.
        let http_client = create_client_with_user_agent(None)?;
        // Closure prevents having to chain `.into_diagnostic()?` multiple times.
        let fetch_fn = async || -> Result<String, reqwest::Error> {
            http_client
                .get(&**url)
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

    /// Extracts [`CrateMetadata`] and [`EndpointUrl`] for `crate_name` from a
    /// [`JsoncManifest`] query result, preserving the full [`miette`] diagnostic chain
    /// via `format!("{err:#}")` on failure.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestCheckError`] if `res_manifest` is `Err` or if `crate_name` is
    /// absent from [`JsoncManifest::crates`].
    ///
    /// [`CrateMetadata`]: crate::self_upgrade::CrateMetadata
    /// [`EndpointUrl`]: crate::self_upgrade::EndpointUrl
    /// [`JsoncManifest::crates`]: crate::self_upgrade::JsoncManifest::crates
    /// [`JsoncManifest`]: crate::self_upgrade::JsoncManifest
    /// [`miette`]: mod@miette
    pub fn extract_crate_metadata_from_jsonc_manifest(
        res_manifest: miette::Result<JsoncManifest>,
        crate_name: &CrateName,
        manifest_url: &EndpointUrl,
    ) -> Result<(CrateMetadata, EndpointUrl), ManifestCheckError> {
        let manifest =
            res_manifest.map_err(|err| ManifestCheckError::FetchOrParseFailed {
                diagnostic: format!("{err:#}"),
            })?;
        let metadata = manifest.crates.get(crate_name).cloned().ok_or_else(|| {
            ManifestCheckError::CrateNotFoundInManifest {
                crate_name: crate_name.clone(),
                manifest_url: manifest_url.clone(),
            }
        })?;
        Ok((metadata, manifest.binaries_release_base_url))
    }
}

/// Failure reason recorded in [`CheckFailed::error`] when a manifest check fails.
///
/// [`CheckFailed::error`]: crate::self_upgrade::CheckFailed::error
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, miette::Diagnostic)]
pub enum ManifestCheckError {
    /// Target crate was not present in the remote `versions.jsonc` manifest's `crates`
    /// map ([`JsoncManifest::crates`]).
    ///
    /// [`JsoncManifest::crates`]: crate::self_upgrade::JsoncManifest::crates
    #[error("Crate '{crate_name}' not found in manifest at '{manifest_url}'")]
    CrateNotFoundInManifest {
        /// Queried crate name.
        crate_name: CrateName,
        /// Manifest endpoint URL where `crate_name` was looked up.
        manifest_url: EndpointUrl,
    },

    /// Network request or JSONC deserialization failed when fetching the manifest.
    #[error("{diagnostic}")]
    FetchOrParseFailed {
        /// Full [`miette`] diagnostic chain formatted via `format!("{err:#}")`.
        ///
        /// [`miette`]: mod@miette
        diagnostic: String,
    },

    /// Dedicated `"self-upgrade"` OS thread or Tokio runtime failed to initialize.
    #[error("{reason}")]
    RuntimeOrThreadInitFailed {
        /// Underlying OS thread or Tokio runtime initialization failure description.
        reason: String,
    },

    /// Dedicated `"self-upgrade"` OS thread panicked during a deferred check.
    #[error("Deferred self-upgrade check thread panicked: {panic_message}")]
    DeferredCheckThreadPanicked {
        /// Extracted panic payload message.
        panic_message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::{super::super::{CrateDistribution, CrateVersion, JsoncScalar,
                               manifest_timeouts},
                *};
    use miette::Context;
    use r3bl_tui::assert_eq2;
    use std::{io::{Read, Write},
              net::TcpListener};

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

    fn serve_one_http_response(status: &'static str, body: &'static str) -> EndpointUrl {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut req_buf = [0_u8; 1024];
                drop(stream.read(&mut req_buf));
                let resp = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                drop(stream.write_all(resp.as_bytes()));
                drop(stream.flush());
            }
        });
        EndpointUrl::try_parse(&format!("http://{addr}/versions.jsonc")).unwrap()
    }

    #[tokio::test]
    async fn test_try_get_jsonc_manifest_from_invalid_url() {
        let invalid_url =
            EndpointUrl::try_parse("http://127.0.0.1:0/nonexistent").unwrap();
        let result = http_req_helpers::try_get_jsonc_manifest_from_url(
            &invalid_url,
            manifest_timeouts::DEFERRED_CHECK_FETCH_MANIFEST_URL_TIMEOUT,
        )
        .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_crate_metadata_preserves_full_miette_diagnostic_chain() {
        let crate_name = "r3bl-cmdr".into();
        let root_err: miette::Result<JsoncManifest> =
            Err(miette::miette!("TLS handshake reset by peer"))
                .wrap_err("GET https://r3bl.com/versions.jsonc failed")
                .wrap_err("manifest fetch failed");

        let err = http_req_helpers::extract_crate_metadata_from_jsonc_manifest(
            root_err,
            &crate_name,
            &EndpointUrl::default_manifest_url(),
        )
        .unwrap_err();

        let err_str = err.to_string();
        assert!(matches!(err, ManifestCheckError::FetchOrParseFailed { .. }));
        assert!(err_str.contains("manifest fetch failed"));
        assert!(err_str.contains("GET https://r3bl.com/versions.jsonc failed"));
        assert!(err_str.contains("TLS handshake reset by peer"));
    }

    #[test]
    fn test_extract_crate_metadata_missing_crate_and_success() {
        let crate_name = "r3bl-cmdr".into();
        let missing_crate: CrateName = "nonexistent-crate".into();
        let manifest = JsoncManifest::try_parse_jsonc(SAMPLE_MANIFEST_0_0_26).unwrap();
        let example_url =
            EndpointUrl::try_parse("https://example.com/versions.jsonc").unwrap();

        let missing_err = http_req_helpers::extract_crate_metadata_from_jsonc_manifest(
            Ok(manifest.clone()),
            &missing_crate,
            &example_url,
        )
        .unwrap_err();
        assert_eq2!(
            missing_err,
            ManifestCheckError::CrateNotFoundInManifest {
                crate_name: missing_crate,
                manifest_url: example_url.clone(),
            }
        );
        assert!(missing_err.to_string().contains("nonexistent-crate"));
        assert!(
            missing_err
                .to_string()
                .contains("https://example.com/versions.jsonc")
        );

        let (meta, release_base_url) =
            http_req_helpers::extract_crate_metadata_from_jsonc_manifest(
                Ok(manifest),
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
    async fn test_fetch_crate_metadata_from_manifest_success_and_http_error() {
        let crate_name: CrateName = "r3bl-cmdr".into();
        let ok_url = serve_one_http_response("200 OK", SAMPLE_MANIFEST_0_0_26);
        let (meta, release_url) = fetch_crate_metadata_from_manifest(
            &ok_url,
            &crate_name,
            manifest_timeouts::DEFERRED_CHECK_FETCH_MANIFEST_URL_TIMEOUT,
        )
        .await
        .unwrap();
        assert_eq2!(meta.version, CrateVersion::try_parse("0.0.26").unwrap());
        assert_eq2!(
            &*release_url,
            "https://github.com/r3bl-org/r3bl-binaries/releases/download"
        );

        let err_url = serve_one_http_response("500 Internal Server Error", "boom");
        let err = fetch_crate_metadata_from_manifest(
            &err_url,
            &crate_name,
            manifest_timeouts::DEFERRED_CHECK_FETCH_MANIFEST_URL_TIMEOUT,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, ManifestCheckError::FetchOrParseFailed { .. }));
    }
}
