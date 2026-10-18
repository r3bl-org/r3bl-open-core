// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Host target triple detection for binary archive resolution.

use super::{UpgradeError, archive};

/// Resolves the Rust target triple for the current host OS and CPU architecture.
///
/// Supported target triples:
/// - `x86_64-unknown-linux-gnu`
/// - `aarch64-unknown-linux-gnu`
/// - `x86_64-apple-darwin`
/// - `aarch64-apple-darwin`
/// - `x86_64-pc-windows-msvc`
/// - `aarch64-pc-windows-msvc`
///
/// # Errors
///
/// Returns [`UpgradeError::UnsupportedPlatform`] if the current OS and architecture
/// combination does not map to a supported target triple.
pub fn get_host_target_triple() -> Result<&'static str, UpgradeError> {
    resolve_target_triple_for(std::env::consts::OS, std::env::consts::ARCH)
}

/// Pure helper that maps `(os, arch)` strings (matching [`std::env::consts::OS`] and
/// [`std::env::consts::ARCH`]) to a canonical Rust target triple.
///
/// # Errors
///
/// Returns [`UpgradeError::UnsupportedPlatform`] if `(os, arch)` is unsupported.
pub fn resolve_target_triple_for(
    os: &str,
    arch: &str,
) -> Result<&'static str, UpgradeError> {
    match (os, arch) {
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Ok("aarch64-unknown-linux-gnu"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("windows", "x86_64") => Ok("x86_64-pc-windows-msvc"),
        ("windows", "aarch64") => Ok("aarch64-pc-windows-msvc"),
        _ => Err(UpgradeError::UnsupportedPlatform(format!("{os}-{arch}"))),
    }
}

/// Returns the archive file extension for a given target triple (`"tar.gz"` across all
/// platforms).
#[must_use]
pub fn get_archive_extension_for_target(_target_triple: &str) -> &'static str {
    archive::TAR_GZ_EXT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_get_host_target_triple_succeeds_on_current_host() {
        let triple = get_host_target_triple().expect("Current host should be supported");
        assert!(!triple.is_empty());
    }

    #[test]
    fn test_resolve_target_triple_matrix() {
        assert_eq2!(
            resolve_target_triple_for("linux", "x86_64").unwrap(),
            "x86_64-unknown-linux-gnu"
        );
        assert_eq2!(
            resolve_target_triple_for("linux", "aarch64").unwrap(),
            "aarch64-unknown-linux-gnu"
        );
        assert_eq2!(
            resolve_target_triple_for("macos", "x86_64").unwrap(),
            "x86_64-apple-darwin"
        );
        assert_eq2!(
            resolve_target_triple_for("macos", "aarch64").unwrap(),
            "aarch64-apple-darwin"
        );
        assert_eq2!(
            resolve_target_triple_for("windows", "x86_64").unwrap(),
            "x86_64-pc-windows-msvc"
        );
        assert_eq2!(
            resolve_target_triple_for("windows", "aarch64").unwrap(),
            "aarch64-pc-windows-msvc"
        );

        let unsupported = resolve_target_triple_for("freebsd", "riscv64");
        assert!(unsupported.is_err());
    }

    #[test]
    fn test_get_archive_extension_for_target() {
        assert_eq2!(
            get_archive_extension_for_target("x86_64-unknown-linux-gnu"),
            "tar.gz"
        );
        assert_eq2!(
            get_archive_extension_for_target("aarch64-apple-darwin"),
            "tar.gz"
        );
        assert_eq2!(
            get_archive_extension_for_target("x86_64-pc-windows-msvc"),
            "tar.gz"
        );
    }
}
