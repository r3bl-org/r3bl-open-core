// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CheckFailed, UpToDate, UpgradeAvailable, UpgradeJourneyStateMarker,
            Upgrading};
use crate::{CrateMetadata, DEBUG_SELF_UPGRADE, EndpointUrl, ManifestCheckError,
            UpgradeActionOnExit, UpgradeJourney, UpgradeStrategy,
            fetch_crate_metadata_from_manifest};

/// Typestate while querying [`R3BL_VERSIONS_JSONC_URL`] (either synchronously on the
/// current thread via [`UpgradeStrategy::Immediate`] or on the dedicated thread via
/// [`UpgradeStrategy::Deferred`]). This type is part of the [Control Plane] (see
/// [Thread Model]).
///
/// [`R3BL_VERSIONS_JSONC_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSONC_URL
/// [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred
/// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Checking {
    /// Active [`UpgradeStrategy`] (`Immediate` or
    /// `Deferred(UpgradeActionOnExit)`).
    ///
    /// [`UpgradeStrategy`]: crate::self_upgrade::UpgradeStrategy
    pub strategy: UpgradeStrategy,
}
impl UpgradeJourneyStateMarker for Checking {}

impl UpgradeJourney<Checking> {
    #[must_use]
    pub(in crate::self_upgrade) fn into_up_to_date(
        self,
        metadata: CrateMetadata,
    ) -> UpgradeJourney<UpToDate> {
        UpgradeJourney {
            config: self.config,
            state: UpToDate { metadata },
        }
    }

    #[must_use]
    pub(in crate::self_upgrade) fn into_upgrade_available(
        self,
        action_on_exit: UpgradeActionOnExit,
        metadata: CrateMetadata,
        binaries_release_base_url: EndpointUrl,
    ) -> UpgradeJourney<UpgradeAvailable> {
        UpgradeJourney {
            config: self.config,
            state: UpgradeAvailable {
                action_on_exit,
                metadata,
                binaries_release_base_url,
            },
        }
    }

    #[must_use]
    pub(in crate::self_upgrade) fn into_upgrading(
        self,
        metadata: CrateMetadata,
        binaries_release_base_url: EndpointUrl,
    ) -> UpgradeJourney<Upgrading> {
        UpgradeJourney {
            config: self.config,
            state: Upgrading {
                metadata,
                binaries_release_base_url,
            },
        }
    }

    #[must_use]
    pub(in crate::self_upgrade) fn into_check_failed(
        self,
        error: ManifestCheckError,
    ) -> UpgradeJourney<CheckFailed> {
        UpgradeJourney {
            config: self.config,
            state: CheckFailed { error },
        }
    }

    /// Consumes `self` on the dedicated thread (in the [Control Plane]; see [Thread
    /// Model]), queries the JSONC manifest via [`fetch_crate_metadata_from_manifest`] (in
    /// the [Execution Plane]) using [`UpgradeStrategy::manifest_timeout`], touches the
    /// `1h` cooldown timestamp file when [`UpToDate`], and returns
    /// [`DeferredCheckOutcome`].
    ///
    /// [`fetch_crate_metadata_from_manifest`]: crate::self_upgrade::fetch_crate_metadata_from_manifest
    /// [`UpgradeStrategy::manifest_timeout`]: crate::self_upgrade::UpgradeStrategy::manifest_timeout
    /// [`UpToDate`]: crate::self_upgrade::UpToDate
    /// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
    /// [Execution Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
    /// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
    #[must_use]
    pub async fn check_for_deferred_upgrade(
        self,
        action_on_exit: UpgradeActionOnExit,
    ) -> DeferredCheckOutcome {
        let timeout = UpgradeStrategy::Deferred(action_on_exit).manifest_timeout();
        let (metadata, binaries_release_base_url) =
            match fetch_crate_metadata_from_manifest(
                self.config.get_jsonc_manifest_url(),
                self.config.get_crate_name(),
                timeout,
            )
            .await
            {
                Ok(pair) => pair,
                Err(error) => {
                    return DeferredCheckOutcome::CheckFailed(
                        self.into_check_failed(error),
                    );
                }
            };

        if metadata
            .version
            .is_newer_than(&self.config.get_current_version())
        {
            self.log_newer_version_available(&metadata);
            DeferredCheckOutcome::UpgradeAvailable(self.into_upgrade_available(
                action_on_exit,
                metadata,
                binaries_release_base_url,
            ))
        } else {
            self.touch_cooldown_on_up_to_date();
            DeferredCheckOutcome::UpToDate(self.into_up_to_date(metadata))
        }
    }

