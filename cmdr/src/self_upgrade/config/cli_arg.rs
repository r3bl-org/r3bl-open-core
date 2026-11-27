// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::upgrade_cli_args, UpgradeActionOnExit, UpgradePolicy, UpgradeStrategy};
use clap::{Parser, ValueEnum};

/// [`clap`] argument adapter for `--upgrade` and `--upgrade-behavior`. We don't use this
/// struct in our domain (the types in this crate), this is only for use at the API
/// boundary with [`clap`]. This type is converted into [`UpgradePolicy`] and discarded
/// for use in our domain, using [`From<UpgradeCLIArgs>`][from].
///
/// > See [Main Entry Point: `UpgradeJourney` & Typestate Lifecycle][ep] in the
/// > [`self_upgrade`] module documentation for the complete self-upgrade architecture.
///
/// # How to use it in your binary
///
/// Binaries that use this struct should flatten it into their top-level [`clap::Parser`]
/// or [`clap::Args`] struct via `#[command(flatten)]` and pass it directly into
/// [`upgrade_journey!`] at the CLI boundary.
///
/// # CLI Arg to Domain Mapping
///
/// Both the fields of this struct are private to prevent contradictory flag combinations
/// from being constructed in Rust code, in our domain. [`UpgradeCLIArgs`] is constructed
/// exclusively by [`clap`], which enforces mutual exclusivity (`conflicts_with`) between
/// [`--upgrade`][u] and [`--upgrade-behavior`][ub] and converts cleanly into
/// [`UpgradePolicy`] (which nests [`UpgradeStrategy`] and [`UpgradeActionOnExit`]):
///
/// ```text
/// Our Domain                          │ ◄── CLI Args passed to `<bin> --upgrade` or
///                                     │     `<bin> --upgrade-behavior
///                                     │            <disable|auto|
///                                     │             notify-only|prompt-to-upgrade>`
/// ────────────────────────────────────┼──────────────────────────────────────────────
/// UpgradePolicy                       │
/// ├─ Enabled(UpgradeStrategy)         │ Immediate upgrade at startup & exit
/// │  ├─ Immediate                     │ ◄── `<bin> --upgrade`
/// │  │                                │
/// │  └─ Deferred(UpgradeActionOnExit) │ Deferred upgrade on exit
/// │     ├─ Auto (default)             │ ◄── `<bin>` or `<bin> --upgrade-behavior auto`
/// │     ├─ PromptToUpgrade            │ ◄── `<bin> --upgrade-behavior prompt-to-upgrade`
/// │     └─ NotifyOnly                 │ ◄── `<bin> --upgrade-behavior notify-only`
/// │                                   │
/// │                                   │ Suppressed: 0 threads, 0 I/O
/// └─ Disabled                         │ ◄── `<bin> --upgrade-behavior disable`
/// ```
///
/// <div class="warning">
///
/// [`UserFeedbackChannel`] is intentionally omitted from [`UpgradeCLIArgs`] and instead
/// passed directly by the binary at compile time via [`upgrade_journey!`]: Output
/// selection ([`Stdout`] vs. [`Stderr`]) is not a user-configurable preference of a
/// binary. For instance, these binaries must keep their [`stdout`] uncorrupted:
/// 1. `rust-analyzer-mcp-server` binary, speaks `JSON-RPC` over [`stdout`] and can't
///    pollute it with upgrade spinners or messages. The user can't be allowed to choose
///    to output user feedback to [`stdout`] by mistake, since this will break the MCP
///    server.
/// 2. `env-source` binary's [`stdout`] is piped into `| source` or `> .env`. So it can't
///    be polluted with upgrade spinners or messages, otherwise it will break the shell
///    script that uses it.
///
/// </div>
///
/// # Example
///
/// ```
/// use clap::Parser;
/// use r3bl_cmdr::{UpgradeCLIArgs, UpgradePolicy, UpgradeStrategy};
///
/// // In production, a binary flattens `UpgradeCLIArgs` into its `CLIArg` struct,
/// // by calling `let cli_arg = CLIArg::parse()`, then passes `cli_arg.upgrade_args`
/// // into `upgrade_journey!`.
/// // In this doctest (where there are no OS CLI args), we call `parse_from`
/// // directly on `UpgradeCLIArgs` to simulate `my-binary --upgrade`:
/// let upgrade_args = UpgradeCLIArgs::parse_from(["my-binary", "--upgrade"]);
/// assert_eq!(
///     UpgradePolicy::from(upgrade_args),
///     UpgradePolicy::Enabled(UpgradeStrategy::Immediate)
/// );
/// ```
///
/// # Why a meta-action flag and not a domain verb subcommand?
///
/// We chose `<bin> --upgrade` (flag) over `<bin> upgrade` (subcommand). Both
/// are common patterns used by CLI programs.
///
/// | CLI Arg    | Type        | Meaning      |
/// | :--------- | :---------- | :----------- |
/// | --upgrade  | flag        | meta-action  |
/// | upgrade    | subcommand  | domain verb  |
///
/// 1. A meta-action acts on the binary itself. These instruct the binary to inspect or
///    mutate its own state (e.g., `mv --help`), rather than executing its domain workload
///    (e.g., `mv src/ dest/`).
/// 2. Using a meta-action avoids positional collisions. For binaries that accept file
///    paths (such as `edi`), a subcommand would collide with a file named `upgrade` (`edi
///    upgrade`). A flag (`edi --upgrade`) avoids corrupting positional arguments.
/// 3. This meta-action allows flattening across diverse CLIs with ease.
///    [`UpgradeCLIArgs`] can be flattened via `#[command(flatten)]` into single-purpose
///    tools (`edi`, `env-source`, `rust-analyzer-mcp-server`) and subcommand-driven tools
///    (`giti`) alike, without a subcommand architecture. Adding a subcommand would force
///    single purpose tools like `edi` to introduce subcommand enums with dispatch logic
///    in its `main()` function, instead of just being able to simply use flattening.
///
/// [`clap`]: mod@clap
/// [`self_upgrade`]: mod@crate::self_upgrade
/// [`Stderr`]: crate::self_upgrade::UserFeedbackChannel::Stderr
/// [`Stdout`]: crate::self_upgrade::UserFeedbackChannel::Stdout
/// [`stdout`]: std::io::stdout
/// [`upgrade_journey!`]: crate::upgrade_journey
/// [`UpgradeActionOnExit`]: crate::self_upgrade::UpgradeActionOnExit
/// [`UpgradePolicy`]: crate::self_upgrade::UpgradePolicy
/// [`UpgradeStrategy`]: crate::self_upgrade::UpgradeStrategy
/// [`UserFeedbackChannel`]: crate::self_upgrade::UserFeedbackChannel
/// [ep]: mod@crate::self_upgrade#main-entry-point-upgradejourney--typestate-lifecycle
/// [from]: UpgradePolicy#impl-From<UpgradeCLIArgs>-for-UpgradePolicy
/// [u]: crate::self_upgrade::upgrade_cli_args::UPGRADE_ARG_LONG
/// [ub]: crate::self_upgrade::upgrade_cli_args::UPGRADE_BEHAVIOR_ARG_LONG
#[derive(Parser, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpgradeCLIArgs {
    /// Trigger an immediate self-upgrade to the latest release and exit.
    #[arg(long = upgrade_cli_args::UPGRADE_ARG_LONG, conflicts_with = "upgrade_behavior")]
    upgrade: bool,

    /// Behavior governing dedicated-thread update checks and application exit handling.
    #[arg(long = upgrade_cli_args::UPGRADE_BEHAVIOR_ARG_LONG, value_enum)]
    upgrade_behavior: Option<CliUpgradeBehavior>,
}

