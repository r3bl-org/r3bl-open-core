// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::StateMarker;

/// Typestate when upgrade checks are disabled via [`UpgradePolicy::Disabled`].
///
/// [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Disabled;
impl StateMarker for Disabled {}
