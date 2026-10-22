// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Core types, configuration, and state definitions for the unified binary self-upgrade
//! engine. See [`SelfBinaryUpgradeConfig`], [`UpgradeSource`], [`UpdateBehavior`],
//! [`UserFeedbackChannel`], and [`CrateDistribution`] for details.

use super::{CrateDistribution, CrateName, CrateVersion};

/// Configuration passed to the self-upgrade engine. This is how a binary tells the
/// upgrade engine how it should do the update & upgrade.
///
/// Unlike tools that implicitly alter their behavior or downgrade actions when `CI=true`,
/// this engine doesn't have side effects from environment variables: `CI=<bool>` does
/// **not** influence behavior resolution. All upgrade behaviors are controlled strictly
/// by either:
/// - Explicit CLI flags (such as passing [`UpdateBehavior`] via `--update-behavior`). For
///   examples of this, you can look at the `CLIArg` structs in our binary crates like
///   `r3bl-cmdr`, `r3bl-build-infra`, `r3bl-rust-analyzer-mcp-server`.
/// - Programmatically creating this config to eliminate ambiguity.
///
/// Use [`Self::new()`] to construct this struct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfBinaryUpgradeConfig {
    /// Name of the crate as registered in `r3bl.com/versions.jsonc` (e.g.,
    /// `"r3bl-cmdr"`).
    pub crate_name: CrateName,

    /// Current version of the binary compiled from `Cargo.toml`
    /// (`env!("CARGO_PKG_VERSION")`).
    pub current_version: CrateVersion,

    /// Behavior governing background update checks and application exit handling.
    pub update_behavior: UpdateBehavior,

    /// Channel where progress messages, spinners, and notifications are written.
    pub user_feedback_channel: UserFeedbackChannel,
}

mod impl_self_binary_upgrade_config {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl SelfBinaryUpgradeConfig {
        /// Creates a new [`SelfBinaryUpgradeConfig`].
        ///
        /// # Arguments
        ///
        /// * `crate_name` - The crate name as registered in `r3bl.com/versions.jsonc`
        ///   (e.g., `"r3bl-cmdr"`, `"r3bl-build-infra"`).
        /// * `current_version` - Semantic version of the running binary, typically parsed
        ///   from `env!("CARGO_PKG_VERSION")` via [`CrateVersion::try_from`].
        /// * `maybe_update_behavior` - Optional override (from CLI args or programmatic
        ///   configuration). When `None`, falls back to [`UpdateBehavior::Auto`].
        /// * `user_feedback_channel` - Output channel where upgrade messages, spinners,
        ///   and notices are written.
        pub fn new(
            crate_name: impl Into<CrateName>,
            current_version: CrateVersion,
            maybe_update_behavior: Option<UpdateBehavior>,
            user_feedback_channel: UserFeedbackChannel,
        ) -> Self {
            let update_behavior = maybe_update_behavior.unwrap_or_default();

            Self {
                crate_name: crate_name.into(),
                current_version,
                update_behavior,
                user_feedback_channel,
            }
        }
    }
}

/// Resolved execution source for performing the upgrade.
///
/// "Resolved" signifies that the upgrade engine has evaluated runtime environment
/// conditions (host target architecture, remote asset availability on GitHub, crate
/// distribution model, and local Rust toolchain availability on `$PATH`) to determine
/// the exact execution path before initiating downloads or builds.
///
/// Evaluated by [`resolve_upgrade_strategy`] prior to calling
/// [`execute_upgrade_with_metadata`].
///
/// [`execute_upgrade_with_metadata`]: crate::core::script::upgrade::execute_upgrade_with_metadata
/// [`resolve_upgrade_strategy`]: crate::core::script::upgrade::resolve_upgrade_strategy
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpgradeSource {
    /// Pre-compiled binary archive hosted on [`R3BL_BINARIES_RELEASE_BASE_URL`].
    ///
    /// [`R3BL_BINARIES_RELEASE_BASE_URL`]: crate::core::script::upgrade::R3BL_BINARIES_RELEASE_BASE_URL
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

/// Behavior governing background update checks and application exit handling.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    clap::ValueEnum,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateBehavior {
    /// Interactive prompt asking the user (`"Yes, upgrade now"` / `"No, thanks"`) at
    /// exit if a newer version is available.
    #[value(name = "prompt-to-upgrade")]
    PromptToUpgrade,

    /// Automatically downloads and installs the upgrade at exit without prompting if a
    /// newer version is available. Default behavior for all workspace binaries.
    #[default]
    #[value(name = "auto")]
    Auto,

    /// Prints a one-line notification that a newer version is available without running
    /// the upgrade. Selected explicitly via `--update-behavior notify-only`.
    #[value(name = "notify-only")]
    NotifyOnly,

    /// Suppresses background version checks and exit upgrade handling entirely.
    #[value(name = "disable")]
    Disable,
}

