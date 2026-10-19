// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Version manifest schema and JSONC parsing/mutation for `versions.jsonc`. See
//! [`VersionsManifest`] for details. The primary read entry point is
//! [`VersionsManifest::try_parse_jsonc()`].

use super::{manifest_keys, semver};
use crate::{HashMap, ok};
use chrono::{DateTime, Utc};
use jsonc_parser::{ParseOptions, cst::CstRootNode, json, parse_to_serde_value};
use serde::Deserialize;
use std::{borrow::Borrow,
          cmp::Ordering,
          fmt::{Display, Formatter},
          ops::Deref,
          str::FromStr};

/// Schema for the centralized version manifest hosted at [`R3BL_VERSIONS_JSON_URL`].
///
/// See the [module-level documentation] for how the version manifest fits into the
/// overall upgrade lifecycle (discovery, version guarding, and open-source vs
/// closed-source routing).
///
/// # Dual-Consumer Workflow
///
/// This struct serves two distinct consumers in the codebase:
/// - **Binary Clients (Read Path)**: Deserialized into [`VersionsManifest`] via
///   [`Self::try_parse_jsonc`] to inspect available versions.
/// - **Release Automation (Write Path)**: Mutated programmatically via
///   [`Self::try_update_crate_in_jsonc`], updating versions and timestamps losslessly
///   without erasing comments, blank lines, or formatting.
///
/// # Why JSONC (`versions.jsonc`)?
///
/// The manifest uses **JSON with Comments (`JSONC`)** parsed via [`jsonc_parser`]:
/// - **Human-readable documentation in-file**: Supports `//` line comments, `/* ... */`
///   block comments, and trailing commas directly inside `versions.jsonc`.
/// - **Lossless comment-preserving updates**: When release tooling updates a crate's
///   version and release timestamp via [`Self::try_update_crate_in_jsonc`],
///   [`jsonc_parser::cst::CstRootNode`] mutates only the target `AST` nodes in-place,
///   preserving 100% of existing comments, blank lines, and formatting.
///
/// # Canonical Schema (`schema_version: 1`)
///
/// - **`schema_version`** ([`ManifestSchemaVersion`]): Supported integer schema version
///   (currently [`ManifestSchemaVersion::V1`] / `1`), validated before routing to the
///   version-specific parser in [`Self::try_parse_jsonc`].
/// - **`crates`** (`crates_map: HashMap<CrateName, CrateVersionMetadata>`): Map keyed by
///   canonical Cargo package name (e.g., `"r3bl-cmdr"`, `"r3bl-build-infra"`,
///   `"r3bl-rust-analyzer-mcp-server"`). Each entry contains:
///   - **`version`** ([`CrateVersion`]): Pinned semantic version string
///     (`"MAJOR.MINOR.PATCH"`).
///   - **`crate_distribution`** ([`CrateDistribution`]): Domain enum serialized in
///     `kebab-case` (`"open-source"` for [`CrateDistribution::OpenSource`] or
///     `"closed-source"` for [`CrateDistribution::ClosedSource`]).
///   - **`released_on`** ([`chrono::DateTime<chrono::Utc>`]): RFC 3339 / ISO 8601 `UTC`
///     timestamp (`YYYY-MM-DDTHH:MM:SSZ`) recording when the release was published.
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
///     // Closed-source product example.
///     "r3bl-closed-source-product": {
///       "version": "1.2.0",
///       "crate_distribution": "closed-source",
///       "released_on": "2026-10-17T12:00:00Z",
///     },
///   },
/// }
/// ```
///
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
/// [module-level documentation]: mod@crate::core::script::upgrade
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct VersionsManifest {
    /// Schema version of the `versions.jsonc` manifest (currently
    /// [`ManifestSchemaVersion::V1`]).
    pub schema_version: ManifestSchemaVersion,

    /// Map of crate names (e.g., `"r3bl-cmdr"`) to their pinned version metadata.
    #[serde(rename = "crates")]
    pub crates_map: HashMap<CrateName, CrateVersionMetadata>,
}

