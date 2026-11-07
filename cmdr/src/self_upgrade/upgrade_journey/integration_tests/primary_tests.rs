// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Primary state-transition and cooldown integration tests for [`UpgradeJourney`].
//!
//! Covers:
//! 1. Single-start guard ([`StartOutcome::AlreadyStarted`]).
//! 2. Immediate upgrade ([`UpgradeStrategy::Immediate`]) sync/async `UpToDate`, cooldown
//!    bypass, `CheckFailed`, and `UpgradeFailed`.
//! 3. Deferred upgrade on exit ([`UpgradeStrategy::Deferred`]) cooldown skip, expiry,
//!    in-flight join on exit, and `Auto` failure handling.
//! 4. Disabled mode ([`UpgradePolicy::Disabled`]).
//!
//! [`StartOutcome::AlreadyStarted`]: crate::self_upgrade::StartOutcome::AlreadyStarted
//! [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
//! [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
//! [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred
//! [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate

use super::{test_fixtures::*, *};
use crate::{CooldownPolicy, CooldownStatus, CrateName, CrateVersion, EndpointUrl,
            JsoncScalar, ManifestCheckError, UpgradeActionOnExit, UpgradeConfig,
            UpgradeError, UpgradePolicy, UpgradeStrategy, UserFeedbackChannel,
            resolve_self_upgrade_last_check_in_base_dir};
use r3bl_tui::{assert_eq2, try_create_temp_dir};
#[cfg(not(doc))]
use serial_test::serial;
use std::{fs::File,
          net::TcpListener,
          time::{Duration, SystemTime}};

#[cfg_attr(not(doc), test)]
#[cfg_attr(not(doc), serial)]
pub fn test_single_start_guard_returns_already_started_without_mutating_singleton() {
    SingletonState::reset_for_testing();

    let make_config = |policy: UpgradePolicy| {
        UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            policy,
            EndpointUrl::default_manifest_url(),
            CooldownPolicy::Disabled,
        )
    };

    let disabled_config = make_config(UpgradePolicy::Disabled);
    let first = UpgradeJourney::new(disabled_config.clone());
    assert_eq2!(
        first.start(),
        StartOutcome::Disabled(UpgradeJourney::new(disabled_config).into_disabled())
    );

    // Duplicate Immediate call must return AlreadyStarted and leave SINGLETON
    // untouched.
    let dup_immediate = UpgradeJourney::new(make_config(UpgradePolicy::Enabled(
        UpgradeStrategy::Immediate,
    )));
    assert_eq2!(dup_immediate.start(), StartOutcome::AlreadyStarted);

    // Duplicate Deferred call must also return AlreadyStarted with 0 threads.
    let dup_thread = UpgradeJourney::new(make_config(UpgradePolicy::Enabled(
        UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto),
    )));
    let dup_outcome = dup_thread.start();
    assert_eq2!(dup_outcome, StartOutcome::AlreadyStarted);
    assert_eq2!(
        dup_outcome.handle_upgrade_on_exit(),
        ExitOutcome::NoDeferredCheckRunning
    );
}

