// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! End-to-end integration test for [`UpgradeSource::ReleaseArchive`] download, SHA-256
//! verification, `.tar.gz` extraction, and in-place binary replacement via
//! [`self_replace`].
//!
//! # Why Subprocess Isolation Is Required
//!
//! [`binary_replacement::apply_extracted_binaries`] inspects [`std::env::current_exe()`]
//! and invokes [`self_replace::self_replace`] on that executable path. Running in-process
//! would overwrite the `cargo test` runner binary inside `target/debug/deps/`.
//!
//! instead, this test uses the **Coordinator / Worker** pattern:
//! 1. **Coordinator**: Copies [`std::env::current_exe()`] into a disposable
//!    [`try_create_temp_dir()`] directory (`temp_dir/bin/<exe_name>`) and spawns that
//!    copy as a child process with `R3BL_TEST_SELF_REPLACE_ISOLATED` set.
//! 2. **Worker**: Runs [`UpgradeJourney::start`] against an ephemeral `127.0.0.1:0`
//!    release server serving a real `.tar.gz` and `.sha256`. Because the worker's
//!    [`std::env::current_exe()`] points to `temp_dir/bin/<exe_name>`,
//!    [`self_replace::self_replace`] safely replaces that temporary copy on disk and
//!    installs the sibling binaries (`edi`, `env-source`) next to it.
//!
//! [`binary_replacement::apply_extracted_binaries`]: crate::self_upgrade::binary_replacement::apply_extracted_binaries
//! [`self_replace::self_replace`]: self_replace::self_replace
//! [`self_replace`]: mod@self_replace
//! [`try_create_temp_dir()`]: r3bl_tui::try_create_temp_dir
//! [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
//! [`UpgradeSource::ReleaseArchive`]: crate::self_upgrade::UpgradeSource::ReleaseArchive

use super::{test_fixtures::*, *};
use crate::{CooldownPolicy, CooldownStatus, CrateVersion, EndpointUrl, JsoncScalar,
            UpgradeConfig, UpgradePolicy, UpgradeStrategy, UserFeedbackChannel,
            sha256_checksum};
use flate2::{Compression, write::GzEncoder};
use r3bl_tui::{assert_eq2, try_create_temp_dir};
use std::{env, fs,
          io::{Read, Write},
          net::TcpListener,
          process::Command};

const ISOLATED_SELF_REPLACE_ENV_VAR: &str = "R3BL_TEST_SELF_REPLACE_ISOLATED";
const TEST_FILTER: &str = "self_upgrade::upgrade_journey::integration_tests::self_replace_test::test_immediate_upgrade_release_archive_downloads_verifies_and_self_replaces";

const UPGRADED_MAIN_BINARY_BYTES: &[u8] = b"#!/bin/sh\necho upgraded-main-v9.9.9\n";
const UPGRADED_EDI_BINARY_BYTES: &[u8] = b"#!/bin/sh\necho upgraded-edi-v9.9.9\n";
const UPGRADED_ENV_SOURCE_BINARY_BYTES: &[u8] =
    b"#!/bin/sh\necho upgraded-env-source-v9.9.9\n";

#[cfg_attr(not(doc), test)]
pub fn test_immediate_upgrade_release_archive_downloads_verifies_and_self_replaces() {
    if env::var(ISOLATED_SELF_REPLACE_ENV_VAR).is_ok() {
        run_self_replace_worker();
        return;
    }

    // Coordinator: copy the test runner binary into a disposable temp directory so the
    // worker subprocess can safely `self_replace` its own executable on disk.
    let temp_dir = try_create_temp_dir().unwrap();
    let bin_dir = temp_dir.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let current_exe = env::current_exe().unwrap();
    let exe_file_name = current_exe.file_name().unwrap();
    let isolated_exe = bin_dir.join(exe_file_name);
    fs::copy(&current_exe, &isolated_exe).unwrap();

    let output = Command::new(&isolated_exe)
        .env(ISOLATED_SELF_REPLACE_ENV_VAR, "1")
        .arg("--exact")
        .arg(TEST_FILTER)
        .arg("--nocapture")
        .output()
        .expect("Failed to spawn isolated self_replace worker subprocess");

    if !output.status.success() {
        panic!(
            "Isolated self_replace worker failed ({}):\n--- stdout ---\n{}\n--- stderr ---\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

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

/// Runs inside the isolated subprocess whose [`std::env::current_exe()`] points to
/// `temp_dir/bin/<exe_name>`.
fn run_self_replace_worker() {
    SingletonState::reset_for_testing();

    let current_exe = env::current_exe().unwrap();
    let current_exe_name = current_exe
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let archive_bytes = build_release_tar_gz(&current_exe_name);
    let archive_sha256 = sha256_checksum::compute_sha256_hex(&archive_bytes);

    let (manifest_url, server_handle) =
        spawn_release_archive_http_server(archive_bytes, archive_sha256);

    let cooldown_temp_dir = try_create_temp_dir().unwrap();
    let timestamp_file_path = cooldown_temp_dir.join("self_replace.last_check");

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

/// Spawns a local HTTP server that handles the 4 sequential requests made during a
/// release-archive upgrade:
/// 1. `GET /versions.jsonc`
/// 2. `HEAD /releases/download/.../*.tar.gz`
/// 3. `GET /releases/download/.../*.tar.gz.sha256`
/// 4. `GET /releases/download/.../*.tar.gz`
fn spawn_release_archive_http_server(
    archive_bytes: Vec<u8>,
    archive_sha256: String,
) -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let release_base_url = format!("http://{addr}/releases/download");
    let manifest_body = with_release_base_url(
        MANIFEST_UPGRADE_AVAILABLE_OPEN_SOURCE_JSONC,
        &release_base_url,
    );

    let handle = std::thread::spawn(move || {
        for _ in 0..4 {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            let mut req_buf = [0_u8; 1024];
            let n = stream.read(&mut req_buf).unwrap_or(0);
            let request_line = String::from_utf8_lossy(&req_buf[..n]);
            let first_line = request_line.lines().next().unwrap_or("");

            if first_line.starts_with("GET /versions.jsonc") {
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{manifest_body}",
                    manifest_body.len()
                );
                drop(stream.write_all(resp.as_bytes()));
            } else if first_line.starts_with("HEAD ") {
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    archive_bytes.len()
                );
                drop(stream.write_all(resp.as_bytes()));
            } else if first_line.contains(".sha256") {
                let sha_body = format!("{archive_sha256}  r3bl-cmdr-v9.9.9.tar.gz\n");
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{sha_body}",
                    sha_body.len()
                );
                drop(stream.write_all(resp.as_bytes()));
            } else {
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    archive_bytes.len()
                );
                drop(stream.write_all(header.as_bytes()));
                drop(stream.write_all(&archive_bytes));
            }
            drop(stream.flush());
        }
    });

    (format!("http://{addr}/versions.jsonc"), handle)
}