mod impl_versions_manifest {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl VersionsManifest {
        /// Primary read entry point: parses a JSONC string (supporting `//` line
        /// comments, `/* ... */` block comments, and trailing commas) into a
        /// [`VersionsManifest`].
        ///
        /// Validates [`ManifestSchemaVersion`] first via an intermediate header before
        /// dispatching to the version-specific parser.
        ///
        /// # Errors
        ///
        /// Returns an error if the JSONC syntax is invalid, if `schema_version` is
        /// unsupported, or if the document does not match the [`VersionsManifest`]
        /// schema.
        pub fn try_parse_jsonc(jsonc_str: &str) -> miette::Result<Self> {
            let value = parse_to_serde_value::<serde_json::Value>(
                jsonc_str,
                &ParseOptions::default(),
            )
            .map_err(|e| miette::miette!("Failed to parse versions.jsonc: {e}"))?;

            let header: ManifestHeader =
                serde_json::from_value(value.clone()).map_err(|e| {
                    miette::miette!("Invalid versions.jsonc schema header: {e}")
                })?;

            match header.schema_version {
                ManifestSchemaVersion::V1 => serde_json::from_value::<Self>(value)
                    .map_err(|e| {
                        miette::miette!("Failed to parse v1 versions.jsonc: {e}")
                    }),
            }
        }

        /// Updates (or inserts) a crate's [`CrateVersionMetadata`] inside an existing
        /// JSONC document string using [`jsonc_parser`]'s Concrete Syntax Tree
        /// ([`CstRootNode`]), preserving 100% of existing comments, blank lines,
        /// and formatting.
        ///
        /// # Returns
        ///
        /// Returns the complete, updated JSONC document as an owned [`String`],
        /// preserving all existing comments, blank lines, and formatting.
        ///
        /// # Errors
        ///
        /// Returns an error if `jsonc_str` cannot be parsed as valid JSONC.
        pub fn try_update_crate_in_jsonc(
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
                root_obj.append(
                    manifest_keys::SCHEMA_VERSION,
                    json!(u32::from(ManifestSchemaVersion::default())),
                );
            }

            let crates_obj = root_obj.object_value_or_set(manifest_keys::CRATES);
            let released_on_str = metadata
                .released_on
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
            let distribution_str = metadata.crate_distribution.to_string();

            let version_str = metadata.version.to_string();
            let crate_obj = crates_obj.object_value_or_set(crate_name);

            if let Some(prop) = crate_obj.get(manifest_keys::VERSION) {
                prop.set_value(json!(version_str.as_str()));
            } else {
                crate_obj.append(manifest_keys::VERSION, json!(version_str.as_str()));
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

/// Minimal header used to validate and inspect `schema_version` before routing to a
/// version-specific manifest parser.
#[derive(Deserialize)]
struct ManifestHeader {
    schema_version: ManifestSchemaVersion,
}

/// Enum representing supported schema versions of the
/// [`R3BL_VERSIONS_JSON_URL`] manifest.
///
/// Deserializes as an integer (`1` for [`Self::V1`]) via [`TryFrom<u32>`], rejecting
/// unknown or future schema versions at deserialization time.
///
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize,
)]
#[serde(try_from = "u32")]
pub enum ManifestSchemaVersion {
    /// Initial schema version (`1`).
    #[default]
    V1 = 1,
}

mod impl_manifest_schema_version {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Formats the schema version as its integer value (e.g., `"1"`).
    impl Display for ManifestSchemaVersion {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", u32::from(*self))
        }
    }

    /// Invoked by [`serde`] via `#[serde(try_from = "u32")]` during JSONC
    /// deserialization to validate the integer `schema_version` field.
    impl TryFrom<u32> for ManifestSchemaVersion {
        type Error = String;

        fn try_from(value: u32) -> Result<Self, Self::Error> {
            match value {
                1 => Ok(Self::V1),
                other => Err(format!(
                    "Unsupported versions.jsonc schema_version: {other} (supported: 1)"
                )),
            }
        }
    }

    /// Converts the enum back to its integer representation for JSONC CST updates
    /// in [`VersionsManifest::try_update_crate_in_jsonc`] and [`Display`].
    impl From<ManifestSchemaVersion> for u32 {
        fn from(version: ManifestSchemaVersion) -> Self {
            match version {
                ManifestSchemaVersion::V1 => 1,
            }
        }
    }
}

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

    /// Constructs a [`CrateName`] from a string slice (e.g., in [`UpgradeConfig::new`]).
    ///
    /// [`UpgradeConfig::new`]: crate::core::script::upgrade::UpgradeConfig::new
    impl From<&str> for CrateName {
        fn from(s: &str) -> Self { Self(s.to_string()) }
    }

    /// Constructs a [`CrateName`] from an owned [`String`].
    impl From<String> for CrateName {
        fn from(s: String) -> Self { Self(s) }
    }
}

