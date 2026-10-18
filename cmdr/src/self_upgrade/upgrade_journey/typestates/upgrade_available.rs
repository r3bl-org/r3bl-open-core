// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{StateMarker, Upgrading};
use crate::{CrateMetadata, EndpointUrl, UpgradeActionOnExit, UpgradeJourney};

/// Typestate when an [`UpgradeStrategy::Deferred`] check found
/// `remote_version > local_version` and awaits [`StartOutcome::handle_upgrade_on_exit`].
///
/// [`UpgradeStrategy::Immediate`] transitions directly from [`Checking`] to
/// [`Upgrading`], so `Immediate` is unrepresentable at application exit.
///
/// [`Checking`]: crate::self_upgrade::Checking
/// [`StartOutcome::handle_upgrade_on_exit`]: crate::self_upgrade::StartOutcome::handle_upgrade_on_exit
/// [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred
/// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
/// [`Upgrading`]: crate::self_upgrade::Upgrading
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeAvailable {
    /// Active upgrade action to execute at application exit.
    pub action_on_exit: UpgradeActionOnExit,

    /// Verified remote [`CrateMetadata`] where
    /// `metadata.version > config.get_current_version()`.
    ///
    /// [`CrateMetadata`]: crate::self_upgrade::CrateMetadata
    pub metadata: CrateMetadata,

    /// Base URL for downloading pre-compiled binary release archives from the manifest.
    pub binaries_release_base_url: EndpointUrl,
}
impl StateMarker for UpgradeAvailable {}

impl UpgradeJourney<UpgradeAvailable> {
    #[must_use]
    pub(in crate::self_upgrade) fn into_upgrading(self) -> UpgradeJourney<Upgrading> {
        UpgradeJourney {
            config: self.config,
            state: Upgrading {
                metadata: self.state.metadata,
                binaries_release_base_url: self.state.binaries_release_base_url,
            },
        }
    }
}
