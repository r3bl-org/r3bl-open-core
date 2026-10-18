// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test fixtures for short-lived mock HTTP server fixtures and HTTP/1.1 response
//! generators.

use super::{constants::*, jsonc_golden::*};
use crate::self_upgrade::EndpointUrl;
use std::{io::{Read, Write},
          net::{SocketAddr, TcpListener, TcpStream},
          time::Duration};

pub mod manifest_server {
    use super::*;

    /// Responds immediately with [`blocks::HTTP_200_OK`] and `arg_response_body`.
    #[must_use]
    pub fn spawn_success(arg_response_body: impl Into<String>) -> ServerSpawnResult {
        spawn_one_shot_http_server_with_options(
            arg_response_body.into(),
            blocks::HTTP_200_OK.to_string(),
            None,
        )
    }

    /// Responds immediately with `arg_status` and `arg_response_body`.
    #[must_use]
    pub fn spawn_error(
        arg_response_body: impl Into<String>,
        arg_status: impl Into<String>,
    ) -> ServerSpawnResult {
        spawn_one_shot_http_server_with_options(
            arg_response_body.into(),
            arg_status.into(),
            None,
        )
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
            blocks::HTTP_200_OK.to_string(),
            Some(delay),
        )
    }

    /// Delays for `delay` before responding with `arg_status` and `arg_response_body`.
    #[must_use]
    pub fn spawn_delayed_error(
        arg_response_body: impl Into<String>,
        arg_status: impl Into<String>,
        delay: Duration,
    ) -> ServerSpawnResult {
        spawn_one_shot_http_server_with_options(
            arg_response_body.into(),
            arg_status.into(),
            Some(delay),
        )
    }

    /// Spawns a single-request local mock HTTP server with configurable response options.
    ///
    /// Binds to an [ephemeral port] on [`LOCALHOST_EPHEMERAL_PORT_BIND_ADDR`], optionally
    /// sleeps for `maybe_delay`, and responds with `status` and `response_body`.
    ///
    /// Returns [`ServerSpawnResult`].
    ///
    /// # Panics
    ///
    /// Panics if binding to [`LOCALHOST_EPHEMERAL_PORT_BIND_ADDR`] or retrieving the
    /// local address fails.
    ///
    /// [ephemeral port]: https://en.wikipedia.org/wiki/Ephemeral_port
    fn spawn_one_shot_http_server_with_options(
        response_body: String,
        status: String,
        maybe_delay: Option<Duration>,
    ) -> ServerSpawnResult {
        server_common_impl::spawn_ephemeral_server(move |listener, _addr| {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };

            let mut req_buf = [0_u8; blocks::HTTP_REQUEST_BUFFER_SIZE];
            drop(stream.read(&mut req_buf));
            if let Some(delay) = maybe_delay {
                std::thread::sleep(delay);
            }
            let resp = http_response::format_http_response(
                &status,
                blocks::MIME_APPLICATION_JSON,
                &response_body,
            );
            drop(stream.write_all(resp.as_bytes()));
            drop(stream.flush());
        })
    }
}

pub mod archive_server {
    use super::*;

    /// Spawns a local mock HTTP server handling the 4 sequential requests made during a
    /// release-archive upgrade:
    /// 1. `GET /versions.jsonc` → manifest JSONC
    /// 2. `HEAD /releases/download/.../*.tar.gz` → 200 OK header with `Content-Length`
    /// 3. `GET /releases/download/.../*.tar.gz.sha256` → SHA-256 checksum string
    /// 4. `GET /releases/download/.../*.tar.gz` → raw archive `.tar.gz` bytes
    ///
    /// # Panics
    ///
    /// Panics if accepting a connection fails or an unrecognized request is received.
    #[must_use]
    pub fn spawn_success(
        archive_bytes: Vec<u8>,
        arg_archive_sha256: impl Into<String>,
    ) -> ServerSpawnResult {
        let archive_sha256 = arg_archive_sha256.into();
        server_common_impl::spawn_ephemeral_server(move |listener, addr| {
            let release_url = placeholder_release_base_url().with_socket_addr(addr);
            let manifest_body = manifest_upgrade_available_open_source(&release_url);

            // Accept the 4 sequential upgrade requests. Panics if accepting fails
            // or if an unexpected HTTP client request is received. There are no server
            // errors.
            for _ in 0..4 {
                let (mut stream, _addr) = listener
                    .accept()
                    .expect("failed to accept connection on test HTTP server");

                let request_line = read_request_line(&mut stream);
                let request = ArchiveRequest::from_request_line(&request_line);
                let resp =
                    request.respond(&manifest_body, &archive_bytes, &archive_sha256);

                write_response(&mut stream, &resp);
            }
        })
    }

