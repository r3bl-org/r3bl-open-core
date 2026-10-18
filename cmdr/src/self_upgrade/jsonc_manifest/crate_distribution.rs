// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::crate_distribution_values::{CLOSED_SOURCE, OPEN_SOURCE},
            JsoncScalar};
use jsonc_parser::cst::CstInputValue;
use std::fmt::{Display, Formatter};

/// Distribution model of a published R3BL crate, stored in
/// [`CrateMetadata::crate_distribution`][2].
///
/// Parses from `kebab-case` (`"open-source"` or `"closed-source"`) via
/// [`JsoncScalar::try_parse()`], converts back into a JSONC CST node via
/// [`JsoncScalar::cst_value()`] for [`JsoncManifest::try_update_crate_in_jsonc()`][1],
/// and formats via [`Display`] for UI and logs.
///
/// See [`CrateMetadata`] and [`JsoncManifest`] for the complete JSONC schema
/// specification and example. Here's a snippet of JSONC that is deserialized into this
/// struct:
/// ```jsonc
/// "crates": {
///     // Interactive CLI suite (`giti`, `edi`, `env-source`).
///     "r3bl-cmdr": {
///       ...
///       "crate_distribution": "open-source",
///       ...
///     },
/// ```
///
/// [1]: crate::self_upgrade::JsoncManifest::try_update_crate_in_jsonc
/// [2]: crate::self_upgrade::CrateMetadata::crate_distribution
/// [`CrateMetadata`]: crate::self_upgrade::CrateMetadata
/// [`Display`]: std::fmt::Display
/// [`JsoncManifest`]: crate::self_upgrade::JsoncManifest
/// [`JsoncScalar::cst_value()`]: crate::self_upgrade::JsoncScalar::cst_value
/// [`JsoncScalar::try_parse()`]: crate::self_upgrade::JsoncScalar::try_parse
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrateDistribution {
    /// Published on `crates.io`; falls back to compiling from source via
    /// `cargo +nightly install <crate> --version <version>` when a pre-built release
    /// binary is unavailable.
    OpenSource,

    /// Distributed exclusively via pre-built release binaries.
    ClosedSource,
}

/// Failure reason returned by [`CrateDistribution::try_parse`].
///
/// [`CrateDistribution::try_parse`]: JsoncScalar::try_parse
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, miette::Diagnostic)]
#[error(
    "Unknown crate distribution '{input}': expected '{OPEN_SOURCE}' or '{CLOSED_SOURCE}'"
)]
pub struct CrateDistributionParseError {
    /// Trimmed input string that did not match a valid distribution model.
    pub input: String,
}

/// Parses an `"open-source"` or `"closed-source"` string slice (from `versions.jsonc` or
/// tests) into a [`CrateDistribution`], and converts back into a [`CstInputValue`].
impl JsoncScalar for CrateDistribution {
    type ParseError = CrateDistributionParseError;

    fn try_parse(distribution_str: &str) -> Result<Self, Self::ParseError> {
        let trimmed = distribution_str.trim_ascii();
        match trimmed {
            OPEN_SOURCE => Ok(Self::OpenSource),
            CLOSED_SOURCE => Ok(Self::ClosedSource),
            _ => Err(CrateDistributionParseError {
                input: trimmed.to_string(),
            }),
        }
    }

    fn cst_value(&self) -> CstInputValue {
        match self {
            Self::OpenSource => OPEN_SOURCE.into(),
            Self::ClosedSource => CLOSED_SOURCE.into(),
        }
    }
}

/// Formats the distribution model (`"open-source"` or `"closed-source"`) for UI error
/// messages and logs.
impl Display for CrateDistribution {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenSource => write!(f, "{OPEN_SOURCE}"),
            Self::ClosedSource => write!(f, "{CLOSED_SOURCE}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3bl_tui::assert_eq2;

    #[test]
    fn test_crate_distribution_try_parse_str_and_display() {
        assert_eq2!(
            CrateDistribution::try_parse("open-source"),
            Ok(CrateDistribution::OpenSource)
        );
        assert_eq2!(
            CrateDistribution::try_parse("  closed-source\n"),
            Ok(CrateDistribution::ClosedSource)
        );
        assert_eq2!(CrateDistribution::OpenSource.to_string(), "open-source");
        assert_eq2!(CrateDistribution::ClosedSource.to_string(), "closed-source");

        for invalid in ["", "invalid", "OpenSource", "open_source"] {
            assert_eq2!(
                CrateDistribution::try_parse(invalid),
                Err(CrateDistributionParseError {
                    input: invalid.to_string()
                })
            );
        }
    }
}