/// A private [`ValueEnum`] backing `--upgrade-behavior` inside [`UpgradeCLIArgs`].
///
/// [`UpgradeCLIArgs`]: crate::self_upgrade::UpgradeCLIArgs
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CliUpgradeBehavior {
    /// Automatically downloads and installs the upgrade at exit without prompting if a
    /// newer version is available. Default behavior for all workspace binaries.
    #[value(name = "auto")]
    Auto,

    /// Interactive prompt asking the user (`"Yes, upgrade now"` / `"No, thanks"`) at
    /// exit if a newer version is available.
    #[value(name = "prompt-to-upgrade")]
    PromptToUpgrade,

    /// Prints a one-line notification that a newer version is available without running
    /// the upgrade.
    #[value(name = "notify-only")]
    NotifyOnly,

    /// Suppresses dedicated-thread version checks and exit upgrade handling entirely.
    #[value(name = "disable")]
    Disable,
}

impl From<UpgradeCLIArgs> for UpgradePolicy {
    /// Convert [`UpgradeCLIArgs`] into domain model.
    fn from(args: UpgradeCLIArgs) -> Self {
        let upgrade: Option<()> = if args.upgrade { Some(()) } else { None };
        let upgrade_behavior = args.upgrade_behavior;

        match (upgrade, upgrade_behavior) {
            // --upgrade
            (Some(()), None) => Self::Enabled(UpgradeStrategy::Immediate),

            // (no flags) or --upgrade-behavior auto
            (None, None | Some(CliUpgradeBehavior::Auto)) => {
                Self::Enabled(UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto))
            }

            // --upgrade-behavior prompt-to-upgrade
            (None, Some(CliUpgradeBehavior::PromptToUpgrade)) => Self::Enabled(
                UpgradeStrategy::Deferred(UpgradeActionOnExit::PromptToUpgrade),
            ),

            // --upgrade-behavior notify-only
            (None, Some(CliUpgradeBehavior::NotifyOnly)) => {
                Self::Enabled(UpgradeStrategy::Deferred(UpgradeActionOnExit::NotifyOnly))
            }

            // --upgrade-behavior disable
            (None, Some(CliUpgradeBehavior::Disable)) => Self::Disabled,

            // Invalid: Both --upgrade and --upgrade-behavior passed in
            (Some(()), Some(_)) => unreachable!(
                "clap `conflicts_with` guarantees `--upgrade` and `--upgrade-behavior` \
                 are mutually exclusive, and private fields prevent manual construction"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use r3bl_tui::assert_eq2;

    fn parse_policy(argv: &[&str]) -> UpgradePolicy {
        let upgrade_args = UpgradeCLIArgs::try_parse_from(argv).unwrap();
        UpgradePolicy::from(upgrade_args)
    }

    #[test]
    fn test_cli_args_parse_and_convert_to_upgrade_policy() {
        assert_eq2!(
            parse_policy(&["test-bin"]),
            UpgradePolicy::Enabled(UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto))
        );
        assert_eq2!(
            parse_policy(&["test-bin", "--upgrade"]),
            UpgradePolicy::Enabled(UpgradeStrategy::Immediate)
        );
        assert_eq2!(
            parse_policy(&["test-bin", "--upgrade-behavior", "auto"]),
            UpgradePolicy::Enabled(UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto))
        );
        assert_eq2!(
            parse_policy(&["test-bin", "--upgrade-behavior", "prompt-to-upgrade"]),
            UpgradePolicy::Enabled(UpgradeStrategy::Deferred(
                UpgradeActionOnExit::PromptToUpgrade
            ))
        );
        assert_eq2!(
            parse_policy(&["test-bin", "--upgrade-behavior", "notify-only"]),
            UpgradePolicy::Enabled(UpgradeStrategy::Deferred(
                UpgradeActionOnExit::NotifyOnly
            ))
        );
        assert_eq2!(
            parse_policy(&["test-bin", "--upgrade-behavior", "disable"]),
            UpgradePolicy::Disabled
        );
    }

    #[test]
    fn test_cli_args_rejects_mutually_exclusive_flags_and_unknown_values() {
        let conflict_err = UpgradeCLIArgs::try_parse_from([
            "test-bin",
            "--upgrade",
            "--upgrade-behavior",
            "auto",
        ])
        .unwrap_err();
        assert_eq2!(
            conflict_err.kind(),
            clap::error::ErrorKind::ArgumentConflict
        );

        let invalid_val_err =
            UpgradeCLIArgs::try_parse_from(["test-bin", "--upgrade-behavior", "bogus"])
                .unwrap_err();
        assert_eq2!(invalid_val_err.kind(), clap::error::ErrorKind::InvalidValue);
    }
}
