// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Subsystem end-to-end integration tests for [`UpgradeJourney`].
//!
//! [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney

use super::*;
use crate::{CooldownPolicy, CooldownStatus, CrateName, CrateVersion, EndpointUrl,
            JsoncScalar, ManifestCheckError, UpgradeActionOnExit, UpgradeConfig,
            UpgradeError, UpgradePolicy, UpgradeStrategy, UserFeedbackChannel,
            resolve_self_upgrade_last_check_in_base_dir};
use r3bl_tui::{assert_eq2, try_create_temp_dir};
use serial_test::serial;
use std::{fs::File,
          io::{Read, Write},
          net::TcpListener,
          time::{Duration, SystemTime}};

fn format_manifest_json(version: &str, distribution: &str) -> String {
    format_manifest_json_with_release_url(
        "https://example.com/releases/download",
        version,
        distribution,
    )
}

fn format_manifest_json_with_release_url(
    release_url: &str,
    version: &str,
    distribution: &str,
) -> String {
    format!(
        r#"{{
            "binaries_release_base_url": "{release_url}",
            "crates": {{
                "r3bl-cmdr": {{
                    "version": "{version}",
                    "crate_distribution": "{distribution}",
                    "released_on": "2026-04-06T12:00:00Z"
                }}
            }}
        }}"#
    )
}

fn spawn_one_shot_http_server(
    status: &'static str,
    body: String,
    delay: Duration,
) -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut req_buf = [0_u8; 1024];
            drop(stream.read(&mut req_buf));
            if !delay.is_zero() {
                std::thread::sleep(delay);
            }
            let resp = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            drop(stream.write_all(resp.as_bytes()));
            drop(stream.flush());
        }
    });
    (format!("http://{addr}/versions.jsonc"), handle)
}

#[test]
#[serial]
fn test_duplicate_start_returns_already_started_without_mutating_singleton() {
    UpgradeSingletonState::reset_for_testing();

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
    assert_eq2!(dup_thread.start(), StartOutcome::AlreadyStarted);
    assert_eq2!(
        handle_upgrade_on_exit(),
        ExitOutcome::NoDeferredCheckRunning
    );
}

