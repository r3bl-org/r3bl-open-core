// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Version manifest schema and JSONC parsing/mutation for `versions.jsonc`.
//!
//! [`VersionsManifest`] in this module defines the [`R3BL_VERSIONS_JSON_URL`] schema,
//! JSONC deserialization, and lossless comment-preserving CST mutation.
//!
//! [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL

use super::manifest_keys;
use crate::ok;
use chrono::{DateTime, Utc};
use jsonc_parser::{ParseOptions, cst::CstRootNode, json, parse_to_serde_value};
use serde::{Deserialize, Serialize};
use std::{borrow::Borrow,
          collections::HashMap,
          fmt::{Display, Formatter},
          ops::Deref};

/// Schema for the centralized version manifest hosted at [`R3BL_VERSIONS_JSON_URL`].
///
/// # Why JSONC (`versions.jsonc`)?
///
/// The manifest uses **JSON with Comments (JSONC)** parsed via [`jsonc_parser`]:
/// - **Human-readable documentation in-file**: Supports `//` line comments, `/* ... */`
///   block comments, and trailing commas directly inside `versions.jsonc`.
/// - **Lossless comment-preserving updates**: When `release.fish` or release tooling
///   updates a crate's version and release timestamp via
///   [`Self::update_crate_in_jsonc_str`], [`jsonc_parser::cst::CstRootNode`] mutates only
///   the target AST nodes in-place, preserving 100% of existing comments, blank lines,
///   and formatting.
///
/// # Canonical Schema (`schema_version: 1`)
///
/// - **`schema_version`** ([`ManifestSchemaVersion`]): Integer schema version (currently
///   `1`).
/// - **`crates`** (`HashMap<CrateName, CrateVersionMetadata>`): Map keyed by canonical
///   Cargo package name (e.g., `"r3bl-cmdr"`, `"r3bl-build-infra"`,
///   `"r3bl-rust-analyzer-mcp-server"`). Each entry contains:
///   - **`version`** ([`CrateVersion`]): Pinned semantic version string
///     (`"MAJOR.MINOR.PATCH"`). Compared strictly via `remote_version > local_version` so
///     local development builds with bumped `Cargo.toml` versions are never downgraded.
///     Also passed as `--version <version>` to `cargo +nightly install` during
///     open-source fallback.
///   - **`crate_distribution`** ([`CrateDistribution`]): Witness enum serialized in
///     `kebab-case`:
///     - `"open-source"` ([`CrateDistribution::OpenSource`]): Published on `crates.io`;
///       eligible for `cargo +nightly install <crate> --version <version>` fallback when
///       pre-compiled GitHub Release binaries are unreachable.
///     - `"closed-source"` ([`CrateDistribution::ClosedSource`]): Proprietary binary in
///       `r3bl-base`; downloads pre-compiled binaries exclusively from
///       `github.com/r3bl-org/r3bl-binaries` and never falls back to `crates.io`.
///   - **`released_on`** ([`chrono::DateTime<chrono::Utc>`]): RFC 3339 / ISO 8601 UTC
///     timestamp (`YYYY-MM-DDTHH:MM:SSZ`) recording when the release was published (also
///     used by commercial binaries in `r3bl-base` to verify the user's `updates_until`
///     maintenance window before upgrading).
///
/// # Example `versions.jsonc`
///
/// ```jsonc
/// // R3BL Centralized Crate Version Manifest
/// // Schema: `r3bl_tui::core::script::upgrade::VersionsManifest`
/// {
///   "schema_version": 1,
///   "crates": {
///     // Interactive CLI suite (`giti`, `edi`, `env-source`).
///     "r3bl-cmdr": {
///       "version": "0.0.25",
///       "crate_distribution": "open-source",
///       "released_on": "2026-10-17T00:00:00Z",
///     },
///     // Commercial tool example (`r3bl-base`).
///     "r3bl-closed-tool": {
///       "version": "1.2.0",
///       "crate_distribution": "closed-source",
///       "released_on": "2026-10-17T12:00:00Z",
///     },
///   },
/// }
/// ```
///
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionsManifest {
    /// Schema version of the `versions.jsonc` manifest (currently `1`).
    pub schema_version: ManifestSchemaVersion,

    /// Map of crate names (e.g., `"r3bl-cmdr"`) to their pinned version metadata.
    pub crates: HashMap<CrateName, CrateVersionMetadata>,
}

