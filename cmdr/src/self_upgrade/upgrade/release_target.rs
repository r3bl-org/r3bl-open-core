// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::super::{UpgradeError, archive, target_platform};
use std::fmt::{Display, Formatter, Result as FmtResult};

/// Supported OS, CPU, and ABI platforms that have pre-compiled release archives.
///
/// Using an enum instead of a raw `&str` guarantees at compile time that [`ReleaseAsset`]
/// only builds filenames and download URLs for platforms we actually ship.
///
/// # ABI Compatibility & Source Fallback
///
/// Checking OS and CPU alone is not enough when two toolchains on the same machine use
/// incompatible C libraries or runtimes (`target_env`):
/// - **Linux ([`glibc`] vs. [`musl`]):** Most Linux desktops, CI runners, and cloud VMs
///   (Ubuntu, Debian, Fedora, RHEL, Amazon Linux) use [`glibc`] (`-gnu`). [`glibc`] is
///   missing on [`musl`]-only systems, mainly minimal Alpine Linux (`FROM alpine`) Docker
///   containers (increasingly replaced in cloud workloads by [`glibc`]-based minimal
///   images like `debian-slim` or Distroless). Since few people run developer CLI tools
///   inside minimal Alpine containers, we skip publishing [`musl`] archives on purpose —
///   and reject [`musl`] in this enum so a [`glibc`] binary (which won't start on Alpine)
///   is never downloaded by mistake.
/// - **Windows ([`MSVC`] vs. [`MinGW`]):** Most Windows Rust installations use [`MSVC`]
///   (`-msvc`), so we skip publishing [`MinGW`] (`-gnu`) archives on purpose.
///
/// Unsupported hosts (such as `linux-x86_64-musl` or `windows-x86_64-gnu`) return
/// [`UpgradeError::UnsupportedPlatform`][1], prompting [`UpgradeSource::try_resolve`] to
/// skip the pre-compiled archive and fall back to
/// [`UpgradeSource::BuildFromSourceRustup`][2] (`cargo +nightly install`) so a native
/// binary is compiled for the host toolchain.
///
/// [1]: crate::self_upgrade::UpgradeError::UnsupportedPlatform
/// [2]: crate::self_upgrade::UpgradeSource::BuildFromSourceRustup
/// [`glibc`]: https://en.wikipedia.org/wiki/Glibc
/// [`MinGW`]: https://en.wikipedia.org/wiki/MinGW
/// [`MSVC`]: https://en.wikipedia.org/wiki/Microsoft_Visual_C%2B%2B
/// [`musl`]: https://en.wikipedia.org/wiki/Musl
/// [`ReleaseAsset`]: crate::self_upgrade::ReleaseAsset
/// [`UpgradeSource::try_resolve`]: crate::self_upgrade::UpgradeSource::try_resolve
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseTarget {
    /// `x86_64-unknown-linux-gnu` (64-bit Intel/AMD Linux, `glibc`).
    LinuxX86_64Gnu,

    /// `aarch64-unknown-linux-gnu` (64-bit ARM Linux, `glibc`).
    LinuxAarch64Gnu,

    /// `x86_64-apple-darwin` (64-bit Intel macOS).
    MacosX86_64,

    /// `aarch64-apple-darwin` (64-bit ARM / Apple Silicon macOS).
    MacosAarch64,

    /// `x86_64-pc-windows-msvc` (64-bit Intel/AMD Windows, MSVC).
    WindowsX86_64Msvc,

    /// `aarch64-pc-windows-msvc` (64-bit ARM Windows, MSVC).
    WindowsAarch64Msvc,
}

impl ReleaseTarget {
    /// Returns the [`ReleaseTarget`] for the current binary's compile-time [`OS`],
    /// [`ARCH`], and `target_env` (via [`Self::try_from_platform`]).
    ///
    /// # Errors
    ///
    /// Returns [`UpgradeError::UnsupportedPlatform`] if the host platform does not map to
    /// a supported [`ReleaseTarget`] (see [ABI Compatibility & Source Fallback]).
    ///
    /// [`ARCH`]: std::env::consts::ARCH
    /// [`OS`]: std::env::consts::OS
    /// [`UpgradeError::UnsupportedPlatform`]: crate::self_upgrade::UpgradeError::UnsupportedPlatform
    /// [ABI Compatibility & Source Fallback]: Self#abi-compatibility--source-fallback
    pub fn try_from_host() -> Result<Self, UpgradeError> {
        Self::try_from_platform(
            std::env::consts::OS,
            std::env::consts::ARCH,
            Self::host_target_env(),
        )
    }

