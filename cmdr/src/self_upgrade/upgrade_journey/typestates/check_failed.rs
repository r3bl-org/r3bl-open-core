// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::UpgradeJourneyStateMarker;
use crate::ManifestCheckError;

/// Typestate when the manifest check encountered a network, parse, or thread/runtime
/// failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckFailed {
    /// Structured [`ManifestCheckError`] describing why the check failed.
    ///
    /// [`ManifestCheckError`]: crate::self_upgrade::ManifestCheckError
    pub error: ManifestCheckError,
}
impl UpgradeJourneyStateMarker for CheckFailed {}