    /// Consumes `self` synchronously on the current thread during an explicit `--upgrade`
    /// ([`UpgradeStrategy::Immediate`]) run (in the [Control Plane]; see [Thread Model]),
    /// queries the JSONC manifest via [`fetch_crate_metadata_from_manifest`] (in the
    /// [Execution Plane]) using [`UpgradeStrategy::manifest_timeout`], touches the `1h`
    /// cooldown timestamp file when [`UpToDate`], and returns [`ImmediateCheckOutcome`].
    ///
    /// [`fetch_crate_metadata_from_manifest`]: crate::self_upgrade::fetch_crate_metadata_from_manifest
    /// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
    /// [`UpgradeStrategy::manifest_timeout`]: crate::self_upgrade::UpgradeStrategy::manifest_timeout
    /// [`UpToDate`]: crate::self_upgrade::UpToDate
    /// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
    /// [Execution Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
    /// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
    #[must_use]
    pub(in crate::self_upgrade) async fn check_for_immediate_upgrade(
        self,
    ) -> ImmediateCheckOutcome {
        let timeout = UpgradeStrategy::Immediate.manifest_timeout();
        let (metadata, binaries_release_base_url) =
            match fetch_crate_metadata_from_manifest(
                self.config.get_jsonc_manifest_url(),
                self.config.get_crate_name(),
                timeout,
            )
            .await
            {
                Ok(pair) => pair,
                Err(error) => {
                    return ImmediateCheckOutcome::CheckFailed(
                        self.into_check_failed(error),
                    );
                }
            };

        if metadata
            .version
            .is_newer_than(&self.config.get_current_version())
        {
            self.log_newer_version_available(&metadata);
            ImmediateCheckOutcome::Upgrading(
                self.into_upgrading(metadata, binaries_release_base_url),
            )
        } else {
            self.touch_cooldown_on_up_to_date();
            ImmediateCheckOutcome::UpToDate(self.into_up_to_date(metadata))
        }
    }

    fn log_newer_version_available(&self, metadata: &CrateMetadata) {
        DEBUG_SELF_UPGRADE.then(|| {
            // % is Display, ? is Debug.
            tracing::info!(
                message = "Newer version available in versions.jsonc",
                crate_name = %self.config.get_crate_name(),
                current_version = %self.config.get_current_version(),
                remote_version = %metadata.version
            );
        });
    }

    fn touch_cooldown_on_up_to_date(&self) {
        self.config
            .get_cooldown_policy()
            .touch(self.config.get_crate_name());
    }
}

/// Exhaustive outcome of
/// [`UpgradeJourney::<Checking>::check_for_deferred_upgrade`].
///
/// Returned directly via `JoinHandle<DeferredCheckOutcome>` from the dedicated thread
/// to [`handle_upgrade_on_exit`]. Makes [`Upgrading`] unrepresentable prior to
/// application exit.
///
/// [`handle_upgrade_on_exit`]: crate::self_upgrade::handle_upgrade_on_exit
/// [`Upgrading`]: crate::self_upgrade::Upgrading
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeferredCheckOutcome {
    /// Local binary is already up to date (`remote <= current`); `1h` cooldown timestamp
    /// file has been touched.
    UpToDate(UpgradeJourney<UpToDate>),

    /// Newer version found (`remote > current`); awaits exit handling via
    /// [`handle_upgrade_on_exit`].
    ///
    /// [`handle_upgrade_on_exit`]: crate::self_upgrade::handle_upgrade_on_exit
    UpgradeAvailable(UpgradeJourney<UpgradeAvailable>),

    /// Network, parse, or thread/runtime error occurred during the manifest check.
    CheckFailed(UpgradeJourney<CheckFailed>),
}

