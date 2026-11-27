// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::{VERSIONS_JSONC_FILE, versions_manifest_keys},
            CrateMetadata, CrateName, EndpointUrl, JsoncScalar};
use jsonc_parser::{ParseOptions, cst::CstRootNode};
use r3bl_tui::{HashMap, ok};
use serde::Deserialize;

/// Schema for the centralized version manifest hosted at [`R3BL_VERSIONS_JSONC_URL`].
///
/// See [Centralized Version Manifest)][1] in the [`self_upgrade`] module documentation
/// for how the version manifest fits into the overall upgrade lifecycle (discovery,
/// version guarding, and open-source vs. closed-source routing).
///
/// # Two Consumers - Binary Clients & Release Automation
///
/// This struct serves two distinct consumers in the codebase:
/// - **Binary Clients (Read Path)**: The JSONC payload is deserialized into
///   [`JsoncManifest`] via [`Self::try_parse_jsonc()`] to inspect available versions to
///   determine if an upgrade is available and what to do.
/// - **Release Automation (Write Path)**: Mutated programmatically via
///   [`Self::try_update_crate_in_jsonc()`] as part of the release process workflow. This
///   allows updates to versions and timestamps losslessly without erasing comments, blank
///   lines, or formatting.
///
/// # Why JSONC?
///
/// The manifest uses **JSON with Comments (JSONC)** parsed via [`jsonc_parser`]:
/// - **Human-readable documentation in-file**: Supports `//` line comments, `/* ... */`
///   block comments, and trailing commas directly inside [`VERSIONS_JSONC_FILE`].
/// - **Lossless comment-preserving updates**: When release tooling updates a crate's
///   version and release timestamp via [`Self::try_update_crate_in_jsonc()`],
///   [`CstRootNode`] mutates only the target CST nodes in-place, preserving all existing
///   comments, blank lines, and formatting.
///
/// # Example [`VERSIONS_JSONC_FILE`]
///
/// ```jsonc
/// // R3BL Centralized Crate Version Manifest
/// // Schema: `r3bl_cmdr::self_upgrade::JsoncManifest`
/// {
///   "binaries_release_base_url": "https://github.com/r3bl-org/r3bl-binaries/releases/download",
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
/// Neither of the JSON object structs ([`JsoncManifest`] and [`CrateMetadata`]) uses
/// [`#[serde(deny_unknown_fields)]`][2] so we can evolve [`VERSIONS_JSONC_FILE`] in the
/// future without breaking older binaries that users already have installed. Note that
/// the scalar fields like [`EndpointUrl`], [`CrateName`], [`CrateVersion`],
/// [`CrateDistribution`], and [`CrateReleaseDate`] deserialize directly from JSON
/// strings.
/// - **Adding new fields**: We can add new fields anywhere in [`VERSIONS_JSONC_FILE`],
///   and older binaries will simply ignore the fields they don't know about.
/// - **Changing the schema**: [`CRATES`] is `V1`. If we ever need a completely new
///   format, we can add a new `"crates_v2"` section next to [`CRATES`] in
///   [`VERSIONS_JSONC_FILE`]. Older binaries will keep reading [`CRATES`] to upgrade
///   themselves, while newer binaries read `"crates_v2"`.
///
/// # How we parse JSONC
///
/// Neither the [`serde_json`] crate nor the [`jsonc_parser`] crate alone is sufficient to
/// parse and mutate [`VERSIONS_JSONC_FILE`]. We use [`jsonc_parser`] + [`serde_json`] on
/// the read path and [`jsonc_parser`] on the write path. The [`JsoncScalar`] trait
/// formalizes this approach for scalar values.
///
/// 1. **Read Path (`jsonc_parser` + `serde::Deserialize`)**:
///    - Limitations:
///      - [`serde_json::from_str()`] cannot parse JSONC directly because it rejects `//`
///        line comments, `/* ... */` block comments, and trailing commas.
///      - [`jsonc_parser`] parses JSONC syntax into an untyped syntax tree, but has no
///        `#[derive(...)]` macro to map directly into strongly-typed Rust structs.
///    - Workaround:
///      - [`Self::try_parse_jsonc()`] bridges the two by doing the following:
///        - [`jsonc_parser::parse_to_serde_value()`] strips comments and trailing commas
///          to produce an untyped [`serde_json::Value`] tree, and then,
///        - [`serde_json::from_value()`] deserializes that tree into [`JsoncManifest`],
///          [`CrateMetadata`], and our scalar types (via
///          [`JsoncScalar::deserialize_str()`]).
/// 2. **Write Path ([`CstRootNode`] + [`JsoncScalar::cst_value()`], and no
///    [`serde::Serialize`])**:
///    - Limitations:
///      - [`JsoncManifest`] and [`CrateMetadata`] intentionally do **not** implement
///        [`serde::Serialize`]. Serializing via [`serde_json::to_string()`] would strip
///        all comments, blank lines, trailing commas, and formatting from
///        [`VERSIONS_JSONC_FILE`].
///    - Workaround:
///      - Instead, [`Self::try_update_crate_in_jsonc()`] parses the document into a
///        lossless [`CstRootNode`], converts updated scalar values into [`CstInputValue`]
///        nodes via [`JsoncScalar::cst_value()`] (preserving separation of concerns from
///        [`Display`] UI formatting), mutates only the target leaf CST nodes in-place,
///        and renders the updated document via `cst_root_node.to_string()`.
/// 3. **Scalars vs. Objects**:
///    - **Scalars like [`EndpointUrl`], [`CrateVersion`], [`CrateDistribution`],
///      [`CrateReleaseDate`]**:
///      - Implement [`JsoncScalar`] because leaf string values are validated via
///        [`JsoncScalar::deserialize_str()`] on the read path and replaced wholesale via
///        [`JsoncScalar::cst_value()`] (`prop.set_value(...)`) on the write path.
///    - **Objects like [`JsoncManifest`], [`CrateMetadata`]**:
///      - Have **no** `JsoncObject` trait or `cst_value()` method because replacing a
///        [`CstObject`] node wholesale would erase any comments or blank lines inside
///        that JSON object.
///      - Instead, they use `#[derive(Deserialize)]` on the read path, and navigate
///        existing [`CstObject`] nodes in-place on the write path, for e.g.,
///        [`Self::try_update_crate_in_jsonc()`] delegates leaf updates to
///        [`CrateMetadata::try_update_cst_object()`][3].
///
/// [1]: mod@crate::self_upgrade#centralized-version-manifest-r3bl_versions_jsonc_url
/// [2]: https://serde.rs/container-attrs.html#deny_unknown_fields
/// [3]: crate::self_upgrade::CrateMetadata::try_update_cst_object
/// [`CrateDistribution`]: crate::self_upgrade::CrateDistribution
/// [`CrateReleaseDate`]: crate::self_upgrade::CrateReleaseDate
/// [`CRATES`]: crate::self_upgrade::versions_manifest_keys::CRATES
/// [`CrateVersion`]: crate::self_upgrade::CrateVersion
/// [`CstInputValue`]: jsonc_parser::cst::CstInputValue
/// [`CstObject`]: jsonc_parser::cst::CstObject
/// [`CstRootNode`]: jsonc_parser::cst::CstRootNode
/// [`Display`]: std::fmt::Display
/// [`JsoncScalar::cst_value()`]: crate::self_upgrade::JsoncScalar::cst_value
/// [`JsoncScalar::deserialize_str()`]: crate::self_upgrade::JsoncScalar::deserialize_str
/// [`JsoncScalar`]: crate::self_upgrade::JsoncScalar
/// [`R3BL_VERSIONS_JSONC_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSONC_URL
/// [`self_upgrade`]: mod@crate::self_upgrade
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct JsoncManifest {
    /// Base URL for downloading pre-compiled binary release archives
    /// ([`BINARIES_RELEASE_BASE_URL`][1]).
    ///
    /// [1]: crate::self_upgrade::versions_manifest_keys::BINARIES_RELEASE_BASE_URL
    #[serde(deserialize_with = "JsoncScalar::deserialize_str")]
    pub binaries_release_base_url: EndpointUrl,

    /// Map of crate names (e.g., `"r3bl-cmdr"`) to their [`CrateMetadata`].
    pub crates: HashMap<CrateName, CrateMetadata>,
}

