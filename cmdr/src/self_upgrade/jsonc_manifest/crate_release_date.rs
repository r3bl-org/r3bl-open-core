// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::JsoncScalar;
use chrono::{DateTime, SecondsFormat, Utc};
use jsonc_parser::cst::CstInputValue;
use r3bl_tui::ok;
use std::{fmt::{Display, Formatter},
          ops::Deref};

/// Release timestamp of a published R3BL crate (UTC RFC 3339 / ISO 8601, e.g.,
/// `"2026-10-17T00:00:00Z"`).
///
/// This struct is stored in [`CrateMetadata::released_on`] and consolidates all
/// [`chrono`] date/time parsing, comparison, and formatting operations for
/// `versions.jsonc` in one place so callers do not need to depend on [`chrono`] types or
/// formatting flags directly.
///
/// See [`CrateMetadata`] and [`JsoncManifest`] for the complete JSONC schema
/// specification and example. Here's a snippet of JSONC that is deserialized into this
/// struct:
/// ```jsonc
/// "crates": {
///     // Interactive CLI suite (`giti`, `edi`, `env-source`).
///     "r3bl-cmdr": {
///       ...
///       "released_on": "2026-10-17T12:00:00Z",
///       ...
///     },
/// ```
///
/// [`CrateMetadata::released_on`]: crate::self_upgrade::CrateMetadata::released_on
/// [`CrateMetadata`]: crate::self_upgrade::CrateMetadata
/// [`JsoncManifest`]: crate::self_upgrade::JsoncManifest
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CrateReleaseDate {
    inner: DateTime<Utc>,
}

impl CrateReleaseDate {
    /// Returns the current UTC timestamp as a [`CrateReleaseDate`].
    #[must_use]
    pub fn now() -> Self { Self { inner: Utc::now() } }
}

/// Parses an RFC 3339 / ISO 8601 UTC timestamp string slice (from `versions.jsonc` or
/// tests) into a [`CrateReleaseDate`].
impl JsoncScalar for CrateReleaseDate {
    type ParseError = chrono::ParseError;

    fn try_parse(date_str: &str) -> Result<Self, Self::ParseError> {
        let date_time = date_str.parse()?;
        let release_date = Self { inner: date_time };
        ok!(release_date)
    }

    fn cst_value(&self) -> CstInputValue {
        self.inner.to_rfc3339_opts(SecondsFormat::Secs, true).into()
    }
}

/// Allows [`CrateReleaseDate`] to coerce to `&DateTime<Utc>` at call sites via deref
/// coercion.
impl Deref for CrateReleaseDate {
    type Target = DateTime<Utc>;

    fn deref(&self) -> &Self::Target { &self.inner }
}

/// Formats the timestamp as an RFC 3339 / ISO 8601 UTC string with second precision
/// and a trailing `'Z'` (`YYYY-MM-DDTHH:MM:SSZ`, e.g., `"2026-10-17T00:00:00Z"`) for
/// UI messages and logs.
impl Display for CrateReleaseDate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.inner.to_rfc3339_opts(SecondsFormat::Secs, true)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3bl_tui::assert_eq2;

    #[test]
    fn test_crate_release_date_parsing_and_formatting() {
        let date = CrateReleaseDate::try_parse("2026-10-17T15:30:00Z").unwrap();
        assert_eq2!(date.to_string(), "2026-10-17T15:30:00Z");

        let parsed = CrateReleaseDate::try_parse("2026-10-18T00:00:00Z").unwrap();
        assert!(parsed > date);

        assert!(CrateReleaseDate::try_parse("not-a-timestamp").is_err());
    }
}