/// Exhaustive outcome of [`UpgradeJourney::<Checking>::check_for_immediate_upgrade`].
///
/// Makes [`UpgradeAvailable`] unrepresentable during an explicit `--upgrade`
/// ([`UpgradeStrategy::Immediate`]) run.
///
/// [`UpgradeAvailable`]: crate::self_upgrade::UpgradeAvailable
/// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::self_upgrade) enum ImmediateCheckOutcome {
    /// Local binary is already up to date (`remote <= current`); `1h` cooldown timestamp
    /// file has been touched.
    UpToDate(UpgradeJourney<UpToDate>),

    /// Newer version found (`remote > current`); ready to run
    /// [`UpgradeJourney::<Upgrading>::execute_immediate_upgrade`].
    Upgrading(UpgradeJourney<Upgrading>),

    /// Network or manifest lookup failed.
    CheckFailed(UpgradeJourney<CheckFailed>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CooldownPolicy, CrateVersion, JsoncScalar, UpgradeConfig, UpgradePolicy,
                UserFeedbackChannel};
    use r3bl_tui::{assert_eq2, try_create_temp_dir};
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
    async fn test_deferred_check_transitions_and_touches_cooldown_only_on_up_to_date() {
        let temp_dir = try_create_temp_dir().unwrap();

        // 1. UpgradeAvailable (`remote 0.0.26 > local 0.0.25`) must NOT touch cooldown.
        let avail_timestamp_file = temp_dir.join("avail.last_check");
        let avail_action = UpgradeActionOnExit::NotifyOnly;
        let avail_strategy = UpgradeStrategy::Deferred(avail_action);
        let avail_cfg = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(avail_strategy),
            serve_one_http_response("200 OK", SAMPLE_MANIFEST_0_0_26),
            CooldownPolicy::new_custom_path(avail_timestamp_file.clone()),
        );
        let avail_outcome = UpgradeJourney::new(avail_cfg)
            .into_checking(avail_strategy)
            .check_for_deferred_upgrade(avail_action)
            .await;
        assert!(!avail_timestamp_file.exists());
        match avail_outcome {
            DeferredCheckOutcome::UpgradeAvailable(journey) => {
                assert_eq2!(journey.get_state().action_on_exit, avail_action);
                assert_eq2!(
                    journey.get_state().metadata.version,
                    CrateVersion::try_parse("0.0.26").unwrap()
                );
            }
            other => panic!("Expected UpgradeAvailable, got {other:?}"),
        }

        // 2. CheckFailed (`500 Internal Server Error`) must NOT touch cooldown.
        let fail_timestamp_file = temp_dir.join("fail.last_check");
        let fail_action = UpgradeActionOnExit::Auto;
        let fail_strategy = UpgradeStrategy::Deferred(fail_action);
        let fail_cfg = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(fail_strategy),
            serve_one_http_response("500 Internal Server Error", "boom"),
            CooldownPolicy::new_custom_path(fail_timestamp_file.clone()),
        );
        let fail_outcome = UpgradeJourney::new(fail_cfg)
            .into_checking(fail_strategy)
            .check_for_deferred_upgrade(fail_action)
            .await;
        assert!(!fail_timestamp_file.exists());
        assert!(matches!(
            fail_outcome,
            DeferredCheckOutcome::CheckFailed(_)
        ));

        // 3. UpToDate (`remote 0.0.26 <= local 0.0.26`) MUST touch cooldown.
        let ok_timestamp_file = temp_dir.join("uptodate.last_check");
        let ok_action = UpgradeActionOnExit::Auto;
        let ok_strategy = UpgradeStrategy::Deferred(ok_action);
        let ok_cfg = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.26").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(ok_strategy),
            serve_one_http_response("200 OK", SAMPLE_MANIFEST_0_0_26),
            CooldownPolicy::new_custom_path(ok_timestamp_file.clone()),
        );
        let ok_outcome = UpgradeJourney::new(ok_cfg)
            .into_checking(ok_strategy)
            .check_for_deferred_upgrade(ok_action)
            .await;
        assert!(ok_timestamp_file.exists());
        assert!(matches!(ok_outcome, DeferredCheckOutcome::UpToDate(_)));
    }

    #[tokio::test]
    async fn test_immediate_check_transitions_and_touches_cooldown_only_on_up_to_date() {
        let temp_dir = try_create_temp_dir().unwrap();

        // 1. Upgrading (`remote 0.0.26 > local 0.0.25`) must NOT touch cooldown yet.
        let upg_timestamp_file = temp_dir.join("imm_upg.last_check");
        let upg_cfg = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
            serve_one_http_response("200 OK", SAMPLE_MANIFEST_0_0_26),
            CooldownPolicy::new_custom_path(upg_timestamp_file.clone()),
        );
        let upg_outcome = UpgradeJourney::new(upg_cfg)
            .into_checking(UpgradeStrategy::Immediate)
            .check_for_immediate_upgrade()
            .await;
        assert!(!upg_timestamp_file.exists());
        match upg_outcome {
            ImmediateCheckOutcome::Upgrading(journey) => {
                assert_eq2!(
                    journey.get_state().metadata.version,
                    CrateVersion::try_parse("0.0.26").unwrap()
                );
            }
            other => panic!("Expected Upgrading, got {other:?}"),
        }

        // 2. CheckFailed (`500 Internal Server Error`) must NOT touch cooldown.
        let fail_timestamp_file = temp_dir.join("imm_fail.last_check");
        let fail_cfg = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
            serve_one_http_response("500 Internal Server Error", "boom"),
            CooldownPolicy::new_custom_path(fail_timestamp_file.clone()),
        );
        let fail_outcome = UpgradeJourney::new(fail_cfg)
            .into_checking(UpgradeStrategy::Immediate)
            .check_for_immediate_upgrade()
            .await;
        assert!(!fail_timestamp_file.exists());
        assert!(matches!(
            fail_outcome,
            ImmediateCheckOutcome::CheckFailed(_)
        ));

        // 3. UpToDate (`remote 0.0.26 <= local 0.0.26`) MUST touch cooldown.
        let ok_timestamp_file = temp_dir.join("imm_ok.last_check");
        let ok_cfg = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.26").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
            serve_one_http_response("200 OK", SAMPLE_MANIFEST_0_0_26),
            CooldownPolicy::new_custom_path(ok_timestamp_file.clone()),
        );
        let ok_outcome = UpgradeJourney::new(ok_cfg)
            .into_checking(UpgradeStrategy::Immediate)
            .check_for_immediate_upgrade()
            .await;
        assert!(ok_timestamp_file.exists());
        assert!(matches!(ok_outcome, ImmediateCheckOutcome::UpToDate(_)));
    }
}
