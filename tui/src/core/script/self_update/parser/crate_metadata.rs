// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::CrateVersion;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::{borrow::Borrow,
          fmt::{Display, Formatter},
          ops::Deref};

/// Type-safe wrapper for crate names (e.g. `"r3bl-cmdr"`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(transparent)]
pub struct CrateName(pub String);

mod impl_crate_name {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl CrateName {
        /// Returns the underlying crate name as a string slice.
        #[must_use]
        pub fn as_str(&self) -> &str { &self.0 }
    }

    /// Enables looking up `HashMap<CrateName, ...>` entries directly with `&str` keys
    /// (e.g., `manifest.crates_map.get("r3bl-cmdr")`).
    impl Borrow<str> for CrateName {
        fn borrow(&self) -> &str { &self.0 }
    }

    /// Allows [`CrateName`] to coerce to `&str` at call sites via deref coercion.
    impl Deref for CrateName {
        type Target = str;
        fn deref(&self) -> &Self::Target { &self.0 }
    }

    /// Allows passing [`CrateName`] to generic APIs accepting `AsRef<str>`.
    impl AsRef<str> for CrateName {
        fn as_ref(&self) -> &str { &self.0 }
    }

    /// Formats the crate name for user-facing messages and structured tracing logs.
    impl Display for CrateName {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    /// Constructs a [`CrateName`] from a string slice (e.g., in
    /// [`SelfBinaryUpgradeConfig::new`]).
    ///
    /// [`SelfBinaryUpgradeConfig::new`]: crate::core::script::self_update::SelfBinaryUpgradeConfig::new
    impl From<&str> for CrateName {
        fn from(s: &str) -> Self { Self(s.to_string()) }
    }

    /// Constructs a [`CrateName`] from an owned [`String`].
    impl From<String> for CrateName {
        fn from(s: String) -> Self { Self(s) }
    }
}

/// Deserialized crate version entry from [`R3BL_VERSIONS_JSON_URL`]. See
/// [`JsoncManifest`] for the complete JSONC schema specification and example.
///
/// [`JsoncManifest`]: crate::core::script::self_update::JsoncManifest
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::self_update::R3BL_VERSIONS_JSON_URL
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CrateVersionMetadata {
    /// Target semantic version string pinned for the crate (e.g., `"0.0.25"`).
    pub version: CrateVersion,

    /// Distribution model of the crate: `"open-source"` (eligible for
    /// `cargo +nightly install <crate> --version <version>` source compilation fallback)
    /// or `"closed-source"`.
    pub crate_distribution: CrateDistribution,

    /// RFC 3339 / ISO 8601 `UTC` timestamp (`YYYY-MM-DDTHH:MM:SSZ`) recording when the
    /// release was published.
    pub released_on: DateTime<Utc>,
}

/// Domain enum capturing the distribution model of a crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CrateDistribution {
    OpenSource,
    ClosedSource,
}

mod impl_crate_distribution {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Formats the distribution in `kebab-case` (`"open-source"` or `"closed-source"`)
    /// for JSONC CST updates in [`JsoncManifest::try_update_crate_in_jsonc`] and logs.
    ///
    /// [`JsoncManifest::try_update_crate_in_jsonc`]: crate::core::script::self_update::JsoncManifest::try_update_crate_in_jsonc
    impl Display for CrateDistribution {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::OpenSource => write!(f, "open-source"),
                Self::ClosedSource => write!(f, "closed-source"),
            }
        }
    }
}
