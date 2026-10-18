// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unhappy-path integration tests for deferred upgrade on exit
//! ([`UpgradeStrategy::Deferred`]):
//! 1. HTTP client timeout (`20ms` timeout vs. `100ms` delayed server) returning
//!    [`ManifestCheckError::FetchOrParseFailed`].
//! 2. In-flight delayed HTTP 500 failure joined at exit returning
//!    [`ExitOutcome::CheckFailed`].
//! 3. Manifest HTTP 500 and connection-refused errors returning
//!    [`ExitOutcome::CheckFailed`] without touching the cooldown timestamp file.
//! 4. Dedicated self-upgrade thread panic recovery returning
//!    [`ExitOutcome::NoDeferredCheckRunning`] without unwinding the main thread.
//! 5. [`UpgradeActionOnExit::Auto`] binary download failure recording
//!    [`UpgradeResolution::UpgradeFailed`] without touching the cooldown timestamp file.
//!
//! [`ExitOutcome::CheckFailed`]: crate::self_upgrade::ExitOutcome::CheckFailed
//! [`ExitOutcome::NoDeferredCheckRunning`]: crate::self_upgrade::ExitOutcome::NoDeferredCheckRunning
//! [`ManifestCheckError::FetchOrParseFailed`]: crate::self_upgrade::ManifestCheckError::FetchOrParseFailed
//! [`UpgradeActionOnExit::Auto`]: crate::self_upgrade::UpgradeActionOnExit::Auto
//! [`UpgradeResolution::UpgradeFailed`]: crate::self_upgrade::UpgradeResolution::UpgradeFailed
//! [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred

