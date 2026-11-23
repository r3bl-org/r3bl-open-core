// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Happy-path integration tests for deferred upgrades on exit
//! ([`UpgradeStrategy::Deferred`]) via [`UpgradeJourney`].
//!
//! Covers:
//! - In-cooldown state skipping the check with `0` HTTP calls ([`ExitOutcome::Skipped`]).
//! - Not-in-cooldown state
//!   ([`RemoteVersionCheckGate::backdate_timestamp_file_for_testing`]) spawning the check
//!   thread and re-entering cooldown on [`ExitOutcome::UpToDate`].
//! - In-flight manifest check joined at exit with [`UpgradeActionOnExit::NotifyOnly`]
//!   returning [`UpgradeResolution::NotifiedOnly`] without entering cooldown.
//! - [`UpgradeActionOnExit::PromptToUpgrade`] in an isolated non-TTY subprocess returning
//!   [`UpgradeResolution::SkippedNonInteractive`] without entering cooldown.
//!
//! For happy-path `remote > current` installations under
//! [`UpgradeActionOnExit::Auto`], see [`super::self_replace_happy_paths`] (release
//! archive) and [`super::install_via_rustup_happy_paths`] (`rustup` + `cargo` source
//! fallback).
//!
//! [`ExitOutcome::Skipped`]: crate::self_upgrade::ExitOutcome::Skipped
//! [`ExitOutcome::UpToDate`]: crate::self_upgrade::ExitOutcome::UpToDate
//! [`RemoteVersionCheckGate::backdate_timestamp_file_for_testing`]: crate::self_upgrade::RemoteVersionCheckGate::backdate_timestamp_file_for_testing
//! [`UpgradeActionOnExit::Auto`]: crate::self_upgrade::UpgradeActionOnExit::Auto
//! [`UpgradeActionOnExit::NotifyOnly`]: crate::self_upgrade::UpgradeActionOnExit::NotifyOnly
//! [`UpgradeActionOnExit::PromptToUpgrade`]: crate::self_upgrade::UpgradeActionOnExit::PromptToUpgrade
//! [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
//! [`UpgradeResolution::NotifiedOnly`]: crate::self_upgrade::UpgradeResolution::NotifiedOnly
//! [`UpgradeResolution::SkippedNonInteractive`]: crate::self_upgrade::UpgradeResolution::SkippedNonInteractive
//! [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred

use super::{super::per_process_start_latch::PerProcessStartLatch, test_fixtures::*};
use crate::{CrateName, CrateVersion, GateStatus, GateStatusError, JsoncScalar,
            RemoteVersionCheckGate, UpgradeActionOnExit, UpgradeConfig, UpgradePolicy,
            UpgradeStrategy, UserFeedbackChannel, self_upgrade::*};
use r3bl_tui::{TempDirChildPath, TerminalInteractiveStatus, assert_eq2,
               check_is_terminal_interactive, try_create_temp_dir};
use serial_test::serial;
use std::{env,
          io::ErrorKind,
          process::{Command, Stdio},
          time::Duration};

const ISOLATED_NON_TTY_PROMPT_ENV_VAR: &str = "R3BL_TEST_DEFERRED_NON_TTY_PROMPT";
const NON_TTY_SYNC_TEST_FILTER: &str = "self_upgrade::upgrade_journey::integration_tests::deferred_upgrade_happy_paths::test_prompt_to_upgrade_in_non_tty_returns_skipped_non_interactive_sync";
const NON_TTY_ASYNC_TEST_FILTER: &str = "self_upgrade::upgrade_journey::integration_tests::deferred_upgrade_happy_paths::test_prompt_to_upgrade_in_non_tty_returns_skipped_non_interactive_async";

#[test]
#[serial]
fn test_in_cooldown_skips_with_zero_http_calls_sync() {
    scenario_in_cooldown_skips_with_zero_http_calls();
}

#[tokio::test]
#[serial]
async fn test_in_cooldown_skips_with_zero_http_calls_async() {
    scenario_in_cooldown_skips_with_zero_http_calls();
}

#[test]
#[serial]
fn test_not_in_cooldown_spawns_check_and_reenters_cooldown_on_up_to_date_sync() {
    scenario_not_in_cooldown_spawns_check_and_reenters_cooldown_on_up_to_date();
}

#[tokio::test]
#[serial]
async fn test_not_in_cooldown_spawns_check_and_reenters_cooldown_on_up_to_date_async() {
    scenario_not_in_cooldown_spawns_check_and_reenters_cooldown_on_up_to_date();
}

#[test]
#[serial]
fn test_inflight_check_joined_at_exit_with_notify_only_stays_not_in_cooldown_sync() {
    scenario_inflight_check_joined_at_exit_with_notify_only_stays_not_in_cooldown();
}

#[tokio::test]
#[serial]
async fn test_inflight_check_joined_at_exit_with_notify_only_stays_not_in_cooldown_async()
{
    scenario_inflight_check_joined_at_exit_with_notify_only_stays_not_in_cooldown();
}

#[test]
fn test_prompt_to_upgrade_in_non_tty_returns_skipped_non_interactive_sync() {
    if env::var(ISOLATED_NON_TTY_PROMPT_ENV_VAR).is_ok() {
        run_non_tty_prompt_worker();
        return;
    }
    run_non_tty_prompt_coordinator(NON_TTY_SYNC_TEST_FILTER);
}

#[tokio::test]
async fn test_prompt_to_upgrade_in_non_tty_returns_skipped_non_interactive_async() {
    if env::var(ISOLATED_NON_TTY_PROMPT_ENV_VAR).is_ok() {
        run_non_tty_prompt_worker();
        return;
    }
    run_non_tty_prompt_coordinator(NON_TTY_ASYNC_TEST_FILTER);
}