    /// Maps `(os, arch, target_env)` strings to a [`ReleaseTarget`].
    ///
    /// # Errors
    ///
    /// Returns [`UpgradeError::UnsupportedPlatform`] if `(os, arch, target_env)` is
    /// unsupported (see [ABI Compatibility & Source Fallback]).
    ///
    /// [`UpgradeError::UnsupportedPlatform`]: crate::self_upgrade::UpgradeError::UnsupportedPlatform
    /// [ABI Compatibility & Source Fallback]: Self#abi-compatibility--source-fallback
    pub fn try_from_platform(
        os: &str,
        arch: &str,
        target_env: &str,
    ) -> Result<Self, UpgradeError> {
        use target_platform::{ABI_GNU, ABI_MSVC, ABI_NONE, ARCH_AARCH64, ARCH_X86_64,
                              OS_LINUX, OS_MACOS, OS_WINDOWS};

        match (os, arch, target_env) {
            (OS_LINUX, ARCH_X86_64, ABI_GNU) => Ok(Self::LinuxX86_64Gnu),
            (OS_LINUX, ARCH_AARCH64, ABI_GNU) => Ok(Self::LinuxAarch64Gnu),
            (OS_MACOS, ARCH_X86_64, ABI_NONE) => Ok(Self::MacosX86_64),
            (OS_MACOS, ARCH_AARCH64, ABI_NONE) => Ok(Self::MacosAarch64),
            (OS_WINDOWS, ARCH_X86_64, ABI_MSVC) => Ok(Self::WindowsX86_64Msvc),
            (OS_WINDOWS, ARCH_AARCH64, ABI_MSVC) => Ok(Self::WindowsAarch64Msvc),
            _ => {
                let sep = archive::COMPONENT_SEPARATOR;
                let platform = if target_env.is_empty() {
                    format!("{os}{sep}{arch}")
                } else {
                    format!("{os}{sep}{arch}{sep}{target_env}")
                };
                Err(UpgradeError::UnsupportedPlatform(platform))
            }
        }
    }

    /// Returns the Rust target triple string used in release archive filenames.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LinuxX86_64Gnu => target_platform::TRIPLE_LINUX_X86_64_GNU,
            Self::LinuxAarch64Gnu => target_platform::TRIPLE_LINUX_AARCH64_GNU,
            Self::MacosX86_64 => target_platform::TRIPLE_MACOS_X86_64,
            Self::MacosAarch64 => target_platform::TRIPLE_MACOS_AARCH64,
            Self::WindowsX86_64Msvc => target_platform::TRIPLE_WINDOWS_X86_64_MSVC,
            Self::WindowsAarch64Msvc => target_platform::TRIPLE_WINDOWS_AARCH64_MSVC,
        }
    }

    /// Returns the archive file extension (`"tar.gz"`).
    #[must_use]
    pub const fn archive_extension(self) -> &'static str { archive::TAR_GZ_EXT }

    /// Returns the compile-time `target_env` string ([`ABI_GNU`], [`ABI_MSVC`],
    /// [`ABI_MUSL`], or [`ABI_NONE`]).
    ///
    /// Unlike [`OS`] and [`ARCH`], `std::env::consts` has no `TARGET_ENV` constant, so we
    /// read it via `cfg!(target_env = "...")` instead of adding a `build.rs` script.
    ///
    /// [`ABI_GNU`]: target_platform::ABI_GNU
    /// [`ABI_MSVC`]: target_platform::ABI_MSVC
    /// [`ABI_MUSL`]: target_platform::ABI_MUSL
    /// [`ABI_NONE`]: target_platform::ABI_NONE
    /// [`ARCH`]: std::env::consts::ARCH
    /// [`OS`]: std::env::consts::OS
    #[must_use]
    const fn host_target_env() -> &'static str {
        if cfg!(target_env = "gnu") {
            target_platform::ABI_GNU
        } else if cfg!(target_env = "msvc") {
            target_platform::ABI_MSVC
        } else if cfg!(target_env = "musl") {
            target_platform::ABI_MUSL
        } else {
            target_platform::ABI_NONE
        }
    }
}

impl Display for ReleaseTarget {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult { f.write_str(self.as_str()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3bl_tui::assert_eq2;

    #[test]
    fn test_try_from_host_succeeds_on_current_host() {
        let target =
            ReleaseTarget::try_from_host().expect("Current host should be supported");
        assert!(!target.as_str().is_empty());
        assert_eq2!(target.to_string(), target.as_str());
        assert_eq2!(target.archive_extension(), "tar.gz");
    }

    #[test]
    fn test_try_from_platform_matrix_and_abi_rejection() {
        assert_eq2!(
            ReleaseTarget::try_from_platform("linux", "x86_64", "gnu")
                .unwrap()
                .as_str(),
            "x86_64-unknown-linux-gnu"
        );
        assert_eq2!(
            ReleaseTarget::try_from_platform("linux", "aarch64", "gnu")
                .unwrap()
                .as_str(),
            "aarch64-unknown-linux-gnu"
        );
        assert_eq2!(
            ReleaseTarget::try_from_platform("macos", "x86_64", "")
                .unwrap()
                .as_str(),
            "x86_64-apple-darwin"
        );
        assert_eq2!(
            ReleaseTarget::try_from_platform("macos", "aarch64", "")
                .unwrap()
                .as_str(),
            "aarch64-apple-darwin"
        );
        assert_eq2!(
            ReleaseTarget::try_from_platform("windows", "x86_64", "msvc")
                .unwrap()
                .as_str(),
            "x86_64-pc-windows-msvc"
        );
        assert_eq2!(
            ReleaseTarget::try_from_platform("windows", "aarch64", "msvc")
                .unwrap()
                .as_str(),
            "aarch64-pc-windows-msvc"
        );

        // Reject musl on Linux and gnu on Windows so ABI-incompatible binaries are never
        // installed.
        assert!(matches!(
            ReleaseTarget::try_from_platform("linux", "x86_64", "musl"),
            Err(UpgradeError::UnsupportedPlatform(ref p)) if p == "linux-x86_64-musl"
        ));
        assert!(matches!(
            ReleaseTarget::try_from_platform("windows", "x86_64", "gnu"),
            Err(UpgradeError::UnsupportedPlatform(ref p)) if p == "windows-x86_64-gnu"
        ));
        assert!(matches!(
            ReleaseTarget::try_from_platform("freebsd", "riscv64", ""),
            Err(UpgradeError::UnsupportedPlatform(ref p)) if p == "freebsd-riscv64"
        ));
    }
}