/// Deserialized crate version entry from [`R3BL_VERSIONS_JSON_URL`]. See
/// [`VersionsManifest`] for the complete JSONC schema specification and example.
///
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
/// [`VersionsManifest`]: crate::core::script::upgrade::VersionsManifest
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CrateVersionMetadata {
    /// Target semantic version string pinned for the crate (e.g., `"0.0.25"`).
    pub version: CrateVersion,

    /// Distribution model of the crate: `"open-source"` (eligible for
    /// `cargo +nightly install <crate> --version <version>` source compilation fallback)
    /// or `"closed-source"`.
    pub crate_distribution: CrateDistribution,

    /// ISO 8601 UTC timestamp (`YYYY-MM-DDTHH:MM:SSZ`) recording when the release was
    /// published.
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
    /// for JSONC CST updates in [`VersionsManifest::try_update_crate_in_jsonc`] and logs.
    impl Display for CrateDistribution {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::OpenSource => write!(f, "open-source"),
                Self::ClosedSource => write!(f, "closed-source"),
            }
        }
    }
}

/// Semantic version of a crate (`MAJOR.MINOR.PATCH`, e.g. `"0.0.25"`).
///
/// Strict [`SemVer`] 3-component format where `major`, `minor`, and `patch` are each
/// unsigned 16-bit integers (`u16`). Pre-release suffixes (such as `"-alpha.1"`), build
/// metadata (such as `"+build"`), and tag prefixes (such as `"v0.1.0"`) are strictly
/// disallowed in `versions.jsonc`.
///
/// [`SemVer`] is used for semantic versioning of crates and comparing versions in strict
/// precedence order (`major`, then `minor`, then `patch`).
///
/// [`SemVer`]: https://semver.org/
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(try_from = "String")]
pub struct CrateVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

mod impl_crate_version {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Compares two [`CrateVersion`] values in strict [`SemVer`] precedence order:
    /// `major` first, then `minor`, then `patch`.
    ///
    /// [`SemVer`]: https://semver.org/
    impl Ord for CrateVersion {
        fn cmp(&self, other: &Self) -> Ordering {
            // Compare (major, minor, patch) in SemVer precedence order.
            (self.major, self.minor, self.patch).cmp(&(
                other.major,
                other.minor,
                other.patch,
            ))
        }
    }

    /// Delegates `<`, `<=`, `>`, `>=` comparisons to the total [`Ord`] implementation.
    impl PartialOrd for CrateVersion {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
    }

