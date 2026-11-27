// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::StateMarker;
use crate::CrateMetadata;

/// Typestate when the remote manifest was checked and `remote_version <= local_version`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpToDate {
    /// Remote [`CrateMetadata`] retrieved from [`R3BL_VERSIONS_JSONC_URL`].
    ///
    /// [`CrateMetadata`]: crate::self_upgrade::CrateMetadata
    /// [`R3BL_VERSIONS_JSONC_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSONC_URL
    pub metadata: CrateMetadata,
}
impl StateMarker for UpToDate {}
