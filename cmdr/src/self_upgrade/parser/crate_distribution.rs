// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use serde::Deserialize;
use std::fmt::{Display, Formatter};

/// Distribution model of a published R3BL crate (`"open-source"` or `"closed-source"`).
///
/// This enum is stored in [`CrateMetadata::crate_distribution`] and determines whether
/// the self-update engine can fall back to compiling from source via
/// `cargo +nightly install` when a pre-built release binary is unavailable.
///
/// See [`CrateMetadata`] and [`JsoncManifest`] for the complete JSONC schema
/// specification and example.
///
/// [`CrateMetadata::crate_distribution`]: crate::self_upgrade::CrateMetadata::crate_distribution
/// [`CrateMetadata`]: crate::self_upgrade::CrateMetadata
/// [`JsoncManifest`]: crate::self_upgrade::JsoncManifest
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CrateDistribution {
    /// Open-source crate published on `crates.io` (`"open-source"`). Eligible for
    /// `cargo +nightly install <crate> --version <version>` source compilation fallback.
    OpenSource,

    /// Closed-source crate distributed exclusively via pre-built release binaries
    /// (`"closed-source"`).
    ClosedSource,
}

/// Formats the distribution in `kebab-case` (`"open-source"` or `"closed-source"`)
/// for JSONC CST updates in [`JsoncManifest::try_update_crate_in_jsonc()`] and logs.
///
/// [`JsoncManifest::try_update_crate_in_jsonc()`]: crate::self_upgrade::JsoncManifest::try_update_crate_in_jsonc
impl Display for CrateDistribution {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenSource => f.write_str(OPEN_SOURCE),
            Self::ClosedSource => f.write_str(CLOSED_SOURCE),
        }
    }
}

/// Kebab-case string representation of [`CrateDistribution::OpenSource`].
const OPEN_SOURCE: &str = "open-source";

/// Kebab-case string representation of [`CrateDistribution::ClosedSource`].
const CLOSED_SOURCE: &str = "closed-source";

#[cfg(test)]
mod tests {
    use super::*;
    use r3bl_tui::assert_eq2;

    #[test]
    fn test_crate_distribution_display_and_serde() {
        assert_eq2!(CrateDistribution::OpenSource.to_string(), OPEN_SOURCE);
        assert_eq2!(CrateDistribution::ClosedSource.to_string(), CLOSED_SOURCE);

        let os: CrateDistribution =
            serde_json::from_str(&format!(r#""{OPEN_SOURCE}""#)).unwrap();
        assert_eq2!(os, CrateDistribution::OpenSource);

        let cs: CrateDistribution =
            serde_json::from_str(&format!(r#""{CLOSED_SOURCE}""#)).unwrap();
        assert_eq2!(cs, CrateDistribution::ClosedSource);

        assert!(serde_json::from_str::<CrateDistribution>(r#""invalid""#).is_err());
    }
}