    /// The 4 distinct HTTP requests expected during an archive upgrade flow.
    ///
    /// Each variant corresponds to a sequential step in the release-archive self-upgrade
    /// journey initiated by `r3bl-cmdr`.
    #[derive(Debug, PartialEq, Eq)]
    enum ArchiveRequest {
        /// Manifest discovery (`GET /versions.jsonc`).
        Manifest,
        /// Asset metadata inspection (`HEAD /releases/download/.../*.tar.gz`).
        ArchiveHeader,
        /// Checksum verification (`GET /releases/download/.../*.tar.gz.sha256`).
        Checksum,
        /// Release binary archive payload (`GET /releases/download/.../*.tar.gz`).
        ArchiveDownload,
    }

    impl ArchiveRequest {
        /// Parses an HTTP Request-Line into its corresponding [`ArchiveRequest`] variant.
        ///
        /// In the HTTP/1.1 specification ([RFC 9112 §3]), the Request-Line is the first
        /// line of an incoming request and follows the structure:
        ///
        /// ```text
        /// <METHOD> <REQUEST_TARGET> <HTTP_VERSION>
        /// ```
        ///
        /// This function splits `request_line` by whitespace and pattern-matches the
        /// method and path components to identify the upgrade step:
        ///
        /// | Method | Target Path             | Variant                             | Purpose                   |
        /// | :----- | :---------------------- | :---------------------------------- | :------------------------ |
        /// | `GET`  | [`VERSIONS_JSONC_PATH`] | [`ArchiveRequest::Manifest`]        | Download upgrade manifest |
        /// | `HEAD` | `.../*.tar.gz`          | [`ArchiveRequest::ArchiveHeader`]   | Inspect archive length    |
        /// | `GET`  | `.../*.tar.gz.sha256`   | [`ArchiveRequest::Checksum`]        | Download checksum file    |
        /// | `GET`  | `.../*.tar.gz`          | [`ArchiveRequest::ArchiveDownload`] | Download release archive  |
        ///
        /// # Panics
        ///
        /// Panics if an unrecognized method or path is received by the test server.
        ///
        /// [RFC 9112 §3]: https://www.rfc-editor.org/rfc/rfc9112#section-3
        fn from_request_line(request_line: &str) -> Self {
            let mut iter = request_line.split_whitespace();
            match (iter.next(), iter.next()) {
                // 1. HEAD /.../*.tar.gz
                (Some(HTTP_METHOD_HEAD), Some(path)) => {
                    if path.ends_with(TAR_GZ_EXT) {
                        return Self::ArchiveHeader;
                    }
                    panic!("unrecognized HEAD request path on test HTTP server: {path}")
                }

                // 2. GET /versions.jsonc, /.../*.tar.gz.sha256, or /.../*.tar.gz
                (Some(HTTP_METHOD_GET), Some(path)) => {
                    if path == VERSIONS_JSONC_PATH {
                        return Self::Manifest;
                    }
                    if path.ends_with(SHA256_EXT) {
                        return Self::Checksum;
                    }
                    if path.ends_with(TAR_GZ_EXT) {
                        return Self::ArchiveDownload;
                    }
                    panic!("unrecognized GET request path on test HTTP server: {path}")
                }

                // 3. Anything else is an unrecognized/malformed request
                (method, path) => {
                    panic!(
                        "unrecognized request on test HTTP server: method={method:?}, path={path:?}"
                    )
                }
            }
        }

