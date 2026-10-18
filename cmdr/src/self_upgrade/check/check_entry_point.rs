// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::{CheckFailed, Checking, CrateMetadata, CrateName, JsoncManifest,
                    UpToDate, UpgradeCheckStatus, UpgradeJourney, UpgradeStrategy,
                    Upgrading},
            record_successful_check, try_get_jsonc_manifest_from_url};
use crate::DEBUG_SELF_UPGRADE;
use r3bl_tui::fg_magenta;

/// Extracts [`CrateMetadata`] for `crate_name` from a [`JsoncManifest`] query result,
/// preserving the full [`miette`] diagnostic chain via `format!("{err:#}")` on failure.
///
/// # Errors
///
/// Returns a formatted diagnostic `String` if `res_manifest` is `Err` or if
/// `crate_name` is absent from [`JsoncManifest::crates`].
///
/// [`CrateMetadata`]: crate::self_upgrade::CrateMetadata
/// [`JsoncManifest::crates`]: crate::self_upgrade::JsoncManifest::crates
/// [`JsoncManifest`]: crate::self_upgrade::JsoncManifest
/// [`miette`]: mod@miette
pub fn extract_crate_metadata_from_manifest_result(
    res_manifest: miette::Result<JsoncManifest>,
    crate_name: &CrateName,
    manifest_url: &str,
) -> Result<CrateMetadata, String> {
    let manifest = res_manifest.map_err(|err| format!("{err:#}"))?;
    manifest.crates.get(crate_name).cloned().ok_or_else(|| {
        format!("Crate '{crate_name}' not found in manifest at '{manifest_url}'")
    })
}

