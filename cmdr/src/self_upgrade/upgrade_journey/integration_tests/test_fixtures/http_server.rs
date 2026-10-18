// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! HTTP server test fixtures for [`self_upgrade`] integration and unit tests. They use
//! [`mockito`] to create a local HTTP server that's created just for the duration of each
//! test.
//!
//! All `spawn_*` helpers launch the server synchronously via
//! [`mockito::Server::new_with_opts`] so that these helpers can be called from both
//! `#[test]` and `#[tokio::test]`. For async tests, [`mockito::Server::new`] will panic.
//!
//! [`self_upgrade`]: mod@crate::self_upgrade

use super::{constants::*, jsonc_golden::*};
use crate::self_upgrade::{EndpointUrl, Sha256Digest};
use std::time::Duration;

pub mod manifest_server {
    use super::*;

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

    /// Spawns a single-request local [`mockito::Server`] serving [`VERSIONS_JSONC_PATH`]
    /// with configurable status and optional response delay.
    fn spawn_one_shot_http_server_with_options(
        response_body: String,
        status: usize,
        maybe_delay: Option<Duration>,
    ) -> ServerSpawnResult {
        // ServerOpts::default() binds to 127.0.0.1:0 (OS-assigned ephemeral port).
        let mut server = mockito::Server::new_with_opts(mockito::ServerOpts::default());
        let addr = server.socket_address();
        let url = EndpointUrl::default_manifest_url().with_socket_addr(addr);

        let endpoint = server
            .mock(HTTP_METHOD_GET, VERSIONS_JSONC_PATH)
            .with_status(status)
            .with_header(HEADER_CONTENT_TYPE, MIME_APPLICATION_JSON)
            .expect(1);

        // Add a delay to the response if specified.
        let endpoint = match maybe_delay {
            Some(delay) => {
                let body_bytes = response_body.into_bytes();
                endpoint.with_body_from_request(move |_| {
                    std::thread::sleep(delay);
                    body_bytes.clone()
                })
            }
            None => endpoint.with_body(response_body),
        }
        .create();

        ServerSpawnResult {
            url,
            server,
            endpoints: vec![endpoint],
        }
    }
}

pub mod archive_server {
    use super::*;

    /// Spawns a local [`mockito::Server`] handling the 4 sequential requests made during
    /// a release-archive upgrade:
    /// 1. `GET /versions.jsonc` -> manifest JSONC
    /// 2. `HEAD /releases/download/.../*.tar.gz` -> `200 OK` header with `Content-Length`
    /// 3. `GET /releases/download/.../*.tar.gz.sha256` -> SHA-256 checksum derived from
    ///    `archive_bytes`
    /// 4. `GET /releases/download/.../*.tar.gz` -> raw archive `.tar.gz` bytes
    #[must_use]
    pub fn spawn_success(archive_bytes: Vec<u8>) -> ServerSpawnResult {
        let archive_sha256 = Sha256Digest::from_bytes(&archive_bytes);
        let TestReleaseAssets {
            archive_filename: _,
            archive_url_path,
            sha256_url_path,
        } = TestReleaseAssets::for_current_host();

        // ServerOpts::default() binds to 127.0.0.1:0 (OS-assigned ephemeral port).
        let mut server = mockito::Server::new_with_opts(mockito::ServerOpts::default());
        let addr = server.socket_address();

        let release_url = placeholder_release_base_url().with_socket_addr(addr);
        let manifest_body = manifest_upgrade_available_open_source(&release_url);

        // Bare 64-char lowercase hex digest with trailing newline.
        let sha_body = format!("{archive_sha256}\n");
        let archive_len = archive_bytes.len().to_string();

        let manifest_endpoint = server
            .mock(HTTP_METHOD_GET, VERSIONS_JSONC_PATH)
            .with_status(HTTP_200_OK)
            .with_header(HEADER_CONTENT_TYPE, MIME_APPLICATION_JSON)
            .with_body(manifest_body)
            .expect(1)
            .create();

        let archive_head_endpoint = server
            .mock(HTTP_METHOD_HEAD, archive_url_path.as_str())
            .with_status(HTTP_200_OK)
            .with_header(HEADER_CONTENT_TYPE, MIME_APPLICATION_OCTET_STREAM)
            .with_header(HEADER_CONTENT_LENGTH, &archive_len)
            .expect(1)
            .create();

        let checksum_endpoint = server
            .mock(HTTP_METHOD_GET, sha256_url_path.as_str())
            .with_status(HTTP_200_OK)
            .with_header(HEADER_CONTENT_TYPE, MIME_TEXT_PLAIN)
            .with_body(sha_body)
            .expect(1)
            .create();

        let archive_download_endpoint = server
            .mock(HTTP_METHOD_GET, archive_url_path.as_str())
            .with_status(HTTP_200_OK)
            .with_header(HEADER_CONTENT_TYPE, MIME_APPLICATION_OCTET_STREAM)
            .with_body(archive_bytes)
            .expect(1)
            .create();

        ServerSpawnResult {
            url: EndpointUrl::default_manifest_url().with_socket_addr(addr),
            server,
            endpoints: vec![
                manifest_endpoint,
                archive_head_endpoint,
                checksum_endpoint,
                archive_download_endpoint,
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
/// - Do not discard it with `let _ = ...` or extract only `.url` while dropping the rest
///   of the struct.
/// - Dropping `server` [`Self::server`] invokes [`mockito::Server::reset`], which
///   immediately clears all registered [`Self::endpoints`] before client HTTP requests
///   can match them.
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
    pub url: EndpointUrl,

    /// Owned [`mockito::Server`] RAII guard keeping registered endpoints alive for the
    /// test scope (dropping [`mockito::Server`] clears all registered endpoints).
    pub server: mockito::Server,

    /// Registered HTTP endpoints ([`mockito::Mock`]), each configured with `.expect(1)`
    /// and verified via [`Self::assert_each_endpoint_called_once`].
    pub endpoints: Vec<mockito::Mock>,
}

impl ServerSpawnResult {
    /// Asserts that every registered HTTP endpoint in [`Self::endpoints`] was called
    /// once during the test.
    pub fn assert_each_endpoint_called_once(&self) {
        for endpoint in &self.endpoints {
            endpoint.assert();
        }
    }
}