mod impl_versions_manifest {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl VersionsManifest {
        /// Parses a JSONC string (supporting `//` line comments, `/* ... */` block
        /// comments, and trailing commas) into a [`VersionsManifest`].
        ///
        /// # Errors
        ///
        /// Returns an error if the JSONC syntax is invalid or does not match the
        /// [`VersionsManifest`] schema.
        pub fn from_jsonc_str(jsonc_str: &str) -> miette::Result<Self> {
            parse_to_serde_value::<Self>(jsonc_str, &ParseOptions::default())
                .map_err(|e| miette::miette!("Failed to parse versions.jsonc: {e}"))
        }

        /// Updates (or inserts) a crate's [`CrateVersionMetadata`] inside an existing
        /// JSONC document string using [`jsonc_parser`]'s Concrete Syntax Tree
        /// ([`CstRootNode`]), preserving 100% of existing comments, blank lines,
        /// and formatting.
        ///
        /// # Errors
        ///
        /// Returns an error if `jsonc_str` cannot be parsed as valid JSONC.
        pub fn update_crate_in_jsonc_str(
            jsonc_str: &str,
            crate_name: &str,
            metadata: &CrateVersionMetadata,
        ) -> miette::Result<String> {
            let root =
                CstRootNode::parse(jsonc_str, &ParseOptions::default()).map_err(|e| {
                    miette::miette!("Failed to parse versions.jsonc CST: {e}")
                })?;
            let root_obj = root.object_value_or_set();

            if root_obj.get(manifest_keys::SCHEMA_VERSION).is_none() {
                root_obj.append(manifest_keys::SCHEMA_VERSION, json!(1));
            }

            let crates_obj = root_obj.object_value_or_set(manifest_keys::CRATES);
            let released_on_str = metadata
                .released_on
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
            let distribution_str = metadata.crate_distribution.to_string();

            let crate_obj = crates_obj.object_value_or_set(crate_name);

            if let Some(prop) = crate_obj.get(manifest_keys::VERSION) {
                prop.set_value(json!(metadata.version.as_str()));
            } else {
                crate_obj
                    .append(manifest_keys::VERSION, json!(metadata.version.as_str()));
            }

            if let Some(prop) = crate_obj.get(manifest_keys::CRATE_DISTRIBUTION) {
                prop.set_value(json!(distribution_str.as_str()));
            } else {
                crate_obj.append(
                    manifest_keys::CRATE_DISTRIBUTION,
                    json!(distribution_str.as_str()),
                );
            }

            if let Some(prop) = crate_obj.get(manifest_keys::RELEASED_ON) {
                prop.set_value(json!(released_on_str.as_str()));
            } else {
                crate_obj
                    .append(manifest_keys::RELEASED_ON, json!(released_on_str.as_str()));
            }

            ok!(root.to_string())
        }
    }
}

/// Schema version of the [`R3BL_VERSIONS_JSON_URL`] manifest.
///
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct ManifestSchemaVersion(pub u32);

mod impl_manifest_schema_version {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Deref for ManifestSchemaVersion {
        type Target = u32;
        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl Display for ManifestSchemaVersion {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl From<u32> for ManifestSchemaVersion {
        fn from(v: u32) -> Self { Self(v) }
    }

    impl From<ManifestSchemaVersion> for u32 {
        fn from(v: ManifestSchemaVersion) -> Self { v.0 }
    }
}

/// Type-safe wrapper for crate names (e.g. `"r3bl-cmdr"`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CrateName(pub String);

mod impl_crate_name {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl CrateName {
        #[must_use]
        pub fn as_str(&self) -> &str { &self.0 }
    }

    impl Borrow<str> for CrateName {
        fn borrow(&self) -> &str { &self.0 }
    }

    impl Deref for CrateName {
        type Target = str;
        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl AsRef<str> for CrateName {
        fn as_ref(&self) -> &str { &self.0 }
    }

    impl Display for CrateName {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl From<&str> for CrateName {
        fn from(s: &str) -> Self { Self(s.to_string()) }
    }

