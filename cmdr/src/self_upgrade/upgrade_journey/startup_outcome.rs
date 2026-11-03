// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::process::{ExitCode, Termination};

/// Outcome of an explicit `--upgrade` ([`UpgradeStrategy::Immediate`]) run inside
/// [`StartupOutcome::UpgradedAndExit`].
///
/// Uses a `Copy` domain enum instead of `miette::Result<()>` so [`UserFeedbackChannel`]
/// remains the single output emitter (preventing `miette`'s `Termination` handler from
/// printing duplicate `Error:` diagnostics on [`stderr`] or leaking output when
/// [`UserFeedbackChannel::Quiet`] is configured).
///
/// [`stderr`]: std::io::stderr
/// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
/// [`UserFeedbackChannel::Quiet`]: crate::self_upgrade::UserFeedbackChannel::Quiet
/// [`UserFeedbackChannel`]: crate::self_upgrade::UserFeedbackChannel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImmediateUpgradeResult {
    /// Binary was already up to date or upgraded cleanly (`exit 0`).
    Succeeded,

    /// Manifest check or binary installation failed (`exit 1`); user-facing retry
    /// feedback has already been emitted via [`UserFeedbackChannel`].
    ///
    /// [`UserFeedbackChannel`]: crate::self_upgrade::UserFeedbackChannel
    Failed,
}

impl ImmediateUpgradeResult {
    /// Maps this result to the corresponding process [`ExitCode`] (`0` for
    /// [`Self::Succeeded`], `1` for [`Self::Failed`]).
    pub const fn exit_code(self) -> ExitCode {
        match self {
            Self::Succeeded => ExitCode::SUCCESS,
            Self::Failed => ExitCode::FAILURE,
        }
    }
}

impl Termination for ImmediateUpgradeResult {
    fn report(self) -> ExitCode { self.exit_code() }
}

/// Outcome returned by [`UpgradeJourney::<NotStarted>::start`][start], telling the
/// binary's `main()` whether `--upgrade` executed immediately, normal application
/// execution should continue, or `start()` was already called earlier in this process.
///
/// [start]: crate::self_upgrade::UpgradeJourney::start
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupOutcome {
    /// `--upgrade` ([`UpgradeStrategy::Immediate`]) executed; `main()` should exit with
    /// [`ImmediateUpgradeResult::exit_code`] (`0` on [`Succeeded`], `1` on [`Failed`]).
    ///
    /// [`Failed`]: ImmediateUpgradeResult::Failed
    /// [`Succeeded`]: ImmediateUpgradeResult::Succeeded
    /// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
    UpgradedAndExit(ImmediateUpgradeResult),

    /// Normal startup ([`UpgradeStrategy::DedicatedThread`] or
    /// [`UpgradePolicy::Disabled`]); `main()` should continue running the application.
    ///
    /// [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
    /// [`UpgradeStrategy::DedicatedThread`]: crate::self_upgrade::UpgradeStrategy::DedicatedThread
    ContinueApplication,

    /// [`UpgradeJourney::start`] was already called earlier in this process; the
    /// duplicate call spawned zero threads and left [`UpgradeCheckStatus`] unchanged.
    ///
    /// [`UpgradeCheckStatus`]: crate::self_upgrade::UpgradeCheckStatus
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    AlreadyStarted,
}
