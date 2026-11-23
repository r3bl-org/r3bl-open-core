Task: HTTP Testing in ROC (`MockHttpServer` Archival & `mockito` Migration)

<!-- BEGIN mktoc -->

- [Overview](#overview)
    - [Target Types & Signatures](#target-types--signatures)
        - [Phase 1 (`tui/src/core/test_fixtures/http_server_fixtures.rs`)](#phase-1-tuisrccoretest_fixtureshttp_server_fixturesrs)
        - [Phase 3 (`cmdr/.../test_fixtures/http_server.rs` backed by `mockito`)](#phase-3-cmdrtest_fixtureshttp_serverrs-backed-by-mockito)
        - [Phase 4: `StartGate` & `StartOutcome` Types](#phase-4-startgate--startoutcome-types)
        - [Phase 5: Deferred Exit & `Skipped` Types](#phase-5-deferred-exit--skipped-types)
        - [Phase 6: Cooldown & Test Fixtures](#phase-6-cooldown--test-fixtures)
        - [Phase 7: Script URL Helpers](#phase-7-script-url-helpers)
        - [Phase 8: High-Fidelity Typed Errors (Zero String Squashing)](#phase-8-high-fidelity-typed-errors-zero-string-squashing)
- [Implementation plan](#implementation-plan)
    - [Phase 1: Extract to `r3bl_tui`](#phase-1-extract-to-r3bl_tui)
    - [Phase 2: Archive to `r3bl-open-core-archive`](#phase-2-archive-to-r3bl-open-core-archive)
    - [Phase 3: Migrate `r3bl-cmdr` to `mockito`](#phase-3-migrate-r3bl-cmdr-to-mockito)
    - [Phase 4: Linear `StartOutcome` & `StartGate`](#phase-4-linear-startoutcome--startgate)
    - [Phase 5: Deferred Exit & `Skipped`](#phase-5-deferred-exit--skipped)
    - [Phase 6: Use-Case Integration Tests](#phase-6-use-case-integration-tests)
    - [Phase 7: Add `mockito` Tests Across ROC](#phase-7-add-mockito-tests-across-roc)
    - [Phase 8: Eliminate Lossy Error Squashing Across ROC](#phase-8-eliminate-lossy-error-squashing-across-roc)
        - [Sub-phase 8.1: Typed `r3bl_tui::script` Errors](#sub-phase-81-typed-r3bl_tuiscript-errors)
        - [Sub-phase 8.2: `JsoncParseError`, `FetchManifestError` & `!Eq` Check Outcomes](#sub-phase-82-jsoncparseerror-fetchmanifesterror--eq-check-outcomes)
        - [Sub-phase 8.3: `TokioRuntimeOutcome` & `ThreadRunnerError`](#sub-phase-83-tokioruntimeoutcome--threadrunnererror)
        - [Sub-phase 8.4: `UpgradeError` Overhaul (Delete `Other(String)`)](#sub-phase-84-upgradeerror-overhaul-delete-otherstring)

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
task executes an 8-phase plan:

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
4. **Replace `SINGLETON` with `StartGate` & Linear `StartOutcome`:** Delete
   `ScopedMutex<SingletonState>` (`singleton.rs`), replace the single-start check with a
   lock-free monotonic `StartGate` (`AtomicBoolExt::try_acquire`), move
   `DeferredCheckHandle` directly into `StartOutcome` with by-value
   `handle_upgrade_on_exit(self) -> ExitOutcome` so handle stealing and double-joins are
   compile-time errors, and split `RuntimeOrThreadInitFailed` vs. `*ThreadPanicked`
   variants across `ManifestCheckError` and `UpgradeError`.
5. **Separate Deferred Exit Handling, `PerProcessStartLatch` & `Skipped`:** Rename
   `StartGate` (`start_gate.rs`) -> `PerProcessStartLatch` (`per_process_start_latch.rs`)
   with `claim()` (documenting why `reset_for_testing` exists on the monotonic per-process
   latch), remove internal barrel re-exports of `per_process_start_latch` and
   `thread_runner` from `upgrade_journey/mod.rs` so they stay contained in their modules,
   rename `CooldownSkipped` -> `Skipped` (`cooldown_skipped.rs` -> `skipped.rs`,
   `into_cooldown_skipped` -> `into_skipped`), move
   `handle_upgrade_on_exit(self) -> ExitOutcome` off `StartOutcome` onto
   `DeferredUpgradeOutcome` so calling exit handling on `ImmediateUpgrade`, `Disabled`, or
   `AlreadyStarted` is unrepresentable, and replace `ExitOutcome::NoDeferredCheckRunning`
   with `ExitOutcome::Skipped(UpgradeJourney<Skipped>)`.
6. **Restructure `self_upgrade` Integration Tests Around Use Cases:** Reorganize
   `cmdr/src/self_upgrade/upgrade_journey/integration_tests/` into a 5-pillar, 9-file
   use-case-driven suite with sync/async caller parity on in-process tests, domain-level
   cooldown test helpers, negative `archive_server` fixtures, and complete happy/unhappy
   path coverage across `Immediate`, `Deferred`, `self_replace`, and `install_via_rustup`.
7. **Add `mockito` HTTP Tests Across ROC (`analytics_client` & `r3bl_tui::script`):** Add
   deterministic local `mockito` tests for `cmdr/src/analytics_client/http_client.rs` and
   `tui/src/core/script/{http_client,github_api,crates_api,download}.rs`, eliminating live
   calls to `api.github.com`, `crates.io`, and `github.com/cloudflare/cfssl` in
   `cargo test`.
8. **Eliminate Lossy Error Squashing Across ROC (`HttpClientError`, `JsoncParseError`,
   `FetchManifestError`, `ThreadRunnerError`, `UpgradeError`):** Replace all lossy
   `format!("{err:#}")` / `.to_string()` string-squashed error fields and `miette::Report`
   enum payloads across `r3bl_tui` (`http_client`, `temp_dir`, `permissions`,
   `TokioRuntimeOutcome`) and `r3bl-cmdr` (`JsoncManifest::try_parse_jsonc`,
   `FetchManifestError`, `ThreadRunnerError`, `UpgradeError`) with high-fidelity typed
   `thiserror::Error` + `miette::Diagnostic` enums preserving real `#[source]` chains.

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

#### Phase 4: `StartGate` & `StartOutcome` Types

Target signatures in `start_gate.rs`, `exit_outcome.rs`, `start_outcome.rs`,
`fetch_manifest.rs`, and `perform_upgrade.rs` (replacing deleted `singleton.rs`):

```rust
// cmdr/src/self_upgrade/upgrade_journey/start_gate.rs
static START_GATE: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::self_upgrade) enum StartClaimStatus {
    Claimed,
    AlreadyStarted,
}

#[derive(Debug)]
pub struct StartGate;

impl StartGate {
    #[must_use]
    pub(in crate::self_upgrade) fn try_claim() -> StartClaimStatus; // via AtomicBoolExt::try_acquire (monotonic per-process latch)

    #[cfg(any(test, doc))]
    pub(crate) fn reset_for_testing(); // via AtomicBoolExt::release
}

// cmdr/src/self_upgrade/upgrade_journey/exit_outcome.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitOutcome {
    NoDeferredCheckRunning,
    UpToDate(UpgradeJourney<UpToDate>),
    CheckFailed(UpgradeJourney<CheckFailed>),
    Completed(UpgradeJourney<Completed>),
}

// cmdr/src/self_upgrade/upgrade_journey/start_outcome.rs
#[derive(Debug)]
pub struct DeferredCheckHandle {
    pub(in crate::self_upgrade) checking: UpgradeJourney<Checking>,
    pub(in crate::self_upgrade) join_handle:
        StdThreadTokioJoinHandle<DeferredCheckOutcome>,
}

impl DeferredCheckHandle {
    #[must_use]
    pub fn get_journey(&self) -> &UpgradeJourney<Checking>;
}

#[must_use = "call .handle_upgrade_on_exit() at shutdown (or exit early on ImmediateUpgrade)"]
#[derive(Debug)]
pub enum StartOutcome {
    Disabled(UpgradeJourney<Disabled>),
    ImmediateUpgrade(ImmediateUpgradeOutcome),
    DeferredUpgrade(DeferredUpgradeOutcome),
    AlreadyStarted,
}

#[derive(Debug)]
pub enum DeferredUpgradeOutcome {
    CooldownSkipped(UpgradeJourney<CooldownSkipped>),
    DeferredCheckSpawned(DeferredCheckHandle),
    DeferredCheckSpawnFailed(UpgradeJourney<CheckFailed>),
}

impl StartOutcome {
    #[must_use]
    pub fn handle_upgrade_on_exit(self) -> ExitOutcome;
}

// cmdr/src/self_upgrade/check/fetch_manifest.rs (ManifestCheckError additions)
pub enum ManifestCheckError {
    // ... existing CrateNotFoundInManifest, FetchOrParseFailed ...
    #[error("{reason}")]
    RuntimeOrThreadInitFailed { reason: String },
    #[error("{panic_msg}")]
    CheckThreadPanicked { panic_msg: String },
}

// cmdr/src/self_upgrade/upgrade/perform_upgrade.rs (UpgradeError additions)
pub enum UpgradeError {
    // ... existing NoViableUpgradeSource, UnsupportedPlatform, ChecksumMismatch, Cancelled, Other ...
    #[error("{reason}")]
    RuntimeOrThreadInitFailed { reason: String },
    #[error("{panic_msg}")]
    UpgradeThreadPanicked { panic_msg: String },
}
```

#### Phase 5: Deferred Exit & `Skipped` Types

Target signatures in `per_process_start_latch.rs`, `thread_runner.rs`,
`upgrade_journey/mod.rs`, `typestates/skipped.rs`, `start_outcome.rs`, and
`exit_outcome.rs`:

```rust
// cmdr/src/self_upgrade/upgrade_journey/per_process_start_latch.rs
static PER_PROCESS_START_LATCH: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartClaimStatus {
    Claimed,
    AlreadyStarted,
}

#[derive(Debug)]
pub struct PerProcessStartLatch;

impl PerProcessStartLatch {
    #[must_use]
    pub fn claim() -> StartClaimStatus;

    #[cfg(any(test, doc))]
    pub fn reset_for_testing();
}

// cmdr/src/self_upgrade/upgrade_journey/mod.rs
mod exit_outcome;
pub(super) mod per_process_start_latch; // Rustdoc links from self_upgrade/mod.rs.
mod start_outcome;
mod thread_runner;
mod typestates;
mod upgrade_journey_macro;

pub use exit_outcome::*;
pub use start_outcome::*;
pub use typestates::*;

// cmdr/src/self_upgrade/upgrade_journey/typestates/skipped.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Skipped;
impl StateMarker for Skipped {}

// cmdr/src/self_upgrade/upgrade_journey/start_outcome.rs
#[must_use = "match ImmediateUpgrade to exit early, or hold DeferredUpgrade to call .handle_upgrade_on_exit() at shutdown"]
#[derive(Debug)]
pub enum StartOutcome {
    Disabled(UpgradeJourney<Disabled>),
    ImmediateUpgrade(ImmediateUpgradeOutcome),
    DeferredUpgrade(DeferredUpgradeOutcome),
    AlreadyStarted,
}

#[must_use = "call .handle_upgrade_on_exit() at application shutdown"]
#[derive(Debug)]
pub enum DeferredUpgradeOutcome {
    Skipped(UpgradeJourney<Skipped>),
    DeferredCheckSpawned(DeferredCheckHandle),
    DeferredCheckSpawnFailed(UpgradeJourney<CheckFailed>),
}

impl DeferredUpgradeOutcome {
    #[must_use]
    pub fn handle_upgrade_on_exit(self) -> ExitOutcome;
}

// cmdr/src/self_upgrade/upgrade_journey/exit_outcome.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitOutcome {
    Skipped(UpgradeJourney<Skipped>),
    UpToDate(UpgradeJourney<UpToDate>),
    CheckFailed(UpgradeJourney<CheckFailed>),
    Completed(UpgradeJourney<Completed>),
}
```

#### Phase 6: Cooldown & Test Fixtures

Target helpers for `remote_version_check_gate.rs`, `jsonc_golden.rs`, `http_server.rs`,
and the 9-file `integration_tests/mod.rs` layout:

- `cmdr/src/self_upgrade/check/remote_version_check_gate.rs`:

```rust
#[cfg(any(test, doc))]
impl RemoteVersionCheckGate {
    /// Creates the cooldown timestamp file and backdates its `mtime` past the
    /// configured cooldown window so `try_check_gate_status` returns
    /// `Ok(GateStatus::Open)`.
    pub fn backdate_timestamp_file_for_testing(&self);
}
```

- `cmdr/src/self_upgrade/upgrade_journey/integration_tests/test_fixtures/jsonc_golden.rs`
  & `http_server.rs`:

```rust
// jsonc_golden.rs
#[must_use]
pub fn manifest_without_crate(other_crate_name: &str) -> String;

// http_server.rs (manifest_server)
#[must_use]
pub fn spawn_uncalled() -> ServerSpawnResult; // .expect(0)

// http_server.rs (archive_server)
#[must_use]
pub fn spawn_closed_source_success(archive_bytes: Vec<u8>) -> ServerSpawnResult;
#[must_use]
pub fn spawn_checksum_mismatch_closed_source(
    archive_bytes: Vec<u8>,
    wrong_sha256: Sha256Digest,
) -> ServerSpawnResult;
#[must_use]
pub fn spawn_archive_get_error(
    distribution: CrateDistribution,
    status: usize,
) -> ServerSpawnResult;
```

- `cmdr/src/self_upgrade/upgrade_journey/integration_tests/mod.rs` 9-file coordinator
  layout:

```rust
pub mod test_fixtures;

#[cfg(test)]
pub mod start_latch_and_disabled;
#[cfg(test)]
pub mod immediate_upgrade_happy_paths;
#[cfg(test)]
pub mod immediate_upgrade_unhappy_paths;
#[cfg(test)]
pub mod deferred_upgrade_happy_paths;
#[cfg(test)]
pub mod deferred_upgrade_unhappy_paths;
#[cfg(test)]
pub mod self_replace_happy_paths;
#[cfg(test)]
pub mod self_replace_unhappy_paths;
#[cfg(test)]
pub mod install_via_rustup_happy_paths;
#[cfg(test)]
pub mod install_via_rustup_unhappy_paths;
```

#### Phase 7: Script URL Helpers

To test `github_api.rs` and `crates_api.rs` against a local `mockito::Server` without
changing their public signatures:

- `tui/src/core/script/github_api.rs`: Extract
  `pub(super) async fn try_get_latest_release_tag_from_url(url: &str) -> miette::Result<String>`
  called by `try_get_latest_release_tag_from_github(org: &str, repo: &str)`.
- `tui/src/core/script/crates_api.rs`: Extract
  `pub(super) async fn try_get_latest_release_version_from_url(url: &str) -> miette::Result<String>`
  called by `try_get_latest_release_version_from_crates_io(crate_name: &str)`.

#### Phase 8: High-Fidelity Typed Errors (Zero String Squashing)

Following the repo's **High-Fidelity Error Handling** guidelines (`design-philosophy` #5:
never squash `miette::Report` or `std::error::Error` into `String`), drop `Clone,
PartialEq, Eq` on error-carrying typestates/outcomes (`CheckFailed`, `Completed`,
`DeferredCheckOutcome`, `ImmediateCheckOutcome`, `ImmediateUpgradeOutcome`, `ExitOutcome`)
and replace every string-squashed or `miette::Report`-wrapped error with typed
`#[derive(Debug, thiserror::Error, miette::Diagnostic)]` definitions using `#[source]` and
`#[diagnostic(transparent)]`:

```rust
// tui/src/core/script/http_client.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("Failed to create HTTP client")]
pub struct HttpClientError(#[from] pub reqwest::Error);

pub fn create_http_client() -> Result<reqwest::Client, HttpClientError>;

// tui/src/core/script/temp_dir.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("Failed to create temporary directory '{}'", path.display())]
pub struct TempDirError {
    pub path: PathBuf,
    #[source]
    pub src_err: std::io::Error,
}

pub fn try_create_temp_dir() -> Result<TempDir, TempDirError>;

// tui/src/core/script/permissions.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum FilePermissionsError {
    #[error("Failed to read metadata for '{}'", path.display())]
    MetadataReadFailed {
        path: PathBuf,
        #[source]
        src_err: std::io::Error,
    },

    #[error("Path is not a regular file: '{}'", path.display())]
    NotARegularFile {
        path: PathBuf,
    },

    #[error("Failed to set permissions ({mode:#o}) on '{}'", path.display())]
    SetPermissionsFailed {
        path: PathBuf,
        mode: u32,
        #[source]
        src_err: std::io::Error,
    },
}

pub fn set_permission(file: impl AsRef<Path>, mode: u32) -> Result<(), FilePermissionsError>;
pub fn try_set_file_executable(file: impl AsRef<Path>) -> Result<(), FilePermissionsError>;

// tui/src/core/common/std_thread_tokio_runner.rs
#[must_use]
#[derive(Debug)]
pub enum TokioRuntimeOutcome<T> {
    Completed(T),
    FailedToStart(std::io::Error),
    Panicked {
        thread_name: &'static str,
        panic_msg: String,
    },
}

// cmdr/src/self_upgrade/jsonc_manifest/jsonc_manifest_struct.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum JsoncParseError {
    #[error("Invalid JSONC syntax in manifest")]
    InvalidSyntax(#[from] jsonc_parser::errors::ParseError),

    #[error("Manifest does not match expected schema")]
    InvalidSchema(#[from] serde_json::Error),
}

impl JsoncManifest {
    pub fn try_parse_jsonc(jsonc_str: &str) -> Result<Self, JsoncParseError>;
}

// cmdr/src/self_upgrade/check/fetch_manifest.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum FetchManifestError {
    #[error("Failed to create HTTP client for manifest '{manifest_url}'")]
    HttpClientCreateFailed {
        manifest_url: EndpointUrl,
        #[source]
        #[diagnostic_source]
        src_err: HttpClientError,
    },

    #[error("Failed to fetch manifest from '{manifest_url}'")]
    HttpRequestFailed {
        manifest_url: EndpointUrl,
        #[source]
        src_err: reqwest::Error,
    },

    #[error("Failed to parse manifest from '{manifest_url}'")]
    JsoncParseFailed {
        manifest_url: EndpointUrl,
        #[source]
        #[diagnostic_source]
        src_err: JsoncParseError,
    },

    #[error("Crate '{crate_name}' not found in manifest at '{manifest_url}'")]
    CrateNotFoundInManifest {
        crate_name: CrateName,
        manifest_url: EndpointUrl,
    },
}

// cmdr/src/self_upgrade/upgrade_journey/thread_runner.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum ThreadRunnerError {
    #[error("Failed to spawn dedicated self-upgrade OS thread")]
    ThreadSpawnFailed {
        #[source]
        src_err: std::io::Error,
    },

    #[error("Failed to initialize Tokio runtime on self-upgrade OS thread")]
    RuntimeInitFailed {
        #[source]
        src_err: std::io::Error,
    },

    #[error("{thread_name} panicked: {panic_msg}")]
    ThreadPanicked {
        thread_name: &'static str,
        panic_msg: String,
    },
}

// cmdr/src/self_upgrade/upgrade/perform_upgrade.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum UpgradeError {
    #[error(
        "No viable upgrade source for '{crate_name}' (distribution={crate_distribution}): {reason}"
    )]
    NoViableUpgradeSource {
        crate_name: CrateName,
        crate_distribution: CrateDistribution,
        reason: NoViableSourceReason,
    },

    #[error("Unsupported host target platform: {0}")]
    UnsupportedPlatform(String),

    #[error("SHA-256 checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch {
        expected: Sha256Digest,
        actual: Sha256Digest,
    },

    #[error("Upgrade cancelled by user")]
    Cancelled,

    #[error(transparent)]
    #[diagnostic(transparent)]
    HttpClientCreateFailed(#[from] HttpClientError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    TempDirCreateFailed(#[from] TempDirError),

    #[error("Failed to download release asset from '{url}'")]
    DownloadHttpFailed {
        url: String,
        #[source]
        src_err: reqwest::Error,
    },

    #[error("Invalid .sha256 file format: {raw_text}")]
    InvalidSha256FileFormat {
        raw_text: String,
    },

    #[error("File I/O failed at '{}'", path.display())]
    FileIo {
        path: PathBuf,
        #[source]
        src_err: std::io::Error,
    },

    #[error("Failed to extract archive '{}'", archive_path.display())]
    ArchiveExtractFailed {
        archive_path: PathBuf,
        #[source]
        src_err: std::io::Error,
    },

    #[error("Downloaded archive '{}' contained no files", archive_path.display())]
    EmptyArchive {
        archive_path: PathBuf,
    },

    #[error(transparent)]
    #[diagnostic(transparent)]
    SetExecutableFailed(#[from] FilePermissionsError),

    #[error("Current executable path '{}' has no valid file name or parent directory", exe_path.display())]
    InvalidCurrentExePath {
        exe_path: PathBuf,
    },

    #[error("Failed to replace running executable via self_replace")]
    BinarySwapFailed {
        #[source]
        src_err: std::io::Error,
    },

    #[error("cargo install exited with status {status}")]
    SourceBuildExitedNonZero {
        status: ExitStatus,
    },

    #[error("Source build PTY or process I/O failed")]
    SourceBuildIo {
        #[source]
        src_err: std::io::Error,
    },
}

// Control Plane wrappers (CheckFailedError & UpgradeFailedError)
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum CheckFailedError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    FetchManifest(#[from] FetchManifestError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    ThreadRunner(#[from] ThreadRunnerError),
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum UpgradeFailedError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    PerformUpgrade(#[from] UpgradeError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    ThreadRunner(#[from] ThreadRunnerError),
}
```

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
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `/home/nazmul/github/r3bl-open-core-archive/test_fixtures/src/http_server_fixtures.rs`
    - [x] `/home/nazmul/github/r3bl-open-core-archive/test_fixtures/src/lib.rs`
    - [x] `tui/src/core/test_fixtures/mod.rs`

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
    - [x] `Cargo.toml`
    - [x] `cmdr/Cargo.toml`
    - [x] `tui/Cargo.toml`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/test_fixtures/mod.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/test_fixtures/http_server.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/test_fixtures/constants.rs`
    - [x] `cmdr/src/self_upgrade/upgrade/sha256_digest.rs`
    - [x] `tui/src/core/common/fast_strings/hex_slice.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/mod.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/immediate_upgrade_tests.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/deferred_upgrade_tests.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/deferred_unhappy_path_tests.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/self_replace_test.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/cargo_install_rustup_test.rs`
    - [x] `cmdr/src/self_upgrade/constants.rs`
    - [x] `cmdr/src/self_upgrade/upgrade/download.rs`
    - [x] `cmdr/src/self_upgrade/upgrade/mod.rs`
    - [x] `cmdr/src/self_upgrade/upgrade/perform_upgrade.rs`
    - [x] `cmdr/src/self_upgrade/upgrade/release_asset.rs`
    - [x] `cmdr/src/self_upgrade/upgrade/release_target.rs`
    - [x] `cmdr/src/self_upgrade/upgrade/upgrade_source.rs`
    - [x] `cmdr/src/self_upgrade/check/fetch_manifest.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/typestates/checking.rs`

### Phase 4: Linear `StartOutcome` & `StartGate`

Replace the `ScopedMutex<SingletonState>` process-wide `SINGLETON` with a lock-free
monotonic `StartGate` (`AtomicBoolExt::try_acquire`) and move `DeferredCheckHandle`
directly into `StartOutcome` (`handle_upgrade_on_exit(self) -> ExitOutcome`) so handle
stealing and duplicate joins are unrepresentable at compile time while preserving full
thread/runtime init and panic diagnostics.

- [x] Add `ManifestCheckError::CheckThreadPanicked { panic_msg: String }` alongside
      `RuntimeOrThreadInitFailed` in `cmdr/src/self_upgrade/check/fetch_manifest.rs`, and
      add symmetric `UpgradeError::RuntimeOrThreadInitFailed { reason: String }` and
      `UpgradeError::UpgradeThreadPanicked { panic_msg: String }` in
      `cmdr/src/self_upgrade/upgrade/perform_upgrade.rs` (removing stale `SINGLETON` doc
      links in both files).
- [x] Delete `cmdr/src/self_upgrade/upgrade_journey/singleton.rs`, create
      `cmdr/src/self_upgrade/upgrade_journey/start_gate.rs` (`StartGate::try_claim` via
      `AtomicBoolExt::try_acquire`, monotonic per-process latch not reset in production,
      plus `#[cfg(any(test, doc))] StartGate::reset_for_testing` and unit test), create
      `cmdr/src/self_upgrade/upgrade_journey/exit_outcome.rs` (`ExitOutcome`), and update
      `cmdr/src/self_upgrade/upgrade_journey/mod.rs`.
- [x] Update `cmdr/src/self_upgrade/upgrade_journey/start_outcome.rs` (preserving existing
      doc-link edits): define `DeferredCheckHandle` (`checking` + `join_handle` +
      `get_journey()`), drop `Clone, PartialEq, Eq` on `StartOutcome` and
      `DeferredUpgradeOutcome` (`#[must_use]` on `StartOutcome`), change
      `handle_upgrade_on_exit(self) -> ExitOutcome` to consume `self` by value (mapping
      `DeferredCheckSpawnFailed` -> `ExitOutcome::CheckFailed`,
      `TokioRuntimeOutcome::FailedToStart` ->
      `ExitOutcome::CheckFailed(RuntimeOrThreadInitFailed)`, and
      `TokioRuntimeOutcome::Panicked` -> `ExitOutcome::CheckFailed(CheckThreadPanicked)`),
      and add unit tests for linear exit handling.
- [x] Update `cmdr/src/self_upgrade/upgrade_journey/typestates/not_started.rs`
      (`StartGate`, `DeferredCheckHandle` construction, `FailedToStart` vs. `Panicked`
      split in `run_immediate_upgrade`, and `# Thread Model` diagrams),
      `cmdr/src/self_upgrade/upgrade_journey/typestates/upgrading.rs` (`FailedToStart` vs.
      `Panicked` split into `UpgradeError`),
      `cmdr/src/self_upgrade/upgrade_journey/typestates/state_machine.rs`, and
      `cmdr/src/self_upgrade/mod.rs` to remove `SINGLETON` and `SingletonState`.
- [x] Update existing integration tests (`start_guard_and_disabled_tests.rs`,
      `immediate_upgrade_tests.rs`, `deferred_upgrade_tests.rs`,
      `deferred_unhappy_path_tests.rs`, `self_replace_test.rs`,
      `cargo_install_rustup_test.rs`, and `integration_tests/mod.rs`) to use
      `StartGate::reset_for_testing()`, pattern-match `!Eq` `StartOutcome`, verify
      `StartOutcome::AlreadyStarted.handle_upgrade_on_exit()` cannot steal an in-flight
      `DeferredCheckSpawned` handle, and assert
      `ExitOutcome::CheckFailed(CheckThreadPanicked)` with preserved panic payload in the
      panic test.
- [x] Run `./check.fish --check`, `./check.fish --fmt`, and targeted
      `cargo test -p r3bl-cmdr -q self_upgrade` to verify compilation, formatting, and all
      unit/integration tests.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `cmdr/src/self_upgrade/check/fetch_manifest.rs`
    - [x] `cmdr/src/self_upgrade/upgrade/perform_upgrade.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/start_gate.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/exit_outcome.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/start_outcome.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/mod.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/typestates/not_started.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/typestates/upgrading.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/typestates/state_machine.rs`
    - [x] `cmdr/src/self_upgrade/mod.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/mod.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/start_guard_and_disabled_tests.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/immediate_upgrade_tests.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/deferred_upgrade_tests.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/deferred_unhappy_path_tests.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/self_replace_test.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/cargo_install_rustup_test.rs`

### Phase 5: Deferred Exit & `Skipped`

Separate immediate/terminal startup outcomes from deferred exit handling so calling
`.handle_upgrade_on_exit()` on `ImmediateUpgrade`, `Disabled`, or `AlreadyStarted` is a
compile-time error, rename `StartGate` -> `PerProcessStartLatch` (`claim()`), remove
internal barrel re-exports for `per_process_start_latch` and `thread_runner`, and rename
`CooldownSkipped` -> `Skipped` while preserving every terminal `UpgradeJourney<State>`
witness in `ExitOutcome`.

- [x] Rename `StartGate::try_claim()` -> `StartGate::claim()` in
      `cmdr/src/self_upgrade/upgrade_journey/start_gate.rs` and
      `cmdr/src/self_upgrade/upgrade_journey/typestates/not_started.rs`, and expand
      `StartGate::reset_for_testing()` rustdoc explaining why `cargo test`'s shared OS
      process requires resetting the monotonic per-process latch between `#[serial]`
      tests.
- [x] Rename `CooldownSkipped` -> `Skipped` across the typestate machine: `git mv`
      `cmdr/src/self_upgrade/upgrade_journey/typestates/cooldown_skipped.rs` ->
      `skipped.rs`, update `cmdr/src/self_upgrade/upgrade_journey/typestates/mod.rs`, and
      rename `into_cooldown_skipped` -> `into_skipped` in
      `cmdr/src/self_upgrade/upgrade_journey/typestates/not_started.rs`.
- [x] Move `handle_upgrade_on_exit(self) -> ExitOutcome` off `StartOutcome` and onto
      `DeferredUpgradeOutcome` in `cmdr/src/self_upgrade/upgrade_journey/start_outcome.rs`
      (adding `#[must_use]` on `DeferredUpgradeOutcome` and updating `StartOutcome`'s
      `#[must_use]` message), and rename
      `DeferredUpgradeOutcome::CooldownSkipped(UpgradeJourney<CooldownSkipped>)` ->
      `DeferredUpgradeOutcome::Skipped(UpgradeJourney<Skipped>)`.
- [x] Replace `ExitOutcome::NoDeferredCheckRunning` with
      `ExitOutcome::Skipped(UpgradeJourney<Skipped>)` in
      `cmdr/src/self_upgrade/upgrade_journey/exit_outcome.rs` so every `ExitOutcome`
      variant carries a lossless `UpgradeJourney<State>` witness.
- [x] Update callers, unit/integration tests, and rustdoc diagrams/intra-doc links across
      `cmdr/src/self_upgrade/` (`start_outcome.rs` unit tests,
      `start_guard_and_disabled_tests.rs`, `deferred_upgrade_tests.rs`,
      `deferred_unhappy_path_tests.rs`, `typestates/state_machine.rs`,
      `typestates/not_started.rs`, `typestates/checking.rs`,
      `typestates/upgrade_available.rs`, `check/cooldown.rs`, `config/upgrade_policy.rs`,
      `upgrade/perform_upgrade.rs`, `upgrade/prompt.rs`, `integration_tests/mod.rs`, and
      `self_upgrade/mod.rs`).
- [x] Rename `start_gate.rs` -> `per_process_start_latch.rs` (`StartGate` ->
      `PerProcessStartLatch`, `START_GATE` -> `PER_PROCESS_START_LATCH`), drop internal
      barrel re-exports (`per_process_start_latch`, `thread_runner`) from
      `cmdr/src/self_upgrade/upgrade_journey/mod.rs` using
      `pub(super) mod per_process_start_latch;` + `mod thread_runner;` with plain `pub`
      items inside, and update direct call-site `use` imports and rustdoc links across
      `typestates/`, `integration_tests/`, `start_outcome.rs`, and `self_upgrade/mod.rs`.
- [x] Run `./check.fish --check` to verify `r3bl-cmdr` compiles cleanly.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `cmdr/src/self_upgrade/upgrade_journey/per_process_start_latch.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/thread_runner.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/mod.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/exit_outcome.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/start_outcome.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/typestates/skipped.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/typestates/mod.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/typestates/not_started.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/typestates/upgrading.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/typestates/state_machine.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/typestates/checking.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/typestates/upgrade_available.rs`
    - [x] `cmdr/src/self_upgrade/check/cooldown.rs`
    - [x] `cmdr/src/self_upgrade/config/upgrade_policy.rs`
    - [x] `cmdr/src/self_upgrade/upgrade/perform_upgrade.rs`
    - [x] `cmdr/src/self_upgrade/upgrade/prompt.rs`
    - [x] `cmdr/src/self_upgrade/mod.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/mod.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/start_guard_and_disabled_tests.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/immediate_upgrade_tests.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/deferred_upgrade_tests.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/deferred_unhappy_path_tests.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/self_replace_test.rs`
    - [x] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/cargo_install_rustup_test.rs`

### Phase 6: Use-Case Integration Tests

Restructure `cmdr/src/self_upgrade/upgrade_journey/integration_tests/` into a 5-pillar,
9-file use-case-driven integration test suite building on Phase 4–5's
`PerProcessStartLatch` and linear `DeferredUpgradeOutcome` ownership, with complete
happy/unhappy path coverage. Because `per_process_start_latch` and `thread_runner` are not
barrel-exported from `upgrade_journey/mod.rs`, every test file (including newly added
`*_unhappy_paths.rs` files) imports
`super::super::per_process_start_latch::PerProcessStartLatch` (and
`super::super::thread_runner::ThreadRunner` when needed) directly.

- [x] Add `RemoteVersionCheckGate::backdate_timestamp_file_for_testing`,
      `GateStatusError`, and
      `try_check_gate_status() -> Result<GateStatus, GateStatusError>` (with
      `GateStatus::{ClosedForCooldown, Open}` and `try_touch_timestamp_file`) in
      `cmdr/src/self_upgrade/check/remote_version_check_gate.rs` (eliminating raw
      `File::set_modified` boilerplate from integration tests, with Control Plane
      fail-open logging in `not_started.rs`) and move the Execution Plane timeout unit
      test
      (`test_fetch_crate_metadata_from_manifest_timeout_returns_fetch_or_parse_failed`)
      into `cmdr/src/self_upgrade/check/fetch_manifest.rs`.
- [x] Extend
      `cmdr/src/self_upgrade/upgrade_journey/integration_tests/test_fixtures/{jsonc_golden,http_server}.rs`
      with `manifest_without_crate`, `manifest_server::spawn_uncalled` (`.expect(0)`), and
      `archive_server` negative helpers (`spawn_closed_source_success`,
      `spawn_checksum_mismatch_closed_source`, `spawn_archive_get_error`).
- [x] Rewrite module-level rustdoc and module list in
      `cmdr/src/self_upgrade/upgrade_journey/integration_tests/mod.rs` documenting: (a)
      why integration tests exist alongside sibling unit tests (Control Plane + Execution
      Plane E2E wiring), (b) the 5 use-case groups / 9-file layout (in-process vs.
      subprocess Coordinator/Worker isolation), (c) the sync (`#[test]`) + async
      (`#[tokio::test]`) `scenario_<name>()` wrapper policy for in-process files (matching
      `fn main()` vs. `#[tokio::main]` binaries), and (d) the ASCII cooldown interaction
      flow across `Immediate` and `Deferred`.
- [x] Implement Group 1 in `start_latch_and_disabled.rs` (`git mv` from
      `start_guard_and_disabled_tests.rs`), each scenario with `_sync` + `_async` wrappers
      and explicit `Arrange -> Act -> Assert` outcome bindings:
    - `UpgradePolicy::Disabled` returns `StartOutcome::Disabled` (`0` HTTP calls via
      `spawn_uncalled`).
    - 1st `Disabled` -> duplicate `Immediate` and duplicate `Deferred` both return
      `StartOutcome::AlreadyStarted` (`0` HTTP calls).
    - 1st `Immediate` -> duplicate `Immediate` and duplicate `Deferred` both return
      `StartOutcome::AlreadyStarted`.
    - 1st `Deferred` (in-flight `50ms` check returning
      `StartOutcome::DeferredUpgrade(deferred)`) -> duplicate `Immediate` and duplicate
      `Deferred` return `StartOutcome::AlreadyStarted` (carrying no
      `DeferredUpgradeOutcome`, so handle stealing is unrepresentable), and consuming the
      1st `deferred.handle_upgrade_on_exit()` still joins and returns
      `ExitOutcome::UpToDate`.
- [x] Implement Group 2 in `immediate_upgrade_happy_paths.rs` (`git mv` from
      `immediate_upgrade_tests.rs`) and `immediate_upgrade_unhappy_paths.rs` (1 use case
      per test, `_sync` + `_async` wrappers, cross-linking `self_replace_happy_paths` and
      `install_via_rustup_happy_paths` for `remote > current` installs):
    - Happy paths: `remote == local` (`0.0.25`) -> `UpToDate` +
      `GateStatus::ClosedForCooldown`; `local > remote` (`0.0.26 > 0.0.25`, no downgrade)
      -> `UpToDate` + `GateStatus::ClosedForCooldown`; active cooldown
      (`try_touch_timestamp_file`) bypassed by `--upgrade` -> hits server + `UpToDate` +
      `GateStatus::ClosedForCooldown`.
    - Unhappy paths (all leaving `GateStatusError::MetadataReadFailed (NotFound)`):
      manifest HTTP 500 -> `CheckFailed(FetchOrParseFailed)`; manifest connection refused
      -> `CheckFailed(FetchOrParseFailed)`; crate missing from manifest ->
      `CheckFailed(CrateNotFoundInManifest)`; `ClosedSource` `9.9.9` with unreachable
      archive -> `Completed(UpgradeFailed(NoViableUpgradeSource))`.
- [x] Implement Group 3 in `deferred_upgrade_happy_paths.rs` (`git mv` from
      `deferred_upgrade_tests.rs`) and `deferred_upgrade_unhappy_paths.rs` (`git mv` from
      `deferred_unhappy_path_tests.rs`) (1 use case per test, `_sync` + `_async` wrappers,
      cross-linking `self_replace_happy_paths` and `install_via_rustup_happy_paths` for
      `Deferred(Auto)` installs):
    - Happy paths: active cooldown -> `DeferredUpgradeOutcome::Skipped` ->
      `ExitOutcome::Skipped` (`0` HTTP calls); expired cooldown
      (`backdate_timestamp_file_for_testing`) and `UpToDate` -> `DeferredCheckSpawned` ->
      `ExitOutcome::UpToDate` with `GateStatus::ClosedForCooldown`; in-flight `50ms` check
      joined at exit with `NotifyOnly` -> `Completed(NotifiedOnly)` with
      `GateStatusError::MetadataReadFailed (NotFound)`; `PromptToUpgrade` in non-TTY ->
      `Completed(SkippedNonInteractive)` with
      `GateStatusError::MetadataReadFailed (NotFound)`.
    - Unhappy paths: in-flight `50ms` HTTP 500 joined at exit ->
      `ExitOutcome::CheckFailed(FetchOrParseFailed)` with
      `GateStatusError::MetadataReadFailed (NotFound)`; manifest connection refused ->
      `ExitOutcome::CheckFailed(FetchOrParseFailed)` with
      `GateStatusError::MetadataReadFailed (NotFound)`; `Deferred(Auto)` with
      `ClosedSource` `9.9.9` and unreachable archive ->
      `ExitOutcome::Completed(UpgradeFailed(NoViableUpgradeSource))` with
      `GateStatusError::MetadataReadFailed (NotFound)`; dedicated thread panic -> caught
      as `ExitOutcome::CheckFailed(CheckThreadPanicked)` preserving the panic message
      without unwinding.
- [x] Implement Group 4 in `self_replace_happy_paths.rs` (`git mv` from
      `self_replace_test.rs`) and `self_replace_unhappy_paths.rs` (isolated
      Coordinator/Worker subprocesses on copied executable):
    - Happy paths: `Immediate` (`--upgrade`) and `Deferred(Auto)` (exit) 4-route `.tar.gz`
      and `.sha256` download, digest verification, `self_replace` of main executable and
      `edi`/`env-source` siblings, and `GateStatus::ClosedForCooldown`.
    - Unhappy paths (`ClosedSource`, verifying copied executable bytes remain unchanged on
      disk and `GateStatusError::MetadataReadFailed (NotFound)`): SHA-256 checksum
      mismatch -> `Completed(UpgradeFailed(ChecksumMismatch))`; `GET *.tar.gz` HTTP 500
      after `HEAD 200` -> `Completed(UpgradeFailed(Other(...)))`; empty/corrupt `.tar.gz`
      archive -> `Completed(UpgradeFailed(Other(...)))`.
- [x] Implement Group 5 in `install_via_rustup_happy_paths.rs` (`git mv` from
      `cargo_install_rustup_test.rs`) and `install_via_rustup_unhappy_paths.rs` (isolated
      Coordinator/Worker subprocesses with stub `rustup` and `cargo` on `PATH` in PTY):
    - Happy paths: `HEAD` archive unreachable -> PTY stub `rustup` and
      `cargo +nightly install` (`OSC 9;4`) for both `Immediate` and `Deferred(Auto)` with
      `GateStatus::ClosedForCooldown`; mid-stream `GET *.tar.gz` HTTP 500 after `HEAD 200`
      (`OpenSource`) -> `try_fallback_to_source` recovers via PTY stub `rustup` and
      `cargo` with `GateStatus::ClosedForCooldown`.
    - Unhappy paths (verifying `GateStatusError::MetadataReadFailed (NotFound)`):
      `OpenSource` archive unreachable with empty `PATH` (no `rustup`) ->
      `Completed(UpgradeFailed(NoViableUpgradeSource { reason: NoLocalToolchain }))`; stub
      `rustup` exits `1` -> short-circuits before `cargo` (`cargo` absent from
      `invocations.log`) and `Completed(UpgradeFailed)`; stub `cargo` emits
      `OscPtyEvent::BuildError` (`OSC 9;4;2;0`) and exits `1` ->
      `Completed(UpgradeFailed)`.
- [x] Run `./check.fish --check` and targeted `cargo test -p r3bl-cmdr -q self_upgrade` to
      verify compilation and all integration/unit tests pass.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `cmdr/src/self_upgrade/check/remote_version_check_gate.rs`
    - [ ] `cmdr/src/self_upgrade/check/fetch_manifest.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/not_started.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/checking.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/upgrading.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/test_fixtures/jsonc_golden.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/test_fixtures/http_server.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/mod.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/start_latch_and_disabled.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/immediate_upgrade_happy_paths.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/immediate_upgrade_unhappy_paths.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/deferred_upgrade_happy_paths.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/deferred_upgrade_unhappy_paths.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/self_replace_happy_paths.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/self_replace_unhappy_paths.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/install_via_rustup_happy_paths.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/install_via_rustup_unhappy_paths.rs`

### Phase 7: Add `mockito` Tests Across ROC

Add deterministic `mockito`-backed unit tests for all remaining HTTP client modules in
`r3bl-cmdr` and `r3bl_tui`, replacing live internet calls in `tui/src/core/script/`.

- [x] Add `#[cfg(test)] mod tests` to `cmdr/src/analytics_client/http_client.rs` testing:
    - `make_get_request`: `200 OK` returns `Ok(Response)`, `500 Internal Server Error`
      returns `Err` via `error_for_status()`.
    - `make_post_request`: verifies `POST` method, `content-type: application/json`
      header, JSON payload via `mockito::Matcher::Json`, `200 OK` success, and `500` error
      handling.
- [x] Add `#[cfg(test)] mod tests` to `tui/src/core/script/http_client.rs` testing
      `create_client_with_user_agent(None)` (`User-Agent: scripting.rs/1.0`) and
      `create_client_with_user_agent(Some("custom-agent/2.0"))` against a `mockito` server
      with `.match_header("user-agent", ...)`.
- [x] Update `tui/src/core/script/download.rs` (`tests_download`) to download from a local
      `mockito::Server` instead of live `github.com/cloudflare/cfssl`, testing initial
      download, file overwrite with updated payload bytes, and HTTP 404/500 error
      propagation.
- [x] Update `tui/src/core/script/github_api.rs`: extract
      `try_get_latest_release_tag_from_url(url: &str)` and replace the live
      `api.github.com/repos/cloudflare/cfssl/releases/latest` test with `mockito` tests
      covering `"v"` prefix stripping (`"v1.6.5"` -> `"1.6.5"`), missing `"tag_name"` JSON
      field error, and HTTP 500 error.
- [x] Update `tui/src/core/script/crates_api.rs`: extract
      `try_get_latest_release_version_from_url(url: &str)` and replace the live
      `crates.io/api/v1/crates/serde` test with `mockito` tests covering valid
      `{"crate": {"max_version": "1.0.228"}}` extraction, missing `"max_version"` JSON
      field error, and HTTP 500 error.
- [x] Run `./check.fish --check` to verify compilation.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `cmdr/src/analytics_client/http_client.rs`
    - [ ] `tui/src/core/script/http_client.rs`
    - [ ] `tui/src/core/script/download.rs`
    - [ ] `tui/src/core/script/github_api.rs`
    - [ ] `tui/src/core/script/crates_api.rs`

### Phase 8: Eliminate Lossy Error Squashing Across ROC

Eliminate all lossy error-to-`String` squashing (`format!("{err:#}")`, `.to_string()`,
`UpgradeError::Other(String)`) and `miette::Report`-inside-enum wrappers across
`r3bl_tui` and `r3bl-cmdr::self_upgrade`. Because `miette::Report` implements a blanket
`From<E: miette::Diagnostic + Send + Sync + 'static>`, replacing `miette::Result` on
`r3bl_tui::script` helpers with typed `thiserror::Error` + `miette::Diagnostic` errors is
backwards-compatible with all existing `miette::Result` `?` callers while giving
`self_upgrade` real `#[source]` error chains.

#### Sub-phase 8.1: Typed `r3bl_tui::script` Errors

Replace `miette::Result` with typed `thiserror::Error` + `miette::Diagnostic` error types
in `http_client.rs`, `temp_dir.rs`, and `permissions.rs` (re-exported via
`tui/src/core/script/mod.rs`).

- [x] Define `HttpClientError(#[from] pub reqwest::Error)` in
      `tui/src/core/script/http_client.rs` and update `create_http_client()` to return
      `Result<reqwest::Client, HttpClientError>`.
- [x] Define `TempDirError { path, #[source] src_err }` in
      `tui/src/core/script/temp_dir.rs` and update `try_create_temp_dir()` to return
      `Result<TempDir, TempDirError>`.
- [x] Define `FilePermissionsError` (`MetadataReadFailed`, `NotARegularFile`,
      `SetPermissionsFailed`) in `tui/src/core/script/permissions.rs` and update
      `set_permission()` and `try_set_file_executable()` to return
      `Result<(), FilePermissionsError>`.
- [x] Run `./check.fish --check` to verify the entire workspace compiles cleanly (existing
      `miette::Result` callers continue using bare `?`).
- [ ] **Mandatory manual review:** Verify every file modified in this sub-phase for
      correct implementation and ensure no regressions.
    - [ ] `tui/src/core/script/http_client.rs`
    - [ ] `tui/src/core/script/temp_dir.rs`
    - [ ] `tui/src/core/script/permissions.rs`

#### Sub-phase 8.2: `JsoncParseError`, `FetchManifestError` & `!Eq` Check Outcomes

Give `JsoncManifest::try_parse_jsonc` a typed `JsoncParseError`, rewrite
`FetchManifestError` in `fetch_manifest.rs` with typed stage variants (`HttpClientCreateFailed`,
`HttpRequestFailed`, `JsoncParseFailed`, `CrateNotFoundInManifest`), and drop `Clone,
PartialEq, Eq` from `CheckFailed` and check-carrying outcome enums.

- [x] Add `JsoncParseError` (`InvalidSyntax(jsonc_parser::errors::ParseError)`,
      `InvalidSchema(serde_json::Error)`) in
      `cmdr/src/self_upgrade/jsonc_manifest/jsonc_manifest_struct.rs` and change
      `JsoncManifest::try_parse_jsonc` to return `Result<Self, JsoncParseError>`.
- [x] Rewrite `FetchManifestError` and `try_get_jsonc_manifest_from_url` in
      `cmdr/src/self_upgrade/check/fetch_manifest.rs` to preserve `HttpClientError`,
      `reqwest::Error`, and `JsoncParseError` directly via `#[source]` (deleting
      `FetchOrParseFailed { diagnostic: String }` and `from_fetch_or_parse_error`), and
      update unit tests in `fetch_manifest.rs` to assert structured variants
      (`src_err.is_timeout()`, `src_err.status()`, `JsoncParseFailed`, etc.).
- [x] Drop `Clone, PartialEq, Eq` from `CheckFailedError` and `CheckFailed` (adding
      `#[diagnostic(transparent)]` on `CheckFailedError` variants) in
      `cmdr/src/self_upgrade/upgrade_journey/typestates/check_failed.rs`, and drop
      `Clone, PartialEq, Eq` from `DeferredCheckOutcome` / `ImmediateCheckOutcome`
      (`checking.rs`), `ImmediateUpgradeOutcome` (`start_outcome.rs`), and `ExitOutcome`
      (`exit_outcome.rs`).
- [x] Convert `assert_eq2!` / `.clone()` sites on `!Eq` check/exit outcomes to structured
      pattern matches in `start_outcome.rs`, `immediate_upgrade_unhappy_paths.rs`,
      `deferred_upgrade_happy_paths.rs`, and `deferred_upgrade_unhappy_paths.rs`
      (asserting real `reqwest::Error` status/connect properties instead of string
      diagnostics).
- [x] Run `./check.fish --check` to verify compilation.
- [ ] **Mandatory manual review:** Verify every file modified in this sub-phase for
      correct implementation and ensure no regressions.
    - [ ] `cmdr/src/self_upgrade/jsonc_manifest/jsonc_manifest_struct.rs`
    - [ ] `cmdr/src/self_upgrade/check/fetch_manifest.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/check_failed.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/checking.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/start_outcome.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/exit_outcome.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/immediate_upgrade_unhappy_paths.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/deferred_upgrade_happy_paths.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/deferred_upgrade_unhappy_paths.rs`

#### Sub-phase 8.3: `TokioRuntimeOutcome` & `ThreadRunnerError`

Eliminate `miette::Report` from `TokioRuntimeOutcome` in `r3bl_tui` and replace the
string-squashed `ThreadRunnerError` variants with `ThreadSpawnFailed`,
`RuntimeInitFailed`, and `ThreadPanicked`.

- [x] Update `TokioRuntimeOutcome<T>` in
      `tui/src/core/common/std_thread_tokio_runner.rs` to hold
      `FailedToStart(std::io::Error)` and
      `Panicked { thread_name: &'static str, panic_msg: String }` (updating its unit
      tests).
- [x] Update `ThreadRunnerError` in
      `cmdr/src/self_upgrade/upgrade_journey/thread_runner.rs` to drop `Clone, PartialEq,
      Eq` and use `ThreadSpawnFailed { #[source] src_err: std::io::Error }`,
      `RuntimeInitFailed { #[source] src_err: std::io::Error }`, and
      `ThreadPanicked { thread_name: &'static str, panic_msg: String }` (updating
      `thread_runner.rs`, `start_outcome.rs`, and `deferred_upgrade_unhappy_paths.rs`
      tests).
- [x] Drop `Clone, PartialEq, Eq` from `UpgradeFailedError`, `UpgradeResolution`, and
      `Completed` (adding `#[diagnostic(transparent)]` on `UpgradeFailedError` variants)
      in `cmdr/src/self_upgrade/upgrade_journey/typestates/completed.rs`, and convert any
      `assert_eq2!` comparisons on `UpgradeResolution` / `Completed` in integration tests
      to pattern matches.
- [x] Run `./check.fish --check` to verify compilation.
- [ ] **Mandatory manual review:** Verify every file modified in this sub-phase for
      correct implementation and ensure no regressions.
    - [ ] `tui/src/core/common/std_thread_tokio_runner.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/thread_runner.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/completed.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/start_outcome.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/deferred_upgrade_unhappy_paths.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/self_replace_happy_paths.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/install_via_rustup_happy_paths.rs`

#### Sub-phase 8.4: `UpgradeError` Overhaul (Delete `Other(String)`)

Delete `UpgradeError::Other(String)` completely and replace all `.map_err(|e|
UpgradeError::Other(...))` call sites across `perform_upgrade.rs`, `download.rs`, and
`swap.rs` with structured `UpgradeError` variants preserving `#[source]` errors.

- [x] Replace `UpgradeError::Other(String)` (and drop `Clone, PartialEq, Eq`) in
      `cmdr/src/self_upgrade/upgrade/perform_upgrade.rs` with typed variants:
      `HttpClientCreateFailed`, `TempDirCreateFailed`, `DownloadHttpFailed`,
      `InvalidSha256FileFormat`, `FileIo`, `ArchiveExtractFailed`, `EmptyArchive`,
      `SetExecutableFailed`, `InvalidCurrentExePath`, `BinarySwapFailed`,
      `SourceBuildExitedNonZero`, and `SourceBuildIo`.
- [x] Update `perform_upgrade.rs`, `download.rs`, and `swap.rs` (plus unit tests in
      `download.rs` and `swap.rs`) to construct those typed variants directly without
      `.to_string()` squashing.
- [x] Update `self_replace_unhappy_paths.rs` and `install_via_rustup_unhappy_paths.rs`
      (and rustdoc comments in `cmdr/src/self_upgrade/`) to match the concrete
      `UpgradeError` variants (`DownloadHttpFailed`, `EmptyArchive`,
      `ArchiveExtractFailed`, `SourceBuildExitedNonZero`) instead of `UpgradeError::Other`.
- [x] Run `./check.fish --check` to verify compilation across the workspace.
- [ ] **Mandatory manual review:** Verify every file modified in this sub-phase for
      correct implementation and ensure no regressions.
    - [ ] `cmdr/src/self_upgrade/upgrade/perform_upgrade.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/download.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/swap.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/self_replace_unhappy_paths.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/install_via_rustup_unhappy_paths.rs`
