// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unhappy-path integration tests for immediate upgrades
//! ([`UpgradeStrategy::Immediate`]) via [`UpgradeJourney`].
//!
//! Covers:
//! - Manifest HTTP 500 returning [`ImmediateUpgradeOutcome::CheckFailed`].
//! - Manifest connection refused returning [`ImmediateUpgradeOutcome::CheckFailed`].
//! - Crate missing from manifest returning
//!   [`FetchManifestError::CrateNotFoundInManifest`].
//! - [`CrateDistribution::ClosedSource`] `9.9.9` with unreachable release archive
//!   returning [`UpgradeResolution::UpgradeFailed`][1] with
//!   [`UpgradeError::NoViableUpgradeSource`].
//!
//! All failure paths leave no cooldown timestamp file on disk
//! ([`GateStatusError::MetadataReadFailed`] with [`ErrorKind::NotFound`]).
//!
//! [1]: crate::self_upgrade::UpgradeResolution::UpgradeFailed
//! [`CrateDistribution::ClosedSource`]: crate::self_upgrade::CrateDistribution::ClosedSource
//! [`FetchManifestError::CrateNotFoundInManifest`]: crate::self_upgrade::FetchManifestError::CrateNotFoundInManifest
//! [`GateStatusError::MetadataReadFailed`]: crate::self_upgrade::GateStatusError::MetadataReadFailed
//! [`ImmediateUpgradeOutcome::CheckFailed`]: crate::self_upgrade::ImmediateUpgradeOutcome::CheckFailed
//! [`UpgradeError::NoViableUpgradeSource`]: crate::self_upgrade::UpgradeError::NoViableUpgradeSource
//! [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
//! [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate

use super::{super::per_process_start_latch::PerProcessStartLatch, test_fixtures::*};
use crate::{CheckFailedError, CrateDistribution, CrateVersion, EndpointUrl,
            FetchManifestError, GateStatusError, JsoncScalar, NoViableSourceReason,
            RemoteVersionCheckGate, UpgradeConfig, UpgradeError, UpgradeFailedError,
            UpgradePolicy, UpgradeStrategy, UserFeedbackChannel, self_upgrade::*};
use r3bl_tui::{TempDirChildPath, assert_eq2, try_create_temp_dir};
use serial_test::serial;
use std::io::ErrorKind;

#[test]
#[serial]
fn test_manifest_http_500_returns_check_failed_without_cooldown_sync() {
    scenario_manifest_http_500_returns_check_failed_without_cooldown();
}

#[tokio::test]
#[serial]
async fn test_manifest_http_500_returns_check_failed_without_cooldown_async() {
    scenario_manifest_http_500_returns_check_failed_without_cooldown();
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
fn test_crate_missing_from_manifest_returns_check_failed_without_cooldown_sync() {
    scenario_crate_missing_from_manifest_returns_check_failed_without_cooldown();
}

#[tokio::test]
#[serial]
async fn test_crate_missing_from_manifest_returns_check_failed_without_cooldown_async() {
    scenario_crate_missing_from_manifest_returns_check_failed_without_cooldown();
}

#[test]
#[serial]
fn test_closed_source_unreachable_archive_returns_upgrade_failed_without_cooldown_sync() {
    scenario_closed_source_unreachable_archive_returns_upgrade_failed_without_cooldown();
}

#[tokio::test]
#[serial]
async fn test_closed_source_unreachable_archive_returns_upgrade_failed_without_cooldown_async()
 {
    scenario_closed_source_unreachable_archive_returns_upgrade_failed_without_cooldown();
}

fn scenario_manifest_http_500_returns_check_failed_without_cooldown() {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let temp_dir = try_create_temp_dir().unwrap();
    let res_server_spawn = manifest_server::spawn_error(
        "server error",
        blocks::HTTP_500_INTERNAL_SERVER_ERROR,
    );
    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
        res_server_spawn.endpoint_url.clone(),
        RemoteVersionCheckGate::new_temp_dir(
            TempDirChildPath::try_new(&temp_dir, "").unwrap(),
            None,
        ),
    );

    // Act.
    let outcome = UpgradeJourney::new(config.clone()).start();

    // Assert.
    res_server_spawn.assert_each_mock_called_once();
    assert!(matches!(
        config
            .get_remote_version_check_gate()
            .try_check_gate_status(),
        Err(GateStatusError::MetadataReadFailed { src_err, .. })
            if src_err.kind() == ErrorKind::NotFound
    ));
    match outcome {
        StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::CheckFailed(journey)) => {
            match journey.get_state().error {
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
            }
        }
        other => panic!("Expected ImmediateUpgrade(CheckFailed), got {other:?}"),
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
        UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
        dead_manifest_url,
        RemoteVersionCheckGate::new_temp_dir(
            TempDirChildPath::try_new(&temp_dir, "").unwrap(),
            None,
        ),
    );

    // Act.
    let outcome = UpgradeJourney::new(config.clone()).start();

    // Assert.
    assert!(matches!(
        config
            .get_remote_version_check_gate()
            .try_check_gate_status(),
        Err(GateStatusError::MetadataReadFailed { src_err, .. })
            if src_err.kind() == ErrorKind::NotFound
    ));
    match outcome {
        StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::CheckFailed(journey)) => {
            match journey.get_state().error {
                CheckFailedError::FetchManifest(
                    FetchManifestError::HttpRequestFailed { ref src_err, .. },
                ) => {
                    assert!(src_err.is_connect());
                }
                ref other => {
                    panic!("Expected HttpRequestFailed(is_connect), got {other:?}")
                }
            }
        }
        other => panic!("Expected ImmediateUpgrade(CheckFailed), got {other:?}"),
    }
}

