// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unhappy-path integration tests for deferred upgrades on exit
//! ([`UpgradeStrategy::Deferred`]):
//! - In-flight delayed HTTP 500 failure joined at exit returning
//!   [`ExitOutcome::CheckFailed`].
//! - Manifest connection refused returning [`ExitOutcome::CheckFailed`].
//! - [`CrateDistribution::ClosedSource`] `9.9.9` with unreachable release archive under
//!   [`UpgradeActionOnExit::Auto`] returning [`UpgradeResolution::UpgradeFailed`][1].
//! - Dedicated self-upgrade thread panic recovery returning [`ExitOutcome::CheckFailed`]
//!   with [`ThreadRunnerError::ThreadPanicked`] without unwinding the main thread.
//!
//! All failure paths leave no cooldown timestamp file on disk
//! ([`GateStatusError::MetadataReadFailed`] with [`ErrorKind::NotFound`]).
//!
//! [1]: crate::self_upgrade::UpgradeResolution::UpgradeFailed
//! [`CrateDistribution::ClosedSource`]: crate::self_upgrade::CrateDistribution::ClosedSource
//! [`ExitOutcome::CheckFailed`]: crate::self_upgrade::ExitOutcome::CheckFailed
//! [`GateStatusError::MetadataReadFailed`]: crate::self_upgrade::GateStatusError::MetadataReadFailed
//! [`ThreadRunnerError::ThreadPanicked`]: crate::self_upgrade::ThreadRunnerError::ThreadPanicked
//! [`UpgradeActionOnExit::Auto`]: crate::self_upgrade::UpgradeActionOnExit::Auto
//! [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred

use super::{super::{per_process_start_latch::PerProcessStartLatch,
                    thread_runner::ThreadRunner},
            test_fixtures::*};
use crate::{CheckFailedError, CrateDistribution, CrateVersion, EndpointUrl,
            FetchManifestError, GateStatusError, JsoncScalar, NoViableSourceReason,
            RemoteVersionCheckGate, ThreadRunnerError, UpgradeActionOnExit,
            UpgradeConfig, UpgradeError, UpgradeFailedError, UpgradePolicy,
            UpgradeStrategy, UserFeedbackChannel, self_upgrade::*};
use r3bl_tui::{TempDirChildPath, assert_eq2, try_create_temp_dir};
use serial_test::serial;
use std::{io::ErrorKind, time::Duration};

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

    // Simulate a short-lived CLI reaching
    // `DeferredUpgradeOutcome::handle_upgrade_on_exit()` while the dedicated thread is
    // still awaiting a delayed (`50ms`) HTTP 500 failure response.
    let res_server_spawn = manifest_server::spawn_delayed_error(
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
        res_server_spawn.endpoint_url.clone(),
        RemoteVersionCheckGate::new_temp_dir(
            TempDirChildPath::try_new(&temp_dir, "").unwrap(),
            None,
        ),
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
    res_server_spawn.assert_each_mock_called_once();
    assert!(matches!(
        config
            .get_remote_version_check_gate()
            .try_check_gate_status(),
        Err(GateStatusError::MetadataReadFailed { src_err, .. })
            if src_err.kind() == ErrorKind::NotFound
    ));
    match exit_outcome {
        ExitOutcome::CheckFailed(journey) => match journey.get_state().error {
            CheckFailedError::FetchManifest(
                FetchManifestError::HttpRequestFailed { ref src_err, .. },
            ) => {
                assert_eq2!(
                    src_err.status(),
                    Some(reqwest::StatusCode::INTERNAL_SERVER_ERROR)
                );
            }
            ref other => {
                panic!("Expected HttpRequestFailed(500), got {other:?}")
            }
        },
        other => panic!("Expected ExitOutcome::CheckFailed after join, got {other:?}"),
    }
}

