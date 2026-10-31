// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use chrono::{DateTime, SecondsFormat, Utc};
use r3bl_tui::ok;
use serde::Deserialize;
use std::{fmt::{Display, Formatter},
          ops::Deref,
          str::FromStr};

/// Release timestamp of a published R3BL crate (UTC RFC 3339 / ISO 8601, e.g.,
/// `"2026-10-17T00:00:00Z"`).
///
/// This struct is stored in [`CrateMetadata::released_on`] and consolidates all
/// [`chrono`] date/time parsing, comparison, and formatting operations for
/// `versions.jsonc` in one place so callers do not need to depend on [`chrono`] types or
/// formatting flags directly.
///
/// See [`CrateMetadata`] and [`JsoncManifest`] for the complete JSONC schema
/// specification and example.
///
/// [`CrateMetadata::released_on`]: crate::self_upgrade::CrateMetadata::released_on
/// [`CrateMetadata`]: crate::self_upgrade::CrateMetadata
/// [`JsoncManifest`]: crate::self_upgrade::JsoncManifest
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(transparent)]
pub struct CrateReleaseDate {
    inner: DateTime<Utc>,
}

impl CrateReleaseDate {
    /// Returns the current UTC timestamp as a [`CrateReleaseDate`].
    #[must_use]
    pub fn now() -> Self { Self { inner: Utc::now() } }
}

/// Allows [`CrateReleaseDate`] to coerce to `&DateTime<Utc>` at call sites via deref
/// coercion.
impl Deref for CrateReleaseDate {
    type Target = DateTime<Utc>;
    
    fn deref(&self) -> &Self::Target { &self.inner }
}

/// Formats the timestamp as an RFC 3339 / ISO 8601 UTC string with second precision
/// and a trailing `'Z'` (`YYYY-MM-DDTHH:MM:SSZ`, e.g., `"2026-10-17T00:00:00Z"`) for
/// JSONC CST updates in [`JsoncManifest::try_update_crate_in_jsonc()`] and logs.
///
/// [`JsoncManifest::try_update_crate_in_jsonc()`]: crate::self_upgrade::JsoncManifest::try_update_crate_in_jsonc
impl Display for CrateReleaseDate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.inner.to_rfc3339_opts(SecondsFormat::Secs, true)
        )
    }
}

/// Parses an RFC 3339 / ISO 8601 UTC timestamp string into a [`CrateReleaseDate`].
impl FromStr for CrateReleaseDate {
    type Err = chrono::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let date_time = s.parse()?;
        let release_date = Self { inner: date_time };
        ok!(release_date)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3bl_tui::assert_eq2;

    #[test]
    fn test_crate_release_date_parsing_and_formatting() {
        let date: CrateReleaseDate = "2026-10-17T15:30:00Z".parse().unwrap();
        assert_eq2!(date.to_string(), "2026-10-17T15:30:00Z");

        let parsed: CrateReleaseDate = "2026-10-18T00:00:00Z".parse().unwrap();
        assert!(parsed > date);

        assert!("not-a-timestamp".parse::<CrateReleaseDate>().is_err());
    }
}
