// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unhappy-path integration tests for deferred upgrades on exit
//! ([`UpgradeStrategy::Deferred`]):
//! - In-flight delayed HTTP 500 failure joined at exit returning
//!   [`ExitOutcome::CheckFailed`].
//! - Manifest connection refused returning [`ExitOutcome::CheckFailed`].
//! - [`CrateDistribution::ClosedSource`] `9.9.9` with unreachable release archive under
//!   [`UpgradeActionOnExit::Auto`] returning [`UpgradeResolution::UpgradeFailed`][1].
//! - Dedicated self-upgrade thread panic recovery returning [`ExitOutcome::CheckFailed`]
//!   with [`ManifestCheckError::CheckThreadPanicked`] without unwinding the main thread.
//!
//! All failure paths leave cooldown as [`CooldownStatus::NotInCooldown`].
//!
//! [1]: crate::self_upgrade::UpgradeResolution::UpgradeFailed
//! [`CooldownStatus::NotInCooldown`]: crate::self_upgrade::CooldownStatus::NotInCooldown
//! [`CrateDistribution::ClosedSource`]: crate::self_upgrade::CrateDistribution::ClosedSource
//! [`ExitOutcome::CheckFailed`]: crate::self_upgrade::ExitOutcome::CheckFailed
//! [`ManifestCheckError::CheckThreadPanicked`]: crate::self_upgrade::ManifestCheckError::CheckThreadPanicked
//! [`UpgradeActionOnExit::Auto`]: crate::self_upgrade::UpgradeActionOnExit::Auto
//! [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred

use super::{super::{per_process_start_latch::PerProcessStartLatch,
                    thread_runner::ThreadRunner},
            test_fixtures::*};
use crate::{CooldownPolicy, CooldownStatus, CrateDistribution, CrateVersion,
            EndpointUrl, JsoncScalar, ManifestCheckError, NoViableSourceReason,
            UpgradeActionOnExit, UpgradeConfig, UpgradeError, UpgradePolicy,
            UpgradeStrategy, UserFeedbackChannel, self_upgrade::*};
use r3bl_tui::{assert_eq2, try_create_temp_dir};
use serial_test::serial;
use std::time::Duration;

#[test]
#[serial]
fn test_inflight_delayed_http_500_returns_check_failed_without_cooldown_sync() {
    scenario_inflight_delayed_http_500_returns_check_failed_without_cooldown();
}

#[tokio::test]
#[serial]
async fn test_inflight_delayed_http_500_returns_check_failed_without_cooldown_async() {
    scenario_inflight_delayed_http_500_returns_check_failed_without_cooldown();
}

#[test]
#[serial]
fn test_manifest_connection_refused_returns_check_failed_without_cooldown_sync() {
    scenario_manifest_connection_refused_returns_check_failed_without_cooldown();
}

#[tokio::test]
#[serial]
async fn test_manifest_connection_refused_returns_check_failed_without_cooldown_async() {
    scenario_manifest_connection_refused_returns_check_failed_without_cooldown();
}

#[test]
#[serial]
fn test_auto_closed_source_unreachable_archive_returns_upgrade_failed_without_cooldown_sync()
 {
    scenario_auto_closed_source_unreachable_archive_returns_upgrade_failed_without_cooldown();
}

#[tokio::test]
#[serial]
async fn test_auto_closed_source_unreachable_archive_returns_upgrade_failed_without_cooldown_async()
 {
    scenario_auto_closed_source_unreachable_archive_returns_upgrade_failed_without_cooldown();
}

#[test]
#[serial]
fn test_dedicated_thread_panic_returns_check_failed_without_unwinding_sync() {
    scenario_dedicated_thread_panic_returns_check_failed_without_unwinding();
}

#[tokio::test]
#[serial]
async fn test_dedicated_thread_panic_returns_check_failed_without_unwinding_async() {
    scenario_dedicated_thread_panic_returns_check_failed_without_unwinding();
}

fn scenario_inflight_delayed_http_500_returns_check_failed_without_cooldown() {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let temp_dir = try_create_temp_dir().unwrap();
    let timestamp_file_path = temp_dir.join("inflight_fail.last_check");

    // Simulate a short-lived CLI reaching
    // `DeferredUpgradeOutcome::handle_upgrade_on_exit()` while the dedicated thread is
    // still awaiting a delayed (`50ms`) HTTP 500 failure response.
    let server_spawn_result = manifest_server::spawn_delayed_error(
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
        server_spawn_result.endpoint_url.clone(),
        CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
    );

    // Act.
    let start_outcome = UpgradeJourney::new(config.clone()).start();
    let StartOutcome::DeferredUpgrade(deferred) = start_outcome else {
        panic!("Expected StartOutcome::DeferredUpgrade, got {start_outcome:?}");
    };
    assert!(matches!(
        deferred,
        DeferredUpgradeOutcome::DeferredCheckSpawned(_)
    ));
    let exit_outcome = deferred.handle_upgrade_on_exit();

    // Assert.
    server_spawn_result.assert_each_mock_called_once();
    assert!(!timestamp_file_path.exists());
    assert_eq2!(
        config
            .get_cooldown_policy()
            .check_status(config.get_crate_name()),
        CooldownStatus::NotInCooldown
    );
    match exit_outcome {
        ExitOutcome::CheckFailed(journey) => {
            assert!(matches!(
                journey.get_state().error,
                ManifestCheckError::FetchOrParseFailed { ref diagnostic } if !diagnostic.is_empty()
            ));
        }
        other => panic!("Expected ExitOutcome::CheckFailed after join, got {other:?}"),
    }
}

