// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::super::{CheckFailed, Checking, CrateMetadata, CrateName, EndpointUrl,
                   JsoncManifest, UpToDate, UpgradeCheckStatus, UpgradeJourney,
                   UpgradeStrategy, Upgrading};
use crate::DEBUG_SELF_UPGRADE;
use miette::{Context, IntoDiagnostic};
use r3bl_tui::{create_client_with_user_agent, fg_magenta, ok};
use std::time::Duration;

/// Performs a dedicated-thread version check by consuming [`UpgradeJourney<Checking>`],
/// querying the JSONC manifest with [`UpgradeStrategy::manifest_timeout`][1],
/// transitioning `journey` into [`UpgradeJourney<UpgradeAvailable>`],
/// [`UpgradeJourney<UpToDate>`], or [`UpgradeJourney<CheckFailed>`], and updating the
/// [`UpgradeCheckStatus`] process-wide singleton.
///
/// Touches the `1h` `mtime` cooldown timestamp file via [`CooldownPolicy::touch`] when
/// the binary is verified [`UpToDate`]. Requires an [`UpgradeJourney<Checking>`]
/// typestate so it is a compile-time error to invoke a network version check when
/// [`UpgradePolicy::Disabled`] was configured.
///
/// # Error Handling
///
/// This check is opportunistic and fails silently when offline or when the manifest is
/// unreachable, storing the full `format!("{err:#}")` diagnostic chain inside
/// [`CheckFailed::error`]. See the [`self_upgrade`] module documentation for details on
/// invocation modes and error handling.
///
/// [1]: crate::self_upgrade::UpgradeStrategy::manifest_timeout
/// [`CheckFailed::error`]: crate::self_upgrade::CheckFailed::error
/// [`CheckFailed`]: crate::self_upgrade::CheckFailed
/// [`Checking`]: crate::self_upgrade::Checking
/// [`CooldownPolicy::touch`]: crate::self_upgrade::CooldownPolicy::touch
/// [`self_upgrade`]: mod@crate::self_upgrade
/// [`UpgradeAvailable`]: crate::self_upgrade::UpgradeAvailable
/// [`UpgradeCheckStatus`]: crate::self_upgrade::UpgradeCheckStatus
/// [`UpgradeJourney<CheckFailed>`]: crate::self_upgrade::UpgradeJourney
/// [`UpgradeJourney<Checking>`]: crate::self_upgrade::UpgradeJourney
/// [`UpgradeJourney<UpgradeAvailable>`]: crate::self_upgrade::UpgradeJourney
/// [`UpgradeJourney<UpToDate>`]: crate::self_upgrade::UpgradeJourney
/// [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
/// [`UpToDate`]: crate::self_upgrade::UpToDate
pub async fn check_for_upgrade(journey: UpgradeJourney<Checking>) {
    // Ensure the singleton reflects `Checking` even when called directly in unit tests.
    UpgradeCheckStatus::set(UpgradeCheckStatus::Checking(journey.clone()));

    let config = journey.get_config();
    let crate_name = config.get_crate_name();
    let crate_version = config.get_current_version();
    let manifest_url = config.get_jsonc_manifest_url();
    let strategy = journey.get_state().strategy;

    let res_manifest = http_req_helpers::try_get_jsonc_manifest_from_url(
        manifest_url,
        strategy.manifest_timeout(),
    )
    .await;

    let (metadata, binaries_release_base_url) =
        match http_req_helpers::extract_crate_metadata_from_jsonc_manifest(
            res_manifest,
            crate_name,
            manifest_url,
        ) {
            Err(error) => {
                DEBUG_SELF_UPGRADE.then(|| {
                    // % is Display, ? is Debug.
                    tracing::debug!(
                        message = "Failed to resolve crate metadata from versions manifest",
                        crate_name = %crate_name,
                        url = %fg_magenta(&**manifest_url),
                        error = %error
                    );
                });
                UpgradeCheckStatus::set(UpgradeCheckStatus::CheckFailed(
                    journey.into_check_failed(error),
                ));
                return;
            }
            Ok(pair) => pair,
        };

    let next_status = if metadata.version.is_newer_than(&crate_version) {
        DEBUG_SELF_UPGRADE.then(|| {
            // % is Display, ? is Debug.
            tracing::info!(
                message = "Newer version available in versions.jsonc",
                crate_name = %crate_name,
                current_version = %crate_version,
                remote_version = %metadata.version
            );
        });
        match strategy {
            UpgradeStrategy::DedicatedThread(action_on_exit) => {
                UpgradeCheckStatus::UpgradeAvailable(journey.into_upgrade_available(
                    action_on_exit,
                    metadata,
                    binaries_release_base_url,
                ))
            }
            UpgradeStrategy::Immediate => UpgradeCheckStatus::Upgrading(
                journey.into_upgrading(metadata, binaries_release_base_url),
            ),
        }
    } else {
        journey
            .get_config()
            .get_cooldown_policy()
            .touch(journey.get_config().get_crate_name());
        UpgradeCheckStatus::UpToDate(journey.into_up_to_date(metadata))
    };

    UpgradeCheckStatus::set(next_status);
}

