// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use serde::Deserialize;
use std::{borrow::Borrow,
          fmt::{Display, Formatter},
          ops::Deref};

/// Name of a published R3BL crate (e.g., `"r3bl-cmdr"`).
///
/// This struct serves as the map key in [`JsoncManifest::crates`] and identifies the
/// target crate in [`UpgradeConfig`] when querying [`R3BL_VERSIONS_JSON_URL`]
/// or updating `versions.jsonc`.
///
/// See [`JsoncManifest`] for the complete JSONC schema specification and example.
///
/// [`JsoncManifest::crates`]: crate::self_upgrade::JsoncManifest::crates
/// [`JsoncManifest`]: crate::self_upgrade::JsoncManifest
/// [`R3BL_VERSIONS_JSON_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSON_URL
/// [`UpgradeConfig`]: crate::self_upgrade::UpgradeConfig
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(transparent)]
pub struct CrateName {
    inner: String,
}

/// Enables looking up `HashMap<CrateName, ...>` entries directly with `&str` keys
/// (e.g., `manifest.crates.get("r3bl-cmdr")`). Without this, a new [`String`] must be
/// allocated in order to look up the entry.
impl Borrow<str> for CrateName {
    fn borrow(&self) -> &str { &self.inner }
}

/// Allows [`CrateName`] to coerce to `&str` at call sites via deref coercion.
impl Deref for CrateName {
    type Target = str;

    fn deref(&self) -> &Self::Target { &self.inner }
}

/// Formats the crate name for user-facing messages and structured tracing logs.
impl Display for CrateName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

/// Constructs a [`CrateName`] from a string slice (e.g., in
/// [`UpgradeConfig::new()`]).
///
/// [`UpgradeConfig::new()`]: crate::self_upgrade::UpgradeConfig::new
impl From<&str> for CrateName {
    fn from(s: &str) -> Self {
        Self {
            inner: s.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3bl_tui::assert_eq2;

    #[test]
    fn test_crate_name_traits_and_conversions() {
        let name_from_str = CrateName::from("r3bl-cmdr");
        assert_eq2!(&*name_from_str, "r3bl-cmdr");
        assert_eq2!(name_from_str.to_string(), "r3bl-cmdr");
    }
}
