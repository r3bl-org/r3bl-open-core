// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::versions_manifest_keys, CrateMetadata, CrateName};
use jsonc_parser::{ParseOptions, cst::CstRootNode};
use miette::IntoDiagnostic;
use r3bl_tui::{HashMap, ok};
use serde::Deserialize;

/// Schema for the centralized version manifest hosted at [`R3BL_VERSIONS_JSON_URL`].
///
/// See the [module-level documentation] for how the version manifest fits into the
/// overall upgrade lifecycle (discovery, version guarding, and open-source vs.
/// closed-source routing).
///
/// # Dual-Consumer Workflow
///
/// This struct serves two distinct consumers in the codebase:
/// - **Binary Clients (Read Path)**: Deserialized into [`JsoncManifest`] via
///   [`Self::try_parse_jsonc()`] to inspect available versions.
/// - **Release Automation (Write Path)**: Mutated programmatically via
///   [`Self::try_update_crate_in_jsonc()`], updating versions and timestamps losslessly
///   without erasing comments, blank lines, or formatting.
///
/// # Why JSONC (`versions.jsonc`)?
///
/// The manifest uses **JSON with Comments (JSONC)** parsed via [`jsonc_parser`]:
/// - **Human-readable documentation in-file**: Supports `//` line comments, `/* ... */`
///   block comments, and trailing commas directly inside `versions.jsonc`.
/// - **Lossless comment-preserving updates**: When release tooling updates a crate's
///   version and release timestamp via [`Self::try_update_crate_in_jsonc()`],
///   [`CstRootNode`] mutates only the target AST nodes in-place, preserving 100% of
///   existing comments, blank lines, and formatting.
///
/// # Example `versions.jsonc`
///
/// ```jsonc
/// // R3BL Centralized Crate Version Manifest
/// // Schema: `r3bl_cmdr::self_upgrade::JsoncManifest`
/// {
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
/// # Forward Compatibility & Schema Evolution
///
/// Neither [`JsoncManifest`] nor [`CrateMetadata`] uses
/// `#[serde(deny_unknown_fields)]`, so we can evolve `versions.jsonc` in the future
/// without breaking older binaries that users already have installed:
/// - **Adding new fields**: We can add new fields anywhere in `versions.jsonc`, and older
///   binaries will simply ignore the fields they don't know about.
/// - **Changing the schema**: `"crates"` is `V1`. If we ever need a completely new
///   format, we can add a new `"crates_v2"` section next to `"crates"` in
///   `versions.jsonc`. Older binaries will keep reading `"crates"` to upgrade themselves,
///   while newer binaries read `"crates_v2"`.
///
/// [`CstRootNode`]: jsonc_parser::cst::CstRootNode
/// [`R3BL_VERSIONS_JSON_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSON_URL
/// [module-level documentation]: mod@crate::self_upgrade
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct JsoncManifest {
    /// Map of crate names (e.g., `"r3bl-cmdr"`) to their [`CrateMetadata`].
    pub crates: HashMap<CrateName, CrateMetadata>,
}

impl JsoncManifest {
    /// Parses a JSONC string (supporting `//` line comments, `/* ... */` block
    /// comments, and trailing commas) into a [`JsoncManifest`].
    ///
    /// Unknown top-level and per-crate fields are ignored so future additive schema
    /// extensions never break older binaries in the wild.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSONC syntax is invalid or if required fields do not
    /// match the [`JsoncManifest`] schema.
    pub fn try_parse_jsonc(jsonc_str: &str) -> miette::Result<Self> {
        let value = jsonc_parser::parse_to_serde_value::<serde_json::Value>(
            jsonc_str,
            &ParseOptions::default(),
        )
        .into_diagnostic()?;

        serde_json::from_value::<Self>(value).into_diagnostic()
    }

