// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::{R3BL_VERSIONS_JSONC_URL,
                    url_schemes::{HTTP, HTTPS}},
            JsoncScalar};
#[cfg(any(test, doc))]
use crate::self_upgrade::integration_tests::test_fixtures::blocks::LOCALHOST_EPHEMERAL_PORT_BIND_ADDR;
use jsonc_parser::cst::CstInputValue;
#[cfg(any(test, doc))]
use std::net::{SocketAddr, TcpListener};
use std::{fmt::{Display, Formatter},
          ops::Deref};

/// Validated HTTP(S) endpoint URL (manifest URL or binary release archive base URL).
///
/// Encapsulates the remote URL string used in 2 places:
/// 1. Querying `versions.jsonc`, via [`UpgradeConfig`].
/// 2. Downloading pre-compiled release archives, via
///    [`JsoncManifest::binaries_release_base_url`][1].
///
/// Every [`EndpointUrl`] is validated by [`JsoncScalar::try_parse`] (directly or via
/// `#[serde(deserialize_with = "JsoncScalar::deserialize_str")]`) to ensure:
/// 1. Valid [WHATWG URL] syntax with a non-empty host.
/// 2. `"https"` or `"http"` scheme (catching typos like `"htps://..."`).
/// 3. No `?query` or `#fragment` components (which would break release asset URL path
///    joining).
///
/// # Testing Support
///
/// These test helpers generate local endpoints from a given [`EndpointUrl`] by swapping
/// out remote protocol, host and port with ephemeral local ones, while leaving path
/// segments untouched (e.g. `releases/download`, or `versions.jsonc`):
///
/// ```text
/// Original URL:
/// ┌────────┬─────────────────────────────┬────────────────────────┐
/// │ https  │ placeholder.invalid         │ /releases/download     │
/// └────────┴─────────────────────────────┴────────────────────────┘
///     │                   │                          │
///     ▼ (Swapped)         ▼ (Swapped to SocketAddr)  ▼ (Preserved)
/// ┌────────┬─────────────────────────────┬────────────────────────┐
/// │ http   │ 127.0.0.1:54321             │ /releases/download     │
/// └────────┴─────────────────────────────┴────────────────────────┘
/// Synthesized Test Endpoint
/// ```
///
/// - [`with_socket_addr`]: Replaces host and port with a local mock server's
///   [`SocketAddr`], switching the scheme to `http`.
/// - [`with_closed_socket_addr`]: Points to an ephemeral loopback port guaranteed to be
///   closed to simulate connection-refused failures (such as unreachable release
///   servers).
///
/// [1]: crate::self_upgrade::JsoncManifest::binaries_release_base_url
/// [`SocketAddr`]: std::net::SocketAddr
/// [`UpgradeConfig`]: crate::self_upgrade::UpgradeConfig
/// [`with_closed_socket_addr`]: Self::with_closed_socket_addr
/// [`with_socket_addr`]: Self::with_socket_addr
/// [WHATWG URL]: https://url.spec.whatwg.org/
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

#[cfg(any(test, doc))]
impl EndpointUrl {
    /// Returns a new [`EndpointUrl`] with the host and port replaced by `addr`
    /// and the scheme switched to `http`. The path (e.g. `releases/download`)
    /// is preserved. See [Testing Support] on [`EndpointUrl`] for an overview and
    /// component diagram.
    ///
    /// # Panics
    ///
    /// Panics if the internal URL cannot be parsed by [`reqwest::Url`] or if
    /// the reconstructed URL fails [`EndpointUrl`] validation.
    ///
    /// [Testing Support]: EndpointUrl#testing-support
    #[must_use]
    pub fn with_socket_addr(&self, addr: SocketAddr) -> Self {
        let mut parsed =
            reqwest::Url::parse(self).expect("EndpointUrl must be valid URL");
        let _ = parsed.set_scheme(HTTP);
        let _ = parsed.set_ip_host(addr.ip());
        let _ = parsed.set_port(Some(addr.port()));
        Self::try_parse(parsed.as_str())
            .expect("reconstructed URL with socket address must be valid EndpointUrl")
    }

