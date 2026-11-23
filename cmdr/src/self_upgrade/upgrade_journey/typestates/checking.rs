// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CheckFailed, CheckFailedError, StateMarker, UpToDate, UpgradeAvailable,
            Upgrading};
use crate::{CrateMetadata, DEBUG_SELF_UPGRADE, EndpointUrl, FetchManifestOutput,
            UpgradeActionOnExit, UpgradeJourney, UpgradeStrategy, VERSIONS_JSONC_FILE,
            try_fetch_manifest};
use const_format::formatcp;

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
impl StateMarker for Checking {}

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
        error: CheckFailedError,
    ) -> UpgradeJourney<CheckFailed> {
        UpgradeJourney {
            config: self.config,
            state: CheckFailed { error },
        }
    }

    /// Consumes `self` on the dedicated thread (in the [Control Plane]; see [Thread
    /// Model]), queries the JSONC manifest via [`try_fetch_manifest`] (in
    /// the [Execution Plane]) using [`UpgradeStrategy::manifest_timeout`], touches the
    /// `1h` cooldown timestamp file when [`UpToDate`], and returns
    /// [`DeferredCheckOutcome`].
    ///
    /// [`try_fetch_manifest`]: crate::self_upgrade::try_fetch_manifest
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
        let FetchManifestOutput {
            metadata,
            binaries_release_base_url,
        } = match try_fetch_manifest(
            self.config.get_jsonc_manifest_url(),
            self.config.get_crate_name(),
            timeout,
        )
        .await
        {
            Ok(info) => info,
            Err(error) => {
                return DeferredCheckOutcome::CheckFailed(
                    self.into_check_failed(error.into()),
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
    /// queries the JSONC manifest via [`try_fetch_manifest`] (in the
    /// [Execution Plane]) using [`UpgradeStrategy::manifest_timeout`], touches the `1h`
    /// cooldown timestamp file when [`UpToDate`], and returns [`ImmediateCheckOutcome`].
    ///
    /// [`try_fetch_manifest`]: crate::self_upgrade::try_fetch_manifest
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
        let FetchManifestOutput {
            metadata,
            binaries_release_base_url,
        } = match try_fetch_manifest(
            self.config.get_jsonc_manifest_url(),
            self.config.get_crate_name(),
            timeout,
        )
        .await
        {
            Ok(info) => info,
            Err(error) => {
                return ImmediateCheckOutcome::CheckFailed(
                    self.into_check_failed(error.into()),
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
                message = formatcp!("Newer version available in {VERSIONS_JSONC_FILE}"),
                crate_name = %self.config.get_crate_name(),
                current_version = %self.config.get_current_version(),
                remote_version = %metadata.version
            );
        });
    }

    fn touch_cooldown_on_up_to_date(&self) {
        if let Err(err) = self
            .config
            .get_remote_version_check_gate()
            .try_touch_timestamp_file()
        {
            DEBUG_SELF_UPGRADE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "Could not enter cooldown after confirming binary is up to date",
                    crate_name = %self.config.get_crate_name(),
                    error = ?err
                );
            });
        }
    }
}

