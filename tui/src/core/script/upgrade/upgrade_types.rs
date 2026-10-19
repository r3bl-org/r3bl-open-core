// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Core types, configuration, and state definitions for the unified binary self-upgrade
//! engine. See [`UpgradeConfig`], [`UpgradeSource`], [`UpgradePolicy`],
//! [`UpgradeOutputTarget`], and [`CrateDistribution`] for details.

use super::{CrateDistribution, CrateName, CrateVersion};

/// Configuration passed to the upgrade engine, to upgrade a single binary.
///
/// Unlike tools that implicitly alter their behavior or downgrade policies when
/// `CI=true`, this engine is 100% deterministic: `CI=<bool>` does **not** influence
/// policy resolution. All upgrade behaviors are controlled strictly through:
/// 1. Explicit CLI flags (such as passing [`UpgradePolicy`] via `--upgrade-policy`). For
///    examples of this, you can look at the `CLIArg` structs in our binary crates like
///    `r3bl-cmdr`, `r3bl-build-infra`, `r3bl-rust-analyzer-mcp-server`.
/// 2. Programmatic configuration composition to eliminate hidden behavior, ambient state
///    dependencies, and ambiguity.
///
/// # Policy Precedence
///
/// When determining the effective [`UpgradePolicy`] in [`Self::new`]:
/// 1. **Explicit CLI override**: If a CLI override (`Option<UpgradePolicy>`) is provided
///    (e.g., via `--upgrade-policy`), that explicit choice is always used.
/// 2. **Default policy**: If no CLI override is provided (`None`), the binary's provided
///    default [`UpgradePolicy`] is used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeConfig {
    /// Name of the crate as registered in `r3bl.com/versions.jsonc` (e.g.,
    /// `"r3bl-cmdr"`).
    pub crate_name: CrateName,

    /// Local version of the binary compiled from `Cargo.toml`
    /// (`env!("CARGO_PKG_VERSION")`).
    pub local_version: CrateVersion,

    /// Behavior when an upgrade is available at application exit.
    pub policy: UpgradePolicy,

    /// Stream where progress messages, spinners, and notifications are written.
    pub output_target: UpgradeOutputTarget,
}

mod impl_upgrade_config {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl UpgradeConfig {
        /// Creates a new [`UpgradeConfig`].
        ///
        /// The effective [`UpgradePolicy`] is resolved by
        /// `cli_policy.unwrap_or(default_policy)`. See the [`UpgradeConfig`] struct-level
        /// documentation for details on policy precedence and explicit configuration
        /// rules.
        #[must_use]
        pub fn new(
            crate_name: impl Into<CrateName>,
            local_version: &str,
            cli_policy: Option<UpgradePolicy>,
            default_policy: UpgradePolicy,
            output_target: UpgradeOutputTarget,
        ) -> Self {
            let policy = cli_policy.unwrap_or(default_policy);
            let local_version = CrateVersion::try_from(local_version).unwrap_or_default();

            Self {
                crate_name: crate_name.into(),
                local_version,
                policy,
                output_target,
            }
        }
    }
}

/// Resolved execution source for performing the upgrade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpgradeSource {
    /// Pre-compiled binary archive hosted on `https://github.com/r3bl-org/r3bl-binaries`.
    GithubReleaseArtifact {
        /// Direct download URL for the `.tar.gz` archive.
        archive_url: String,

        /// Direct download URL for the `.sha256` checksum file.
        sha256_url: String,

        /// Pinned target version string from `versions.jsonc`.
        pinned_version: CrateVersion,
    },

    /// Build from `crates.io` source using `rustup toolchain install nightly --force`
    /// followed by `cargo +nightly install <crate> --version <pinned_version>`.
    /// Only eligible when [`CrateDistribution::OpenSource`].
    BuildFromSourceRustup {
        /// Pinned target version string from `versions.jsonc`.
        pinned_version: CrateVersion,
    },

    /// Build from `crates.io` source directly using
    /// `cargo +nightly install <crate> --version <pinned_version>` (when `cargo` is on
    /// `$PATH` without `rustup`).
    /// Only eligible when [`CrateDistribution::OpenSource`].
    BuildFromSourceCargo {
        /// Pinned target version string from `versions.jsonc`.
        pinned_version: CrateVersion,
    },
}

