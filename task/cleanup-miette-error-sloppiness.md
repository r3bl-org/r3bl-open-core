# Task: Clean Up Library miette::Result

<!-- BEGIN mktoc -->

- [Task: Clean Up Library miette::Result](#task-clean-up-library-mietteresult)
- [Overview](#overview)
    - [Rule Being Enforced](#rule-being-enforced)
    - [Legitimate `miette::Result` Boundaries](#legitimate-mietteresult-boundaries)
    - [Architecture Overview](#architecture-overview)
    - [Key Design Decisions](#key-design-decisions)
    - [Verification Strategy](#verification-strategy)
        - [Automated Checks](#automated-checks)
        - [Manual Verification](#manual-verification)
    - [Target Types & Signatures](#target-types--signatures)
        - [Phase 1 Types](#phase-1-types)
        - [Phase 2 Types](#phase-2-types)
        - [Phase 3 Types](#phase-3-types)
        - [Phase 4 Types](#phase-4-types)
        - [Phase 5 Types](#phase-5-types)
        - [Phase 6 Types](#phase-6-types)
        - [Phase 7 Types](#phase-7-types)
        - [Phase 8 Types](#phase-8-types)
- [Implementation Plan](#implementation-plan)
    - [Phase 1: HTTP & Manifest Cleanup](#phase-1-http--manifest-cleanup)
        - [Sub-phase 1.1: cmdr Manifest & Analytics](#sub-phase-11-cmdr-manifest--analytics)
        - [Sub-phase 1.2: tui Script HTTP & TempDir](#sub-phase-12-tui-script-http--tempdir)
    - [Phase 2: Remaining Script Helpers](#phase-2-remaining-script-helpers)
        - [Sub-phase 2.1: Command Runner & Env](#sub-phase-21-command-runner--env)
        - [Sub-phase 2.2: Env-Source & Git Ops](#sub-phase-22-env-source--git-ops)
    - [Phase 3: Network I/O & KV Storage](#phase-3-network-io--kv-storage)
    - [Phase 4: Logging & File I/O](#phase-4-logging--file-io)
    - [Phase 5: Terminal I/O & Raw Mode](#phase-5-terminal-io--raw-mode)
        - [Sub-phase 5.1: Raw Mode & Capabilities](#sub-phase-51-raw-mode--capabilities)
        - [Sub-phase 5.2: Mode Controller, Output & OSC](#sub-phase-52-mode-controller-output--osc)
    - [Phase 6: VT100, OfsBuf, PTY & RRT](#phase-6-vt100-ofsbuf-pty--rrt)
        - [Sub-phase 6.1: OfsBuf & VT100 Parser](#sub-phase-61-ofsbuf--vt100-parser)
        - [Sub-phase 6.2: PTY Engine, Session & Mux](#sub-phase-62-pty-engine-session--mux)
        - [Sub-phase 6.3: RRT & Mio Poller](#sub-phase-63-rrt--mio-poller)
    - [Phase 7: TUI Engines, Layout & Readline](#phase-7-tui-engines-layout--readline)
        - [Sub-phase 7.1: Layout, Color, Style & Focus](#sub-phase-71-layout-color-style--focus)
        - [Sub-phase 7.2: Readline, Choose & Spinner](#sub-phase-72-readline-choose--spinner)
        - [Sub-phase 7.3: Gap Buffer, Editor & Dialog](#sub-phase-73-gap-buffer-editor--dialog)
    - [Phase 8: build-infra & CommonError Removal](#phase-8-build-infra--commonerror-removal)
        - [Sub-phase 8.1: build-infra Helpers](#sub-phase-81-build-infra-helpers)
        - [Sub-phase 8.2: Delete CommonError & Audit](#sub-phase-82-delete-commonerror--audit)
    - [Phase 9: Review Refinements (4-Tier Naming & Explicit `#[source]`)](#phase-9-review-refinements-4-tier-naming--explicit-source)
    - [Phase 10: Fallible Function `try_` Prefix Standardization](#phase-10-fallible-function-try_-prefix-standardization)
    - [Phase 11: Eliminate `CommonResult` Type Alias](#phase-11-eliminate-commonresult-type-alias)
    - [Phase 12: Eliminate Remaining Premature `miette::Result` / `miette::Report` (`TuiAvailability`, `giti`, `edi`)](#phase-12-eliminate-remaining-premature-mietteresult--miettereport-tuiavailability-giti-edi)
        - [Sub-phase 12.1: Replace `TuiAvailability<T>` & `IntoErr` with Typed `Result<T, E>` in `r3bl_tui`](#sub-phase-121-replace-tuiavailabilityt--intoerr-with-typed-resultt-e-in-r3bl_tui)
        - [Sub-phase 12.2: Pure Typed Errors Across `cmdr` (`giti` & `edi`) Up to `src/bin/*.rs`](#sub-phase-122-pure-typed-errors-across-cmdr-giti--edi-up-to-srcbinrs)
    - [Mandatory Manual Review](#mandatory-manual-review)

<!-- END mktoc -->

## Overview

Across `r3bl_tui`, `r3bl-cmdr`, and `r3bl-build-infra`, many library and internal helper
functions return `miette::Result<T>` (or `CommonResult<T>`, which aliases
`miette::Result<T>`). Returning `miette::Report` from library code erases the concrete
error type, which either spreads `miette::Result` upward into callers or forces typed
subsystems to squash errors into `String` (`format!("{err:#}")` / `Other(String)`). Other
infallible helpers wrap unconditional `ok!()` values in `miette::Result` or `CommonResult`
without any failure path.

### Rule Being Enforced

Per `.agents/skills/design-philosophy/SKILL.md` (Principle #3 and Principle #5):

1. **Concrete typed library errors:** Library and reusable functions (`r3bl_tui` modules,
   internal `r3bl-cmdr` / `build-infra` / `rust-analyzer-mcp-server` helpers) **must**
   return concrete typed errors (`Result<T, MyError>`) that derive
   `#[derive(Debug, thiserror::Error, miette::Diagnostic)]`, preserving underlying causes
   via `#[source]` (or `#[error(transparent)]` + `#[diagnostic(transparent)]` when
   delegating directly) and explicit `.map_err(...)` at call sites (no implicit
   `#[from]`).
2. **4-tier semantic naming (`*Error` / `*Result` / `*Outcome` / `*Report`):** Never name
   non-`Result` types `*Result`. Use `*Error` for matchable failures, `*Result` strictly
   for `std::result::Result` aliases, `*Outcome` for structured domain outputs meant for
   branching, and `*Report` for human-facing CLI/UI presentation payloads.
3. **Remove fake `Result` wrappers:** Functions that never fail return `T` directly (or
   return nothing instead of `Result<(), _>` when they originally returned `()`).
4. **Backwards compatibility via `?`:** Because `miette::Report` implements
   `From<E: miette::Diagnostic + Send + Sync + 'static>`, replacing `miette::Result<T>`
   with `Result<T, MyError>` on library functions works seamlessly with bare `?` inside
   application boundaries returning `miette::Result`.
5. **Retire legacy generic error bags:** Once all concrete call sites migrate to typed
   errors, delete `CommonError`, `CommonErrorType`, and `CommonResultWithError` from
   `tui/src/core/common/common_result_and_error.rs`.
6. **Prefix fallible functions & methods with `try_`:** Fallible free functions and
   inherent methods returning `Result<T, E>` (or a `Future` / wrapper resolving to
   `Result<T, E>`) carry a `try_` prefix so fallibility is explicit at every call site
   (excluding `main`, `#[test]`, trait methods, and unconditional `Err` constructors).

### Legitimate `miette::Result` Boundaries

`miette::Result<T>` and `miette::Report` remain strictly at these intentional boundaries:

1. **Binary process entry points, examples, and tests:**
   `cmdr/src/bin/{edi,giti,env-source}.rs`, `build-infra/src/bin/cargo-rustdoc-fmt.rs`,
   `rust-analyzer-mcp-server/src/main.rs`, `tui/examples/**/*.rs`, and `#[test]`
   functions.
2. **Dyn-compatible heterogeneous TUI plugin traits:** `App` (`app.rs`),
   `Component<S, AS>` and `SurfaceRender<S, AS>` (`component.rs`),
   `ComponentRegistry::route_event_to_focused_component` (`component_registry.rs`), plus
   `EditorComponent` and `DialogComponent` trait implementations (whose per-frame errors
   are caught and logged inside `main_event_loop.rs` and never escape
   `MainEventLoopFuture`).

### Architecture Overview

Typed library and application-module errors flow upward into binary `main()` entry points
and dyn-compatible TUI plugin traits automatically via `miette::Report`'s blanket
`From<E: miette::Diagnostic>` impl:

```text
┌──────────────────────────────────────────────────────────────────────┐
│ Process Exit & Dyn Plugin Trait Boundaries (keep miette::Result)     │
│ * Binaries: cmdr/src/bin/*, build-infra/src/bin/*, mcp-server, tests │
│ * Dyn plugin traits: App, Component<S, AS>, SurfaceRender<S, AS>     │
└──────────────────────────────────▲───────────────────────────────────┘
                                   │ bare `?` / `.into()` via From<E: miette::Diagnostic>
┌──────────────────────────────────┴───────────────────────────────────┐
│ Library & App Subsystems (concrete Result<T, E> or infallible T)     │
│ * cmdr/src/{giti,edi,self_upgrade,analytics_client} -> Typed *Error  │
│ * TUI entry points -> MainEventLoopError, ChooseError, ReadlineError,│
│   SpinnerError, PtyMuxError (no TuiAvailability / IntoErr wrapper)   │
│ * script/ HTTP, command, env, git -> Typed *Error & CommandRunReport │
│ * network_io/ & kv.rs -> CompressionError, KvErrorCouldNot(#[source])│
│ * log/ & into_existing.rs -> TracingInitError (.try_init()?), File   │
│ * terminal_io/, osc/, raw_mode/ -> RawModeError, TerminalIoError     │
│ * ofs_buf/ & vt_100 -> Copy OfsBufBoundsError; clear_ops infallible  │
│ * pty/ & rrt/ -> PtyError, PtyMuxError, RRTWorker::CreateError       │
│ * layout/, editor/, dialog/ -> LayoutError, GapBufferError, etc.     │
│ * build-infra/ -> CliValidationError, RunCargoFmtError, WorkspaceRoot│
└──────────────────────────────────────────────────────────────────────┘
```

### Key Design Decisions

1. **Full workspace scope and `CommonError` retirement:** All 16 previously missed library
   files are included across Phases 4 through 8, ending with deleting `CommonError`,
   `CommonErrorType`, and `CommonResultWithError` in Sub-phase 8.2 while keeping
   `CommonResult<R = ()> = miette::Result<R>` and `IntoErr` strictly for `App`,
   `Component`, `SurfaceRender`, `TuiAvailability`, and CLI entry points.
2. **Remove fake `Result` wrappers:** Functions with zero `Err` branches
   (`vt_100_impl_clear_ops.rs` 7 methods, `pty_mux` `OutputRenderer` and
   `router::handle_input`, `GlobalData::try_to_create_instance`, `Animator::stop`,
   `editor_engine::apply_event` and `render_engine`, `DialogEngineApi::apply_event`,
   `tracing_init::try_create_display_layer`, `spawn_pty_reader_thread` inner closure,
   internal `main_event_loop` render/signal helpers, and `add_root_box`) drop `Result` and
   return `T` directly (or return nothing when formerly `Result<(), _>`).
3. **Zero-allocation `Copy` `OfsBufBoundsError`:** `OfsBuf` and `OfsBufVT100` bounds
   mutators return `Result<(), OfsBufBoundsError>` instead of heap-allocating
   `miette::Report` strings on hot terminal emulation paths.
4. **`RRTWorkerAndInterrupt<W>`, `RRTWorker::CreateError`, and `SubscribeError<E>`:**
   `RRTWorker::create_and_register_os_sources` returns a readable struct
   `Result<RRTWorkerAndInterrupt<Self>, Self::CreateError>` instead of an opaque tuple or
   erased `miette::Report`, preserving concrete `MioPollWorkerCreateError` inside
   `SubscribeError::WorkerCreation(#[source] E)`.
5. **`TracingConfig::install_global` panic fix:** Switches `.init()` to `.try_init()` and
   adds `TracingInitError::GlobalSubscriberAlreadySet(#[source] TryInitError)` so
   duplicate global subscriber setup returns an error as documented instead of panicking.
6. **4-tier semantic naming (`*Error` / `*Result` / `*Outcome` / `*Report`):** Renamed
   `CommandOutputResult` to `ProcessExecutionOutcome` (`process_execution_outcome.rs`),
   replaced the `ResultAndCommand` tuple alias with `CommandRunOutcome<T>` struct in
   `command_run.rs`, and renamed `CommandRunResult<T>` to `CommandRunReport<T>`
   (`command_run_report.rs`).
7. **Explicit `#[source]` over implicit `#[from]`:** Removed all 38 `#[from]` attributes
   across `tui`, `cmdr`, `build-infra`, and `rust-analyzer-mcp-server`, replacing them
   with `#[source]` (or bare transparent delegation on `#[error(transparent)]` variants)
   plus explicit `.map_err(...)` conversions at every call site.

### Verification Strategy

#### Automated Checks

1. **After every sub-phase:** Run fast workspace typechecking via `./check.fish --check`.
2. **After modifying unit tests in a file:** Run only the targeted unit tests via
   `cargo test -p <crate> -q <test_name>`.
3. **Cross-platform verification:** After platform-gated edits in Sub-phase 2.2, Sub-phase
   5.1, and Sub-phase 6.2, verify Windows metadata compilation via
   `cargo rustc -p r3bl_tui --target x86_64-pc-windows-gnu -- --emit=metadata`.
4. **In Sub-phase 8.1:** Run `cargo install --path build-infra --force` (per
   `build-infra/AGENTS.md`).
5. **Final pre-commit milestone:** Run `./check.fish --full` only upon explicit user
   request or final task completion.

#### Manual Verification

Every sub-phase ends with `git diff` auditing across modified files and an unchecked
mandatory manual review checklist (`[ ]`) to verify byte-perfect rustdoc preservation and
zero unintended regressions.

### Target Types & Signatures

#### Phase 1 Types

Target definitions for `cmdr` self-upgrade/analytics and `r3bl_tui::script` HTTP and
temp-dir helpers:

```rust
// cmdr/src/self_upgrade/jsonc_manifest/jsonc_manifest_struct.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum JsoncUpdateError {
    #[error("Existing manifest failed validation")]
    InvalidManifest(#[source] JsoncParseError),

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
    HttpClientCreateFailed(#[source] HttpClientError),

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
    HttpClientCreateFailed(#[source] HttpClientError),

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
    HttpClientCreateFailed(#[source] HttpClientError),

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
    HttpClientCreateFailed(#[source] HttpClientError),

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

#### Phase 2 Types

Target definitions for `tui/src/core/script/` command runner, environment, package
manager, env-source, and git helpers:

```rust
// tui/src/core/script/command_impl/command_run.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum CommandRunError {
    #[error("Command '{program}' failed to start")]
    SpawnFailed {
        program: String,
        #[source]
        src_err: std::io::Error,
    },

    #[error("Command '{cmd_str}' exited with non-zero status {status}: {stderr}")]
    NonZeroExit {
        cmd_str: String,
        status: std::process::ExitStatus,
        stderr: String,
    },

    #[error("Pipe I/O failed while running '{program}'")]
    PipeIo {
        program: String,
        #[source]
        src_err: std::io::Error,
    },
}

// tui/src/core/script/environment.rs
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, miette::Diagnostic)]
#[error("Failed to read environment variable '{key}'")]
pub struct EnvVarError {
    pub key: EnvKeys,
    #[source]
    pub src_err: std::env::VarError,
}

// tui/src/core/script/package_manager.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum PackageManagerError {
    #[error("No supported package manager detected on this system")]
    UnsupportedSystem,

    #[error("Failed to spawn package manager command '{program}'")]
    SpawnFailed {
        program: String,
        #[source]
        src_err: std::io::Error,
    },

    #[error("Failed to install package '{package_name}' (status {status}): {stderr}")]
    InstallFailed {
        package_name: String,
        status: std::process::ExitStatus,
        stderr: String,
    },
}

// tui/src/core/script/env_source/run_shell.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("Failed to spawn shell '{shell}' to source '{}'", script_path.display())]
pub struct EnvSourceError {
    pub shell: &'static str,
    pub script_path: PathBuf,
    #[source]
    pub src_err: std::io::Error,
}

// tui/src/core/script/command_impl/command_run.rs
#[derive(Debug)]
pub struct CommandRunOutcome<T> {
    pub result: Result<T, CommandRunError>,
    pub command: Command,
}

// tui/src/core/script/fs_path.rs
pub type FsOpResult<T> = Result<T, FsOpError>;
```

#### Phase 3 Types

Target definitions for `tui/src/network_io/` and `tui/src/core/storage/kv.rs`:

```rust
// tui/src/network_io/compress.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum CompressionError {
    #[error("Failed to compress payload")]
    CompressFailed(#[source] std::io::Error),

    #[error("Failed to decompress payload")]
    DecompressFailed(#[source] std::io::Error),
}

// tui/src/network_io/json_serde.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum JsonSerdeError {
    #[error("Failed to serialize JSON payload")]
    SerializeFailed(#[source] serde_json::Error),

    #[error("Failed to deserialize JSON payload")]
    DeserializeFailed(#[source] serde_json::Error),
}

// tui/src/network_io/length_prefix_protocol.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum LengthPrefixProtocolError {
    #[error("Handshake timed out")]
    HandshakeTimeout(#[source] tokio::time::error::Elapsed),

    #[error("Invalid protocol magic number: expected {expected:#x}, got {actual:#x}")]
    InvalidMagicNumber { expected: u64, actual: u64 },

    #[error("Invalid protocol version: expected {expected}, got {actual}")]
    InvalidProtocolVersion { expected: u64, actual: u64 },

    #[error("Payload size {size} exceeds maximum allowed {max}")]
    PayloadTooLarge { size: u64, max: u64 },

    #[error("Payload byte length {size} exceeds protocol length integer bounds")]
    PayloadLengthOverflow {
        size: usize,
        #[source]
        src_err: std::num::TryFromIntError,
    },

    #[error("Protocol stream I/O failed")]
    Io(#[source] std::io::Error),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Compression(CompressionError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    JsonSerde(JsonSerdeError),
}

// tui/src/core/storage/kv.rs (kv_error::KvErrorCouldNot preserving kv::Error -> sled::Error chain)
#[derive(thiserror::Error, Debug, miette::Diagnostic)]
pub enum KvErrorCouldNot {
    #[error("📑 Could not create db folder: '{db_folder_path}' on disk")]
    CreateDbFolder {
        db_folder_path: String,
        #[source]
        src_err: kv::Error,
    },

    #[error("📦 Could not create bucket from store: '{bucket_name}'")]
    CreateBucketFromStore {
        bucket_name: String,
        #[source]
        src_err: kv::Error,
    },

    #[error("🔽 Could not save key/value pair to bucket")]
    SaveKeyValuePairToBucket(#[source] kv::Error),

    #[error("🔼 Could not load key/value pair from bucket")]
    LoadKeyValuePairFromBucket(#[source] kv::Error),

    #[error("❌ Could not remove key/value pair from bucket")]
    RemoveKeyValuePairFromBucket(#[source] kv::Error),
}
```

#### Phase 4 Types

Target definitions for `tui/src/core/log/` and
`tui/src/core/stack_alloc_types/into_existing.rs`:

```rust
// tui/src/core/log/
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum TracingInitError {
    #[error("Rolling log file writer has already been initialized")]
    LogFileWriterAlreadyInitialized,

    #[error("Log file path '{}' has no parent directory", path.display())]
    MissingParentDirectory { path: PathBuf },

    #[error("Log file path '{}' has no file name", path.display())]
    MissingFileName { path: PathBuf },

    #[error("Failed to create log directory")]
    CreateLogDirectoryFailed(#[source] FsOpError),

    #[error("Global tracing subscriber has already been set")]
    GlobalSubscriberAlreadySet(#[source] tracing_subscriber::util::TryInitError),
}

// tui/src/core/log/tracing_init.rs (infallible display layer builder)
pub fn create_display_layer<S>(
    level_filter: LevelFilter,
    writer_config: WriterConfig,
) -> Option<Box<DynLayer<S>>>;

// tui/src/core/stack_alloc_types/into_existing.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum InlineStringFileError {
    #[error("Failed to read or write '{}'", path.display())]
    Io {
        path: PathBuf,
        #[source]
        src_err: std::io::Error,
    },

    #[error("File '{}' contains invalid UTF-8", path.display())]
    InvalidUtf8 {
        path: PathBuf,
        #[source]
        src_err: std::str::Utf8Error,
    },
}
```

#### Phase 5 Types

Target definitions for raw mode, terminal capabilities, mode controller, output device,
and OSC sender:

```rust
// tui/src/core/ansi/terminal_raw_mode/raw_mode_core.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[non_exhaustive]
pub enum RawModeError {
    #[error("Failed to get terminal file descriptor: {0}")]
    #[diagnostic(code(r3bl_tui::terminal_raw_mode::get_fd_failed))]
    GetFileDescriptorFailed(#[source] std::io::Error),

    #[cfg(unix)]
    #[error("Failed to get/set terminal attributes: {0}")]
    #[diagnostic(code(r3bl_tui::terminal_raw_mode::attributes_failed))]
    AttributesFailed(#[source] rustix::io::Errno),

    #[error("Terminal raw mode operation failed: {0}")]
    #[diagnostic(code(r3bl_tui::terminal_raw_mode::io_failed))]
    BackendIo(#[source] std::io::Error),

    #[error("Platform is not supported for DirectToAnsi raw mode")]
    UnsupportedPlatform,
}

// tui/src/core/terminal_io/capabilities/capabilities_public_api.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum TerminalSizeError {
    #[error("Failed to query terminal window size")]
    Io(#[source] std::io::Error),

    #[cfg(unix)]
    #[error("Failed to query terminal window size via tcgetwinsize")]
    Errno(#[source] rustix::io::Errno),
}

// tui/src/core/terminal_io/output_device.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum TerminalIoError {
    #[error("Terminal I/O failed")]
    Io(#[source] std::io::Error),

    #[error(transparent)]
    #[diagnostic(transparent)]
    RawMode(RawModeError),

    #[error(
        "Cannot activate full-screen TUI: another FullScreenTuiModeGuard is already active"
    )]
    FullScreenAlreadyActive,
}
```

#### Phase 6 Types

Target definitions for `OfsBuf`, VT100 parser, PTY engine/session/mux, and RRT /
`mio_poller`:

```rust
// tui/src/tui/terminal_lib_backends/ofs_buf/
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error, miette::Diagnostic)]
pub enum OfsBufBoundsError {
    #[error("Row {row:?} is out of viewport bounds")]
    RowOutOfBounds { row: VPRow },

    #[error("Column {col:?} is out of viewport bounds")]
    ColOutOfBounds { col: VPCol },

    #[error("Column range {start:?}..{end:?} is invalid for line width")]
    InvalidColRange { start: VPCol, end: VPCol },

    #[error("Row range {start:?}..{end:?} is invalid for viewport height")]
    InvalidRowRange { start: VPRow, end: VPRow },

    #[error("Character count is zero after clamping")]
    EmptyOperation,
}

// tui/src/core/pty/
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum PtyError {
    #[error("Working directory does not exist: '{}'", path.display())]
    CwdNotFound { path: PathBuf },

    #[error("Controlled PTY half was already consumed")]
    ControlledAlreadyConsumed,

    #[error("Failed to open PTY pair")]
    OpenFailed(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),

    #[error("Failed to spawn command inside PTY")]
    SpawnCommandFailed(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),

    #[error("PTY controller operation failed")]
    ControllerFailed(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),

    #[error("PTY I/O or thread spawn failed")]
    Io(#[source] std::io::Error),

    #[error("PTY orchestrator thread panicked")]
    OrchestratorPanicked,
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum PtyMuxError {
    #[error("Failed to start PTY process '{process_name}' ({command})")]
    ProcessStartFailed {
        process_name: String,
        command: String,
        #[source]
        src_err: PtyError,
    },

    #[error(transparent)]
    #[diagnostic(transparent)]
    TerminalIo(TerminalIoError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    RawMode(RawModeError),
}

// tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/mio_poll_worker.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum MioPollWorkerCreateError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    PollCreation(PollCreationError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    SoftwareInterruptCreation(SoftwareInterruptCreationError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    StdinRegistration(StdinRegistrationError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    SignalCreation(SignalCreationError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    SignalRegistration(SignalRegistrationError),
}

// tui/src/core/resilient_reactor_thread/rrt_worker.rs
#[derive(Debug)]
pub struct RRTWorkerAndInterrupt<W: RRTWorker> {
    pub worker: W,
    pub interrupt: W::Interrupt,
}

pub trait RRTWorker: Send + Debug + 'static {
    type CreateError: std::error::Error + miette::Diagnostic + Send + Sync + 'static;

    fn create_and_register_os_sources(
        config: Self::Config,
        receiver: tokio::sync::broadcast::Receiver<Self::Input>,
    ) -> Result<RRTWorkerAndInterrupt<Self>, Self::CreateError>
    where
        Self: Sized;
}

// tui/src/core/resilient_reactor_thread/rrt_types.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum SubscribeError<
    E: std::error::Error + miette::Diagnostic + Send + Sync + 'static,
> {
    #[error("RRT internal state mutex poisoned")]
    MutexPoisoned,

    #[error("Failed to create worker thread resources")]
    WorkerCreation(#[source] E),

    #[error("Failed to spawn RRT worker thread")]
    ThreadSpawn(#[source] std::io::Error),
}

// tui/src/core/resilient_reactor_thread/rrt_input_sender.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error, miette::Diagnostic)]
#[error("Worker thread is permanently stopped")]
pub struct RRTWorkerStoppedError;
```

#### Phase 7 Types

Target definitions for layout, color, stylesheet, focus, readline/choose/spinner, gap
buffer, editor, and dialog:

```rust
// tui/src/core/ansi/color/rgb_value.rs
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, miette::Diagnostic)]
#[error("Invalid hex color format: '{input}'")]
pub struct InvalidHexColorError {
    pub input: String,
}

// tui/src/core/tui_style/tui_stylesheet.rs
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, miette::Diagnostic)]
#[error("Style id must be defined")]
pub struct StylesheetError;

// tui/src/tui/terminal_window/manage_focus/has_focus.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error, miette::Diagnostic)]
pub enum ModalFocusError {
    #[error("Modal id can only be set if base id is already set")]
    NoBaseFocusSet,

    #[error("Modal id is already set to {current:?}; cannot set to {requested:?}")]
    ModalAlreadySet {
        current: Option<FlexBoxId>,
        requested: FlexBoxId,
    },
}

// tui/src/readline_async/
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum ChooseError {
    #[error("Terminal I/O failed")]
    Io(#[source] std::io::Error),

    #[error(transparent)]
    #[diagnostic(transparent)]
    RawMode(RawModeError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    TerminalIo(TerminalIoError),
}

// tui/src/tui/editor/zero_copy_gap_buffer/
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error, miette::Diagnostic)]
pub enum GapBufferError {
    #[error("Line index {line_index} out of bounds")]
    LineIndexOutOfBounds { line_index: usize },

    #[error("Segment index {seg_index} out of bounds for line with {segment_count} segments")]
    SegmentIndexOutOfBounds {
        seg_index: usize,
        segment_count: usize,
    },

    #[error("Invalid segment range {start}..{end} for segment count {segment_count}")]
    InvalidSegmentRange {
        start: usize,
        end: usize,
        segment_count: usize,
    },

    #[error("Byte index {byte_index} exceeds content length {content_len}")]
    ByteIndexOutOfBounds {
        byte_index: usize,
        content_len: usize,
    },

    #[error("Byte end index {end_index} exceeds content length {content_len}")]
    ByteEndIndexOutOfBounds {
        end_index: usize,
        content_len: usize,
    },
}

// tui/src/tui/syntax_highlighting/md_parser_syn_hi/md_parser_syn_hi_impl.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error, miette::Diagnostic)]
#[error("Failed to parse markdown document")]
pub struct MdParseError;

// tui/src/tui/terminal_lib_backends/compositor_render_ops_to_ofs_buf.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error, miette::Diagnostic)]
#[error("Display size too small for row {row:?}")]
pub struct DisplaySizeTooSmallError {
    pub row: VPRow,
}

// tui/src/tui/dialog/dialog_engine/dialog_engine_api.rs
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, miette::Diagnostic)]
pub enum DialogEngineError {
    #[error("Window size is too small; minimum size is {min_cols} cols x {min_rows} rows")]
    DisplaySizeTooSmall { min_cols: u8, min_rows: u8 },

    #[error(transparent)]
    #[diagnostic(transparent)]
    InvalidPercentage(InvalidPercentageError),

    #[error("Dialog buffer does not exist for component id {id:?}")]
    DialogBufferNotFound { id: FlexBoxId },
}
```

#### Phase 8 Types

Target definitions for `build-infra` internal helpers (followed by deleting `CommonError`,
`CommonErrorType`, and `CommonResultWithError` in `common_result_and_error.rs`):

```rust
// build-infra/src/cargo_rustdoc_fmt/cli_arg.rs
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, miette::Diagnostic)]
pub enum CliValidationError {
    #[error("--lines-force requires an explicit target file path (e.g. 'cargo rustdoc-fmt --lines-force 10:20 src/lib.rs')")]
    MissingTargetPath,

    #[error("--lines-force only supports formatting a single file at a time, but {count} paths were provided")]
    MultiplePathsProvided { count: usize },

    #[error("File not found: '{}'", path.display())]
    FileNotFound { path: PathBuf },

    #[error("--lines-force requires a file, but directory was provided: '{}'", path.display())]
    DirectoryProvided { path: PathBuf },

    #[error("--lines-force requires a Rust source file (.rs), but got: '{}'", path.display())]
    NotRustFile { path: PathBuf },
}

// build-infra/src/cargo_rustdoc_fmt/formatters/technical_term/technical_term_dictionary.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum TechnicalTermDictionaryError {
    #[error("Failed to read terms file '{}'", path.display())]
    ReadTermsFile {
        path: PathBuf,
        #[source]
        src_err: std::io::Error,
    },

    #[error("Failed to parse technical terms JSON5 seed")]
    ParseSeedFailed(#[source] json5::Error),
}

// build-infra/src/common/cargo_fmt_runner.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum CargoFmtError {
    #[error("Failed to execute cargo fmt command")]
    SpawnFailed(#[source] std::io::Error),

    #[error("cargo fmt failed with exit code {exit_code:?}: {stderr}")]
    NonZeroExit {
        exit_code: Option<i32>,
        stderr: String,
    },
}

// build-infra/src/common/workspace_utils.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum WorkspaceRootError {
    #[error("Failed to determine current working directory")]
    CurrentDirIo(#[source] std::io::Error),

    #[error("Failed to read Cargo manifest '{}'", path.display())]
    ReadManifest {
        path: PathBuf,
        #[source]
        src_err: std::io::Error,
    },

    #[error("Could not find workspace root; make sure you are inside a Cargo workspace directory")]
    NotFound,
}
```

## Implementation Plan

### Phase 1: HTTP & Manifest Cleanup

Finish typing the `miette::Result` functions directly adjacent to `self_upgrade` and the
HTTP and temp-dir helpers touched in `http-testing-in-roc.md`.

#### Sub-phase 1.1: cmdr Manifest & Analytics

Covers `cmdr/src/self_upgrade/jsonc_manifest/jsonc_manifest_struct.rs` and
`cmdr/src/analytics_client/http_client.rs`.

- [x] Define `JsoncUpdateError` in
      `cmdr/src/self_upgrade/jsonc_manifest/jsonc_manifest_struct.rs` and update
      `JsoncManifest::try_update_crate_in_jsonc` to return
      `Result<String, JsoncUpdateError>` (removing `miette::bail!` and
      `.into_diagnostic()`).
- [x] Define `AnalyticsHttpError` in `cmdr/src/analytics_client/http_client.rs` and update
      `make_get_request` and `make_post_request` to return
      `Result<Response, AnalyticsHttpError>` (updating unit tests to assert structured
      variants).
- [x] Run `./check.fish --check` to verify compilation.

#### Sub-phase 1.2: tui Script HTTP & TempDir

Covers `temp_dir.rs`, `download.rs`, `github_api.rs`, and `crates_api.rs` in
`tui/src/core/script/`.

- [x] Define `TempDirChildPathError` in `tui/src/core/script/temp_dir.rs` and update
      `TempDirChildPath::try_new` to return `Result<Self, TempDirChildPathError>`
      (removing `miette::bail!`).
- [x] Define `DownloadFileError` in `tui/src/core/script/download.rs` and update
      `try_download_file_overwrite_existing` to return `Result<(), DownloadFileError>`.
- [x] Define `GitHubApiError` in `tui/src/core/script/github_api.rs` and update
      `try_get_latest_release_tag_from_github` and `try_get_latest_release_tag_from_url`
      to return `Result<String, GitHubApiError>`.
- [x] Define `CratesApiError` in `tui/src/core/script/crates_api.rs` and update
      `try_get_latest_release_version_from_crates_io` and
      `try_get_latest_release_version_from_url` to return
      `Result<String, CratesApiError>`.
- [x] Update unit tests in `temp_dir.rs`, `download.rs`, `github_api.rs`, and
      `crates_api.rs` to assert structured error variants.
- [x] Run `./check.fish --check` to verify compilation.

### Phase 2: Remaining Script Helpers

Replace `miette::Result` and `CommonResult` across the remaining `tui/src/core/script/`
submodules and update `giti` and `build-infra` call sites.

#### Sub-phase 2.1: Command Runner & Env

Covers `command_run.rs`, `environment.rs`, `package_manager.rs`, and `fs_path.rs` in
`tui/src/core/script/`.

- [x] Define `CommandRunError` in `tui/src/core/script/command_impl/command_run.rs` and
      update `Run::run`, `Run::run_interactive`, `run`, `run_interactive`, and `pipe`
      (removing `bail_command_ran_and_failed!` `miette::miette!`) to return
      `Result<_, CommandRunError>`.
- [x] Define `EnvVarError` in `tui/src/core/script/environment.rs` and update `try_get`
      and `try_get_path_prefixed` to return `Result<_, EnvVarError>`.
- [x] Define `PackageManagerError` in `tui/src/core/script/package_manager.rs` and update
      `check_if_package_is_installed` and `install_package` to return
      `Result<_, PackageManagerError>`.
- [x] Update `FsOpResult<T>` in `tui/src/core/script/fs_path.rs` to alias
      `Result<T, FsOpError>` directly instead of `miette::Result<T, FsOpError>`.
- [x] Run `./check.fish --check` to verify compilation.

#### Sub-phase 2.2: Env-Source & Git Ops

Covers `env_source/` (`core.rs`, `run_shell.rs`, `conformance_tests/test_fixtures.rs`),
`git/` (`types/core.rs`, `branch_ops.rs`, `file_ops.rs`, `status_ops.rs`),
`cmdr/src/giti/`, and `build-infra/src/bin/cargo-rustdoc-fmt.rs`.

- [x] Define `EnvSourceError` in `tui/src/core/script/env_source/run_shell.rs` and update
      `try_source_and_export_env`, `try_source_and_export_env_unix`,
      `try_source_and_export_env_windows`, and `core.rs::try_env_source` to return
      `Result<_, EnvSourceError>` (fixing bare tail calls in
      `env_source/conformance_tests/test_fixtures.rs`).
- [x] Update `ResultAndCommand<T, E = CommandRunError> = (Result<T, E>, Command)` in
      `tui/src/core/script/git/types/core.rs` and update the 12 production functions in
      `git/branch_ops.rs`, `git/file_ops.rs`, and `git/status_ops.rs`.
- [x] Update `CommandRunResult::Fail(..., err.into())` conversions in
      `cmdr/src/giti/branch/{branch_new_command,branch_checkout_command,branch_delete_command}.rs`
      and simplify `.map_err(...)` to `?` in `build-infra/src/bin/cargo-rustdoc-fmt.rs`.
- [x] Run `./check.fish --check` and
      `cargo rustc -p r3bl_tui --target x86_64-pc-windows-gnu -- --emit=metadata`.

### Phase 3: Network I/O & KV Storage

Replace `miette::Result` on compression, JSON serialization, length-prefixed framing, and
embedded KV store helpers (`compress.rs`, `json_serde.rs`, `length_prefix_protocol.rs`,
and `kv.rs`).

- [x] Define `CompressionError` in `tui/src/network_io/compress.rs` and update `compress`
      and `decompress` to return `Result<Buffer, CompressionError>`.
- [x] Define `JsonSerdeError` in `tui/src/network_io/json_serde.rs` and update
      `try_serialize` and `try_deserialize` to return `Result<_, JsonSerdeError>`.
- [x] Define `LengthPrefixProtocolError` in `tui/src/network_io/length_prefix_protocol.rs`
      and update the 6 functions in `handshake` and `byte_io` to return
      `Result<_, LengthPrefixProtocolError>`.
- [x] Update `KvErrorCouldNot` in `tui/src/core/storage/kv.rs` to hold
      `#[source] kv::Error` (removing unused dead variants while preserving the
      `kv::Error` to `sled::Error` `.source()` chain), update all 6 production functions
      to return `Result<_, KvErrorCouldNot>`, remove `.into_diagnostic()` in
      `iterate_bucket`, and update `kv_tests::perform_db_operations_error_conditions` to
      traverse `std::error::Error::source`.
- [x] Run `./check.fish --check` to verify compilation.

### Phase 4: Logging & File I/O

Replace `miette::Result` across tracing initialization (`tui/src/core/log/`) and stack
string file helpers (`tui/src/core/stack_alloc_types/into_existing.rs`).

- [x] Define `TracingInitError` in `tui/src/core/log/` and update
      `rolling_file_appender_impl::try_create`, `tracing_init::try_create_layers`,
      `tracing_init::try_create_file_layer`, `TracingConfig::install_thread_local`,
      `TracingConfig::install_global`, `try_initialize_logging_global`, and
      `try_initialize_logging_thread_local` to return `Result<_, TracingInitError>`.
- [x] Remove the fake `Result` from `tracing_init::try_create_display_layer` (renaming to
      `create_display_layer -> Option<Box<DynLayer<S>>>`), and switch
      `TracingConfig::install_global` from `.init()` to `.try_init()?` so duplicate global
      subscriber setup returns `TracingInitError::GlobalSubscriberAlreadySet` as
      documented.
- [x] Define `InlineStringFileError` in `tui/src/core/stack_alloc_types/into_existing.rs`
      and update `try_read_file_path_into_inline_string` and `try_write_str_to_file` to
      return `Result<(), InlineStringFileError>`.
- [x] Run `./check.fish --check` to verify compilation.

### Phase 5: Terminal I/O & Raw Mode

Replace `miette::Result` across terminal raw mode, capabilities, mode controller, output
device, and OSC sender with concrete `RawModeError`, `TerminalSizeError`, and
`TerminalIoError`.

#### Sub-phase 5.1: Raw Mode & Capabilities

Covers `raw_mode_core.rs`, `raw_mode_unix.rs`, `raw_mode_windows.rs`,
`capabilities_public_api.rs`, and `choose_api.rs` (conversion `.into()`).

- [x] Move and unify `RawModeError` into
      `tui/src/core/ansi/terminal_raw_mode/raw_mode_core.rs` (gating
      `AttributesFailed(#[source] rustix::io::Errno)` with `#[cfg(unix)]`), update
      `raw_mode_core.rs` (3 fns) and `raw_mode_windows.rs` (2 fns) to return
      `Result<_, RawModeError>`, and remove stale lock-poisoning mentions from
      `raw_mode_core.rs` docs.
- [x] Define `TerminalSizeError` in
      `tui/src/core/terminal_io/capabilities/capabilities_public_api.rs` and update
      `get_size()` to return `Result<VPSize, TerminalSizeError>` (updating
      `TuiAvailability::Broken(e.into())` in `tui/src/readline_async/choose_api.rs`);
      derive `thiserror::Error` + `miette::Diagnostic` on `TerminalNotInteractiveReason`.
- [x] Run `./check.fish --check` and
      `cargo rustc -p r3bl_tui --target x86_64-pc-windows-gnu -- --emit=metadata`.

#### Sub-phase 5.2: Mode Controller, Output & OSC

Covers `terminal_mode_controller.rs`, `output_device.rs`, `osc_sender.rs`, and
`readline_async_impl/types.rs`.

- [x] Define `TerminalIoError` in `tui/src/core/terminal_io/output_device.rs` (re-exported
      via `terminal_io/mod.rs`) and update
      `OutputDevice::{flush, setup_full_screen_tui, teardown_full_screen_tui}` and
      closures to return `Result<_, TerminalIoError>`.
- [x] Update all 28 functions (14 trait methods and 14 `OutputDevice` impls) in
      `tui/src/core/terminal_io/terminal_mode_controller.rs` to return `RawModeError` (on
      `enter_raw_mode`) and `TerminalIoError` (on the 13 I/O methods).
- [x] Update all 17 functions in `tui/src/core/ansi/osc/osc_sender.rs` to return
      `Result<(), TerminalIoError>`.
- [x] Replace the lossy `From<miette::Report> for ReadlineError` in
      `tui/src/readline_async/readline_async_impl/types.rs` with typed
      `#[from] TerminalIoError` and `#[from] RawModeError` (or `From` bridges).
- [x] Run `./check.fish --check` to verify compilation.

### Phase 6: VT100, OfsBuf, PTY & RRT

Eliminate `miette::Result` and fake `Result` wrappers across `OfsBuf`, the VT100 parser,
`pty/`, `resilient_reactor_thread/`, and `mio_poller/`.

#### Sub-phase 6.1: OfsBuf & VT100 Parser

Covers `vt_100_impl_clear_ops.rs`, `vt_100_shim_clear_ops.rs`,
`vt_100_shim_terminal_ops.rs`, `ofs_buf/` (`core.rs`, `char_ops.rs`, `line_level_ops.rs`,
`storage/types.rs`, `storage/impls/flat_2d_array_impl.rs`,
`storage/impls/growable_buffer.rs`), `active_buffer_routing.rs`,
`vt_100_impl_char_ops.rs`, `vt_100_impl_line_ops.rs`, `vt_100_impl_scroll_ops.rs`, and
`test_fixtures_ofs_buf_vt_100.rs`.

- [x] Remove fake `Result` from all 7 methods in
      `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_clear_ops.rs`
      (return nothing) and delete dead `if let Err(err)` branches in
      `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_clear_ops.rs` and
      `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_terminal_ops.rs`.
- [x] Define `Copy` enum `OfsBufBoundsError` in
      `tui/src/tui/terminal_lib_backends/ofs_buf/core.rs` and update the 8 production
      functions across `char_ops.rs`, `line_level_ops.rs`, `storage/types.rs`,
      `storage/impls/flat_2d_array_impl.rs`, and `storage/impls/growable_buffer.rs` to
      return `Result<(), OfsBufBoundsError>`.
- [x] Update the 22 production functions across `ofs_buf_vt_100/active_buffer_routing.rs`,
      `ops_impl_ofs_buf/vt_100_impl_{char,line,scroll}_ops.rs`, plus the 5
      `#[cfg(any(test, doc))]` helpers in `test_fixtures_ofs_buf_vt_100.rs` to return
      `Result<(), OfsBufBoundsError>`.
- [x] Run `./check.fish --check` to verify compilation.

#### Sub-phase 6.2: PTY Engine, Session & Mux

Covers `pty_engine/pty_pair.rs`, `pty_session/{builder.rs,type_aliases.rs,threads/*.rs}`,
and `pty_mux/{process_manager.rs,output_renderer.rs,input_router/router.rs,mux.rs}`.

- [x] Define `PtyError` and `PtyMuxError` in `tui/src/core/pty/` (converting
      `portable-pty` `anyhow::Error` via `.into()` into
      `Box<dyn std::error::Error + Send + Sync + 'static>` without adding `anyhow` to
      `tui/Cargo.toml`).
- [x] Update `pty_engine/pty_pair.rs`, `pty_session/{builder.rs,type_aliases.rs}`,
      `pty_session/threads/{orchestrator.rs,writer.rs}`, `pty_mux/process_manager.rs`, and
      `pty_mux/mux.rs` (`PTYMux::run`, `run_event_loop`, `update_terminal_title`,
      `PTYMuxBuilder::build` returning `Broken(err.into())`) to use `PtyError` and
      `PtyMuxError`.
- [x] Remove fake `Result` wrappers from `spawn_pty_reader_thread` inner thread closure
      (`reader.rs` -> `JoinHandle<()>`),
      `OutputRenderer::{render_from_active_buffer, render_initial_status_bar}`
      (`output_renderer.rs` -> return nothing), and `router::handle_input` (`router.rs` ->
      `Continuation`).
- [x] Run `./check.fish --check` and
      `cargo rustc -p r3bl_tui --target x86_64-pc-windows-gnu -- --emit=metadata`.

#### Sub-phase 6.3: RRT & Mio Poller

Covers `mio_poller/`, `resilient_reactor_thread/`, `input_device_public_api.rs`, and all
RRT test `RRTWorker` implementations and callers.

- [x] Update
      `MioSoftwareInterrupt::create_and_register_synthetic_software_interrupt_source` in
      `mio_poll_interrupt.rs` to return `Result<Self, SoftwareInterruptCreationError>`,
      and define `MioPollWorkerCreateError` in `mio_poll_worker.rs`.
- [x] Define `RRTWorkerAndInterrupt<W: RRTWorker>` and add
      `type CreateError: std::error::Error + miette::Diagnostic + Send + Sync + 'static;`
      to `RRTWorker` (`rrt_worker.rs`, re-exported via `resilient_reactor_thread/mod.rs`),
      updating `create_and_register_os_sources` to return
      `Result<RRTWorkerAndInterrupt<Self>, Self::CreateError>`.
- [x] Parameterize `SubscribeError<E>` (`rrt_types.rs`), and propagate
      `RRTWorkerAndInterrupt` destructuring and `SubscribeError<W::CreateError>` across
      `rrt.rs`, `rrt_engine.rs`, `rrt_subscriber_guard.rs`, `mio_poll_worker.rs`,
      `input_device_public_api.rs`, and doc examples in `resilient_reactor_thread/mod.rs`.
- [x] Define `RRTWorkerStoppedError` in `rrt_input_sender.rs` and update
      `InputSender::send` from `Result<(), miette::Report>` to
      `Result<(), RRTWorkerStoppedError>`.
- [x] Update all RRT test `RRTWorker` implementations and callers
      (`rrt_subscriber_guard.rs`, `unit_tests/group_b_state_machine.rs`,
      `unit_tests/group_c_input_sender.rs`, `rrt_integration_tests/smart_sender_test.rs`,
      `rrt_integration_tests/pty_test_production_factory_restart.rs`,
      `rrt_integration_tests/pty_test_production_poll_error.rs`,
      `process_isolated_tests/fixtures.rs`, and
      `process_isolated_tests/group_b_run_worker_loop.rs`) for `RRTWorkerAndInterrupt` and
      `type CreateError`.
- [x] Run `./check.fish --check` to verify compilation.

### Phase 7: TUI Engines, Layout & Readline

Type layout, styling, focus, readline/choose/spinner, gap buffer, editor, and dialog
subsystems while removing the remaining fake `Result` wrappers.

#### Sub-phase 7.1: Layout, Color, Style & Focus

Covers `layout/`, `rgb_value.rs`, `tui_stylesheet.rs`, `has_focus.rs`,
`shared_global_data.rs`, `animator_struct.rs`, `main_event_loop.rs`, and the
`downcast_ref::<CommonError>()` and `animator.stop()` callers in `edi`, `ex_editor`,
`ex_rc`, and `ex_app_no_layout`.

- [x] Derive `thiserror::Error` on `LayoutError` in `tui/src/tui/layout/layout_error.rs`
      (removing manual `Error`/`Display` impls), return `Result<T, LayoutError>` from
      `LayoutError` constructors and `unwrap_or_err!`, update
      `layout_and_positioning_traits.rs` and `surface.rs` (8 prod methods) to return
      `Result<T, LayoutError>`, and drop fake `Result` on `add_root_box` (returns
      nothing).
- [x] Define `InvalidHexColorError` (`rgb_value.rs`), `StylesheetError`
      (`tui_stylesheet.rs`, updating `add_style`, `add_styles`, `TryAdd::try_add` + 2
      impls), and `ModalFocusError` (`has_focus.rs::try_set_modal_id`), and replace
      `err.downcast_ref::<CommonError>()` in `cmdr/src/edi/app_main.rs` and
      `tui/examples/tui_apps/ex_editor/app_main.rs` with typed `ModalFocusError` matching.
- [x] Remove fake `Result` from `GlobalData::try_to_create_instance -> GlobalData<S, AS>`
      (`shared_global_data.rs`), `Animator::stop(&mut self)` (`animator_struct.rs`,
      updating `ex_rc/app_main.rs` and `ex_app_no_layout/app_main.rs`), and internal
      `main_event_loop.rs` methods (`initialize_app_and_render`,
      `handle_main_thread_signal`, `handle_render_signal`, `AppManager::render_app`),
      while keeping `App` (`app.rs`), `Component` (`component.rs`), `SurfaceRender`, and
      `MainEventLoopFuture` (`terminal_window_api.rs`) on `CommonResult`.
- [x] Run `./check.fish --check` to verify compilation.

#### Sub-phase 7.2: Readline, Choose & Spinner

Covers `tui/src/readline_async/` modules and `choose_impl/crossterm_macros.rs`.

- [x] Update `queue_commands!` and `queue_commands_no_lock!` in
      `tui/src/readline_async/choose_impl/crossterm_macros.rs` to return
      `std::io::Result<()>` instead of `miette::Result<()>` (and remove dead
      `execute_commands!` / `execute_commands_no_lock!`).
- [x] Update `ReadlineAsyncContext::{read_line, request_shutdown}`
      (`readline_async_api.rs`) to return `Result<_, ReadlineError>`, and replace
      `CommonResultWithError` with `Result` in `readline_struct.rs` and
      `line_control_task.rs`.
- [x] Update `Spinner::try_start_task` (`spinner.rs`) and all 5 functions in
      `spinner_impl/spinner_print.rs` to return `Result<(), std::io::Error>`.
- [x] Define `ChooseError` and update `FunctionComponent` (`function_component.rs`),
      `SelectComponent` (`select_component.rs`), `event_loop.rs`, and `ChooseFuture`
      (`choose_api.rs`), keeping `TuiAvailabilityChooseExt` on `miette::Result`.
- [x] Run `./check.fish --check` to verify compilation.

#### Sub-phase 7.3: Gap Buffer, Editor & Dialog

Covers `zero_copy_gap_buffer/`, `md_parser_syn_hi_impl.rs`,
`compositor_render_ops_to_ofs_buf.rs`, `crossterm_backend/input_device_impl.rs`,
`engine_public_api.rs`, `editor_component_struct.rs`, `dialog_engine_api.rs`, and
`dialog_component_struct.rs`.

- [x] Define `GapBufferError` in `tui/src/tui/editor/zero_copy_gap_buffer/mod.rs` and
      update all 8 methods in `zcgb_delete_ops.rs`, `zcgb_insert_ops.rs`, and
      `zcgb_seg_builder_ops.rs` to return `Result<(), GapBufferError>` (and update
      `zcgb_basic_ops.rs` to avoid `drop` on `Copy` result).
- [x] Define `MdParseError` (`md_parser_syn_hi_impl.rs::try_parse_and_highlight`),
      `DisplaySizeTooSmallError`
      (`compositor_render_ops_to_ofs_buf.rs::print_text_with_attributes`), and type
      private `CrosstermInputDevice::next_raw` (`crossterm_backend/input_device_impl.rs`)
      with `std::io::Result<crossterm::event::Event>`.
- [x] Remove fake `Result` from
      `editor_engine::apply_event -> EditorEngineApplyEventResult` and
      `editor_engine::render_engine` (`engine_public_api.rs`, updating
      `editor_component_struct.rs`), update `syn_hi_r3bl_path::try_render_content` to
      return `Result<(), MdParseError>`, define `DialogEngineError`
      (`dialog_engine_api.rs`) for `render_engine`, `make_flex_box_for_dialog`, and
      `render_editor`, and remove fake `Result` from
      `DialogEngineApi::apply_event -> DialogEngineApplyResponse` (updating
      `dialog_component_struct.rs` to return `DialogEngineError` converted via `?`).
- [x] Run `./check.fish --check` to verify compilation.

### Phase 8: build-infra & CommonError Removal

Type all remaining `build-infra` library helpers and delete `CommonError`,
`CommonErrorType`, and `CommonResultWithError` once zero production callers remain.

#### Sub-phase 8.1: build-infra Helpers

Covers `cli_arg.rs`, `technical_term_dictionary.rs`, `cargo_fmt_runner.rs`, and
`workspace_utils.rs` in `build-infra/src/`.

- [x] Define `CliValidationError` in `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs` and
      update `CLIArg::validate` to return `Result<(), CliValidationError>`.
- [x] Define `TechnicalTermDictionaryError` in
      `build-infra/src/cargo_rustdoc_fmt/formatters/technical_term/technical_term_dictionary.rs`
      and update `from_seed` and `parse_seed` to return
      `Result<_, TechnicalTermDictionaryError>`.
- [x] Define `CargoFmtError` in `build-infra/src/common/cargo_fmt_runner.rs` and update
      `run_cargo_fmt_on_files` to return `Result<(), CargoFmtError>`.
- [x] Define `WorkspaceRootError` in `build-infra/src/common/workspace_utils.rs` and
      update `get_workspace_root` and `helpers::find_workspace_root_from` to return
      `Result<PathBuf, WorkspaceRootError>`.
- [x] Run `./check.fish --check` and `cargo install --path build-infra --force`.

#### Sub-phase 8.2: Delete CommonError & Audit

Covers `tui/src/core/common/common_result_and_error.rs` and
`tui/examples/tui_apps/main.rs`.

- [x] Replace `CommonError::new_error_result_with_only_msg` in
      `tui/examples/tui_apps/main.rs` with `miette::miette!`.
- [x] Delete `CommonError`, `CommonErrorType`, and `CommonResultWithError` (and update
      doctests) in `tui/src/core/common/common_result_and_error.rs`, keeping only
      `pub type CommonResult<R = ()> = miette::Result<R>;` and `IntoErr`.
- [x] Grep across `tui/src/`, `cmdr/src/`, `build-infra/src/`, and
      `rust-analyzer-mcp-server/src/` to verify `miette::Result`, `CommonResult`, and
      `miette::Report` appear solely at legitimate boundaries and `#[test]` functions.
- [x] Run `./check.fish --check` and
      `cargo rustc -p r3bl_tui --target x86_64-pc-windows-gnu -- --emit=metadata` to
      verify compilation.

### Phase 9: Review Refinements (4-Tier Naming & Explicit `#[source]`)

Follow-up consistency improvements requested during manual code review: adopt the 4-tier
semantic naming convention (`*Error` / `*Result` / `*Outcome` / `*Report`) and eliminate
all implicit `#[from]` conversions workspace-wide in favor of explicit `#[source]` plus
`.map_err(...)`.

- [x] Replace `ResultAndCommand<T, E>` tuple alias in
      `tui/src/core/script/git/types/core.rs` with
      `CommandRunOutcome<T> { pub result: Result<T, CommandRunError>, pub command: Command }`
      in `tui/src/core/script/command_impl/command_run.rs`, updating all git ops, tests,
      and `giti` callers.
- [x] Rename `CommandRunResult<T>` to `CommandRunReport<T>` (`command_run_report.rs`)
      across `r3bl_tui` and `r3bl-cmdr` (`giti`), and rename `CommandOutputResult` to
      `ProcessExecutionOutcome` (`process_execution_outcome.rs`) across `r3bl_tui`,
      `r3bl-cmdr`, `r3bl-build-infra`, and `r3bl-rust-analyzer-mcp-server`.
- [x] Remove all 38 `#[from]` attributes across `tui/src/`, `cmdr/src/`,
      `build-infra/src/`, and `rust-analyzer-mcp-server/src/`, replacing them with
      `#[source]` (or bare transparent delegation on `#[error(transparent)]` variants) and
      explicit `.map_err(...)` / variant constructors at every call site.
- [x] Codify the 4-tier naming convention, error struct vs. error enum guidance, and
      explicit `#[source]` over `#[from]` rule in
      `.agents/skills/design-philosophy/{SKILL.md,patterns.md}`, and create the article
      planning task `task/error-design-pattern-article.md`.
- [x] Reinstall `r3bl-build-infra` and `r3bl-rust-analyzer-mcp-server` via
      `cargo install --path ... --force`, and verify compilation across Linux
      (`--all-targets`), Windows (`x86_64-pc-windows-gnu`), and targeted unit tests.

### Phase 10: Fallible Function `try_` Prefix Standardization

Follow-up naming consistency sweep requested during manual code review: prefix all
fallible production free functions and inherent methods across the workspace with `try_`
(excluding `main`, `#[test]`, trait methods, and unconditional `Err` constructors).

- [x] Rename all fallible production functions in `build-infra/` (`try_from_seed_file`,
      `try_parse_seed_content`, `try_run_cargo_fmt_on_files`, `try_run`, `try_validate`,
      `try_parse_line_range`, `try_get_workspace_root`, `try_find_workspace_root_from`).
- [x] Rename all 21 fallible production functions/methods in `rust-analyzer-mcp-server/`
      (including `RustAnalyzerClient::try_start` and the 9 internal `try_handle_*` MCP
      handlers while keeping MCP wire tool name strings intact).
- [x] Rename all 31 fallible production functions/methods in `cmdr/` (`analytics_client`,
      `bin/giti.rs`, `edi/app_main.rs`, `giti/branch/*`, `self_upgrade/upgrade/*`).
- [x] Rename all ~164 fallible production functions/methods across `tui/examples/`,
      `tui/src/core/` (`ansi`, `log`, `pty`, `resilient_reactor_thread`, `script`,
      `storage`, `terminal_io`, `tui_style`), `tui/src/network_io/`,
      `tui/src/readline_async/` (including `try_choose`), and `tui/src/tui/` (`dialog`,
      `editor/zero_copy_gap_buffer`, `terminal_lib_backends`, `terminal_window`), plus
      `tui/src/lib.rs` and `tui/README.md` references.
- [x] Codify the `try_` prefix rule in
      `.agents/skills/design-philosophy/{SKILL.md,patterns.md}` and update the blueprint
      in `task/error-design-pattern-article.md`.
- [x] Verify workspace compilation via `./check.fish --check`.
- [ ] Reinstall `r3bl-build-infra` and `r3bl-rust-analyzer-mcp-server` via
      `cargo install --path ... --force` once their manual review chunks are finalized.

### Phase 11: Eliminate `CommonResult` Type Alias

Follow-up cleanup requested during manual code review: remove the legacy
`pub type CommonResult<R = ()> = miette::Result<R>;` alias so that intentional
`miette::Result` erasure at application entry points and dyn-compatible TUI framework
trait boundaries (`App`, `Component`, `SurfaceRender`, `TerminalWindow`) is loud and
unmistakable instead of hidden behind an innocent-looking `CommonResult` alias.

- [x] Simplify infallible `main()` / `main_impl()` in `cmdr/src/bin/giti.rs` to return
      `()` (dropping `CommonResult`, `ok!()`, `try_` prefix, and
      `#[allow(clippy::unwrap_in_result)]`).
- [x] Replace all remaining `CommonResult` / `CommonResult<T>` usages with explicit
      `miette::Result<()>` / `miette::Result<T>` across `cmdr/src/`, `tui/src/`,
      `tui/examples/`, `.agents/skills/design-philosophy/{SKILL.md,patterns.md}`, and
      `docs/release-notes/r3bl_tui/v0.8.0.md`.
- [x] Remove `pub type CommonResult` from
      `tui/src/core/common/common_result_and_error.rs`, rename the file to
      `tui/src/core/common/into_err.rs` (matching its sole remaining `IntoErr` trait), and
      update `tui/src/core/common/mod.rs`.
- [x] Verify zero remaining `CommonResult` references in active code and run
      `./check.fish --check`.

### Phase 12: Eliminate Remaining Premature `miette::Result` / `miette::Report` (`TuiAvailability`, `giti`, `edi`)

Eliminate the remaining premature `miette::Result` / `miette::Report` conversions in
`r3bl_tui` and `cmdr/src/*` so concrete
`#[derive(Debug, thiserror::Error, miette::Diagnostic)]` errors are preserved all the way
up to `src/bin/*.rs` (`main()`), leaving `miette::Result` strictly at binary entry points,
examples/tests, and `dyn`-compatible TUI plugin traits (`App`, `Component`,
`SurfaceRender`).

#### Sub-phase 12.1: Replace `TuiAvailability<T>` & `IntoErr` with Typed `Result<T, E>` in `r3bl_tui`

Replace the 3-variant `TuiAvailability<T>` wrapper (`Available` / `NotAvailable` /
`Broken(miette::Report)`) and `IntoErr` across all 5 interactive TUI entry points with
standard `Result<T, ConcreteError>`, folding pre-flight interactivity and terminal-size
checks directly into `MainEventLoopFuture` and `ChooseFuture`:

```rust
// tui/src/core/terminal_io/capabilities/capabilities_public_api.rs
// Delete `pub enum TuiAvailability<T>` and `IntoErr` impls.
pub fn try_ensure_terminal_interactive() -> Result<(), TerminalNotInteractiveReason>;

// tui/src/tui/terminal_window/terminal_window_api.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum MainEventLoopError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    NotInteractive(TerminalNotInteractiveReason),

    #[error(transparent)]
    #[diagnostic(transparent)]
    TerminalSize(TerminalSizeError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    RawMode(RawModeError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    FullScreenSetup(TerminalIoError),
}

pub type MainEventLoopFuture<S, AS> = Pin<
    Box<
        dyn Future<
            Output = Result<
                (GlobalData<S, AS>, InputDevice, OutputDevice),
                MainEventLoopError,
            >,
        >,
    >,
>;

impl TerminalWindow {
    pub fn try_main_event_loop<S, AS>(
        app: BoxedSafeApp<S, AS>,
        exit_keys: &[InputEvent],
        state: S,
    ) -> MainEventLoopFuture<S, AS>;
}

// tui/src/readline_async/choose_api.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum ChooseError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    NotInteractive(TerminalNotInteractiveReason),

    #[error(transparent)]
    #[diagnostic(transparent)]
    TerminalSize(TerminalSizeError),

    #[error("Terminal I/O failed")]
    Io(#[source] std::io::Error),

    #[error(transparent)]
    #[diagnostic(transparent)]
    RawMode(RawModeError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    TerminalIo(TerminalIoError),
}

pub fn try_choose<'a>(...) -> ChooseFuture<'a>;

pub trait ChooseFutureExt {
    async fn try_get_first_choice(self) -> Result<Option<InlineString>, ChooseError>;
    async fn try_get_all_choices(self) -> Result<Option<ItemsOwned>, ChooseError>;
}

// tui/src/readline_async/readline_async_impl/types.rs & readline_async_api.rs
pub enum ReadlineError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    NotInteractive(TerminalNotInteractiveReason),

    #[error(transparent)]
    #[diagnostic(transparent)]
    TerminalSize(TerminalSizeError),

    #[error(transparent)]
    IO(io::Error),

    #[error(transparent)]
    #[diagnostic(transparent)]
    TerminalIo(TerminalIoError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    RawMode(RawModeError),

    #[error("line writers closed")]
    Closed,
}

impl ReadlineAsyncContext {
    pub async fn try_new(
        read_line_prompt: Option<impl AsRef<str>>,
        channel_capacity: Option<ChannelCapacity>,
    ) -> Result<ReadlineAsyncContext, ReadlineError>;
}

// tui/src/readline_async/spinner.rs
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum SpinnerError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    NotInteractive(TerminalNotInteractiveReason),

    #[error("Failed to start spinner task")]
    Io(#[source] std::io::Error),
}

impl Spinner {
    pub async fn try_start(...) -> Result<Spinner, SpinnerError>;
}

// tui/src/core/pty/pty_mux/mux.rs
pub enum PtyMuxError {
    #[error("At least one process must be configured")]
    NoProcessesConfigured,

    #[error("Maximum of {max} processes allowed (got {count})")]
    TooManyProcesses { count: usize, max: usize },

    #[error(transparent)]
    #[diagnostic(transparent)]
    NotInteractive(TerminalNotInteractiveReason),

    #[error(transparent)]
    #[diagnostic(transparent)]
    TerminalSize(TerminalSizeError),

    #[error("Failed to start PTY process '{process_name}' ({command})")]
    ProcessStartFailed {
        process_name: String,
        command: String,
        #[source]
        src_err: PtyError,
    },

    #[error(transparent)]
    #[diagnostic(transparent)]
    TerminalIo(TerminalIoError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    RawMode(RawModeError),
}

impl PTYMuxBuilder {
    pub fn try_build(self) -> Result<PTYMux, PtyMuxError>;
}

// tui/src/core/script/command_impl/command_run_report.rs
pub enum CommandRunReport<T: Debug + Display> {
    Noop(InlineString, T),
    Run(InlineString, T, TokioCommand),
    Fail(InlineString, TokioCommand, CommandRunError),
}
```

- [x] Delete `tui/src/core/common/into_err.rs`, remove `mod into_err` /
      `pub use into_err::*` from `tui/src/core/common/mod.rs`, delete `TuiAvailability<T>`
      from `capabilities_public_api.rs`, and add `try_ensure_terminal_interactive()`.
- [x] Update `TerminalWindow` (`try_main_event_loop` + `MainEventLoopError`), `try_choose`
      (`ChooseError` + `ChooseFutureExt::{try_get_first_choice, try_get_all_choices}`),
      `ReadlineAsyncContext::try_new` (`ReadlineError`), `Spinner::try_start`
      (`SpinnerError`), `PTYMuxBuilder::try_build` (`PtyMuxError`), and
      `CommandRunReport::Fail(..., CommandRunError)`.
- [x] Update all `tui/src/` integration tests, rustdoc examples (`tui/src/lib.rs`,
      `pty_mux/mod.rs`, `readline_async/mod.rs`, `choose_impl/mod.rs`, `history.rs`), and
      `tui/examples/**/*.rs` call sites.

#### Sub-phase 12.2: Pure Typed Errors Across `cmdr` (`giti` & `edi`) Up to `src/bin/*.rs`

Propagate concrete typed errors across `cmdr/src/giti/*` and `cmdr/src/edi/*` so
`miette::Report` conversion occurs strictly inside
`cmdr/src/bin/{giti,edi,env-source}.rs`:

```rust
// cmdr/src/giti/branch/branch_command.rs (re-exported from cmdr/src/giti/branch/mod.rs)
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum GitiBranchError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Git(CommandRunError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Choose(ChooseError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Readline(ReadlineError),
}

pub async fn try_handle_branch_command(
    sub_cmd: Option<BranchSubcommand>,
    maybe_branch_name: Option<String>,
) -> Result<CommandRunReport<CommandRunDetails>, GitiBranchError>;

pub async fn try_handle_branch_checkout_command(
    maybe_branch_name: Option<String>,
) -> Result<CommandRunReport<CommandRunDetails>, GitiBranchError>;

pub async fn try_handle_branch_delete_command(
    maybe_branch_name: Option<String>,
) -> Result<CommandRunReport<CommandRunDetails>, GitiBranchError>;

pub async fn try_handle_branch_new_command(
    maybe_branch_name: Option<String>,
) -> Result<CommandRunReport<CommandRunDetails>, GitiBranchError>;

// Infallible helpers drop fake Result + `try_` prefix:
// cmdr/src/giti/branch/branch_checkout_command.rs
pub async fn checkout_branch(
    branch_name: &str,
    current_branch: &str,
) -> CommandRunReport<CommandRunDetails>;

// cmdr/src/giti/branch/branch_delete_command.rs
pub async fn delete_selected_branches(
    branches: &ItemsOwned,
) -> CommandRunReport<CommandRunDetails>;

// cmdr/src/edi/app_main.rs
pub fn try_create_stylesheet() -> Result<TuiStylesheet, StylesheetError>;

// cmdr/src/edi/launcher.rs
pub async fn try_run_app(
    maybe_file_path: Option<&str>,
) -> Result<(), MainEventLoopError>;
```

- [x] Define `GitiBranchError`, migrate all `cmdr/src/giti/branch/*` functions to
      `Result<..., GitiBranchError>`, drop fake `Result` from infallible `checkout_branch`
      and `delete_selected_branches`, pass `CommandRunError` directly into
      `CommandRunReport::Fail`, and convert `GitiBranchError -> miette::Report` solely in
      `cmdr/src/bin/giti.rs`.
- [x] Update `cmdr/src/edi/app_main.rs`
      (`try_create_stylesheet -> Result<TuiStylesheet, StylesheetError>`),
      `cmdr/src/edi/launcher.rs` (`try_run_app -> Result<(), MainEventLoopError>`), and
      `cmdr/src/bin/edi.rs` (sole `?` conversion to `miette::Result<()>`), plus update
      `ChooseFutureExt` / `Spinner::try_start` call sites in `edi/ui_templates.rs`,
      `analytics_client/upgrade_check.rs`, and
      `self_upgrade/upgrade/{prompt,build,perform_upgrade}.rs`.
- [x] Run `./check.fish --check` and verify zero `miette::Result` / `miette::Report`
      usages remain in `cmdr/src/` (outside `cmdr/src/bin/` and `dyn App` /
      `SurfaceRender` trait impls) or `tui/src/` (outside `dyn App` / `Component` /
      `SurfaceRender` traits, doctests, and `#[test]`s).

### Mandatory Manual Review

Verify every file modified across Phases 1 through 12 for correct implementation,
preserved rustdoc comments, and zero regressions.

#### Batch 1

- [ ] **Mandatory manual review:** Complete manual review of all modified files.
    - [x] `build-infra/src/bin/cargo-rustdoc-fmt.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`
    - [x] `build-infra/src/cargo_rustdoc_fmt/formatters/technical_term/technical_term_dictionary.rs`
    - [x] `build-infra/src/common/run_cargo_fmt.rs`
    - [x] `build-infra/src/common/workspace_utils.rs`
    - [x] `cmdr/src/analytics_client/report_analytics.rs` (replaced deleted
          `http_client.rs`; also updated `analytics_client/mod.rs` and
          `self_upgrade/constants.rs`)
    - [ ] `cmdr/src/bin/giti.rs`
    - [ ] `cmdr/src/edi/app_main.rs`
    - [ ] `cmdr/src/giti/branch/branch_checkout_command.rs`
    - [ ] `cmdr/src/giti/branch/branch_command.rs`
    - [ ] `cmdr/src/giti/branch/branch_delete_command.rs`
    - [ ] `cmdr/src/giti/branch/branch_new_command.rs`
    - [ ] `cmdr/src/giti/ui_str.rs`
    - [ ] `cmdr/src/self_upgrade/jsonc_manifest/jsonc_manifest_struct.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/download.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/perform_upgrade.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/swap.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/upgrade_source.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/start_outcome.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/check_failed.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/checking.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/completed.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/not_started.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/upgrading.rs`
    - [ ] `rust-analyzer-mcp-server/src/error.rs`
    - [ ] `rust-analyzer-mcp-server/src/lsp/subprocess.rs`
    - [ ] `rust-analyzer-mcp-server/src/lsp/transport.rs`
    - [ ] `rust-analyzer-mcp-server/src/mcp/server.rs`
    - [ ] `rust-analyzer-mcp-server/src/mcp/tools.rs`
    - [ ] `tui/examples/shell_async.rs`
    - [ ] `tui/examples/tui_apps/ex_app_no_layout/app_main.rs`
    - [ ] `tui/examples/tui_apps/ex_editor/app_main.rs`
    - [ ] `tui/examples/tui_apps/ex_rc/app_main.rs`
    - [ ] `tui/examples/tui_apps/main.rs`
    - [ ] `tui/src/core/ansi/color/rgb_value.rs`
    - [ ] `tui/src/core/ansi/osc/osc_sender.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_core.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_unix.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_windows.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100/active_buffer_routing.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_clear_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_terminal_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_char_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_clear_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_line_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_scroll_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/test_fixtures_ofs_buf_vt_100.rs`
    - [ ] `tui/src/core/common/into_err.rs` _(deleted in Phase 12; was
          `common_result_and_error.rs`)_
    - [ ] `tui/src/core/log/log_public_api.rs`
    - [ ] `tui/src/core/log/rolling_file_appender_impl.rs`
    - [ ] `tui/src/core/log/tracing_config.rs`
    - [ ] `tui/src/core/log/tracing_init.rs`
    - [ ] `tui/src/core/pty/pty_engine/pty_engine_types.rs`
    - [ ] `tui/src/core/pty/pty_engine/pty_pair.rs`
    - [ ] `tui/src/core/pty/pty_mux/input_router/router.rs`
    - [ ] `tui/src/core/pty/pty_mux/mux.rs`
    - [ ] `tui/src/core/pty/pty_mux/output_renderer.rs`
    - [ ] `tui/src/core/pty/pty_mux/process_manager.rs`
    - [ ] `tui/src/core/pty/pty_session/builder.rs`
    - [ ] `tui/src/core/pty/pty_session/threads/orchestrator.rs`
    - [ ] `tui/src/core/pty/pty_session/threads/reader.rs`
    - [ ] `tui/src/core/pty/pty_session/threads/writer.rs`
    - [ ] `tui/src/core/pty/pty_session/type_aliases.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/mod.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/process_isolated_tests/fixtures.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/process_isolated_tests/group_b_run_worker_loop.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/process_isolated_tests/group_c_rrt_integration.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_engine.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_input_sender.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_integration_tests/pty_test_production_factory_restart.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_integration_tests/pty_test_production_poll_error.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_integration_tests/smart_sender_test.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_subscriber_guard.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_types.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_worker.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/unit_tests/group_b_state_machine.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/unit_tests/group_c_input_sender.rs`
    - [ ] `tui/src/core/script/command_impl/process_execution_outcome.rs`
    - [ ] `tui/src/core/script/command_impl/command_run_report.rs`
    - [ ] `tui/src/core/script/command_impl/command_run.rs`
    - [ ] `tui/src/core/script/command_impl/mod.rs`
    - [ ] `tui/src/core/script/crates_api.rs`
    - [ ] `tui/src/core/script/download.rs`
    - [ ] `tui/src/core/script/env_source/conformance_tests/test_fixtures.rs`
    - [ ] `tui/src/core/script/env_source/core.rs`
    - [ ] `tui/src/core/script/env_source/run_shell.rs`
    - [ ] `tui/src/core/script/environment.rs`
    - [ ] `tui/src/core/script/fs_path.rs`
    - [ ] `tui/src/core/script/git/branch_ops.rs`
    - [ ] `tui/src/core/script/git/file_ops.rs`
    - [ ] `tui/src/core/script/git/git_test_fixtures.rs`
    - [ ] `tui/src/core/script/git/status_ops.rs`
    - [ ] `tui/src/core/script/git/types/core.rs`
    - [ ] `tui/src/core/script/github_api.rs`
    - [ ] `tui/src/core/script/package_manager.rs`
    - [ ] `tui/src/core/script/temp_dir.rs`
    - [ ] `tui/src/core/stack_alloc_types/into_existing.rs`
    - [ ] `tui/src/core/storage/kv.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/capabilities_public_api.rs`
    - [ ] `tui/src/core/terminal_io/output_device.rs`
    - [ ] `tui/src/core/terminal_io/terminal_mode_controller.rs`
    - [ ] `tui/src/core/tui_style/tui_stylesheet.rs`
    - [ ] `tui/src/network_io/compress.rs`
    - [ ] `tui/src/network_io/json_serde.rs`
    - [ ] `tui/src/network_io/length_prefix_protocol.rs`
    - [ ] `tui/src/readline_async/choose_api.rs`
    - [ ] `tui/src/readline_async/choose_impl/crossterm_macros.rs`
    - [ ] `tui/src/readline_async/choose_impl/event_loop.rs`
    - [ ] `tui/src/readline_async/choose_impl/function_component.rs`
    - [ ] `tui/src/readline_async/choose_impl/select_component.rs`
    - [ ] `tui/src/readline_async/readline_async_api.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/line_control_task.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/line_state/event_handlers.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/line_state/output.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_struct.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/types.rs`
    - [ ] `tui/src/readline_async/spinner.rs`
    - [ ] `tui/src/readline_async/spinner_impl/spinner_print.rs`
    - [ ] `tui/src/tui/animator/animator_struct.rs`
    - [ ] `tui/src/tui/dialog/dialog_component/dialog_component_struct.rs`
    - [ ] `tui/src/tui/dialog/dialog_engine/dialog_engine_api.rs`
    - [ ] `tui/src/tui/editor/editor_component/editor_component_struct.rs`
    - [ ] `tui/src/tui/editor/editor_engine/engine_public_api.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/mod.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_basic_ops.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_delete_ops.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_insert_ops.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_seg_builder_ops.rs`
    - [ ] `tui/src/tui/layout/layout_and_positioning_traits.rs`
    - [ ] `tui/src/tui/layout/layout_error.rs`
    - [ ] `tui/src/tui/layout/surface.rs`
    - [ ] `tui/src/tui/syntax_highlighting/md_parser_syn_hi/md_parser_syn_hi_impl.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/compositor_render_ops_to_ofs_buf.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/crossterm_backend/input_device_impl.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/input_device_public_api.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/mio_poll_interrupt.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/mio_poll_worker.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/char_ops.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/core.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/line_level_ops.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/impls/flat_2d_array_impl.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/impls/growable_buffer.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/types.rs`
    - [ ] `tui/src/tui/terminal_window/main_event_loop.rs`
    - [ ] `tui/src/tui/terminal_window/manage_focus/has_focus.rs`
    - [ ] `tui/src/tui/terminal_window/shared_global_data.rs`

#### Batch 2

- [ ] **Phase 10 call-site, doc & skill updates:** Mechanical call-site, test, example,
      and documentation updates from the `try_` prefix rename sweep (135 files).
    - [ ] `.agents/skills/design-philosophy/SKILL.md`
    - [ ] `.agents/skills/design-philosophy/patterns.md`
    - [ ] `build-infra/src/cargo_rustdoc_fmt/formatters/technical_term/link_terms_entry_point.rs`
    - [ ] `build-infra/src/cargo_rustdoc_fmt/formatters/technical_term/ref_defs.rs`
    - [ ] `build-infra/src/cargo_rustdoc_fmt/formatters/technical_term/upgrade_term.rs`
    - [ ] `cmdr/src/analytics_client/report_analytics.rs`
    - [ ] `cmdr/src/analytics_client/upgrade_check.rs`
    - [ ] `cmdr/src/edi/ui_templates.rs`
    - [ ] `cmdr/src/self_upgrade/mod.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/build.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/prompt.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/self_replace_happy_paths.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/test_fixtures/http_server.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/state_machine.rs`
    - [ ] `rust-analyzer-mcp-server/src/lib.rs`
    - [ ] `rust-analyzer-mcp-server/src/lsp/client.rs`
    - [ ] `rust-analyzer-mcp-server/src/lsp/diagnostics.rs`
    - [ ] `rust-analyzer-mcp-server/src/lsp/queries.rs`
    - [ ] `rust-analyzer-mcp-server/src/main.rs`
    - [ ] `rust-analyzer-mcp-server/tests/mcp_conformance.rs`
    - [ ] `tui/README.md`
    - [ ] `tui/examples/choose_interactive.rs`
    - [ ] `tui/examples/choose_quiz_game.rs`
    - [ ] `tui/examples/choose_with_and_without_readline_async.rs`
    - [ ] `tui/examples/mouse_inspector.rs`
    - [ ] `tui/examples/osc_diagnostics.rs`
    - [ ] `tui/examples/pty_2d_panning_example.rs`
    - [ ] `tui/examples/pty_mux_example.rs`
    - [ ] `tui/examples/pty_rw_echo_example.rs`
    - [ ] `tui/examples/pty_simple_example.rs`
    - [ ] `tui/examples/readline_async.rs`
    - [ ] `tui/examples/readline_async_multiline.rs`
    - [ ] `tui/examples/spawn_pty_interactive.rs`
    - [ ] `tui/examples/spawn_pty_output_capture.rs`
    - [ ] `tui/src/core/ansi/constants/raw_mode_constants.rs`
    - [ ] `tui/src/core/ansi/generator/ansi_output.rs`
    - [ ] `tui/src/core/ansi/osc/mod.rs`
    - [ ] `tui/src/core/ansi/osc/osc_codes.rs`
    - [ ] `tui/src/core/ansi/osc/pty_osc_progress_scanner.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/mod.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_basic_enable_disable.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_flag_verification.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_input_behavior.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_multiple_cycles.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_poison_recovery.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/validation_tests/test_dev_tty_fallback_manual.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_char_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_line_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_scroll_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_control_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_osc_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/keyboard/csi_u.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/terminal_events.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/validation_tests/observe_real_interactive_terminal_input_events.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_bracketed_paste_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_csi_u_keyboard_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_input_device_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_keyboard_modifiers_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_keypad_and_special_keys_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_mio_poller_singleton_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_mio_poller_subscribe_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_mio_poller_thread_lifecycle_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_mio_poller_thread_reuse_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_mouse_events_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_osc_color_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_osc_safety_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_osc_sender_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_sigwinch_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_terminal_events_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_utf8_text_test.rs`
    - [ ] `tui/src/core/common/common_enums.rs`
    - [ ] `tui/src/core/common/std_thread_tokio_runner.rs`
    - [ ] `tui/src/core/coordinates/bounds_check/mod.rs`
    - [ ] `tui/src/core/coordinates/bounds_check/range/range_convert_ext.rs`
    - [ ] `tui/src/core/pty/e2e_tests/async_session_test.rs`
    - [ ] `tui/src/core/pty/e2e_tests/error_handling_test.rs`
    - [ ] `tui/src/core/pty/e2e_tests/osc_capture_test.rs`
    - [ ] `tui/src/core/pty/e2e_tests/resize_test.rs`
    - [ ] `tui/src/core/pty/e2e_tests/session_test.rs`
    - [ ] `tui/src/core/pty/mod.rs`
    - [ ] `tui/src/core/pty/pty_mux/mod.rs`
    - [ ] `tui/src/core/pty/pty_session/mod.rs`
    - [ ] `tui/src/core/pty/pty_session/session.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_integration_tests/double_panic_prevention_test.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_monitor.rs`
    - [ ] `tui/src/core/stack_alloc_types/items.rs`
    - [ ] `tui/src/core/terminal_io/backend_compat_tests/backend_compat_input_test.rs`
    - [ ] `tui/src/core/terminal_io/backend_compat_tests/backend_compat_output_test.rs`
    - [ ] `tui/src/core/terminal_io/input_device.rs`
    - [ ] `tui/src/core/terminal_io/input_event.rs`
    - [ ] `tui/src/core/terminal_io/mod.rs`
    - [ ] `tui/src/core/terminal_io/shared_writer.rs`
    - [ ] `tui/src/core/test_fixtures/pty_test_fixtures/generate_pty_test.rs`
    - [ ] `tui/src/core/test_fixtures/pty_test_fixtures/pty_test_child/pty_test_child_impl.rs`
    - [ ] `tui/src/core/test_fixtures/pty_test_fixtures/spawn_controlled_in_pty.rs`
    - [ ] `tui/src/core/tui_style/tui_style_impl.rs`
    - [ ] `tui/src/lib.rs`
    - [ ] `tui/src/readline_async/choose_impl/choose_integration_tests/mod.rs`
    - [ ] `tui/src/readline_async/choose_impl/choose_integration_tests/pty_shared_writer_pause_test.rs`
    - [ ] `tui/src/readline_async/choose_impl/mod.rs`
    - [ ] `tui/src/readline_async/choose_impl/style.rs`
    - [ ] `tui/src/readline_async/modal_terminal_guard.rs`
    - [ ] `tui/src/readline_async/mod.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/event_conversion.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/history.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/line_state/core.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/lock_manager.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_concurrent_input_output_deadlock_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_editor_state_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_multiline_output_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_readline_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_shared_writer_no_blank_line_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/readline_async_pty_test_fixtures.rs`
    - [ ] `tui/src/tui/dialog/dialog_engine/dialog_engine_struct.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_access_ops.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_adapters.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_line.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_line_metadata.rs`
    - [ ] `tui/src/tui/mod.rs`
    - [ ] `tui/src/tui/syntax_highlighting/convert_syntect_to_styled_text.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/backend_selection.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/at_most_one_instance_assert.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/channel_types.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/handler_signals.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/pixel_char_renderer.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/render_to_ansi.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/bulk_ops.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/paint_impl.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/pixel_char_bench.rs`
    - [ ] `tui/src/tui/terminal_window/terminal_window_api.rs`
    - [ ] `tui/src/tui/terminal_window/terminal_window_integration_tests/pty_main_event_loop_test.rs`

#### Batch 3

- [ ] **Phase 11 (`CommonResult` elimination):** Removal of `pub type CommonResult`,
      rename of `common_result_and_error.rs` -> `into_err.rs`, infallible `giti` `main`,
      and explicit `miette::Result` migration across workspace (45 files).
    - [ ] `.agents/skills/design-philosophy/SKILL.md`
    - [ ] `.agents/skills/design-philosophy/patterns.md`
    - [ ] `cmdr/src/bin/edi.rs`
    - [ ] `cmdr/src/bin/env-source.rs`
    - [ ] `cmdr/src/bin/giti.rs`
    - [ ] `cmdr/src/edi/app_main.rs`
    - [ ] `cmdr/src/edi/launcher.rs`
    - [ ] `cmdr/src/giti/branch/branch_checkout_command.rs`
    - [ ] `cmdr/src/giti/branch/branch_command.rs`
    - [ ] `cmdr/src/giti/branch/branch_delete_command.rs`
    - [ ] `cmdr/src/giti/branch/branch_new_command.rs`
    - [ ] `docs/release-notes/r3bl_tui/v0.8.0.md`
    - [ ] `tui/examples/spinner.rs`
    - [ ] `tui/examples/tui_apps/ex_app_no_layout/app_main.rs`
    - [ ] `tui/examples/tui_apps/ex_app_no_layout/launcher.rs`
    - [ ] `tui/examples/tui_apps/ex_app_with_1col_layout/app_main.rs`
    - [ ] `tui/examples/tui_apps/ex_app_with_1col_layout/launcher.rs`
    - [ ] `tui/examples/tui_apps/ex_app_with_1col_layout/single_column_component.rs`
    - [ ] `tui/examples/tui_apps/ex_app_with_2col_layout/app_main.rs`
    - [ ] `tui/examples/tui_apps/ex_app_with_2col_layout/column_render_component.rs`
    - [ ] `tui/examples/tui_apps/ex_app_with_2col_layout/launcher.rs`
    - [ ] `tui/examples/tui_apps/ex_editor/app_main.rs`
    - [ ] `tui/examples/tui_apps/ex_editor/launcher.rs`
    - [ ] `tui/examples/tui_apps/ex_pitch/app_main.rs`
    - [ ] `tui/examples/tui_apps/ex_pitch/launcher.rs`
    - [ ] `tui/examples/tui_apps/ex_rc/app_main.rs`
    - [ ] `tui/examples/tui_apps/ex_rc/launcher.rs`
    - [ ] `tui/examples/tui_apps/main.rs`
    - [ ] `tui/src/core/common/into_err.rs` _(deleted in Phase 12; was
          `common_result_and_error.rs`)_
    - [ ] `tui/src/core/common/mod.rs`
    - [ ] `tui/src/core/decl_macros.rs`
    - [ ] `tui/src/core/stack_alloc_types/memory_allocator.rs`
    - [ ] `tui/src/core/tui_style/tui_stylesheet.rs`
    - [ ] `tui/src/core/tui_styled_text/tui_styled_texts_impl.rs`
    - [ ] `tui/src/tui/dialog/dialog_component/dialog_component_struct.rs`
    - [ ] `tui/src/tui/editor/editor_component/editor_component_struct.rs`
    - [ ] `tui/src/tui/layout/props.rs`
    - [ ] `tui/src/tui/layout/surface.rs`
    - [ ] `tui/src/tui/syntax_highlighting/convert_syntect_to_styled_text.rs`
    - [ ] `tui/src/tui/syntax_highlighting/md_parser_syn_hi/md_parser_syn_hi_impl.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/render_tui_styled_texts.rs`
    - [ ] `tui/src/tui/terminal_window/app.rs`
    - [ ] `tui/src/tui/terminal_window/component.rs`
    - [ ] `tui/src/tui/terminal_window/main_event_loop.rs`
    - [ ] `tui/src/tui/terminal_window/manage_focus/component_registry.rs`
    - [ ] `tui/src/tui/terminal_window/terminal_window_api.rs`
    - [ ] `tui/src/tui/terminal_window/terminal_window_integration_tests/pty_main_event_loop_test.rs`

#### Batch 4

- [ ] **Phase 12 (`TuiAvailability` removal + pure typed `giti` & `edi` errors up to
      `main`):** Replacement of `TuiAvailability<T>` / `IntoErr` with typed `Result<T, E>`
      across `r3bl_tui` and pure typed error propagation across `cmdr` (`giti` & `edi`) up
      to `src/bin/*.rs` (50 files).
    - [ ] `cmdr/src/analytics_client/upgrade_check.rs`
    - [ ] `cmdr/src/bin/edi.rs`
    - [ ] `cmdr/src/bin/giti.rs`
    - [ ] `cmdr/src/edi/app_main.rs`
    - [ ] `cmdr/src/edi/launcher.rs`
    - [ ] `cmdr/src/edi/ui_templates.rs`
    - [ ] `cmdr/src/giti/branch/branch_checkout_command.rs`
    - [ ] `cmdr/src/giti/branch/branch_command.rs`
    - [ ] `cmdr/src/giti/branch/branch_delete_command.rs`
    - [ ] `cmdr/src/giti/branch/branch_new_command.rs`
    - [ ] `cmdr/src/giti/branch/mod.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/build.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/perform_upgrade.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/prompt.rs`
    - [ ] `tui/examples/choose_interactive.rs`
    - [ ] `tui/examples/choose_quiz_game.rs`
    - [ ] `tui/examples/choose_with_and_without_readline_async.rs`
    - [ ] `tui/examples/mouse_inspector.rs`
    - [ ] `tui/examples/pty_2d_panning_example.rs`
    - [ ] `tui/examples/pty_mux_example.rs`
    - [ ] `tui/examples/readline_async.rs`
    - [ ] `tui/examples/readline_async_multiline.rs`
    - [ ] `tui/examples/shell_async.rs`
    - [ ] `tui/examples/spawn_pty_interactive.rs`
    - [ ] `tui/examples/spinner.rs`
    - [ ] `tui/examples/tui_apps/ex_app_no_layout/launcher.rs`
    - [ ] `tui/examples/tui_apps/ex_app_with_1col_layout/launcher.rs`
    - [ ] `tui/examples/tui_apps/ex_app_with_2col_layout/launcher.rs`
    - [ ] `tui/examples/tui_apps/ex_editor/launcher.rs`
    - [ ] `tui/examples/tui_apps/ex_pitch/launcher.rs`
    - [ ] `tui/examples/tui_apps/ex_rc/launcher.rs`
    - [ ] `tui/examples/tui_apps/main.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/mouse.rs`
    - [ ] `tui/src/core/common/mod.rs` _(plus deleted `into_err.rs`)_
    - [ ] `tui/src/core/pty/pty_mux/mod.rs`
    - [ ] `tui/src/core/pty/pty_mux/mux.rs`
    - [ ] `tui/src/core/script/command_impl/command_run_report.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/capabilities_public_api.rs`
    - [ ] `tui/src/lib.rs`
    - [ ] `tui/src/readline_async/choose_api.rs`
    - [ ] `tui/src/readline_async/choose_impl/choose_integration_tests/pty_shared_writer_pause_test.rs`
    - [ ] `tui/src/readline_async/choose_impl/mod.rs`
    - [ ] `tui/src/readline_async/mod.rs`
    - [ ] `tui/src/readline_async/readline_async_api.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/history.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/types.rs`
    - [ ] `tui/src/readline_async/spinner.rs`
    - [ ] `tui/src/readline_async/spinner_impl/spinner_integration_tests/pty_spinner_test.rs`
    - [ ] `tui/src/tui/terminal_window/main_event_loop.rs`
    - [ ] `tui/src/tui/terminal_window/terminal_window_api.rs`