/// Performs the synchronous [`UpgradeStrategy::Immediate`] (`--upgrade`)
/// manifest check, recording intermediate and terminal check states in
/// [`UpgradeCheckStatus`] and returning an [`ImmediateCheckOutcome`] for immediate
/// upgrade execution.
///
/// Uses [`UpgradeStrategy::manifest_timeout`] (`5s` for
/// [`UpgradeStrategy::Immediate`]), bypasses cooldown timestamp file reads on entry, and
/// touches the `1h` cooldown timestamp file via [`CooldownPolicy::touch`] when the
/// local binary is already [`UpToDate`].
///
/// [`CooldownPolicy::touch`]: crate::self_upgrade::CooldownPolicy::touch
/// [`UpgradeCheckStatus`]: crate::self_upgrade::UpgradeCheckStatus
/// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
/// [`UpgradeStrategy::manifest_timeout`]: crate::self_upgrade::UpgradeStrategy::manifest_timeout
/// [`UpToDate`]: crate::self_upgrade::UpToDate
pub async fn check_for_immediate_upgrade(
    journey: UpgradeJourney<Checking>,
) -> ImmediateCheckOutcome {
    UpgradeCheckStatus::set(UpgradeCheckStatus::Checking(journey.clone()));

    let name = journey.get_config().get_crate_name();
    let version = journey.get_config().get_current_version();
    let manifest_url = journey.get_config().get_jsonc_manifest_url();
    let timeout = journey.get_state().strategy.manifest_timeout();

    let res_manifest =
        http_req_helpers::try_get_jsonc_manifest_from_url(manifest_url, timeout).await;

    let (metadata, binaries_release_base_url) =
        match http_req_helpers::extract_crate_metadata_from_jsonc_manifest(
            res_manifest,
            name,
            manifest_url,
        ) {
            Err(error) => {
                DEBUG_SELF_UPGRADE.then(|| {
                    // % is Display, ? is Debug.
                    tracing::debug!(
                        message = "Immediate upgrade manifest check failed",
                        crate_name = %name,
                        url = %fg_magenta(&**manifest_url),
                        error = %error
                    );
                });
                let failed = journey.into_check_failed(error);
                UpgradeCheckStatus::set(UpgradeCheckStatus::CheckFailed(failed.clone()));
                return ImmediateCheckOutcome::CheckFailed(failed);
            }
            Ok(pair) => pair,
        };

    if metadata.version.is_newer_than(&version) {
        DEBUG_SELF_UPGRADE.then(|| {
            // % is Display, ? is Debug.
            tracing::info!(
                message = "Immediate upgrade found newer version in versions.jsonc",
                crate_name = %name,
                current_version = %version,
                remote_version = %metadata.version
            );
        });
        let upgrading = journey.into_upgrading(metadata, binaries_release_base_url);
        UpgradeCheckStatus::set(UpgradeCheckStatus::Upgrading(upgrading.clone()));
        ImmediateCheckOutcome::Upgrading(upgrading)
    } else {
        journey
            .get_config()
            .get_cooldown_policy()
            .touch(journey.get_config().get_crate_name());
        let up_to_date = journey.into_up_to_date(metadata);
        UpgradeCheckStatus::set(UpgradeCheckStatus::UpToDate(up_to_date.clone()));
        ImmediateCheckOutcome::UpToDate(up_to_date)
    }
}

/// These helper methods make it easier to write unit tests. Even though ergnonomically
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

    /// Dedicated `"self-upgrade-check"` OS thread or Tokio runtime failed to initialize.
    #[error("{reason}")]
    RuntimeOrThreadInitFailed {
        /// Underlying OS thread or Tokio runtime initialization failure description.
        reason: String,
    },

    /// Dedicated `"self-upgrade-check"` OS thread panicked during execution.
    #[error("Dedicated self-upgrade check thread panicked: {panic_message}")]
    CheckThreadPanicked {
        /// Extracted panic payload message.
        panic_message: String,
    },
}