/// Error returned by [`JsoncManifest::try_parse_jsonc`] when the manifest document is
/// syntactically invalid JSONC or fails [`JsoncManifest`] schema validation.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum JsoncParseError {
    /// Invalid JSONC syntax (unterminated string/comment, unexpected token, etc.).
    #[error("Invalid JSONC syntax in manifest: {0}")]
    InvalidSyntax(#[source] jsonc_parser::errors::ParseError),

    /// Valid JSONC syntax, and missing or invalid [`JsoncManifest`] schema fields.
    #[error("Manifest does not match expected schema: {0}")]
    InvalidSchema(#[source] serde_json::Error),
}

/// Error returned by [`JsoncManifest::try_update_crate_in_jsonc`] when the existing
/// manifest fails validation, cannot be parsed into a [`CstRootNode`], or does not
/// contain an entry for the requested crate.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum JsoncUpdateError {
    /// Existing manifest failed JSONC syntax or [`JsoncManifest`] schema validation.
    #[error("Existing manifest failed validation")]
    InvalidManifest(#[source] JsoncParseError),

    /// Failed to parse the JSONC document into a Concrete Syntax Tree ([`CstRootNode`]).
    #[error("Failed to parse JSONC into CST")]
    CstParseFailed(#[source] jsonc_parser::errors::ParseError),

    /// No existing entry for `crate_name` was found in `file`.
    #[error("Crate '{crate_name}' not found in {file}")]
    CrateNotFound {
        /// Crate name looked up under `crates`.
        crate_name: String,
        /// Manifest filename where the lookup failed.
        file: &'static str,
    },
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
    /// Returns [`JsoncParseError::InvalidSyntax`] if the JSONC syntax is invalid or
    /// [`JsoncParseError::InvalidSchema`] if required fields do not match the
    /// [`JsoncManifest`] schema.
    pub fn try_parse_jsonc(jsonc_str: &str) -> Result<Self, JsoncParseError> {
        let value = jsonc_parser::parse_to_serde_value::<serde_json::Value>(
            jsonc_str,
            &ParseOptions::default(),
        )
        .map_err(JsoncParseError::InvalidSyntax)?;

        serde_json::from_value::<Self>(value).map_err(JsoncParseError::InvalidSchema)
    }

    /// Updates an existing crate's [`CrateMetadata`] in-place inside a JSONC
    /// document string using [`jsonc_parser`]'s Concrete Syntax Tree (CST,
    /// [`CstRootNode`]).
    ///
    /// # Workflow
    ///
    /// New crate entries in [`VERSIONS_JSONC_FILE`] are initially authored by hand (with
    /// their descriptive comments and formatting). When publishing a new release of
    /// an existing crate, release tooling calls this function to update that crate's
    /// data in place (while preserving all existing comments, blank lines, and
    /// formatting) by updating these values:
    /// 1. [`VERSION`].
    /// 2. [`CRATE_DISTRIBUTION`].
    /// 3. [`RELEASED_ON`].
    ///
    /// # Returns
    ///
    /// Returns the complete, updated JSONC document as an owned [`String`].
    ///
    /// # Errors
    ///
    /// Returns [`JsoncUpdateError::InvalidManifest`] if `jsonc_str` fails syntax or
    /// schema validation, [`JsoncUpdateError::CstParseFailed`] if CST parsing fails, or
    /// [`JsoncUpdateError::CrateNotFound`] if no entry exists for `crate_name`.
    ///
    /// [`CRATE_DISTRIBUTION`]: crate::self_upgrade::versions_manifest_keys::CRATE_DISTRIBUTION
    /// [`CstRootNode`]: jsonc_parser::cst::CstRootNode
    /// [`RELEASED_ON`]: crate::self_upgrade::versions_manifest_keys::RELEASED_ON
    /// [`VERSION`]: crate::self_upgrade::versions_manifest_keys::VERSION
    pub fn try_update_crate_in_jsonc(
        jsonc_str: &str,
        crate_name: &CrateName,
        metadata: &CrateMetadata,
    ) -> Result<String, JsoncUpdateError> {
        use versions_manifest_keys::CRATES;

        // Validate that `jsonc_str` conforms to the `JsoncManifest` schema.
        let _unused =
            Self::try_parse_jsonc(jsonc_str).map_err(JsoncUpdateError::InvalidManifest)?;

        // Parse the JSONC string into a Concrete Syntax Tree (CST) for
        // comment-preserving in-place mutation.
        let cst_root_node = CstRootNode::parse(jsonc_str, &ParseOptions::default())
            .map_err(JsoncUpdateError::CstParseFailed)?;

        // Create this closure so that we can use `?` to return early.
        let maybe_entry_updated_fn = || -> Option<()> {
            // Get `root_object` from CST root node.
            let root_object = cst_root_node.object_value()?;

            // Navigate to the `root_object["crates"][crate_name]` child object and
            // delegate leaf property updates to `CrateMetadata::try_update_cst_object`.
            let crates_object = root_object.object_value(CRATES)?;
            let crate_object = crates_object.object_value(crate_name)?;
            metadata.try_update_cst_object(&crate_object)
        };

        // If the closure returns `None`, it means the crate was not found, so we return
        // a typed error.
        if maybe_entry_updated_fn().is_none() {
            return Err(JsoncUpdateError::CrateNotFound {
                crate_name: crate_name.to_string(),
                file: VERSIONS_JSONC_FILE,
            });
        }

        // Convert CST back to a formatted String, preserving all comments and
        // formatting.
        ok!(cst_root_node.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{super::{CrateDistribution, CrateReleaseDate, CrateVersion},
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
            "binaries_release_base_url": "https://github.com/r3bl-org/r3bl-binaries/releases/download",
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
                binaries_release_base_url: EndpointUrl::try_parse(
                    "https://github.com/r3bl-org/r3bl-binaries/releases/download"
                )
                .unwrap(),
                crates: HashMap::from_iter([
                    (
                        "r3bl-cmdr".into(),
                        CrateMetadata {
                            version: CrateVersion::try_parse("0.0.25").unwrap(),
                            crate_distribution: CrateDistribution::OpenSource,
                            released_on: CrateReleaseDate::try_parse(
                                "2026-10-17T00:00:00Z",
                            )
                            .unwrap(),
                        },
                    ),
                    (
                        "r3bl-closed-source-product".into(),
                        CrateMetadata {
                            version: CrateVersion::try_parse("1.2.0").unwrap(),
                            crate_distribution: CrateDistribution::ClosedSource,
                            released_on: CrateReleaseDate::try_parse(
                                "2026-10-17T12:00:00Z",
                            )
                            .unwrap(),
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
            "binaries_release_base_url": "https://github.com/r3bl-org/r3bl-binaries/releases/download",
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
                binaries_release_base_url: EndpointUrl::try_parse(
                    "https://github.com/r3bl-org/r3bl-binaries/releases/download"
                )
                .unwrap(),
                crates: HashMap::from_iter([(
                    "r3bl-cmdr".into(),
                    CrateMetadata {
                        version: CrateVersion::try_parse("0.0.25").unwrap(),
                        crate_distribution: CrateDistribution::OpenSource,
                        released_on: CrateReleaseDate::try_parse("2026-10-17T00:00:00Z")
                            .unwrap(),
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
            "binaries_release_base_url": "https://github.com/r3bl-org/r3bl-binaries/releases/download",
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
            version: CrateVersion::try_parse("0.0.26").unwrap(),
            crate_distribution: CrateDistribution::OpenSource,
            released_on: CrateReleaseDate::try_parse("2026-10-18T15:30:00Z").unwrap(),
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
            "binaries_release_base_url": "https://github.com/r3bl-org/r3bl-binaries/releases/download",
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
        let jsonc = r#"{ "binaries_release_base_url": "https://example.com/releases/download", "crates": {} }"#;
        let meta = CrateMetadata {
            version: CrateVersion::try_parse("0.0.1").unwrap(),
            crate_distribution: CrateDistribution::OpenSource,
            released_on: CrateReleaseDate::try_parse("2026-10-18T00:00:00Z").unwrap(),
        };
        let err = JsoncManifest::try_update_crate_in_jsonc(
            jsonc,
            &"missing-crate".into(),
            &meta,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            JsoncUpdateError::CrateNotFound {
                ref crate_name,
                file,
            } if crate_name == "missing-crate" && file == VERSIONS_JSONC_FILE
        ));
    }

    #[test]
    fn test_jsonc_manifest_update_invalid_manifest() {
        let bad_jsonc = r#"{ "crates": "#;
        let meta = CrateMetadata {
            version: CrateVersion::try_parse("0.0.1").unwrap(),
            crate_distribution: CrateDistribution::OpenSource,
            released_on: CrateReleaseDate::try_parse("2026-10-18T00:00:00Z").unwrap(),
        };
        assert!(matches!(
            JsoncManifest::try_update_crate_in_jsonc(
                bad_jsonc,
                &"r3bl-cmdr".into(),
                &meta,
            ),
            Err(JsoncUpdateError::InvalidManifest(
                JsoncParseError::InvalidSyntax(_)
            ))
        ));
    }

    #[test]
    fn test_jsonc_manifest_parse_invalid_syntax() {
        let bad_jsonc = r#"{ "crates": "#;
        assert!(matches!(
            JsoncManifest::try_parse_jsonc(bad_jsonc),
            Err(JsoncParseError::InvalidSyntax(_))
        ));
    }

    #[test]
    fn test_jsonc_manifest_rejects_missing_binaries_release_base_url() {
        let missing_base_url_jsonc = r#"{
            "crates": {
                "r3bl-cmdr": {
                    "version": "0.0.25",
                    "crate_distribution": "open-source",
                    "released_on": "2026-10-17T00:00:00Z"
                }
            }
        }"#;
        let err = JsoncManifest::try_parse_jsonc(missing_base_url_jsonc)
            .expect_err("Manifest missing binaries_release_base_url must be rejected");
        assert!(matches!(err, JsoncParseError::InvalidSchema(_)));
        assert!(
            err.to_string()
                .contains(versions_manifest_keys::BINARIES_RELEASE_BASE_URL)
        );
    }

    #[test]
    fn test_jsonc_manifest_rejects_malformed_binaries_release_base_url() {
        for bad_base_url in [
            "",
            "github.com/r3bl-org/r3bl-binaries/releases/download",
            "htps://github.com/r3bl-org/r3bl-binaries/releases/download",
        ] {
            let malformed_jsonc = format!(
                r#"{{
                    "binaries_release_base_url": "{bad_base_url}",
                    "crates": {{
                        "r3bl-cmdr": {{
                            "version": "0.0.25",
                            "crate_distribution": "open-source",
                            "released_on": "2026-10-17T00:00:00Z"
                        }}
                    }}
                }}"#
            );
            assert!(
                JsoncManifest::try_parse_jsonc(&malformed_jsonc).is_err(),
                "Expected malformed binaries_release_base_url '{bad_base_url}' to fail"
            );
        }
    }
}
