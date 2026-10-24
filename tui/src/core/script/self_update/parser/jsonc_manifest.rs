// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::versions_manifest_keys, CrateName, CrateVersionMetadata};
use crate::{HashMap, ok};
use jsonc_parser::{ParseOptions, cst::CstRootNode, json, parse_to_serde_value};
use miette::IntoDiagnostic;
use serde::Deserialize;

/// Schema for the centralized version manifest hosted at [`R3BL_VERSIONS_JSON_URL`].
///
/// See the [module-level documentation] for how the version manifest fits into the
/// overall upgrade lifecycle (discovery, version guarding, and open-source vs
/// closed-source routing).
///
/// # Dual-Consumer Workflow
///
/// This struct serves two distinct consumers in the codebase:
/// - **Binary Clients (Read Path)**: Deserialized into [`JsoncManifest`] via
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
/// # Example `versions.jsonc`
///
/// ```jsonc
/// // R3BL Centralized Crate Version Manifest
/// // Schema: `r3bl_tui::core::script::self_update::JsoncManifest`
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
/// Neither [`JsoncManifest`] nor [`CrateVersionMetadata`] uses
/// `#[serde(deny_unknown_fields)]`, so we can evolve `versions.jsonc` in the future
/// without breaking older binaries that users already have installed:
/// - **Adding new fields**: We can add new fields anywhere in `versions.jsonc`, and older
///   binaries will simply ignore the fields they don't know about.
/// - **Changing the schema**: `"crates"` is `V1`. If we ever need a completely new
///   format, we can add a new `"crates_v2"` section next to `"crates"` in
///   `versions.jsonc`. Older binaries will keep reading `"crates"` to upgrade themselves,
///   while newer binaries read `"crates_v2"`.
///
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::self_update::R3BL_VERSIONS_JSON_URL
/// [module-level documentation]: mod@crate::core::script::self_update
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct JsoncManifest {
    /// Map of crate names (e.g., `"r3bl-cmdr"`) to their [`CrateVersionMetadata`].
    pub crates: HashMap<CrateName, CrateVersionMetadata>,
}

mod impl_jsonc_manifest {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl JsoncManifest {
        /// Primary read entry point: parses a JSONC string (supporting `//` line
        /// comments, `/* ... */` block comments, and trailing commas) into a
        /// [`JsoncManifest`].
        ///
        /// Unknown top-level and per-crate fields are ignored so future additive schema
        /// extensions never break older binaries in the wild.
        ///
        /// # Errors
        ///
        /// Returns an error if the JSONC syntax is invalid or if required fields do not
        /// match the [`JsoncManifest`] schema.
        pub fn try_parse_jsonc(jsonc_str: &str) -> miette::Result<Self> {
            let value = parse_to_serde_value::<serde_json::Value>(
                jsonc_str,
                &ParseOptions::default(),
            )
            .into_diagnostic()?;

            serde_json::from_value::<Self>(value).into_diagnostic()
        }

        /// Primary write entry point: updates an existing crate's
        /// [`CrateVersionMetadata`] in-place inside a JSONC document string using
        /// [`jsonc_parser`]'s Concrete Syntax Tree ([`CstRootNode`]).
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
        pub fn try_update_crate_in_jsonc(
            jsonc_str: &str,
            crate_name: &CrateName,
            metadata: &CrateVersionMetadata,
        ) -> miette::Result<String> {
            use versions_manifest_keys::{CRATE_DISTRIBUTION, CRATES, RELEASED_ON,
                                         VERSION};

            // Parse the JSONC string.
            let cst_root_node = CstRootNode::parse(jsonc_str, &ParseOptions::default())
                .into_diagnostic()?;

            // Create closure to update crate entry, so that `?` can be used to exit
            // early.
            let maybe_entry_updated = || -> Option<()> {
                // Get root_object from CST.
                let root_object = cst_root_node.object_value()?;

                // Get root_object["crates"][crate_name] => crate_object.
                let crate_object =
                    root_object.object_value(CRATES)?.object_value(crate_name)?;

                // Update crate_object['version'].
                crate_object
                    .get(VERSION)?
                    .set_value(json!(metadata.version.to_string()));

                // Update crate_object['crate_distribution'].
                crate_object
                    .get(CRATE_DISTRIBUTION)?
                    .set_value(json!(metadata.crate_distribution.to_string()));

                // Update crate_object['released_on'].
                crate_object.get(RELEASED_ON)?.set_value(json!(
                    metadata
                        .released_on
                        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
                ));
                Some(())
            };

            // If the closure returns `None`, crate entry not found & error out.
            let Some(()) = maybe_entry_updated() else {
                miette::bail!(
                    "Invalid or missing '{crate_name}' entry in versions.jsonc"
                );
            };

            // Convert CST back to string.
            ok!(cst_root_node.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{super::{CrateDistribution, CrateVersion},
                *};
    use crate::assert_eq2;

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
        assert_eq2!(manifest.crates.len(), 2);

        let cmdr = &manifest.crates["r3bl-cmdr"];
        assert_eq2!(cmdr.version, CrateVersion::try_from("0.0.25").unwrap());
        assert_eq2!(cmdr.crate_distribution, CrateDistribution::OpenSource);
        assert_eq2!(cmdr.released_on.to_rfc3339(), "2026-10-17T00:00:00+00:00");

        let closed = &manifest.crates["r3bl-closed-source-product"];
        assert_eq2!(closed.version, CrateVersion::try_from("1.2.0").unwrap());
        assert_eq2!(closed.crate_distribution, CrateDistribution::ClosedSource);
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
        assert_eq2!(manifest.crates.len(), 1);
        assert_eq2!(
            manifest.crates["r3bl-cmdr"].version,
            CrateVersion::try_from("0.0.25").unwrap()
        );
    }

    #[test]
    fn test_jsonc_manifest_cst_update_preserves_comments() {
        let original_jsonc = r#"{
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

        let updated_meta = CrateVersionMetadata {
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

        // Verify both comments were preserved verbatim.
        assert!(
            updated_jsonc.contains("// Top-level header comment that MUST be preserved")
        );
        assert!(updated_jsonc.contains("// Comment on r3bl-cmdr that MUST be preserved"));

        // Verify the updated JSONC re-parses cleanly with the new version.
        let reparsed = JsoncManifest::try_parse_jsonc(&updated_jsonc).unwrap();
        assert_eq2!(
            reparsed.crates["r3bl-cmdr"].version,
            CrateVersion::try_from("0.0.26").unwrap()
        );
        assert_eq2!(
            reparsed.crates["r3bl-cmdr"].released_on.to_rfc3339(),
            "2026-10-18T15:30:00+00:00"
        );
    }
}
