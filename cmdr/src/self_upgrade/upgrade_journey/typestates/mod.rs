// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

// Private submodules.
mod check_failed;
mod checking;
mod completed;
mod cooldown_skipped;
mod disabled;
mod not_started;
mod up_to_date;
mod upgrade_available;
mod upgrading;

// Public barrel re-exports.
pub use check_failed::*;
pub use checking::*;
pub use completed::*;
pub use cooldown_skipped::*;
pub use disabled::*;
pub use not_started::*;
pub use up_to_date::*;
pub use upgrade_available::*;
pub use upgrading::*;

/// Marker trait implemented by all valid typestates of [`UpgradeJourney`].
///
/// [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
pub trait UpgradeJourneyStateMarker {}