    /// Formats the version as `"MAJOR.MINOR.PATCH"` (e.g., `"0.0.25"`) for UI strings,
    /// release URLs, and JSONC CST updates.
    impl Display for CrateVersion {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}{}{}{}{}",
                self.major,
                semver::DELIMITER,
                self.minor,
                semver::DELIMITER,
                self.patch
            )
        }
    }

    /// Primary parser for [`CrateVersion`], used both directly by Rust callers
    /// (`"0.0.25".parse::<CrateVersion>()`) and indirectly by [`serde`] deserialization
    /// via [`TryFrom<String>`].
    impl FromStr for CrateVersion {
        type Err = String;

        fn from_str(version_str: &str) -> Result<Self, Self::Err> {
            let parts: Vec<&str> = version_str.trim().split(semver::DELIMITER).collect();

            match parts.as_slice() {
                [major_str, minor_str, patch_str] => {
                    let major = major_str.parse::<u16>().map_err(|_| {
                        format!("Invalid major version in '{version_str}'")
                    })?;
                    let minor = minor_str.parse::<u16>().map_err(|_| {
                        format!("Invalid minor version in '{version_str}'")
                    })?;
                    let patch = patch_str.parse::<u16>().map_err(|_| {
                        format!("Invalid patch version in '{version_str}'")
                    })?;
                    Ok(Self {
                        major,
                        minor,
                        patch,
                    })
                }
                _ => Err(format!(
                    "Invalid semantic version '{version_str}': expected 'MAJOR.MINOR.PATCH'"
                )),
            }
        }
    }

    /// Delegates to [`FromStr`].
    impl TryFrom<&str> for CrateVersion {
        type Error = String;

        fn try_from(version_str: &str) -> Result<Self, Self::Error> {
            version_str.parse()
        }
    }

    /// Invoked by [`serde`] via `#[serde(try_from = "String")]` during JSONC
    /// deserialization; delegates to [`TryFrom<&str>`].
    impl TryFrom<String> for CrateVersion {
        type Error = String;

        fn try_from(s: String) -> Result<Self, Self::Error> { Self::try_from(s.as_str()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_crate_version_parsing_and_comparison() {
        let v_0_0_9 = CrateVersion::try_from("0.0.9").unwrap();
        let v_0_0_25 = CrateVersion::try_from("0.0.25").unwrap();
        let v_0_1_0 = CrateVersion::try_from("0.1.0").unwrap();
        let v_1_2_3 = CrateVersion::try_from("1.2.3").unwrap();

        assert_eq2!(
            v_0_0_25,
            CrateVersion {
                major: 0,
                minor: 0,
                patch: 25
            }
        );
        assert_eq2!(v_0_0_25.to_string(), "0.0.25");
        assert_eq2!(
            v_1_2_3,
            CrateVersion {
                major: 1,
                minor: 2,
                patch: 3
            }
        );

        // Numerical SemVer ordering (where string comparison would fail on 0.0.9 vs
        // 0.0.25).
        assert!(v_0_0_25 > v_0_0_9);
        assert!(v_0_1_0 > v_0_0_25);
        assert!(v_1_2_3 > v_0_1_0);
        assert_eq2!(
            v_0_0_25.cmp(&CrateVersion::try_from("0.0.25").unwrap()),
            Ordering::Equal
        );

        // Invalid segment counts are rejected by slice pattern matching.
        assert!(CrateVersion::try_from("invalid").is_err());
        assert!(CrateVersion::try_from("1.2").is_err());
        assert!(CrateVersion::try_from("1.2.3.4").is_err());

        // Non-numeric major, minor, or patch segments are rejected.
        assert!(CrateVersion::try_from("x.2.3").is_err());
        assert!(CrateVersion::try_from("1.x.3").is_err());
        assert!(CrateVersion::try_from("1.2.x").is_err());

        // Strict SemVer: v-prefixes, pre-release tags, and build metadata are rejected.
        assert!(CrateVersion::try_from("v0.1.0").is_err());
        assert!(CrateVersion::try_from("1.2.3-alpha.1").is_err());
        assert!(CrateVersion::try_from("1.2.3+build.42").is_err());
        assert!(CrateVersion::try_from("1.2.3-alpha.1+build.42").is_err());

        // Malformed version in JSONC is rejected at deserialization time.
        let bad_jsonc = r#"{
            "schema_version": 1,
            "crates": {
                "r3bl-cmdr": {
                    "version": "not-a-semver",
                    "crate_distribution": "open-source",
                    "released_on": "2026-10-17T00:00:00Z"
                }
            }
        }"#;
        assert!(VersionsManifest::try_parse_jsonc(bad_jsonc).is_err());
    }

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
                // Closed-source product
                "r3bl-closed-source-product": {
                    "version": "1.2.0",
                    "crate_distribution": "closed-source",
                    "released_on": "2026-10-17T12:00:00Z"
                },
            }
        }"#;

        let manifest = VersionsManifest::try_parse_jsonc(jsonc)
            .expect("Failed to parse JSONC manifest");
        assert_eq2!(manifest.schema_version, ManifestSchemaVersion::V1);
        assert_eq2!(manifest.crates_map.len(), 2);

        let cmdr = &manifest.crates_map["r3bl-cmdr"];
        assert_eq2!(cmdr.version, CrateVersion::try_from("0.0.25").unwrap());
        assert_eq2!(cmdr.crate_distribution, CrateDistribution::OpenSource);
        assert_eq2!(cmdr.released_on.to_rfc3339(), "2026-10-17T00:00:00+00:00");

        let closed = &manifest.crates_map["r3bl-closed-source-product"];
        assert_eq2!(closed.version, CrateVersion::try_from("1.2.0").unwrap());
        assert_eq2!(closed.crate_distribution, CrateDistribution::ClosedSource);
    }

    #[test]
    fn test_versions_manifest_rejects_unsupported_schema_version() {
        let unsupported_v2 = r#"{
            "schema_version": 2,
            "crates": {}
        }"#;
        let err = VersionsManifest::try_parse_jsonc(unsupported_v2)
            .expect_err("Expected schema_version 2 to be rejected");
        assert!(
            err.to_string()
                .contains("Unsupported versions.jsonc schema_version: 2 (supported: 1)"),
            "Unexpected error message: {err}"
        );
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
            version: CrateVersion::try_from("0.0.26").unwrap(),
            crate_distribution: CrateDistribution::OpenSource,
            released_on: "2026-10-18T15:30:00Z".parse().unwrap(),
        };

        let updated_jsonc = VersionsManifest::try_update_crate_in_jsonc(
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
        let reparsed = VersionsManifest::try_parse_jsonc(&updated_jsonc).unwrap();
        assert_eq2!(
            reparsed.crates_map["r3bl-cmdr"].version,
            CrateVersion::try_from("0.0.26").unwrap()
        );
        assert_eq2!(
            reparsed.crates_map["r3bl-cmdr"].released_on.to_rfc3339(),
            "2026-10-18T15:30:00+00:00"
        );
    }
}
