// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

/// A self-upgrade policy for a binary invocation, stored in [`UpgradeConfig`].
///
/// 1. CLI binaries convert [`UpgradeCLIArgs`] into [`UpgradePolicy`] at the
///    argument-parsing boundary via [`From<UpgradeCLIArgs> for UpgradePolicy`][from].
///    Also see [`UpgradeCLIArgs` (CLI-to-Domain Mapping)][map] for how `--upgrade` and
///    `--upgrade-behavior` map to these variants.
/// 2. Programmatic callers and tests construct [`UpgradePolicy`] directly.
///
/// [`UpgradeJourney::start()`] evaluates this enum to either bypass version checks
/// completely ([`Self::Disabled`]) or check the remote version manifest using an active
/// [`UpgradeStrategy`] ([`Self::Enabled`]).
///
/// # Domain Hierarchy
///
/// ```text
/// UpgradePolicy
/// ├─ Enabled(UpgradeStrategy)
/// │  ├─ Immediate                               (immediate upgrade at startup & exit)
/// │  └─ DedicatedThread(UpgradeActionOnExit)    (background check & action on exit)
/// │     ├─ Auto (default)
/// │     ├─ PromptToUpgrade
/// │     └─ NotifyOnly
/// └─ Disabled                                   (suppressed: 0 threads, 0 I/O)
/// ```
///
/// See [Main Entry Point: `UpgradeJourney` & Typestate Lifecycle][ep] in the
/// [`self_upgrade`] module documentation.
///
/// [`self_upgrade`]: mod@crate::self_upgrade
/// [`upgrade_journey!`]: crate::upgrade_journey
/// [`UpgradeCLIArgs`]: crate::self_upgrade::UpgradeCLIArgs
/// [`UpgradeConfig`]: crate::self_upgrade::UpgradeConfig
/// [`UpgradeJourney::start()`]: crate::self_upgrade::UpgradeJourney::start
/// [`UpgradeStrategy`]: crate::self_upgrade::UpgradeStrategy
/// [ep]: mod@crate::self_upgrade#main-entry-point-upgradejourney--typestate-lifecycle
/// [from]: Self#impl-From<UpgradeCLIArgs>-for-UpgradePolicy
/// [map]: crate::self_upgrade::UpgradeCLIArgs#cli-arg-to-domain-mapping
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradePolicy {
    /// Upgrade checking is enabled with the specified [`UpgradeStrategy`].
    Enabled(UpgradeStrategy),

    /// Version checks and exit upgrade handling are suppressed for this session.
    Disabled,
}

mod impl_upgrade_policy {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Default for UpgradePolicy {
        fn default() -> Self { Self::Enabled(UpgradeStrategy::default()) }
    }
}

/// An active upgrade strategy carried inside [`UpgradePolicy::Enabled`] and
/// [`Checking`].
///
/// [`Checking`]: crate::self_upgrade::Checking
/// [`UpgradePolicy::Enabled`]: crate::self_upgrade::UpgradePolicy::Enabled
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeStrategy {
    /// Checks the remote manifest and performs the upgrade synchronously during
    /// [`UpgradeJourney::start()`][start], then exits.
    ///
    /// [start]: crate::self_upgrade::UpgradeJourney::start
    Immediate,

    /// Spawns a dedicated check thread during [`UpgradeJourney::start()`][start] (unless
    /// skipped by cooldown) and runs the [`UpgradeActionOnExit`] in
    /// [`UpgradeCheckStatus::handle_upgrade_on_exit()`][exit].
    ///
    /// [exit]: crate::self_upgrade::UpgradeCheckStatus::handle_upgrade_on_exit
    /// [start]: crate::self_upgrade::UpgradeJourney::start
    DedicatedThread(UpgradeActionOnExit),
}

mod impl_upgrade_strategy {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Default for UpgradeStrategy {
        fn default() -> Self { Self::DedicatedThread(UpgradeActionOnExit::default()) }
    }
}

/// The upgrade action executed at application exit when
/// [`UpgradeStrategy::DedicatedThread`][map] finds a newer version.
///
/// This enum is carried inside [`UpgradeStrategy::DedicatedThread`][map] during
/// [`Checking`] and unpacked into [`UpgradeAvailable`] once `remote_version >
/// local_version` is verified. Because [`UpgradePolicy::Disabled`] and
/// [`UpgradeStrategy::Immediate`][2] are handled earlier in the [`UpgradeJourney`]
/// typestate pipeline, neither is representable at application exit.
///
/// See [Main Entry Point: `UpgradeJourney` & Typestate Lifecycle][ep] in the
/// [`self_upgrade`] module documentation.
///
/// [2]: crate::self_upgrade::UpgradeStrategy::Immediate
/// [`Checking`]: crate::self_upgrade::Checking
/// [`self_upgrade`]: mod@crate::self_upgrade
/// [`UpgradeAvailable`]: crate::self_upgrade::UpgradeAvailable
/// [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
/// [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
/// [ep]: mod@crate::self_upgrade#main-entry-point-upgradejourney--typestate-lifecycle
/// [map]: crate::self_upgrade::UpgradeStrategy::DedicatedThread
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UpgradeActionOnExit {
    /// Automatically downloads and installs the upgrade at exit without prompting if a
    /// newer version is available. Default behavior for all workspace binaries.
    #[default]
    Auto,

    /// Interactive prompt asking the user (`"Yes, upgrade now"` / `"No, thanks"`) at
    /// exit if a newer version is available.
    PromptToUpgrade,

    /// Prints a one-line notification that a newer version is available without running
    /// the upgrade.
    NotifyOnly,
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3bl_tui::assert_eq2;

    #[test]
    fn test_upgrade_policy_and_strategy_defaults() {
        assert_eq2!(
            UpgradeStrategy::default(),
            UpgradeStrategy::DedicatedThread(UpgradeActionOnExit::Auto)
        );
        assert_eq2!(
            UpgradePolicy::default(),
            UpgradePolicy::Enabled(UpgradeStrategy::DedicatedThread(
                UpgradeActionOnExit::Auto
            ))
        );
    }
}