/// Upgrade policy determining how the binary behaves at exit when a newer version is
/// available.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    clap::ValueEnum,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum UpgradePolicy {
    /// Interactive prompt asking the user (`"Yes, upgrade now"` / `"No, thanks"`) at
    /// exit. Used by interactive TUI apps like `giti` and `edi`.
    #[value(name = "interactive-prompt", alias = "prompt", alias = "interactive")]
    InteractivePrompt,

    /// Automatically downloads and installs the upgrade at exit without prompting.
    /// Used by non-interactive CLI/agent tools like `cargo-rustdoc-fmt` and
    /// `rust-analyzer-mcp-server`.
    #[value(name = "auto-upgrade", alias = "auto")]
    AutoUpgrade,

    /// Prints a one-line notification that a newer version is available without running
    /// the upgrade. Selected explicitly via `--upgrade-policy notify-only`.
    #[value(name = "notify-only", alias = "notify")]
    NotifyOnly,

    /// Suppresses version checks and exit upgrade handling entirely.
    #[value(name = "never", alias = "off", alias = "disabled")]
    Never,
}

/// Target output stream for upgrade messages, spinners, and progress reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeOutputTarget {
    /// Write upgrade output to [`stdout`] (default for standard CLI and TUI tools).
    ///
    /// [`stdout`]: std::io::stdout
    Stdout,

    /// Write upgrade output exclusively to [`stderr`] (mandatory for
    /// `rust-analyzer-mcp-server` and [`stdout`]-piped utilities like `env-source` so
    /// [`stdout`] stays 100% uncorrupted).
    ///
    /// [`stderr`]: std::io::stderr
    /// [`stdout`]: std::io::stdout
    Stderr,

    /// Suppress terminal output and emit structured `tracing` events only.
    Quiet,
}

mod impl_upgrade_output_target {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl UpgradeOutputTarget {
        /// Prints a line to the configured output target.
        pub fn println(&self, msg: impl AsRef<str>) {
            match self {
                Self::Stdout => println!("{}", msg.as_ref()),
                Self::Stderr => eprintln!("{}", msg.as_ref()),
                Self::Quiet => {
                    tracing::info!(message = %msg.as_ref());
                }
            }
        }

        /// Prints an error line to the configured output target ([`stderr`] for both
        /// `Stdout` and `Stderr` targets, `tracing::error!` for `Quiet`).
        ///
        /// [`stderr`]: std::io::stderr
        pub fn eprintln(&self, msg: impl AsRef<str>) {
            match self {
                Self::Stdout | Self::Stderr => eprintln!("{}", msg.as_ref()),
                Self::Quiet => {
                    tracing::error!(message = %msg.as_ref());
                }
            }
        }
    }
}

/// Atomic state representation of the background version check.
///
/// Stored inside an [`AtomicU8`] and manipulated via [`AtomicU8Ext`].
///
/// [`AtomicU8`]: std::sync::atomic::AtomicU8
/// [`AtomicU8Ext`]: crate::AtomicU8Ext
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UpgradeCheckResult {
    /// Background version check has not yet started.
    NotChecked = 0,

    /// Background version check is currently in flight.
    Checking = 1,

    /// Local binary version is greater than or equal to the remote `versions.jsonc`
    /// version.
    UpToDate = 2,

    /// Remote `versions.jsonc` version is strictly greater than local binary version.
    UpgradeAvailable = 3,

    /// Network or parsing error occurred while checking `versions.jsonc`.
    FailedCheck = 4,
}

mod impl_upgrade_check_result {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<u8> for UpgradeCheckResult {
        fn from(value: u8) -> Self {
            match value {
                0 => Self::NotChecked,
                1 => Self::Checking,
                2 => Self::UpToDate,
                3 => Self::UpgradeAvailable,
                _ => Self::FailedCheck,
            }
        }
    }

    impl From<UpgradeCheckResult> for u8 {
        fn from(value: UpgradeCheckResult) -> Self {
            match value {
                UpgradeCheckResult::NotChecked => 0,
                UpgradeCheckResult::Checking => 1,
                UpgradeCheckResult::UpToDate => 2,
                UpgradeCheckResult::UpgradeAvailable => 3,
                UpgradeCheckResult::FailedCheck => 4,
            }
        }
    }
}

