// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Single-start latch ([`PerProcessStartLatch`], [`StartOutcome::AlreadyStarted`]) and
//! disabled policy ([`UpgradePolicy::Disabled`]) integration tests for
//! [`UpgradeJourney`].
//!
//! [`PerProcessStartLatch`]: crate::self_upgrade::upgrade_journey::per_process_start_latch::PerProcessStartLatch
//! [`StartOutcome::AlreadyStarted`]: crate::self_upgrade::StartOutcome::AlreadyStarted
//! [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
//! [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled

use super::{super::per_process_start_latch::PerProcessStartLatch, test_fixtures::*};
use crate::{CrateVersion, EndpointUrl, GateStatusError, JsoncScalar,
            RemoteVersionCheckGate, UpgradeActionOnExit, UpgradeConfig, UpgradePolicy,
            UpgradeStrategy, UserFeedbackChannel, self_upgrade::*};
use r3bl_tui::{TempDirChildPath, assert_eq2, try_create_temp_dir};
use serial_test::serial;
use std::{io::ErrorKind, time::Duration};

#[test]
#[serial]
fn test_disabled_policy_returns_disabled_with_zero_http_calls_sync() {
    scenario_disabled_policy_returns_disabled_with_zero_http_calls();
}

#[tokio::test]
#[serial]
async fn test_disabled_policy_returns_disabled_with_zero_http_calls_async() {
    scenario_disabled_policy_returns_disabled_with_zero_http_calls();
}

#[test]
#[serial]
fn test_duplicate_after_disabled_returns_already_started_sync() {
    scenario_duplicate_after_disabled_returns_already_started();
}

#[tokio::test]
#[serial]
async fn test_duplicate_after_disabled_returns_already_started_async() {
    scenario_duplicate_after_disabled_returns_already_started();
}

#[test]
#[serial]
fn test_duplicate_after_immediate_returns_already_started_sync() {
    scenario_duplicate_after_immediate_returns_already_started();
}

#[tokio::test]
#[serial]
async fn test_duplicate_after_immediate_returns_already_started_async() {
    scenario_duplicate_after_immediate_returns_already_started();
}

#[test]
#[serial]
fn test_duplicate_after_inflight_deferred_returns_already_started_without_stealing_handle_sync()
 {
    scenario_duplicate_after_inflight_deferred_returns_already_started_without_stealing_handle();
}

#[tokio::test]
#[serial]
async fn test_duplicate_after_inflight_deferred_returns_already_started_without_stealing_handle_async()
 {
    scenario_duplicate_after_inflight_deferred_returns_already_started_without_stealing_handle();
}

fn scenario_disabled_policy_returns_disabled_with_zero_http_calls() {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let temp_dir = try_create_temp_dir().unwrap();
    let res_server_spawn = manifest_server::spawn_uncalled();
    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Disabled,
        res_server_spawn.endpoint_url.clone(),
        RemoteVersionCheckGate::new_temp_dir(
            TempDirChildPath::try_new(&temp_dir, "").unwrap(),
            None,
        ),
    );
    // Act.
    let outcome = UpgradeJourney::new(config.clone()).start();

    // Assert.
    res_server_spawn.assert_expectations();
    assert!(matches!(
        config
            .get_remote_version_check_gate()
            .try_check_gate_status(),
        Err(GateStatusError::MetadataReadFailed { src_err, .. })
            if src_err.kind() == ErrorKind::NotFound
    ));
    match outcome {
        StartOutcome::Disabled(journey) => {
            assert_eq2!(journey, UpgradeJourney::new(config).into_disabled());
        }
        other => panic!("Expected StartOutcome::Disabled, got {other:?}"),
    }
}