    impl From<String> for CrateName {
        fn from(s: String) -> Self { Self(s) }
    }
}

/// Deserialized crate version entry from [`R3BL_VERSIONS_JSON_URL`]. See
/// [`VersionsManifest`] for the complete JSONC schema specification and example.
///
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
/// [`VersionsManifest`]: crate::core::script::upgrade::VersionsManifest
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrateVersionMetadata {
    /// Target semantic version string pinned for the crate (e.g., `"0.0.25"`).
    pub version: CrateVersion,

    /// Distribution and licensing model of the crate: `"open-source"` (eligible for
    /// `cargo +nightly install <crate> --version <version>` source compilation fallback)
    /// or `"closed-source"`.
    pub crate_distribution: CrateDistribution,

    /// ISO 8601 UTC timestamp (`YYYY-MM-DDTHH:MM:SSZ`) recording when the release was
    /// published.
    pub released_on: DateTime<Utc>,
}

/// Witness enum capturing the distribution and licensing model of a crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CrateDistribution {
    OpenSource,
    ClosedSource,
}

mod impl_crate_distribution {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Display for CrateDistribution {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::OpenSource => write!(f, "open-source"),
                Self::ClosedSource => write!(f, "closed-source"),
            }
        }
    }
}

/// Semantic version string of a crate (e.g. `"0.0.25"`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CrateVersion(pub String);

mod impl_crate_version {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl CrateVersion {
        #[must_use]
        pub fn as_str(&self) -> &str { &self.0 }
    }

    impl Borrow<str> for CrateVersion {
        fn borrow(&self) -> &str { &self.0 }
    }

    impl Deref for CrateVersion {
        type Target = str;
        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl AsRef<str> for CrateVersion {
        fn as_ref(&self) -> &str { &self.0 }
    }

    impl Display for CrateVersion {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl From<&str> for CrateVersion {
        fn from(s: &str) -> Self { Self(s.to_string()) }
    }

    impl From<String> for CrateVersion {
        fn from(s: String) -> Self { Self(s) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_versions_manifest_deserialization_with_jsonc_comments() {
        let jsonc = r#"{
            // R3BL version manifest (JSONC with comments & trailing commas)
            "schema_version": 1,
            "crates": {
                /* Open-source CLI suite */
                "r3bl-cmdr": {
                    "version": "0.0.25",
                    "crate_distribution": "open-source",
                    "released_on": "2026-10-17T00:00:00Z",
                },
                // Closed-source binary
                "r3bl-closed-tool": {
                    "version": "1.2.0",
                    "crate_distribution": "closed-source",
                    "released_on": "2026-10-17T12:00:00Z"
                },
            }
        }"#;

        let manifest = VersionsManifest::from_jsonc_str(jsonc)
            .expect("Failed to parse JSONC manifest");
        assert_eq2!(*manifest.schema_version, 1);
        assert_eq2!(manifest.crates.len(), 2);

        let cmdr = &manifest.crates["r3bl-cmdr"];
        assert_eq2!(cmdr.version.as_str(), "0.0.25");
        assert_eq2!(cmdr.crate_distribution, CrateDistribution::OpenSource);
        assert_eq2!(cmdr.released_on.to_rfc3339(), "2026-10-17T00:00:00+00:00");

        let closed = &manifest.crates["r3bl-closed-tool"];
        assert_eq2!(closed.version.as_str(), "1.2.0");
        assert_eq2!(closed.crate_distribution, CrateDistribution::ClosedSource);
    }

    #[test]
    fn test_versions_manifest_cst_update_preserves_comments() {
        let original_jsonc = r#"{
  // Top-level header comment that MUST be preserved
  "schema_version": 1,
  "crates": {
    // Comment on r3bl-cmdr that MUST be preserved
    "r3bl-cmdr": {
      "version": "0.0.25",
      "crate_distribution": "open-source",
      "released_on": "2026-10-17T00:00:00Z"
    }
  }
}
"#;

        let updated_meta = CrateVersionMetadata {
            version: CrateVersion::from("0.0.26"),
            crate_distribution: CrateDistribution::OpenSource,
            released_on: "2026-10-18T15:30:00Z".parse().unwrap(),
        };

        let updated_jsonc = VersionsManifest::update_crate_in_jsonc_str(
            original_jsonc,
            "r3bl-cmdr",
            &updated_meta,
        )
        .expect("CST update failed");

        // Verify both comments were preserved verbatim.
        assert!(
            updated_jsonc.contains("// Top-level header comment that MUST be preserved")
        );
        assert!(updated_jsonc.contains("// Comment on r3bl-cmdr that MUST be preserved"));

        // Verify the updated JSONC re-parses cleanly with the new version.
        let reparsed = VersionsManifest::from_jsonc_str(&updated_jsonc).unwrap();
        assert_eq2!(reparsed.crates["r3bl-cmdr"].version.as_str(), "0.0.26");
        assert_eq2!(
            reparsed.crates["r3bl-cmdr"].released_on.to_rfc3339(),
            "2026-10-18T15:30:00+00:00"
        );
    }
}
