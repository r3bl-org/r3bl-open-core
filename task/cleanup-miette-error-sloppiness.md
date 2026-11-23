Task: Clean Up Inappropriate `miette::Result` Usage Across Library Code

<!-- BEGIN mktoc -->

- [Overview](#overview)
    - [Rule Being Enforced](#rule-being-enforced)
    - [Target Types & Signatures (Phase 1)](#target-types--signatures-phase-1)
- [Implementation Plan](#implementation-plan)
    - [Phase 1: Immediate Cleanup (`self_upgrade`, `analytics_client`, `tui::script` HTTP & `TempDirChildPath`)](#phase-1-immediate-cleanup-self_upgrade-analytics_client-tuiscript-http--tempdirchildpath)
        - [Sub-phase 1.1: `r3bl-cmdr` (`jsonc_manifest_struct.rs` & `analytics_client/http_client.rs`)](#sub-phase-11-r3bl-cmdr-jsonc_manifest_structrs--analytics_clienthttp_clientrs)
        - [Sub-phase 1.2: `r3bl_tui::script` (`temp_dir.rs`, `download.rs`, `github_api.rs`, `crates_api.rs`)](#sub-phase-12-r3bl_tuiscript-temp_dirrs-downloadrs-github_apirs-crates_apirs)
    - [Phase 2: Remaining `tui/src/core/script/` Helpers](#phase-2-remaining-tuisrccorescript-helpers)
    - [Phase 3: `tui/src/network_io/` & `tui/src/core/storage/kv.rs`](#phase-3-tuisrcnetwork_io--tuisrccorestoragekvrs)
    - [Phase 4: `tui/src/core/log/` (Tracing & Rolling File Appender)](#phase-4-tuisrccorelog-tracing--rolling-file-appender)
    - [Phase 5: Terminal I/O, OSC & Raw Mode](#phase-5-terminal-io-osc--raw-mode)
    - [Phase 6: VT100 Parser, Offscreen Buffer, PTY & Resilient Reactor](#phase-6-vt100-parser-offscreen-buffer-pty--resilient-reactor)
    - [Phase 7: `CommonResult<T>` Audit & `build-infra` Internal Helpers](#phase-7-commonresultt-audit--build-infra-internal-helpers)

<!-- END mktoc -->

## Overview

Across `r3bl_tui`, `r3bl-cmdr`, and `build-infra`, many library and internal helper
functions return `miette::Result<T>` (or `CommonResult<T>`, which aliases
`miette::Result<T>`). Returning `miette::Report` from library code erases the concrete
error type, which either spreads `miette::Result` upward into callers or forces typed
subsystems to squash errors into `String` (`format!("{err:#}")` / `Other(String)`).

### Rule Being Enforced

Per `.agents/skills/design-philosophy/SKILL.md` (Principle #5: _High-Fidelity Error
Handling_):

1. **Library / reusable functions** (`r3bl_tui` modules, internal `r3bl-cmdr` /
   `build-infra` helpers) **must** return concrete typed errors (`Result<T, MyError>`)
   that derive `#[derive(Debug, thiserror::Error, miette::Diagnostic)]`, preserving
   underlying causes via `#[source]` (or `#[from]` only when `From` conversion is actually
   needed and unambiguous).
2. **Application boundaries** (`main()`, CLI command entry points like
   `cmdr/src/bin/*.rs`, `build-infra/src/bin/cargo-rustdoc-fmt.rs`, and `#[test]`
   functions) are the **only** places where `miette::Result<T>` belongs.
3. Because `miette::Report` implements
   `From<E: miette::Diagnostic + Send + Sync + 'static>`, replacing `miette::Result<T>`
   with `Result<T, MyError>` on library functions is backwards-compatible with all
   existing `miette::Result` callers using bare `?`.

### Target Types & Signatures (Phase 1)

```rust
// cmdr/src/self_upgrade/jsonc_manifest/jsonc_manifest_struct.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum JsoncUpdateError {
    #[error("Existing manifest failed validation")]
    InvalidManifest(#[from] JsoncParseError),

    #[error("Failed to parse JSONC into CST")]
    CstParseFailed(#[source] jsonc_parser::errors::ParseError),

    #[error("Crate '{crate_name}' not found in {file}")]
    CrateNotFound {
        crate_name: String,
        file: &'static str,
    },
}

// cmdr/src/analytics_client/http_client.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum AnalyticsHttpError {
    #[error("Failed to create HTTP client")]
    HttpClientCreateFailed(#[from] HttpClientError),

    #[error("HTTP request to '{url}' failed")]
    RequestFailed {
        url: String,
        #[source]
        src_err: reqwest::Error,
    },
}

// tui/src/core/script/temp_dir.rs
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, miette::Diagnostic)]
#[error("sub_dir must be relative and not contain parent components: {}", path.display())]
pub struct TempDirChildPathError {
    pub path: PathBuf,
}

// tui/src/core/script/download.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum DownloadFileError {
    #[error("Failed to create HTTP client")]
    HttpClientCreateFailed(#[from] HttpClientError),

    #[error("HTTP download from '{url}' failed")]
    HttpRequestFailed {
        url: String,
        #[source]
        src_err: reqwest::Error,
    },

    #[error("Failed to write downloaded file '{}'", path.display())]
    FileIo {
        path: PathBuf,
        #[source]
        src_err: std::io::Error,
    },
}

// tui/src/core/script/github_api.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum GitHubApiError {
    #[error("Failed to create HTTP client")]
    HttpClientCreateFailed(#[from] HttpClientError),

    #[error("GitHub API request to '{url}' failed")]
    HttpRequestFailed {
        url: String,
        #[source]
        src_err: reqwest::Error,
    },

    #[error("Missing 'tag_name' string field in GitHub release JSON from '{url}'")]
    MissingTagName { url: String },
}

// tui/src/core/script/crates_api.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum CratesApiError {
    #[error("Failed to create HTTP client")]
    HttpClientCreateFailed(#[from] HttpClientError),

    #[error("crates.io API request to '{url}' failed")]
    HttpRequestFailed {
        url: String,
        #[source]
        src_err: reqwest::Error,
    },

    #[error("Missing 'crate.max_version' string field in crates.io JSON from '{url}'")]
    MissingMaxVersion { url: String },
}
```

## Implementation Plan

### Phase 1: Immediate Cleanup (`self_upgrade`, `analytics_client`, `tui::script` HTTP & `TempDirChildPath`)

Finish typing the remaining `miette::Result` functions directly adjacent to `self_upgrade`
and the HTTP/temp-dir helpers touched in `http-testing-in-roc.md`.

#### Sub-phase 1.1: `r3bl-cmdr` (`jsonc_manifest_struct.rs` & `analytics_client/http_client.rs`)

- [ ] Define `JsoncUpdateError` in
      `cmdr/src/self_upgrade/jsonc_manifest/jsonc_manifest_struct.rs` and update
      `JsoncManifest::try_update_crate_in_jsonc` to return
      `Result<String, JsoncUpdateError>` (removing `miette::bail!` and
      `.into_diagnostic()`).
- [ ] Define `AnalyticsHttpError` in `cmdr/src/analytics_client/http_client.rs` and update
      `make_get_request` and `make_post_request` to return
      `Result<Response, AnalyticsHttpError>` (updating unit tests to assert structured
      variants).
- [ ] Run `./check.fish --check` to verify compilation.
- [ ] **Mandatory manual review:** Verify every file modified in this sub-phase for
      correct implementation and ensure no regressions.
    - [ ] `cmdr/src/self_upgrade/jsonc_manifest/jsonc_manifest_struct.rs`
    - [ ] `cmdr/src/analytics_client/http_client.rs`

#### Sub-phase 1.2: `r3bl_tui::script` (`temp_dir.rs`, `download.rs`, `github_api.rs`, `crates_api.rs`)

- [ ] Define `TempDirChildPathError` in `tui/src/core/script/temp_dir.rs` and update
      `TempDirChildPath::try_new` to return `Result<Self, TempDirChildPathError>`
      (removing `miette::bail!`).
- [ ] Define `DownloadFileError` in `tui/src/core/script/download.rs` and update
      `try_download_file_overwrite_existing` to return `Result<(), DownloadFileError>`.
- [ ] Define `GitHubApiError` in `tui/src/core/script/github_api.rs` and update
      `try_get_latest_release_tag_from_github` and `try_get_latest_release_tag_from_url`
      to return `Result<String, GitHubApiError>`.
- [ ] Define `CratesApiError` in `tui/src/core/script/crates_api.rs` and update
      `try_get_latest_release_version_from_crates_io` and
      `try_get_latest_release_version_from_url` to return
      `Result<String, CratesApiError>`.
- [ ] Update unit tests in `temp_dir.rs`, `download.rs`, `github_api.rs`, and
      `crates_api.rs` to assert structured error variants.
- [ ] Run `./check.fish --check` to verify compilation.
- [ ] **Mandatory manual review:** Verify every file modified in this sub-phase for
      correct implementation and ensure no regressions.
    - [ ] `tui/src/core/script/temp_dir.rs`
    - [ ] `tui/src/core/script/download.rs`
    - [ ] `tui/src/core/script/github_api.rs`
    - [ ] `tui/src/core/script/crates_api.rs`

### Phase 2: Remaining `tui/src/core/script/` Helpers

Replace `miette::Result` across the remaining script utility modules with typed
`thiserror::Error` + `miette::Diagnostic` errors:

- [ ] `tui/src/core/script/command_impl/command_runner.rs` (5 fns)
- [ ] `tui/src/core/script/environment.rs` (2 fns)
- [ ] `tui/src/core/script/package_manager.rs` (2 fns)
- [ ] `tui/src/core/script/env_source/core.rs` (1 fn) & `run_shell.rs` (9 fns)
- [ ] `tui/src/core/script/git/branch_ops.rs` (8 fns), `file_ops.rs` (2 fns),
      `status_ops.rs` (3 fns)
- [ ] Run `./check.fish --check` to verify compilation.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/src/core/script/command_impl/command_runner.rs`
    - [ ] `tui/src/core/script/environment.rs`
    - [ ] `tui/src/core/script/package_manager.rs`
    - [ ] `tui/src/core/script/env_source/core.rs`
    - [ ] `tui/src/core/script/env_source/run_shell.rs`
    - [ ] `tui/src/core/script/git/branch_ops.rs`
    - [ ] `tui/src/core/script/git/file_ops.rs`
    - [ ] `tui/src/core/script/git/status_ops.rs`

### Phase 3: `tui/src/network_io/` & `tui/src/core/storage/kv.rs`

Replace `miette::Result` on pure serialization, framing, compression, and embedded KV
helpers:

- [ ] `tui/src/network_io/compress.rs` (2 fns) — typed `CompressionError` wrapping
      `std::io::Error`.
- [ ] `tui/src/network_io/json_serde.rs` (2 prod fns) — typed `JsonSerdeError` wrapping
      `serde_json::Error`.
- [ ] `tui/src/network_io/length_prefix_protocol.rs` (6 fns) — typed
      `LengthPrefixProtocolError` wrapping I/O, compression, and JSON errors.
- [ ] `tui/src/core/storage/kv.rs` (8 fns) — return the existing `KvError` directly
      instead of erasing it into `miette::Result`.
- [ ] Run `./check.fish --check` to verify compilation.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/src/network_io/compress.rs`
    - [ ] `tui/src/network_io/json_serde.rs`
    - [ ] `tui/src/network_io/length_prefix_protocol.rs`
    - [ ] `tui/src/core/storage/kv.rs`

### Phase 4: `tui/src/core/log/` (Tracing & Rolling File Appender)

Replace `miette::Result` across tracing initialization with a typed `TracingInitError`:

- [ ] `tui/src/core/log/rolling_file_appender_impl.rs` (1 fn)
- [ ] `tui/src/core/log/tracing_init.rs` (3 fns)
- [ ] `tui/src/core/log/tracing_config.rs` (2 fns)
- [ ] `tui/src/core/log/log_public_api.rs` (2 fns)
- [ ] Run `./check.fish --check` to verify compilation.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/src/core/log/rolling_file_appender_impl.rs`
    - [ ] `tui/src/core/log/tracing_init.rs`
    - [ ] `tui/src/core/log/tracing_config.rs`
    - [ ] `tui/src/core/log/log_public_api.rs`

### Phase 5: Terminal I/O, OSC & Raw Mode

Most of these functions only perform `std::io::Write` / terminal syscalls and wrap
`std::io::Error` into `miette::Report`:

- [ ] `tui/src/core/terminal_io/terminal_mode_controller.rs` (28 fns)
- [ ] `tui/src/core/ansi/osc/osc_sender.rs` (17 fns)
- [ ] `tui/src/core/terminal_io/output_device.rs` (5 fns)
- [ ] `tui/src/core/terminal_io/capabilities/capabilities_public_api.rs` (3 fns)
- [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_core.rs` (3 fns) &
      `raw_mode_windows.rs` (2 fns)
- [ ] Run `./check.fish --check` to verify compilation.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/src/core/terminal_io/terminal_mode_controller.rs`
    - [ ] `tui/src/core/ansi/osc/osc_sender.rs`
    - [ ] `tui/src/core/terminal_io/output_device.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/capabilities_public_api.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_core.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_windows.rs`

### Phase 6: VT100 Parser, Offscreen Buffer, PTY & Resilient Reactor

Replace `miette::Result` across low-level buffer, PTY, and reactor internals:

- [ ] `tui/src/core/ansi/vt_100_pty_output_parser/`
      (`ofs_buf_vt_100/active_buffer_routing.rs`,
      `ops_impl_ofs_buf/vt_100_impl_{char,clear,line,scroll}_ops.rs`) (30 fns)
- [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/` (`char_ops.rs`, `line_level_ops.rs`,
      `storage/impls/flat_2d_array_impl.rs`, `storage/growable_buffer.rs`,
      `storage/types.rs`) (9 fns)
- [ ] `tui/src/core/pty/` (`pty_engine/pty_pair.rs`, `pty_mux/`, `pty_session/`) (24 fns)
- [ ] `tui/src/core/resilient_reactor_thread/` (`rrt_worker.rs`,
      `rrt_subscriber_guard.rs`) &
      `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/` (4 fns)
- [ ] Run `./check.fish --check` to verify compilation.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/` (modified files)
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/` (modified files)
    - [ ] `tui/src/core/pty/` (modified files)
    - [ ] `tui/src/core/resilient_reactor_thread/` & `mio_poller/` (modified files)

### Phase 7: `CommonResult<T>` Audit & `build-infra` Internal Helpers

Audit `CommonResult<T>` (`tui/src/core/common/common_result_and_error.rs`) across the TUI
framework and type the remaining `build-infra` internal helpers:

- [ ] Audit `tui/src/tui/layout/surface.rs` (32 `CommonResult` fns — already has
      `LayoutError` in `layout_error.rs`) and `layout_and_positioning_traits.rs` (8 fns).
- [ ] Audit `tui/src/readline_async/` (`choose_api.rs`, `choose_impl/`,
      `readline_async_api.rs`, `spinner.rs`, `spinner_impl/spinner_print.rs`).
- [ ] Audit `tui/src/tui/terminal_window/` (`terminal_window_api.rs`,
      `main_event_loop.rs`, `app.rs`, `component.rs`), keeping `miette::Result` only at
      genuine application boundaries (`cmdr/src/bin/*.rs`, `cmdr/src/edi/app_main.rs`,
      `build-infra/src/bin/cargo-rustdoc-fmt.rs`).
- [ ] Type `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs` (`validate`) and
      `build-infra/src/cargo_rustdoc_fmt/formatters/technical_term/technical_term_dictionary.rs`
      (`from_seed`, `parse_seed`).
- [ ] Run `./check.fish --check` to verify compilation.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/src/tui/layout/surface.rs`
    - [ ] `tui/src/tui/layout/layout_and_positioning_traits.rs`
    - [ ] `tui/src/readline_async/` (modified files)
    - [ ] `tui/src/tui/terminal_window/` (modified files)
    - [ ] `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`
    - [ ] `build-infra/src/cargo_rustdoc_fmt/formatters/technical_term/technical_term_dictionary.rs`