#[cfg_attr(not(doc), test)]
#[cfg_attr(not(doc), serial)]
pub fn test_immediate_upgrade_up_to_date_sync_touches_and_bypasses_cooldown() {
    SingletonState::reset_for_testing();
    let crate_name: CrateName = "r3bl-cmdr".into();
    let temp_dir = try_create_temp_dir().unwrap();
    let timestamp_file_path =
        resolve_self_upgrade_last_check_in_base_dir(&temp_dir, &crate_name);

    // Pre-populate an active cooldown timestamp file to verify
    // `UpgradeStrategy::Immediate` (`--upgrade`) bypasses it and still performs the
    // live HTTP check.
    let (url, server_handle) =
        spawn_one_shot_http_server("200 OK", MANIFEST_UP_TO_DATE_JSONC, Duration::ZERO);
    let config = UpgradeConfig::new(
        crate_name,
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
        EndpointUrl::try_parse(url.as_str()).unwrap(),
        CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
    );
    config.get_cooldown_policy().touch(config.get_crate_name());
    let backdated_within_cooldown = SystemTime::now() - Duration::from_mins(30);
    File::open(&timestamp_file_path)
        .unwrap()
        .set_modified(backdated_within_cooldown)
        .unwrap();

    let outcome = UpgradeJourney::new(config).start();
    server_handle.join().unwrap();

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

#[cfg_attr(not(doc), tokio::test)]
#[cfg_attr(not(doc), serial)]
pub async fn test_immediate_upgrade_up_to_date_async() {
    SingletonState::reset_for_testing();
    let (url, server_handle) =
        spawn_one_shot_http_server("200 OK", MANIFEST_UP_TO_DATE_JSONC, Duration::ZERO);
    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
        EndpointUrl::try_parse(url.as_str()).unwrap(),
        CooldownPolicy::Disabled,
    );

    let outcome = UpgradeJourney::new(config).start();
    server_handle.join().unwrap();

    assert!(matches!(
        outcome,
        StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::UpToDate(_))
    ));
}

#[cfg_attr(not(doc), test)]
#[cfg_attr(not(doc), serial)]
pub fn test_immediate_upgrade_check_failed_and_upgrade_failed_transitions() {
    SingletonState::reset_for_testing();
    let temp_dir = try_create_temp_dir().unwrap();
    let timestamp_file_path = temp_dir.join("failed.last_check");

    // 1. Manifest HTTP 500 -> CheckFailed, no cooldown timestamp file.
    let (bad_url, bad_handle) = spawn_one_shot_http_server(
        "500 Internal Server Error",
        "server error",
        Duration::ZERO,
    );
    let fail_cfg = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
        EndpointUrl::try_parse(bad_url.as_str()).unwrap(),
        CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
    );
    let fail_outcome = UpgradeJourney::new(fail_cfg).start();
    bad_handle.join().unwrap();

    assert!(!timestamp_file_path.exists());
    match fail_outcome {
        StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::CheckFailed(journey)) => {
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
    let closed_release_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let closed_release_addr = closed_release_listener.local_addr().unwrap();
    drop(closed_release_listener);

    let (newer_url, newer_handle) = spawn_one_shot_http_server(
        "200 OK",
        with_release_base_url(
            MANIFEST_UPGRADE_AVAILABLE_CLOSED_SOURCE_JSONC,
            &format!("http://{closed_release_addr}/releases/download"),
        ),
        Duration::ZERO,
    );

    let upg_fail_cfg = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
        EndpointUrl::try_parse(newer_url.as_str()).unwrap(),
        CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
    );
    let upg_fail_outcome = UpgradeJourney::new(upg_fail_cfg).start();
    newer_handle.join().unwrap();

    assert!(!timestamp_file_path.exists());
    match upg_fail_outcome {
        StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::Completed(journey)) => {
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
            panic!("Expected ImmediateUpgrade(Completed(UpgradeFailed)), got {other:?}")
        }
    }
}

#[cfg_attr(not(doc), test)]
#[cfg_attr(not(doc), serial)]
pub fn test_deferred_upgrade_cooldown_skip_and_expiry() {
    SingletonState::reset_for_testing();
    let temp_dir = try_create_temp_dir().unwrap();
    let timestamp_file_path = temp_dir.join("cooldown.last_check");

    // 1. Active 1h cooldown timestamp file -> CooldownSkipped with 0 network/thread work.
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

    // 2. Expired timestamp file (`now - 2h`) -> spawns dedicated thread, queries server,
    //    refreshes timestamp file.
    SingletonState::reset_for_testing();
    let expired_time = SystemTime::now() - Duration::from_secs(7200);
    File::open(&timestamp_file_path)
        .unwrap()
        .set_modified(expired_time)
        .unwrap();

    let (url, server_handle) =
        spawn_one_shot_http_server("200 OK", MANIFEST_UP_TO_DATE_JSONC, Duration::ZERO);
    let expired_cfg = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto)),
        EndpointUrl::try_parse(url.as_str()).unwrap(),
        CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
    );

    let run_outcome = UpgradeJourney::new(expired_cfg.clone()).start();
    assert!(matches!(
        run_outcome,
        StartOutcome::DeferredUpgrade(DeferredUpgradeOutcome::DeferredCheckSpawned(_))
    ));

    let exit_outcome = run_outcome.handle_upgrade_on_exit();
    server_handle.join().unwrap();

    assert!(matches!(exit_outcome, ExitOutcome::UpToDate(_)));
    assert_eq2!(
        expired_cfg
            .get_cooldown_policy()
            .check_status(expired_cfg.get_crate_name()),
        CooldownStatus::Active
    );
}

