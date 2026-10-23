// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CrateName, CrateVersion, UpdateBehavior};

/// Configuration passed to the self-upgrade engine. This is how a binary (like `edi` or
/// `giti`) tells the upgrade engine how it should handle its (self) update & upgrade.
///
/// This engine doesn't have side effects from environment variables. `CI=<bool>` does
/// **not** influence its behavior.
///
/// 1. Binaries (such as `giti` or `edi`) **always** construct this struct via
///    [`Self::new()`].
/// 2. To support standard CLI arguments, binaries may include [`UpgradeCLIArgs`] in their
///    [`clap`] parser via `#[command(flatten)]` to make them top-level flags (and not
///    sub-commands):
///    - [`UpgradeCLIArgs::update_behavior`]: This is the only CLI flag that influences
///      this config. Its value can be passed to [`Self::new()`] as
///      `maybe_update_behavior` argument.
///    - [`UpgradeCLIArgs::upgrade`]: Does not affect this configuration.
///
/// Callers can also construct this struct programmatically with explicit
/// [`UpdateBehavior`] settings (e.g. in integration tests).
///
/// Use [`Self::new()`] to construct this struct.
///
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::self_update::R3BL_VERSIONS_JSON_URL
/// [`UpgradeCLIArgs::update_behavior`]: crate::core::script::self_update::UpgradeCLIArgs::update_behavior
/// [`UpgradeCLIArgs::upgrade`]: crate::core::script::self_update::UpgradeCLIArgs::upgrade
/// [`UpgradeCLIArgs`]: crate::core::script::self_update::UpgradeCLIArgs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfBinaryUpgradeConfig {
    /// Name of the crate as registered in [`R3BL_VERSIONS_JSON_URL`] (e.g.,
    /// `"r3bl-cmdr"`).
    ///
    /// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::self_update::R3BL_VERSIONS_JSON_URL
    pub crate_name: CrateName,

    /// Current version of the binary compiled from `Cargo.toml` which you can get from
    /// [`env!("CARGO_PKG_VERSION")`][env-macro], which returns something like `0.0.25`.
    /// This is typically parsed into a [`CrateVersion`] via [`CrateVersion::try_from`].
    ///
    /// [env-macro]: std::env!
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
        /// * `crate_name` - The crate name as registered in [`R3BL_VERSIONS_JSON_URL`]
        ///   (e.g., `"r3bl-cmdr"`, `"r3bl-build-infra"`).
        /// * `current_version` - Semantic version of the running binary, typically parsed
        ///   from [`env!("CARGO_PKG_VERSION")`][env-macro] via
        ///   [`CrateVersion::try_from`].
        /// * `maybe_update_behavior` - Optional override (from CLI args or programmatic
        ///   configuration). When `None`, falls back to [`UpdateBehavior::Auto`].
        /// * `user_feedback_channel` - Output channel where upgrade messages, spinners,
        ///   and notices are written.
        ///
        /// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::self_update::R3BL_VERSIONS_JSON_URL
        /// [env-macro]: std::env!
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_eq2;

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
}