/// Target output channel for upgrade messages, spinners, and progress reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserFeedbackChannel {
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

mod impl_user_feedback_channel {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl UserFeedbackChannel {
        /// Prints a line to the configured feedback channel.
        pub fn println(&self, msg: impl AsRef<str>) {
            match self {
                Self::Stdout => println!("{}", msg.as_ref()),
                Self::Stderr => eprintln!("{}", msg.as_ref()),
                Self::Quiet => {
                    tracing::info!(message = %msg.as_ref());
                }
            }
        }

        /// Prints an error line to the configured feedback channel ([`stderr`] for both
        /// `Stdout` and `Stderr` channels, `tracing::error!` for `Quiet`).
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
    fn test_self_binary_upgrade_config_behavior_resolution() {
        let make_config = |maybe_behavior| {
            SelfBinaryUpgradeConfig::new(
                "r3bl-cmdr",
                CrateVersion {
                    major: 0,
                    minor: 0,
                    patch: 1,
                },
                maybe_behavior,
                UserFeedbackChannel::Stdout,
            )
        };

        // 1. Explicit CLI override always wins.
        assert_eq2!(
            make_config(Some(UpdateBehavior::PromptToUpgrade)).update_behavior,
            UpdateBehavior::PromptToUpgrade
        );

        assert_eq2!(
            make_config(Some(UpdateBehavior::Disable)).update_behavior,
            UpdateBehavior::Disable
        );

        assert_eq2!(
            make_config(Some(UpdateBehavior::NotifyOnly)).update_behavior,
            UpdateBehavior::NotifyOnly
        );

        assert_eq2!(
            make_config(Some(UpdateBehavior::Auto)).update_behavior,
            UpdateBehavior::Auto
        );

        // 2. When no CLI override is provided (None), falls back to universal default
        //    Auto.
        assert_eq2!(make_config(None).update_behavior, UpdateBehavior::Auto);
    }

    #[test]
    fn test_update_behavior_clap_value_enum() {
        use clap::ValueEnum;

        let prompt = UpdateBehavior::from_str("prompt-to-upgrade", true).unwrap();
        assert_eq2!(prompt, UpdateBehavior::PromptToUpgrade);

        let auto = UpdateBehavior::from_str("auto", true).unwrap();
        assert_eq2!(auto, UpdateBehavior::Auto);
        assert_eq2!(UpdateBehavior::default(), UpdateBehavior::Auto);

        let notify = UpdateBehavior::from_str("notify-only", true).unwrap();
        assert_eq2!(notify, UpdateBehavior::NotifyOnly);

        let disable = UpdateBehavior::from_str("disable", true).unwrap();
        assert_eq2!(disable, UpdateBehavior::Disable);

        // Verify that aliases and obsolete names are rejected to prevent ambiguity.
        assert!(UpdateBehavior::from_str("prompt", true).is_err());
        assert!(UpdateBehavior::from_str("interactive", true).is_err());
        assert!(UpdateBehavior::from_str("interactive-prompt", true).is_err());
        assert!(UpdateBehavior::from_str("auto-upgrade", true).is_err());
        assert!(UpdateBehavior::from_str("notify", true).is_err());
        assert!(UpdateBehavior::from_str("disabled", true).is_err());
        assert!(UpdateBehavior::from_str("none", true).is_err());
        assert!(UpdateBehavior::from_str("never", true).is_err());
        assert!(UpdateBehavior::from_str("off", true).is_err());
    }
}