fn scenario_manifest_connection_refused_returns_check_failed_without_cooldown() {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let temp_dir = try_create_temp_dir().unwrap();
    let dead_manifest_url = EndpointUrl::default_manifest_url().with_closed_socket_addr();
    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto)),
        dead_manifest_url,
        RemoteVersionCheckGate::new_temp_dir(
            TempDirChildPath::try_new(&temp_dir, "").unwrap(),
            None,
        ),
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
    assert!(matches!(
        config
            .get_remote_version_check_gate()
            .try_check_gate_status(),
        Err(GateStatusError::MetadataReadFailed { src_err, .. })
            if src_err.kind() == ErrorKind::NotFound
    ));
    match exit_outcome {
        ExitOutcome::CheckFailed(journey) => match journey.get_state().error {
            CheckFailedError::FetchManifest(
                FetchManifestError::HttpRequestFailed { ref src_err, .. },
            ) => {
                assert!(src_err.is_connect());
            }
            ref other => {
                panic!("Expected HttpRequestFailed(is_connect), got {other:?}")
            }
        },
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
    let dead_url = placeholder_release_base_url().with_closed_socket_addr();
    let res_server_spawn = manifest_server::spawn_success(
        manifest_upgrade_available_closed_source(&dead_url),
    );
    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto)),
        res_server_spawn.endpoint_url.clone(),
        RemoteVersionCheckGate::new_temp_dir(
            TempDirChildPath::try_new(&temp_dir, "").unwrap(),
            None,
        ),
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
    res_server_spawn.assert_each_mock_called_once();
    assert!(matches!(
        config
            .get_remote_version_check_gate()
            .try_check_gate_status(),
        Err(GateStatusError::MetadataReadFailed { src_err, .. })
            if src_err.kind() == ErrorKind::NotFound
    ));
    match exit_outcome {
        ExitOutcome::Completed(completed) => {
            assert!(matches!(
                completed.get_state().resolution,
                UpgradeResolution::UpgradeFailed(UpgradeFailedError::PerformUpgrade(
                    UpgradeError::NoViableUpgradeSource {
                        crate_distribution: CrateDistribution::ClosedSource,
                        reason: NoViableSourceReason::BinaryReleaseUnreachable,
                        ..
                    }
                ))
            ));
        }
        other => panic!("Expected Completed(UpgradeFailed), got {other:?}"),
    }
}

fn scenario_dedicated_thread_panic_returns_check_failed_without_unwinding() {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let temp_dir = try_create_temp_dir().unwrap();
    let strategy = UpgradeStrategy::Deferred(UpgradeActionOnExit::NotifyOnly);
    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(strategy),
        EndpointUrl::default_manifest_url(),
        RemoteVersionCheckGate::new_temp_dir(
            TempDirChildPath::try_new(&temp_dir, "").unwrap(),
            None,
        ),
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

    // Act: Main thread must catch `ThreadRunnerError::ThreadPanicked` without unwinding
    // and return `ExitOutcome::CheckFailed` with the panic payload preserved.
    let exit_outcome = deferred.handle_upgrade_on_exit();

    // Assert.
    assert!(matches!(
        config
            .get_remote_version_check_gate()
            .try_check_gate_status(),
        Err(GateStatusError::MetadataReadFailed { src_err, .. })
            if src_err.kind() == ErrorKind::NotFound
    ));
    match exit_outcome {
        ExitOutcome::CheckFailed(journey) => match journey.get_state().error {
            CheckFailedError::ThreadRunner(ThreadRunnerError::ThreadPanicked {
                ref panic_msg,
                ..
            }) => {
                assert!(
                    panic_msg.contains("simulated dedicated self-upgrade thread panic")
                );
            }
            ref other => {
                panic!(
                    "Expected CheckFailedError::ThreadRunner(ThreadPanicked), got {other:?}"
                )
            }
        },
        other => panic!("Expected ExitOutcome::CheckFailed, got {other:?}"),
    }
}