#[cfg_attr(not(doc), test)]
#[cfg_attr(not(doc), serial)]
pub fn test_deferred_upgrade_exit_joins_inflight_dedicated_thread_and_completes() {
    SingletonState::reset_for_testing();

    // Simulate a short-lived CLI reaching `StartOutcome::handle_upgrade_on_exit()`
    // while the dedicated thread is still awaiting the HTTP response (`50ms`).
    let (url, server_handle) = spawn_one_shot_http_server(
        "200 OK",
        MANIFEST_UPGRADE_AVAILABLE_OPEN_SOURCE_JSONC,
        Duration::from_millis(50),
    );
    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Deferred(
            UpgradeActionOnExit::NotifyOnly,
        )),
        EndpointUrl::try_parse(url.as_str()).unwrap(),
        CooldownPolicy::Disabled,
    );

    let outcome = UpgradeJourney::new(config).start();
    assert!(matches!(
        outcome,
        StartOutcome::DeferredUpgrade(DeferredUpgradeOutcome::DeferredCheckSpawned(_))
    ));

    // Immediately invoke exit handler while manifest check is still in flight.
    let exit_outcome = outcome.handle_upgrade_on_exit();
    server_handle.join().unwrap();

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

#[cfg_attr(not(doc), test)]
#[cfg_attr(not(doc), serial)]
pub fn test_deferred_upgrade_exit_auto_failure_records_completed_without_cooldown() {
    SingletonState::reset_for_testing();
    let temp_dir = try_create_temp_dir().unwrap();
    let timestamp_file_path = temp_dir.join("auto_fail.last_check");

    let closed_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let closed_addr = closed_listener.local_addr().unwrap();
    drop(closed_listener);

    let (manifest_url, server_handle) = spawn_one_shot_http_server(
        "200 OK",
        with_release_base_url(
            MANIFEST_UPGRADE_AVAILABLE_CLOSED_SOURCE_JSONC,
            &format!("http://{closed_addr}/releases/download"),
        ),
        Duration::ZERO,
    );
    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto)),
        EndpointUrl::try_parse(manifest_url.as_str()).unwrap(),
        CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
    );

    let outcome = UpgradeJourney::new(config).start();
    assert!(matches!(
        outcome,
        StartOutcome::DeferredUpgrade(DeferredUpgradeOutcome::DeferredCheckSpawned(_))
    ));

    let exit_outcome = outcome.handle_upgrade_on_exit();
    server_handle.join().unwrap();

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

#[cfg_attr(not(doc), tokio::test)]
#[cfg_attr(not(doc), serial)]
pub async fn test_disabled_async_and_sync() {
    SingletonState::reset_for_testing();
    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Disabled,
        EndpointUrl::default_manifest_url(),
        CooldownPolicy::Disabled,
    );
    let outcome = UpgradeJourney::new(config.clone()).start();
    assert_eq2!(
        outcome,
        StartOutcome::Disabled(UpgradeJourney::new(config).into_disabled())
    );
    assert_eq2!(
        outcome.handle_upgrade_on_exit(),
        ExitOutcome::NoDeferredCheckRunning
    );
}