/// Exhaustive outcome of
/// [`UpgradeJourney::<Checking>::check_for_deferred_upgrade`].
///
/// Returned directly via [`DeferredCheckHandle`] from the dedicated thread to
/// [`DeferredUpgradeOutcome::handle_upgrade_on_exit`]. Makes [`Upgrading`]
/// unrepresentable prior to application exit.
///
/// [`DeferredCheckHandle`]: crate::self_upgrade::DeferredCheckHandle
/// [`DeferredUpgradeOutcome::handle_upgrade_on_exit`]: crate::self_upgrade::DeferredUpgradeOutcome::handle_upgrade_on_exit
/// [`Upgrading`]: crate::self_upgrade::Upgrading
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeferredCheckOutcome {
    /// Local binary is already up to date (`remote <= current`); `1h` cooldown timestamp
    /// file has been touched.
    UpToDate(UpgradeJourney<UpToDate>),

    /// Newer version found (`remote > current`); awaits exit handling via
    /// [`DeferredUpgradeOutcome::handle_upgrade_on_exit`].
    ///
    /// [`DeferredUpgradeOutcome::handle_upgrade_on_exit`]: crate::self_upgrade::DeferredUpgradeOutcome::handle_upgrade_on_exit
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
    use crate::{CrateVersion, GateStatus, GateStatusError, JsoncScalar,
                RemoteVersionCheckGate, UpgradeConfig, UpgradePolicy,
                UserFeedbackChannel,
                self_upgrade::integration_tests::test_fixtures::{blocks,
                                                                 manifest_server}};
    use r3bl_tui::{TempDirChildPath, assert_eq2, try_create_temp_dir};
    use std::io::ErrorKind;

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
    async fn test_deferred_check_transitions_and_touches_cooldown_only_on_up_to_date() {
        let temp_dir = try_create_temp_dir().unwrap();

        // 1. UpgradeAvailable (`remote 0.0.26 > local 0.0.25`) must NOT touch cooldown.
        let avail_action = UpgradeActionOnExit::NotifyOnly;
        let avail_strategy = UpgradeStrategy::Deferred(avail_action);
        let res_avail_server_spawn =
            manifest_server::spawn_success(SAMPLE_MANIFEST_0_0_26);
        let avail_cfg = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(avail_strategy),
            res_avail_server_spawn.endpoint_url.clone(),
            RemoteVersionCheckGate::new_temp_dir(
                TempDirChildPath::try_new(&temp_dir, "avail").unwrap(),
                None,
            ),
        );
        let avail_gate = avail_cfg.get_remote_version_check_gate().clone();
        let avail_outcome = UpgradeJourney::new(avail_cfg)
            .into_checking(avail_strategy)
            .check_for_deferred_upgrade(avail_action)
            .await;
        res_avail_server_spawn.assert_each_mock_called_once();
        assert!(matches!(
            avail_gate.try_check_gate_status(),
            Err(GateStatusError::MetadataReadFailed { src_err, .. })
                if src_err.kind() == ErrorKind::NotFound
        ));
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
        let fail_action = UpgradeActionOnExit::Auto;
        let fail_strategy = UpgradeStrategy::Deferred(fail_action);
        let res_fail_server_spawn =
            manifest_server::spawn_error("boom", blocks::HTTP_500_INTERNAL_SERVER_ERROR);
        let fail_cfg = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(fail_strategy),
            res_fail_server_spawn.endpoint_url.clone(),
            RemoteVersionCheckGate::new_temp_dir(
                TempDirChildPath::try_new(&temp_dir, "fail").unwrap(),
                None,
            ),
        );
        let fail_gate = fail_cfg.get_remote_version_check_gate().clone();
        let fail_outcome = UpgradeJourney::new(fail_cfg)
            .into_checking(fail_strategy)
            .check_for_deferred_upgrade(fail_action)
            .await;
        res_fail_server_spawn.assert_each_mock_called_once();
        assert!(matches!(
            fail_gate.try_check_gate_status(),
            Err(GateStatusError::MetadataReadFailed { src_err, .. })
                if src_err.kind() == ErrorKind::NotFound
        ));
        assert!(matches!(fail_outcome, DeferredCheckOutcome::CheckFailed(_)));

        // 3. UpToDate (`remote 0.0.26 <= local 0.0.26`) MUST touch cooldown.
        let ok_action = UpgradeActionOnExit::Auto;
        let ok_strategy = UpgradeStrategy::Deferred(ok_action);
        let res_ok_server_spawn =
            manifest_server::spawn_success(SAMPLE_MANIFEST_0_0_26);
        let ok_cfg = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.26").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(ok_strategy),
            res_ok_server_spawn.endpoint_url.clone(),
            RemoteVersionCheckGate::new_temp_dir(
                TempDirChildPath::try_new(&temp_dir, "uptodate").unwrap(),
                None,
            ),
        );
        let ok_gate = ok_cfg.get_remote_version_check_gate().clone();
        let ok_outcome = UpgradeJourney::new(ok_cfg)
            .into_checking(ok_strategy)
            .check_for_deferred_upgrade(ok_action)
            .await;
        res_ok_server_spawn.assert_each_mock_called_once();
        assert_eq2!(
            ok_gate.try_check_gate_status().unwrap(),
            GateStatus::ClosedForCooldown
        );
        assert!(matches!(ok_outcome, DeferredCheckOutcome::UpToDate(_)));
    }

    #[tokio::test]
    async fn test_immediate_check_transitions_and_touches_cooldown_only_on_up_to_date() {
        let temp_dir = try_create_temp_dir().unwrap();

        // 1. Upgrading (`remote 0.0.26 > local 0.0.25`) must NOT touch cooldown yet.
        let res_upg_server_spawn =
            manifest_server::spawn_success(SAMPLE_MANIFEST_0_0_26);
        let upg_cfg = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
            res_upg_server_spawn.endpoint_url.clone(),
            RemoteVersionCheckGate::new_temp_dir(
                TempDirChildPath::try_new(&temp_dir, "imm_upg").unwrap(),
                None,
            ),
        );
        let upg_gate = upg_cfg.get_remote_version_check_gate().clone();
        let upg_outcome = UpgradeJourney::new(upg_cfg)
            .into_checking(UpgradeStrategy::Immediate)
            .check_for_immediate_upgrade()
            .await;
        res_upg_server_spawn.assert_each_mock_called_once();
        assert!(matches!(
            upg_gate.try_check_gate_status(),
            Err(GateStatusError::MetadataReadFailed { src_err, .. })
                if src_err.kind() == ErrorKind::NotFound
        ));
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
        let res_fail_server_spawn =
            manifest_server::spawn_error("boom", blocks::HTTP_500_INTERNAL_SERVER_ERROR);
        let fail_cfg = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
            res_fail_server_spawn.endpoint_url.clone(),
            RemoteVersionCheckGate::new_temp_dir(
                TempDirChildPath::try_new(&temp_dir, "imm_fail").unwrap(),
                None,
            ),
        );
        let fail_gate = fail_cfg.get_remote_version_check_gate().clone();
        let fail_outcome = UpgradeJourney::new(fail_cfg)
            .into_checking(UpgradeStrategy::Immediate)
            .check_for_immediate_upgrade()
            .await;
        res_fail_server_spawn.assert_each_mock_called_once();
        assert!(matches!(
            fail_gate.try_check_gate_status(),
            Err(GateStatusError::MetadataReadFailed { src_err, .. })
                if src_err.kind() == ErrorKind::NotFound
        ));
        assert!(matches!(
            fail_outcome,
            ImmediateCheckOutcome::CheckFailed(_)
        ));

        // 3. UpToDate (`remote 0.0.26 <= local 0.0.26`) MUST touch cooldown.
        let res_ok_server_spawn =
            manifest_server::spawn_success(SAMPLE_MANIFEST_0_0_26);
        let ok_cfg = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.26").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
            res_ok_server_spawn.endpoint_url.clone(),
            RemoteVersionCheckGate::new_temp_dir(
                TempDirChildPath::try_new(&temp_dir, "imm_ok").unwrap(),
                None,
            ),
        );
        let ok_gate = ok_cfg.get_remote_version_check_gate().clone();
        let ok_outcome = UpgradeJourney::new(ok_cfg)
            .into_checking(UpgradeStrategy::Immediate)
            .check_for_immediate_upgrade()
            .await;
        res_ok_server_spawn.assert_each_mock_called_once();
        assert_eq2!(
            ok_gate.try_check_gate_status().unwrap(),
            GateStatus::ClosedForCooldown
        );
        assert!(matches!(ok_outcome, ImmediateCheckOutcome::UpToDate(_)));
    }
}
