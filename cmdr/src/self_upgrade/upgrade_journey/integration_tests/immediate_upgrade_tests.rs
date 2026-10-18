// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Immediate upgrade ([`UpgradeStrategy::Immediate`]) sync/async [`UpToDate`], cooldown
//! bypass, [`CheckFailed`], and [`UpgradeResolution::UpgradeFailed`] integration tests
//! for [`UpgradeJourney`].
//!
//! [`CheckFailed`]: crate::self_upgrade::CheckFailed
//! [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
//! [`UpgradeResolution::UpgradeFailed`]: crate::self_upgrade::UpgradeResolution::UpgradeFailed
//! [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
//! [`UpToDate`]: crate::self_upgrade::UpToDate

use super::test_fixtures::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CooldownPolicy, CrateName, CrateVersion, JsoncScalar,
                ManifestCheckError, UpgradeConfig, UpgradeError, UpgradePolicy,
                UpgradeStrategy, UserFeedbackChannel,
                resolve_self_upgrade_last_check_in_base_dir, self_upgrade::*};
    use r3bl_tui::{assert_eq2, try_create_temp_dir};
    use serial_test::serial;
    use std::{fs::File,
              time::{Duration, SystemTime}};

    #[test]
    #[serial]
    fn test_immediate_upgrade_up_to_date_sync_touches_and_bypasses_cooldown() {
        SingletonState::reset_for_testing();

        let crate_name: CrateName = "r3bl-cmdr".into();
        let temp_dir = try_create_temp_dir().unwrap();
        let timestamp_file_path =
            resolve_self_upgrade_last_check_in_base_dir(&temp_dir, &crate_name);

        // Pre-populate an active cooldown timestamp file to verify
        // `UpgradeStrategy::Immediate` (`--upgrade`) bypasses it and still performs the
        // live HTTP check.
        let server_spawn_result = manifest_server::spawn_success(manifest_up_to_date());
        let config = UpgradeConfig::new(
            crate_name,
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
            server_spawn_result.url.clone(),
            CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
        );
        config.get_cooldown_policy().touch(config.get_crate_name());
        let backdated_within_cooldown = SystemTime::now() - Duration::from_mins(30);
        File::open(&timestamp_file_path)
            .unwrap()
            .set_modified(backdated_within_cooldown)
            .unwrap();

        let outcome = UpgradeJourney::new(config).start();
        server_spawn_result.assert_each_endpoint_called_once();

        assert!(matches!(
            outcome,
            StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::UpToDate(_))
        ));

        // Verify the timestamp file `mtime` was refreshed by the live `UpToDate` check.
        let refreshed_mtime = std::fs::metadata(&timestamp_file_path)
            .unwrap()
            .modified()
            .unwrap();
        assert!(refreshed_mtime > backdated_within_cooldown);
    }

    #[tokio::test]
    #[serial]
    async fn test_immediate_upgrade_up_to_date_async() {
        SingletonState::reset_for_testing();

        let server_spawn_result = manifest_server::spawn_success(manifest_up_to_date());
        let config = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
            server_spawn_result.url.clone(),
            CooldownPolicy::Disabled,
        );

        let outcome = UpgradeJourney::new(config).start();
        server_spawn_result.assert_each_endpoint_called_once();

        assert!(matches!(
            outcome,
            StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::UpToDate(_))
        ));
    }

    #[test]
    #[serial]
    fn test_immediate_upgrade_check_failed_and_upgrade_failed_transitions() {
        SingletonState::reset_for_testing();

        let temp_dir = try_create_temp_dir().unwrap();
        let timestamp_file_path = temp_dir.join("failed.last_check");

        // 1. Manifest HTTP 500 -> CheckFailed, no cooldown timestamp file.
        let bad_server_spawn_result = manifest_server::spawn_error(
            "server error",
            blocks::HTTP_500_INTERNAL_SERVER_ERROR,
        );
        let fail_cfg = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
            bad_server_spawn_result.url.clone(),
            CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
        );
        let fail_outcome = UpgradeJourney::new(fail_cfg).start();
        bad_server_spawn_result.assert_each_endpoint_called_once();

        assert!(!timestamp_file_path.exists());
        match fail_outcome {
            StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::CheckFailed(
                journey,
            )) => {
                assert!(matches!(
                    journey.get_state().error,
                    ManifestCheckError::FetchOrParseFailed { ref diagnostic } if !diagnostic.is_empty()
                ));
            }
            other => panic!("Expected ImmediateUpgrade(CheckFailed), got {other:?}"),
        }

        // 2. Newer ClosedSource version + unreachable release archive server ->
        // transitions Checking -> Upgrading -> Completed(UpgradeFailed), no timestamp
        // file.
        SingletonState::reset_for_testing();

        let dead_url = placeholder_release_base_url().with_closed_socket_addr();
        let newer_server_spawn_result = manifest_server::spawn_success(
            manifest_upgrade_available_closed_source(&dead_url),
        );

        let upg_fail_cfg = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
            newer_server_spawn_result.url.clone(),
            CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
        );
        let upg_fail_outcome = UpgradeJourney::new(upg_fail_cfg).start();
        newer_server_spawn_result.assert_each_endpoint_called_once();

        assert!(!timestamp_file_path.exists());
        match upg_fail_outcome {
            StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::Completed(
                journey,
            )) => {
                assert_eq2!(
                    journey.get_state().metadata.version,
                    CrateVersion::try_parse("9.9.9").unwrap()
                );
                assert!(matches!(
                    journey.get_state().resolution,
                    UpgradeResolution::UpgradeFailed(
                        UpgradeError::NoViableUpgradeSource { .. }
                    )
                ));
            }
            other => {
                panic!(
                    "Expected ImmediateUpgrade(Completed(UpgradeFailed)), got {other:?}"
                )
            }
        }
    }
}
