// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CheckFailed, Completed, Skipped, UpToDate, UpgradeJourney};

/// Outcome returned by [`DeferredUpgradeOutcome::handle_upgrade_on_exit`][1] when
/// completing a deferred upgrade on exit ([`UpgradeStrategy::Deferred`]).
///
/// [1]: crate::self_upgrade::DeferredUpgradeOutcome::handle_upgrade_on_exit
/// [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitOutcome {
    /// Deferred check was skipped at startup because the `1h` cooldown timestamp file
    /// was still active ([`DeferredUpgradeOutcome::Skipped`][1]).
    ///
    /// [1]: crate::self_upgrade::DeferredUpgradeOutcome::Skipped
    Skipped(UpgradeJourney<Skipped>),

    /// Dedicated thread finished and found `remote <= current` (`1h` cooldown timestamp
    /// file was touched).
    UpToDate(UpgradeJourney<UpToDate>),

    /// Manifest check failed before or during the dedicated thread's execution (OS
    /// thread spawn failure, Tokio runtime initialization failure, dedicated-thread
    /// panic, network/parse error, or crate absent from manifest).
    CheckFailed(UpgradeJourney<CheckFailed>),

    /// Dedicated thread found `remote > current` ([`UpgradeAvailable`]), transitioned
    /// through [`Upgrading`], and finished [`UpgradeActionOnExit`] with a terminal
    /// [`UpgradeResolution`] inside [`Completed`].
    ///
    /// [`UpgradeActionOnExit`]: crate::self_upgrade::UpgradeActionOnExit
    /// [`UpgradeAvailable`]: crate::self_upgrade::UpgradeAvailable
    /// [`UpgradeResolution`]: crate::self_upgrade::UpgradeResolution
    /// [`Upgrading`]: crate::self_upgrade::Upgrading
    Completed(UpgradeJourney<Completed>),
}
