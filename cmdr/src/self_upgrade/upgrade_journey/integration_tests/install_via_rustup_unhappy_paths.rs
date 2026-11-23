// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unhappy-path integration tests for open-source `rustup` + `cargo` source-build
//! fallback execution:
//! - [`CrateDistribution::OpenSource`] with unreachable archive and empty `PATH` (no
//!   `rustup`) returning [`UpgradeError::NoViableUpgradeSource`] with
//!   [`NoViableSourceReason::NoLocalToolchain`].
//! - Stub `rustup` succeeding on `--version` and exiting `1` on `toolchain install` ->
//!   short-circuits before invoking `cargo` (`cargo` absent from `invocations.log`) and
//!   returns [`UpgradeError::SourceBuildExitedNonZero`].
//! - Stub `cargo` emitting `OSC 9;4;2;0` ([`OscPtyEvent::BuildError`]) and exiting `1` ->
//!   returns [`UpgradeError::SourceBuildExitedNonZero`].
//!
//! All failure paths run in an isolated Coordinator / Worker subprocess and leave no
//! cooldown timestamp file on disk ([`GateStatusError::MetadataReadFailed`] with
//! [`ErrorKind::NotFound`]).
//!
//! [`CrateDistribution::OpenSource`]: crate::self_upgrade::CrateDistribution::OpenSource
//! [`GateStatusError::MetadataReadFailed`]: crate::self_upgrade::GateStatusError::MetadataReadFailed
//! [`NoViableSourceReason::NoLocalToolchain`]: crate::self_upgrade::NoViableSourceReason::NoLocalToolchain
//! [`OscPtyEvent::BuildError`]: r3bl_tui::OscPtyEvent::BuildError
//! [`UpgradeError::NoViableUpgradeSource`]: crate::self_upgrade::UpgradeError::NoViableUpgradeSource
//! [`UpgradeError::SourceBuildExitedNonZero`]: crate::self_upgrade::UpgradeError::SourceBuildExitedNonZero

use super::{super::per_process_start_latch::PerProcessStartLatch, test_fixtures::*};
use crate::{CrateDistribution, CrateVersion, GateStatusError, JsoncScalar,
            NoViableSourceReason, RemoteVersionCheckGate, UpgradeConfig, UpgradeError,
            UpgradePolicy, UpgradeStrategy, UserFeedbackChannel, self_upgrade::*};
use r3bl_tui::{OSC_PROGRESS_END, OSC_PROGRESS_START, OscSequence, TempDirChildPath,
               assert_eq2, pc, try_create_temp_dir, try_set_file_executable};
use std::{env, fs, io::ErrorKind, path::Path, process::Command};

const ISOLATED_RUSTUP_UNHAPPY_ENV_VAR: &str =
    "R3BL_TEST_INSTALL_VIA_RUSTUP_UNHAPPY_ISOLATED";
const INVOCATIONS_LOG_ENV_VAR: &str = "R3BL_TEST_TOOLCHAIN_UNHAPPY_INVOCATIONS_LOG";

const NO_TOOLCHAIN_FILTER: &str = "self_upgrade::upgrade_journey::integration_tests::install_via_rustup_unhappy_paths::test_open_source_unreachable_archive_with_empty_path_returns_no_local_toolchain";
const RUSTUP_NONZERO_FILTER: &str = "self_upgrade::upgrade_journey::integration_tests::install_via_rustup_unhappy_paths::test_rustup_nonzero_exit_short_circuits_before_cargo";
const CARGO_BUILD_ERROR_FILTER: &str = "self_upgrade::upgrade_journey::integration_tests::install_via_rustup_unhappy_paths::test_cargo_build_error_osc_and_nonzero_exit_returns_upgrade_failed";

