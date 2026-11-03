// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::UpgradeJourneyStateMarker;
use crate::{CrateMetadata, UpgradeError};

/// Terminal outcome recorded inside [`Completed`] after an immediate upgrade or exit
/// upgrade action finishes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpgradeResolution {
    /// Binary replacement or source installation succeeded (and the `1h` cooldown
    /// timestamp file was refreshed).
    Upgraded,

    /// Download, checksum verification, swap, or source compilation failed; carries the
    /// structured [`UpgradeError`].
    ///
    /// [`UpgradeError`]: crate::self_upgrade::UpgradeError
    UpgradeFailed(UpgradeError),

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completed {
    /// Remote [`CrateMetadata`] targeted by the upgrade flow.
    ///
    /// [`CrateMetadata`]: crate::self_upgrade::CrateMetadata
    pub metadata: CrateMetadata,

    /// Terminal [`UpgradeResolution`] describing how the upgrade flow concluded.
    pub resolution: UpgradeResolution,
}
impl UpgradeJourneyStateMarker for Completed {}
