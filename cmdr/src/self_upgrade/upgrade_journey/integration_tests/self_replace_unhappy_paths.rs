// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unhappy-path integration tests for binary release archive verification and extraction
//! ([`CrateDistribution::ClosedSource`], so archive download/verification errors never
//! fall back to `cargo install`):
//! - SHA-256 checksum mismatch -> [`UpgradeError::ChecksumMismatch`].
//! - `GET *.tar.gz` HTTP 500 after `HEAD 200` -> [`UpgradeError::Other`].
//! - Empty `.tar.gz` archive -> [`UpgradeError::Other`] (`"contained no files"`).
//! - Corrupt non-gzip archive bytes -> [`UpgradeError::Other`].
//!
//! All scenarios run in an isolated Coordinator / Worker subprocess on a copied
//! executable and assert that the executable bytes on disk remain unchanged, no sibling
//! binaries are created, and no cooldown timestamp file is created
//! ([`GateStatusError::MetadataReadFailed`] with [`ErrorKind::NotFound`]).
//!
//! [`CrateDistribution::ClosedSource`]: crate::self_upgrade::CrateDistribution::ClosedSource
//! [`GateStatusError::MetadataReadFailed`]: crate::self_upgrade::GateStatusError::MetadataReadFailed
//! [`UpgradeError::ChecksumMismatch`]: crate::self_upgrade::UpgradeError::ChecksumMismatch
//! [`UpgradeError::Other`]: crate::self_upgrade::UpgradeError::Other

use super::{super::per_process_start_latch::PerProcessStartLatch, test_fixtures::*};
use crate::{CrateDistribution, CrateVersion, GateStatusError, JsoncScalar,
            RemoteVersionCheckGate, Sha256Digest, UpgradeConfig, UpgradeError,
            UpgradePolicy, UpgradeStrategy, UserFeedbackChannel, self_upgrade::*};
use flate2::{Compression, write::GzEncoder};
use r3bl_tui::{TempDirChildPath, assert_eq2, try_create_temp_dir};
use std::{env, fs, io::ErrorKind, process::Command};

const ISOLATED_SELF_REPLACE_UNHAPPY_ENV_VAR: &str =
    "R3BL_TEST_SELF_REPLACE_UNHAPPY_ISOLATED";
const CHECKSUM_MISMATCH_FILTER: &str = "self_upgrade::upgrade_journey::integration_tests::self_replace_unhappy_paths::test_checksum_mismatch_fails_without_replacing_binary";
const ARCHIVE_GET_500_FILTER: &str = "self_upgrade::upgrade_journey::integration_tests::self_replace_unhappy_paths::test_archive_get_http_500_fails_without_replacing_binary";
const EMPTY_ARCHIVE_FILTER: &str = "self_upgrade::upgrade_journey::integration_tests::self_replace_unhappy_paths::test_empty_archive_fails_without_replacing_binary";
const CORRUPT_ARCHIVE_FILTER: &str = "self_upgrade::upgrade_journey::integration_tests::self_replace_unhappy_paths::test_corrupt_archive_fails_without_replacing_binary";

