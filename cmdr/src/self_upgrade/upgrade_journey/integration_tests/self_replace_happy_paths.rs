// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Happy-path integration tests for [`UpgradeSource::ReleaseArchive`] download, SHA-256
//! verification, `.tar.gz` extraction, and in-place binary replacement via
//! [`self_replace`].
//!
//! # Why Subprocess Isolation Is Required
//!
//! [`binary_replacement::apply_extracted_binaries`] inspects [`std::env::current_exe()`]
//! and invokes [`self_replace::self_replace`] on that executable path. Running
//! in-process would overwrite the `cargo test` runner binary inside `target/debug/deps/`.
//!
//! Instead, these tests use the **Coordinator / Worker** pattern:
//! 1. **Coordinator**: Copies [`std::env::current_exe()`] into a disposable
//!    [`try_create_temp_dir()`] directory (`temp_dir/bin/<exe_name>`) and spawns that
//!    copy as a child process with `R3BL_TEST_SELF_REPLACE_HAPPY_ISOLATED` set.
//! 2. **Worker**: Runs [`UpgradeJourney::start`] (and
//!    [`DeferredUpgradeOutcome::handle_upgrade_on_exit`] for deferred auto mode) against
//!    an ephemeral `127.0.0.1:0` release server serving a real `.tar.gz` and `.sha256`.
//!    Because the worker's [`std::env::current_exe()`] points to
//!    `temp_dir/bin/<exe_name>`, [`self_replace::self_replace`] safely replaces that
//!    temporary copy on disk and installs the sibling binaries (`edi`, `env-source`) next
//!    to it.
//!
//! [`binary_replacement::apply_extracted_binaries`]: crate::self_upgrade::binary_replacement::apply_extracted_binaries
//! [`DeferredUpgradeOutcome::handle_upgrade_on_exit`]: crate::self_upgrade::DeferredUpgradeOutcome::handle_upgrade_on_exit
//! [`self_replace::self_replace`]: self_replace::self_replace
//! [`self_replace`]: mod@self_replace
//! [`try_create_temp_dir()`]: r3bl_tui::try_create_temp_dir
//! [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
//! [`UpgradeSource::ReleaseArchive`]: crate::self_upgrade::UpgradeSource::ReleaseArchive

use super::{super::per_process_start_latch::PerProcessStartLatch, test_fixtures::*};
use crate::{CooldownPolicy, CooldownStatus, CrateVersion, JsoncScalar,
            UpgradeActionOnExit, UpgradeConfig, UpgradePolicy, UpgradeStrategy,
            UserFeedbackChannel, self_upgrade::*};
use flate2::{Compression, write::GzEncoder};
use r3bl_tui::{assert_eq2, try_create_temp_dir};
use std::{env, fs, process::Command};

const ISOLATED_SELF_REPLACE_HAPPY_ENV_VAR: &str = "R3BL_TEST_SELF_REPLACE_HAPPY_ISOLATED";
const IMMEDIATE_TEST_FILTER: &str = "self_upgrade::upgrade_journey::integration_tests::self_replace_happy_paths::test_immediate_self_replace_replaces_main_and_sibling_binaries";
const DEFERRED_AUTO_TEST_FILTER: &str = "self_upgrade::upgrade_journey::integration_tests::self_replace_happy_paths::test_deferred_auto_self_replace_replaces_main_and_sibling_binaries";

const UPGRADED_MAIN_BINARY_BYTES: &[u8] = b"#!/bin/sh\necho upgraded-main-v9.9.9\n";
const UPGRADED_EDI_BINARY_BYTES: &[u8] = b"#!/bin/sh\necho upgraded-edi-v9.9.9\n";
const UPGRADED_ENV_SOURCE_BINARY_BYTES: &[u8] =
    b"#!/bin/sh\necho upgraded-env-source-v9.9.9\n";

#[test]
fn test_immediate_self_replace_replaces_main_and_sibling_binaries() {
    if env::var(ISOLATED_SELF_REPLACE_HAPPY_ENV_VAR).is_ok() {
        run_self_replace_happy_worker(UpgradeStrategy::Immediate);
        return;
    }
    run_self_replace_happy_coordinator(IMMEDIATE_TEST_FILTER);
}

#[test]
fn test_deferred_auto_self_replace_replaces_main_and_sibling_binaries() {
    if env::var(ISOLATED_SELF_REPLACE_HAPPY_ENV_VAR).is_ok() {
        run_self_replace_happy_worker(UpgradeStrategy::Deferred(
            UpgradeActionOnExit::Auto,
        ));
        return;
    }
    run_self_replace_happy_coordinator(DEFERRED_AUTO_TEST_FILTER);
}

