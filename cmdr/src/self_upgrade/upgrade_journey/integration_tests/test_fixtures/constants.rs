// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Composed compile-time constants and endpoint test fixtures.

use crate::self_upgrade::{EndpointUrl, JsoncScalar, ReleaseAsset, ReleaseTarget, archive};
use const_format::formatcp;

/// Primitive building blocks, hostnames, and network addresses for test fixtures.
pub mod blocks {
    /// HTTP URL scheme prefix for local test server endpoints.
    pub const HTTP_PREFIX: &str = "http://";

    /// HTTPS URL scheme prefix.
    pub const HTTPS_PREFIX: &str = "https://";

    /// Loopback address with port `0` requesting an OS-assigned port number ([ephemeral
    /// port]).
    ///
    /// [ephemeral port]: https://en.wikipedia.org/wiki/Ephemeral_port
    pub const LOCALHOST_EPHEMERAL_PORT_BIND_ADDR: &str = "127.0.0.1:0";

    /// HTTP method name for `GET` requests.
    pub const HTTP_METHOD_GET: &str = "GET";

    /// HTTP method name for `HEAD` requests.
    pub const HTTP_METHOD_HEAD: &str = "HEAD";

    /// HTTP header name for payload MIME type.
    pub const HEADER_CONTENT_TYPE: &str = "content-type";

    /// HTTP header name for payload byte length.
    pub const HEADER_CONTENT_LENGTH: &str = "content-length";

    /// MIME content type for JSON responses.
    pub const MIME_APPLICATION_JSON: &str = "application/json";

    /// MIME content type for binary payload downloads.
    pub const MIME_APPLICATION_OCTET_STREAM: &str = "application/octet-stream";

    /// MIME content type for plain text responses (e.g., checksum files).
    pub const MIME_TEXT_PLAIN: &str = "text/plain";

    /// Numeric HTTP status code for a successful response (`200 OK`).
    pub const HTTP_200_OK: usize = 200;

    /// Numeric HTTP status code for an internal server error (`500 Internal Server
    /// Error`).
    pub const HTTP_500_INTERNAL_SERVER_ERROR: usize = 500;

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

/// Request path for the central version manifest file.
pub const VERSIONS_JSONC_PATH: &str = formatcp!(
    "{slash}{file}",
    slash = archive::URL_PATH_SEPARATOR,
    file = blocks::VERSIONS_JSONC_FILE,
);

/// Request path prefix for binary release asset downloads on the local test server.
pub const RELEASES_DOWNLOAD_REQUEST_PATH: &str = formatcp!(
    "{slash}{path}",
    slash = archive::URL_PATH_SEPARATOR,
    path = blocks::RELEASES_DOWNLOAD_PATH,
);

/// Host-resolved release archive filename and local test HTTP server request paths
/// derived from [`ReleaseAsset::format_archive_filename`] and
/// [`ReleaseAsset::format_urls`] (see its [URL Anatomy] for why crate and version appear
/// in both the tag and filename).
///
/// This struct resolves routes for the test runner's [`ReleaseTarget`] (rather than
/// hardcoding Linux `const` strings) so mock servers match what
/// [`UpgradeSource::try_resolve`] requests across Linux, macOS, and Windows instead of
/// `404`-ing on non-Linux hosts.
///
/// [`ReleaseAsset::format_archive_filename`]: crate::self_upgrade::ReleaseAsset::format_archive_filename
/// [`ReleaseAsset::format_urls`]: crate::self_upgrade::ReleaseAsset::format_urls
/// [`ReleaseTarget`]: crate::self_upgrade::ReleaseTarget
/// [`UpgradeSource::try_resolve`]: crate::self_upgrade::UpgradeSource::try_resolve
/// [URL Anatomy]: crate::self_upgrade::ReleaseAsset::format_urls#url-anatomy
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestReleaseAssets {
    /// Archive filename (written into the mock `.sha256` checksum body).
    pub archive_filename: String,

    /// Relative URL path for `HEAD` and `GET` archive requests on the mock server.
    pub archive_path: String,

    /// Relative URL path for `GET` `.sha256` checksum requests on the mock server.
    pub sha256_path: String,
}

impl TestReleaseAssets {
    /// Builds [`TestReleaseAssets`] for the current test runner's [`ReleaseTarget`]
    /// (via [`ReleaseTarget::try_from_host`]).
    ///
    /// # Panics
    ///
    /// Panics if the current host platform does not map to a supported [`ReleaseTarget`].
    ///
    /// [`ReleaseTarget::try_from_host`]: crate::self_upgrade::ReleaseTarget::try_from_host
    /// [`ReleaseTarget`]: crate::self_upgrade::ReleaseTarget
    #[must_use]
    pub fn for_current_host() -> Self {
        // Determined at runtime on the current host.
        let target = ReleaseTarget::try_from_host()
            .expect("test host must map to a supported ReleaseTarget");

        // Constants.
        let archive_filename = ReleaseAsset::format_archive_filename(
            blocks::TEST_CRATE_NAME,
            blocks::TEST_UPGRADE_AVAILABLE_VERSION,
            target,
        );

        // Constants.
        let (archive_path, sha256_path) = ReleaseAsset::format_urls(
            RELEASES_DOWNLOAD_REQUEST_PATH,
            blocks::TEST_CRATE_NAME,
            blocks::TEST_UPGRADE_AVAILABLE_VERSION,
            target,
        );

        Self {
            archive_filename,
            archive_path,
            sha256_path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3bl_tui::assert_eq2;

    #[test]
    fn test_release_assets_stay_in_sync_with_production_urls() {
        let assets = TestReleaseAssets::for_current_host();
        let target = ReleaseTarget::try_from_host().unwrap();
        let (archive_url, sha256_url) = ReleaseAsset::format_urls(
            &placeholder_release_base_url(),
            blocks::TEST_CRATE_NAME,
            blocks::TEST_UPGRADE_AVAILABLE_VERSION,
            target,
        );
        let host_origin = format!("{}{}", blocks::HTTPS_PREFIX, blocks::PLACEHOLDER_HOST);

        assert_eq2!(
            assets.archive_filename,
            format!("r3bl-cmdr-v9.9.9-{target}.tar.gz")
        );
        assert_eq2!(format!("{host_origin}{}", assets.archive_path), archive_url);
        assert_eq2!(format!("{host_origin}{}", assets.sha256_path), sha256_url);
    }
}
