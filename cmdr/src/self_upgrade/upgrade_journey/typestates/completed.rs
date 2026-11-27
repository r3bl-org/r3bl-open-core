// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::StateMarker;
use crate::{CrateMetadata, ThreadRunnerError, UpgradeError};

/// Control Plane failure reason recorded in [`UpgradeResolution::UpgradeFailed`] when an
/// upgrade execution or interactive exit prompt fails.
///
/// Composes [Execution Plane] upgrade errors ([`UpgradeError`]) with [Control Plane]
/// thread/runtime failures ([`ThreadRunnerError`]).
///
/// [`ThreadRunnerError`]: crate::self_upgrade::ThreadRunnerError
/// [`UpgradeError`]: crate::self_upgrade::UpgradeError
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [Execution Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum UpgradeFailedError {
    /// Download, checksum verification, binary replacement, or source build failed in
    /// the [Execution Plane].
    ///
    /// [Execution Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
    #[error(transparent)]
    #[diagnostic(transparent)]
    PerformUpgrade(UpgradeError),

    /// Scoped OS thread failed to spawn, initialize its Tokio runtime, or panicked in
    /// the [Control Plane].
    ///
    /// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
    #[error(transparent)]
    #[diagnostic(transparent)]
    ThreadRunner(ThreadRunnerError),
}

/// Terminal outcome recorded inside [`Completed`] after an immediate upgrade or exit
/// upgrade action finishes.
#[derive(Debug)]
pub enum UpgradeResolution {
    /// Binary replacement or source installation succeeded (and the `1h` cooldown
    /// timestamp file was refreshed).
    Upgraded,

    /// Download, checksum verification, swap, source compilation, or thread/runtime
    /// execution failed; carries the structured [`UpgradeFailedError`].
    ///
    /// [`UpgradeFailedError`]: crate::self_upgrade::UpgradeFailedError
    UpgradeFailed(UpgradeFailedError),

    /// User selected `"No, thanks"` or dismissed the interactive prompt.
    DeclinedByUser,

    /// [`UpgradeActionOnExit::PromptToUpgrade`] was configured, and the session was
    /// non-interactive so the modal prompt was skipped without blocking.
    ///
    /// [`UpgradeActionOnExit::PromptToUpgrade`]: crate::self_upgrade::UpgradeActionOnExit::PromptToUpgrade
    SkippedNonInteractive,

    /// [`UpgradeActionOnExit::NotifyOnly`] printed the one-line availability notice.
    ///
    /// [`UpgradeActionOnExit::NotifyOnly`]: crate::self_upgrade::UpgradeActionOnExit::NotifyOnly
    NotifiedOnly,
}

/// Terminal typestate after an immediate upgrade or exit upgrade action finishes.
#[derive(Debug)]
pub struct Completed {
    /// Remote [`CrateMetadata`] targeted by the upgrade flow.
    ///
    /// [`CrateMetadata`]: crate::self_upgrade::CrateMetadata
    pub metadata: CrateMetadata,

    /// Terminal [`UpgradeResolution`] describing how the upgrade flow concluded.
    pub resolution: UpgradeResolution,
}
impl StateMarker for Completed {}
