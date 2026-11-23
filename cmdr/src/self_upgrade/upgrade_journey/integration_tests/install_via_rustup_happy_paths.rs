// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Happy-path integration tests for [`UpgradeSource::BuildFromSourceRustup`] fallback
//! execution (`rustup toolchain install nightly --force` + `cargo +nightly install` with
//! [`OSC`] `9;4` progress events in a [`PTY`]).
//!
//! # Why Subprocess Isolation Is Required
//!
//! [`UpgradeSource::try_resolve`], [`UpgradeSource::try_fallback_to_source`], and
//! [`run_source_build_with_spinner`] locate `rustup` and `cargo` via `$PATH`. Because
//! modifying environment variables in-process is unsafe in multi-threaded test runners,
//! these tests use the **Coordinator / Worker** pattern:
//! 1. **Coordinator**: Creates stub `rustup` and `cargo` executables in a disposable
//!    `temp_dir/bin/` directory (formatting [`OSC`] `9;4` sequences via
//!    [`OscSequence::ProgressUpdate`] and [`OscSequence::ProgressCleared`]), prepends
//!    `temp_dir/bin/` to `PATH`, and spawns a child test runner process with
//!    `R3BL_TEST_INSTALL_VIA_RUSTUP_HAPPY_ISOLATED` set.
//! 2. **Worker**: Runs [`UpgradeJourney::start`] (and
//!    [`DeferredUpgradeOutcome::handle_upgrade_on_exit`] for deferred auto mode) either
//!    with a closed release archive port (`HEAD` unreachable) or against
//!    [`archive_server::spawn_archive_get_error`] (`HEAD 200`, `GET *.tar.gz` HTTP 500
//!    triggering [`UpgradeSource::try_fallback_to_source`]), driving the stub `rustup`
//!    and `cargo` binaries through [`PtySessionBuilder`].
//!
//! [`archive_server::spawn_archive_get_error`]:
//!     super::test_fixtures::archive_server::spawn_archive_get_error
//! [`DeferredUpgradeOutcome::handle_upgrade_on_exit`]:
//!     crate::self_upgrade::DeferredUpgradeOutcome::handle_upgrade_on_exit
//! [`OSC`]: r3bl_tui::OscSequence
//! [`OscSequence::ProgressCleared`]: r3bl_tui::OscSequence::ProgressCleared
//! [`OscSequence::ProgressUpdate`]: r3bl_tui::OscSequence::ProgressUpdate
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`PtySessionBuilder`]: r3bl_tui::core::pty::PtySessionBuilder
//! [`run_source_build_with_spinner`]: crate::self_upgrade::run_source_build_with_spinner
//! [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
//! [`UpgradeSource::BuildFromSourceRustup`]:
//!     crate::self_upgrade::UpgradeSource::BuildFromSourceRustup
//! [`UpgradeSource::try_fallback_to_source`]:
//!     crate::self_upgrade::UpgradeSource::try_fallback_to_source
//! [`UpgradeSource::try_resolve`]: crate::self_upgrade::UpgradeSource::try_resolve

use super::{super::per_process_start_latch::PerProcessStartLatch, test_fixtures::*};
use crate::{RemoteVersionCheckGate, GateStatus, CrateDistribution, CrateVersion,
            JsoncScalar, UpgradeActionOnExit, UpgradeConfig, UpgradePolicy,
            UpgradeStrategy, UserFeedbackChannel, self_upgrade::*};
use r3bl_tui::{OscSequence, TempDirChildPath, assert_eq2, pc, try_create_temp_dir,
               try_set_file_executable};
use std::{env, fs, path::Path, process::Command};

const ISOLATED_RUSTUP_HAPPY_ENV_VAR: &str = "R3BL_TEST_INSTALL_VIA_RUSTUP_HAPPY_ISOLATED";
const INVOCATIONS_LOG_ENV_VAR: &str = "R3BL_TEST_TOOLCHAIN_INVOCATIONS_LOG";

