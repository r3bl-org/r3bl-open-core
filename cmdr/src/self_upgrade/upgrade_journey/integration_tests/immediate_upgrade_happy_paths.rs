// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Happy-path integration tests for immediate upgrades
//! ([`UpgradeStrategy::Immediate`][1]) via [`UpgradeJourney`][2].
//!
//! Covers:
//! - `remote == local` (`0.0.25`) returning [`UpToDate`][3] and entering cooldown.
//! - `local > remote` (`0.0.26 > 0.0.25`, no downgrade) returning [`UpToDate`][3] and
//!   entering cooldown.
//! - In-cooldown state bypassed by `--upgrade` and its [`mtime`][4] refreshed.
//!
//! For happy-path `remote > current` installations under
//! [`UpgradeStrategy::Immediate`][1], see [`super::self_replace_happy_paths`] (release
//! archive) and [`super::install_via_rustup_happy_paths`] (`rustup` + `cargo` source
//! fallback).
//!
//! [1]: crate::self_upgrade::UpgradeStrategy::Immediate
//! [2]: crate::self_upgrade::UpgradeJourney
//! [3]: crate::self_upgrade::UpToDate
//! [4]: https://en.wikipedia.org/wiki/MAC_times

use super::{super::per_process_start_latch::PerProcessStartLatch, test_fixtures::*};
use crate::{CrateName, CrateVersion, GateStatus, JsoncScalar, RemoteVersionCheckGate,
            UpgradeConfig, UpgradePolicy, UpgradeStrategy, UserFeedbackChannel,
            r3bl_file_paths, self_upgrade::*};
use r3bl_tui::{TempDirChildPath, assert_eq2, try_create_temp_dir};
use serial_test::serial;
use std::{fs::File,
          time::{Duration, SystemTime}};

#[test]
#[serial]
fn test_remote_equals_local_returns_up_to_date_and_enters_cooldown_sync() {
    scenario_remote_equals_local_returns_up_to_date_and_enters_cooldown();
}

#[tokio::test]
#[serial]
async fn test_remote_equals_local_returns_up_to_date_and_enters_cooldown_async() {
    scenario_remote_equals_local_returns_up_to_date_and_enters_cooldown();
}

#[test]
#[serial]
fn test_local_newer_than_remote_returns_up_to_date_without_downgrade_sync() {
    scenario_local_newer_than_remote_returns_up_to_date_without_downgrade();
}

#[tokio::test]
#[serial]
async fn test_local_newer_than_remote_returns_up_to_date_without_downgrade_async() {
    scenario_local_newer_than_remote_returns_up_to_date_without_downgrade();
}

#[test]
#[serial]
fn test_in_cooldown_is_bypassed_by_immediate_and_refreshes_mtime_sync() {
    scenario_in_cooldown_is_bypassed_by_immediate_and_refreshes_mtime();
}

#[tokio::test]
#[serial]
async fn test_in_cooldown_is_bypassed_by_immediate_and_refreshes_mtime_async() {
    scenario_in_cooldown_is_bypassed_by_immediate_and_refreshes_mtime();
}

fn scenario_remote_equals_local_returns_up_to_date_and_enters_cooldown() {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let crate_name: CrateName = "r3bl-cmdr".into();
    let temp_dir = try_create_temp_dir().unwrap();
    let res_server_spawn = manifest_server::spawn_success(manifest_up_to_date());
    let config = UpgradeConfig::new(
        crate_name.clone(),
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
        outcome,
        StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::UpToDate(_))
    ));
    assert_eq2!(
        config
            .get_remote_version_check_gate()
            .try_check_gate_status()
            .unwrap(),
        GateStatus::ClosedForCooldown
    );
}

fn scenario_local_newer_than_remote_returns_up_to_date_without_downgrade() {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let crate_name: CrateName = "r3bl-cmdr".into();
    let temp_dir = try_create_temp_dir().unwrap();
    let res_server_spawn = manifest_server::spawn_success(manifest_up_to_date());
    let config = UpgradeConfig::new(
        crate_name.clone(),
        CrateVersion::try_parse("0.0.26").unwrap(),
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
        outcome,
        StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::UpToDate(_))
    ));
    assert_eq2!(
        config
            .get_remote_version_check_gate()
            .try_check_gate_status()
            .unwrap(),
        GateStatus::ClosedForCooldown
    );
}

fn scenario_in_cooldown_is_bypassed_by_immediate_and_refreshes_mtime() {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let crate_name: CrateName = "r3bl-cmdr".into();
    let temp_dir = try_create_temp_dir().unwrap();

    // Pre-populate an in-cooldown timestamp file to verify
    // `UpgradeStrategy::Immediate` (`--upgrade`) bypasses it and still performs the
    // live HTTP check.
    let res_server_spawn = manifest_server::spawn_success(manifest_up_to_date());
    let config = UpgradeConfig::new(
        crate_name.clone(),
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
        res_server_spawn.endpoint_url.clone(),
        RemoteVersionCheckGate::new_temp_dir(
            TempDirChildPath::try_new(&temp_dir, "").unwrap(),
            None,
        ),
    );
    let timestamp_file_path =
        temp_dir.join(r3bl_file_paths::constants::SELF_UPGRADE_LAST_CHECK_FILE);
    config
        .get_remote_version_check_gate()
        .try_touch_timestamp_file()
        .unwrap();
    let backdated_within_cooldown = SystemTime::now() - Duration::from_mins(30);
    File::options()
        .write(true)
        .open(&timestamp_file_path)
        .unwrap()
        .set_modified(backdated_within_cooldown)
        .unwrap();
    assert_eq2!(
        config
            .get_remote_version_check_gate()
            .try_check_gate_status()
            .unwrap(),
        GateStatus::ClosedForCooldown
    );

    // Act.
    let outcome = UpgradeJourney::new(config.clone()).start();

    // Assert.
    res_server_spawn.assert_each_mock_called_once();
    assert!(matches!(
        outcome,
        StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::UpToDate(_))
    ));
    assert_eq2!(
        config
            .get_remote_version_check_gate()
            .try_check_gate_status()
            .unwrap(),
        GateStatus::ClosedForCooldown
    );
    let refreshed_mtime = std::fs::metadata(&timestamp_file_path)
        .unwrap()
        .modified()
        .unwrap();
    assert!(refreshed_mtime > backdated_within_cooldown);
}
