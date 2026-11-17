// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! HTTP server test fixtures for [`self_upgrade`] integration and unit tests. They use
//! [`mockito`] to create a local HTTP server that's created just for the duration of each
//! test.
//!
//! All `spawn_*` helpers are called by [`self_upgrade`] unit and integration tests.
//! - They pass the mock server's URL to the [`reqwest`] client code in the tests.
//! - The tests launch these server fixtures synchronously via
//!   [`mockito::Server::new_with_opts`] so they work in both `#[test]` and
//!   `#[tokio::test]` contexts.
//! - We avoid using [`mockito::Server::new`] which panics in async tests.
//!
//! [`self_upgrade`]: mod@crate::self_upgrade

use super::{constants::*, jsonc_golden::*};
use crate::self_upgrade::{EndpointUrl, Sha256Digest};
use std::time::Duration;

pub mod manifest_server {
    use super::*;

    /// Spawns a local [`mockito::Server`] that only handles a single request. It only
    /// generates a response to the [`VERSIONS_JSONC_PATH`] path in the `GET` request.
    ///
    /// # Arguments
    ///
    /// * `response_body`: Response body string served for the `GET` request.
    /// * `status`: Status code to set in the HTTP response status line (not header).
    /// * `maybe_delay`: Optional delay to wait before sending the response.
    ///
    /// # Callers & Client Under Test
    ///
    /// 1. **Tests** call `spawn_*` to start the mock server and pass the returned
    ///    [`ServerSpawnResult::endpoint_url`] into [`self_upgrade`] (the code under
    ///    test).
    /// 2. **Code under test: [`self_upgrade`]** invokes
    ///    [`fetch_crate_metadata_from_manifest`][1], whose [`reqwest`] client sends a
    ///    `GET` request to [`VERSIONS_JSONC_PATH`].
    ///
    /// [1]: crate::self_upgrade::fetch_crate_metadata_from_manifest
    /// [`self_upgrade`]: mod@crate::self_upgrade
    /// [`ServerSpawnResult::endpoint_url`]: super::ServerSpawnResult::endpoint_url
    #[must_use]
    pub fn spawn_one_shot_http_server_with_options(
        response_body: String,
        status: usize,
        maybe_delay: Option<Duration>,
    ) -> ServerSpawnResult {
        // ServerOpts::default() binds to 127.0.0.1:0 (OS-assigned ephemeral port).
        let mut spawned_server =
            mockito::Server::new_with_opts(mockito::ServerOpts::default());

        // Create the URL containing the OS-assigned port (determined at runtime).
        let spawned_server_endpoint_url = EndpointUrl::default_manifest_url()
            .with_socket_addr(spawned_server.socket_address());

        // Create the mock GET request handler.
        let get_manifest_mock_route_handler_builder = spawned_server
            .mock(HTTP_METHOD_GET, VERSIONS_JSONC_PATH)
            .with_status(status)
            .expect(1);

        // Add a delay to the response if specified for the given method + path.
        let get_manifest_route_builder = if let Some(delay) = maybe_delay {
            let body_bytes = response_body.into_bytes();
            get_manifest_mock_route_handler_builder.with_body_from_request(move |_| {
                std::thread::sleep(delay);
                body_bytes.clone()
            })
        } else {
            get_manifest_mock_route_handler_builder.with_body(response_body)
        };

        ServerSpawnResult {
            endpoint_url: spawned_server_endpoint_url,
            server: spawned_server,
            mock_route_handlers: vec![get_manifest_route_builder.create()],
        }
    }

    /// Responds immediately with [`blocks::HTTP_200_OK`] and `arg_response_body`.
    #[must_use]
    pub fn spawn_success(arg_response_body: impl Into<String>) -> ServerSpawnResult {
        spawn_one_shot_http_server_with_options(
            arg_response_body.into(),
            blocks::HTTP_200_OK,
            None,
        )
    }

    /// Responds immediately with `status` and `arg_response_body`.
    #[must_use]
    pub fn spawn_error(
        arg_response_body: impl Into<String>,
        status: usize,
    ) -> ServerSpawnResult {
        spawn_one_shot_http_server_with_options(arg_response_body.into(), status, None)
    }

