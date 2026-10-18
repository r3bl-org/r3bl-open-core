// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Convenience macro to construct an [`UpgradeJourney`] in its initial [`NotStarted`]
/// typestate, which serves as the main entry point for the self-upgrade process.
///
/// It does the following:
/// 1. Accepts a [`UserFeedbackChannel`] and [`UpgradeCLIArgs`] which are passed to it by
///    the caller (binary crate `main()` function).
/// 2. Then it creates an [`UpgradeConfig`] from:
///    - the passed [`UserFeedbackChannel`],
///    - an [`UpgradePolicy`] converted from [`UpgradeCLIArgs`],
///    - the calling crate's compile-time package identity, and
///    - the default [`EndpointUrl`] and [`CooldownPolicy`].
/// 3. And finally returns an [`UpgradeJourney<NotStarted>`] ready to [`.start()`].
///
/// For custom overrides (in tests to pass custom URLs or [`CooldownPolicy`]), construct
/// [`UpgradeConfig::new()`] and pass it to [`UpgradeJourney::new()`] directly.
///
/// See [Main Entry Point: `UpgradeJourney` & Typestate Lifecycle][ep] in the
/// [`self_upgrade`] module documentation.
///
/// # Examples
///
/// ```
/// use clap::Parser;
/// use r3bl_cmdr::{UpgradeActionOnExit, UpgradeCLIArgs, UpgradePolicy, UpgradeStrategy,
///                 UserFeedbackChannel, upgrade_journey};
///
/// // In production, a binary flattens `UpgradeCLIArgs` into its `CLIArg` struct,
/// // by calling `let cli_arg = CLIArg::parse()`, then passes `cli_arg.upgrade_args`
/// // into `upgrade_journey!`.
/// // In this doctest (where there are no OS CLI args), we call `parse_from`
/// // directly on `UpgradeCLIArgs` to simulate what clap produces:
/// let upgrade_args =
///     UpgradeCLIArgs::parse_from(["my-binary", "--upgrade-behavior", "prompt-to-upgrade"]);
/// let journey = upgrade_journey!(UserFeedbackChannel::Stderr, upgrade_args);
/// assert_eq!(
///     journey.get_config().get_upgrade_policy(),
///     UpgradePolicy::Enabled(UpgradeStrategy::Deferred(
///         UpgradeActionOnExit::PromptToUpgrade
///     ))
/// );
/// ```
///
/// # Why provide a declarative macro instead of a constructor?
///
/// 1. **Library vs. binary crate boundary**: Because [`self_upgrade`] lives in the
///    `r3bl-cmdr` library crate and is linked by other binary crates (`r3bl-build-infra`,
///    `r3bl-rust-analyzer-mcp-server`), calling [`env!("CARGO_PKG_NAME")`][env] inside a
///    regular function in [`self_upgrade`], like [`UpgradeConfig::new()`], would expand
///    when compiling `r3bl-cmdr`, and permanently bake `"r3bl-cmdr"`'s name and version
///    into the library!
/// 2. **Call-site expansion**: A `macro_rules!` macro expands at the **invocation site**
///    inside the downstream binary crate, resolving [`env!("CARGO_PKG_NAME")`][env] and
///    `CARGO_PKG_VERSION_*` against that binary's own `Cargo.toml` without forcing
///    callers to pass noisy identity boilerplate.
/// 3. **Not available at runtime**: `CARGO_PKG_NAME` and `CARGO_PKG_VERSION_*` are only
///    set by Cargo during `cargo build`; they do not exist in [`std::env::var`] when a
///    user runs the compiled binary on their machine.
/// 4. **Compile-time version construction (infallible at runtime)**: Cargo's pre-split
///    `CARGO_PKG_VERSION_{MAJOR,MINOR,PATCH,PRE}` variables are converted inside a `const
///    { ... }` block via [`CrateVersion::from_cargo_parts`][1], so any pre-release or
///    out-of-range `Cargo.toml` version fails at compile time (`cargo check` / `cargo
///    build`) and [`upgrade_journey!`] can never panic or fail at runtime.
///
/// [1]: crate::self_upgrade::CrateVersion::from_cargo_parts
/// [`.start()`]: crate::self_upgrade::UpgradeJourney::start
/// [`CooldownPolicy`]: crate::self_upgrade::CooldownPolicy
/// [`EndpointUrl`]: crate::self_upgrade::EndpointUrl
/// [`NotStarted`]: crate::self_upgrade::NotStarted
/// [`self_upgrade`]: mod@crate::self_upgrade
/// [`upgrade_journey!`]: crate::upgrade_journey
/// [`UpgradeCLIArgs`]: crate::self_upgrade::UpgradeCLIArgs
/// [`UpgradeConfig::new()`]: crate::self_upgrade::UpgradeConfig::new
/// [`UpgradeConfig`]: crate::self_upgrade::UpgradeConfig
/// [`UpgradeJourney::new()`]: crate::self_upgrade::UpgradeJourney::new
/// [`UpgradeJourney<NotStarted>`]: crate::self_upgrade::UpgradeJourney
/// [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
/// [`UpgradePolicy`]: crate::self_upgrade::UpgradePolicy
/// [`UserFeedbackChannel`]: crate::self_upgrade::UserFeedbackChannel
/// [env]: std::env!
/// [ep]: mod@crate::self_upgrade#main-entry-point-upgradejourney--typestate-lifecycle
#[macro_export]
macro_rules! upgrade_journey {
    ($user_feedback_channel:expr, $upgrade_cli_args:expr) => {{
        let crate_name = ::core::env!("CARGO_PKG_NAME");

        // This const block ensures that from_cargo_parts is evaluated at compile
        // time; without it, this expression would evaluate at runtime.
        let current_version = const {
            $crate::CrateVersion::from_cargo_parts(
                ::core::env!("CARGO_PKG_VERSION_MAJOR"),
                ::core::env!("CARGO_PKG_VERSION_MINOR"),
                ::core::env!("CARGO_PKG_VERSION_PATCH"),
                ::core::env!("CARGO_PKG_VERSION_PRE"),
            )
        };

        let upgrade_policy = $crate::UpgradePolicy::from($upgrade_cli_args);
        let manifest_url = $crate::EndpointUrl::default_manifest_url();
        let cooldown_policy = $crate::CooldownPolicy::default();

        let config = $crate::UpgradeConfig::new(
            crate_name,
            current_version,
            $user_feedback_channel,
            upgrade_policy,
            manifest_url,
            cooldown_policy,
        );

        $crate::UpgradeJourney::<$crate::NotStarted>::new(config)
    }};
}

