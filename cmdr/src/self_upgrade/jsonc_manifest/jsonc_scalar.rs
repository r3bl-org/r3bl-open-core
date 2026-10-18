// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use jsonc_parser::cst::CstInputValue;
use serde::{Deserialize, Deserializer};

/// A trait for fallible parsing and lossless CST serialization of [`JsoncManifest`]
/// scalar types.
///
/// Types implementing this trait:
/// 1. Parse and validate a `&str` via [`Self::try_parse()`].
/// 2. Get [`serde`] field deserialization for free on the read path via
///    [`Self::deserialize_str()`].
/// 3. Convert back into a [`CstInputValue`] on the lossless CST write path via
///    [`Self::cst_value()`], which:
///    - decouples this from [`Display`] and preserves the separation of concerns of
///      serialization from UI formatting,
///    - while avoiding [`serde::Serialize`], which would strip JSONC comments.
///
/// See [How we parse JSONC] on [`JsoncManifest`] for the complete read/write architecture
/// and why [`jsonc_parser`], [`serde::Deserialize`], and [`Self::cst_value()`] are used
/// together.
///
/// This trait is implemented by the following types:
/// - [`EndpointUrl`].
/// - [`CrateVersion`].
/// - [`CrateReleaseDate`].
/// - [`CrateDistribution`].
///
/// The `#[serde(deserialize_with = "JsoncScalar::deserialize_str")]` attribute is used on
/// the following fields (which correspond to the types above):
/// - [`JsoncManifest::binaries_release_base_url`][1].
/// - [`CrateMetadata::version`][2].
/// - [`CrateMetadata::released_on`][3].
/// - [`CrateMetadata::crate_distribution`][4].
///
/// [1]: crate::self_upgrade::JsoncManifest::binaries_release_base_url
/// [2]: crate::self_upgrade::CrateMetadata::version
/// [3]: crate::self_upgrade::CrateMetadata::released_on
/// [4]: crate::self_upgrade::CrateMetadata::crate_distribution
/// [`CrateDistribution`]: crate::self_upgrade::CrateDistribution
/// [`CrateReleaseDate`]: crate::self_upgrade::CrateReleaseDate
/// [`CrateVersion`]: crate::self_upgrade::CrateVersion
/// [`Display`]: std::fmt::Display
/// [`EndpointUrl`]: crate::self_upgrade::EndpointUrl
/// [`JsoncManifest`]: crate::self_upgrade::JsoncManifest
/// [How we parse JSONC]: crate::self_upgrade::JsoncManifest#how-we-parse-jsonc
pub trait JsoncScalar: Sized {
    /// The associated error type (bounded by [`std::error::Error`]) returned when
    /// [`Self::try_parse`] fails.
    ///
    /// The [`std::error::Error`] trait bound guarantees:
    /// - [`Display`]: required by [`serde::de::Error::custom`] in
    ///   [`Self::deserialize_str`].
    /// - [`Debug`]: required by [`Result::unwrap`] / [`Result::expect`] at call sites and
    ///   in tests.
    /// - [`std::error::Error`]: required for `?` error propagation (e.g.,
    ///   [`miette::IntoDiagnostic`]).
    ///
    /// [`Debug`]: std::fmt::Debug
    /// [`Display`]: std::fmt::Display
    type ParseError: std::error::Error;

    /// Parses a string slice `s` to return a validated value of this type.
    ///
    /// # Errors
    ///
    /// Returns [`Self::ParseError`] if `s` is not a valid representation of `Self`.
    fn try_parse(s: &str) -> Result<Self, Self::ParseError>;

    /// Converts `self` into its canonical JSONC CST node value ([`CstInputValue`]) for
    /// comment-preserving in-place updates in
    /// [`JsoncManifest::try_update_crate_in_jsonc()`].
    ///
    /// Keeping wire serialization in [`Self::cst_value()`] (rather than coupling it to
    /// [`Display`]) preserves separation of concerns between human-facing terminal output
    /// and the `versions.jsonc` wire format.
    ///
    /// [`Display`]: std::fmt::Display
    /// [`JsoncManifest::try_update_crate_in_jsonc()`]: crate::self_upgrade::JsoncManifest::try_update_crate_in_jsonc
    fn cst_value(&self) -> CstInputValue;

    /// Deserializes a JSON string into `Self` by delegating to [`Self::try_parse`].
    ///
    /// Use with `#[serde(deserialize_with = "JsoncScalar::deserialize_str")]` on struct
    /// fields so concrete types do not need `TryFrom<String>` or `TryFrom<&str>`
    /// implementations.
    ///
    /// # Errors
    ///
    /// Returns a [`serde::de::Error`] if the input is not a string or fails
    /// [`Self::try_parse`].
    fn deserialize_str<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::try_parse(&s).map_err(serde::de::Error::custom)
    }
}
