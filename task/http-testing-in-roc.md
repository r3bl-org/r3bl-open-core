Task: HTTP Testing in ROC (`MockHttpServer` Archival & `mockito` Migration)

<!-- BEGIN mktoc -->

- [Task: HTTP Testing in ROC (`MockHttpServer` Archival & `mockito` Migration)](#task-http-testing-in-roc-mockhttpserver-archival--mockito-migration)
- [Overview](#overview)
    - [Target Types & Signatures](#target-types--signatures)
        - [Phase 1 (`tui/src/core/test_fixtures/http_server_fixtures.rs`)](#phase-1-tuisrccoretest_fixtureshttp_server_fixturesrs)
        - [Phase 3 (`cmdr/.../test_fixtures/http_server.rs` backed by `mockito`)](#phase-3-cmdrtest_fixtureshttp_serverrs-backed-by-mockito)
        - [Phase 4 (`tui/src/core/script/` URL-configurable helpers)](#phase-4-tuisrccorescript-url-configurable-helpers)
- [Implementation plan](#implementation-plan)
    - [Phase 1: Extract to `r3bl_tui`](#phase-1-extract-to-r3bl_tui)
    - [Phase 2: Archive to `r3bl-open-core-archive`](#phase-2-archive-to-r3bl-open-core-archive)
    - [Phase 3: Migrate `r3bl-cmdr` to `mockito`](#phase-3-migrate-r3bl-cmdr-to-mockito)
    - [Phase 4: Add `mockito` Tests Across ROC](#phase-4-add-mockito-tests-across-roc)

<!-- END mktoc -->

## Overview

`cmdr/src/self_upgrade/upgrade_journey/integration_tests/test_fixtures/http_server.rs`
contains a zero-dependency, `std::net::TcpListener`-based HTTP/1.1 mock server mixed with
`r3bl-cmdr` domain logic (`EndpointUrl`, `manifest_server`, `archive_server`). Meanwhile,
other HTTP modules across the monorepo (`cmdr/src/analytics_client/http_client.rs` and
`tui/src/core/script/{http_client,github_api,crates_api,download}.rs`) either lack HTTP
unit tests entirely or make live internet calls during `cargo test`.

To avoid maintaining a custom HTTP/1.1 server (request body framing, header matching, JSON
diffing) while preserving the valuable `std` implementation for future reference, this
task executes a 4-phase plan:

1. **Extract Reusable `http_server_fixtures` into `r3bl_tui`:** Separate the
   domain-agnostic `std::net::TcpListener` HTTP/1.1 server and response formatters from
   `r3bl-cmdr` into `tui/src/core/test_fixtures/http_server_fixtures.rs` (`MockHttpServer`
   / `HttpServerSpawnResult`), leaving only `self_upgrade` domain wrappers in
   `cmdr/.../test_fixtures/http_server.rs`.
2. **Archive `http_server_fixtures` to `r3bl-open-core-archive`:** Move the clean,
   decoupled `http_server_fixtures.rs` module into the `r3bl_test_fixtures` crate inside
   the archive monorepo
   (`/home/nazmul/github/r3bl-open-core-archive/test_fixtures/src/http_server_fixtures.rs`,
   re-exported from `/home/nazmul/github/r3bl-open-core-archive/test_fixtures/src/lib.rs`
   alongside `input_device_fixtures`, `output_device_fixtures`, and
   `tcp_stream_fixtures`), then remove the temporary re-export from `r3bl_tui`.
3. **Migrate `r3bl-cmdr` `self_upgrade` Tests to `mockito`:** Add `mockito` to
   `[dev-dependencies]` and replace the custom `TcpListener` servers in
   `cmdr/.../test_fixtures/http_server.rs`,
   `cmdr/src/self_upgrade/check/fetch_manifest.rs`, and
   `cmdr/src/self_upgrade/upgrade_journey/typestates/checking.rs` with `mockito::Server`.
4. **Add `mockito` HTTP Tests Across ROC (`analytics_client` & `r3bl_tui::script`):** Add
   deterministic local `mockito` tests for `cmdr/src/analytics_client/http_client.rs` and
   `tui/src/core/script/{http_client,github_api,crates_api,download}.rs`, eliminating live
   calls to `api.github.com`, `crates.io`, and `github.com/cloudflare/cfssl` in
   `cargo test`.

### Target Types & Signatures

#### Phase 1 (`tui/src/core/test_fixtures/http_server_fixtures.rs`)

```rust
/// Result of spawning a short-lived local HTTP/1.1 test server on `127.0.0.1:0`.
#[derive(Debug)]
pub struct HttpServerSpawnResult {
    /// Bound local socket address (`127.0.0.1:<ephemeral_port>`).
    pub addr: SocketAddr,
    /// Base URL (`"http://127.0.0.1:<ephemeral_port>"`) without trailing slash.
    pub base_url: String,
    /// Handle to the spawned worker thread.
    pub join_handle: std::thread::JoinHandle<()>,
}

/// Lightweight builder and spawner for a one-shot or custom `std::net::TcpListener`
/// HTTP/1.1 test server.
#[derive(Debug, Clone)]
pub struct MockHttpServer {
    status: String,
    content_type: String,
    body: Vec<u8>,
    maybe_delay: Option<Duration>,
}

impl MockHttpServer {
    #[must_use]
    pub fn new() -> Self;
    #[must_use]
    pub fn status(self, arg_status: impl Into<String>) -> Self;
    #[must_use]
    pub fn content_type(self, arg_content_type: impl Into<String>) -> Self;
    #[must_use]
    pub fn body(self, arg_body: impl Into<Vec<u8>>) -> Self;
    #[must_use]
    pub fn delay(self, delay: Duration) -> Self;
    #[must_use]
    pub fn spawn_one_shot(self) -> HttpServerSpawnResult;
    #[must_use]
    pub fn spawn_custom(
        worker_fn: impl FnOnce(TcpListener, SocketAddr) + Send + 'static,
    ) -> HttpServerSpawnResult;
}

pub mod http_wire {
    #[must_use]
    pub fn format_http_header(status: &str, content_type: &str, content_length: usize) -> String;
    #[must_use]
    pub fn format_http_response(status: &str, content_type: &str, body: &str) -> String;
    #[must_use]
    pub fn format_http_binary_response(status: &str, content_type: &str, body: &[u8]) -> Vec<u8>;
    pub fn read_request_line(stream: &mut TcpStream) -> String;
    pub fn write_response(stream: &mut TcpStream, response: &[u8]);
}
```

#### Phase 3 (`cmdr/.../test_fixtures/http_server.rs` backed by `mockito`)

```rust
/// Result of spawning a `mockito::Server` configured for `self_upgrade` tests.
#[derive(Debug)]
pub struct ServerSpawnResult {
    /// Endpoint URL pointing to `/versions.jsonc` on the `mockito::Server`.
    pub url: EndpointUrl,
    /// Owned `mockito::Server` RAII guard keeping registered endpoints alive for the
    /// test scope.
    pub server: mockito::Server,
    /// Registered HTTP endpoints verified via
    /// [`ServerSpawnResult::assert_each_endpoint_called_once`].
    pub endpoints: Vec<mockito::Mock>,
}

impl ServerSpawnResult {
    /// Asserts that every registered HTTP endpoint on this server was called once.
    pub fn assert_each_endpoint_called_once(&self) {
        for endpoint in &self.endpoints {
            endpoint.assert();
        }
    }
}
```

#### Phase 4 (`tui/src/core/script/` URL-configurable helpers)

To test `github_api.rs` and `crates_api.rs` against a local `mockito::Server` without
changing their public signatures:

- `tui/src/core/script/github_api.rs`: Extract
  `pub(super) async fn try_get_latest_release_tag_from_url(url: &str) -> miette::Result<String>`
  called by `try_get_latest_release_tag_from_github(org: &str, repo: &str)`.
- `tui/src/core/script/crates_api.rs`: Extract
  `pub(super) async fn try_get_latest_release_version_from_url(url: &str) -> miette::Result<String>`
  called by `try_get_latest_release_version_from_crates_io(crate_name: &str)`.

## Implementation plan

### Phase 1: Extract to `r3bl_tui`

Extract the domain-agnostic HTTP/1.1 test server and wire formatters into
`tui/src/core/test_fixtures/http_server_fixtures.rs` and update `r3bl-cmdr` to consume
them so the reusable module is cleanly separated from `self_upgrade` domain types.

- [x] Create `tui/src/core/test_fixtures/http_server_fixtures.rs` with `MockHttpServer`,
      `HttpServerSpawnResult`, `http_wire` helpers (`format_http_header`,
      `format_http_response`, `format_http_binary_response`, `read_request_line`,
      `write_response`), standard HTTP constants (`LOCALHOST_EPHEMERAL_PORT_BIND_ADDR`,
      `HTTP_REQUEST_BUFFER_SIZE`, `HTTP_METHOD_GET`, `HTTP_METHOD_HEAD`, `HTTP_200_OK`,
      `HTTP_500_INTERNAL_SERVER_ERROR`, `MIME_APPLICATION_JSON`,
      `MIME_APPLICATION_OCTET_STREAM`, `MIME_TEXT_PLAIN`), and unit tests.
- [x] Export `pub mod http_server_fixtures;` and `pub use http_server_fixtures::*;` in
      `tui/src/core/test_fixtures/mod.rs`, add module-level rustdocs explaining why
      `test_fixtures` is compiled unconditionally, rename `retry.rs` ->
      `retry_fixtures.rs`, and shard inline helpers out of `mod.rs` into
      `assertion_fixtures.rs` and `isolated_process_fixtures/process_command_helpers.rs`.
- [x] Update
      `cmdr/src/self_upgrade/upgrade_journey/integration_tests/test_fixtures/http_server.rs`
      and `constants.rs` so `manifest_server` and `archive_server` delegate to
      `r3bl_tui::MockHttpServer` and `r3bl_tui::http_wire`.
- [x] Run `./check.fish --check` to verify both crates compile cleanly.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `tui/src/core/test_fixtures/http_server_fixtures.rs`
    - [x] `tui/src/core/test_fixtures/retry_fixtures.rs`
    - [x] `tui/src/core/test_fixtures/assertion_fixtures.rs`
    - [x] `tui/src/core/test_fixtures/isolated_process_fixtures/process_command_helpers.rs`
    - [x] `tui/src/core/test_fixtures/isolated_process_fixtures/mod.rs`
    - [x] `tui/src/core/test_fixtures/mod.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/test_fixtures/http_server.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/test_fixtures/constants.rs`

### Phase 2: Archive to `r3bl-open-core-archive`

Move the decoupled `http_server_fixtures.rs` into the `r3bl_test_fixtures` crate in
`/home/nazmul/github/r3bl-open-core-archive/test_fixtures/` (which mirrors
`tui/src/core/test_fixtures/` with `input_device_fixtures`, `output_device_fixtures`, and
`tcp_stream_fixtures`) to preserve the zero-dependency `std` implementation, then remove
the temporary export from `r3bl_tui`.

- [x] Copy `tui/src/core/test_fixtures/http_server_fixtures.rs` to
      `/home/nazmul/github/r3bl-open-core-archive/test_fixtures/src/http_server_fixtures.rs`
      (adjusting any `r3bl_tui` doc-link references so it is self-contained inside
      `r3bl_test_fixtures`).
- [x] Attach and re-export `pub mod http_server_fixtures;` and
      `pub use http_server_fixtures::*;` in
      `/home/nazmul/github/r3bl-open-core-archive/test_fixtures/src/lib.rs`.
- [x] Commit `test_fixtures/src/http_server_fixtures.rs` and `test_fixtures/src/lib.rs` in
      `/home/nazmul/github/r3bl-open-core-archive` and push to `origin`.
- [x] Remove `tui/src/core/test_fixtures/http_server_fixtures.rs` and its re-export in
      `tui/src/core/test_fixtures/mod.rs` as part of transitioning `r3bl-open-core` to
      `mockito` in Phase 3.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `/home/nazmul/github/r3bl-open-core-archive/test_fixtures/src/http_server_fixtures.rs`
    - [ ] `/home/nazmul/github/r3bl-open-core-archive/test_fixtures/src/lib.rs`
    - [ ] `tui/src/core/test_fixtures/mod.rs`

### Phase 3: Migrate `r3bl-cmdr` to `mockito`

Replace all custom `TcpListener` HTTP servers in `r3bl-cmdr` (`self_upgrade` unit and
integration tests) with `mockito`.

- [x] Add `mockito = { version = "1.7", default-features = false }` to
      `[dev-dependencies]` in `cmdr/Cargo.toml` and `tui/Cargo.toml`.
- [x] Add `"assert-json-diff"` to the `cargo-unmaintained` ignore list in root
      `Cargo.toml` (and prune stale entries no longer in `Cargo.lock`):

    ```toml
    [workspace.metadata.unmaintained]
    ignore = [
      "gethostname",      # Transitive dependency via copypasta -> x11-clipboard -> x11rb (r3bl_tui).
      "home",             # Transitive dependency via which (r3bl-rust-analyzer-mcp-server).
      "assert-json-diff", # Transitive dev-dependency via mockito (r3bl-cmdr, r3bl_tui).
    ]
    ```

- [x] Rewrite
      `cmdr/src/self_upgrade/upgrade_journey/integration_tests/test_fixtures/http_server.rs`
      to use `mockito::Server::new_with_opts`:
    - `manifest_server::{spawn_success, spawn_error, spawn_delayed_success, spawn_delayed_error}`
      using `.with_body_from_request` for delayed responses.
    - `archive_server::spawn_success` registering the 4 upgrade route mocks
      (`GET /versions.jsonc`, `HEAD .../*.tar.gz`, `GET .../*.tar.gz.sha256`,
      `GET .../*.tar.gz`) on a single `mockito::Server`, backed by
      `TestReleaseAssets::for_current_host()` in `constants.rs` (delegating directly to
      `ReleaseAsset::format_archive_filename` and `ReleaseAsset::format_urls` in
      `release_asset.rs`).
- [x] Update call sites in `immediate_upgrade_tests.rs`, `deferred_upgrade_tests.rs`,
      `deferred_unhappy_path_tests.rs`, `self_replace_test.rs`, and
      `cargo_install_rustup_test.rs` to call
      `server_spawn_result.assert_each_endpoint_called_once()` instead of
      `server.join_handle.join().unwrap()`.
- [x] Replace the inline `serve_one_http_response` `TcpListener` helpers in
      `cmdr/src/self_upgrade/check/fetch_manifest.rs` and
      `cmdr/src/self_upgrade/upgrade_journey/typestates/checking.rs` with `mockito`.
- [x] Run `./check.fish --check` to verify `r3bl-cmdr` compiles cleanly.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `Cargo.toml`
    - [ ] `cmdr/Cargo.toml`
    - [ ] `tui/Cargo.toml`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/test_fixtures/mod.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/test_fixtures/http_server.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/test_fixtures/constants.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/immediate_upgrade_tests.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/deferred_upgrade_tests.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/deferred_unhappy_path_tests.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/self_replace_test.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/cargo_install_rustup_test.rs`
    - [ ] `cmdr/src/self_upgrade/constants.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/download.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/mod.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/perform_upgrade.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/release_asset.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/release_target.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/upgrade_source.rs`
    - [ ] `cmdr/src/self_upgrade/check/fetch_manifest.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/checking.rs`

### Phase 4: Add `mockito` Tests Across ROC

Add deterministic `mockito`-backed unit tests for all remaining HTTP client modules in
`r3bl-cmdr` and `r3bl_tui`, replacing live internet calls in `tui/src/core/script/`.

- [ ] Add `#[cfg(test)] mod tests` to `cmdr/src/analytics_client/http_client.rs` testing:
    - `make_get_request`: `200 OK` returns `Ok(Response)`, `500 Internal Server Error`
      returns `Err` via `error_for_status()`.
    - `make_post_request`: verifies `POST` method, `content-type: application/json`
      header, JSON payload via `mockito::Matcher::Json`, `200 OK` success, and `500` error
      handling.
- [ ] Add `#[cfg(test)] mod tests` to `tui/src/core/script/http_client.rs` testing
      `create_client_with_user_agent(None)` (`User-Agent: scripting.rs/1.0`) and
      `create_client_with_user_agent(Some("custom-agent/2.0"))` against a `mockito` server
      with `.match_header("user-agent", ...)`.
- [ ] Update `tui/src/core/script/download.rs` (`tests_download`) to download from a local
      `mockito::Server` instead of live `github.com/cloudflare/cfssl`, testing initial
      download, file overwrite with updated payload bytes, and HTTP 404/500 error
      propagation.
- [ ] Update `tui/src/core/script/github_api.rs`: extract
      `try_get_latest_release_tag_from_url(url: &str)` and replace the live
      `api.github.com/repos/cloudflare/cfssl/releases/latest` test with `mockito` tests
      covering `"v"` prefix stripping (`"v1.6.5"` -> `"1.6.5"`), missing `"tag_name"` JSON
      field error, and HTTP 500 error.
- [ ] Update `tui/src/core/script/crates_api.rs`: extract
      `try_get_latest_release_version_from_url(url: &str)` and replace the live
      `crates.io/api/v1/crates/serde` test with `mockito` tests covering valid
      `{"crate": {"max_version": "1.0.228"}}` extraction, missing `"max_version"` JSON
      field error, and HTTP 500 error.
- [ ] Run `./check.fish --check` to verify compilation.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `cmdr/src/analytics_client/http_client.rs`
    - [ ] `tui/src/core/script/http_client.rs`
    - [ ] `tui/src/core/script/download.rs`
    - [ ] `tui/src/core/script/github_api.rs`
    - [ ] `tui/src/core/script/crates_api.rs`