fn scenario_crate_missing_from_manifest_returns_check_failed_without_cooldown() {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let temp_dir = try_create_temp_dir().unwrap();
    let res_server_spawn =
        manifest_server::spawn_success(manifest_without_crate("some-other-crate"));
    let expected_url = res_server_spawn.endpoint_url.clone();
    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
        expected_url.clone(),
        RemoteVersionCheckGate::new_temp_dir(
            TempDirChildPath::try_new(&temp_dir, "").unwrap(),
            None,
        ),
    );

    // Act.
    let outcome = UpgradeJourney::new(config.clone()).start();

    // Assert.
    res_server_spawn.assert_each_mock_called_once();
    assert!(matches!(
        config
            .get_remote_version_check_gate()
            .try_check_gate_status(),
        Err(GateStatusError::MetadataReadFailed { src_err, .. })
            if src_err.kind() == ErrorKind::NotFound
    ));
    match outcome {
        StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::CheckFailed(journey)) => {
            match journey.get_state().error {
                CheckFailedError::FetchManifest(
                    FetchManifestError::CrateNotFoundInManifest {
                        ref crate_name,
                        ref manifest_url,
                    },
                ) => {
                    assert_eq2!(crate_name, &"r3bl-cmdr".into());
                    assert_eq2!(manifest_url, &expected_url);
                }
                ref other => {
                    panic!("Expected CrateNotFoundInManifest, got {other:?}")
                }
            }
        }
        other => panic!("Expected ImmediateUpgrade(CheckFailed), got {other:?}"),
    }
}

fn scenario_closed_source_unreachable_archive_returns_upgrade_failed_without_cooldown() {
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
        UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
        res_server_spawn.endpoint_url.clone(),
        RemoteVersionCheckGate::new_temp_dir(
            TempDirChildPath::try_new(&temp_dir, "").unwrap(),
            None,
        ),
    );

    // Act.
    let outcome = UpgradeJourney::new(config.clone()).start();

    // Assert.
    res_server_spawn.assert_each_mock_called_once();
    assert!(matches!(
        config
            .get_remote_version_check_gate()
            .try_check_gate_status(),
        Err(GateStatusError::MetadataReadFailed { src_err, .. })
            if src_err.kind() == ErrorKind::NotFound
    ));
    match outcome {
        StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::Completed(journey)) => {
            assert_eq2!(
                journey.get_state().metadata.version,
                CrateVersion::try_parse("9.9.9").unwrap()
            );
            assert!(matches!(
                journey.get_state().resolution,
                UpgradeResolution::UpgradeFailed(UpgradeFailedError::PerformUpgrade(
                    UpgradeError::NoViableUpgradeSource {
                        crate_distribution: CrateDistribution::ClosedSource,
                        reason: NoViableSourceReason::BinaryReleaseUnreachable,
                        ..
                    }
                ))
            ));
        }
        other => {
            panic!("Expected ImmediateUpgrade(Completed(UpgradeFailed)), got {other:?}")
        }
    }
}