fn scenario_manifest_connection_refused_returns_check_failed_without_cooldown() {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let temp_dir = try_create_temp_dir().unwrap();
    let timestamp_file_path = temp_dir.join("conn_refused.last_check");
    let dead_manifest_url = EndpointUrl::default_manifest_url().with_closed_socket_addr();
    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto)),
        dead_manifest_url,
        CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
    );

    // Act.
    let start_outcome = UpgradeJourney::new(config.clone()).start();
    let StartOutcome::DeferredUpgrade(deferred) = start_outcome else {
        panic!("Expected StartOutcome::DeferredUpgrade, got {start_outcome:?}");
    };
    assert!(matches!(
        deferred,
        DeferredUpgradeOutcome::DeferredCheckSpawned(_)
    ));
    let exit_outcome = deferred.handle_upgrade_on_exit();

    // Assert.
    assert!(!timestamp_file_path.exists());
    assert_eq2!(
        config
            .get_cooldown_policy()
            .check_status(config.get_crate_name()),
        CooldownStatus::NotInCooldown
    );
    match exit_outcome {
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

fn scenario_auto_closed_source_unreachable_archive_returns_upgrade_failed_without_cooldown()
 {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let temp_dir = try_create_temp_dir().unwrap();
    let timestamp_file_path = temp_dir.join("auto_fail.last_check");
    let dead_url = placeholder_release_base_url().with_closed_socket_addr();
    let server_spawn_result = manifest_server::spawn_success(
        manifest_upgrade_available_closed_source(&dead_url),
    );
    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto)),
        server_spawn_result.endpoint_url.clone(),
        CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
    );

    // Act.
    let start_outcome = UpgradeJourney::new(config.clone()).start();
    let StartOutcome::DeferredUpgrade(deferred) = start_outcome else {
        panic!("Expected StartOutcome::DeferredUpgrade, got {start_outcome:?}");
    };
    assert!(matches!(
        deferred,
        DeferredUpgradeOutcome::DeferredCheckSpawned(_)
    ));
    let exit_outcome = deferred.handle_upgrade_on_exit();

    // Assert.
    server_spawn_result.assert_each_mock_called_once();
    assert!(!timestamp_file_path.exists());
    assert_eq2!(
        config
            .get_cooldown_policy()
            .check_status(config.get_crate_name()),
        CooldownStatus::NotInCooldown
    );
    match exit_outcome {
        ExitOutcome::Completed(completed) => {
            assert!(matches!(
                completed.get_state().resolution,
                UpgradeResolution::UpgradeFailed(UpgradeError::NoViableUpgradeSource {
                    crate_distribution: CrateDistribution::ClosedSource,
                    reason: NoViableSourceReason::BinaryReleaseUnreachable,
                    ..
                })
            ));
        }
        other => panic!("Expected Completed(UpgradeFailed), got {other:?}"),
    }
}

fn scenario_dedicated_thread_panic_returns_check_failed_without_unwinding() {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let temp_dir = try_create_temp_dir().unwrap();
    let timestamp_file_path = temp_dir.join("thread_panic.last_check");
    let strategy = UpgradeStrategy::Deferred(UpgradeActionOnExit::NotifyOnly);
    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(strategy),
        EndpointUrl::default_manifest_url(),
        CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
    );
    let checking = UpgradeJourney::new(config.clone()).into_checking(strategy);

    let join_handle = ThreadRunner::run_non_blocking(
        checking.get_config(),
        async move || -> DeferredCheckOutcome {
            panic!("simulated dedicated self-upgrade thread panic");
        },
    )
    .unwrap();

    let deferred = DeferredUpgradeOutcome::DeferredCheckSpawned(DeferredCheckHandle {
        checking,
        join_handle,
    });

    // Act: Main thread must catch `TokioRuntimeOutcome::Panicked` without unwinding and
    // return `ExitOutcome::CheckFailed` with the panic payload preserved.
    let exit_outcome = deferred.handle_upgrade_on_exit();

    // Assert.
    assert!(!timestamp_file_path.exists());
    assert_eq2!(
        config
            .get_cooldown_policy()
            .check_status(config.get_crate_name()),
        CooldownStatus::NotInCooldown
    );
    match exit_outcome {
        ExitOutcome::CheckFailed(journey) => match journey.get_state().error {
            ManifestCheckError::CheckThreadPanicked { ref panic_msg } => {
                assert!(
                    panic_msg.contains("simulated dedicated self-upgrade thread panic")
                );
            }
            ref other => {
                panic!("Expected ManifestCheckError::CheckThreadPanicked, got {other:?}")
            }
        },
        other => panic!("Expected ExitOutcome::CheckFailed, got {other:?}"),
    }
}
