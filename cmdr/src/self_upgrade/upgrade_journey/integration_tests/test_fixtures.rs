// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Shared HTTP response payloads, manifest helpers, and local ephemeral HTTP server
//! fixtures for [`integration_tests`].
//!
//! [`integration_tests`]: mod@super

use std::{io::{Read, Write},
          net::TcpListener,
          time::Duration};

/// Placeholder `binaries_release_base_url` value inside the `http_responses/*.jsonc`
/// files, replaced at runtime by [`with_release_base_url`] when a test needs to point
/// release downloads at an ephemeral `127.0.0.1:<port>` server.
pub const PLACEHOLDER_RELEASE_BASE_URL: &str = "https://example.com/releases/download";

/// Manifest JSONC payload where `r3bl-cmdr` is at `0.0.25` (`open-source`), matching the
/// local test version `0.0.25` ([`UpToDate`]).
///
/// [`UpToDate`]: crate::self_upgrade::UpToDate
pub const MANIFEST_UP_TO_DATE_JSONC: &str =
    include_str!("http_responses/manifest_up_to_date.jsonc");

/// Manifest JSONC payload where `r3bl-cmdr` has a newer `9.9.9` (`open-source`) release
/// available.
pub const MANIFEST_UPGRADE_AVAILABLE_OPEN_SOURCE_JSONC: &str =
    include_str!("http_responses/manifest_upgrade_available_open_source.jsonc");

/// Manifest JSONC payload where `r3bl-cmdr` has a newer `9.9.9` (`closed-source`) release
/// available.
pub const MANIFEST_UPGRADE_AVAILABLE_CLOSED_SOURCE_JSONC: &str =
    include_str!("http_responses/manifest_upgrade_available_closed_source.jsonc");

/// Replaces [`PLACEHOLDER_RELEASE_BASE_URL`] inside one of the `MANIFEST_*_JSONC`
/// fixtures with an ephemeral local `release_base_url` (e.g.,
/// `"http://127.0.0.1:54321/releases/download"`).
#[must_use]
pub fn with_release_base_url(manifest_jsonc: &str, release_base_url: &str) -> String {
    manifest_jsonc.replace(PLACEHOLDER_RELEASE_BASE_URL, release_base_url)
}

/// Spawns an ephemeral local HTTP server on `127.0.0.1:0` that accepts a single request,
/// optionally sleeps for `delay`, and responds with `status` and `body`.
///
/// Returns `(manifest_url, join_handle)`.
pub fn spawn_one_shot_http_server(
    status: &'static str,
    body: impl Into<String>,
    delay: Duration,
) -> (String, std::thread::JoinHandle<()>) {
    let body = body.into();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut req_buf = [0_u8; 1024];
            drop(stream.read(&mut req_buf));
            if !delay.is_zero() {
                std::thread::sleep(delay);
            }
            let resp = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            drop(stream.write_all(resp.as_bytes()));
            drop(stream.flush());
        }
    });
    (format!("http://{addr}/versions.jsonc"), handle)
}