/// Outcome of [`check_for_immediate_upgrade`] during an explicit `--upgrade`
/// ([`UpgradeStrategy::Immediate`]) run.
///
/// Unlike [`check_for_upgrade`], [`UpgradeStrategy::Immediate`] transitions directly
/// from [`Checking`] to [`Upgrading`] when a newer version exists so [`UpgradeAvailable`]
/// (which requires an [`UpgradeActionOnExit`]) is never entered.
///
/// [`Checking`]: crate::self_upgrade::Checking
/// [`UpgradeActionOnExit`]: crate::self_upgrade::UpgradeActionOnExit
/// [`UpgradeAvailable`]: crate::self_upgrade::UpgradeAvailable
/// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
/// [`Upgrading`]: crate::self_upgrade::Upgrading
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImmediateCheckOutcome {
    /// Network or manifest lookup failed; [`UpgradeCheckStatus::CheckFailed`] has been
    /// recorded in the process-wide singleton.
    ///
    /// [`UpgradeCheckStatus::CheckFailed`]: crate::self_upgrade::UpgradeCheckStatus::CheckFailed
    CheckFailed(UpgradeJourney<CheckFailed>),

    /// Local binary is already up to date (`remote <= current`); the `1h` cooldown
    /// timestamp file has been touched and [`UpgradeCheckStatus::UpToDate`] recorded.
    ///
    /// [`UpgradeCheckStatus::UpToDate`]: crate::self_upgrade::UpgradeCheckStatus::UpToDate
    UpToDate(UpgradeJourney<UpToDate>),

    /// Newer version found (`remote > current`); [`UpgradeCheckStatus::Upgrading`] has
    /// been recorded and [`perform_upgrade`] should run immediately.
    ///
    /// [`perform_upgrade`]: crate::self_upgrade::perform_upgrade
    /// [`UpgradeCheckStatus::Upgrading`]: crate::self_upgrade::UpgradeCheckStatus::Upgrading
    Upgrading(UpgradeJourney<Upgrading>),
}

#[cfg(test)]
mod tests {
    use super::{super::super::{CooldownPolicy, CrateDistribution, CrateVersion,
                               JsoncScalar, UpgradeActionOnExit, UpgradeConfig,
                               UpgradePolicy, UserFeedbackChannel, manifest_timeouts},
                *};
    use miette::Context;
    use r3bl_tui::{assert_eq2, try_create_temp_dir};
    use serial_test::serial;
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

    fn serve_one_http_response(status: &'static str, body: &'static str) -> String {
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
        format!("http://{addr}/versions.jsonc")
    }

    #[tokio::test]
    async fn test_try_get_jsonc_manifest_from_invalid_url() {
        let invalid_url =
            EndpointUrl::try_parse("http://127.0.0.1:0/nonexistent").unwrap();
        let result = http_req_helpers::try_get_jsonc_manifest_from_url(
            &invalid_url,
            manifest_timeouts::DEDICATED_THREAD_MANIFEST_TIMEOUT,
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
    #[serial]
    async fn test_check_for_upgrade_touches_cooldown_only_on_up_to_date() {
        UpgradeCheckStatus::reset_for_testing();
        let temp_dir = try_create_temp_dir().unwrap();

        // 1. UpgradeAvailable (`remote 0.0.26 > local 0.0.25`) must NOT touch cooldown.
        let avail_timestamp_file = temp_dir.join("avail.last_check");
        let avail_url = serve_one_http_response("200 OK", SAMPLE_MANIFEST_0_0_26);
        let avail_strategy =
            UpgradeStrategy::DedicatedThread(UpgradeActionOnExit::NotifyOnly);
        let avail_cfg = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(avail_strategy),
            EndpointUrl::try_parse(avail_url.as_str()).unwrap(),
            CooldownPolicy::new_custom_path(avail_timestamp_file.clone()),
        );
        let avail_checking = UpgradeJourney::new(avail_cfg).into_checking(avail_strategy);
        check_for_upgrade(avail_checking).await;
        assert!(!avail_timestamp_file.exists());
        assert!(matches!(
            UpgradeCheckStatus::get(),
            UpgradeCheckStatus::UpgradeAvailable(_)
        ));

        // 2. CheckFailed (`500 Internal Server Error`) must NOT touch cooldown.
        let fail_timestamp_file = temp_dir.join("fail.last_check");
        let fail_url = serve_one_http_response("500 Internal Server Error", "boom");
        let fail_strategy = UpgradeStrategy::DedicatedThread(UpgradeActionOnExit::Auto);
        let fail_cfg = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(fail_strategy),
            EndpointUrl::try_parse(fail_url.as_str()).unwrap(),
            CooldownPolicy::new_custom_path(fail_timestamp_file.clone()),
        );
        let fail_checking = UpgradeJourney::new(fail_cfg).into_checking(fail_strategy);
        check_for_upgrade(fail_checking).await;
        assert!(!fail_timestamp_file.exists());
        assert!(matches!(
            UpgradeCheckStatus::get(),
            UpgradeCheckStatus::CheckFailed(_)
        ));

        // 3. UpToDate (`remote 0.0.26 <= local 0.0.26`) MUST touch cooldown.
        let ok_timestamp_file = temp_dir.join("uptodate.last_check");
        let ok_url = serve_one_http_response("200 OK", SAMPLE_MANIFEST_0_0_26);
        let ok_strategy = UpgradeStrategy::DedicatedThread(UpgradeActionOnExit::Auto);
        let ok_cfg = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.26").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(ok_strategy),
            EndpointUrl::try_parse(ok_url.as_str()).unwrap(),
            CooldownPolicy::new_custom_path(ok_timestamp_file.clone()),
        );
        let ok_checking = UpgradeJourney::new(ok_cfg).into_checking(ok_strategy);
        check_for_upgrade(ok_checking).await;
        assert!(ok_timestamp_file.exists());
        assert!(matches!(
            UpgradeCheckStatus::get(),
            UpgradeCheckStatus::UpToDate(_)
        ));
    }
}
