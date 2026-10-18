// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::super::{UpgradeError, archive};
use std::fmt::{Display, Formatter, Result as FmtResult};

/// Canonical Rust target triples supported by pre-compiled release archives in
/// [`R3BL_BINARIES_RELEASE_BASE_URL`].
///
/// Encoding the supported target matrix as a `Copy` enum (rather than `&str`) makes
/// unsupported target triples unrepresentable when formatting release archive names and
/// asset URLs, without requiring a `build.rs` script.
///
/// [`R3BL_BINARIES_RELEASE_BASE_URL`]: crate::self_upgrade::R3BL_BINARIES_RELEASE_BASE_URL
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseTarget {
    /// `x86_64-unknown-linux-gnu` (`glibc`).
    LinuxX86_64Gnu,

    /// `aarch64-unknown-linux-gnu` (`glibc`).
    LinuxAarch64Gnu,

    /// `x86_64-apple-darwin` (Intel macOS).
    MacosX86_64,

    /// `aarch64-apple-darwin` (Apple Silicon macOS).
    MacosAarch64,

    /// `x86_64-pc-windows-msvc`.
    WindowsX86_64Msvc,

    /// `aarch64-pc-windows-msvc`.
    WindowsAarch64Msvc,
}

impl ReleaseTarget {
    /// Canonical LLVM/Rust target triple string used in release asset filenames.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LinuxX86_64Gnu => "x86_64-unknown-linux-gnu",
            Self::LinuxAarch64Gnu => "aarch64-unknown-linux-gnu",
            Self::MacosX86_64 => "x86_64-apple-darwin",
            Self::MacosAarch64 => "aarch64-apple-darwin",
            Self::WindowsX86_64Msvc => "x86_64-pc-windows-msvc",
            Self::WindowsAarch64Msvc => "aarch64-pc-windows-msvc",
        }
    }

    /// Archive file extension for this release target (`"tar.gz"` across all targets).
    #[must_use]
    pub const fn archive_extension(self) -> &'static str { archive::TAR_GZ_EXT }
}

impl Display for ReleaseTarget {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult { f.write_str(self.as_str()) }
}

/// Resolves the [`ReleaseTarget`] for the current host OS, CPU architecture, and
/// C ABI (`target_env`).
///
/// # Errors
///
/// Returns [`UpgradeError::UnsupportedPlatform`] if the host `(os, arch, target_env)`
/// tuple (such as `linux-x86_64-musl` or `freebsd-riscv64`) does not map to a supported
/// pre-compiled [`ReleaseTarget`].
pub fn get_host_target_triple() -> Result<ReleaseTarget, UpgradeError> {
    resolve_target_triple_for(
        std::env::consts::OS,
        std::env::consts::ARCH,
        host_target_env(),
    )
}

/// Compile-time `target_env` discriminator (`"gnu"`, `"msvc"`, `"musl"`, or `""`)
/// resolved via `rustc` `cfg!` predicates without requiring a `build.rs` script.
#[must_use]
const fn host_target_env() -> &'static str {
    if cfg!(target_env = "gnu") {
        "gnu"
    } else if cfg!(target_env = "msvc") {
        "msvc"
    } else if cfg!(target_env = "musl") {
        "musl"
    } else {
        ""
    }
}

/// Pure helper that maps `(os, arch, target_env)` strings to a canonical
/// [`ReleaseTarget`].
///
/// Explicitly checks `target_env` so `musl` Linux builds (`"linux"`, `"x86_64"`,
/// `"musl"`) or `MinGW` Windows builds (`"windows"`, `"x86_64"`, `"gnu"`) never falsely
/// match `glibc` or MSVC pre-compiled binaries.
///
/// # Errors
///
/// Returns [`UpgradeError::UnsupportedPlatform`] if `(os, arch, target_env)` is
/// unsupported.
pub fn resolve_target_triple_for(
    os: &str,
    arch: &str,
    target_env: &str,
) -> Result<ReleaseTarget, UpgradeError> {
    match (os, arch, target_env) {
        ("linux", "x86_64", "gnu") => Ok(ReleaseTarget::LinuxX86_64Gnu),
        ("linux", "aarch64", "gnu") => Ok(ReleaseTarget::LinuxAarch64Gnu),
        ("macos", "x86_64", "") => Ok(ReleaseTarget::MacosX86_64),
        ("macos", "aarch64", "") => Ok(ReleaseTarget::MacosAarch64),
        ("windows", "x86_64", "msvc") => Ok(ReleaseTarget::WindowsX86_64Msvc),
        ("windows", "aarch64", "msvc") => Ok(ReleaseTarget::WindowsAarch64Msvc),
        _ => {
            let platform = if target_env.is_empty() {
                format!("{os}-{arch}")
            } else {
                format!("{os}-{arch}-{target_env}")
            };
            Err(UpgradeError::UnsupportedPlatform(platform))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3bl_tui::assert_eq2;

    #[test]
    fn test_get_host_target_triple_succeeds_on_current_host() {
        let target = get_host_target_triple().expect("Current host should be supported");
        assert!(!target.as_str().is_empty());
        assert_eq2!(target.to_string(), target.as_str());
        assert_eq2!(target.archive_extension(), "tar.gz");
    }

    #[test]
    fn test_resolve_target_triple_matrix_and_abi_rejection() {
        assert_eq2!(
            resolve_target_triple_for("linux", "x86_64", "gnu")
                .unwrap()
                .as_str(),
            "x86_64-unknown-linux-gnu"
        );
        assert_eq2!(
            resolve_target_triple_for("linux", "aarch64", "gnu")
                .unwrap()
                .as_str(),
            "aarch64-unknown-linux-gnu"
        );
        assert_eq2!(
            resolve_target_triple_for("macos", "x86_64", "")
                .unwrap()
                .as_str(),
            "x86_64-apple-darwin"
        );
        assert_eq2!(
            resolve_target_triple_for("macos", "aarch64", "")
                .unwrap()
                .as_str(),
            "aarch64-apple-darwin"
        );
        assert_eq2!(
            resolve_target_triple_for("windows", "x86_64", "msvc")
                .unwrap()
                .as_str(),
            "x86_64-pc-windows-msvc"
        );
        assert_eq2!(
            resolve_target_triple_for("windows", "aarch64", "msvc")
                .unwrap()
                .as_str(),
            "aarch64-pc-windows-msvc"
        );

        // Reject musl on Linux and gnu on Windows so ABI-incompatible binaries are never
        // installed.
        assert!(matches!(
            resolve_target_triple_for("linux", "x86_64", "musl"),
            Err(UpgradeError::UnsupportedPlatform(ref p)) if p == "linux-x86_64-musl"
        ));
        assert!(matches!(
            resolve_target_triple_for("windows", "x86_64", "gnu"),
            Err(UpgradeError::UnsupportedPlatform(ref p)) if p == "windows-x86_64-gnu"
        ));
        assert!(matches!(
            resolve_target_triple_for("freebsd", "riscv64", ""),
            Err(UpgradeError::UnsupportedPlatform(ref p)) if p == "freebsd-riscv64"
        ));
    }
}