use super::test_fixtures::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CooldownPolicy, CrateName, CrateVersion, JsoncScalar,
                ManifestCheckError, UpgradeActionOnExit, UpgradeConfig, UpgradeError,
                UpgradePolicy, UpgradeStrategy, UserFeedbackChannel, self_upgrade::*};
    use r3bl_tui::{assert_eq2, try_create_temp_dir};
    use serial_test::serial;
    use std::time::Duration;

    #[tokio::test]
    #[serial]
    async fn test_fetch_crate_metadata_from_manifest_timeout_returns_fetch_or_parse_failed()
     {
        let crate_name: CrateName = "r3bl-cmdr".into();
        let server = manifest_server::spawn_delayed_success(
            manifest_upgrade_available_open_source(&placeholder_release_base_url()),
            Duration::from_millis(100),
        );

        let result = fetch_crate_metadata_from_manifest(
            &server.url,
            &crate_name,
            Duration::from_millis(20),
        )
        .await;
        server.join_handle.join().unwrap();

        match result {
            Err(ManifestCheckError::FetchOrParseFailed { ref diagnostic }) => {
                assert!(!diagnostic.is_empty());
            }
            other => panic!("Expected Err(FetchOrParseFailed) on timeout, got {other:?}"),
        }
    }

    #[test]
    #[serial]
    fn test_deferred_upgrade_exit_joins_inflight_delayed_failure_and_returns_check_failed()
     {
        SingletonState::reset_for_testing();
        let temp_dir = try_create_temp_dir().unwrap();
        let timestamp_file_path = temp_dir.join("inflight_fail.last_check");

        // Simulate a short-lived CLI reaching `StartOutcome::handle_upgrade_on_exit()`
        // while the dedicated thread is still awaiting a delayed (`50ms`) HTTP 500
        // failure response.
        let server = manifest_server::spawn_delayed_error(
            "delayed server failure",
            blocks::HTTP_500_INTERNAL_SERVER_ERROR,
            Duration::from_millis(50),
        );
        let config = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::Deferred(
                UpgradeActionOnExit::NotifyOnly,
            )),
            server.url,
            CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
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
        server.join_handle.join().unwrap();

        assert!(!timestamp_file_path.exists());
        match exit_outcome {
            ExitOutcome::CheckFailed(journey) => {
                assert!(matches!(
                    journey.get_state().error,
                    ManifestCheckError::FetchOrParseFailed { ref diagnostic } if !diagnostic.is_empty()
                ));
            }
            other => {
                panic!("Expected ExitOutcome::CheckFailed after join, got {other:?}")
            }
        }
    }

    #[test]
    #[serial]
    fn test_deferred_upgrade_exit_manifest_network_failure_returns_check_failed_without_cooldown()
     {
        let temp_dir = try_create_temp_dir().unwrap();
        let timestamp_file_path = temp_dir.join("net_fail.last_check");

        // 1. Manifest HTTP 500 error -> ExitOutcome::CheckFailed, no cooldown file.
        SingletonState::reset_for_testing();
        let bad_server =
            manifest_server::spawn_error("server error", blocks::HTTP_500_INTERNAL_SERVER_ERROR);
        let http_500_cfg = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto)),
            bad_server.url,
            CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
        );

        let http_500_start = UpgradeJourney::new(http_500_cfg).start();
        assert!(matches!(
            http_500_start,
            StartOutcome::DeferredUpgrade(DeferredUpgradeOutcome::DeferredCheckSpawned(
                _
            ))
        ));
        let http_500_exit = http_500_start.handle_upgrade_on_exit();
        bad_server.join_handle.join().unwrap();

        assert!(!timestamp_file_path.exists());
        match http_500_exit {
            ExitOutcome::CheckFailed(journey) => {
                assert!(matches!(
                    journey.get_state().error,
                    ManifestCheckError::FetchOrParseFailed { ref diagnostic } if !diagnostic.is_empty()
                ));
            }
            other => {
                panic!("Expected ExitOutcome::CheckFailed on HTTP 500, got {other:?}")
            }
        }

        // 2. Connection refused (closed socket) -> ExitOutcome::CheckFailed, no cooldown
        //    file.
        SingletonState::reset_for_testing();
        let dead_manifest_url = placeholder_release_base_url().with_closed_socket_addr();
        let refused_cfg = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto)),
            dead_manifest_url,
            CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
        );

        let refused_start = UpgradeJourney::new(refused_cfg).start();
        assert!(matches!(
            refused_start,
            StartOutcome::DeferredUpgrade(DeferredUpgradeOutcome::DeferredCheckSpawned(
                _
            ))
        ));
        let refused_exit = refused_start.handle_upgrade_on_exit();

        assert!(!timestamp_file_path.exists());
        match refused_exit {
            ExitOutcome::CheckFailed(journey) => {
                assert!(matches!(
                    journey.get_state().error,
                    ManifestCheckError::FetchOrParseFailed { ref diagnostic } if !diagnostic.is_empty()
                ));
            }
            other => panic!(
                "Expected ExitOutcome::CheckFailed on connection refused, got {other:?}"
            ),
        }
    }

    #[test]
    #[serial]
    fn test_deferred_upgrade_exit_recovers_when_dedicated_thread_panics() {
        SingletonState::reset_for_testing();

        let strategy = UpgradeStrategy::Deferred(UpgradeActionOnExit::NotifyOnly);
        let config = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(strategy),
            EndpointUrl::default_manifest_url(),
            CooldownPolicy::Disabled,
        );
        let checking = UpgradeJourney::new(config).into_checking(strategy);

        let handle = ThreadRunner::run_non_blocking(
            checking.get_config(),
            async move || -> DeferredCheckOutcome {
                panic!("simulated dedicated self-upgrade thread panic");
            },
        )
        .unwrap();
        SingletonState::set_deferred_check_handle(handle);

        let outcome = StartOutcome::DeferredUpgrade(
            DeferredUpgradeOutcome::DeferredCheckSpawned(checking),
        );

        // Main thread must catch `TokioRuntimeOutcome::Panicked` without unwinding and
        // return `ExitOutcome::NoDeferredCheckRunning`.
        assert_eq2!(
            outcome.handle_upgrade_on_exit(),
            ExitOutcome::NoDeferredCheckRunning
        );
        assert_eq2!(
            outcome.handle_upgrade_on_exit(),
            ExitOutcome::NoDeferredCheckRunning
        );
    }

    #[test]
    #[serial]
    fn test_deferred_upgrade_exit_auto_failure_records_completed_without_cooldown() {
        SingletonState::reset_for_testing();
        let temp_dir = try_create_temp_dir().unwrap();
        let timestamp_file_path = temp_dir.join("auto_fail.last_check");

        let dead_url = placeholder_release_base_url().with_closed_socket_addr();
        let server = manifest_server::spawn_success(
            manifest_upgrade_available_closed_source(&dead_url),
        );
        let config = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto)),
            server.url,
            CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
        );

        let outcome = UpgradeJourney::new(config).start();
        assert!(matches!(
            outcome,
            StartOutcome::DeferredUpgrade(DeferredUpgradeOutcome::DeferredCheckSpawned(
                _
            ))
        ));

        let exit_outcome = outcome.handle_upgrade_on_exit();
        server.join_handle.join().unwrap();

        assert!(!timestamp_file_path.exists());
        match exit_outcome {
            ExitOutcome::Completed(completed) => {
                assert!(matches!(
                    completed.get_state().resolution,
                    UpgradeResolution::UpgradeFailed(
                        UpgradeError::NoViableUpgradeSource { .. }
                    )
                ));
            }
            other => panic!("Expected Completed(UpgradeFailed), got {other:?}"),
        }
    }
}