    /// Returns a new [`EndpointUrl`] pointing to an ephemeral loopback port that is
    /// guaranteed to be closed and reject connection attempts.
    ///
    /// It does this by binding a [`TcpListener`] to
    /// [`LOCALHOST_EPHEMERAL_PORT_BIND_ADDR`] requesting an OS-assigned ephemeral port,
    /// immediately dropping the listener, and delegating to [`with_socket_addr`]. See
    /// [Testing Support] on [`EndpointUrl`] for an overview and component diagram.
    ///
    /// # Why?
    ///
    /// In integration tests, we test client error handling when a pre-built binary
    /// release server is unreachable (testing fallback to `rustup` + `cargo install`, and
    /// testing error transitions). Rather than hardcoding an arbitrary port (which might
    /// conflict or succeed if occupied), this guarantees a local connection-refused port.
    ///
    /// # Panics
    ///
    /// Panics if binding to [`LOCALHOST_EPHEMERAL_PORT_BIND_ADDR`] or retrieving the
    /// local address fails.
    ///
    /// [`TcpListener`]: std::net::TcpListener
    /// [`with_socket_addr`]: Self::with_socket_addr
    /// [Testing Support]: EndpointUrl#testing-support
    #[must_use]
    pub fn with_closed_socket_addr(&self) -> Self {
        let listener = TcpListener::bind(LOCALHOST_EPHEMERAL_PORT_BIND_ADDR)
            .expect("must bind ephemeral address");
        let addr = listener.local_addr().expect("must get local address");
        drop(listener);
        self.with_socket_addr(addr)
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

/// Formats the endpoint URL for diagnostic messages and structured tracing logs.
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
    /// Failed [WHATWG URL] parsing (empty string, missing scheme, empty host, bad port,
    /// etc.).
    ///
    /// [WHATWG URL]: https://url.spec.whatwg.org/
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
    use crate::self_upgrade::integration_tests::test_fixtures::*;
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

        let expected_loopback_url = format!(
            "{}{}/{}",
            blocks::HTTP_PREFIX,
            blocks::LOCALHOST_EPHEMERAL_PORT_BIND_ADDR,
            blocks::VERSIONS_JSONC_FILE
        );
        let loopback = EndpointUrl::try_parse(&expected_loopback_url).unwrap();
        assert_eq2!(&*loopback, expected_loopback_url);

        let addr: SocketAddr =
            blocks::LOCALHOST_EPHEMERAL_PORT_BIND_ADDR.parse().unwrap();
        let from_generator = EndpointUrl::default_manifest_url().with_socket_addr(addr);
        assert_eq2!(loopback, from_generator);
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
            concat!("http://", "127.0.0.1:99999/versions.jsonc"),
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
        let json_input = format!(r#""{R3BL_VERSIONS_JSONC_URL}""#);
        let mut de = serde_json::Deserializer::from_str(&json_input);
        let parsed = EndpointUrl::deserialize_str(&mut de).unwrap();
        assert_eq2!(&*parsed, R3BL_VERSIONS_JSONC_URL);

        let mut empty_de = serde_json::Deserializer::from_str(r#""""#);
        assert!(EndpointUrl::deserialize_str(&mut empty_de).is_err());

        let mut bad_scheme_de =
            serde_json::Deserializer::from_str(r#""htps://example.com""#);
        assert!(EndpointUrl::deserialize_str(&mut bad_scheme_de).is_err());
    }

    #[test]
    fn test_with_socket_addr() {
        let original = placeholder_release_base_url();
        let addr: std::net::SocketAddr = "127.0.0.1:54321".parse().unwrap();
        let modified = original.with_socket_addr(addr);
        assert_eq2!(
            &*modified,
            format!("{}127.0.0.1:54321/{}", blocks::HTTP_PREFIX, blocks::RELEASES_DOWNLOAD_PATH)
        );
    }

    #[test]
    fn test_with_closed_socket_addr() {
        let original = placeholder_release_base_url();
        let modified = original.with_closed_socket_addr();
        assert!(modified.starts_with(blocks::HTTP_PREFIX));
        assert!(modified.contains("127.0.0.1:"));
        assert!(modified.ends_with(blocks::RELEASES_DOWNLOAD_PATH));
    }
}
