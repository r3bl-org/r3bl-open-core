// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! End-to-end integration test for [`UpgradeSource::BuildFromSourceRustup`] fallback
//! execution (`rustup toolchain install nightly --force` + `cargo +nightly install`
//! with [`OSC`] `9;4` progress events in a [`PTY`]).
//!
//! # Why Subprocess Isolation Is Required
//!
//! [`UpgradeSource::try_resolve`] and [`run_source_build_with_spinner`] locate `rustup`
//! and `cargo` via `$PATH`. Because modifying environment variables in-process is unsafe
//! in multi-threaded test runners, this test uses the **Coordinator / Worker** pattern:
//! 1. **Coordinator**: Creates stub `rustup` and `cargo` executables in a disposable
//!    `temp_dir/bin/` directory (formatting [`OSC`] `9;4` sequences via
//!    [`OscSequence::ProgressUpdate`] and [`OscSequence::ProgressCleared`]), prepends
//!    `temp_dir/bin/` to `PATH`, and spawns a child test runner process with
//!    `R3BL_TEST_CARGO_INSTALL_ISOLATED` set.
//! 2. **Worker**: Serves [`MANIFEST_UPGRADE_AVAILABLE_OPEN_SOURCE_JSONC`] pointing
//!    `binaries_release_base_url` at a closed local port so
//!    [`UpgradeSource::try_resolve`] falls back to
//!    [`UpgradeSource::BuildFromSourceRustup`] and drives the stub `rustup` and `cargo`
//!    binaries through [`PtySessionBuilder`].
//!
//! [`MANIFEST_UPGRADE_AVAILABLE_OPEN_SOURCE_JSONC`]: super::test_fixtures::MANIFEST_UPGRADE_AVAILABLE_OPEN_SOURCE_JSONC
//! [`OSC`]: r3bl_tui::osc_codes::OscSequence
//! [`OscSequence::ProgressCleared`]: r3bl_tui::osc_codes::OscSequence::ProgressCleared
//! [`OscSequence::ProgressUpdate`]: r3bl_tui::osc_codes::OscSequence::ProgressUpdate
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`PtySessionBuilder`]: r3bl_tui::core::pty::PtySessionBuilder
//! [`run_source_build_with_spinner`]: crate::self_upgrade::run_source_build_with_spinner
//! [`UpgradeSource::BuildFromSourceRustup`]: crate::self_upgrade::UpgradeSource::BuildFromSourceRustup
//! [`UpgradeSource::try_resolve`]: crate::self_upgrade::UpgradeSource::try_resolve

use super::{test_fixtures::*, *};
use crate::{CooldownPolicy, CooldownStatus, CrateVersion, EndpointUrl, JsoncScalar,
            UpgradeConfig, UpgradePolicy, UpgradeStrategy, UserFeedbackChannel};
use r3bl_tui::{OscSequence, assert_eq2, pc, try_create_temp_dir, try_set_file_executable};
use std::{env, fs, net::TcpListener, path::Path, process::Command, time::Duration};

const ISOLATED_CARGO_INSTALL_ENV_VAR: &str = "R3BL_TEST_CARGO_INSTALL_ISOLATED";
const INVOCATIONS_LOG_ENV_VAR: &str = "R3BL_TEST_TOOLCHAIN_INVOCATIONS_LOG";
const TEST_FILTER: &str = "self_upgrade::upgrade_journey::integration_tests::cargo_install_rustup_test::test_immediate_upgrade_open_source_falls_back_to_rustup_and_cargo_install_pty";

#[cfg_attr(not(doc), test)]
pub fn test_immediate_upgrade_open_source_falls_back_to_rustup_and_cargo_install_pty() {
    if env::var(ISOLATED_CARGO_INSTALL_ENV_VAR).is_ok() {
        run_cargo_install_worker();
        return;
    }

    // Coordinator: create stub `rustup` and `cargo` executables in `temp_dir/bin/` and
    // prepend `temp_dir/bin/` to `PATH` for the isolated worker subprocess.
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
        .env(ISOLATED_CARGO_INSTALL_ENV_VAR, "1")
        .env(INVOCATIONS_LOG_ENV_VAR, &invocations_log)
        .env("PATH", &prepended_path)
        .arg("--exact")
        .arg(TEST_FILTER)
        .arg("--nocapture")
        .output()
        .expect("Failed to spawn isolated cargo_install_rustup worker subprocess");

    if !output.status.success() {
        panic!(
            "Isolated cargo_install_rustup worker failed ({}):\n--- stdout ---\n{}\n--- stderr ---\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

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

/// Runs inside the isolated subprocess with `temp_dir/bin` prepended to `PATH`.
fn run_cargo_install_worker() {
    SingletonState::reset_for_testing();

    // Bind and immediately drop a listener to obtain a guaranteed-closed local port so
    // the release archive `HEAD` probe fails and triggers the open-source `rustup` +
    // `cargo install` fallback.
    let closed_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let closed_addr = closed_listener.local_addr().unwrap();
    drop(closed_listener);

    let (manifest_url, server_handle) = spawn_one_shot_http_server(
        "200 OK",
        with_release_base_url(
            MANIFEST_UPGRADE_AVAILABLE_OPEN_SOURCE_JSONC,
            &format!("http://{closed_addr}/releases/download"),
        ),
        Duration::ZERO,
    );

    let cooldown_temp_dir = try_create_temp_dir().unwrap();
    let timestamp_file_path = cooldown_temp_dir.join("cargo_install.last_check");

    let config = UpgradeConfig::new(
        "r3bl-cmdr",
        CrateVersion::try_parse("0.0.25").unwrap(),
        UserFeedbackChannel::Quiet,
        UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
        EndpointUrl::try_parse(manifest_url.as_str()).unwrap(),
        CooldownPolicy::new_custom_path(timestamp_file_path),
    );

    let outcome = UpgradeJourney::new(config.clone()).start();
    server_handle.join().unwrap();

    match outcome {
        StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::Completed(journey)) => {
            assert_eq2!(
                journey.get_state().metadata.version,
                CrateVersion::try_parse("9.9.9").unwrap()
            );
            assert_eq2!(journey.get_state().resolution, UpgradeResolution::Upgraded);
        }
        other => {
            panic!("Expected ImmediateUpgrade(Completed(Upgraded)), got {other:?}")
        }
    }

    assert_eq2!(
        config
            .get_cooldown_policy()
            .check_status(config.get_crate_name()),
        CooldownStatus::Active
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

        let cargo_path = bin_dir.join("cargo.cmd");
        let cargo_script = format!(
            "@echo off\r\necho cargo %*>> \"%{INVOCATIONS_LOG_ENV_VAR}%\"\r\necho {osc_progress_50}\r\necho {osc_progress_cleared}\r\nexit /b 0\r\n"
        );
        fs::write(&cargo_path, cargo_script).unwrap();
    }
}