#[test]
#[serial]
fn test_path1_immediate_up_to_date_sync_touches_and_bypasses_cooldown() {
    UpgradeSingletonState::reset_for_testing();
    let crate_name: CrateName = "r3bl-cmdr".into();
    let temp_dir = try_create_temp_dir().unwrap();
    let timestamp_file_path =
        resolve_self_upgrade_last_check_in_base_dir(&temp_dir, &crate_name);

    // Pre-populate an active cooldown timestamp file to verify
    // `UpgradeStrategy::Immediate` (`--upgrade`) bypasses it and still performs the
    // live HTTP check.
    let (url, server_handle) = spawn_one_shot_http_server(
        "200 OK",
        format_manifest_json("0.0.25", "open-source"),
        Duration::ZERO,
    );
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

#[tokio::test]
#[serial]
async fn test_path1_immediate_up_to_date_async() {
    UpgradeSingletonState::reset_for_testing();
    let (url, server_handle) = spawn_one_shot_http_server(
        "200 OK",
        format_manifest_json("0.0.25", "open-source"),
        Duration::ZERO,
    );
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

#[test]
#[serial]
fn test_path1_immediate_check_failed_and_upgrade_failed_transitions() {
    UpgradeSingletonState::reset_for_testing();
    let temp_dir = try_create_temp_dir().unwrap();
    let timestamp_file_path = temp_dir.join("failed.last_check");

    // 1. Manifest HTTP 500 -> CheckFailed, no cooldown timestamp file.
    let (bad_url, bad_handle) = spawn_one_shot_http_server(
        "500 Internal Server Error",
        "server error".to_string(),
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
    UpgradeSingletonState::reset_for_testing();
    let closed_release_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let closed_release_addr = closed_release_listener.local_addr().unwrap();
    drop(closed_release_listener);

    let (newer_url, newer_handle) = spawn_one_shot_http_server(
        "200 OK",
        format_manifest_json_with_release_url(
            &format!("http://{closed_release_addr}/releases/download"),
            "9.9.9",
            "closed-source",
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

#[test]
#[serial]
fn test_path2_deferred_cooldown_skip_and_expiry() {
    UpgradeSingletonState::reset_for_testing();
    let temp_dir = try_create_temp_dir().unwrap();
    let timestamp_file_path = temp_dir.join("cooldown.last_check");

    // 1. Active 1h cooldown timestamp file -> CooldownSkipped with 0 network/thread work.
    let active_cfg = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Deferred(
            UpgradeActionOnExit::Auto,
        )),
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
        handle_upgrade_on_exit(),
        ExitOutcome::NoDeferredCheckRunning
    );

    // 2. Expired timestamp file (`now - 2h`) -> spawns dedicated thread, queries server,
    //    refreshes timestamp file.
    UpgradeSingletonState::reset_for_testing();
    let expired_time = SystemTime::now() - Duration::from_secs(7200);
    File::open(&timestamp_file_path)
        .unwrap()
        .set_modified(expired_time)
        .unwrap();

    let (url, server_handle) = spawn_one_shot_http_server(
        "200 OK",
        format_manifest_json("0.0.25", "open-source"),
        Duration::ZERO,
    );
    let expired_cfg = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Deferred(
            UpgradeActionOnExit::Auto,
        )),
        EndpointUrl::try_parse(url.as_str()).unwrap(),
        CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
    );

    let run_outcome = UpgradeJourney::new(expired_cfg.clone()).start();
    assert!(matches!(
        run_outcome,
        StartOutcome::DeferredUpgrade(DeferredUpgradeOutcome::DeferredCheckSpawned(_))
    ));

    let exit_outcome = handle_upgrade_on_exit();
    server_handle.join().unwrap();

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
fn test_path2_scenario_2c_exit_joins_inflight_dedicated_thread_and_completes() {
    UpgradeSingletonState::reset_for_testing();

    // Simulate Scenario 2c: short-lived CLI reaches `handle_upgrade_on_exit()` while
    // the `self-upgrade` dedicated thread is still awaiting the HTTP response (`50ms`).
    let (url, server_handle) = spawn_one_shot_http_server(
        "200 OK",
        format_manifest_json("0.0.26", "open-source"),
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
    let exit_outcome = handle_upgrade_on_exit();
    server_handle.join().unwrap();

    match exit_outcome {
        ExitOutcome::Completed(journey) => {
            assert_eq2!(
                journey.get_state().metadata.version,
                CrateVersion::try_parse("0.0.26").unwrap()
            );
            assert_eq2!(
                journey.get_state().resolution,
                UpgradeResolution::NotifiedOnly
            );
        }
        other => panic!("Expected Completed(NotifiedOnly) after join, got {other:?}"),
    }
}

#[tokio::test]
#[serial]
async fn test_path3_disabled_async_and_sync() {
    UpgradeSingletonState::reset_for_testing();
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
        handle_upgrade_on_exit(),
        ExitOutcome::NoDeferredCheckRunning
    );
}

#[test]
#[serial]
fn test_execute_action_on_exit_auto_upgrade_failure_records_completed_without_cooldown() {
    UpgradeSingletonState::reset_for_testing();
    let temp_dir = try_create_temp_dir().unwrap();
    let timestamp_file_path = temp_dir.join("auto_fail.last_check");

    let closed_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let closed_addr = closed_listener.local_addr().unwrap();
    drop(closed_listener);

    let (manifest_url, server_handle) = spawn_one_shot_http_server(
        "200 OK",
        format_manifest_json_with_release_url(
            &format!("http://{closed_addr}/releases/download"),
            "9.9.9",
            "closed-source",
        ),
        Duration::ZERO,
    );
    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Deferred(
            UpgradeActionOnExit::Auto,
        )),
        EndpointUrl::try_parse(manifest_url.as_str()).unwrap(),
        CooldownPolicy::new_custom_path(timestamp_file_path.clone()),
    );

    let outcome = UpgradeJourney::new(config).start();
    assert!(matches!(
        outcome,
        StartOutcome::DeferredUpgrade(DeferredUpgradeOutcome::DeferredCheckSpawned(_))
    ));

    let exit_outcome = handle_upgrade_on_exit();
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