/// Errors that can occur during strategy resolution or upgrade execution.
#[derive(Debug, thiserror::Error)]
pub enum UpgradeError {
    /// Neither GitHub release binaries nor a viable local source build toolchain are
    /// available.
    #[error(
        "No viable upgrade source for '{crate_name}' (distribution={crate_distribution}): {reason}"
    )]
    NoViableUpgradeSource {
        crate_name: CrateName,
        crate_distribution: CrateDistribution,
        reason: String,
    },

    /// Host platform or architecture does not have pre-compiled binaries and cannot fall
    /// back.
    #[error("Unsupported host target platform: {0}")]
    UnsupportedPlatform(String),

    /// SHA-256 checksum mismatch on downloaded archive.
    #[error("SHA-256 checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    /// User canceled the operation via `Ctrl+C`.
    #[error("Upgrade cancelled by user")]
    Cancelled,

    /// General I/O, network, or archive error.
    #[error("{0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_upgrade_check_result_roundtrip() {
        let variants = [
            UpgradeCheckResult::NotChecked,
            UpgradeCheckResult::Checking,
            UpgradeCheckResult::UpToDate,
            UpgradeCheckResult::UpgradeAvailable,
            UpgradeCheckResult::FailedCheck,
        ];

        for variant in variants {
            let byte: u8 = variant.into();
            let converted = UpgradeCheckResult::from(byte);
            assert_eq2!(variant, converted);
        }

        // Unknown u8 maps to FailedCheck.
        assert_eq2!(
            UpgradeCheckResult::from(99),
            UpgradeCheckResult::FailedCheck
        );
    }

    #[test]
    fn test_upgrade_config_policy_resolution() {
        let make_config = |cli_policy, default_policy| {
            UpgradeConfig::new(
                "r3bl-cmdr",
                "0.0.1",
                cli_policy,
                default_policy,
                UpgradeOutputTarget::Stdout,
            )
        };

        // 1. Explicit CLI override always wins.
        assert_eq2!(
            make_config(
                Some(UpgradePolicy::AutoUpgrade),
                UpgradePolicy::InteractivePrompt,
            )
            .policy,
            UpgradePolicy::AutoUpgrade
        );

        assert_eq2!(
            make_config(Some(UpgradePolicy::Never), UpgradePolicy::AutoUpgrade,).policy,
            UpgradePolicy::Never
        );

        assert_eq2!(
            make_config(
                Some(UpgradePolicy::NotifyOnly),
                UpgradePolicy::InteractivePrompt,
            )
            .policy,
            UpgradePolicy::NotifyOnly
        );

        // 2. When no CLI override is provided (None), falls back to default_policy.
        assert_eq2!(
            make_config(None, UpgradePolicy::AutoUpgrade).policy,
            UpgradePolicy::AutoUpgrade
        );

        assert_eq2!(
            make_config(None, UpgradePolicy::InteractivePrompt).policy,
            UpgradePolicy::InteractivePrompt
        );

        assert_eq2!(
            make_config(None, UpgradePolicy::Never).policy,
            UpgradePolicy::Never
        );
    }

    #[test]
    fn test_upgrade_policy_clap_value_enum() {
        use clap::ValueEnum;

        let prompt = UpgradePolicy::from_str("interactive-prompt", true).unwrap();
        assert_eq2!(prompt, UpgradePolicy::InteractivePrompt);
        let prompt_alias = UpgradePolicy::from_str("prompt", true).unwrap();
        assert_eq2!(prompt_alias, UpgradePolicy::InteractivePrompt);

        let auto = UpgradePolicy::from_str("auto-upgrade", true).unwrap();
        assert_eq2!(auto, UpgradePolicy::AutoUpgrade);
        let auto_alias = UpgradePolicy::from_str("auto", true).unwrap();
        assert_eq2!(auto_alias, UpgradePolicy::AutoUpgrade);

        let notify = UpgradePolicy::from_str("notify-only", true).unwrap();
        assert_eq2!(notify, UpgradePolicy::NotifyOnly);
        let notify_alias = UpgradePolicy::from_str("notify", true).unwrap();
        assert_eq2!(notify_alias, UpgradePolicy::NotifyOnly);

        let never = UpgradePolicy::from_str("never", true).unwrap();
        assert_eq2!(never, UpgradePolicy::Never);
        let off_alias = UpgradePolicy::from_str("off", true).unwrap();
        assert_eq2!(off_alias, UpgradePolicy::Never);
    }
}
