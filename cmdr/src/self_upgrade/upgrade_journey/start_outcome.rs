// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CheckFailed, Checking, Completed, CooldownSkipped, Disabled, UpToDate,
            UpgradeJourney};

/// Outcome returned by [`UpgradeJourney::<NotStarted>::start`][start], telling the
/// binary's `main()` whether upgrade checks are disabled, `--upgrade` executed
/// immediately, a deferred upgrade was scheduled for exit, or `start()` was already
/// called earlier in this process.
///
/// [start]: crate::self_upgrade::UpgradeJourney::start
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartOutcome {
    /// `--upgrade-behavior disable` ([`UpgradePolicy::Disabled`]): upgrade checks are
    /// disabled; `main()` discards this value and continues running the application.
    ///
    /// [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
    Disabled(UpgradeJourney<Disabled>),

    /// `--upgrade` ([`UpgradeStrategy::Immediate`]) executed; `main()` should exit
    /// immediately without running normal application logic.
    ///
    /// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
    ImmediateUpgrade(ImmediateUpgradeOutcome),

    /// Deferred upgrade on exit ([`UpgradeStrategy::Deferred`]); `main()` discards this
    /// value and continues running the application.
    ///
    /// [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred
    DeferredUpgrade(DeferredUpgradeOutcome),

    /// [`UpgradeJourney::start`] was already called earlier in this process; the
    /// duplicate call spawned zero threads and left [`SINGLETON`] unchanged.
    ///
    /// [`SINGLETON`]: crate::self_upgrade::SINGLETON
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    AlreadyStarted,
}

/// Outcome of an explicit `--upgrade` ([`UpgradeStrategy::Immediate`]) run inside
/// [`StartOutcome::ImmediateUpgrade`].
///
/// Uses a pure domain enum instead of `miette::Result<()>` so [`UserFeedbackChannel`]
/// remains the single output emitter (preventing `miette`'s `Termination` handler from
/// printing duplicate `Error:` diagnostics on [`stderr`] or leaking output when
/// [`UserFeedbackChannel::Quiet`] is configured).
///
/// [`stderr`]: std::io::stderr
/// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
/// [`UserFeedbackChannel::Quiet`]: crate::self_upgrade::UserFeedbackChannel::Quiet
/// [`UserFeedbackChannel`]: crate::self_upgrade::UserFeedbackChannel
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImmediateUpgradeOutcome {
    /// Local binary was already up to date (`remote <= current`).
    UpToDate(UpgradeJourney<UpToDate>),

    /// Manifest check or Tokio runtime initialization failed; user-facing retry feedback
    /// has already been emitted via [`UserFeedbackChannel`].
    ///
    /// [`UserFeedbackChannel`]: crate::self_upgrade::UserFeedbackChannel
    CheckFailed(UpgradeJourney<CheckFailed>),

    /// Upgrade ran and finished with [`UpgradeResolution::Upgraded`] or
    /// [`UpgradeResolution::UpgradeFailed`].
    ///
    /// [`UpgradeResolution::Upgraded`]: crate::self_upgrade::UpgradeResolution::Upgraded
    /// [`UpgradeResolution::UpgradeFailed`]: crate::self_upgrade::UpgradeResolution::UpgradeFailed
    Completed(UpgradeJourney<Completed>),
}

/// Startup state when [`UpgradeJourney::start`] returns
/// [`StartOutcome::DeferredUpgrade`].
///
/// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeferredUpgradeOutcome {
    /// Deferred check was skipped because the `1h` cooldown timestamp file is still
    /// active.
    CooldownSkipped(UpgradeJourney<CooldownSkipped>),

    /// Dedicated thread was spawned with [`UpgradeJourney<Checking>`] and its
    /// [`JoinHandle<DeferredCheckOutcome>`][jh] was stored in [`SINGLETON`] for
    /// [`handle_upgrade_on_exit`].
    ///
    /// [`handle_upgrade_on_exit`]: crate::self_upgrade::handle_upgrade_on_exit
    /// [`SINGLETON`]: crate::self_upgrade::SINGLETON
    /// [`UpgradeJourney<Checking>`]: crate::self_upgrade::UpgradeJourney
    /// [jh]: std::thread::JoinHandle
    DeferredCheckSpawned(UpgradeJourney<Checking>),

    /// Dedicated OS thread failed to spawn; transitioned to
    /// [`UpgradeJourney<CheckFailed>`].
    ///
    /// [`UpgradeJourney<CheckFailed>`]: crate::self_upgrade::UpgradeJourney
    DeferredCheckSpawnFailed(UpgradeJourney<CheckFailed>),
}
