// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CheckFailed, UpToDate, UpgradeAvailable, UpgradeJourneyStateMarker,
            Upgrading};
use crate::{CrateMetadata, EndpointUrl, ManifestCheckError, UpgradeActionOnExit,
            UpgradeJourney, UpgradeStrategy};

/// Typestate while querying [`R3BL_VERSIONS_JSONC_URL`] (either synchronously via
/// [`UpgradeStrategy::Immediate`] or on the dedicated check thread via
/// [`UpgradeStrategy::DedicatedThread`]).
///
/// [`R3BL_VERSIONS_JSONC_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSONC_URL
/// [`UpgradeStrategy::DedicatedThread`]: crate::self_upgrade::UpgradeStrategy::DedicatedThread
/// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Checking {
    /// Active [`UpgradeStrategy`] (`Immediate` or
    /// `DedicatedThread(UpgradeActionOnExit)`).
    ///
    /// [`UpgradeStrategy`]: crate::self_upgrade::UpgradeStrategy
    pub strategy: UpgradeStrategy,
}
impl UpgradeJourneyStateMarker for Checking {}

impl UpgradeJourney<Checking> {
    #[must_use]
    pub(in crate::self_upgrade) fn into_up_to_date(
        self,
        metadata: CrateMetadata,
    ) -> UpgradeJourney<UpToDate> {
        UpgradeJourney {
            config: self.config,
            state: UpToDate { metadata },
        }
    }

    #[must_use]
    pub(in crate::self_upgrade) fn into_upgrade_available(
        self,
        action_on_exit: UpgradeActionOnExit,
        metadata: CrateMetadata,
        binaries_release_base_url: EndpointUrl,
    ) -> UpgradeJourney<UpgradeAvailable> {
        UpgradeJourney {
            config: self.config,
            state: UpgradeAvailable {
                action_on_exit,
                metadata,
                binaries_release_base_url,
            },
        }
    }

    #[must_use]
    pub(in crate::self_upgrade) fn into_upgrading(
        self,
        metadata: CrateMetadata,
        binaries_release_base_url: EndpointUrl,
    ) -> UpgradeJourney<Upgrading> {
        UpgradeJourney {
            config: self.config,
            state: Upgrading {
                metadata,
                binaries_release_base_url,
            },
        }
    }

    #[must_use]
    pub(in crate::self_upgrade) fn into_check_failed(
        self,
        error: ManifestCheckError,
    ) -> UpgradeJourney<CheckFailed> {
        UpgradeJourney {
            config: self.config,
            state: CheckFailed { error },
        }
    }
}