/// Coordinator: copies the test runner binary into a disposable temp directory, runs
/// `test_filter` inside the child copy, and asserts all binaries were replaced on disk.
fn run_self_replace_happy_coordinator(test_filter: &str) {
    let temp_dir = try_create_temp_dir().unwrap();
    let bin_dir = temp_dir.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let current_exe = env::current_exe().unwrap();
    let exe_file_name = current_exe.file_name().unwrap();
    let isolated_exe = bin_dir.join(exe_file_name);
    fs::copy(&current_exe, &isolated_exe).unwrap();

    let output = Command::new(&isolated_exe)
        .env(ISOLATED_SELF_REPLACE_HAPPY_ENV_VAR, "1")
        .arg("--exact")
        .arg(test_filter)
        .arg("--nocapture")
        .output()
        .expect("Failed to spawn isolated self_replace happy worker subprocess");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "Isolated self_replace happy worker failed ({}):\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
        output.status,
    );

    // Verify on disk that `self_replace` overwrote the isolated executable and installed
    // both sibling binaries alongside it.
    assert_eq2!(fs::read(&isolated_exe).unwrap(), UPGRADED_MAIN_BINARY_BYTES);
    assert_eq2!(
        fs::read(bin_dir.join("edi")).unwrap(),
        UPGRADED_EDI_BINARY_BYTES
    );
    assert_eq2!(
        fs::read(bin_dir.join("env-source")).unwrap(),
        UPGRADED_ENV_SOURCE_BINARY_BYTES
    );
}

/// Worker: runs inside the isolated subprocess whose [`std::env::current_exe()`] points
/// to `temp_dir/bin/<exe_name>`.
fn run_self_replace_happy_worker(strategy: UpgradeStrategy) {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let current_exe = env::current_exe().unwrap();
    let current_exe_name = current_exe
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let archive_bytes = build_release_tar_gz(&current_exe_name);
    let server_spawn_result = archive_server::spawn_success(archive_bytes);

    let cooldown_temp_dir = try_create_temp_dir().unwrap();
    let timestamp_file_path = cooldown_temp_dir.join("self_replace.last_check");

    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(strategy),
        server_spawn_result.endpoint_url.clone(),
        CooldownPolicy::new_custom_path(timestamp_file_path),
    );

    // Act & Assert.
    let start_outcome = UpgradeJourney::new(config.clone()).start();
    match strategy {
        UpgradeStrategy::Immediate => {
            server_spawn_result.assert_each_mock_called_once();
            match start_outcome {
                StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::Completed(
                    journey,
                )) => {
                    assert_eq2!(
                        journey.get_state().metadata.version,
                        CrateVersion::try_parse("9.9.9").unwrap()
                    );
                    assert_eq2!(
                        journey.get_state().resolution,
                        UpgradeResolution::Upgraded
                    );
                }
                other => {
                    panic!(
                        "Expected ImmediateUpgrade(Completed(Upgraded)), got {other:?}"
                    )
                }
            }
        }
        UpgradeStrategy::Deferred(_) => {
            let StartOutcome::DeferredUpgrade(deferred) = start_outcome else {
                panic!("Expected StartOutcome::DeferredUpgrade, got {start_outcome:?}");
            };
            assert!(matches!(
                deferred,
                DeferredUpgradeOutcome::DeferredCheckSpawned(_)
            ));
            let exit_outcome = deferred.handle_upgrade_on_exit();
            server_spawn_result.assert_each_mock_called_once();
            match exit_outcome {
                ExitOutcome::Completed(journey) => {
                    assert_eq2!(
                        journey.get_state().metadata.version,
                        CrateVersion::try_parse("9.9.9").unwrap()
                    );
                    assert_eq2!(
                        journey.get_state().resolution,
                        UpgradeResolution::Upgraded
                    );
                }
                other => {
                    panic!("Expected ExitOutcome::Completed(Upgraded), got {other:?}")
                }
            }
        }
    }

    assert_eq2!(
        config
            .get_cooldown_policy()
            .check_status(config.get_crate_name()),
        CooldownStatus::InCooldown
    );
}

/// Builds an in-memory `.tar.gz` containing the replacement main binary
/// (`current_exe_name`) and sibling binaries (`edi`, `env-source`).
fn build_release_tar_gz(current_exe_name: &str) -> Vec<u8> {
    let enc = GzEncoder::new(Vec::new(), Compression::default());
    let mut tar_builder = tar::Builder::new(enc);

    for (name, bytes) in [
        (current_exe_name, UPGRADED_MAIN_BINARY_BYTES),
        ("edi", UPGRADED_EDI_BINARY_BYTES),
        ("env-source", UPGRADED_ENV_SOURCE_BINARY_BYTES),
    ] {
        let mut header = tar::Header::new_gnu();
        header.set_size(u64::try_from(bytes.len()).unwrap());
        header.set_mode(0o755);
        header.set_cksum();
        tar_builder.append_data(&mut header, name, bytes).unwrap();
    }

    let enc = tar_builder.into_inner().unwrap();
    enc.finish().unwrap()
}
