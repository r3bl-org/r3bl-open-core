// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::versions_manifest_keys, CrateDistribution, CrateReleaseDate,
            CrateVersion, JsoncScalar};
use jsonc_parser::cst::CstObject;
use serde::Deserialize;

/// Release and distribution metadata for a published R3BL crate.
///
/// This struct represents a crate entry parsed from [`R3BL_VERSIONS_JSONC_URL`]. The
/// [`self_upgrade`] engine uses it to check the following information:
/// 1. pinned (current) release version,
/// 2. the distribution model (open-source vs. closed-source fallback), and
/// 3. the release timestamp.
///
/// See [`JsoncManifest`] for the complete JSONC schema specification and example. Here's
/// a snippet of JSONC that is deserialized into this struct:
/// ```jsonc
/// "crates": {
///     // Interactive CLI suite (`giti`, `edi`, `env-source`).
///     "r3bl-cmdr": {
///       "version": "0.0.25",
///       "crate_distribution": "open-source",
///       "released_on": "2026-10-17T00:00:00Z",
///     },
/// ```
///
/// [`JsoncManifest`]: crate::self_upgrade::JsoncManifest
/// [`R3BL_VERSIONS_JSONC_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSONC_URL
/// [`self_upgrade`]: crate::self_upgrade
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CrateMetadata {
    /// Target semantic version string pinned for the crate (e.g., `"0.0.25"`).
    #[serde(deserialize_with = "JsoncScalar::deserialize_str")]
    pub version: CrateVersion,

    /// Distribution model of the crate: `"open-source"` (eligible for
    /// `cargo +nightly install <crate> --version <version>` source compilation fallback)
    /// or `"closed-source"`.
    #[serde(deserialize_with = "JsoncScalar::deserialize_str")]
    pub crate_distribution: CrateDistribution,

    /// RFC 3339 / ISO 8601 UTC timestamp (`YYYY-MM-DDTHH:MM:SSZ`) recording when the
    /// release was published.
    #[serde(deserialize_with = "JsoncScalar::deserialize_str")]
    pub released_on: CrateReleaseDate,
}

impl CrateMetadata {
    /// Updates a crate entry's [`CstObject`] node in-place inside `versions.jsonc` with
    /// the scalar values from `self` (via [`JsoncScalar::cst_value()`][3]).
    ///
    /// Called by [`JsoncManifest::try_update_crate_in_jsonc()`][1] after navigating to
    /// `root_object["crates"][crate_name]`. This method mutates the three leaf properties
    /// inside that crate object while preserving all surrounding comments, blank lines,
    /// and trailing commas:
    /// ```jsonc
    /// // Interactive CLI suite (`giti`, `edi`, `env-source`).
    /// "r3bl-cmdr": {                           // root_object["crates"]["r3bl-cmdr"]
    ///   "version": "0.0.25",                   // <- version.cst_value()
    ///   "crate_distribution": "open-source",   // <- crate_distribution.cst_value()
    ///   "released_on": "2026-10-17T00:00:00Z", // <- released_on.cst_value()
    /// },
    /// ```
    ///
    /// # Returns
    ///
    /// Returns `Some(())` if all three required property keys ([`VERSION`],
    /// [`CRATE_DISTRIBUTION`][2], and [`RELEASED_ON`]) exist on `crate_object` and were
    /// updated, or `None` if any property key is missing.
    ///
    /// [1]: crate::self_upgrade::JsoncManifest::try_update_crate_in_jsonc
    /// [2]: crate::self_upgrade::versions_manifest_keys::CRATE_DISTRIBUTION
    /// [3]: crate::self_upgrade::JsoncScalar::cst_value
    /// [`RELEASED_ON`]: crate::self_upgrade::versions_manifest_keys::RELEASED_ON
    /// [`VERSION`]: crate::self_upgrade::versions_manifest_keys::VERSION
    #[must_use]
    pub fn try_update_cst_object(&self, crate_object: &CstObject) -> Option<()> {
        use versions_manifest_keys::{CRATE_DISTRIBUTION, RELEASED_ON, VERSION};

        // Update the `crate_object["version"]` prop value.
        let version_prop = crate_object.get(VERSION)?;
        version_prop.set_value(self.version.cst_value());

        // Update the `crate_object["crate_distribution"]` prop value.
        let distribution_prop = crate_object.get(CRATE_DISTRIBUTION)?;
        distribution_prop.set_value(self.crate_distribution.cst_value());

        // Update the `crate_object["released_on"]` prop value.
        let released_on_prop = crate_object.get(RELEASED_ON)?;
        released_on_prop.set_value(self.released_on.cst_value());

        Some(())
    }
}
