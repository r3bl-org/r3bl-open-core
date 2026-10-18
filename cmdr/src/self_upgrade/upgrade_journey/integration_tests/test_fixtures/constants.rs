// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Composed compile-time constants and endpoint test fixtures.

use crate::self_upgrade::{EndpointUrl, JsoncScalar};
use const_format::formatcp;

/// Primitive building blocks, hostnames, and network addresses for test fixtures.
pub mod blocks {

    /// HTTP URL scheme prefix for local test server endpoints.
    pub const HTTP_PREFIX: &str = "http://";

    /// HTTPS URL scheme prefix.
    pub const HTTPS_PREFIX: &str = "https://";

    /// Non-resolving mock host ([RFC 2606 §2]).
    ///
    /// [RFC 2606 §2]: https://datatracker.ietf.org/doc/html/rfc2606#section-2
    pub const PLACEHOLDER_HOST: &str = "placeholder.invalid";

    /// Relative URL path for binary release asset downloads.
    pub const RELEASES_DOWNLOAD_PATH: &str = "releases/download";

    /// Central version manifest filename.
    pub const VERSIONS_JSONC_FILE: &str = "versions.jsonc";

    /// Test crate name matching `Cargo.toml`.
    pub const TEST_CRATE_NAME: &str = "r3bl-cmdr";

    /// Test crate version matching local test version for up-to-date checks.
    pub const TEST_CURRENT_VERSION: &str = "0.0.25";

    /// Newer semver used to trigger upgrade-available branches.
    pub const TEST_UPGRADE_AVAILABLE_VERSION: &str = "9.9.9";

    /// Static ISO-8601 release timestamp for test manifest entries.
    pub const TEST_RELEASED_ON_TIMESTAMP: &str = "2026-04-06T12:00:00Z";

    /// Loopback address with port 0 requesting an OS-assigned port number (ephemeral
    /// port).
    pub const LOCALHOST_EPHEMERAL_PORT_BIND_ADDR: &str = "127.0.0.1:0";

    /// Buffer capacity in bytes for reading incoming test HTTP request headers.
    pub const HTTP_REQUEST_BUFFER_SIZE: usize = 1_024;

    /// HTTP method name for `GET` requests.
    pub const HTTP_METHOD_GET: &str = "GET";

    /// HTTP method name for `HEAD` requests.
    pub const HTTP_METHOD_HEAD: &str = "HEAD";

    /// File extension for SHA-256 checksum files.
    pub const SHA256_EXT: &str = ".sha256";

    /// File extension for compressed tarball release archives.
    pub const TAR_GZ_EXT: &str = ".tar.gz";

    /// MIME content type for JSON responses.
    pub const MIME_APPLICATION_JSON: &str = "application/json";

    /// MIME content type for binary payload downloads.
    pub const MIME_APPLICATION_OCTET_STREAM: &str = "application/octet-stream";

    /// MIME content type for plain text responses (e.g., checksum files).
    pub const MIME_TEXT_PLAIN: &str = "text/plain";

    /// HTTP status line for a successful response.
    pub const HTTP_200_OK: &str = "200 OK";

    /// HTTP status line for an internal server error response.
    pub const HTTP_500_INTERNAL_SERVER_ERROR: &str = "500 Internal Server Error";
}

pub use blocks::*;

/// Default placeholder `binaries_release_base_url` value used by
/// [`placeholder_release_base_url`].
pub const PLACEHOLDER_RELEASE_BASE_URL: &str = formatcp!(
    "{scheme}{host}/{path}",
    scheme = blocks::HTTPS_PREFIX,
    host = blocks::PLACEHOLDER_HOST,
    path = blocks::RELEASES_DOWNLOAD_PATH,
);

/// Returns the default placeholder [`EndpointUrl`] for release archive downloads
/// ([`PLACEHOLDER_RELEASE_BASE_URL`]).
///
/// # Panics
/// Panics if [`PLACEHOLDER_RELEASE_BASE_URL`] fails to parse as a valid [`EndpointUrl`].
#[must_use]
pub fn placeholder_release_base_url() -> EndpointUrl {
    EndpointUrl::try_parse(PLACEHOLDER_RELEASE_BASE_URL)
        .expect("PLACEHOLDER_RELEASE_BASE_URL must produce a valid EndpointUrl")
}

/// Expected filename of the test release archive `.tar.gz`.
pub const TEST_ARCHIVE_FILENAME: &str = formatcp!(
    "{crate_name}-v{version}.tar.gz",
    crate_name = blocks::TEST_CRATE_NAME,
    version = blocks::TEST_UPGRADE_AVAILABLE_VERSION,
);

/// Request path for the central version manifest file.
pub const VERSIONS_JSONC_PATH: &str =
    formatcp!("/{file}", file = blocks::VERSIONS_JSONC_FILE);