fn scenario_duplicate_after_disabled_returns_already_started() {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let res_server_spawn = manifest_server::spawn_uncalled();
    let make_config = |policy: UpgradePolicy, url: EndpointUrl| {
        UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            policy,
            url,
            RemoteVersionCheckGate::Disabled,
        )
    };
    let disabled_config = make_config(
        UpgradePolicy::Disabled,
        res_server_spawn.endpoint_url.clone(),
    );
    let dup_immediate_config = make_config(
        UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
        res_server_spawn.endpoint_url.clone(),
    );
    let dup_deferred_config = make_config(
        UpgradePolicy::Enabled(UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto)),
        res_server_spawn.endpoint_url.clone(),
    );

    // Act.
    let first_outcome = UpgradeJourney::new(disabled_config.clone()).start();
    let dup_immediate_outcome = UpgradeJourney::new(dup_immediate_config).start();
    let dup_deferred_outcome = UpgradeJourney::new(dup_deferred_config).start();

    // Assert.
    res_server_spawn.assert_expectations();
    match first_outcome {
        StartOutcome::Disabled(journey) => {
            assert_eq2!(
                journey,
                UpgradeJourney::new(disabled_config).into_disabled()
            );
        }
        other => panic!("Expected StartOutcome::Disabled, got {other:?}"),
    }
    assert!(matches!(
        dup_immediate_outcome,
        StartOutcome::AlreadyStarted
    ));
    assert!(matches!(dup_deferred_outcome, StartOutcome::AlreadyStarted));
}

fn scenario_duplicate_after_immediate_returns_already_started() {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let res_server_spawn = manifest_server::spawn_success(manifest_up_to_date());
    let make_config = |policy: UpgradePolicy| {
        UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            policy,
            res_server_spawn.endpoint_url.clone(),
            RemoteVersionCheckGate::Disabled,
        )
    };
    let first_config = make_config(UpgradePolicy::Enabled(UpgradeStrategy::Immediate));
    let dup_immediate_config =
        make_config(UpgradePolicy::Enabled(UpgradeStrategy::Immediate));
    let dup_deferred_config = make_config(UpgradePolicy::Enabled(
        UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto),
    ));

    // Act.
    let first_outcome = UpgradeJourney::new(first_config).start();
    let dup_immediate_outcome = UpgradeJourney::new(dup_immediate_config).start();
    let dup_deferred_outcome = UpgradeJourney::new(dup_deferred_config).start();

    // Assert.
    res_server_spawn.assert_each_mock_called_once();
    assert!(matches!(
        first_outcome,
        StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::UpToDate(_))
    ));
    assert!(matches!(
        dup_immediate_outcome,
        StartOutcome::AlreadyStarted
    ));
    assert!(matches!(dup_deferred_outcome, StartOutcome::AlreadyStarted));
}

fn scenario_duplicate_after_inflight_deferred_returns_already_started_without_stealing_handle()
 {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let res_server_spawn = manifest_server::spawn_delayed_success(
        manifest_up_to_date(),
        Duration::from_millis(50),
    );
    let make_config = |policy: UpgradePolicy| {
        UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            policy,
            res_server_spawn.endpoint_url.clone(),
            RemoteVersionCheckGate::Disabled,
        )
    };
    let main_config = make_config(UpgradePolicy::Enabled(UpgradeStrategy::Deferred(
        UpgradeActionOnExit::Auto,
    )));
    let dup_immediate_config =
        make_config(UpgradePolicy::Enabled(UpgradeStrategy::Immediate));
    let dup_deferred_config = make_config(UpgradePolicy::Enabled(
        UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto),
    ));

    // Act.
    let main_outcome = UpgradeJourney::new(main_config).start();
    let StartOutcome::DeferredUpgrade(main_deferred) = main_outcome else {
        panic!("Expected StartOutcome::DeferredUpgrade, got {main_outcome:?}");
    };
    assert!(matches!(
        main_deferred,
        DeferredUpgradeOutcome::DeferredCheckSpawned(_)
    ));

    // Duplicate callers receive `AlreadyStarted` (which holds no
    // `DeferredUpgradeOutcome`), so stealing `main_deferred`'s handle is unrepresentable.
    let dup_immediate_outcome = UpgradeJourney::new(dup_immediate_config).start();
    let dup_deferred_outcome = UpgradeJourney::new(dup_deferred_config).start();
    let exit_outcome = main_deferred.handle_upgrade_on_exit();

    // Assert.
    res_server_spawn.assert_each_mock_called_once();
    assert!(matches!(
        dup_immediate_outcome,
        StartOutcome::AlreadyStarted
    ));
    assert!(matches!(dup_deferred_outcome, StartOutcome::AlreadyStarted));
    assert!(matches!(exit_outcome, ExitOutcome::UpToDate(_)));
}
