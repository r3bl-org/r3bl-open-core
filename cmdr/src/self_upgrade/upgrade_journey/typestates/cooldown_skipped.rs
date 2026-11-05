// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::UpgradeJourneyStateMarker;

/// Typestate when an [`UpgradeStrategy::Deferred`] check is bypassed because the
/// `1h` `mtime` cooldown timestamp file is still active.
///
/// [`UpgradeStrategy::Deferred`]: crate::self_upgrade::UpgradeStrategy::Deferred
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CooldownSkipped;
impl UpgradeJourneyStateMarker for CooldownSkipped {}