#[test]
fn test_checksum_mismatch_fails_without_replacing_binary() {
    if env::var(ISOLATED_SELF_REPLACE_UNHAPPY_ENV_VAR).is_ok() {
        let current_exe_name = current_exe_filename();
        let archive_bytes =
            build_single_file_tar_gz(&current_exe_name, b"valid-tar-gz-payload");
        let actual_digest = Sha256Digest::from_bytes(&archive_bytes);
        let wrong_sha256 = Sha256Digest::try_decode_hex_str(
            "0000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap();
        let res_server_spawn = archive_server::spawn_checksum_mismatch_closed_source(
            archive_bytes,
            wrong_sha256,
        );
        run_self_replace_unhappy_worker(res_server_spawn, |err| {
            assert_eq2!(
                err,
                &UpgradeError::ChecksumMismatch {
                    expected: wrong_sha256,
                    actual: actual_digest,
                }
            );
        });
        return;
    }
    run_self_replace_unhappy_coordinator(CHECKSUM_MISMATCH_FILTER);
}

#[test]
fn test_archive_get_http_500_fails_without_replacing_binary() {
    if env::var(ISOLATED_SELF_REPLACE_UNHAPPY_ENV_VAR).is_ok() {
        let res_server_spawn = archive_server::spawn_archive_get_error(
            CrateDistribution::ClosedSource,
            blocks::HTTP_500_INTERNAL_SERVER_ERROR,
        );
        run_self_replace_unhappy_worker(res_server_spawn, |err| {
            assert!(matches!(err, UpgradeError::Other(msg) if !msg.is_empty()));
        });
        return;
    }
    run_self_replace_unhappy_coordinator(ARCHIVE_GET_500_FILTER);
}

#[test]
fn test_empty_archive_fails_without_replacing_binary() {
    if env::var(ISOLATED_SELF_REPLACE_UNHAPPY_ENV_VAR).is_ok() {
        let enc = GzEncoder::new(Vec::new(), Compression::default());
        let tar_builder = tar::Builder::new(enc);
        let enc = tar_builder.into_inner().unwrap();
        let empty_tar_gz = enc.finish().unwrap();

        let res_server_spawn =
            archive_server::spawn_closed_source_success(empty_tar_gz);
        run_self_replace_unhappy_worker(res_server_spawn, |err| {
            assert!(
                matches!(err, UpgradeError::Other(msg) if msg.contains("contained no files")),
                "Expected 'contained no files' error, got {err:?}"
            );
        });
        return;
    }
    run_self_replace_unhappy_coordinator(EMPTY_ARCHIVE_FILTER);
}

#[test]
fn test_corrupt_archive_fails_without_replacing_binary() {
    if env::var(ISOLATED_SELF_REPLACE_UNHAPPY_ENV_VAR).is_ok() {
        let res_server_spawn = archive_server::spawn_closed_source_success(
            b"corrupt-not-a-gzip-archive".to_vec(),
        );
        run_self_replace_unhappy_worker(res_server_spawn, |err| {
            assert!(matches!(err, UpgradeError::Other(msg) if !msg.is_empty()));
        });
        return;
    }
    run_self_replace_unhappy_coordinator(CORRUPT_ARCHIVE_FILTER);
}

/// Coordinator: copies the test runner binary into a disposable temp directory, records
/// its initial SHA-256 digest, runs `test_filter` inside the child copy, and asserts the
/// copied binary bytes remain unchanged on disk.
fn run_self_replace_unhappy_coordinator(test_filter: &str) {
    let temp_dir = try_create_temp_dir().unwrap();
    let bin_dir = temp_dir.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let current_exe = env::current_exe().unwrap();
    let exe_file_name = current_exe.file_name().unwrap();
    let isolated_exe = bin_dir.join(exe_file_name);
    fs::copy(&current_exe, &isolated_exe).unwrap();

    let original_digest = Sha256Digest::from_bytes(&fs::read(&isolated_exe).unwrap());

    let output = Command::new(&isolated_exe)
        .env(ISOLATED_SELF_REPLACE_UNHAPPY_ENV_VAR, "1")
        .arg("--exact")
        .arg(test_filter)
        .arg("--nocapture")
        .output()
        .expect("Failed to spawn isolated self_replace unhappy worker subprocess");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "Isolated self_replace unhappy worker failed ({}):\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
        output.status,
    );

    // Verify executable bytes remained untouched on disk and no sibling binaries were
    // written.
    assert_eq2!(
        Sha256Digest::from_bytes(&fs::read(&isolated_exe).unwrap()),
        original_digest
    );
    assert!(!bin_dir.join("edi").exists());
    assert!(!bin_dir.join("env-source").exists());
}

fn run_self_replace_unhappy_worker(
    res_server_spawn: ServerSpawnResult,
    assert_err: impl FnOnce(&UpgradeError),
) {
    // Arrange.
    PerProcessStartLatch::reset_for_testing();
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

fn current_exe_filename() -> String {
    env::current_exe()
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

fn build_single_file_tar_gz(entry_name: &str, bytes: &[u8]) -> Vec<u8> {
    let enc = GzEncoder::new(Vec::new(), Compression::default());
    let mut tar_builder = tar::Builder::new(enc);
    let mut header = tar::Header::new_gnu();
    header.set_size(u64::try_from(bytes.len()).unwrap());
    header.set_mode(0o755);
    header.set_cksum();
    tar_builder
        .append_data(&mut header, entry_name, bytes)
        .unwrap();
    let enc = tar_builder.into_inner().unwrap();
    enc.finish().unwrap()
}