#[cfg(test)]
mod tests {
    use crate::{CooldownPolicy, CrateName, EndpointUrl, NotStarted, UpgradeCLIArgs,
                UpgradePolicy, UpgradeStrategy, UserFeedbackChannel};
    use r3bl_tui::assert_eq2;

    #[test]
    fn test_upgrade_journey_macro_defaults_and_policy_conversion() {
        use clap::Parser;

        let default_args = UpgradeCLIArgs::try_parse_from(["test-bin"]).unwrap();
        let immediate_args =
            UpgradeCLIArgs::try_parse_from(["test-bin", "--upgrade"]).unwrap();

        let default_journey = upgrade_journey!(UserFeedbackChannel::Quiet, default_args);
        assert_eq2!(
            default_journey.get_config().get_crate_name(),
            &CrateName::from(env!("CARGO_PKG_NAME"))
        );
        assert_eq2!(
            default_journey.get_config().get_jsonc_manifest_url(),
            &EndpointUrl::default_manifest_url()
        );
        assert_eq2!(
            default_journey.get_config().get_cooldown_policy(),
            &CooldownPolicy::default()
        );
        assert_eq2!(
            default_journey.get_config().get_upgrade_policy(),
            UpgradePolicy::default()
        );
        assert_eq2!(default_journey.get_state(), &NotStarted);

        let immediate_journey =
            upgrade_journey!(UserFeedbackChannel::Stderr, immediate_args);
        assert_eq2!(
            immediate_journey.get_config().get_upgrade_policy(),
            UpgradePolicy::Enabled(UpgradeStrategy::Immediate)
        );
    }
}
