// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CrateDistribution, CrateReleaseDate, CrateVersion};
use serde::Deserialize;

/// Release and distribution metadata for a published R3BL crate.
///
/// This struct represents a crate entry parsed from [`R3BL_VERSIONS_JSON_URL`]. The
/// self-update engine uses it to check the pinned release version, determine the
/// distribution model (open-source vs. closed-source fallback), and track the release
/// timestamp.
///
/// See [`JsoncManifest`] for the complete JSONC schema specification and example.
///
/// [`JsoncManifest`]: crate::self_upgrade::JsoncManifest
/// [`R3BL_VERSIONS_JSON_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSON_URL
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CrateMetadata {
    /// Target semantic version string pinned for the crate (e.g., `"0.0.25"`).
    pub version: CrateVersion,

    /// Distribution model of the crate: `"open-source"` (eligible for
    /// `cargo +nightly install <crate> --version <version>` source compilation fallback)
    /// or `"closed-source"`.
    pub crate_distribution: CrateDistribution,

    /// RFC 3339 / ISO 8601 UTC timestamp (`YYYY-MM-DDTHH:MM:SSZ`) recording when the
    /// release was published.
    pub released_on: CrateReleaseDate,
}