const IMMEDIATE_HEAD_UNREACHABLE_FILTER: &str = "self_upgrade::upgrade_journey::integration_tests::install_via_rustup_happy_paths::test_immediate_head_unreachable_runs_rustup_and_cargo_in_pty";
const DEFERRED_HEAD_UNREACHABLE_FILTER: &str = "self_upgrade::upgrade_journey::integration_tests::install_via_rustup_happy_paths::test_deferred_auto_head_unreachable_runs_rustup_and_cargo_in_pty";
const MID_STREAM_GET_500_FILTER: &str = "self_upgrade::upgrade_journey::integration_tests::install_via_rustup_happy_paths::test_mid_stream_archive_get_500_falls_back_to_rustup_and_cargo_in_pty";

#[test]
fn test_immediate_head_unreachable_runs_rustup_and_cargo_in_pty() {
    if env::var(ISOLATED_RUSTUP_HAPPY_ENV_VAR).is_ok() {
        run_head_unreachable_worker(UpgradeStrategy::Immediate);
        return;
    }
    run_install_via_rustup_happy_coordinator(IMMEDIATE_HEAD_UNREACHABLE_FILTER);
}

#[test]
fn test_deferred_auto_head_unreachable_runs_rustup_and_cargo_in_pty() {
    if env::var(ISOLATED_RUSTUP_HAPPY_ENV_VAR).is_ok() {
        run_head_unreachable_worker(UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto));
        return;
    }
    run_install_via_rustup_happy_coordinator(DEFERRED_HEAD_UNREACHABLE_FILTER);
}

#[test]
fn test_mid_stream_archive_get_500_falls_back_to_rustup_and_cargo_in_pty() {
    if env::var(ISOLATED_RUSTUP_HAPPY_ENV_VAR).is_ok() {
        run_mid_stream_get_500_worker();
        return;
    }
    run_install_via_rustup_happy_coordinator(MID_STREAM_GET_500_FILTER);
}

/// Coordinator: creates stub `rustup` and `cargo` binaries in `temp_dir/bin/`, prepends
/// `temp_dir/bin/` to `PATH`, spawns `test_filter`, and asserts all 3 toolchain
/// invocations succeeded in order.
fn run_install_via_rustup_happy_coordinator(test_filter: &str) {
    let temp_dir = try_create_temp_dir().unwrap();
    let bin_dir = temp_dir.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let invocations_log = temp_dir.join("invocations.log");

    write_stub_toolchain_binaries(&bin_dir);

    let mut paths = vec![bin_dir];
    if let Some(existing_path) = env::var_os("PATH") {
        paths.extend(env::split_paths(&existing_path));
    }
    let prepended_path = env::join_paths(paths).unwrap();

    let output = Command::new(env::current_exe().unwrap())
        .env(ISOLATED_RUSTUP_HAPPY_ENV_VAR, "1")
        .env(INVOCATIONS_LOG_ENV_VAR, &invocations_log)
        .env("PATH", &prepended_path)
        .arg("--exact")
        .arg(test_filter)
        .arg("--nocapture")
        .output()
        .expect("Failed to spawn isolated install_via_rustup happy worker subprocess");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "Isolated install_via_rustup happy worker failed ({}):\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
        output.status,
    );

    // Verify the exact sequence of CLI invocations recorded by the stub toolchain.
    let log_contents = fs::read_to_string(&invocations_log).unwrap();
    let logged_lines: Vec<&str> = log_contents.lines().collect();
    assert_eq2!(
        logged_lines,
        vec![
            "rustup --version",
            "rustup toolchain install nightly --force",
            "cargo +nightly install r3bl-cmdr --version 9.9.9 --force",
        ]
    );
}

fn run_head_unreachable_worker(strategy: UpgradeStrategy) {
    // Obtain a guaranteed-closed local port so the release archive `HEAD` probe fails
    // and triggers the open-source `rustup` + `cargo install` fallback.
    let dead_url = placeholder_release_base_url().with_closed_socket_addr();
    let res_server_spawn =
        manifest_server::spawn_success(manifest_upgrade_available_open_source(&dead_url));
    execute_and_assert_upgraded_worker(strategy, res_server_spawn);
}

fn run_mid_stream_get_500_worker() {
    let res_server_spawn = archive_server::spawn_archive_get_error(
        CrateDistribution::OpenSource,
        blocks::HTTP_500_INTERNAL_SERVER_ERROR,
    );
    execute_and_assert_upgraded_worker(UpgradeStrategy::Immediate, res_server_spawn);
}