fn scenario_in_cooldown_skips_with_zero_http_calls() {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let crate_name: CrateName = "r3bl-cmdr".into();
    let temp_dir = try_create_temp_dir().unwrap();
    let res_server_spawn = manifest_server::spawn_uncalled();
    let config = UpgradeConfig::new(
        crate_name.clone(),
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto)),
        res_server_spawn.endpoint_url.clone(),
        RemoteVersionCheckGate::new_temp_dir(
            TempDirChildPath::try_new(&temp_dir, "").unwrap(),
            None,
        ),
    );
    config
        .get_remote_version_check_gate()
        .try_touch_timestamp_file()
        .unwrap();
    assert_eq2!(
        config
            .get_remote_version_check_gate()
            .try_check_gate_status()
            .unwrap(),
        GateStatus::ClosedForCooldown
    );

    // Act.
    let start_outcome = UpgradeJourney::new(config.clone()).start();
    let StartOutcome::DeferredUpgrade(deferred) = start_outcome else {
        panic!("Expected StartOutcome::DeferredUpgrade, got {start_outcome:?}");
    };
    assert!(matches!(deferred, DeferredUpgradeOutcome::Skipped(_)));
    let exit_outcome = deferred.handle_upgrade_on_exit();

    // Assert.
    res_server_spawn.assert_expectations();
    match exit_outcome {
        ExitOutcome::Skipped(skipped_journey) => {
            assert_eq2!(
                skipped_journey,
                UpgradeJourney::new(config.clone()).into_skipped()
            );
        }
        other => panic!("Expected ExitOutcome::Skipped, got {other:?}"),
    }
    assert_eq2!(
        config
            .get_remote_version_check_gate()
            .try_check_gate_status()
            .unwrap(),
        GateStatus::ClosedForCooldown
    );
}

fn scenario_not_in_cooldown_spawns_check_and_reenters_cooldown_on_up_to_date() {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let crate_name: CrateName = "r3bl-cmdr".into();
    let temp_dir = try_create_temp_dir().unwrap();
    let remote_version_check_gate = RemoteVersionCheckGate::new_temp_dir(
        TempDirChildPath::try_new(&temp_dir, "").unwrap(),
        None,
    );
    remote_version_check_gate.backdate_timestamp_file_for_testing();
    assert_eq2!(
        remote_version_check_gate.try_check_gate_status().unwrap(),
        GateStatus::Open
    );

    let res_server_spawn = manifest_server::spawn_success(manifest_up_to_date());
    let config = UpgradeConfig::new(
        crate_name.clone(),
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto)),
        res_server_spawn.endpoint_url.clone(),
        remote_version_check_gate,
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
    assert!(matches!(exit_outcome, ExitOutcome::UpToDate(_)));
    assert_eq2!(
        config
            .get_remote_version_check_gate()
            .try_check_gate_status()
            .unwrap(),
        GateStatus::ClosedForCooldown
    );
}

fn scenario_inflight_check_joined_at_exit_with_notify_only_stays_not_in_cooldown() {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let temp_dir = try_create_temp_dir().unwrap();
    let res_server_spawn = manifest_server::spawn_delayed_success(
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
        ExitOutcome::Completed(journey) => {
            assert_eq2!(
                journey.get_state().metadata.version,
                CrateVersion::try_parse("9.9.9").unwrap()
            );
            assert!(matches!(
                journey.get_state().resolution,
                UpgradeResolution::NotifiedOnly
            ));
        }
        other => panic!("Expected Completed(NotifiedOnly) after join, got {other:?}"),
    }
}

/// Spawns `test_filter` inside a headless child process with [`stdin`] bound to
/// `/dev/null` and [`stdout`]/[`stderr`] piped so [`check_is_terminal_interactive`]
/// reliably sees a non-TTY environment even when `cargo test` runs inside an interactive
/// terminal.
///
/// [`stderr`]: std::io::stderr
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
fn run_non_tty_prompt_coordinator(test_filter: &str) {
    let output = Command::new(env::current_exe().unwrap())
        .env(ISOLATED_NON_TTY_PROMPT_ENV_VAR, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .arg("--exact")
        .arg(test_filter)
        .arg("--nocapture")
        .output()
        .expect("Failed to spawn isolated non-TTY prompt worker subprocess");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "Isolated non-TTY prompt worker failed ({}):\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
        output.status,
    );
}

fn run_non_tty_prompt_worker() {
    // Arrange.
    assert!(
        matches!(
            check_is_terminal_interactive(),
            TerminalInteractiveStatus::NotAvailable(_)
        ),
        "Non-TTY prompt worker must not run with an interactive terminal"
    );
    PerProcessStartLatch::reset_for_testing();
    let temp_dir = try_create_temp_dir().unwrap();
    let res_server_spawn = manifest_server::spawn_success(
        manifest_upgrade_available_open_source(&placeholder_release_base_url()),
    );
    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Deferred(
            UpgradeActionOnExit::PromptToUpgrade,
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
        ExitOutcome::Completed(journey) => {
            assert_eq2!(
                journey.get_state().metadata.version,
                CrateVersion::try_parse("9.9.9").unwrap()
            );
            assert!(matches!(
                journey.get_state().resolution,
                UpgradeResolution::SkippedNonInteractive
            ));
        }
        other => panic!(
            "Expected Completed(SkippedNonInteractive) in non-TTY subprocess, got {other:?}"
        ),
    }
}