/// Performs a dedicated-thread version check by consuming [`UpgradeJourney<Checking>`],
/// querying the JSONC manifest with [`UpgradeStrategy::manifest_timeout`],
/// transitioning `journey` into [`UpgradeJourney<UpgradeAvailable>`],
/// [`UpgradeJourney<UpToDate>`], or [`UpgradeJourney<CheckFailed>`], and updating the
/// [`UpgradeCheckStatus`] process-wide singleton.
///
/// Touches the `1h` `mtime` cooldown timestamp file via [`record_successful_check`] when
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
/// [`CheckFailed::error`]: crate::self_upgrade::CheckFailed::error
/// [`CheckFailed`]: crate::self_upgrade::CheckFailed
/// [`Checking`]: crate::self_upgrade::Checking
/// [`record_successful_check`]: crate::self_upgrade::record_successful_check
/// [`self_upgrade`]: mod@crate::self_upgrade
/// [`UpgradeAvailable`]: crate::self_upgrade::UpgradeAvailable
/// [`UpgradeCheckStatus`]: crate::self_upgrade::UpgradeCheckStatus
/// [`UpgradeJourney<CheckFailed>`]: crate::self_upgrade::UpgradeJourney
/// [`UpgradeJourney<Checking>`]: crate::self_upgrade::UpgradeJourney
/// [`UpgradeJourney<UpgradeAvailable>`]: crate::self_upgrade::UpgradeJourney
/// [`UpgradeJourney<UpToDate>`]: crate::self_upgrade::UpgradeJourney
/// [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
/// [`UpgradeStrategy::manifest_timeout`]: crate::self_upgrade::UpgradeStrategy::manifest_timeout
/// [`UpToDate`]: crate::self_upgrade::UpToDate
pub async fn check_for_upgrade(journey: UpgradeJourney<Checking>) {
    // Ensure the singleton reflects `Checking` even when called directly in unit tests.
    UpgradeCheckStatus::set(UpgradeCheckStatus::Checking(journey.clone()));

    let name = journey.config().get_crate_name().clone();
    let version = journey.config().get_current_version();
    let manifest_url = journey.config().get_jsonc_manifest_url().to_string();
    let strategy = journey.state().strategy;

    let res_manifest =
        try_get_jsonc_manifest_from_url(&manifest_url, strategy.manifest_timeout())
            .await;

    let metadata = match extract_crate_metadata_from_manifest_result(
        res_manifest,
        &name,
        &manifest_url,
    ) {
        Err(error) => {
            DEBUG_SELF_UPGRADE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "Failed to resolve crate metadata from versions manifest",
                    crate_name = %name,
                    url = %fg_magenta(&manifest_url),
                    error = %error
                );
            });
            UpgradeCheckStatus::set(UpgradeCheckStatus::CheckFailed(
                journey.into_check_failed(error),
            ));
            return;
        }
        Ok(metadata) => metadata,
    };

    let next_status = if metadata.version.is_newer_than(&version) {
        DEBUG_SELF_UPGRADE.then(|| {
            // % is Display, ? is Debug.
            tracing::info!(
                message = "Newer version available in versions.jsonc",
                crate_name = %name,
                current_version = %version,
                remote_version = %metadata.version
            );
        });
        match strategy {
            UpgradeStrategy::DedicatedThread(action_on_exit) => {
                UpgradeCheckStatus::UpgradeAvailable(
                    journey.into_upgrade_available(action_on_exit, metadata),
                )
            }
            UpgradeStrategy::Immediate => {
                UpgradeCheckStatus::Upgrading(journey.into_upgrading(metadata))
            }
        }
    } else {
        record_successful_check(journey.config());
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
/// touches the `1h` cooldown timestamp file via [`record_successful_check`] when the
/// local binary is already [`UpToDate`].
///
/// [`record_successful_check`]: crate::self_upgrade::record_successful_check
/// [`UpgradeCheckStatus`]: crate::self_upgrade::UpgradeCheckStatus
/// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
/// [`UpgradeStrategy::manifest_timeout`]: crate::self_upgrade::UpgradeStrategy::manifest_timeout
/// [`UpToDate`]: crate::self_upgrade::UpToDate
pub async fn check_for_immediate_upgrade(
    journey: UpgradeJourney<Checking>,
) -> ImmediateCheckOutcome {
    UpgradeCheckStatus::set(UpgradeCheckStatus::Checking(journey.clone()));

    let name = journey.config().get_crate_name().clone();
    let version = journey.config().get_current_version();
    let manifest_url = journey.config().get_jsonc_manifest_url().to_string();
    let timeout = journey.state().strategy.manifest_timeout();

    let res_manifest = try_get_jsonc_manifest_from_url(&manifest_url, timeout).await;

    let metadata = match extract_crate_metadata_from_manifest_result(
        res_manifest,
        &name,
        &manifest_url,
    ) {
        Err(error) => {
            DEBUG_SELF_UPGRADE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "Immediate upgrade manifest check failed",
                    crate_name = %name,
                    url = %fg_magenta(&manifest_url),
                    error = %error
                );
            });
            let failed = journey.into_check_failed(error);
            UpgradeCheckStatus::set(UpgradeCheckStatus::CheckFailed(failed.clone()));
            return ImmediateCheckOutcome::CheckFailed(failed);
        }
        Ok(metadata) => metadata,
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
        let upgrading = journey.into_upgrading(metadata);
        UpgradeCheckStatus::set(UpgradeCheckStatus::Upgrading(upgrading.clone()));
        ImmediateCheckOutcome::Upgrading(upgrading)
    } else {
        record_successful_check(journey.config());
        let up_to_date = journey.into_up_to_date(metadata);
        UpgradeCheckStatus::set(UpgradeCheckStatus::UpToDate(up_to_date.clone()));
        ImmediateCheckOutcome::UpToDate(up_to_date)
    }
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
    use super::{super::super::{CooldownPolicy, CrateDistribution, CrateIdentity,
                               CrateVersion, UpgradeActionOnExit, UpgradeConfig,
                               UpgradePolicy, UserFeedbackChannel},
                *};
    use miette::Context;
    use r3bl_tui::{assert_eq2, try_create_temp_dir};
    use serial_test::serial;
    use std::{io::{Read, Write},
              net::TcpListener};

    const SAMPLE_MANIFEST_0_0_26: &str = r#"{
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

    #[test]
    fn test_extract_crate_metadata_preserves_full_miette_diagnostic_chain() {
        let root_err: miette::Result<JsoncManifest> =
            Err(miette::miette!("TLS handshake reset by peer"))
                .wrap_err("GET https://r3bl.com/versions.jsonc failed")
                .wrap_err("manifest fetch failed");

        let err_str = extract_crate_metadata_from_manifest_result(
            root_err,
            &CrateName::from("r3bl-cmdr"),
            "https://r3bl.com/versions.jsonc",
        )
        .unwrap_err();

        assert!(err_str.contains("manifest fetch failed"));
        assert!(err_str.contains("GET https://r3bl.com/versions.jsonc failed"));
        assert!(err_str.contains("TLS handshake reset by peer"));
    }

    #[test]
    fn test_extract_crate_metadata_missing_crate_and_success() {
        let manifest = JsoncManifest::try_parse_jsonc(SAMPLE_MANIFEST_0_0_26).unwrap();

        let missing_err = extract_crate_metadata_from_manifest_result(
            Ok(manifest.clone()),
            &CrateName::from("nonexistent-crate"),
            "https://example.com/versions.jsonc",
        )
        .unwrap_err();
        assert!(missing_err.contains("nonexistent-crate"));
        assert!(missing_err.contains("https://example.com/versions.jsonc"));

        let meta = extract_crate_metadata_from_manifest_result(
            Ok(manifest),
            &CrateName::from("r3bl-cmdr"),
            "https://example.com/versions.jsonc",
        )
        .unwrap();
        assert_eq2!(meta.version, CrateVersion::try_from("0.0.26").unwrap());
        assert_eq2!(meta.crate_distribution, CrateDistribution::OpenSource);
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
            CrateIdentity::new("r3bl-cmdr", CrateVersion::try_from("0.0.25").unwrap()),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(avail_strategy),
            Some(avail_url),
            None,
            CooldownPolicy::CustomTimestampFilePath(avail_timestamp_file.clone()),
        );
        let avail_checking =
            UpgradeJourney::new(avail_cfg).into_checking(avail_strategy);
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
            CrateIdentity::new("r3bl-cmdr", CrateVersion::try_from("0.0.25").unwrap()),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(fail_strategy),
            Some(fail_url),
            None,
            CooldownPolicy::CustomTimestampFilePath(fail_timestamp_file.clone()),
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
            CrateIdentity::new("r3bl-cmdr", CrateVersion::try_from("0.0.26").unwrap()),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(ok_strategy),
            Some(ok_url),
            None,
            CooldownPolicy::CustomTimestampFilePath(ok_timestamp_file.clone()),
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
