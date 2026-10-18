// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::{R3BL_VERSIONS_JSONC_URL,
                    url_schemes::{HTTP, HTTPS}},
            JsoncScalar};
use jsonc_parser::cst::CstInputValue;
use std::{fmt::{Display, Formatter},
          ops::Deref};

/// Validated HTTP(S) endpoint URL (manifest URL or binary release archive base URL).
///
/// Encapsulates the remote URL string used for querying `versions.jsonc` (via
/// [`UpgradeConfig`]) or downloading pre-compiled release archives (via
/// [`JsoncManifest::binaries_release_base_url`][1]).
///
/// Every [`EndpointUrl`] is validated by [`JsoncScalar::try_parse`] (directly or via
/// `#[serde(deserialize_with = "JsoncScalar::deserialize_str")]`) to ensure:
/// 1. Valid WHATWG URL syntax with a non-empty host.
/// 2. `"https"` or `"http"` scheme (catching typos like `"htps://..."`).
/// 3. No `?query` or `#fragment` components (which would break release asset URL path
///    joining).
///
/// [1]: crate::self_upgrade::JsoncManifest::binaries_release_base_url
/// [`UpgradeConfig`]: crate::self_upgrade::UpgradeConfig
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EndpointUrl {
    inner: String,
}

impl EndpointUrl {
    /// Returns the default canonical URL for the centralized version manifest
    /// ([`R3BL_VERSIONS_JSONC_URL`]).
    ///
    /// # Panics
    ///
    /// Panics only if the compile-time [`R3BL_VERSIONS_JSONC_URL`] constant is invalid
    /// (verified by unit tests).
    ///
    /// [`R3BL_VERSIONS_JSONC_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSONC_URL
    #[must_use]
    pub fn default_manifest_url() -> Self {
        Self::try_parse(R3BL_VERSIONS_JSONC_URL)
            .expect("R3BL_VERSIONS_JSONC_URL must be a valid HTTP(S) endpoint URL")
    }
}

/// Validates and constructs an [`EndpointUrl`] from a string slice.
impl JsoncScalar for EndpointUrl {
    type ParseError = EndpointUrlParseError;

    fn try_parse(url_str: &str) -> Result<Self, Self::ParseError> {
        let trimmed = url_str.trim_ascii();

        let parsed = reqwest::Url::parse(trimmed).map_err(|err| {
            EndpointUrlParseError::InvalidSyntax {
                input: trimmed.to_string(),
                reason: err.to_string(),
            }
        })?;

        if !matches!(parsed.scheme(), HTTP | HTTPS) {
            return Err(EndpointUrlParseError::UnsupportedScheme {
                scheme: parsed.scheme().to_string(),
            });
        }

        if parsed.query().is_some() || parsed.fragment().is_some() {
            return Err(EndpointUrlParseError::UnexpectedQueryOrFragment {
                input: trimmed.to_string(),
            });
        }

        Ok(Self {
            inner: trimmed.to_string(),
        })
    }

    fn cst_value(&self) -> CstInputValue { self.inner.clone().into() }
}

/// Allows [`EndpointUrl`] to coerce to `&str` at call sites via deref coercion.
impl Deref for EndpointUrl {
    type Target = str;

    fn deref(&self) -> &Self::Target { &self.inner }
}

impl Display for EndpointUrl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

/// Failure reason returned by [`EndpointUrl::try_parse`].
///
/// [`EndpointUrl::try_parse`]: JsoncScalar::try_parse
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, miette::Diagnostic)]
pub enum EndpointUrlParseError {
    /// Failed WHATWG URL parsing (empty string, missing scheme, empty host, bad port,
    /// etc.).
    #[error("Invalid endpoint URL '{input}': {reason}")]
    InvalidSyntax {
        /// Trimmed input string that failed URL parsing.
        input: String,
        /// Underlying URL parser diagnostic message.
        reason: String,
    },