fn execute_and_assert_upgraded_worker(
    strategy: UpgradeStrategy,
    res_server_spawn: ServerSpawnResult,
) {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let cooldown_temp_dir = try_create_temp_dir().unwrap();
    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(strategy),
        res_server_spawn.endpoint_url.clone(),
        RemoteVersionCheckGate::new_temp_dir(
            TempDirChildPath::try_new(&cooldown_temp_dir, "").unwrap(),
            None,
        ),
    );

    // Act & Assert.
    let start_outcome = UpgradeJourney::new(config.clone()).start();
    match strategy {
        UpgradeStrategy::Immediate => {
            res_server_spawn.assert_each_mock_called_once();
            match start_outcome {
                StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::Completed(
                    journey,
                )) => {
                    assert_eq2!(
                        journey.get_state().metadata.version,
                        CrateVersion::try_parse("9.9.9").unwrap()
                    );
                    assert!(matches!(
                        journey.get_state().resolution,
                        UpgradeResolution::Upgraded
                    ));
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
            res_server_spawn.assert_each_mock_called_once();
            match exit_outcome {
                ExitOutcome::Completed(journey) => {
                    assert_eq2!(
                        journey.get_state().metadata.version,
                        CrateVersion::try_parse("9.9.9").unwrap()
                    );
                    assert!(matches!(
                        journey.get_state().resolution,
                        UpgradeResolution::Upgraded
                    ));
                }
                other => {
                    panic!("Expected ExitOutcome::Completed(Upgraded), got {other:?}")
                }
            }
        }
    }

    assert_eq2!(
        config
            .get_remote_version_check_gate()
            .try_check_gate_status().unwrap(),
        GateStatus::ClosedForCooldown
    );
}

/// Writes cross-platform stub `rustup` and `cargo` scripts into `bin_dir` that log their
/// arguments to `$R3BL_TEST_TOOLCHAIN_INVOCATIONS_LOG` and emit [`OscSequence`] progress
/// sequences over the [`PTY`].
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
fn write_stub_toolchain_binaries(bin_dir: &Path) {
    let osc_progress_50 = OscSequence::ProgressUpdate(pc!(50).unwrap());
    let osc_progress_cleared = OscSequence::ProgressCleared;

    #[cfg(unix)]
    {
        let rustup_path = bin_dir.join("rustup");
        let rustup_script = format!(
            "#!/bin/sh\necho \"rustup $*\" >> \"${INVOCATIONS_LOG_ENV_VAR}\"\necho \"info: syncing channel updates for 'nightly'\"\necho \"info: installing component 'cargo'\"\nexit 0\n"
        );
        fs::write(&rustup_path, rustup_script).unwrap();
        try_set_file_executable(&rustup_path).unwrap();

        let cargo_path = bin_dir.join("cargo");
        let cargo_script = format!(
            "#!/bin/sh\necho \"cargo $*\" >> \"${INVOCATIONS_LOG_ENV_VAR}\"\ncat << 'OSC_EOF'\n{osc_progress_50}\n{osc_progress_cleared}\nOSC_EOF\nexit 0\n"
        );
        fs::write(&cargo_path, cargo_script).unwrap();
        try_set_file_executable(&cargo_path).unwrap();
    }

    #[cfg(windows)]
    {
        let rustup_path = bin_dir.join("rustup.cmd");
        let rustup_script = format!(
            "@echo off\r\necho rustup %*>> \"%{INVOCATIONS_LOG_ENV_VAR}%\"\r\necho info: syncing channel updates for 'nightly'\r\nexit /b 0\r\n"
        );
        fs::write(&rustup_path, rustup_script).unwrap();
        try_set_file_executable(&rustup_path).unwrap();

        let cargo_path = bin_dir.join("cargo.cmd");
        let cargo_script = format!(
            "@echo off\r\necho cargo %*>> \"%{INVOCATIONS_LOG_ENV_VAR}%\"\r\necho {osc_progress_50}\r\necho {osc_progress_cleared}\r\nexit /b 0\r\n"
        );
        fs::write(&cargo_path, cargo_script).unwrap();
        try_set_file_executable(&cargo_path).unwrap();
    }
}
