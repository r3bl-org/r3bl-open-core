// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unhappy-path integration tests for immediate upgrades
//! ([`UpgradeStrategy::Immediate`]) via [`UpgradeJourney`].
//!
//! Covers:
//! - Manifest HTTP 500 returning [`ImmediateUpgradeOutcome::CheckFailed`].
//! - Manifest connection refused returning [`ImmediateUpgradeOutcome::CheckFailed`].
//! - Crate missing from manifest returning
//!   [`ManifestCheckError::CrateNotFoundInManifest`].
//! - [`CrateDistribution::ClosedSource`] `9.9.9` with unreachable release archive returning
//!   [`UpgradeResolution::UpgradeFailed`][1] with
//!   [`UpgradeError::NoViableUpgradeSource`].
//!
//! All failure paths leave cooldown as [`CooldownStatus::NotInCooldown`].
//!
//! [1]: crate::self_upgrade::UpgradeResolution::UpgradeFailed
//! [`CooldownStatus::NotInCooldown`]: crate::self_upgrade::CooldownStatus::NotInCooldown
//! [`CrateDistribution::ClosedSource`]: crate::self_upgrade::CrateDistribution::ClosedSource
//! [`ImmediateUpgradeOutcome::CheckFailed`]: crate::self_upgrade::ImmediateUpgradeOutcome::CheckFailed
//! [`ManifestCheckError::CrateNotFoundInManifest`]: crate::self_upgrade::ManifestCheckError::CrateNotFoundInManifest
//! [`UpgradeError::NoViableUpgradeSource`]: crate::self_upgrade::UpgradeError::NoViableUpgradeSource
//! [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
//! [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate

use super::{super::per_process_start_latch::PerProcessStartLatch, test_fixtures::*};
use crate::{CooldownPolicy, CooldownStatus, CrateDistribution, CrateVersion,
            EndpointUrl, JsoncScalar, ManifestCheckError, NoViableSourceReason,
            UpgradeConfig, UpgradeError, UpgradePolicy, UpgradeStrategy,
            UserFeedbackChannel, self_upgrade::*};
use r3bl_tui::{assert_eq2, try_create_temp_dir};
use serial_test::serial;

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
    let timestamp_file_path = temp_dir.join("http_500.last_check");
    let server_spawn_result =
        manifest_server::spawn_error("server error", blocks::HTTP_500_INTERNAL_SERVER_ERROR);
    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
        server_spawn_result.endpoint_url.clone(),
        CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
    );

    // Act.
    let outcome = UpgradeJourney::new(config.clone()).start();

    // Assert.
    server_spawn_result.assert_each_mock_called_once();
    assert!(!timestamp_file_path.exists());
    assert_eq2!(
        config
            .get_cooldown_policy()
            .check_status(config.get_crate_name()),
        CooldownStatus::NotInCooldown
    );
    match outcome {
        StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::CheckFailed(journey)) => {
            assert!(matches!(
                journey.get_state().error,
                ManifestCheckError::FetchOrParseFailed { ref diagnostic } if !diagnostic.is_empty()
            ));
        }
        other => panic!("Expected ImmediateUpgrade(CheckFailed), got {other:?}"),
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
        UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
        dead_manifest_url,
        CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
    );

    // Act.
    let outcome = UpgradeJourney::new(config.clone()).start();

    // Assert.
    assert!(!timestamp_file_path.exists());
    assert_eq2!(
        config
            .get_cooldown_policy()
            .check_status(config.get_crate_name()),
        CooldownStatus::NotInCooldown
    );
    match outcome {
        StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::CheckFailed(journey)) => {
            assert!(matches!(
                journey.get_state().error,
                ManifestCheckError::FetchOrParseFailed { ref diagnostic } if !diagnostic.is_empty()
            ));
        }
        other => panic!("Expected ImmediateUpgrade(CheckFailed), got {other:?}"),
    }
}

fn scenario_crate_missing_from_manifest_returns_check_failed_without_cooldown() {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let temp_dir = try_create_temp_dir().unwrap();
    let timestamp_file_path = temp_dir.join("missing_crate.last_check");
    let server_spawn_result =
        manifest_server::spawn_success(manifest_without_crate("some-other-crate"));
    let expected_url = server_spawn_result.endpoint_url.clone();
    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
        expected_url.clone(),
        CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
    );

    // Act.
    let outcome = UpgradeJourney::new(config.clone()).start();

    // Assert.
    server_spawn_result.assert_each_mock_called_once();
    assert!(!timestamp_file_path.exists());
    assert_eq2!(
        config
            .get_cooldown_policy()
            .check_status(config.get_crate_name()),
        CooldownStatus::NotInCooldown
    );
    match outcome {
        StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::CheckFailed(journey)) => {
            assert_eq2!(
                journey.get_state().error,
                ManifestCheckError::CrateNotFoundInManifest {
                    crate_name: "r3bl-cmdr".into(),
                    manifest_url: expected_url,
                }
            );
        }
        other => panic!("Expected ImmediateUpgrade(CheckFailed), got {other:?}"),
    }
}

fn scenario_closed_source_unreachable_archive_returns_upgrade_failed_without_cooldown() {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let temp_dir = try_create_temp_dir().unwrap();
    let timestamp_file_path = temp_dir.join("closed_source_fail.last_check");
    let dead_url = placeholder_release_base_url().with_closed_socket_addr();
    let server_spawn_result = manifest_server::spawn_success(
        manifest_upgrade_available_closed_source(&dead_url),
    );
    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
        server_spawn_result.endpoint_url.clone(),
        CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
    );

    // Act.
    let outcome = UpgradeJourney::new(config.clone()).start();

    // Assert.
    server_spawn_result.assert_each_mock_called_once();
    assert!(!timestamp_file_path.exists());
    assert_eq2!(
        config
            .get_cooldown_policy()
            .check_status(config.get_crate_name()),
        CooldownStatus::NotInCooldown
    );
    match outcome {
        StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::Completed(journey)) => {
            assert_eq2!(
                journey.get_state().metadata.version,
                CrateVersion::try_parse("9.9.9").unwrap()
            );
            assert!(matches!(
                journey.get_state().resolution,
                UpgradeResolution::UpgradeFailed(UpgradeError::NoViableUpgradeSource {
                    crate_distribution: CrateDistribution::ClosedSource,
                    reason: NoViableSourceReason::BinaryReleaseUnreachable,
                    ..
                })
            ));
        }
        other => {
            panic!("Expected ImmediateUpgrade(Completed(UpgradeFailed)), got {other:?}")
        }
    }
}
