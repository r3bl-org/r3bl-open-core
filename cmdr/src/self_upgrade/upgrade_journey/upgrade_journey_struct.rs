// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::UpgradeJourneyStateMarker;
use crate::UpgradeConfig;

/// State machine for the self-upgrade lifecycle using the [Typestate Pattern].
///
/// Holds [`UpgradeConfig`] and the current `State` ([`NotStarted`], [`Disabled`],
/// [`CooldownSkipped`], [`Checking`], [`UpToDate`], [`UpgradeAvailable`],
/// [`CheckFailed`], [`Upgrading`], or [`Completed`]). Because `config`, `state`, and all
/// `into_*` transition methods are module-private (`pub(in crate::self_upgrade)`),
/// external callers cannot forge state transitions; every transition consumes
/// `UpgradeJourney<PrevState>` by value and produces `UpgradeJourney<NextState>`.
///
/// See [Main Entry Point: `UpgradeJourney` & Typestate Lifecycle][ep] in the
/// [`self_upgrade`] module documentation.
///
/// [`CheckFailed`]: crate::self_upgrade::CheckFailed
/// [`Checking`]: crate::self_upgrade::Checking
/// [`Completed`]: crate::self_upgrade::Completed
/// [`CooldownSkipped`]: crate::self_upgrade::CooldownSkipped
/// [`Disabled`]: crate::self_upgrade::Disabled
/// [`NotStarted`]: crate::self_upgrade::NotStarted
/// [`R3BL_VERSIONS_JSONC_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSONC_URL
/// [`self_upgrade`]: mod@crate::self_upgrade
/// [`UpgradeAvailable`]: crate::self_upgrade::UpgradeAvailable
/// [`UpgradeConfig`]: crate::self_upgrade::UpgradeConfig
/// [`Upgrading`]: crate::self_upgrade::Upgrading
/// [`UpToDate`]: crate::self_upgrade::UpToDate
/// [ep]: mod@crate::self_upgrade#main-entry-point-upgradejourney--typestate-lifecycle
/// [Typestate Pattern]: https://stanford-cs242.github.io/f19/lectures/08-2-typestate.html
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeJourney<State: UpgradeJourneyStateMarker> {
    pub(in crate::self_upgrade) config: UpgradeConfig,
    pub(in crate::self_upgrade) state: State,
}

impl<State: UpgradeJourneyStateMarker> UpgradeJourney<State> {
    /// Returns a reference to the immutable [`UpgradeConfig`] for this journey.
    ///
    /// [`UpgradeConfig`]: crate::self_upgrade::UpgradeConfig
    #[must_use]
    pub fn get_config(&self) -> &UpgradeConfig { &self.config }

    /// Returns a reference to the current typestate payload `State`.
    #[must_use]
    pub fn get_state(&self) -> &State { &self.state }
}