    /// Updates an existing crate's [`CrateMetadata`] in-place inside a JSONC
    /// document string using [`jsonc_parser`]'s Concrete Syntax Tree (CST,
    /// [`CstRootNode`]).
    ///
    /// # Workflow
    ///
    /// New crate entries in `versions.jsonc` are initially authored by hand (with
    /// their descriptive comments and formatting). When publishing a new release of
    /// an existing crate, release tooling calls this function to update that crate's
    /// data in place (while preserving all existing comments, blank lines, and
    /// formatting) by updating these values:
    /// 1. `"version"`.
    /// 2. `"crate_distribution"`.
    /// 3. `"released_on"`.
    ///
    /// # Returns
    ///
    /// Returns the complete, updated JSONC document as an owned [`String`].
    ///
    /// # Errors
    ///
    /// Returns an error if `jsonc_str` is not valid JSONC or does not contain an
    /// existing valid entry for `crate_name`.
    ///
    /// [`CstRootNode`]: jsonc_parser::cst::CstRootNode
    pub fn try_update_crate_in_jsonc(
        jsonc_str: &str,
        crate_name: &CrateName,
        metadata: &CrateMetadata,
    ) -> miette::Result<String> {
        use versions_manifest_keys::{CRATE_DISTRIBUTION, CRATES, RELEASED_ON, VERSION};

        // Validate that `jsonc_str` conforms to the `JsoncManifest` schema.
        let _unused = Self::try_parse_jsonc(jsonc_str)?;

        // Parse the JSONC string into a Concrete Syntax Tree (CST) for
        // comment-preserving in-place mutation.
        let cst_root_node =
            CstRootNode::parse(jsonc_str, &ParseOptions::default()).into_diagnostic()?;

        // Create this closure so that we can use `?` to return early.
        let maybe_entry_updated_fn = || -> Option<()> {
            // Get `root_object` from CST root node.
            let root_object = cst_root_node.object_value()?;

            // Navigate to the `root_object["crates"][crate_name]` child object.
            let crate_entry =
                root_object.object_value(CRATES)?.object_value(crate_name)?;

            // Update the `root_object["crates"][crate_name]["version"]` prop value.
            let version_prop = crate_entry.get(VERSION)?;
            version_prop.set_value(metadata.version.to_string().into());

            // Update the `root_object["crates"][crate_name]["crate_distribution"]`
            // prop value.
            let distribution_prop = crate_entry.get(CRATE_DISTRIBUTION)?;
            distribution_prop.set_value(metadata.crate_distribution.to_string().into());

            // Update the `root_object["crates"][crate_name]["released_on"]` prop
            // value.
            let released_on_prop = crate_entry.get(RELEASED_ON)?;
            released_on_prop.set_value(metadata.released_on.to_string().into());

            Some(()) // Means success!
        };

        // If the closure returns `None`, it means the crate was not found, so we
        // bail.
        if maybe_entry_updated_fn().is_none() {
            miette::bail!("Crate '{crate_name}' not found in versions.jsonc");
        }

        // Convert CST back to a formatted String, preserving all comments and
        // formatting.
        ok!(cst_root_node.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{super::{CrateDistribution, CrateVersion},
                *};
    use r3bl_tui::{HashMap, assert_eq2};

    #[track_caller]
    fn assert_str_diff_eq(actual: &str, expected: &str) {
        if actual != expected {
            let diff = similar::TextDiff::from_lines(expected, actual);
            let unified_diff = diff
                .unified_diff()
                .context_radius(3)
                .header("expected", "actual")
                .to_string();
            eprintln!("=== DIFF (expected vs actual) ===\n{unified_diff}");
            panic!("Actual string output does not match expected string output");
        }
    }

    #[test]
    fn test_jsonc_manifest_deserialization_with_jsonc_comments() {
        let jsonc = r#"{
            // R3BL version manifest (JSONC with comments & trailing commas)
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

        let manifest = JsoncManifest::try_parse_jsonc(jsonc)
            .expect("Failed to parse JSONC manifest");

        assert_eq2!(
            manifest,
            JsoncManifest {
                crates: HashMap::from_iter([
                    (
                        "r3bl-cmdr".into(),
                        CrateMetadata {
                            version: CrateVersion::try_from("0.0.25").unwrap(),
                            crate_distribution: CrateDistribution::OpenSource,
                            released_on: "2026-10-17T00:00:00Z".parse().unwrap(),
                        },
                    ),
                    (
                        "r3bl-closed-source-product".into(),
                        CrateMetadata {
                            version: CrateVersion::try_from("1.2.0").unwrap(),
                            crate_distribution: CrateDistribution::ClosedSource,
                            released_on: "2026-10-17T12:00:00Z".parse().unwrap(),
                        },
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_jsonc_manifest_ignores_unknown_future_fields() {
        let future_jsonc = r#"{
            "future_top_level_field": "ignored-by-older-binaries",
            "crates": {
                "r3bl-cmdr": {
                    "version": "0.0.25",
                    "crate_distribution": "open-source",
                    "released_on": "2026-10-17T00:00:00Z",
                    "future_crate_field": { "min_glibc": "2.31" }
                }
            }
        }"#;
        let manifest = JsoncManifest::try_parse_jsonc(future_jsonc).expect(
            "Expected unknown future fields to be ignored for forward compatibility",
        );

        assert_eq2!(
            manifest,
            JsoncManifest {
                crates: HashMap::from_iter([(
                    "r3bl-cmdr".into(),
                    CrateMetadata {
                        version: CrateVersion::try_from("0.0.25").unwrap(),
                        crate_distribution: CrateDistribution::OpenSource,
                        released_on: "2026-10-17T00:00:00Z".parse().unwrap(),
                    },
                )]),
            }
        );
    }

    #[test]
    fn test_jsonc_manifest_cst_update_preserves_comments() {
        let original_jsonc = r#"
        {
            // Top-level header comment that MUST be preserved
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

        let updated_meta = CrateMetadata {
            version: CrateVersion::try_from("0.0.26").unwrap(),
            crate_distribution: CrateDistribution::OpenSource,
            released_on: "2026-10-18T15:30:00Z".parse().unwrap(),
        };

        let updated_jsonc = JsoncManifest::try_update_crate_in_jsonc(
            original_jsonc,
            &"r3bl-cmdr".into(),
            &updated_meta,
        )
        .expect("CST update failed");

        let expected_jsonc = r#"
        {
            // Top-level header comment that MUST be preserved
            "crates": {
                // Comment on r3bl-cmdr that MUST be preserved
                "r3bl-cmdr": {
                "version": "0.0.26",
                "crate_distribution": "open-source",
                "released_on": "2026-10-18T15:30:00Z"
                }
            }
        }
        "#;

        assert_str_diff_eq(&updated_jsonc, expected_jsonc);
    }

    #[test]
    fn test_jsonc_manifest_update_crate_not_found() {
        let jsonc = r#"{ "crates": {} }"#;
        let meta = CrateMetadata {
            version: CrateVersion::try_from("0.0.1").unwrap(),
            crate_distribution: CrateDistribution::OpenSource,
            released_on: "2026-10-18T00:00:00Z".parse().unwrap(),
        };
        let result =
            JsoncManifest::try_update_crate_in_jsonc(jsonc, &"missing-crate".into(), &meta);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing-crate"));
    }

    #[test]
    fn test_jsonc_manifest_parse_invalid_syntax() {
        let bad_jsonc = r#"{ "crates": "#;
        assert!(JsoncManifest::try_parse_jsonc(bad_jsonc).is_err());
    }
}
