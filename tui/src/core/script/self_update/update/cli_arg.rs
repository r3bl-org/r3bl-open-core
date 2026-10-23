// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Command-line arguments for self-upgrade operations and background update behaviors.
///
/// Workspace binaries flatten this struct into their top-level [`clap::Parser`] or
/// [`clap::Args`] struct via `#[command(flatten)]` to provide consistent `--upgrade` and
/// `--update-behavior` flags across all tools.
///
/// See the [`self_update`] module documentation for the complete self-upgrade
/// architecture and lifecycle.
///
/// # Mutual Exclusivity
///
/// The [`upgrade`] and [`update_behavior`] arguments are mutually exclusive. Passing
/// both results in a [`clap`] validation error because `--upgrade` triggers an
/// immediate upgrade and exits at startup, whereas `--update-behavior` configures
/// passive background check handling for normal application sessions.
///
/// # Example
///
/// ```
/// use clap::Parser;
/// use r3bl_tui::{UpdateBehavior, UpgradeCLIArgs};
///
/// #[derive(Parser, Debug)]
/// struct CLIArg {
///     #[command(flatten)]
///     upgrade_args: UpgradeCLIArgs,
/// }
///
/// let args = CLIArg::parse_from(["my-binary", "--upgrade"]);
/// assert!(args.upgrade_args.upgrade);
/// assert_eq!(args.upgrade_args.update_behavior, None);
/// ```
///
/// [`clap::Args`]: clap::Args
/// [`clap::Parser`]: clap::Parser
/// [`clap`]: mod@clap
/// [`self_update`]: mod@crate::core::script::self_update
/// [`update_behavior`]: Self::update_behavior
/// [`upgrade`]: Self::upgrade
#[derive(clap::Args, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UpgradeCLIArgs {
    /// Trigger an immediate self-upgrade to the latest release and exit.
    #[arg(long = "upgrade", conflicts_with = "update_behavior")]
    pub upgrade: bool,

    /// Behavior governing background update checks and application exit handling.
    #[arg(long = "update-behavior", value_enum)]
    pub update_behavior: Option<UpdateBehavior>,
}

/// Behavior governing background update checks and application exit handling. These are
/// the values that can be passed to [`UpgradeCLIArgs::update_behavior`].
///
/// Controls how background version check results are acted upon when the application
/// exits. See [`UpgradeCLIArgs`] for the command-line argument that configures this
/// behavior.
///
/// [`UpgradeCLIArgs`]: crate::core::script::self_update::UpgradeCLIArgs
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_eq2;

    #[derive(clap::Parser, Debug, PartialEq, Eq)]
    struct TestCLIArg {
        #[command(flatten)]
        upgrade_args: UpgradeCLIArgs,
    }

    #[test]
    fn test_upgrade_cli_args_defaults() {
        use clap::Parser;

        let parsed = TestCLIArg::try_parse_from(["test-bin"]).unwrap();
        assert_eq2!(
            parsed.upgrade_args,
            UpgradeCLIArgs {
                upgrade: false,
                update_behavior: None,
            }
        );
    }

    #[test]
    fn test_upgrade_cli_args_upgrade_flag() {
        use clap::Parser;

        let parsed = TestCLIArg::try_parse_from(["test-bin", "--upgrade"]).unwrap();
        assert_eq2!(
            parsed.upgrade_args,
            UpgradeCLIArgs {
                upgrade: true,
                update_behavior: None,
            }
        );
    }

    #[test]
    fn test_upgrade_cli_args_update_behavior_flag() {
        use clap::Parser;

        let parsed = TestCLIArg::try_parse_from([
            "test-bin",
            "--update-behavior",
            "prompt-to-upgrade",
        ])
        .unwrap();
        assert_eq2!(
            parsed.upgrade_args,
            UpgradeCLIArgs {
                upgrade: false,
                update_behavior: Some(UpdateBehavior::PromptToUpgrade),
            }
        );
    }

    #[test]
    fn test_upgrade_cli_args_conflicts_with() {
        use clap::Parser;

        let err = TestCLIArg::try_parse_from([
            "test-bin",
            "--upgrade",
            "--update-behavior",
            "auto",
        ])
        .unwrap_err();
        assert_eq2!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
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
    }
}