    /// Valid URL syntax, but scheme is neither `"https"` nor `"http"` (e.g., typo
    /// `"htps://..."` or `"file://..."`).
    #[error(
        "Unsupported endpoint URL scheme '{scheme}://': expected 'https://' or 'http://'"
    )]
    UnsupportedScheme {
        /// Parsed non-HTTP(S) scheme string.
        scheme: String,
    },

    /// Contains `?query` or `#fragment` components (which break `<base_url>/v<ver>-...`
    /// path joining).
    #[error("Endpoint URL '{input}' must not contain '?query' or '#fragment' components")]
    UnexpectedQueryOrFragment {
        /// Trimmed input URL containing a query or fragment.
        input: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3bl_tui::assert_eq2;

    #[test]
    fn test_endpoint_url_traits_and_defaults() {
        let default_url = EndpointUrl::default_manifest_url();
        assert_eq2!(&*default_url, R3BL_VERSIONS_JSONC_URL);
        assert_eq2!(default_url.to_string(), R3BL_VERSIONS_JSONC_URL);

        let custom_parsed =
            EndpointUrl::try_parse("  https://example.com/manifest.jsonc\n").unwrap();
        let custom_trimmed =
            EndpointUrl::try_parse("https://example.com/manifest.jsonc").unwrap();
        assert_eq2!(custom_parsed, custom_trimmed);
        assert_eq2!(&*custom_parsed, "https://example.com/manifest.jsonc");

        let loopback =
            EndpointUrl::try_parse("http://127.0.0.1:0/versions.jsonc").unwrap();
        assert_eq2!(&*loopback, "http://127.0.0.1:0/versions.jsonc");
    }

    #[test]
    fn test_endpoint_url_validation_rejects_malformed_inputs() {
        // Invalid syntax (empty, missing scheme, empty host, out-of-range port).
        for bad_syntax in [
            "",
            "   ",
            "r3bl.com/versions.jsonc",
            "/releases/download",
            "https://",
            "http://127.0.0.1:99999/versions.jsonc",
        ] {
            assert!(
                matches!(
                    EndpointUrl::try_parse(bad_syntax),
                    Err(EndpointUrlParseError::InvalidSyntax { .. })
                ),
                "Expected InvalidSyntax for '{bad_syntax}'"
            );
        }

        // Unsupported scheme (catches "htps://" typo and non-HTTP schemes).
        for bad_scheme in [
            "htps://github.com/r3bl-org/r3bl-binaries/releases/download",
            "file:///tmp/versions.jsonc",
            "ftp://example.com/releases",
        ] {
            assert!(
                matches!(
                    EndpointUrl::try_parse(bad_scheme),
                    Err(EndpointUrlParseError::UnsupportedScheme { .. })
                ),
                "Expected UnsupportedScheme for '{bad_scheme}'"
            );
        }

        // Unexpected query or fragment.
        for with_query_or_frag in [
            "https://example.com/releases/download?token=abc",
            "https://example.com/versions.jsonc#section",
        ] {
            assert!(
                matches!(
                    EndpointUrl::try_parse(with_query_or_frag),
                    Err(EndpointUrlParseError::UnexpectedQueryOrFragment { .. })
                ),
                "Expected UnexpectedQueryOrFragment for '{with_query_or_frag}'"
            );
        }
    }

    #[test]
    fn test_endpoint_url_serde_deserialization() {
        let mut de =
            serde_json::Deserializer::from_str(r#""https://example.com/versions.jsonc""#);
        let parsed = EndpointUrl::deserialize_str(&mut de).unwrap();
        assert_eq2!(&*parsed, "https://example.com/versions.jsonc");

        let mut empty_de = serde_json::Deserializer::from_str(r#""""#);
        assert!(EndpointUrl::deserialize_str(&mut empty_de).is_err());

        let mut bad_scheme_de =
            serde_json::Deserializer::from_str(r#""htps://example.com""#);
        assert!(EndpointUrl::deserialize_str(&mut bad_scheme_de).is_err());
    }
}