        /// Generates the wire-format HTTP response bytes for this request variant.
        fn respond(
            &self,
            manifest_body: &str,
            archive_bytes: &[u8],
            archive_sha256: &str,
        ) -> Vec<u8> {
            match self {
                Self::Manifest => http_response::format_http_response(
                    HTTP_200_OK,
                    MIME_APPLICATION_JSON,
                    manifest_body,
                )
                .into_bytes(),
                Self::ArchiveHeader => http_response::format_http_header(
                    HTTP_200_OK,
                    MIME_APPLICATION_OCTET_STREAM,
                    archive_bytes.len(),
                )
                .into_bytes(),
                Self::Checksum => {
                    let sha_body =
                        http_response::format_sha256_checksum_body(archive_sha256);
                    http_response::format_http_response(
                        HTTP_200_OK,
                        MIME_TEXT_PLAIN,
                        &sha_body,
                    )
                    .into_bytes()
                }
                Self::ArchiveDownload => http_response::format_http_binary_response(
                    HTTP_200_OK,
                    MIME_APPLICATION_OCTET_STREAM,
                    archive_bytes,
                ),
            }
        }
    }

    /// Reads incoming request bytes from `stream` and extracts the first line (the HTTP
    /// Request-Line).
    ///
    /// Here is an example of a typical HTTP request:
    /// ```text
    /// GET /versions.jsonc HTTP/1.1\r\n    <─── Line 1 (Kept, Request-Line)
    /// Host: 127.0.0.1:41235\r\n           <─── Line 2 (Thrown away, Header)
    /// User-Agent: reqwest/0.12\r\n        <─── Line 3 (Thrown away, Header)
    /// Accept: */*\r\n                     <─── Line 4 (Thrown away, Header)
    /// Accept-Encoding: gzip, br\r\n       <─── Line 5 (Thrown away, Header)
    /// \r\n                                <─── Line 6 (Thrown away, End of Headers)
    /// ```
    fn read_request_line(stream: &mut TcpStream) -> String {
        let mut request_buffer = [0_u8; HTTP_REQUEST_BUFFER_SIZE];
        let bytes_read = stream.read(&mut request_buffer).unwrap_or(0);
        let raw_request = String::from_utf8_lossy(&request_buffer[..bytes_read]);
        raw_request.lines().next().unwrap_or("").to_string()
    }

    /// Writes the full response byte buffer to `stream` and flushes.
    fn write_response(stream: &mut TcpStream, response: &[u8]) {
        drop(stream.write_all(response));
        drop(stream.flush());
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_archive_request_from_request_line() {
            assert_eq!(
                ArchiveRequest::from_request_line("GET /versions.jsonc HTTP/1.1"),
                ArchiveRequest::Manifest
            );
            assert_eq!(
                ArchiveRequest::from_request_line(
                    "HEAD /releases/download/v9.9.9/r3bl-cmdr-v9.9.9.tar.gz HTTP/1.1"
                ),
                ArchiveRequest::ArchiveHeader
            );
            assert_eq!(
                ArchiveRequest::from_request_line(
                    "GET /releases/download/v9.9.9/r3bl-cmdr-v9.9.9.tar.gz.sha256 HTTP/1.1"
                ),
                ArchiveRequest::Checksum
            );
            assert_eq!(
                ArchiveRequest::from_request_line(
                    "GET /releases/download/v9.9.9/r3bl-cmdr-v9.9.9.tar.gz HTTP/1.1"
                ),
                ArchiveRequest::ArchiveDownload
            );
        }

        #[test]
        #[should_panic(expected = "unrecognized request on test HTTP server")]
        fn test_archive_request_from_request_line_panics_on_unknown_method() {
            let _ = ArchiveRequest::from_request_line("POST /invalid HTTP/1.1");
        }

        #[test]
        #[should_panic(expected = "unrecognized GET request path on test HTTP server")]
        fn test_archive_request_from_request_line_panics_on_unknown_get_path() {
            let _ = ArchiveRequest::from_request_line("GET /unknown/path.txt HTTP/1.1");
        }

        #[test]
        #[should_panic(expected = "unrecognized HEAD request path on test HTTP server")]
        fn test_archive_request_from_request_line_panics_on_unknown_head_path() {
            let _ = ArchiveRequest::from_request_line("HEAD /unknown/path.txt HTTP/1.1");
        }
    }
}

/// Result of spawning a short-lived mock HTTP server.
#[derive(Debug)]
pub struct ServerSpawnResult {
    /// Endpoint URL pointing to `versions.jsonc`. This is needed because the server
    /// binds to a dynamic [ephemeral port] (`127.0.0.1:0`) that tests must target at
    /// runtime.
    ///
    /// [ephemeral port]: https://en.wikipedia.org/wiki/Ephemeral_port
    pub url: EndpointUrl,