    /// Delays for `delay` before responding with [`blocks::HTTP_200_OK`] and
    /// `arg_response_body`.
    #[must_use]
    pub fn spawn_delayed_success(
        arg_response_body: impl Into<String>,
        delay: Duration,
    ) -> ServerSpawnResult {
        spawn_one_shot_http_server_with_options(
            arg_response_body.into(),
            blocks::HTTP_200_OK,
            Some(delay),
        )
    }

    /// Delays for `delay` before responding with `status` and `arg_response_body`.
    #[must_use]
    pub fn spawn_delayed_error(
        arg_response_body: impl Into<String>,
        status: usize,
        delay: Duration,
    ) -> ServerSpawnResult {
        spawn_one_shot_http_server_with_options(
            arg_response_body.into(),
            status,
            Some(delay),
        )
    }
}

pub mod archive_server {
    use super::*;

    /// Spawns a local [`mockito::Server`] that handles the 4 sequential requests of a
    /// pre-compiled `.tar.gz` binary archive upgrade
    /// ([`UpgradeSource::ReleaseArchive`][1], not
    /// [`UpgradeSource::BuildFromSourceRustup`][2]).
    ///
    /// It generates responses for:
    /// 1. manifest,
    /// 2. availability (`HEAD`),
    /// 3. checksum (using `archive_bytes`),
    /// 4. archive download (using `archive_bytes`).
    ///
    /// # Arguments
    ///
    /// * `archive_bytes`: Raw `.tar.gz` archive bytes used to derive responses 3 and 4
    ///   below.
    ///
    /// # Callers & Client Under Test
    ///
    /// 1. **Tests** call `spawn_success` to start the mock server, pass the returned
    ///    [`ServerSpawnResult::endpoint_url`][10] into [`UpgradeConfig::new`][11], and
    ///    run [`UpgradeJourney::start`].
    /// 2. **Code under test: [`self_upgrade`]**'s internal [`reqwest`] clients then send
    ///    these 4 sequential requests:
    ///    1. `GET /versions.jsonc` -> manifest JSONC
    ///       - Requested by [`fetch_crate_metadata_from_manifest`][4] (driven by
    ///         [`Checking`][3]) during the version check.
    ///       - Hardcoded and generated in memory via
    ///         [`manifest_upgrade_available_open_source`].
    ///    2. `HEAD /releases/download/.../*.tar.gz` -> `200 OK`
    ///       - Requested by [`ReleaseAsset::check_available`][5] (called from
    ///         [`UpgradeSource::try_resolve`][6]) to confirm a pre-compiled archive
    ///         exists for the host target triple; it only checks that `response.status()`
    ///         is `2xx` (`is_success()`).
    ///       - This is hardcoded.
    ///    3. `GET /releases/download/.../*.tar.gz.sha256` -> SHA-256 checksum
    ///       - Requested by [`sha256_checksum::fetch_expected_sha256`][7] (called from
    ///         [`download_and_verify_archive`][8]) for verification.
    ///       - Derived from `archive_bytes`.
    ///    4. `GET /releases/download/.../*.tar.gz` -> raw `.tar.gz` archive bytes
    ///       - Requested by [`archive_stream::stream_archive_to_disk`][9] (called from
    ///         [`download_and_verify_archive`][8]) and streamed to extract and
    ///         self-replace the running binary in place.
    ///       - Derived from `archive_bytes`.
    ///
    /// [10]: super::ServerSpawnResult::endpoint_url
    /// [11]: crate::self_upgrade::UpgradeConfig::new
    /// [1]: crate::self_upgrade::UpgradeSource::ReleaseArchive
    /// [2]: crate::self_upgrade::UpgradeSource::BuildFromSourceRustup
    /// [3]: crate::self_upgrade::Checking
    /// [4]: crate::self_upgrade::fetch_crate_metadata_from_manifest
    /// [5]: crate::self_upgrade::ReleaseAsset::check_available
    /// [6]: crate::self_upgrade::UpgradeSource::try_resolve
    /// [7]: crate::self_upgrade::sha256_checksum::fetch_expected_sha256
    /// [8]: crate::self_upgrade::download_and_verify_archive
    /// [9]: crate::self_upgrade::archive_stream::stream_archive_to_disk
    /// [`self_upgrade`]: mod@crate::self_upgrade
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    #[must_use]
    pub fn spawn_success(archive_bytes: Vec<u8>) -> ServerSpawnResult {
        // ServerOpts::default() binds to 127.0.0.1:0 (OS-assigned ephemeral port).
        let mut spawned_server =
            mockito::Server::new_with_opts(mockito::ServerOpts::default());
        let addr = spawned_server.socket_address();

        // Create the URL containing the OS-assigned port (determined at runtime).
        let spawned_server_endpoint_url =
            EndpointUrl::default_manifest_url().with_socket_addr(addr);

        let TestReleaseAssets {
            archive_filename: _,
            archive_url_path,
            sha256_url_path,
        } = TestReleaseAssets::for_current_host();

        let get_manifest_route_builder = spawned_server
            .mock(HTTP_METHOD_GET, VERSIONS_JSONC_PATH)
            .with_status(HTTP_200_OK)
            .with_body({
                // Embed the server's runtime URL into the manifest.
                let response_release_url =
                    placeholder_release_base_url().with_socket_addr(addr);
                manifest_upgrade_available_open_source(&response_release_url)
            })
            .expect(1);

        // We don't include the `Content-Length` with the response, since `self_upgrade`
        // (which uses reqwest HTTP client) only checks the `2xx` status code. And no
        // other test code checks for `Content-Length`. See
        // `get_archive_mock_route_handler_builder` where the `Content-Length` is set.
        let head_archive_route_builder = spawned_server
            .mock(HTTP_METHOD_HEAD, archive_url_path.as_str())
            .with_status(HTTP_200_OK)
            .expect(1);

        let get_checksum_route_builder = spawned_server
            .mock(HTTP_METHOD_GET, sha256_url_path.as_str())
            .with_status(HTTP_200_OK)
            .with_body({
                let archive_sha256 = Sha256Digest::from_bytes(&archive_bytes);
                // Bare 64-char lowercase hex digest with trailing newline.
                format!("{archive_sha256}\n")
            })
            .expect(1);

        let get_archive_route_builder = spawned_server
            .mock(HTTP_METHOD_GET, archive_url_path.as_str())
            .with_status(HTTP_200_OK)
            .with_body(archive_bytes) // This sets the Content-Length header automatically.
            .expect(1);

        ServerSpawnResult {
            endpoint_url: spawned_server_endpoint_url,
            server: spawned_server,
            mock_route_handlers: vec![
                get_manifest_route_builder.create(),
                head_archive_route_builder.create(),
                get_checksum_route_builder.create(),
                get_archive_route_builder.create(),
            ],
        }
    }
}