#[test]
fn test_open_source_unreachable_archive_with_empty_path_returns_no_local_toolchain() {
    if env::var(ISOLATED_RUSTUP_UNHAPPY_ENV_VAR).is_ok() {
        run_open_source_failure_worker(|err| {
            assert!(matches!(
                err,
                UpgradeError::NoViableUpgradeSource {
                    crate_distribution: CrateDistribution::OpenSource,
                    reason: NoViableSourceReason::NoLocalToolchain,
                    ..
                }
            ));
        });
        return;
    }

    let temp_dir = try_create_temp_dir().unwrap();
    let empty_bin_dir = temp_dir.join("empty_bin");
    fs::create_dir_all(&empty_bin_dir).unwrap();

    let output = Command::new(env::current_exe().unwrap())
        .env(ISOLATED_RUSTUP_UNHAPPY_ENV_VAR, "1")
        .env("PATH", &empty_bin_dir)
        .arg("--exact")
        .arg(NO_TOOLCHAIN_FILTER)
        .arg("--nocapture")
        .output()
        .expect("Failed to spawn empty-PATH worker subprocess");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "Isolated empty-PATH worker failed ({}):\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
        output.status,
    );
}

#[test]
fn test_rustup_nonzero_exit_short_circuits_before_cargo() {
    if env::var(ISOLATED_RUSTUP_UNHAPPY_ENV_VAR).is_ok() {
        run_open_source_failure_worker(|err| {
            assert!(
                matches!(err, UpgradeError::SourceBuildExitedNonZero { status } if !status.success()),
                "Expected SourceBuildExitedNonZero, got {err:?}"
            );
        });
        return;
    }

    let logged_lines = run_stub_failure_coordinator(
        RUSTUP_NONZERO_FILTER,
        StubFailureMode::RustupNonZeroAfterProbe,
    );
    assert_eq2!(
        logged_lines,
        vec![
            "rustup --version",
            "rustup toolchain install nightly --force",
        ]
    );
}

#[test]
fn test_cargo_build_error_osc_and_nonzero_exit_returns_upgrade_failed() {
    if env::var(ISOLATED_RUSTUP_UNHAPPY_ENV_VAR).is_ok() {
        run_open_source_failure_worker(|err| {
            assert!(
                matches!(err, UpgradeError::SourceBuildExitedNonZero { status } if !status.success()),
                "Expected SourceBuildExitedNonZero, got {err:?}"
            );
        });
        return;
    }

    let logged_lines = run_stub_failure_coordinator(
        CARGO_BUILD_ERROR_FILTER,
        StubFailureMode::CargoBuildErrorOscNonZero,
    );
    assert_eq2!(
        logged_lines,
        vec![
            "rustup --version",
            "rustup toolchain install nightly --force",
            "cargo +nightly install r3bl-cmdr --version 9.9.9 --force",
        ]
    );
}

#[derive(Clone, Copy)]
enum StubFailureMode {
    RustupNonZeroAfterProbe,
    CargoBuildErrorOscNonZero,
}

fn run_stub_failure_coordinator(test_filter: &str, mode: StubFailureMode) -> Vec<String> {
    let temp_dir = try_create_temp_dir().unwrap();
    let bin_dir = temp_dir.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let invocations_log = temp_dir.join("invocations.log");

    write_failing_stub_toolchain(&bin_dir, mode);

    let mut paths = vec![bin_dir];
    if let Some(existing_path) = env::var_os("PATH") {
        paths.extend(env::split_paths(&existing_path));
    }
    let prepended_path = env::join_paths(paths).unwrap();

    let output = Command::new(env::current_exe().unwrap())
        .env(ISOLATED_RUSTUP_UNHAPPY_ENV_VAR, "1")
        .env(INVOCATIONS_LOG_ENV_VAR, &invocations_log)
        .env("PATH", &prepended_path)
        .arg("--exact")
        .arg(test_filter)
        .arg("--nocapture")
        .output()
        .expect("Failed to spawn isolated install_via_rustup unhappy worker subprocess");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "Isolated install_via_rustup unhappy worker failed ({}):\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
        output.status,
    );

    fs::read_to_string(&invocations_log)
        .unwrap()
        .lines()
        .map(ToOwned::to_owned)
        .collect()
}

fn run_open_source_failure_worker(assert_err: impl FnOnce(&UpgradeError)) {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
    let dead_url = placeholder_release_base_url().with_closed_socket_addr();
    let res_server_spawn =
        manifest_server::spawn_success(manifest_upgrade_available_open_source(&dead_url));
    let cooldown_temp_dir = try_create_temp_dir().unwrap();
    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
        res_server_spawn.endpoint_url.clone(),
        RemoteVersionCheckGate::new_temp_dir(
            TempDirChildPath::try_new(&cooldown_temp_dir, "").unwrap(),
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
            match journey.get_state().resolution {
                UpgradeResolution::UpgradeFailed(UpgradeFailedError::PerformUpgrade(
                    ref err,
                )) => assert_err(err),
                ref other => {
                    panic!(
                        "Expected UpgradeResolution::UpgradeFailed(PerformUpgrade), got {other:?}"
                    )
                }
            }
        }
        other => {
            panic!("Expected ImmediateUpgrade(Completed(UpgradeFailed)), got {other:?}")
        }
    }
}