    /// Handle to the spawned thread. Joining it ensures clean teardown without leaked
    /// threads/sockets and propagates any server panics to the test thread.
    pub join_handle: std::thread::JoinHandle<()>,
}

mod server_common_impl {
    use super::*;
    // Used for shorter rustdoc link ref defs
    #[allow(unused_imports)]
    use r3bl_tui::core::resilient_reactor_thread::RRT;

    /// Spawns a background thread running `worker_fn` with a [`TcpListener`] bound to an
    /// [ephemeral port] on [`LOCALHOST_EPHEMERAL_PORT_BIND_ADDR`].
    ///
    /// Returns a [`ServerSpawnResult`] containing the synthesized manifest URL and the
    /// thread's [`JoinHandle`].
    ///
    /// # Note on `'static` trait bound
    ///
    /// - The `'static` bound prevents the `worker_fn` closure from capturing borrowed
    ///   references (`&`) to the caller's stack frame. Otherwise, these would dangle if
    ///   the caller returns while the spawned thread is still running. All captured
    ///   variables must be moved by value, making them fully owned by the spawned thread.
    /// - This does not mean the thread or closure lives forever. For an in-depth
    ///   breakdown, see [`RRT`].
    ///
    /// # Panics
    ///
    /// Panics if binding to [`LOCALHOST_EPHEMERAL_PORT_BIND_ADDR`] or retrieving the
    /// local address fails.
    ///
    /// [`JoinHandle`]: std::thread::JoinHandle
    /// [`RRT`]: RRT#static-trait-bound-vs-static-lifetime-annotation
    /// [ephemeral port]: https://en.wikipedia.org/wiki/Ephemeral_port
    #[must_use]
    pub fn spawn_ephemeral_server(
        worker_fn: impl FnOnce(TcpListener, SocketAddr)
        + Send
        // Type contains no references shorter than 'static. This does not
        // mean it lives forever.
        + 'static,
    ) -> ServerSpawnResult {
        // Create a socket listener that binds to an OS assigned port.
        let listener = match TcpListener::bind(blocks::LOCALHOST_EPHEMERAL_PORT_BIND_ADDR)
        {
            Ok(listener) => listener,
            Err(err) => panic!("failed to bind test HTTP server: {err}"),
        };

        // Get the local socket address including the OS assigned port number.
        let addr = match listener.local_addr() {
            Ok(addr) => addr,
            Err(err) => {
                panic!("failed to get local address for test HTTP server: {err}")
            }
        };

        // Synthesize the URL pointing to `versions.jsonc` on the test server.
        let url = EndpointUrl::default_manifest_url().with_socket_addr(addr);

        // Spawn a thread to execute the `worker_fn`.
        let join_handle = std::thread::spawn(move || worker_fn(listener, addr));

        ServerSpawnResult { url, join_handle }
    }
}

mod http_response {
    use super::*;

    /// Formats an HTTP/1.1 response header string with `Content-Type`, `Content-Length`,
    /// and `Connection: close`, terminated by `\r\n\r\n`.
    #[must_use]
    pub fn format_http_header(
        status: &str,
        content_type: &str,
        content_length: usize,
    ) -> String {
        format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {content_length}\r\nConnection: close\r\n\r\n"
        )
    }

    /// Formats an HTTP/1.1 response string with `Content-Type`, `Content-Length`, and
    /// `Connection: close`.
    #[must_use]
    pub fn format_http_response(status: &str, content_type: &str, body: &str) -> String {
        format!(
            "{}{body}",
            format_http_header(status, content_type, body.len())
        )
    }

    /// Formats an HTTP/1.1 response with `Content-Type`, `Content-Length`, and
    /// `Connection: close` for binary payload bytes.
    #[must_use]
    pub fn format_http_binary_response(
        status: &str,
        content_type: &str,
        body: &[u8],
    ) -> Vec<u8> {
        let header = format_http_header(status, content_type, body.len());
        let mut resp = Vec::with_capacity(header.len() + body.len());
        resp.extend_from_slice(header.as_bytes());
        resp.extend_from_slice(body);
        resp
    }

    /// Formats the `.sha256` checksum manifest file contents for the test release
    /// archive.
    #[must_use]
    pub fn format_sha256_checksum_body(archive_sha256: &str) -> String {
        format!("{archive_sha256}  {TEST_ARCHIVE_FILENAME}\n")
    }
}