/// Result of spawning a [`mockito::Server`] configured for [`self_upgrade`] tests.
///
/// # [RAII] Lifetime Requirement for `server` field
///
/// Callers must keep this [`ServerSpawnResult`] bound for the duration of the test
/// execution.
/// - Do not discard it with `let _ = ...` or extract only `.endpoint_url` while dropping
///   the rest of the struct.
/// - Dropping `server` [`Self::server`] invokes [`mockito::Server::reset`], which
///   immediately clears all registered [`Self::mock_route_handlers`] before client HTTP
///   requests can match them.
///
/// [`self_upgrade`]: mod@crate::self_upgrade
/// [RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
#[derive(Debug)]
pub struct ServerSpawnResult {
    /// Endpoint URL pointing to `/versions.jsonc` on the local [`mockito::Server`]. This
    /// is needed because the server binds to a dynamic [ephemeral port] (`127.0.0.1:0`)
    /// that tests must target at runtime.
    ///
    /// [ephemeral port]: https://en.wikipedia.org/wiki/Ephemeral_port
    pub endpoint_url: EndpointUrl,

    /// Owned [`mockito::Server`] RAII guard keeping registered mock route handlers alive
    /// for the test scope (dropping [`mockito::Server`] clears all registered mock route
    /// handlers).
    pub server: mockito::Server,

    /// Registered HTTP mock route handlers ([`mockito::Mock`]), each configured with
    /// `.expect(1)` and verified via [`Self::assert_each_mock_called_once`].
    pub mock_route_handlers: Vec<mockito::Mock>,
}

impl ServerSpawnResult {
    /// Asserts that every registered HTTP mock route handler in
    /// [`Self::mock_route_handlers`] was called once during the test.
    pub fn assert_each_mock_called_once(&self) {
        for mock_route_handler in &self.mock_route_handlers {
            mock_route_handler.assert();
        }
    }
}
