// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Deferred upgrade on exit ([`UpgradeStrategy::Deferred`]) cooldown skip, expiry, and
//! in-flight join completion integration tests for [`UpgradeJourney`].
//!
//! [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
//! [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred

use super::test_fixtures::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CooldownPolicy, CooldownStatus, CrateVersion, EndpointUrl, JsoncScalar,
                UpgradeActionOnExit, UpgradeConfig, UpgradePolicy, UpgradeStrategy,
                UserFeedbackChannel, self_upgrade::*};
    use r3bl_tui::{assert_eq2, try_create_temp_dir};
    use serial_test::serial;
    use std::{fs::File,
              time::{Duration, SystemTime}};

    #[test]
    #[serial]
    fn test_deferred_upgrade_cooldown_skip_and_expiry() {
        SingletonState::reset_for_testing();
        
        let temp_dir = try_create_temp_dir().unwrap();
        let timestamp_file_path = temp_dir.join("cooldown.last_check");

        // 1. Active 1h cooldown timestamp file -> CooldownSkipped with 0 network/thread
        //    work.
        let active_cfg = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto)),
            EndpointUrl::try_parse("http://127.0.0.1:0/never_called.jsonc").unwrap(),
            CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
        );
        active_cfg
            .get_cooldown_policy()
            .touch(active_cfg.get_crate_name());

        let skip_outcome = UpgradeJourney::new(active_cfg).start();
        assert!(matches!(
            skip_outcome,
            StartOutcome::DeferredUpgrade(DeferredUpgradeOutcome::CooldownSkipped(_))
        ));
        assert_eq2!(
            skip_outcome.handle_upgrade_on_exit(),
            ExitOutcome::NoDeferredCheckRunning
        );

        // 2. Expired timestamp file (`now - 2h`) -> spawns dedicated thread, queries
        //    server, refreshes timestamp file.
        SingletonState::reset_for_testing();

        let expired_time = SystemTime::now() - Duration::from_secs(7200);
        File::open(&timestamp_file_path)
            .unwrap()
            .set_modified(expired_time)
            .unwrap();

        let server_spawn_result = manifest_server::spawn_success(manifest_up_to_date());
        let expired_cfg = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto)),
            server_spawn_result.url.clone(),
            CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
        );

        let run_outcome = UpgradeJourney::new(expired_cfg.clone()).start();
        assert!(matches!(
            run_outcome,
            StartOutcome::DeferredUpgrade(DeferredUpgradeOutcome::DeferredCheckSpawned(
                _
            ))
        ));

        let exit_outcome = run_outcome.handle_upgrade_on_exit();
        server_spawn_result.assert_each_endpoint_called_once();

        assert!(matches!(exit_outcome, ExitOutcome::UpToDate(_)));
        assert_eq2!(
            expired_cfg
                .get_cooldown_policy()
                .check_status(expired_cfg.get_crate_name()),
            CooldownStatus::Active
        );
    }

    #[test]
    #[serial]
    fn test_deferred_upgrade_exit_joins_inflight_dedicated_thread_and_completes() {
        SingletonState::reset_for_testing();

        // Simulate a short-lived CLI reaching `StartOutcome::handle_upgrade_on_exit()`
        // while the dedicated thread is still awaiting the HTTP response (`50ms`).
        let server_spawn_result = manifest_server::spawn_delayed_success(
            manifest_upgrade_available_open_source(&placeholder_release_base_url()),
            Duration::from_millis(50),
        );
        let config = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::Deferred(
                UpgradeActionOnExit::NotifyOnly,
            )),
            server_spawn_result.url.clone(),
            CooldownPolicy::Disabled,
        );

        let outcome = UpgradeJourney::new(config).start();
        assert!(matches!(
            outcome,
            StartOutcome::DeferredUpgrade(DeferredUpgradeOutcome::DeferredCheckSpawned(
                _
            ))
        ));

        // Immediately invoke exit handler while manifest check is still in flight.
        let exit_outcome = outcome.handle_upgrade_on_exit();
        server_spawn_result.assert_each_endpoint_called_once();

        match exit_outcome {
            ExitOutcome::Completed(journey) => {
                assert_eq2!(
                    journey.get_state().metadata.version,
                    CrateVersion::try_parse("9.9.9").unwrap()
                );
                assert_eq2!(
                    journey.get_state().resolution,
                    UpgradeResolution::NotifiedOnly
                );
            }
            other => panic!("Expected Completed(NotifiedOnly) after join, got {other:?}"),
        }
    }
}