fn write_failing_stub_toolchain(bin_dir: &Path, mode: StubFailureMode) {
    let osc_progress_50 = OscSequence::ProgressUpdate(pc!(50).unwrap());
    // `OSC 9;4;2;0` encodes `OscPtyEvent::BuildError` for `PtyOscProgressScanner`.
    let osc_build_error = format!("{OSC_PROGRESS_START}2;0{OSC_PROGRESS_END}");

    #[cfg(unix)]
    {
        let rustup_path = bin_dir.join("rustup");
        let rustup_script = match mode {
            StubFailureMode::RustupNonZeroAfterProbe => format!(
                "#!/bin/sh\necho \"rustup $*\" >> \"${INVOCATIONS_LOG_ENV_VAR}\"\nif [ \"$1\" = \"--version\" ]; then\n  exit 0\nfi\necho \"error: toolchain download failed\"\nexit 1\n"
            ),
            StubFailureMode::CargoBuildErrorOscNonZero => format!(
                "#!/bin/sh\necho \"rustup $*\" >> \"${INVOCATIONS_LOG_ENV_VAR}\"\necho \"info: syncing channel updates for 'nightly'\"\nexit 0\n"
            ),
        };
        fs::write(&rustup_path, rustup_script).unwrap();
        try_set_file_executable(&rustup_path).unwrap();

        let cargo_path = bin_dir.join("cargo");
        let cargo_script = match mode {
            StubFailureMode::RustupNonZeroAfterProbe => format!(
                "#!/bin/sh\necho \"cargo $*\" >> \"${INVOCATIONS_LOG_ENV_VAR}\"\nexit 0\n"
            ),
            StubFailureMode::CargoBuildErrorOscNonZero => format!(
                "#!/bin/sh\necho \"cargo $*\" >> \"${INVOCATIONS_LOG_ENV_VAR}\"\ncat << 'OSC_EOF'\n{osc_progress_50}\n{osc_build_error}\nOSC_EOF\nexit 1\n"
            ),
        };
        fs::write(&cargo_path, cargo_script).unwrap();
        try_set_file_executable(&cargo_path).unwrap();
    }

    #[cfg(windows)]
    {
        let rustup_path = bin_dir.join("rustup.cmd");
        let rustup_script = match mode {
            StubFailureMode::RustupNonZeroAfterProbe => format!(
                "@echo off\r\necho rustup %*>> \"%{INVOCATIONS_LOG_ENV_VAR}%\"\r\nif \"%1\"==\"--version\" exit /b 0\r\necho error: toolchain download failed\r\nexit /b 1\r\n"
            ),
            StubFailureMode::CargoBuildErrorOscNonZero => format!(
                "@echo off\r\necho rustup %*>> \"%{INVOCATIONS_LOG_ENV_VAR}%\"\r\necho info: syncing channel updates for 'nightly'\r\nexit /b 0\r\n"
            ),
        };
        fs::write(&rustup_path, rustup_script).unwrap();
        try_set_file_executable(&rustup_path).unwrap();

        let cargo_path = bin_dir.join("cargo.cmd");
        let cargo_script = match mode {
            StubFailureMode::RustupNonZeroAfterProbe => format!(
                "@echo off\r\necho cargo %*>> \"%{INVOCATIONS_LOG_ENV_VAR}%\"\r\nexit /b 0\r\n"
            ),
            StubFailureMode::CargoBuildErrorOscNonZero => format!(
                "@echo off\r\necho cargo %*>> \"%{INVOCATIONS_LOG_ENV_VAR}%\"\r\necho {osc_progress_50}\r\necho {osc_build_error}\r\nexit /b 1\r\n"
            ),
        };
        fs::write(&cargo_path, cargo_script).unwrap();
        try_set_file_executable(&cargo_path).unwrap();
    }
}
