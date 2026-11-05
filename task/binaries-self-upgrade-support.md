# Task: Binary Self-Upgrade & Bootstrap

<!-- prettier-ignore-start -->
<!-- BEGIN mktoc -->

- [Task: Binary Self-Upgrade & Bootstrap](#task-binary-self-upgrade--bootstrap)
- [Overview](#overview)
    - [Dual-Engine Strategy](#dual-engine-strategy)
    - [Version Manifest](#version-manifest)
    - [CLI Flag](#cli-flag)
- [Lifecycle Flowchart](#lifecycle-flowchart)
- [Architecture](#architecture)
    - [1. r3bl-cmdr (Shared Upgrade Engine)](#1-r3bl-cmdr-shared-upgrade-engine)
    - [2. r3bl-cmdr (Binaries: `giti`, `edi`, `env-source`)](#2-r3bl-cmdr-binaries-giti-edi-env-source)
    - [3. r3bl-build-infra](#3-r3bl-build-infra)
    - [4. r3bl-rust-analyzer-mcp-server](#4-r3bl-rust-analyzer-mcp-server)
    - [5. Bootstrap Scripts & Manifest on `r3bl.com`](#5-bootstrap-scripts--manifest-on-r3blcom)
    - [6. Release Script & `github.com/r3bl-org/r3bl-binaries`](#6-release-script--githubcomr3bl-orgr3bl-binaries)
    - [7. Website & Documentation](#7-website--documentation)
- [Implementation Plan](#implementation-plan)
    - [Phase 1: Shared Upgrade Module](#phase-1-shared-upgrade-module)
    - [Phase 2: Refactor & Unify Upgrade Engine Architecture](#phase-2-refactor--unify-upgrade-engine-architecture)
    - [Phase 2.1: Move `self_upgrade` from `r3bl_tui` to `r3bl-cmdr`](#phase-21-move-self_upgrade-from-r3bl_tui-to-r3bl-cmdr)
    - [Phase 2.2: Make illegal states unrepresentable (Typestate Pattern + Single Entry Point + `UpgradeCheckStatus` Process-Wide Singleton)](#phase-22-make-illegal-states-unrepresentable-typestate-pattern--single-entry-point--upgradecheckstatus-process-wide-singleton)
    - [Phase 2.3: Consolidate `binaries_release_base_url` into `versions.jsonc` & Introduce `EndpointUrl` Newtype](#phase-23-consolidate-binaries_release_base_url-into-versionsjsonc--introduce-endpointurl-newtype)
    - [Phase 2.4: Split `upgrade_journey_public_api.rs` into `upgrade_journey/` Module & Extract Per-Typestate Files](#phase-24-split-upgrade_journey_public_apirs-into-upgrade_journey-module--extract-per-typestate-files)
    - [Phase 2.5: Make Typestate Transitions Explicit & Pure (Decouple Workers from `UpgradeCheckStatus` Singleton)](#phase-25-make-typestate-transitions-explicit--pure-decouple-workers-from-upgradecheckstatus-singleton)
    - [Phase 3: Bootstrap, Manifest & Release Script](#phase-3-bootstrap-manifest--release-script)
    - [Phase 4: Refactor cmdr](#phase-4-refactor-cmdr)
    - [Phase 5: Integrate build-infra](#phase-5-integrate-build-infra)
    - [Phase 6: Integrate mcp-server](#phase-6-integrate-mcp-server)
    - [Phase 7: Docs & Website](#phase-7-docs--website)
    - [Phase 8: Verification & Testing](#phase-8-verification--testing)
- [Cross-Platform Test Matrix](#cross-platform-test-matrix)
    - [Test Scenario Matrix](#test-scenario-matrix)
    - [OS-Specific Verification Checklist](#os-specific-verification-checklist)
        - [Linux (`x86_64-unknown-linux-gnu`)](#linux-x86_64-unknown-linux-gnu)
        - [macOS Apple Silicon (`aarch64-apple-darwin`) & Intel (`x86_64-apple-darwin`)](#macos-apple-silicon-aarch64-apple-darwin--intel-x86_64-apple-darwin)
        - [Windows (`x86_64-pc-windows-msvc`)](#windows-x86_64-pc-windows-msvc)

<!-- END mktoc -->
<!-- prettier-ignore-end -->

## Overview

Transition all R3BL binary tools, both open-source (`r3bl-cmdr`, `r3bl-build-infra`,
`r3bl-rust-analyzer-mcp-server`) and external closed-source R3BL binary crates, to a
unified, hybrid distribution and self-upgrade architecture.

> **Architecture & Schema Reference**:
>
> - **Code Schema**: [`JsoncManifest`] in
>   `cmdr/src/self_upgrade/jsonc_manifest/jsonc_manifest_struct.rs` defines the Rust
>   schema, JSONC parser (`JsoncManifest::try_parse_jsonc`), and comment-preserving CST
>   updater (`JsoncManifest::try_update_crate_in_jsonc`) for
>   `https://r3bl.com/versions.jsonc`.
> - **Architecture Specification**: This document is the technical specification for the
>   shared client-side upgrade engine (`r3bl_cmdr::self_upgrade`), centralized manifest
>   schema (`https://r3bl.com/versions.jsonc`), and binary distribution repository
>   (`https://github.com/r3bl-org/r3bl-binaries`).
> - **External Release & Distribution Pipeline**: Cross-platform binary compilation,
>   archive packaging, SHA-256 checksum generation, and publishing of `versions.jsonc`,
>   `install.sh`, and `install.ps1` are performed by external release tooling that depends
>   on `r3bl-cmdr` (calling `JsoncManifest::try_update_crate_in_jsonc()`).

1. **Primary (Fast Binary Download, Open & Closed Source)**: 1-2 second HTTP download and
   atomic in-place binary swap via `self_replace` from pre-compiled GitHub Releases hosted
   on the dedicated history-free binary distribution repo
   (`https://github.com/r3bl-org/r3bl-binaries`).
2. **Fallback (Resilient Source Compilation, Open Source Only)**: For open-source crates
   (`"crate_distribution": "open-source"` / `CrateDistribution::OpenSource` in
   `versions.jsonc`), if `github.com/r3bl-org/r3bl-binaries` is down, blocked, or lacks a
   release asset for the host target triple, automatically fall back to building the
   pinned version from `crates.io` using the shared PTY session manager
   (`rustup toolchain install nightly --force` +
   `cargo +nightly install <crate> --version <ver> --force`) with live OSC 9;4 compilation
   progress tracking. Closed-source binaries (`"crate_distribution": "closed-source"` /
   `CrateDistribution::ClosedSource`) have no source fallback and exit with a clean retry
   message.

### Dual-Engine Strategy

**Fast Binary Downloads + Resilient Open-Source Fallback**: This hybrid architecture
provides the ideal balance between instant binary upgrades across all R3BL products and
resilient source compilation for open-source crates:

- **Instant Upgrades by Default**: 99% of upgrades take 1-2 seconds with zero toolchain
  dependencies on user machines.
- **Zero Toolchain Dependencies for End Users**: Users and AI coding agents using
  `https://r3bl.com/install.sh` or `https://r3bl.com/install.ps1` do not need Rust
  nightly, `rustup`, `cargo`, GCC, or system dev headers.
- **Centralized History-Free Binary Distribution (`github.com/r3bl-org/r3bl-binaries`)**:
  All pre-compiled `.tar.gz` archives for both open-source and closed-source tools across
  Linux, macOS, and Windows are attached as GitHub Release assets on
  `github.com/r3bl-org/r3bl-binaries`. Because release assets live on GitHub's object
  storage CDN (`objects.githubusercontent.com`) outside `.git` history, the repository
  remains near 0 KB forever and never counts against GitHub's 1 GB Git repository quota.
- **Resilient Fallback for Open-Source Crates**: If GitHub CDN is unreachable and the
  crate has `"crate_distribution": "open-source"`, the engine checks the local environment
  for `rustup` or `cargo` and automatically compiles the target version from `crates.io`.
- **Universal Branded Bootstrap Scripts (`r3bl.com`)**: Single-line first-time installs
  served directly from `r3bl.com`:
    - Linux/macOS: `curl -LsSf https://r3bl.com/install.sh | sh`
    - Windows: `irm https://r3bl.com/install.ps1 | iex`

### Version Manifest

**`r3bl.com/versions.jsonc` (Schema: [`JsoncManifest`] in
`cmdr/src/self_upgrade/jsonc_manifest/jsonc_manifest_struct.rs`)**: Dedicated-thread
version checks (bounded by a `2s` timeout and a `1h` `mtime` cache cooldown per crate) and
bootstrap scripts query `https://r3bl.com/versions.jsonc` for version numbers and crate
distribution, never `crates.io` or GitHub's rate-limited REST API.

Because we use **JSONC** (JSON with Comments and trailing commas) across the codebase,
`r3bl_cmdr` parses `versions.jsonc` via `jsonc-parser` (`features = ["serde", "cst"]`):

- **Deserialization (`JsoncManifest::try_parse_jsonc`)**: Parses JSONC with `//` line
  comments, `/* ... */` block comments, and trailing commas, ignoring unknown future
  fields so additive schema extensions never break older binaries in the wild.
- **Comment-Preserving Mutation (`JsoncManifest::try_update_crate_in_jsonc`)**: Uses
  `jsonc_parser::cst::CstRootNode` (Concrete Syntax Tree) so external release tooling can
  update a crate's version entry in `versions.jsonc` while preserving 100% of existing
  comments and formatting, before deploying `versions.jsonc` to `r3bl.com` and
  `github.com/r3bl-org/r3bl-binaries`.

```jsonc
{
    // Base URL prefix for downloading pre-compiled binary release archives.
    "binaries_release_base_url": "https://github.com/r3bl-org/r3bl-binaries/releases/download",
    "crates": {
        // Open-source interactive CLI suite (`giti`, `edi`, `env-source`).
        "r3bl-cmdr": {
            "version": "0.0.25",
            "crate_distribution": "open-source",
            "released_on": "2026-10-17T00:00:00Z",
        },
        // Open-source build & documentation tooling (`cargo-rustdoc-fmt`).
        "r3bl-build-infra": {
            "version": "0.1.0",
            "crate_distribution": "open-source",
            "released_on": "2026-10-17T00:00:00Z",
        },
        // Open-source Rust Analyzer MCP server (`rust-analyzer-mcp-server`).
        "r3bl-rust-analyzer-mcp-server": {
            "version": "0.1.0",
            "crate_distribution": "open-source",
            "released_on": "2026-10-17T00:00:00Z",
        },
    },
}
```

- **Schema Fields (defined in [`JsoncManifest`] & [`CrateMetadata`])**:
    - **Additive-Only Forward Compatibility**: Neither `JsoncManifest` nor `CrateMetadata`
      uses `#[serde(deny_unknown_fields)]`, so any new fields added in the future are
      safely ignored by existing binaries in the wild.
    - `binaries_release_base_url` (`EndpointUrl` wrapper): Base URL prefix for downloading
      pre-compiled binary release archives (e.g.
      `"https://github.com/r3bl-org/r3bl-binaries/releases/download"`). Storing this in
      `versions.jsonc` makes it the single source of truth and allows moving binary
      release storage without rebuilding binaries.
    - `version` (`CrateVersion` struct with
      `pub major: u16, pub minor: u16, pub patch: u16` and
      `#[serde(deserialize_with = "JsoncScalar::deserialize_str")]`): Validated semantic
      version parsed via slice pattern matching (`[major_str, minor_str, patch_str]`) at
      deserialization time, with explicit `Ord` / `PartialOrd` comparing
      `(self.major, self.minor, self.patch)` in SemVer precedence order.
    - `crate_distribution` (`CrateDistribution` with
      `#[serde(deserialize_with = "JsoncScalar::deserialize_str")]`): Enum witness
      indicating distribution and licensing model:
        - `"open-source"` (`CrateDistribution::OpenSource`): Open-source crates in
          `r3bl-open-core` eligible for `cargo install` source compilation fallback.
        - `"closed-source"` (`CrateDistribution::ClosedSource`): Closed-source R3BL binary
          crates distributed exclusively as pre-compiled binaries.
    - `released_on` (`CrateReleaseDate` wrapping `chrono::DateTime<chrono::Utc>`): ISO
      8601 / RFC 3339 UTC timestamp (`YYYY-MM-DDTHH:MM:SSZ`) recording when the release
      was published (and enabling release-date verification for closed-source
      distributions).
- **Strictly Newer Version Comparison (`remote_version > local_version`)**:
    - Both remote (`versions.jsonc`) and local (`CARGO_PKG_VERSION`) versions are parsed
      into `CrateVersion { major, minor, patch }` and compared directly via
      `CrateVersion`'s `Ord` implementation (`remote_version > local_version` rather than
      `!=`). This prevents local development builds (whose `Cargo.toml` version has been
      bumped ahead of `versions.jsonc`) from falsely triggering an upgrade that would
      downgrade the local binary.
- **Pinned Version in `crates.io` Compilation Fallback (Open Source Only)**:
    - When `crate_distribution == CrateDistribution::OpenSource` and the pre-compiled
      binary cannot be downloaded from `github.com/r3bl-org/r3bl-binaries`, the fallback
      compilation codepath **never** installs an unpinned "latest" version from
      `crates.io`. Instead, it **always** passes `--version <pinned_ver>` using the exact
      version `<pinned_ver>` read from `r3bl.com/versions.jsonc`:
      `cargo +nightly install <crate_name> --version <pinned_ver> --force`.
    - Closed-source binaries (`CrateDistribution::ClosedSource`) have no fallback and only
      download pre-compiled binaries from `github.com/r3bl-org/r3bl-binaries`.
- **Direct CDN Asset Checking**: When an upgrade is available, a lightweight HTTP `HEAD`
  request directly checks the `r3bl-org/r3bl-binaries` GitHub Release asset download URL:
  `https://github.com/r3bl-org/r3bl-binaries/releases/download/v{ver}-{crate}/{crate}-v{ver}-{target}.tar.gz`.
  This hits GitHub's global CDN directly with zero REST API overhead.

### CLI Flag

**Dedicated `--upgrade` and `--upgrade-behavior` Flags Across All Binaries**: All
workspace binaries flatten `UpgradeCLIArgs` to provide top-level CLI arguments for
standalone self-upgrades or explicit exit upgrade behavior (with zero magic environment
variable inspection):

- `--upgrade`: Supports `giti --upgrade`, `edi --upgrade`, `env-source --upgrade`,
  `cargo-rustdoc-fmt --upgrade` (and `cargo rustdoc-fmt --upgrade`), and
  `rust-analyzer-mcp-server --upgrade`. When passed, the binary bypasses normal
  application startup, executes the immediate check (`5s` manifest timeout, bypassing
  cache cooldown) and upgrade flow, and exits (`0` on `ImmediateUpgradeResult::Succeeded`,
  `1` on `ImmediateUpgradeResult::Failed`).
- `--upgrade-behavior <behavior>`: Explicit `clap::ValueEnum` override
  (`prompt-to-upgrade`, `auto`, `notify-only`, `disable`) encapsulated inside
  `UpgradeCLIArgs` and converted cleanly into `UpgradePolicy` at the CLI boundary. When
  omitted, defaults to `auto` (`UpgradeActionOnExit::Auto` via `UpgradePolicy::default()`)
  across ALL workspace binaries without exception (`giti`, `edi`, `env-source`,
  `cargo-rustdoc-fmt`, `rust-analyzer-mcp-server`), with `prompt-to-upgrade` available as
  an explicit opt-in override for users who prefer interactive prompts. Dedicated-thread
  checks use a `2s` manifest timeout and a `1h` `mtime` cache cooldown
  (`<cache_dir>/r3bl/upgrade-check/<crate_name>.last_check`). All feedback for
  `env-source` and `rust-analyzer-mcp-server` routes to `stderr`
  (`UserFeedbackChannel::Stderr`) so `stdout` remains pure.

#### Binary Configuration Matrix

| Binary                     | Default `UpgradeActionOnExit` | `user_feedback_channel`       | Rationale                                                                                                                                    |
| :------------------------- | :---------------------------- | :---------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------- |
| `giti`                     | `UpgradeActionOnExit::Auto`   | `UserFeedbackChannel::Stdout` | Seamless auto-upgrade at exit; interactive prompt opt-in via `--upgrade-behavior prompt-to-upgrade`.                                         |
| `edi`                      | `UpgradeActionOnExit::Auto`   | `UserFeedbackChannel::Stdout` | Seamless auto-upgrade at exit; interactive prompt opt-in via `--upgrade-behavior prompt-to-upgrade`.                                         |
| `cargo-rustdoc-fmt`        | `UpgradeActionOnExit::Auto`   | `UserFeedbackChannel::Stdout` | Non-interactive developer & CI doc formatter; seamless auto-upgrade at exit (`1h` `mtime` cooldown avoids repeated checks).                  |
| `rust-analyzer-mcp-server` | `UpgradeActionOnExit::Auto`   | `UserFeedbackChannel::Stderr` | MCP/LSP server; `stderr` ensures stdio JSON-RPC transport on `stdout` remains 100% uncorrupted.                                              |
| `env-source`               | `UpgradeActionOnExit::Auto`   | `UserFeedbackChannel::Stderr` | Sub-2ms shell startup within `1h` `mtime` cooldown (max `2s` wait once per cooldown window); `stderr` keeps piped `stdout` 100% uncorrupted. |

## Lifecycle Flowchart

```text
╭──────────────────────────────────────────────────────────────────────────────╮
│ Invocation: `upgrade_journey!(channel, cli_args).start()`                    │
╰──────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭──────────────────────────────────────────────────────────────────────────────╮
│ 1. Version Check (`r3bl.com/versions.jsonc` & `SINGLETON` Coordination)      │
│                                                                              │
│ ├─► Duplicate `.start()` (`SINGLETON != NotStarted`) ──► `AlreadyStarted`    │
│ ├─► `--upgrade-behavior disable` ──► `SINGLETON = Disabled` (0 threads/I/O)  │
│ ├─► `DedicatedThread` within `1h` `mtime` stamp ──► `CooldownSkipped` (~0ms) │
│ └─► GET https://r3bl.com/versions.jsonc (`5s` `--upgrade`, `2s` thread)      │
│     ├─► Parses `binaries_release_base_url`, `version`, `crate_distribution`, │
│     │   and `released_on`                                                    │
│     ├─► Compares `(major, minor, patch)`: `remote_version > local_version`   │
│     └─► Stores `UpgradeJourney<State>` in `SINGLETON` (`UpgradeCheckStatus`) │
╰──────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭──────────────────────────────────────────────────────────────────────────────╮
│ 2. Trigger Point                                                             │
│                                                                              │
│ ├─► Path 1: Dedicated `--upgrade` (`UpgradeStrategy::Immediate`)             │
│ │   └─► Synchronously checks & upgrades (`Checking -> Upgrading -> Completed`│
│ │       or `UpToDate` / `CheckFailed`), returns `UpgradedAndExit(result)`    │
│ │                                                                            │
│ └─► Path 2: Application exit (`UpgradeCheckStatus::handle_upgrade_on_exit`)  │
│     ├─► Joins `thread_check_handle` outside lock (Scenario 2c wait <= `2s`)  │
│     ├─► Auto (Universal default): Automatically upgrades (`Upgrading`)       │
│     ├─► PromptToUpgrade (opt-in): Prompts user ("Yes, upgrade now" / "No")   │
│     ├─► NotifyOnly: Prints one-line notification                             │
│     └─► Disable: Suppressed at startup (`UpgradeCheckStatus::Disabled`)      │
│     (Controlled explicitly via `--upgrade-behavior <behavior>`, no magic env)│
╰──────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭──────────────────────────────────────────────────────────────────────────────╮
│ 3. Upgrade Source Resolution (`UpgradeSource::try_resolve`)                 │
│                                                                              │
│ Internal `get_host_target_triple()` detects OS & Architecture                │
│ HEAD <binaries_release_base_url>/v{pinned_ver}-{crate}/                      │
│      {crate}-v{pinned_ver}-{target}.tar.gz                                   │
│                                                                              │
│ ├─► Returns 200 / 302:                                                       │
│ │   └─► `UpgradeSource::ReleaseArchive`                                      │
│ │                                                                            │
│ └─► Returns 404 / 5xx / Network Error / Offline:                             │
│     ├─► `CrateDistribution::ClosedSource` ──► Error: `NoViableUpgradeSource` │
│     ├─► `CrateDistribution::OpenSource`:                                     │
│     │   ├─► `rustup` on PATH ──► `UpgradeSource::BuildFromSourceRustup`      │
│     │   └─► No `rustup` on PATH ──► Error: `NoViableUpgradeSource`           │
│     │                               (nightly required for `#![feature]`)     │
╰──────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭──────────────────────────────────────────────────────────────────────────────╮
│ 4. Execution Engine                                                          │
│                                                                              │
│ ├─► [ReleaseArchive]                                                         │
│ │   Stream download with % and MB spinner, verify `.sha256`, unpack archive, │
│ │   atomic swap via `self_replace` (plus sibling binaries in archive).       │
│ │   (If stream fails mid-way & `OpenSource`, falls back to source build).    │
│ │                                                                            │
│ ├─► [BuildFromSourceRustup] (OpenSource only; uses pinned_ver)               │
│ │   Run `rustup toolchain install nightly --force` in PTY session,           │
│ │   then `cargo +nightly install <crate> --version <pinned_ver>` (OSC 9;4).  │
│ │                                                                            │
│ └─► [All Attempts Fail]                                                      │
│     Emit clean error on `UserFeedbackChannel` with retry: `<binary> --upgrade`│
╰──────────────────────────────────────────────────────────────────────────────╯
```

## Architecture

### 1. r3bl-cmdr (Shared Upgrade Engine)

**Shared Upgrade Engine**: The unified upgrade module is housed inside
`r3bl_cmdr::self_upgrade` (`cmdr/src/self_upgrade/`), alongside
`r3bl_cmdr::analytics_client` (which manages `~/.config/r3bl-cmdr` and analytics
reporting). Other binary crates (`r3bl-build-infra`, `r3bl-rust-analyzer-mcp-server`, and
external R3BL binary crates) depend on both `r3bl_tui` and `r3bl-cmdr`:

- [x] `constants.rs`:
    - [x] Centralized constants: `toolchain_bin` (`rustup`, `cargo`), `toolchain_cli_args`
          (`toolchain`, `install`, `nightly`, `+nightly`, `--version`, `--force`),
          `upgrade_cli_args` (`UPGRADE_ARG_LONG`, `UPGRADE_BEHAVIOR_ARG_LONG`),
          `manifest_timeouts` (`IMMEDIATE_MANIFEST_TIMEOUT`,
          `DEDICATED_THREAD_MANIFEST_TIMEOUT`, `DEDICATED_THREAD_CHECK_COOLDOWN`),
          `check_cooldown_cache` (`CACHE_TOP_LEVEL_DIR`, `UPGRADE_CHECK_CACHE_SUBDIR`,
          `LAST_CHECK_FILE_EXT`), `archive`, `versions_manifest_keys`, `upgrade_reasons`,
          `semver`, and `url_schemes`.
- [x] `jsonc_manifest/`:
    - [x] `jsonc_manifest_struct.rs`: `JsoncManifest` schema for
          `https://r3bl.com/versions.jsonc` (`binaries_release_base_url: EndpointUrl` and
          `crates: HashMap<CrateName, CrateMetadata>`), providing `try_parse_jsonc()`
          (ignoring unknown future fields) and comment-preserving
          `try_update_crate_in_jsonc()` via `jsonc-parser` CST.
    - [x] `jsonc_scalar.rs`: `JsoncScalar` trait (`try_parse(&str)`, `cst_value(&self)`,
          and `deserialize_str`) unifying validation, serde deserialization, and lossless
          CST serialization across manifest scalar types.
    - [x] Domain newtypes and enums: `EndpointUrl` (validated HTTP/HTTPS URL via
          `JsoncScalar`, `default_manifest_url()`, and `EndpointUrlParseError`),
          `CrateDistribution` (`OpenSource` / `ClosedSource`, via `JsoncScalar` and
          `CrateDistributionParseError`), `CrateName`,
          `CrateVersion { pub major: u16, pub minor: u16, pub patch: u16 }` (parsed via
          slice pattern matching with `JsoncScalar` and explicit tuple `.cmp()` `Ord`),
          `CrateReleaseDate`, and `CrateMetadata`.
- [x] `config/`:
    - [x] `cli_arg.rs`: Opaque `UpgradeCLIArgs` (`--upgrade`, `--upgrade-behavior` backed
          by private `CliUpgradeBehavior`) with `From<UpgradeCLIArgs> for UpgradePolicy`
          at the CLI boundary.
    - [x] `upgrade_policy.rs`: Pure domain model with `UpgradePolicy`
          (`Enabled(UpgradeStrategy)`, `Disabled`), `UpgradeStrategy` (`Immediate`,
          `DedicatedThread(UpgradeActionOnExit)`), and `UpgradeActionOnExit`
          (`PromptToUpgrade`, `Auto`, `NotifyOnly`).
    - [x] `upgrade_config.rs`: Immutable `UpgradeConfig` (`crate_name`, `current_version`,
          `user_feedback_channel`, `upgrade_policy`, `jsonc_manifest_url: EndpointUrl`,
          `cooldown_policy`) and `CooldownPolicy` (`DefaultCacheDir`,
          `CustomTimestampFilePath(PathBuf)`, `Disabled`).
    - [x] `user_feedback_channel.rs`: `UserFeedbackChannel` (`Stdout`, `Stderr`, `Quiet`).
- [x] `upgrade_journey/` (including `upgrade_singleton.rs`):
    - [x] `upgrade_journey!` macro, `UpgradeJourney<State>` typestate machine across 9
          states (`NotStarted`, `Disabled`, `CooldownSkipped`, `Checking`, `UpToDate`,
          `UpgradeAvailable`, `CheckFailed`, `Upgrading`, `Completed`) in
          `upgrade_journey/typestates/`, `StartOutcome`
          (`UpgradedAndExit(ImmediateUpgradeResult)`, `ContinueApplication`,
          `AlreadyStarted`), and `SINGLETON` (`UpgradeCheckStatus` +
          `thread_check_handle: Option<JoinHandle<()>>` with
          `UpgradeCheckStatus::handle_upgrade_on_exit()`).
- [x] `check/`:
    - [x] `fetch_manifest.rs` (refactored from `check_entry_point.rs` in Phase 2.5):
          `fetch_crate_metadata_from_manifest(manifest_url, crate_name, timeout)` and
          `ManifestCheckError` (pure HTTP/JSONC manifest fetch & metadata extraction with
          zero knowledge of `UpgradeJourney` or `UpgradeCheckStatus`).
    - [x] `cooldown.rs`: `CooldownPolicy`, `CooldownStatus`, and `1h` `mtime` stamp check
          and touch helpers (`<cache_dir>/r3bl/<crate_name>/self_upgrade_last_check`).
- [x] `upgrade/`:
    - [x] `target.rs`: `get_host_target_triple()` (`x86_64-unknown-linux-gnu`,
          `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`,
          `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`).
    - [x] `bin_name.rs`: `get_current_bin_name()`.
    - [x] `release.rs`: Constructs direct asset download URLs
          (`format_release_asset_urls`) under `binaries_release_base_url` and performs
          lightweight HTTP `HEAD` asset accessibility check
          (`check_release_asset_available`).
    - [x] `upgrade_source.rs`: `UpgradeSource` (`ReleaseArchive`, `BuildFromSourceRustup`)
          with `UpgradeSource::try_resolve` and `UpgradeSource::try_fallback_to_source`.
    - [x] `build.rs`: Generalized PTY session manager (`PtySessionBuilder` with
          `DefaultPtySessionConfig + PtySessionConfigToken::CaptureOsc`) for `rustup`
          toolchain updates and
          `cargo +nightly install <crate> --version <pinned_version_from_versions_jsonc> --force`.
    - [x] `download.rs`: Streams binary archive from `binaries_release_base_url`, verifies
          SHA-256 checksum, updates `Spinner`, and checks for `tokio::signal::ctrl_c()`.
    - [x] `swap.rs`: Unpacks `.tar.gz` and performs atomic in-place binary swapping via
          `self_replace` on current executable and updates any sibling binaries.
    - [x] `prompt.rs`: Interactive exit prompt (`choose`) returning
          `PromptUpgradeDecision` (`UpgradeNow`, `DeclinedByUser`,
          `SkippedNonInteractive`).
    - [x] `ui_strings.rs`, `feedback_device.rs`, and `perform_upgrade.rs`
          (`perform_upgrade` and `UpgradeError`).
- [x] `mod.rs`: Private modules and public barrel exports re-exported at `r3bl_cmdr::*`.

### 2. r3bl-cmdr (Binaries: `giti`, `edi`, `env-source`)

**Interactive Command-Line & Developer Productivity Tools (`giti`, `edi`, `env-source`)**:

- [ ] Flatten `UpgradeCLIArgs` (`--upgrade` and `--upgrade-behavior`) into `giti`
      (`cmdr/src/giti/clap_config.rs`), `edi` (`cmdr/src/edi/clap_config.rs`), and
      `env-source` (`cmdr/src/env_source/clap_config.rs`).
- [ ] Invoke `upgrade_journey!(channel, cli_arg.upgrade_args).start()` at startup in
      `cmdr/src/bin/giti.rs`, `cmdr/src/bin/edi.rs`, and `cmdr/src/bin/env-source.rs`
      (exiting immediately on `StartOutcome::UpgradedAndExit`), and invoke
      `UpgradeCheckStatus::handle_upgrade_on_exit()` before exit.
- [ ] **Output routing & latency for `env-source`**:
    - [ ] `env-source` is a synchronous shell startup tool whose `stdout` is piped into
          `source` / `Invoke-Expression` (`env-source -i ~/.profile -o fish | source`).
    - [ ] Like all binaries, `env-source` defaults to `UpgradeActionOnExit::Auto` (with a
          `1h` `mtime` cache cooldown so normal shell startups skip thread/network I/O in
          `< 0.05 ms`, and a `2s` manifest timeout cap once per `1h` window), configured
          with `UserFeedbackChannel::Stderr` so all progress, notifications, and upgrade
          messages route exclusively to `stderr` and never corrupt `stdout` shell exports.
    - [ ] `env-source --upgrade` runs the standalone upgrade flow via
          `upgrade_journey!(UserFeedbackChannel::Stderr, cli_arg.upgrade_args).start()`.
- [ ] Refactor `cmdr/src/analytics_client/upgrade_check.rs` and `ui_str.rs` (used by
      `giti` and `edi`) to delegate to `r3bl_cmdr::self_upgrade::*` using
      `UpgradeActionOnExit::Auto` as the default (with
      `UpgradeActionOnExit::PromptToUpgrade` available via
      `--upgrade-behavior prompt-to-upgrade`) and `UserFeedbackChannel::Stdout`.
- [ ] Preserve `cmdr`-specific interactive prompts, `ExitContext` handling, emojis, and
      lolcat greetings.
- [ ] Release archive for `r3bl-cmdr` on `github.com/r3bl-org/r3bl-binaries` bundles all 3
      binaries: `giti`, `edi`, and `env-source`. When any one of the 3 binaries upgrades,
      it uses `self_replace` on itself and updates the other 2 sibling binaries in the
      same directory so the entire suite stays on the same version.

### 3. r3bl-build-infra

**Build Infrastructure & Documentation Tools (`cargo-rustdoc-fmt`)**:

- [ ] Add `r3bl-cmdr` dependency to `build-infra/Cargo.toml` alongside `r3bl_tui`.
- [ ] Flatten `UpgradeCLIArgs` (`--upgrade` and `--upgrade-behavior`) into `CLIArg` in
      `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs` (compatible with both
      `cargo-rustdoc-fmt --upgrade` and `cargo rustdoc-fmt --upgrade` via
      `strip_cargo_subcommand_injection()`).
- [ ] Start `upgrade_journey!(UserFeedbackChannel::Stdout, cli_arg.upgrade_args).start()`
      in `build-infra/src/bin/cargo-rustdoc-fmt.rs` (handling
      `StartOutcome::UpgradedAndExit`) and call
      `UpgradeCheckStatus::handle_upgrade_on_exit()` before exit.
- [ ] Release archive for `r3bl-build-infra` on `github.com/r3bl-org/r3bl-binaries`
      bundles `cargo-rustdoc-fmt` (and any future binaries added to `build-infra`, such as
      `spawny` in `task/build-infra-spawny.md`).

### 4. r3bl-rust-analyzer-mcp-server

**Model Context Protocol Semantic Analysis Server (`rust-analyzer-mcp-server`)**:

- [ ] Add `r3bl-cmdr` dependency to `rust-analyzer-mcp-server/Cargo.toml` alongside
      `r3bl_tui`.
- [ ] **Pure `std::thread` Compatibility**: `rust-analyzer-mcp-server` intentionally runs
      without a global `#[tokio::main]` async runtime on its MCP/LSP bridge threads. It
      calls the synchronous `upgrade_journey!(...).start()` and
      `UpgradeCheckStatus::handle_upgrade_on_exit()` entry points, which isolate async
      HTTP/PTY I/O onto a private `current_thread` Tokio runtime.
- [ ] Flatten `UpgradeCLIArgs` (`--upgrade` and `--upgrade-behavior`) into `CLIArg` in
      `rust-analyzer-mcp-server/src/cli_arg.rs` (making `workspace` optional when
      `--upgrade` is passed).
- [ ] Start `upgrade_journey!(UserFeedbackChannel::Stderr, cli_arg.upgrade_args).start()`
      in `rust-analyzer-mcp-server/src/main.rs` (handling
      `StartOutcome::UpgradedAndExit`), and invoke
      `UpgradeCheckStatus::handle_upgrade_on_exit()` at shutdown to guarantee `stdout`
      JSON-RPC message transport remains 100% uncorrupted.
- [ ] Release archive for `r3bl-rust-analyzer-mcp-server` on
      `github.com/r3bl-org/r3bl-binaries` bundles `rust-analyzer-mcp-server`.

### 5. Bootstrap Scripts & Manifest on `r3bl.com`

**Centralized Installers (`r3bl.com/install.sh`, `r3bl.com/install.ps1`) & Version
Manifest (`r3bl.com/versions.jsonc`)**:

- Authored and deployed by external release tooling to both `../r3bl_website/`
  (`https://r3bl.com/`) and `../r3bl-binaries/`
  (`https://github.com/r3bl-org/r3bl-binaries`):
    - `https://r3bl.com/versions.jsonc`: Central JSONC manifest (with preserved comments)
      mapping crate names to `{ "version", "crate_distribution", "released_on" }`.
    - `https://r3bl.com/install.sh`: POSIX shell bootstrap script for Linux & macOS
      (`curl -LsSf https://r3bl.com/install.sh | sh`).
    - `https://r3bl.com/install.ps1`: PowerShell bootstrap script for Windows
      (`irm https://r3bl.com/install.ps1 | iex`).
- **First-Time Install Caveat During Outages**:
    - For open-source crates, all documentation also provides the manual source
      compilation fallback at the end: `cargo install <crate> --force`.

### 6. Release Script & `github.com/r3bl-org/r3bl-binaries`

**External Release Pipeline & Binary Distribution Repository**:

- [x] **Dedicated Binary Distribution Repository (`github.com/r3bl-org/r3bl-binaries`)**:
    - [x] Created at `https://github.com/r3bl-org/r3bl-binaries` (and cloned locally at
          `../r3bl-binaries`) with a single `README.md` commit on `main` and repository
          metadata configured. Used to host GitHub Release binary archives (`.tar.gz` and
          `.sha256`) and mirror `versions.jsonc`, `install.sh`, and `install.ps1` for all
          R3BL open-source and closed-source tools.
- [ ] **External Release Orchestration & Manifest Publisher**:
    - [ ] External release tooling calls
          `r3bl_cmdr::JsoncManifest::try_update_crate_in_jsonc()` to update
          `versions.jsonc` in-place and deploys binary archives, `versions.jsonc`,
          `install.sh`, and `install.ps1` to `../r3bl-binaries` and `../r3bl_website`.

### 7. Website & Documentation

**Consolidated Showcase & Installation Guides Across All Mediums**:

- [ ] **Crate READMEs & `lib.rs` Modules**: Each binary crate (`cmdr`, `build-infra`,
      `rust-analyzer-mcp-server`, and root `r3bl-open-core`) provides:
    - [ ]   1. Primary: 1-line bootstrap script (`https://r3bl.com/install.sh` /
           `https://r3bl.com/install.ps1`).
    - [ ]   2. Feature docs & `--upgrade` / `--upgrade-behavior` CLI flag instructions.
    - [ ]   3. End-of-file fallback instructions: `cargo install <crate> --force`.
- [ ] **R3BL Website (`r3bl.com` in `../r3bl_website/`)**:
    - [ ] Host `versions.jsonc`, `install.sh`, and `install.ps1` at the root of
          `../r3bl_website/` so they are served at
          `https://r3bl.com/{versions.jsonc,install.sh,install.ps1}`.
    - [ ] Update `../r3bl_website/index.html` to consolidate the entire tool suite
          (`r3bl-cmdr` [`giti`, `edi`, `env-source`], `r3bl-build-infra`
          [`cargo-rustdoc-fmt`], `r3bl-rust-analyzer-mcp-server`) with copy-pasteable
          `https://r3bl.com/install.sh` and `https://r3bl.com/install.ps1` commands.
    - [ ] Replace placeholder text with modern product highlights and clear architecture
          diagrams.
    - [ ] Clean up and modernize `../r3bl_website/css/styles.css` and root variable
          styling.
    - [ ] Provide fallback source compilation instructions
          (`cargo install <crate> --force`) for open-source tools.

## Implementation Plan

### Phase 1: Shared Upgrade Module

**Module implementation in `r3bl_tui`**:

- [x] Add `self-replace = "1.5"`, `tar = { version = "0.4", default-features = false }`,
      `flate2 = { version = "1.1.5", default-features = false, features = ["zlib-rs"] }`,
      and `jsonc-parser = { version = "0.34", features = ["serde", "cst"] }` to
      `tui/Cargo.toml`.
- [x] Create `tui/src/core/script/upgrade/constants.rs` centralizing toolchain binary
      names, CLI flags, archive constants, JSONC manifest keys, and error reason strings.
- [x] Create `tui/src/core/script/upgrade/versions_manifest.rs` defining `JsoncManifest`
      (the schema for `versions.jsonc` with `try_parse_jsonc` validating
      `ManifestSchemaVersion` via `ManifestHeader` before dispatching to the
      version-specific parser, and CST-based `try_update_crate_in_jsonc`),
      `ManifestSchemaVersion` (`V1 = 1` witness enum with
      `#[serde(try_from = "u32", into = "u32")]`), `CrateDistribution`, `CrateName`,
      `CrateVersion`, and `CrateVersionMetadata`.
- [x] Create `tui/src/core/script/upgrade/upgrade_types.rs` defining `UpgradeConfig`,
      `UpgradeSource`, `UpdateBehavior`, `UserFeedbackChannel`, and `UpgradeError`.
- [x] Create `tui/src/core/script/upgrade/target_triple.rs` implementing
      `get_host_target_triple()`.
- [x] Create `tui/src/core/script/upgrade/github_release.rs` implementing
      `github.com/r3bl-org/r3bl-binaries` asset URL formatting and HTTP `HEAD`
      availability check.
- [x] Create `tui/src/core/script/upgrade/strategy_resolver.rs` implementing
      `resolve_upgrade_strategy(crate_name, pinned_version, distribution)` and local
      `$PATH` inspection (gating source build fallback on
      `CrateDistribution::OpenSource`).
- [x] Move and generalize PTY/OSC `rustup` & `cargo` build engine from
      `cmdr/src/analytics_client/upgrade_check.rs` into
      `tui/src/core/script/upgrade/source_build.rs` (always passing
      `--version <pinned_version>` from `versions.jsonc` to `cargo +nightly install`).
- [x] Create `tui/src/core/script/upgrade/version_check.rs` querying
      `https://r3bl.com/versions.jsonc` via `JsoncManifest::try_parse_jsonc`, comparing
      versions strictly via `(major, minor, patch)` (`remote_version > local_version`),
      with both async and sync-thread variants and `AtomicU8` state management.
- [x] Create `tui/src/core/script/upgrade/run_upgrade.rs` implementing
      `handle_upgrade_at_exit()`, `handle_upgrade_at_exit_sync()`,
      `run_upgrade_cli_arg_interceptor()`, `run_upgrade_sync()`, binary streaming download
      from `github.com/r3bl-org/r3bl-binaries` with `.sha256` check, `.tar.gz` archive
      extraction, `self_replace` on the running executable plus updating sibling binaries
      in the archive, and automatic source fallback execution (pinned to
      `--version <pinned_ver>`) when `CrateDistribution::OpenSource`.
- [x] Create `tui/src/core/script/upgrade/ui_strings.rs` providing parameterized
      formatters.
- [x] Create `tui/src/core/script/upgrade/mod.rs` with private modules and public barrel
      exports.
- [x] Wire `upgrade` into `tui/src/core/script/mod.rs` and export at `r3bl_tui::*`.
- [x] Add unit tests in `tui/src/core/script/upgrade/` for `versions.jsonc` parsing (with
      comments, trailing commas, and unsupported `schema_version` rejection), CST
      comment-preserving updates, strict `remote > local` version comparison,
      `upgrade_types`, `versions_manifest`, strategy resolver (both
      `CrateDistribution::OpenSource` and `CrateDistribution::ClosedSource`), target
      triple, and UI strings.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions (unreviewed files merged into Phase 2
      review).
    - [x] `tui/Cargo.toml`
    - [x] `tui/src/core/script/upgrade/constants.rs`

### Phase 2: Refactor & Unify Upgrade Engine Architecture

**Refactoring to Simplify and Unify `r3bl_tui::core::script::self_update`**:

- [x] Align on strategy scope:
    - Retain Nightly source build fallback since `tui` and `cmdr` strictly require
      `#![feature(...)]`.
    - Eliminate the broken `BuildFromSourceCargo` variant. Empirical check confirmed
      standalone Cargo (`/usr/bin/cargo +nightly`) fails with
      `error: no such command: '+nightly'`, and building on stable fails with
      `error: the option 'Z' is only accepted on the nightly compiler` plus `#![feature]`
      rejections.
    - Require `rustup` on `$PATH` as the single prerequisite for open-source source
      fallback.
- [x] Collapse `UpgradeSource` from 3 variants to 2:
    - `GithubReleaseArtifact { archive_url, sha256_url, pinned_version }`
    - `BuildFromSourceRustup { pinned_version }`
    - Remove `BuildFromSourceCargo` and update all matching sites across
      `strategy_resolver.rs`, `run_upgrade.rs`, `upgrade_types.rs`, and tests.
    - Update `Cross-Platform Test Matrix` Scenario 3 and 7 to remove
      `BuildFromSourceCargo` and verify missing `rustup` cleanly yields
      `Err(NoViableUpgradeSource)`.
- [x] Organize upgrade engine into cohesive domain submodules (`parser/`, `update/`, and
      `upgrade/`):
    - `entry_point.rs`: Public consumer entry points (`handle_upgrade_at_exit` / `_sync`,
      `run_upgrade_cli_arg_interceptor` / `_sync`, `run_upgrade` / `_sync`).
    - `parser/`: `crate_distribution.rs`, `crate_metadata.rs`, `crate_name.rs`,
      `crate_release_date.rs`, `crate_version.rs`, `jsonc_manifest.rs`, `mod.rs` (manifest
      schema and parsing).
    - `update/`: `cli_arg.rs`, `config.rs`, `engine.rs`, `state.rs`, `mod.rs`
      (checking/fetching metadata and state cache).
    - `upgrade/`: `build.rs`, `download.rs`, `prompt.rs`, `release.rs`, `strategy.rs`,
      `strings.rs`, `swap.rs`, `target.rs`, `mod.rs` (asset download, verification, binary
      swap, compilation).
- [x] Natural sync vs async API separation:
    - `start_background_version_check` is unified and works in both async and sync
      binaries via `tokio::runtime::Handle::try_current()` with OS thread fallback.
    - Async binaries (`giti`, `edi`) use async APIs: `start_background_version_check`,
      `run_upgrade`, `run_upgrade_cli_arg_interceptor`, `handle_upgrade_at_exit`.
    - Sync binaries (`env-source`, `rust-analyzer-mcp-server`) use
      `start_background_version_check` and dedicated `_sync` APIs with lightweight private
      `current_thread` runtimes on the calling thread: `run_upgrade_sync`,
      `run_upgrade_cli_arg_interceptor_sync`, `handle_upgrade_at_exit_sync`.
    - Zero runtime bridge complexity: no dynamic `runtime_flavor` checks, no
      `block_in_place`, and no nested runtime issues.
- [x] Consolidate bifurcated global state (`UPGRADE_CHECK_STATE` and
      `CACHED_CRATE_METADATA`) into a unified `UpgradeCheckState` enum in dedicated
      `upgrade_check_state.rs` to make illegal states unrepresentable:
    - [x] Define `UpgradeCheckStatus` enum with variants `NotChecked`, `Checking`,
          `UpToDate(CrateVersionMetadata)`, `UpgradeAvailable(CrateVersionMetadata)`,
          `CheckFailed`.
    - [x] Replace `UPGRADE_CHECK_STATE: AtomicU8` and
          `CACHED_CRATE_METADATA: ScopedMutex<Option<...>>` with a single static
          `SINGLETON: LazyLock<ScopedMutex<UpgradeCheckCache, { PanicOnSpecificLockNesting }>>`
          initialized to `UpgradeCheckCache::default()`
          (`UpgradeCheckStatus::NotChecked`).
    - [x] Update `check_engine::check_for_upgrade` to atomically write the resolved state
          (`UpToDate(metadata)` or `UpgradeAvailable(metadata)`) or `CheckFailed`.
    - [x] Update consumer call sites in `upgrade_entry_point.rs`:
        - `handle_upgrade_at_exit`: pattern match
          `UpgradeCheckState::UpgradeAvailable(metadata)` directly.
        - `handle_upgrade_at_exit_sync`: check `is_upgrade_available()`.
        - `ensure_version_checked_and_handle_at_exit`: check
          `UpgradeCheckState::NotChecked`.
    - [x] Update documentation, ASCII lifecycle diagrams, and unit tests.
- [x] Update documentation, Unicode decision tree, and all unit tests in
      `tui/src/core/script/self_update/` to reflect the unified architecture.
- [x] Align module naming with package manager conventions (`upgrade` -> `self_update`,
      `check` -> `update`, `update` -> `upgrade`):
    - [x] Rename subdirectories (`update/` -> `upgrade/`, `check/` -> `update/`,
          `upgrade/` -> `self_update/`).
    - [x] Update `tui/src/core/script/mod.rs` (`pub mod self_update;`) and barrel
          re-exports.
    - [x] Update internal module declarations and intra-doc links to `self_update::*`.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions (remaining files moved to
      `cmdr/src/self_upgrade/` in Phase 2.5).
    - [x] `tui/src/core/script/self_update/constants.rs`
    - [x] `tui/src/core/script/self_update/parser/mod.rs`
    - [x] `tui/src/core/script/self_update/parser/jsonc_manifest.rs`
    - [x] `tui/src/core/script/self_update/parser/crate_distribution.rs`
    - [x] `tui/src/core/script/self_update/parser/crate_metadata.rs`
    - [x] `tui/src/core/script/self_update/parser/crate_name.rs`
    - [x] `tui/src/core/script/self_update/parser/crate_release_date.rs`
    - [x] `tui/src/core/script/self_update/parser/crate_version.rs`
    - [x] `tui/src/core/script/self_update/update/config.rs`
    - [x] `tui/src/core/script/self_update/update/cli_arg.rs`
    - [x] `tui/src/core/script/self_update/update/state.rs`

### Phase 2.1: Move `self_upgrade` from `r3bl_tui` to `r3bl-cmdr`

**Architectural relocation of `self_upgrade/` from `r3bl_tui` into `r3bl-cmdr`
(`cmdr/src/self_upgrade/`)**:

- [x] Move `tui/src/core/script/self_update/` directory to `cmdr/src/self_upgrade/`.
- [x] Remove `self_update` module declaration and barrel re-export from
      `tui/src/core/script/mod.rs`.
- [x] Remove `self-replace`, `tar`, and `jsonc-parser` from `tui/Cargo.toml` (keeping
      `r3bl_tui` free of R3BL binary self-upgrade distribution logic).
- [x] Add `self-replace`, `tar`, `flate2`, `jsonc-parser`, `chrono`, `sha2`, `thiserror`,
      `futures-util`, and `reqwest` `"stream"` feature to `cmdr/Cargo.toml` (plus
      `similar` in `[dev-dependencies]`).
- [x] Create `cmdr/src/global_constants.rs` containing `DEBUG_ANALYTICS_CLIENT` and
      `DEBUG_SELF_UPGRADE`. Attach `pub mod global_constants;` and
      `pub use global_constants::*;` in `cmdr/src/lib.rs`.
- [x] Attach `pub mod self_upgrade;` and `pub use self_upgrade::*;` in `cmdr/src/lib.rs`.
- [x] Update `crate::` imports and rustdoc intra-doc links in `cmdr/src/self_upgrade/**`
      to reference `r3bl_tui::*` for TUI primitives and `crate::self_upgrade::*` for
      `self_upgrade` items.
- [x] Standardize all debug constants across `r3bl-cmdr` and `r3bl_tui` following
      Convention B (`DEBUG_TUI`, `DEBUG_SCRIPT`, `DEBUG_CHOOSE`, `DEBUG_ANALYTICS_CLIENT`,
      `DEBUG_SELF_UPGRADE`).
- [x] Verify workspace compiles (`./check.fish --check`), unit tests pass
      (`cargo test -p r3bl-cmdr --lib self_upgrade`), and docs build cleanly
      (`./check.fish --quick-doc`).
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `cmdr/src/global_constants.rs`
    - [x] `cmdr/src/self_upgrade/constants.rs`
    - [x] `cmdr/src/self_upgrade/parser/mod.rs`
    - [x] `cmdr/src/self_upgrade/parser/jsonc_manifest_entry_point.rs`
    - [x] `cmdr/src/self_upgrade/parser/crate_distribution.rs`
    - [x] `cmdr/src/self_upgrade/parser/crate_metadata.rs`
    - [x] `cmdr/src/self_upgrade/parser/crate_name.rs`
    - [x] `cmdr/src/self_upgrade/parser/crate_release_date.rs`
    - [x] `cmdr/src/self_upgrade/parser/crate_version.rs`
    - [x] `cmdr/src/self_upgrade/config/mod.rs`
    - [x] `cmdr/src/self_upgrade/config/cli_arg.rs`
    - [x] `cmdr/src/self_upgrade/config/crate_identity.rs`
    - [x] `cmdr/src/self_upgrade/config/upgrade_config.rs`
    - [x] `cmdr/src/self_upgrade/config/user_feedback_channel.rs`
    - [x] `cmdr/src/self_upgrade/check/mod.rs`
    - [x] `cmdr/src/self_upgrade/check/state.rs`
    - [x] `cmdr/src/self_upgrade/check/check_entry_point.rs`
    - [x] `cmdr/src/self_upgrade/check/manifest_query.rs`

### Phase 2.2: Make illegal states unrepresentable (Typestate Pattern + Single Entry Point + `UpgradeCheckStatus` Process-Wide Singleton)

#### Problem Statement: Disjointed Entry Points, Witness Proliferation & Lifecycle Gaps

Previously, the self-upgrade engine suffered from six design flaws:

1. **Two Disconnected Entry Points**: `UpgradeConfig::start_background_version_check` and
   `UpgradeConfig::run_upgrade` were separate methods, forcing every binary's `main()` to
   inspect `cli_arg.upgrade_args.upgrade` manually and branch on which method to call.
2. **Proliferation of Ad-Hoc Witness Types**: Instead of one type progressing through
   compiler-verified states, the lifecycle was scattered across 6 wrapper types
   (`UpgradeBehavior`, `ExitUpgradeAction`, `UpgradeConfig`, `DisabledUpgradeCheck`,
   `EnabledUpgradeConfig`, `UpgradeCheckHandle`, and `AvailableUpgrade`).
3. **Redundant Macro Layer (`upgrade_config!` + `upgrade_journey!`)**: Production callers
   only construct `UpgradeJourney<NotStarted>`, while tests overriding URLs, versions, or
   cooldown policies call `UpgradeConfig::new()` directly, making a separate
   `upgrade_config!` macro unnecessary cognitive overhead.
4. **Unchecked Multiple `.start()` Calls**: `SINGLETON` did not guard against duplicate
   `UpgradeJourney::start()` calls in `main()`, allowing multiple dedicated check threads
   to spawn and race via `UpgradeCheckStatus::set()`.
5. **Path 1 (`--upgrade`) Bypassed Typestate & `SINGLETON`**: `ImmediateUpgrade` ran a
   separate codepath (`get_remote_crate_metadata`) without transitioning `UpgradeJourney`
   or recording state in `SINGLETON`, leaving `SINGLETON` stuck at `NotChecked`, giving no
   in-progress (`Upgrading`) or terminal (`Completed`) state, and returning
   `miette::Result<()>` to `main()` (which caused `miette`'s `Termination` handler to
   print a duplicate `Error:` on `stderr` and leak onto `stderr` even in
   `UserFeedbackChannel::Quiet` mode).
6. **Scenario 2c Race on Short-Lived Binaries (`thread_check_handle` Dropped)**: Spawning
   the check thread dropped its `JoinHandle`. When a fast CLI (`env-source`,
   `cargo-rustdoc-fmt`, or a quick subcommand) reached `handle_upgrade_on_exit()` in a few
   milliseconds while the manifest request was still `Checking`, exit handling silently
   no-opped and OS exit killed the check thread mid-flight.

---

#### Selected Architecture: Typestate `UpgradeJourney<State>` + Single Entry Point + Coordinated `SINGLETON`

We unify the entire self-upgrade lifecycle around **three core pillars**:

1. **Clean CLI-to-Domain Boundary (`UpgradeCLIArgs` -> `UpgradePolicy` &
   `UpgradeStrategy`), Timeouts & `1h` `mtime` Cooldown**:
    - `UpgradeCLIArgs` resolves `--upgrade` and `--upgrade-behavior` into
      `UpgradePolicy::Enabled(UpgradeStrategy)` or `UpgradePolicy::Disabled`.
    - `UpgradeStrategy` (`Immediate` vs. `DedicatedThread(UpgradeActionOnExit)`) governs
      both the manifest HTTP timeout (`5s` when `--upgrade` is explicitly requested vs.
      `2s` on the dedicated check thread) and the `1h` `mtime` cache cooldown
      (`<cache_dir>/r3bl/upgrade-check/<crate_name>.last_check`, bypassed by `--upgrade`
      and touched only after a verified `UpToDate` check or `Upgraded` completion):

    ```rust
    /// Manifest HTTP timeouts & cache cooldown in `cmdr/src/self_upgrade/constants.rs`.
    pub mod manifest_timeouts {
        use std::time::Duration;

        /// Timeout for `versions.jsonc` during explicit `--upgrade` (`UpgradeStrategy::Immediate`).
        pub const IMMEDIATE_MANIFEST_TIMEOUT: Duration = Duration::from_secs(5);

        /// Timeout for `versions.jsonc` on the dedicated check thread (`UpgradeStrategy::DedicatedThread`).
        pub const DEDICATED_THREAD_MANIFEST_TIMEOUT: Duration = Duration::from_secs(2);

        /// Cooldown window (`1h`) after `UpToDate` or `Upgraded` before `DedicatedThread` re-checks.
        pub const DEDICATED_THREAD_CHECK_COOLDOWN: Duration = Duration::from_secs(60 * 60);
    }

    /// Cooldown stamp policy carried by static [`UpgradeConfig`].
    #[derive(Debug, Clone, PartialEq, Eq, Default)]
    pub enum CooldownPolicy {
        /// Production policy (`dirs::cache_dir()/r3bl/upgrade-check/<crate_name>.last_check`).
        #[default]
        DefaultCacheDir,
        /// Custom stamp path for isolated filesystem unit tests.
        CustomStampPath(std::path::PathBuf),
        /// Disables disk cooldown reads and writes for deterministic unit tests.
        Disabled,
    }

    /// Strategy for an enabled self-upgrade inside [`UpgradePolicy::Enabled`] and [`Checking`].
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum UpgradeStrategy {
        /// Path 1 (`--upgrade`): Synchronous check (`5s` timeout, bypasses cooldown) and
        /// immediate upgrade in `start()`.
        Immediate,
        /// Path 2 (`--upgrade-behavior`): Dedicated check thread (`2s` timeout, `1h`
        /// `mtime` cooldown) spawned in `start()` and joined in `handle_upgrade_on_exit()`.
        DedicatedThread(UpgradeActionOnExit),
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum UpgradePolicy {
        Enabled(UpgradeStrategy),
        Disabled,
    }
    ```

2. **Immutable `UpgradeConfig` + 9-State Typestate `UpgradeJourney<State>`**:
    - `UpgradeConfig` holds all immutable session settings (`crate_identity`,
      `user_feedback_channel`, `upgrade_policy`, `jsonc_manifest_url`,
      `binaries_release_base_url`, `cooldown_policy`) passed explicitly to
      `UpgradeConfig::new(...)` so unit tests pass `CooldownPolicy::Disabled` or
      `CooldownPolicy::CustomTimestampFilePath(...)` without ever touching `~/.cache`. The
      `upgrade_config!` macro is deleted; its architectural rustdoc
      (`# Why provide a declarative macro instead of a constructor?` and `# Panics`) and
      hygiene-hardened `CARGO_PKG_NAME` / `CARGO_PKG_VERSION` expansion move directly onto
      `upgrade_journey!`.
    - `UpgradeJourney<State>` holds private `config: UpgradeConfig` and `state: State`
      across 9 plain state structs (`NotStarted`, `Disabled`, `CooldownSkipped`,
      `Checking`, `UpToDate`, `UpgradeAvailable`, `CheckFailed`, `Upgrading`,
      `Completed`). `NotStarted` is a unit struct (`UpgradeJourney::new(config)` takes a
      single `UpgradeConfig` parameter), while `Checking` (`strategy: UpgradeStrategy`)
      and `UpgradeAvailable` (`action_on_exit: UpgradeActionOnExit`) carry narrowed
      witnesses unpacked from `config.get_upgrade_policy()`. All `into_*` transition
      methods are `pub(super)` so external crates cannot forge state transitions:

    ```rust
    pub trait UpgradeJourneyStateMarker {}

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct UpgradeJourney<State: UpgradeJourneyStateMarker> {
        config: UpgradeConfig,
        state: State,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct NotStarted;
    impl UpgradeJourneyStateMarker for NotStarted {}

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Disabled;
    impl UpgradeJourneyStateMarker for Disabled {}

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CooldownSkipped;
    impl UpgradeJourneyStateMarker for CooldownSkipped {}

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Checking {
        pub strategy: UpgradeStrategy,
    }
    impl UpgradeJourneyStateMarker for Checking {}

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct UpToDate {
        pub metadata: CrateMetadata,
    }
    impl UpgradeJourneyStateMarker for UpToDate {}

    /// Only entered on `UpgradeStrategy::DedicatedThread`; `UpgradeStrategy::Immediate` transitions
    /// directly from `Checking` to `Upgrading` so `Immediate` is unrepresentable at exit.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct UpgradeAvailable {
        pub action_on_exit: UpgradeActionOnExit,
        pub metadata: CrateMetadata,
    }
    impl UpgradeJourneyStateMarker for UpgradeAvailable {}

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CheckFailed {
        /// Full diagnostic chain formatted via `format!("{err:#}")`.
        pub error: String,
    }
    impl UpgradeJourneyStateMarker for CheckFailed {}

    /// Active upgrade or interactive exit prompt in progress after `SINGLETON` leaf lock
    /// is released (`Checking -> Upgrading` in Path 1; `UpgradeAvailable -> Upgrading` in Path 2).
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Upgrading {
        pub metadata: CrateMetadata,
    }
    impl UpgradeJourneyStateMarker for Upgrading {}

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum UpgradeResolution {
        Upgraded,
        UpgradeFailed(String),
        DeclinedByUser,
        SkippedNonInteractive,
        NotifiedOnly,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Completed {
        pub metadata: CrateMetadata,
        pub resolution: UpgradeResolution,
    }
    impl UpgradeJourneyStateMarker for Completed {}
    ```

3. **Single Entry Point (`UpgradeJourney::<NotStarted>::start`) & Coordinated
   `SINGLETON`**:
    - Calling `.start(self) -> StartOutcome` atomically claims `SINGLETON` if its status
      is `UpgradeCheckStatus::NotStarted`. Any subsequent `.start()` call in the same
      process leaves `SINGLETON` untouched, spawns zero threads, logs via
      `DEBUG_SELF_UPGRADE`, and returns `StartOutcome::AlreadyStarted`.
    - `StartOutcome::UpgradedAndExit(ImmediateUpgradeResult)` returns a `Copy` domain enum
      (`Succeeded | Failed`, with `exit_code(self) -> std::process::ExitCode`) instead of
      `miette::Result<()>` so `UserFeedbackChannel` remains the single output emitter
      (preventing duplicate `stderr` errors and `Quiet` leaks in
      `fn main() -> CommonResult`):

        ```rust
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum ImmediateUpgradeResult {
            Succeeded,
            Failed,
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum StartOutcome {
            /// `--upgrade` executed; `main()` exits (`0` on `Succeeded`, `1` on `Failed`).
            UpgradedAndExit(ImmediateUpgradeResult),
            /// Normal startup (`DedicatedThread` or `Disabled`); `main()` continues.
            ContinueApplication,
            /// `start()` was already called earlier in this process; duplicate ignored.
            AlreadyStarted,
        }
        ```

    - Inside `upgrade_check_cache.rs`, `SINGLETON` protects a private struct pairing
      `status: UpgradeCheckStatus` with `thread_check_handle: Option<JoinHandle<()>>`
      using standard fail-fast `ScopedMutex` locking (and `SINGLETON.write` +
      `handle.join()` drain inside `#[cfg(test)] pub(crate) fn reset_for_testing()`):

        ```rust
        #[derive(Debug, Default)]
        struct UpgradeSingletonState {
            status: UpgradeCheckStatus,
            thread_check_handle: Option<std::thread::JoinHandle<()>>,
        }

        #[derive(Debug, Clone, PartialEq, Eq, Default)]
        pub enum UpgradeCheckStatus {
            #[default]
            NotStarted,
            Disabled(UpgradeJourney<Disabled>),
            CooldownSkipped(UpgradeJourney<CooldownSkipped>),
            Checking(UpgradeJourney<Checking>),
            UpToDate(UpgradeJourney<UpToDate>),
            UpgradeAvailable(UpgradeJourney<UpgradeAvailable>),
            CheckFailed(UpgradeJourney<CheckFailed>),
            Upgrading(UpgradeJourney<Upgrading>),
            Completed(UpgradeJourney<Completed>),
        }
        ```

    - In `UpgradeStrategy::DedicatedThread`, `.start()` spawns a single named OS thread
      via `std::thread::Builder::new().name("self-upgrade-check".into()).spawn(...)`
      (gracefully transitioning `Err(io_err)` to `CheckFailed` without panicking), and
      builds `tokio::runtime::Builder::new_current_thread()` directly inside
      `catch_unwind` on that thread (avoiding a nested second OS thread while leaving
      `tui/src/core/common/run_in_tokio.rs` untouched for Path 1 and exit upgrades).
    - At shutdown, `UpgradeCheckStatus::handle_upgrade_on_exit()`:
        1. Takes `thread_check_handle.take()` and releases `SINGLETON` **before** calling
           `handle.join()` (preventing lock-order deadlock and guaranteeing Scenario 2c
           waits up to `2s` when cooldown was expired).
        2. Atomically transitions `UpgradeCheckStatus::UpgradeAvailable(journey)` into
           `UpgradeCheckStatus::Upgrading(upgrading)` inside `SINGLETON.write` (keeping
           `SINGLETON != NotStarted` and ensuring idempotent one-shot execution while the
           leaf lock is released), runs `Auto` / `PromptToUpgrade` / `NotifyOnly`, touches
           the `1h` cooldown stamp if `Upgraded`, and stores
           `UpgradeCheckStatus::Completed(completed)`.

#### Three Binary Execution Paths & Typestate Lifecycle

```text
SINGLETON starts at: UpgradeCheckStatus::NotStarted (thread_check_handle: None)
  │
  ▼
let journey = upgrade_journey!(UserFeedbackChannel::Stdout, cli_arg.upgrade_args);
let outcome = journey.start(); // handle StartOutcome::UpgradedAndExit(Succeeded | Failed)
  │
  ├── [Duplicate .start() when SINGLETON != NotStarted]
  │     └── 0 threads spawned, SINGLETON unchanged ──► StartOutcome::AlreadyStarted
  │
  ├── Path 1: <bin> --upgrade (UpgradePolicy::Enabled(UpgradeStrategy::Immediate), 5s timeout, bypasses cooldown)
  │     └── SINGLETON = Checking(Immediate) ──► manifest check via run_in_tokio:
  │           ├── CheckFailed(err) ──► channel.eprintln(retry) ──► UpgradedAndExit(Failed)
  │           ├── UpToDate(meta)   ──► touch 1h stamp + println ──► UpgradedAndExit(Succeeded)
  │           └── Upgrading(meta)  ──► perform_upgrade() via run_in_tokio:
  │                 ├── Ok(())   ──► touch 1h stamp, Completed(Upgraded)      ──► UpgradedAndExit(Succeeded)
  │                 └── Err(err) ──► Completed(UpgradeFailed(format!("{err:#}"))) ──► UpgradedAndExit(Failed)
  │
  ├── Path 2: <bin> [--upgrade-behavior=auto|prompt|notify] (DedicatedThread, 2s timeout, 1h mtime cooldown)
  │     ├── Within 1h mtime stamp ──► SINGLETON = CooldownSkipped, 0 threads ──► ContinueApplication (~0.01 ms)
  │     ├── Expired/missing stamp ──► SINGLETON = Checking(DedicatedThread(action)) + Some(handle)
  │     │     (single `self-upgrade-check` OS thread builds `new_current_thread` Tokio runtime directly)
  │     └── At exit: UpgradeCheckStatus::handle_upgrade_on_exit()
  │           1. Takes thread_check_handle.take() (releases lock), then calls h.join():
  │                ├── CooldownSkipped or Scenario 2a/2b (already done): returns in 0 ms
  │                └── Scenario 2c (short main() on expired stamp): waits up to 2s for completion
  │           2. Inspects resolved SINGLETON:
  │                ├── Scenario 2a [UpgradeAvailable]: atomically sets SINGLETON = Upgrading(meta),
  │                │     runs Auto / PromptToUpgrade (PromptUpgradeDecision) / NotifyOnly,
  │                │     touches 1h stamp if Upgraded, sets SINGLETON = Completed(...)
  │                └── Scenario 2b [CooldownSkipped | UpToDate | CheckFailed]: 0 ms silent no-op
  │
  └── Path 3: <bin> --upgrade-behavior=disable (UpgradePolicy::Disabled)
        └── SINGLETON = Disabled, 0 threads, 0 I/O ──► ContinueApplication ──► 0 ms no-op exit
```

#### Module Organization & File Responsibilities

| File                                                   | Responsibility in Typestate Architecture                                                                                                                                                                                                                                                                                                                                                                           |
| :----------------------------------------------------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **`cmdr/src/self_upgrade/constants.rs`**               | Defines `upgrade_cli_args::{UPGRADE_ARG_LONG, UPGRADE_BEHAVIOR_ARG_LONG}`, `manifest_timeouts::{IMMEDIATE_MANIFEST_TIMEOUT (5s), DEDICATED_THREAD_MANIFEST_TIMEOUT (2s), DEDICATED_THREAD_CHECK_COOLDOWN (1h)}`, and `check_cooldown_cache`.                                                                                                                                                                       |
| **`cmdr/src/self_upgrade/config/cli_arg.rs`**          | Opaque `clap` CLI boundary adapter (`UpgradeCLIArgs` with private fields + private `CliUpgradeBehavior`) and `From<UpgradeCLIArgs> for UpgradePolicy`.                                                                                                                                                                                                                                                             |
| **`cmdr/src/self_upgrade/config/upgrade_policy.rs`**   | Pure domain model: `UpgradePolicy` (`Enabled(UpgradeStrategy)`, `Disabled`), `UpgradeStrategy` (`Immediate`, `DedicatedThread(UpgradeActionOnExit)` with `manifest_timeout()`), and `UpgradeActionOnExit` (`PromptToUpgrade`, `Auto`, `NotifyOnly`).                                                                                                                                                               |
| **`cmdr/src/self_upgrade/config/upgrade_config.rs`**   | Immutable `UpgradeConfig` (`crate_name`, `current_version`, `user_feedback_channel`, `upgrade_policy`, `jsonc_manifest_url`, `cooldown_policy`) + `CooldownPolicy` (`DefaultCacheDir`, `CustomTimestampFilePath(PathBuf)`, `Disabled`) constructed via `UpgradeConfig::new()`. `upgrade_config!` macro is deleted.                                                                                                 |
| **`cmdr/src/self_upgrade/check/cooldown.rs`**          | `CooldownPolicy`, `CooldownStatus` (`Active`, `ExpiredOrMissing`), `CooldownPolicy::status(&CrateName)`, and `CooldownPolicy::touch(&CrateName)` using `<cache_dir>/r3bl/<crate_name>/self_upgrade_last_check` `mtime`.                                                                                                                                                                                            |
| **`cmdr/src/self_upgrade/check/check_entry_point.rs`** | `check_for_upgrade(journey: UpgradeJourney<Checking>)`, `check_for_immediate_upgrade(journey: UpgradeJourney<Checking>)`, `try_get_jsonc_manifest_from_url(url: &EndpointUrl, timeout: Duration)` parameterized by `UpgradeStrategy::manifest_timeout()`, and pure `extract_crate_metadata_from_manifest_result` preserving full `format!("{err:#}")` diagnostic chains and touching the `1h` stamp on `UpToDate`. |
| **`cmdr/src/self_upgrade/upgrade/prompt.rs`**          | `PromptUpgradeDecision` (`UpgradeNow`, `DeclinedByUser`, `SkippedNonInteractive`) returned by `prompt_user_to_upgrade` (eliminating boolean blindness on non-interactive terminals).                                                                                                                                                                                                                               |
| **`cmdr/src/self_upgrade/upgrade/bin_name.rs`**        | `get_current_bin_name()` (replaces deleted `metadata_resolver.rs` and obsolete `get_remote_crate_metadata()`).                                                                                                                                                                                                                                                                                                     |
| **`cmdr/src/self_upgrade/upgrade_journey/`**           | Hygiene-hardened `upgrade_journey!` macro (`upgrade_journey_macro.rs`), `UpgradeJourney<State>` (`upgrade_journey_struct.rs`), 9 per-file typestate structs (`typestates/{not_started..completed}.rs`), `pub(in crate::self_upgrade) fn into_*` transitions, `UpgradeResolution`, `ImmediateUpgradeResult`, `StartOutcome` (`start_outcome.rs`), `.start(self)`, and `Upgrading::execute_action_on_exit`.          |
| **`cmdr/src/self_upgrade/upgrade_check_cache.rs`**     | Private `UpgradeSingletonState` (`status` + `thread_check_handle`), 9-variant `UpgradeCheckStatus` (dead `try_get_crate_metadata` and `is_upgrade_available` removed), atomic single-start claim, deadlock-safe `join()` + `UpgradeAvailable -> Upgrading -> Completed` exit handling, and poison-safe + thread-draining `#[cfg(test)] pub(crate) fn reset_for_testing()`.                                         |
| **`cmdr/src/self_upgrade/mod.rs`**                     | Module-level narrative and ASCII diagram covering all 3 binary execution paths, Scenario 2a/2b/2c (`thread_check_handle` join + `1h` `mtime` cooldown), `5s` vs. `2s` timeouts, `ImmediateUpgradeResult`, and `StartOutcome::AlreadyStarted`.                                                                                                                                                                      |

#### Implementation Steps for Phase 2.2

- [x] Initial typestate scaffolding (`UpgradePolicy`, static `UpgradeConfig`,
      `upgrade_journey_public_api.rs`, `upgrade_check_cache.rs`, `check_entry_point.rs`).
- [x] Update `cmdr/src/self_upgrade/constants.rs` and
      `cmdr/src/self_upgrade/check/manifest_query.rs`:
    - Fix `upgrade_cli_args` (`UPGRADE_ARG_LONG = "upgrade"`,
      `UPGRADE_BEHAVIOR_ARG_LONG = "upgrade-behavior"`).
    - Add `manifest_timeouts` (`IMMEDIATE_MANIFEST_TIMEOUT = 5s`,
      `DEDICATED_THREAD_MANIFEST_TIMEOUT = 2s`, `DEDICATED_THREAD_CHECK_COOLDOWN = 1h`)
      and `check_cooldown_cache` constants.
    - Parameterize `try_get_jsonc_manifest_from_url(url: &str, timeout: Duration)`.
- [x] Update `cmdr/src/self_upgrade/config/` (`cli_arg.rs`, `upgrade_policy.rs`,
      `upgrade_config.rs`, `crate_identity.rs`) and add
      `cmdr/src/self_upgrade/check/cooldown.rs`:
    - Wire `UPGRADE_ARG_LONG` and `UPGRADE_BEHAVIOR_ARG_LONG` into `UpgradeCLIArgs` (with
      private fields and private `CliUpgradeBehavior`), convert cleanly into
      `UpgradePolicy` (`Enabled(UpgradeStrategy) | Disabled`) via `From<UpgradeCLIArgs>`,
      and extract `UpgradePolicy`, `UpgradeStrategy`
      (`Immediate | DedicatedThread(UpgradeActionOnExit)` with `manifest_timeout()`), and
      `UpgradeActionOnExit` into `cmdr/src/self_upgrade/config/upgrade_policy.rs`.
    - Add `CooldownPolicy` (`DefaultCacheDir`, `CustomStampPath(PathBuf)`, `Disabled`) to
      `UpgradeConfig::new(...)`, delete `upgrade_config!`, and update doc comments in
      `upgrade_config.rs` and `crate_identity.rs`.
    - Create `cmdr/src/self_upgrade/check/cooldown.rs` (wired in `check/mod.rs`) with
      `CooldownStatus`, `check_cooldown_status`, and `record_successful_check` plus
      hermetic unit tests using `CooldownPolicy::CustomStampPath` and `Disabled`.
- [x] Clean up `cmdr/src/self_upgrade/upgrade/` (`prompt.rs`, `bin_name.rs`,
      `download.rs`, `build.rs`, `mod.rs`):
    - Replace `bool` return in `prompt_user_to_upgrade` (`prompt.rs`) with
      `PromptUpgradeDecision` (`UpgradeNow`, `DeclinedByUser`, `SkippedNonInteractive`).
    - Move `get_current_bin_name()` into `cmdr/src/self_upgrade/upgrade/bin_name.rs`,
      delete `cmdr/src/self_upgrade/upgrade/metadata_resolver.rs` (removing dead
      `get_remote_crate_metadata`), and update `upgrade/mod.rs`.
    - Delete unused `download_and_verify_archive_sync` (`download.rs`) and
      `run_source_build_with_spinner_sync` (`build.rs`).
- [x] Update `cmdr/src/self_upgrade/upgrade_check_cache.rs`,
      `cmdr/src/self_upgrade/check/check_entry_point.rs`, and
      `cmdr/src/self_upgrade/upgrade_journey_public_api.rs`:
    - Expand `UpgradeJourney<State>` and `UpgradeCheckStatus` to 9 states (`NotStarted`,
      `Disabled`, `CooldownSkipped`, `Checking`, `UpToDate`, `UpgradeAvailable`,
      `CheckFailed`, `Upgrading`, `Completed`) with `pub(super)` `into_*` transitions,
      `ImmediateUpgradeResult`, and `StartOutcome::AlreadyStarted`.
    - Migrate architectural rustdoc + hygiene-hardened `::core::env!` expansion onto
      `upgrade_journey!`.
    - Delete dead `try_get_crate_metadata()` and `is_upgrade_available()` from
      `upgrade_check_cache.rs`, wrap `status` + `thread_check_handle` inside private
      `UpgradeSingletonState`, implement deadlock-safe `join()` + atomic
      `UpgradeAvailable -> Upgrading -> Completed` exit handling, and implement
      poison-safe + thread-draining `#[cfg(test)] pub(crate) fn reset_for_testing()`.
    - Build `new_current_thread` Tokio runtime directly inside `catch_unwind` on the
      `self-upgrade-check` OS thread spawned via `std::thread::Builder`, format errors
      with `format!("{err:#}")`, and touch the `1h` cooldown stamp only on `UpToDate` and
      `UpgradeResolution::Upgraded`.
    - Rewrite `#[serial]` unit tests (with leading `reset_for_testing()` and local mock
      TCP server for Path 1) covering duplicate `.start()`, `1h` cooldown skip/expiry,
      Path 1 `SINGLETON` transitions + `ImmediateUpgradeResult`, Scenario 2c `join()`
      completion, `Upgrading` one-shot guard, and idempotent `handle_upgrade_on_exit()`.
- [x] Update `cmdr/src/self_upgrade/mod.rs`
      (`# Main Entry Point: [UpgradeJourney] & Typestate Lifecycle` and intra-doc links)
      to document all 3 binary execution paths, Scenario 2a/2b/2c (`thread_check_handle`
      join + `1h` `mtime` cooldown), `5s` vs. `2s` timeouts, `ImmediateUpgradeResult`, and
      `StartOutcome::AlreadyStarted`.
- [x] Consolidate `UpgradePolicy` into immutable `UpgradeConfig` (Variant 2A):
    - Move `upgrade_policy: UpgradePolicy` + `get_upgrade_policy(&self) -> UpgradePolicy`
      into `UpgradeConfig` (`cmdr/src/self_upgrade/config/upgrade_config.rs`) and update
      `UpgradeConfig::new(...)` and rustdoc in `upgrade_config.rs` and
      `upgrade_policy.rs`.
    - Collapse `NotStarted` into a unit struct (`pub struct NotStarted;`), simplify
      `UpgradeJourney::<NotStarted>::new(config: UpgradeConfig)`, dispatch `.start(self)`
      via `self.config.get_upgrade_policy()` (keeping narrowed `strategy: UpgradeStrategy`
      in `Checking` and `action_on_exit: UpgradeActionOnExit` in `UpgradeAvailable`),
      construct `CrateVersion` at compile time via
      `const { CrateVersion::from_cargo_parts(...) }` in `upgrade_journey!` (using Cargo's
      pre-split `CARGO_PKG_VERSION_{MAJOR,MINOR,PATCH,PRE}` while keeping runtime
      `FromStr` slice-pattern validation for `versions.jsonc` / tests), and update unit
      tests across `self_upgrade`.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `cmdr/src/self_upgrade/constants.rs`
    - [x] `cmdr/src/self_upgrade/mod.rs`
    - [x] `cmdr/src/self_upgrade/config/mod.rs`
    - [x] `cmdr/src/self_upgrade/config/cli_arg.rs`
    - [x] `cmdr/src/self_upgrade/config/upgrade_policy.rs`

### Phase 2.3: Consolidate `binaries_release_base_url` into `versions.jsonc` & Introduce `EndpointUrl` Newtype

#### Problem Statement

1. **Compiled-In GitHub URL (`R3BL_BINARIES_RELEASE_BASE_URL`)**: While `versions.jsonc`
   is served from R3BL-owned `https://r3bl.com/versions.jsonc`,
   `R3BL_BINARIES_RELEASE_BASE_URL`
   (`"https://github.com/r3bl-org/r3bl-binaries/releases/download"`) was still baked into
   compiled binaries via `UpgradeConfig::new()`. Moving binary hosting to another provider
   in the future would leave already-shipped binaries hitting a stale `github.com` URL
   (and permanently strand closed-source binaries that have no `crates.io` fallback).
2. **Stale Constant Naming (`R3BL_VERSIONS_JSON_URL`)**: The manifest constant was named
   `R3BL_VERSIONS_JSON_URL` despite pointing to `versions.jsonc` and parsing into
   `JsoncManifest`.
3. **Raw `String` / `Option<String>` URL Fields**: `UpgradeConfig` held two raw `String`
   fields (`jsonc_manifest_url` and `binaries_release_base_url`) constructed via two
   `Option<String>` arguments in `UpgradeConfig::new()`.

#### Selected Design

1. **Single Compiled-In Anchor (`R3BL_VERSIONS_JSONC_URL`) + Dynamic Release Base URL in
   `versions.jsonc`**:
    - Rename `R3BL_VERSIONS_JSON_URL` -> `R3BL_VERSIONS_JSONC_URL`
      (`"https://r3bl.com/versions.jsonc"`).
    - Delete `R3BL_BINARIES_RELEASE_BASE_URL` from `constants.rs` and add top-level
      `"binaries_release_base_url"` (`versions_manifest_keys::BINARIES_RELEASE_BASE_URL`)
      to `JsoncManifest` so `versions.jsonc` is the single source of truth for where
      binary release archives live.
    - `JsoncManifest::try_update_crate_in_jsonc()` continues mutating only
      `"crates"[crate_name]` in-place via CST, leaving `"binaries_release_base_url"`
      untouched during routine crate version bumps while validating its presence via
      `Self::try_parse_jsonc()`.
2. **Shared Validated `EndpointUrl` Newtype & Uniform `JsoncScalar` Contract
   (`cmdr/src/self_upgrade/jsonc_manifest/endpoint_url.rs`,
   `cmdr/src/self_upgrade/jsonc_manifest/jsonc_scalar.rs`)**:
    - Use a single shared validated wrapper over `String` (`EndpointUrl`) for both
      `UpgradeConfig::jsonc_manifest_url` and `JsoncManifest::binaries_release_base_url`
      to minimize cognitive load (since no struct or constructor holds both URLs
      side-by-side and explicit field/parameter names distinguish them).
    - Define `JsoncScalar` (`pub trait JsoncScalar: Sized`) with
      `fn try_parse(s: &str) -> Result<Self, Self::ParseError>`,
      `fn cst_value(&self) -> CstInputValue`, and a default
      `fn deserialize_str<'de, D>(deserializer: D) -> Result<Self, D::Error>` adapter used
      via `#[serde(deserialize_with = "JsoncScalar::deserialize_str")]` on `JsoncManifest`
      and `CrateMetadata` fields. This unifies all four manifest scalar types
      (`EndpointUrl`, `CrateVersion`, `CrateReleaseDate`, and `CrateDistribution`) under a
      single parsing and CST serialization contract and eliminates `FromStr`,
      `TryFrom<&str>`, and `TryFrom<String>` boilerplate.
    - Validate `EndpointUrl` via `JsoncScalar::try_parse` (`reqwest::Url::parse` +
      `url_schemes::{HTTP, HTTPS}` + no `?query` or `#fragment`, returning
      `EndpointUrlParseError`), implement `Deref<Target = str>` (matching `CrateName` and
      `CrateReleaseDate`), and provide `EndpointUrl::default_manifest_url()` (rather than
      `Default`, since `R3BL_VERSIONS_JSONC_URL` only applies to the manifest endpoint):

    ```rust
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct EndpointUrl {
        inner: String,
    }

    impl EndpointUrl {
        #[must_use]
        pub fn default_manifest_url() -> Self {
            Self::try_parse(R3BL_VERSIONS_JSONC_URL)
                .expect("R3BL_VERSIONS_JSONC_URL must be a valid HTTP(S) endpoint URL")
        }
    }
    // Also implements: JsoncScalar (ParseError = EndpointUrlParseError), Deref<Target = str>, Display
    ```

    ```rust
    #[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
    pub struct JsoncManifest {
        #[serde(deserialize_with = "JsoncScalar::deserialize_str")]
        pub binaries_release_base_url: EndpointUrl,
        pub crates: HashMap<CrateName, CrateMetadata>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct UpgradeConfig {
        crate_name: CrateName,
        current_version: CrateVersion,
        user_feedback_channel: UserFeedbackChannel,
        upgrade_policy: UpgradePolicy,
        jsonc_manifest_url: EndpointUrl,
        cooldown_policy: CooldownPolicy,
    }
    ```

3. **Typestate & Provider-Neutral Upgrade Plumbing**:
    - Remove `binaries_release_base_url` from `UpgradeConfig` (`UpgradeConfig::new()`
      accepts `jsonc_manifest_url: EndpointUrl` directly, and `upgrade_journey!` passes
      `$crate::EndpointUrl::default_manifest_url()`).
    - Extract `(CrateMetadata, EndpointUrl)` in `version_check.rs` and carry
      `pub binaries_release_base_url: EndpointUrl` on `UpgradeAvailable` and `Upgrading`
      (keeping `CrateMetadata` 1-for-1 with per-crate JSONC entries).
    - Pass `&EndpointUrl` into `perform_upgrade` -> `UpgradeSource::try_resolve` ->
      `format_release_asset_urls`, and make `Github`-coupled internal names in `upgrade/`
      provider-neutral (`UpgradeSource::ReleaseArchive`, `format_release_asset_urls`,
      `check_release_asset_available`, `execute_release_archive_or_fallback`,
      `download_and_replace_release_archive`).

#### Implementation Steps for Phase 2.3

- [x] **Step 1 — Add `EndpointUrl` & Rename `R3BL_VERSIONS_JSONC_URL`**:
    - [x] Rename `R3BL_VERSIONS_JSON_URL` -> `R3BL_VERSIONS_JSONC_URL` in
          `cmdr/src/self_upgrade/constants.rs` and across intra-doc links in
          `cmdr/src/self_upgrade/`, and add
          `versions_manifest_keys::BINARIES_RELEASE_BASE_URL`.
    - [x] Create `cmdr/src/self_upgrade/jsonc_manifest/endpoint_url.rs` defining
          `EndpointUrl` (with unit tests) and barrel-export it from
          `cmdr/src/self_upgrade/jsonc_manifest/mod.rs`.
- [x] **Step 2 — Wire `binaries_release_base_url` Through `JsoncManifest`,
      `UpgradeConfig`, Typestates & `upgrade/`**:
    - [x] Add `pub binaries_release_base_url: EndpointUrl` to `JsoncManifest` in
          `cmdr/src/self_upgrade/jsonc_manifest/jsonc_manifest_struct.rs` and update JSONC
          examples and test fixtures (`jsonc_manifest_struct.rs`, `crate_version.rs`).
    - [x] Update `UpgradeConfig` (`cmdr/src/self_upgrade/config/upgrade_config.rs`) and
          `upgrade_journey!` (`cmdr/src/self_upgrade/upgrade_journey_public_api.rs`) to
          take `jsonc_manifest_url: EndpointUrl`, remove `binaries_release_base_url` from
          `UpgradeConfig`, and delete `R3BL_BINARIES_RELEASE_BASE_URL` from
          `constants.rs`.
    - [x] Update `manifest_query.rs`, `version_check.rs`, `upgrade_check_cache.rs`, and
          `upgrade_journey_public_api.rs` so `UpgradeAvailable` and `Upgrading` carry
          `pub binaries_release_base_url: EndpointUrl` parsed from `JsoncManifest`.
    - [x] Update `release.rs`, `upgrade_source.rs`, and `perform_upgrade.rs` to consume
          `&EndpointUrl` and use provider-neutral symbol names
          (`UpgradeSource::ReleaseArchive`, `format_release_asset_urls`,
          `check_release_asset_available`, `execute_release_archive_or_fallback`,
          `download_and_replace_release_archive`), and update all unit tests across
          `self_upgrade/`.
        - [x] Introduce `JsoncScalar` trait
              (`cmdr/src/self_upgrade/jsonc_manifest/jsonc_scalar.rs`, re-exported in
              `jsonc_manifest/mod.rs`) with `try_parse(&str)`, `cst_value(&self)`, and
              default `deserialize_str`
              (`#[serde(deserialize_with = "JsoncScalar::deserialize_str")]`), implement
              it on all manifest scalar types (`EndpointUrl`, `CrateVersion`,
              `CrateReleaseDate`, and `CrateDistribution`), remove `FromStr`,
              `TryFrom<&str>`, `TryFrom<String>`, and unused `Borrow<str>` (on
              `CrateName`), replace `EndpointUrl::as_str()` with `Deref<Target = str>`,
              and update all test call sites to use `JsoncScalar::try_parse`.
- [x] **Step 3 — Documentation & Spec Alignment**:
    - [x] Update top-level schema examples and narrative in
          `task/binaries-self-upgrade-support.md` to reflect `"binaries_release_base_url"`
          in `versions.jsonc` and `R3BL_VERSIONS_JSONC_URL`.
    - [x] Run `./check.fish --check`, `cargo rustdoc-fmt`,
          `cargo doc -p r3bl-cmdr --no-deps`, and targeted `self_upgrade` unit tests.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `cmdr/src/self_upgrade/constants.rs`
    - [x] `cmdr/src/self_upgrade/mod.rs`
    - [x] `cmdr/src/self_upgrade/jsonc_manifest/mod.rs`
    - [x] `cmdr/src/self_upgrade/jsonc_manifest/jsonc_manifest_struct.rs`
    - [x] `cmdr/src/self_upgrade/jsonc_manifest/jsonc_scalar.rs`
    - [x] `cmdr/src/self_upgrade/jsonc_manifest/endpoint_url.rs`

### Phase 2.4: Split `upgrade_journey_public_api.rs` into `upgrade_journey/` Module & Extract Per-Typestate Files

#### Problem Statement

`cmdr/src/self_upgrade/upgrade_journey_public_api.rs` has grown to 1,486 lines, mixing the
declarative `upgrade_journey!` macro, the `UpgradeJourney<State>` container struct, all 9
typestate structs and their state-transition/execution `impl` blocks, startup outcome
types (`StartOutcome`, `ImmediateUpgradeResult`), Tokio/check-thread runtime helpers, and
11 integration/unit tests in a single monolithic file.

#### Selected Design (`cmdr/src/self_upgrade/upgrade_journey/` + `typestates/`)

Replace `cmdr/src/self_upgrade/upgrade_journey_public_api.rs` with a dedicated
`cmdr/src/self_upgrade/upgrade_journey/` submodule (and a nested `typestates/` directory
with 1 file per typestate), preserving 100% of the flat public API surface via barrel
re-exports (`pub use upgrade_journey::*;` in `cmdr/src/self_upgrade/mod.rs`):

```text
cmdr/src/self_upgrade/
├── mod.rs                          (replaces `mod upgrade_journey_public_api;` with `mod upgrade_journey;`)
└── upgrade_journey/
    ├── mod.rs                      (private submodules + `pub use *;` barrel re-exports + lifecycle `#[cfg(test)] mod tests`)
    ├── upgrade_journey_macro.rs    (`#[macro_export] macro_rules! upgrade_journey`)
    ├── upgrade_journey_struct.rs   (`pub struct UpgradeJourney<State: UpgradeJourneyStateMarker>` + `config()` / `state()` accessors)
    ├── start_outcome.rs          (`pub enum StartOutcome`, `pub enum ImmediateUpgradeResult`, `Termination` impl)
    ├── tokio_thread_runner.rs      (`run_in_tokio_logged`, `run_on_dedicated_check_thread`, `extract_panic_payload_message` + unit tests)
    └── typestates/
        ├── mod.rs                  (`pub trait UpgradeJourneyStateMarker {}` + private submodules + `pub use *;` barrel re-exports)
        ├── not_started.rs          (`pub struct NotStarted;` + `impl UpgradeJourney<NotStarted>` with `new()`, `start()`, `run_immediate_upgrade()`, `log_duplicate_start()`, `into_disabled()`, `into_cooldown_skipped()`, `into_checking()`)
        ├── disabled.rs             (`pub struct Disabled;` + `impl UpgradeJourneyStateMarker for Disabled`)
        ├── cooldown_skipped.rs     (`pub struct CooldownSkipped;` + `impl UpgradeJourneyStateMarker for CooldownSkipped`)
        ├── checking.rs             (`pub struct Checking` + `impl UpgradeJourney<Checking>` with `into_up_to_date()`, `into_upgrade_available()`, `into_upgrading()`, `into_check_failed()`)
        ├── up_to_date.rs           (`pub struct UpToDate` + `impl UpgradeJourneyStateMarker for UpToDate`)
        ├── upgrade_available.rs    (`pub struct UpgradeAvailable` + `impl UpgradeJourney<UpgradeAvailable>` with `into_upgrading()`)
        ├── check_failed.rs         (`pub struct CheckFailed` + `impl UpgradeJourneyStateMarker for CheckFailed`)
        ├── upgrading.rs            (`pub struct Upgrading` + `impl UpgradeJourney<Upgrading>` with `into_completed()`, `execute_action_on_exit()`)
        └── completed.rs            (`pub struct Completed`, `pub enum UpgradeResolution`, `impl UpgradeJourneyStateMarker for Completed`)
```

#### Encapsulation & Visibility Rules (`pub(in crate::self_upgrade)`)

- In `cmdr/src/self_upgrade/upgrade_journey/upgrade_journey_struct.rs`:
    ```rust
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct UpgradeJourney<State: UpgradeJourneyStateMarker> {
        pub(in crate::self_upgrade) config: UpgradeConfig,
        pub(in crate::self_upgrade) state: State,
    }
    ```
- In `typestates/{not_started,checking,upgrade_available,upgrading}.rs` and
  `tokio_thread_runner.rs`:
    - Mark internal state transition methods (`into_disabled`, `into_cooldown_skipped`,
      `into_checking`, `into_up_to_date`, `into_upgrade_available`, `into_upgrading`,
      `into_check_failed`, `into_completed`), `execute_action_on_exit`,
      `run_in_tokio_logged`, and `run_on_dedicated_check_thread` as
      `pub(in crate::self_upgrade)`.
    - This keeps `UpgradeJourney`'s fields and transitions strictly private to
      `crate::self_upgrade` (impossible for external callers to forge transitions) while
      allowing `upgrade_journey::typestates::*`, `check::version_entry_point`, and
      `upgrade_check_cache` to access them cleanly across submodule boundaries.

#### Implementation Steps for Phase 2.4

- [x] Create `cmdr/src/self_upgrade/upgrade_journey/typestates/` with `mod.rs`
      (`UpgradeJourneyStateMarker`) and 1 file per state (`not_started.rs`, `disabled.rs`,
      `cooldown_skipped.rs`, `checking.rs`, `up_to_date.rs`, `upgrade_available.rs`,
      `check_failed.rs`, `upgrading.rs`, `completed.rs`), preserving all rustdoc comments
      and intra-doc links byte-for-byte.
- [x] Create `cmdr/src/self_upgrade/upgrade_journey/` files (`upgrade_journey_macro.rs`,
      `upgrade_journey_struct.rs`, `start_outcome.rs`, `tokio_thread_runner.rs`, and
      `mod.rs` with barrel re-exports and lifecycle tests).
- [x] Update `cmdr/src/self_upgrade/mod.rs` to replace
      `mod upgrade_journey_public_api; pub use upgrade_journey_public_api::*;` with
      `mod upgrade_journey; pub use upgrade_journey::*;`, and delete
      `cmdr/src/self_upgrade/upgrade_journey_public_api.rs`.
- [x] Run `./check.fish --check` and `./check.fish --fmt` to verify compilation,
      formatting, and rustdoc links.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `cmdr/src/self_upgrade/mod.rs`
    - [x] `cmdr/src/self_upgrade/jsonc_manifest/crate_distribution.rs`
    - [x] `cmdr/src/self_upgrade/jsonc_manifest/crate_metadata.rs`
    - [x] `cmdr/src/self_upgrade/jsonc_manifest/crate_name.rs`
    - [x] `cmdr/src/self_upgrade/jsonc_manifest/crate_release_date.rs`
    - [x] `cmdr/src/self_upgrade/jsonc_manifest/crate_version.rs`
    - [x] `cmdr/src/self_upgrade/config/mod.rs`
    - [x] `cmdr/src/self_upgrade/check/cooldown.rs`
    - [x] `cmdr/src/self_upgrade/config/upgrade_config.rs`
    - [x] `cmdr/src/self_upgrade/check/mod.rs`

### Phase 2.5: Make Typestate Transitions Explicit & Pure (Decouple Workers from `UpgradeCheckStatus` Singleton)

#### Problem Statement

During manual review of Phase 2.4 (`cmdr/src/self_upgrade/check/check_entry_point.rs`),
three architectural issues were identified in how typestate transitions and the
`UpgradeCheckStatus` singleton interact:

1. **Hidden State Transitions Swallowed Into Global Singleton**:
    - `check_for_upgrade(journey: UpgradeJourney<Checking>)` accepts the `Checking`
      typestate by value and returns `()`, silently mutating
      `UpgradeCheckStatus::set(...)` internally across its branches instead of returning
      the transitioned `UpgradeJourney<NextState>` to its caller.
    - `run_on_dedicated_check_thread` (`tokio_thread_runner.rs`) similarly swallows Tokio
      runtime initialization errors and panics directly into
      `UpgradeCheckStatus::set(...)` instead of returning a `CheckFailed` typestate
      outcome.
2. **Duplicated Manifest Check Pipeline**:
    - `check_for_upgrade` and `check_for_immediate_upgrade` in `check_entry_point.rs`
      duplicate the HTTP manifest fetch, metadata extraction, version comparison, and
      cooldown `touch()` logic solely because `check_for_upgrade` returned `()` while
      `check_for_immediate_upgrade` both mutated `UpgradeCheckStatus::set(...)` and
      returned `ImmediateCheckOutcome`.
3. **Asymmetric Subsystem Boundaries (`check/` vs. `upgrade/` vs. `typestates/`)**:
    - Every file in `upgrade/` (`perform_upgrade.rs`, `upgrade_source.rs`, etc.) and
      `check/cooldown.rs` is pure domain/I/O logic with zero knowledge of
      `UpgradeJourney<State>` or `UpgradeCheckStatus`, whereas `typestates/<state>.rs`
      (such as `typestates/upgrading.rs`) owns `impl UpgradeJourney<State>` orchestration.
    - `check_entry_point.rs` broke this symmetry by mixing free-floating
      `UpgradeJourney<Checking>` transition functions and `UpgradeCheckStatus::set(...)`
      mutations with low-level HTTP/JSONC manifest fetching.

#### Selected Design (Direction 1 + Option 1B: Pure Typestate Methods, Strategy-Specific Outcomes, Centralized Singleton Driver)

Separate **deciding what to do** (**Control Plane**) from **doing the work** (**Execution
Plane**) across `self_upgrade/`:

- **Execution Plane (Network, Disk & Build Operations: `check/` and `upgrade/`)**:
  Performs the actual HTTP manifest fetch, disk timestamp file reads/writes, archive
  download, SHA-256 check, binary swap, PTY source build, and user prompt. Stateless with
  respect to the state machine: these functions don't know or care which
  `UpgradeJourney<State>` is active, never import or touch `UpgradeJourney` or
  `UpgradeCheckStatus`, and take only the configuration and metadata parameters they need
  (`&UpgradeConfig`, `&EndpointUrl`, `&CrateMetadata`).
- **Control Plane (Typestate Orchestration, Lifecycle & Threading: `upgrade_journey/`)**:
    - **Typestate Orchestration (`upgrade_journey/typestates/<state>.rs`)**: Every active
      execution step on `UpgradeJourney<Checking>` and `UpgradeJourney<Upgrading>` is a
      `pub(in crate::self_upgrade)` method consuming `self` by value and returning the
      next typestate (or a strategy-specific outcome enum with zero `unreachable!`
      branches).
    - **Streamlined Singleton Lifecycle & Threading (`not_started.rs`,
      `tokio_thread_runner.rs` & `upgrade_singleton.rs`)**: Eliminate the 9-variant
      `UpgradeCheckStatus` enum completely. Instead, `SINGLETON` holds
      `UpgradeSingletonState { started: bool, dedicated_thread_handle: Option<JoinHandle<DedicatedCheckOutcome>> }`,
      where the dedicated thread returns `DedicatedCheckOutcome` directly over its
      `JoinHandle` (never touching `SINGLETON`), and both
      `UpgradeJourney::<NotStarted>::start` (`StartOutcome`) and `handle_upgrade_on_exit`
      (`ExitOutcome`) return the transitioned `UpgradeJourney<State>` typestates directly
      by value.

##### 1. Pure Manifest Fetch Service (`cmdr/src/self_upgrade/check/fetch_manifest.rs`)

Rename `cmdr/src/self_upgrade/check/check_entry_point.rs` ->
`cmdr/src/self_upgrade/check/fetch_manifest.rs` (updating `check/mod.rs`), removing all
`UpgradeJourney` and `UpgradeCheckStatus` imports and exposing the pure manifest query
service alongside `ManifestCheckError`:

```rust
/// Fetches `versions.jsonc` from `manifest_url` within `timeout` and extracts
/// `(CrateMetadata, EndpointUrl)` for `crate_name`, logging any fetch or parse failure
/// via [`DEBUG_SELF_UPGRADE`].
///
/// Pure manifest I/O service with zero knowledge of [`UpgradeJourney`] or
/// [`UpgradeCheckStatus`].
pub async fn fetch_crate_metadata_from_manifest(
    manifest_url: &EndpointUrl,
    crate_name: &CrateName,
    timeout: Duration,
) -> Result<(CrateMetadata, EndpointUrl), ManifestCheckError>
```

##### 2. Strategy-Specific Check Outcomes & Pure Methods on `UpgradeJourney<Checking>` (`typestates/checking.rs`)

Define exhaustive 3-variant outcome enums (zero `unreachable!` match arms anywhere) and
move check orchestration onto `impl UpgradeJourney<Checking>` in
`cmdr/src/self_upgrade/upgrade_journey/typestates/checking.rs`:

```rust
/// Exhaustive outcome of [`UpgradeJourney::<Checking>::check_for_dedicated_thread_upgrade`].
///
/// Makes `Upgrading` unrepresentable prior to application exit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::self_upgrade) enum DedicatedCheckOutcome {
    /// Local binary is already up to date (`remote <= current`); `1h` cooldown
    /// timestamp file has been touched.
    UpToDate(UpgradeJourney<UpToDate>),

    /// Newer version found (`remote > current`); awaits exit handling via
    /// [`UpgradeCheckStatus::handle_upgrade_on_exit`].
    UpgradeAvailable(UpgradeJourney<UpgradeAvailable>),

    /// Network, parse, or thread/runtime error occurred during the manifest check.
    CheckFailed(UpgradeJourney<CheckFailed>),
}

/// Exhaustive outcome of [`UpgradeJourney::<Checking>::check_for_immediate_upgrade`].
///
/// Makes `UpgradeAvailable` unrepresentable during an explicit `--upgrade` run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::self_upgrade) enum ImmediateCheckOutcome {
    /// Network or manifest lookup failed.
    CheckFailed(UpgradeJourney<CheckFailed>),

    /// Local binary is already up to date (`remote <= current`); `1h` cooldown
    /// timestamp file has been touched.
    UpToDate(UpgradeJourney<UpToDate>),

    /// Newer version found (`remote > current`); ready to run
    /// [`UpgradeJourney::<Upgrading>::execute_immediate_upgrade`].
    Upgrading(UpgradeJourney<Upgrading>),
}

impl UpgradeJourney<Checking> {
    #[must_use]
    pub(in crate::self_upgrade) async fn check_for_dedicated_thread_upgrade(
        self,
        action_on_exit: UpgradeActionOnExit,
    ) -> DedicatedCheckOutcome { ... }

    #[must_use]
    pub(in crate::self_upgrade) async fn check_for_immediate_upgrade(
        self,
    ) -> ImmediateCheckOutcome { ... }
}
```

> **Encapsulation note**: Keeping `check_for_dedicated_thread_upgrade` and
> `check_for_immediate_upgrade` `pub(in crate::self_upgrade)` (matching
> `execute_action_on_exit`) ensures external callers who inspect a cloned
> `UpgradeCheckStatus::Checking(journey)` via `UpgradeCheckStatus::get()` cannot
> re-trigger network checks outside the engine.

##### 3. Pure Immediate Upgrade Execution on `UpgradeJourney<Upgrading>` (`typestates/upgrading.rs`)

Add `execute_immediate_upgrade(self)` alongside `execute_action_on_exit(self, ...)` in
`cmdr/src/self_upgrade/upgrade_journey/typestates/upgrading.rs`:

```rust
impl UpgradeJourney<Upgrading> {
    /// Executes an immediate (`--upgrade`) installation outside the `SINGLETON` leaf
    /// lock, touches the `1h` cooldown timestamp file if and only if installation
    /// succeeded, and consumes `self` into [`UpgradeJourney<Completed>`].
    #[must_use]
    pub(in crate::self_upgrade) async fn execute_immediate_upgrade(
        self,
    ) -> UpgradeJourney<Completed> { ... }
}
```

##### 4. Pure Dedicated Thread Runner (`tokio_thread_runner.rs`)

Update `run_on_dedicated_thread` so it returns `DedicatedCheckOutcome` directly (catching
Tokio runtime build failures into `ManifestCheckError::RuntimeOrThreadInitFailed` and
panics into `ManifestCheckError::DedicatedThreadPanicked`):

```rust
#[must_use]
pub(in crate::self_upgrade) fn run_on_dedicated_thread<F>(
    checking: UpgradeJourney<Checking>,
    async_fn: F,
) -> DedicatedCheckOutcome
where
    F: AsyncFnOnce(UpgradeJourney<Checking>) -> DedicatedCheckOutcome,
```

##### 5. Streamlined `SINGLETON` (`JoinHandle<DeferredCheckOutcome>`) & Typed Startup/Exit Outcomes (`start_outcome.rs`, `upgrade_singleton.rs`, & `typestates/not_started.rs`)

- **Delete the 9-variant `UpgradeCheckStatus` enum completely**:
    - `SINGLETON` holds
      `UpgradeSingletonState { start_status: SingletonStartStatus, deferred_check_handle: Option<JoinHandle<DeferredCheckOutcome>> }`.
    - In `UpgradeJourney::<NotStarted>::start`:
        - Atomically claims
          `UpgradeSingletonState::try_claim_start() -> SingletonStartStatus` once at the
          top of `start(self) -> StartOutcome` (returning `StartOutcome::AlreadyStarted`
          on duplicate calls when `SingletonStartStatus::AlreadyStarted` is observed).
        - For `UpgradeStrategy::Deferred`, spawns the dedicated thread named
          `dedicated_thread::DEDICATED_THREAD_NAME` (`"self-upgrade"`) returning
          `DeferredCheckOutcome` directly via `JoinHandle<DeferredCheckOutcome>`, and
          stores the handle via `UpgradeSingletonState::set_deferred_check_handle(handle)`
          (so the dedicated thread never accesses `SINGLETON`).
- **Return typed `UpgradeJourney<State>` directly from `.start()` and
  `handle_upgrade_on_exit()`**:
    - In `start_outcome.rs` (pure domain types with zero `ExitCode` or `Termination`
      boilerplate):

        ```rust
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum StartOutcome {
            Disabled(UpgradeJourney<Disabled>),
            ImmediateUpgrade(ImmediateUpgradeOutcome),
            DeferredUpgrade(DeferredUpgradeOutcome),
            AlreadyStarted,
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum ImmediateUpgradeOutcome {
            UpToDate(UpgradeJourney<UpToDate>),
            CheckFailed(UpgradeJourney<CheckFailed>),
            Completed(UpgradeJourney<Completed>),
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum DeferredUpgradeOutcome {
            CooldownSkipped(UpgradeJourney<CooldownSkipped>),
            DeferredCheckSpawned(UpgradeJourney<Checking>),
            DeferredCheckSpawnFailed(UpgradeJourney<CheckFailed>),
        }
        ```

    - In `upgrade_singleton.rs`:

        ```rust
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum ExitOutcome {
            NoDeferredCheckRunning,
            UpToDate(UpgradeJourney<UpToDate>),
            CheckFailed(UpgradeJourney<CheckFailed>),
            Completed(UpgradeJourney<Completed>),
        }

        pub fn handle_upgrade_on_exit() -> ExitOutcome { ... }
        ```

    - **State Machine & Lifecycle Flow diagram (`upgrade_journey_struct.rs`)**:

        ```text
        1. At startup: upgrade_journey!(channel, cli_args) ──► <NotStarted>.start()
        ┌─────────────────────────────────────────────────────────────────────────┘
        ├─ 1st .start() (claims SINGLETON.start_status; branches on UpgradePolicy):
        │  │
        │  ├─ UpgradePolicy::Disabled (--upgrade-behavior disable)
        │  │  └─► <Disabled> ──► StartOutcome::Disabled(<Disabled>)
        │  │
        │  ├─ UpgradePolicy::Enabled(Immediate) (--upgrade)
        │  │  │ (Runs entire lifecycle inside .start(); main() exits immediately)
        │  │  └─► <Checking> (fetches versions.jsonc, 5s timeout; bypasses 1h cooldown)
        │  │      ├─► <CheckFailed> (network/parse error)
        │  │      │   └─► ImmediateUpgrade(CheckFailed(<CheckFailed>))
        │  │      ├─► <UpToDate> (remote <= current) ──► touch timestamp file
        │  │      │   └─► ImmediateUpgrade(UpToDate(<UpToDate>))
        │  │      └─► <Upgrading> (remote > current) ──► perform_upgrade()
        │  │          ├─► <Completed(Upgraded)> ──► touch timestamp file
        │  │          │   └─► ImmediateUpgrade(Completed(<Completed>))
        │  │          └─► <Completed(UpgradeFailed)>
        │  │              └─► ImmediateUpgrade(Completed(<Completed>))
        │  │
        │  └─ UpgradePolicy::Enabled(Deferred(action))
        │     │ (Returns DeferredUpgrade immediately; handle_upgrade_on_exit() later
        │     │  takes and joins JoinHandle<DeferredCheckOutcome> from SINGLETON)
        │     ├─ Active 1h cooldown ──► <CooldownSkipped>
        │     │  └─► DeferredUpgrade(CooldownSkipped(<CooldownSkipped>))
        │     └─ Expired/missing timestamp file ──► <Checking>
        │        ├─► Main thread: DeferredUpgrade(DeferredCheckSpawned(<Checking>))
        │        └─► Dedicated thread (2s timeout; returns DeferredCheckOutcome):
        │            ├─► DeferredCheckOutcome::UpToDate(<UpToDate>) ──► touch file
        │            ├─► DeferredCheckOutcome::CheckFailed(<CheckFailed>)
        │            └─► DeferredCheckOutcome::UpgradeAvailable(<UpgradeAvailable>)
        │
        └─ 2nd+ .start() (SINGLETON.start_status == AlreadyStarted)
           └─► StartOutcome::AlreadyStarted (no-op)

        2. At exit: handle_upgrade_on_exit() ──► ExitOutcome
        ┌──────────────────────────────────────────────────┘
        ├─ Step 1: Takes JoinHandle<DeferredCheckOutcome> from SINGLETON, drops lock,
        │          and joins dedicated thread (waits <= 2s)
        │  └─ If None (Disabled | Immediate | CooldownSkipped | already called)
        │     └─► ExitOutcome::NoDeferredCheckRunning
        └─ Step 2: Inspects DeferredCheckOutcome returned by dedicated thread:
           ├─ UpToDate(<UpToDate>) ──────► ExitOutcome::UpToDate(<UpToDate>)
           ├─ CheckFailed(<CheckFailed>) ► ExitOutcome::CheckFailed(<CheckFailed>)
           └─ UpgradeAvailable(<UpgradeAvailable>) ──► <Upgrading>
              ├─ Executes UpgradeActionOnExit:
              │  ├─ Auto ────────────► perform_upgrade()
              │  ├─ PromptToUpgrade ─► prompt user ──► perform_upgrade()
              │  └─ NotifyOnly ──────► print notification
              └─► ExitOutcome::Completed(<Completed>) (touches file if Upgraded)

        Legend:
        - <State>              = UpgradeJourney<State> (e.g. <NotStarted>)
        - DeferredUpgrade(..)  = StartOutcome::DeferredUpgrade(DeferredUpgradeOutcome::..)
        - ImmediateUpgrade(..) = StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::..)
        ```

##### 6. Control Plane vs. Execution Plane Documentation & Upward Rustdoc Links

- Add `## Control Plane vs. Execution Plane` in `cmdr/src/self_upgrade/mod.rs` separating
  **deciding what to do** from **doing the work**:
    - **Control Plane (State Machine, Lifecycle & Threading)**: `upgrade_journey/`
      (including `upgrade_singleton.rs`, `tokio_thread_runner.rs`, and `typestates/`).
    - **Execution Plane (Network, Disk & Build Operations)**: `check/` and `upgrade/`.
- Add upward rustdoc links (using `[Control Plane]` and `[Execution Plane]` reference
  links pointing to `mod@crate::self_upgrade#control-plane-vs-execution-plane`, plus
  intra-doc links to the `pub` typestates that invoke each service) in:
    - `cmdr/src/self_upgrade/check/fetch_manifest.rs`
      (`fetch_crate_metadata_from_manifest` -> invoked by [`Checking`] to transition into
      [`UpToDate`], [`UpgradeAvailable`], [`Upgrading`], or [`CheckFailed`]).
    - `cmdr/src/self_upgrade/check/cooldown.rs` (`CooldownPolicy` -> `check_status()`
      invoked by [`UpgradeJourney::start`] into [`CooldownSkipped`] vs. [`Checking`];
      `touch()` invoked when [`Checking`] resolves to [`UpToDate`] or [`Upgrading`]
      resolves to [`Completed`]).
    - `cmdr/src/self_upgrade/upgrade/perform_upgrade.rs` (`perform_upgrade` -> invoked by
      [`Upgrading`] to resolve into [`Completed`]).
    - `cmdr/src/self_upgrade/upgrade/prompt.rs` (`prompt_user_to_upgrade` -> invoked by
      [`Upgrading`] at exit when `UpgradeActionOnExit::PromptToUpgrade` is active).

#### Implementation Steps for Phase 2.5

- [x] Rename `cmdr/src/self_upgrade/check/check_entry_point.rs` ->
      `cmdr/src/self_upgrade/check/fetch_manifest.rs` (and update
      `cmdr/src/self_upgrade/check/mod.rs`), extracting pure
      `fetch_crate_metadata_from_manifest` + `ManifestCheckError` with zero
      `UpgradeJourney` or `UpgradeCheckStatus` imports.
- [x] Add `DedicatedCheckOutcome`, `ImmediateCheckOutcome`,
      `UpgradeJourney::<Checking>::check_for_dedicated_thread_upgrade`, and
      `UpgradeJourney::<Checking>::check_for_immediate_upgrade` to
      `cmdr/src/self_upgrade/upgrade_journey/typestates/checking.rs`, along with parallel
      (non-`#[serial]`, singleton-free) unit tests verifying outcomes and cooldown
      `touch()` behavior.
- [x] Add `UpgradeJourney::<Upgrading>::execute_immediate_upgrade` to
      `cmdr/src/self_upgrade/upgrade_journey/typestates/upgrading.rs`.
- [x] Update `run_on_dedicated_check_thread` in
      `cmdr/src/self_upgrade/upgrade_journey/tokio_thread_runner.rs` to return
      `DedicatedCheckOutcome` directly without mutating `UpgradeCheckStatus`, and update
      its panic unit test to assert on the returned `DedicatedCheckOutcome` without
      `#[serial]`.
- [x] Implement `into_check_status(self) -> UpgradeCheckStatus` on `DedicatedCheckOutcome`
      and `ImmediateCheckOutcome` in
      `cmdr/src/self_upgrade/upgrade_journey/typestates/checking.rs`, move
      `upgrade_check_cache.rs` ->
      `cmdr/src/self_upgrade/upgrade_journey/upgrade_singleton.rs`, and update
      `cmdr/src/self_upgrade/upgrade_journey/typestates/not_started.rs` so all singleton
      updates happen explicitly at the top-level orchestrators.
- [x] Document `## Control Plane vs. Execution Plane` in `cmdr/src/self_upgrade/mod.rs`
      and add upward Control Plane / Execution Plane rustdoc links in
      `check/fetch_manifest.rs`, `check/cooldown.rs`, `upgrade/perform_upgrade.rs`, and
      `upgrade/prompt.rs`.
- [x] Run `./check.fish --check`, `./check.fish --fmt`, and targeted `self_upgrade` unit
      tests (`cargo test -p r3bl-cmdr --lib self_upgrade -q --message-format=short`) to
      verify compilation, formatting, rustdoc links, and test coverage.
- [x] Eliminate the 9-variant `UpgradeCheckStatus` enum, return `DedicatedCheckOutcome`
      directly via `JoinHandle<DedicatedCheckOutcome>`, return typed
      `UpgradeJourney<State>` outcomes from `.start()` and `handle_upgrade_on_exit()`, and
      standardize naming on "dedicated thread":
    - [x] **Step 1 (`constants.rs` & `check/fetch_manifest.rs`)**:
        - Rename `pub mod check_thread` -> `pub mod dedicated_thread` and
          `CHECK_THREAD_NAME` -> `DEDICATED_THREAD_NAME = "self-upgrade"` in
          `cmdr/src/self_upgrade/constants.rs`.
        - Rename `ManifestCheckError::CheckThreadPanicked` ->
          `ManifestCheckError::DedicatedThreadPanicked` and remove `UpgradeCheckStatus`
          from rustdoc in `cmdr/src/self_upgrade/check/fetch_manifest.rs`.
    - [x] **Step 2 (`tokio_thread_runner.rs`)**:
        - Rename `run_on_dedicated_check_thread` -> `run_on_dedicated_thread` in
          `cmdr/src/self_upgrade/upgrade_journey/tokio_thread_runner.rs`, construct
          `ManifestCheckError::DedicatedThreadPanicked`, remove `UpgradeCheckStatus` from
          rustdoc, and update unit test naming.
    - [x] **Step 3 (`typestates/checking.rs` & `typestates/upgrading.rs`)**:
        - In `cmdr/src/self_upgrade/upgrade_journey/typestates/checking.rs`: make
          `pub enum DedicatedCheckOutcome` public, delete
          `DedicatedCheckOutcome::into_check_status()` and
          `ImmediateCheckOutcome::into_check_status()`, and remove `UpgradeCheckStatus`
          imports/rustdoc.
        - In `cmdr/src/self_upgrade/upgrade_journey/typestates/upgrading.rs`: simplify
          `execute_immediate_upgrade(self)` to return `UpgradeJourney<Completed>` directly
          (instead of a tuple with `ImmediateUpgradeResult`).
    - [x] **Step 4 (`start_outcome.rs`)**:
        - Replace `ImmediateUpgradeResult` (and its `ExitCode` / `Termination` impls) with
          pure domain enum `ImmediateUpgradeOutcome`
          (`UpToDate(UpgradeJourney<UpToDate>)`,
          `CheckFailed(UpgradeJourney<CheckFailed>)`,
          `Completed(UpgradeJourney<Completed>)`).
        - Add `ContinueOutcome` (`Disabled(UpgradeJourney<Disabled>)`,
          `CooldownSkipped(UpgradeJourney<CooldownSkipped>)`,
          `DedicatedThreadSpawned(UpgradeJourney<Checking>)`,
          `DedicatedThreadSpawnFailed(UpgradeJourney<CheckFailed>)`) and update
          `StartOutcome` (`UpgradedAndExit(ImmediateUpgradeOutcome)`,
          `ContinueApplication(ContinueOutcome)`, `AlreadyStarted`).
    - [x] **Step 5 (`upgrade_singleton.rs`)**:
        - Delete `pub enum UpgradeCheckStatus` completely from
          `cmdr/src/self_upgrade/upgrade_journey/upgrade_singleton.rs`.
        - Add witness enum `pub enum SingletonStartStatus` (`#[default] NotStarted`,
          `AlreadyStarted`) and update `UpgradeSingletonState` to hold
          `pub start_status: SingletonStartStatus` and
          `pub dedicated_thread_handle: Option<JoinHandle<DedicatedCheckOutcome>>` with
          `try_claim_start() -> SingletonStartStatus`, `set_dedicated_thread_handle(...)`,
          and `#[cfg(test)] pub(crate) fn reset_for_testing()`.
        - Add `pub enum ExitOutcome` (`NoDedicatedThreadRunning`,
          `UpToDate(UpgradeJourney<UpToDate>)`,
          `CheckFailed(UpgradeJourney<CheckFailed>)`,
          `Completed(UpgradeJourney<Completed>)`) and
          `pub fn handle_upgrade_on_exit() -> ExitOutcome` that takes and joins
          `dedicated_thread_handle` outside the `SINGLETON` leaf lock and executes
          `UpgradeAvailable -> Upgrading -> Completed` directly on the stack.
        - Update unit tests in `upgrade_singleton.rs` to test `try_claim_start()`,
          `reset_for_testing()`, and `handle_upgrade_on_exit() -> ExitOutcome`.
    - [x] **Step 6 (`typestates/not_started.rs`)**:
        - Call `UpgradeSingletonState::try_claim_start()` once at the top of
          `UpgradeJourney::<NotStarted>::start(self) -> StartOutcome`.
        - Spawn the dedicated thread (`dedicated_thread::DEDICATED_THREAD_NAME`) returning
          `DedicatedCheckOutcome` directly into `JoinHandle<DedicatedCheckOutcome>` and
          store it via `UpgradeSingletonState::set_dedicated_thread_handle(handle)` (zero
          `SINGLETON` access from the dedicated thread).
        - Update `run_immediate_upgrade` to return `ImmediateUpgradeOutcome` directly on
          the stack with zero `SINGLETON` access, and update the `# Thread Model` rustdoc
          diagrams.
    - [x] **Step 7 (Rustdocs & `journey_integration_tests.rs`)**:
        - Update rustdoc links and prose across `cmdr/src/self_upgrade/mod.rs`,
          `upgrade_journey_struct.rs`, `typestates/upgrade_available.rs`,
          `config/upgrade_policy.rs`, `upgrade/perform_upgrade.rs`, and
          `upgrade/prompt.rs` (replacing `UpgradeCheckStatus::handle_upgrade_on_exit` with
          `handle_upgrade_on_exit` and removing `UpgradeCheckStatus` references).
        - Update `cmdr/src/self_upgrade/upgrade_journey/journey_integration_tests.rs` to
          assert directly on the returned `StartOutcome` (`ImmediateUpgradeOutcome`,
          `ContinueOutcome`) and `ExitOutcome` values.
        - Run `./check.fish --check`, `./check.fish --fmt`, and targeted `self_upgrade`
          unit tests
          (`cargo test -p r3bl-cmdr --lib self_upgrade -q --message-format=short`).
    - [x] **Step 8 (`Deferred` vs. `Immediate` Domain Naming Alignment)**:
        - Rename `UpgradeStrategy::DedicatedThread` -> `UpgradeStrategy::Deferred`,
          `IMMEDIATE_MANIFEST_TIMEOUT` -> `IMMEDIATE_CHECK_FETCH_MANIFEST_URL_TIMEOUT`,
          `DEDICATED_THREAD_MANIFEST_TIMEOUT` ->
          `DEFERRED_CHECK_FETCH_MANIFEST_URL_TIMEOUT`,
          `DEDICATED_THREAD_CHECK_COOLDOWN` -> `DEFERRED_CHECK_COOLDOWN_DELAY`, and
          `ManifestCheckError::DedicatedThreadPanicked` ->
          `ManifestCheckError::DeferredCheckThreadPanicked` across `constants.rs`,
          `config/{upgrade_policy,cli_arg,upgrade_config}.rs`, and
          `check/{cooldown,fetch_manifest}.rs`.
        - Rename `DedicatedCheckOutcome` -> `DeferredCheckOutcome` and
          `UpgradeJourney::<Checking>::check_for_dedicated_thread_upgrade` ->
          `check_for_deferred_upgrade` in `typestates/checking.rs` and
          `tokio_thread_runner.rs`.
        - Update `StartOutcome` in `start_outcome.rs` to promote
          `StartOutcome::Disabled(UpgradeJourney<Disabled>)` to the top level alongside
          `StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome)`,
          `StartOutcome::DeferredUpgrade(DeferredUpgradeOutcome)` (with `CooldownSkipped`,
          `DeferredCheckSpawned`, `DeferredCheckSpawnFailed`), and
          `StartOutcome::AlreadyStarted`.
        - Rename `dedicated_thread_handle` -> `deferred_check_handle`,
          `set_dedicated_thread_handle` -> `set_deferred_check_handle`, and
          `ExitOutcome::NoDedicatedThreadRunning` -> `ExitOutcome::NoDeferredCheckRunning`
          in `upgrade_singleton.rs`, move `upgrade_journey/upgrade_journey_struct.rs` ->
          `upgrade_journey/typestates/state_machine.rs`, and update `not_started.rs`,
          `typestates/state_machine.rs`, `upgrade_journey_macro.rs`,
          `journey_integration_tests.rs`, and `cmdr/src/self_upgrade/mod.rs`.
    - [ ] **Step 9**: TODO
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions (ordered top-down: Control Plane ->
      Execution Plane).
    - [ ] `cmdr/src/self_upgrade/mod.rs`
    - **Step A: Entry Point, Config & Struct Envelope (Control Plane)**
        - [x] `cmdr/src/self_upgrade/upgrade_journey/mod.rs`
        - [x] `cmdr/src/self_upgrade/upgrade_journey/upgrade_journey_macro.rs`
        - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/state_machine.rs`
        - [ ] `cmdr/src/self_upgrade/config/mod.rs`
        - [ ] `cmdr/src/self_upgrade/config/upgrade_config.rs`
        - [ ] `cmdr/src/self_upgrade/config/upgrade_policy.rs`
        - [ ] `cmdr/src/self_upgrade/config/cli_arg.rs`
        - [ ] `cmdr/src/self_upgrade/config/user_feedback_channel.rs`
        - [ ] `cmdr/src/self_upgrade/upgrade_journey/start_outcome.rs`
        - [ ] `cmdr/src/self_upgrade/upgrade_journey/tokio_thread_runner.rs`
    - **Step B: Core Orchestrator & Active Typestates (Control Plane)**
        - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/not_started.rs`
        - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/checking.rs`
        - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/upgrading.rs`
        - [ ] `cmdr/src/self_upgrade/upgrade_journey/upgrade_singleton.rs`
    - **Step C: Leaf Typestates & Journey Integration Tests (Control Plane)**
        - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/mod.rs`
        - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/disabled.rs`
        - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/cooldown_skipped.rs`
        - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/up_to_date.rs`
        - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/upgrade_available.rs`
        - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/check_failed.rs`
        - [ ] `cmdr/src/self_upgrade/upgrade_journey/typestates/completed.rs`
        - [ ] `cmdr/src/self_upgrade/upgrade_journey/journey_integration_tests.rs`
    - **Step D: Stateless Services & Value Types (Execution Plane)**
        - [ ] `cmdr/src/self_upgrade/constants.rs`
        - [ ] `cmdr/src/self_upgrade/check/mod.rs`
        - [ ] `cmdr/src/self_upgrade/check/fetch_manifest.rs`
        - [ ] `cmdr/src/self_upgrade/check/cooldown.rs`
        - [ ] `cmdr/src/self_upgrade/upgrade/mod.rs`
        - [ ] `cmdr/src/self_upgrade/upgrade/perform_upgrade.rs`
        - [ ] `cmdr/src/self_upgrade/upgrade/prompt.rs`
        - [ ] `cmdr/src/self_upgrade/upgrade/upgrade_source.rs`
        - [ ] `cmdr/src/self_upgrade/upgrade/release.rs`
        - [ ] `cmdr/src/self_upgrade/upgrade/target.rs`
        - [ ] `cmdr/src/self_upgrade/upgrade/ui_strings.rs`
        - [ ] `cmdr/src/self_upgrade/upgrade/feedback_device.rs`
    - [ ] `task/binaries-self-upgrade-support.md`

### Phase 3: Bootstrap, Manifest & Release Script

**External release pipeline & binary distribution repository**:

- [x] Create and configure `https://github.com/r3bl-org/r3bl-binaries` repository with
      metadata, topics, and initial `README.md` on `main`.
- [ ] External release pipeline (`versions.jsonc`, `install.sh`, `install.ps1`, and
      multi-platform binary archive publishing to `r3bl-org/r3bl-binaries` and
      `r3bl.com`):
    - [ ] Phase 1: Build manifest update CLI wrapping
          `r3bl_cmdr::JsoncManifest::try_update_crate_in_jsonc()` and deploying/pushing to
          `../r3bl_website` and `../r3bl-binaries`.
    - [ ] Phase 2: Author canonical `versions.jsonc`, `install.sh`, and `install.ps1` and
          deploy them to `../r3bl-binaries` and `../r3bl_website`.
    - [ ] Phase 3: Build release orchestration script (`release.fish`) for multi-platform
          binary packaging, GitHub Release uploads to `r3bl-org/r3bl-binaries`, and
          `versions.jsonc` publishing.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `../r3bl-binaries/README.md`

### Phase 4: Refactor cmdr

**Integration into `r3bl-cmdr` suite (`giti`, `edi`, `env-source`)**:

- [ ] Flatten `UpgradeCLIArgs` (`--upgrade` and `--upgrade-behavior`) into `giti`
      (`cmdr/src/giti/clap_config.rs`), `edi` (`cmdr/src/edi/clap_config.rs`), and
      `env-source` (`cmdr/src/env_source/clap_config.rs`).
- [ ] Start `upgrade_journey!` at startup in `cmdr/src/bin/giti.rs`,
      `cmdr/src/bin/edi.rs`, and `cmdr/src/bin/env-source.rs` (exiting on
      `StartOutcome::UpgradedAndExit(ImmediateUpgradeOutcome)`), and invoke
      `handle_upgrade_on_exit()` before exit:
    - `giti`:
      `upgrade_journey!(UserFeedbackChannel::Stdout, cli_arg.upgrade_args).start()`
    - `edi`: `upgrade_journey!(UserFeedbackChannel::Stdout, cli_arg.upgrade_args).start()`
    - `env-source`:
      `upgrade_journey!(UserFeedbackChannel::Stderr, cli_arg.upgrade_args).start()`
- [ ] Refactor `cmdr/src/analytics_client/upgrade_check.rs` and `ui_str.rs` to delegate to
      `crate::self_upgrade::*`.
- [ ] Run `cmdr` unit tests and verify `giti`, `edi`, and `env-source` compile and
      function.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `cmdr/src/analytics_client/upgrade_check.rs`
    - [ ] `cmdr/src/analytics_client/ui_str.rs`
    - [ ] `cmdr/src/bin/giti.rs`
    - [ ] `cmdr/src/bin/edi.rs`
    - [ ] `cmdr/src/bin/env-source.rs`
    - [ ] `cmdr/src/giti/clap_config.rs`
    - [ ] `cmdr/src/edi/clap_config.rs`
    - [ ] `cmdr/src/env_source/clap_config.rs`
    - [ ] `cmdr/Cargo.toml`

### Phase 5: Integrate build-infra

**Integration into `r3bl-build-infra` (`cargo-rustdoc-fmt`)**:

- [ ] Add `r3bl-cmdr` dependency to `build-infra/Cargo.toml`.
- [ ] Flatten `UpgradeCLIArgs` (`--upgrade` and `--upgrade-behavior`) into `CLIArg` in
      `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`.
- [ ] Start `upgrade_journey!(UserFeedbackChannel::Stdout, cli_arg.upgrade_args).start()`
      in `build-infra/src/bin/cargo-rustdoc-fmt.rs` (exiting on
      `StartOutcome::UpgradedAndExit(ImmediateUpgradeOutcome)`) and call
      `handle_upgrade_on_exit()` before exit.
- [ ] Ensure `--upgrade-behavior` explicitly controls exit behavior without inspecting
      magic environment variables like `CI`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `build-infra/src/bin/cargo-rustdoc-fmt.rs`
    - [ ] `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`
    - [ ] `build-infra/Cargo.toml`

### Phase 6: Integrate mcp-server

**Integration into `r3bl-rust-analyzer-mcp-server`**:

- [ ] Add `r3bl-cmdr` dependency to `rust-analyzer-mcp-server/Cargo.toml`.
- [ ] Flatten `UpgradeCLIArgs` (`--upgrade` and `--upgrade-behavior`) into `CLIArg` in
      `rust-analyzer-mcp-server/src/cli_arg.rs`.
- [ ] Start `upgrade_journey!(UserFeedbackChannel::Stderr, cli_arg.upgrade_args).start()`
      at startup in `rust-analyzer-mcp-server/src/main.rs` (exiting on
      `StartOutcome::UpgradedAndExit(ImmediateUpgradeOutcome)`) and invoke
      `handle_upgrade_on_exit()` at shutdown.
- [ ] Ensure `--upgrade-behavior` explicitly controls shutdown behavior without inspecting
      magic environment variables like `CI`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `rust-analyzer-mcp-server/src/cli_arg.rs`
    - [ ] `rust-analyzer-mcp-server/src/main.rs`
    - [ ] `rust-analyzer-mcp-server/Cargo.toml`

### Phase 7: Docs & Website

**Documentation, skills, release guides, and website updates**:

- [ ] Update `cmdr/README.md` and `cmdr/src/lib.rs`:
    - [ ] Add 1-line bootstrap install commands (`https://r3bl.com/install.sh` and
          `https://r3bl.com/install.ps1`).
    - [ ] Document `--upgrade` and `--upgrade-behavior` CLI arguments for `giti`, `edi`,
          and `env-source`.
    - [ ] Add end-of-file fallback instructions: `cargo install r3bl-cmdr --force`.
- [ ] Update `build-infra/README.md` and `build-infra/src/lib.rs`:
    - [ ] Add bootstrap install commands (`https://r3bl.com/install.sh` /
          `https://r3bl.com/install.ps1` with `build-infra`).
    - [ ] Document `cargo-rustdoc-fmt --upgrade` / `cargo rustdoc-fmt --upgrade` and
          `--upgrade-behavior`.
    - [ ] Add end-of-file fallback instructions: `cargo install r3bl-build-infra --force`.
- [ ] Update `rust-analyzer-mcp-server/README.md` and
      `rust-analyzer-mcp-server/src/lib.rs`:
    - [ ] Add bootstrap install commands (`https://r3bl.com/install.sh` /
          `https://r3bl.com/install.ps1` with `mcp-server`).
    - [ ] Document `rust-analyzer-mcp-server --upgrade` and `--upgrade-behavior` CLI
          arguments.
    - [ ] Add end-of-file fallback instructions:
          `cargo install r3bl-rust-analyzer-mcp-server --force`.
- [ ] Update root `README.md`:
    - [ ] Consolidate all binary tools with quick 1-line bootstrap install commands from
          `r3bl.com`.
    - [ ] Document `--upgrade` and `--upgrade-behavior` usage across tools.
    - [ ] Add fallback `cargo install <crate> --force` instructions.
- [ ] Update `r3bl.com` website in `../r3bl_website/index.html`:
    - [ ] Replace placeholder content with product showcase for `r3bl-cmdr` (`giti`,
          `edi`, `env-source`), `r3bl-build-infra` (`cargo-rustdoc-fmt`), and
          `r3bl-rust-analyzer-mcp-server`.
    - [ ] Provide prominent copy-pasteable bootstrap install commands
          (`https://r3bl.com/install.sh` and `https://r3bl.com/install.ps1`).
    - [ ] Provide fallback source compilation instructions
          (`cargo install <crate> --force`).
    - [ ] Clean up and modernize `../r3bl_website/css/styles.css` and `css/root_vars.css`.
- [ ] Update `.agents/skills/release-crate/SKILL.md` to incorporate
      `./release.fish <crate> <version>`, `github.com/r3bl-org/r3bl-binaries`, and
      `../r3bl_website/versions.jsonc`.
- [ ] Update `docs/release-guide.md` to reflect binary artifact generation, upload to
      `github.com/r3bl-org/r3bl-binaries`, and `versions.jsonc` updates via
      `release.fish`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `cmdr/README.md`
    - [ ] `cmdr/src/lib.rs`
    - [ ] `build-infra/README.md`
    - [ ] `build-infra/src/lib.rs`
    - [ ] `rust-analyzer-mcp-server/README.md`
    - [ ] `rust-analyzer-mcp-server/src/lib.rs`
    - [ ] `README.md`
    - [ ] `../r3bl_website/index.html`
    - [ ] `../r3bl_website/css/styles.css`
    - [ ] `../r3bl_website/css/root_vars.css`
    - [ ] `.agents/skills/release-crate/SKILL.md`
    - [ ] `docs/release-guide.md`

### Phase 8: Verification & Testing

**Comprehensive verification across the entire workspace**:

- [ ] Audit workspace to ensure no unused imports or dead functions remain.
- [ ] Run `./check.fish --check` across the entire workspace.
- [ ] Run `./check.fish --build` across the entire workspace.
- [ ] Run `./check.fish --clippy` across the entire workspace.
- [ ] Run `./check.fish --test` across the entire workspace.
- [ ] Run verification tests matching the
      [Cross-Platform Test Matrix](#cross-platform-test-matrix).
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `task/binaries-self-upgrade-support.md`

## Cross-Platform Test Matrix

Comprehensive validation matrix ensuring correct behavior across all supported operating
systems (Linux, macOS Apple Silicon, macOS Intel, and Windows), network states, crate
distribution (`CrateDistribution::OpenSource` vs. `CrateDistribution::ClosedSource`), and
toolchain environments.

### Test Scenario Matrix

| Scenario                                   | Crate Distribution | Network State                                           | Local Toolchain                   | Target Resolved                  | Expected Outcome & Verification                                                                                                                                                                                      |
| :----------------------------------------- | :----------------- | :------------------------------------------------------ | :-------------------------------- | :------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **1. Fast Binary Upgrade**                 | Open or Closed     | `r3bl.com/versions.jsonc` + `r3bl-org/r3bl-binaries` OK | Any / None needed                 | `ReleaseArchive`                 | Streams `.tar.gz` from `<binaries_release_base_url>`, verifies `.sha256`, swaps binary via `self_replace` (and updates sibling binaries), exits 0.                                                                   |
| **2. Binaries Repo Down + Full Rustup**    | `"open-source"`    | `r3bl-org/r3bl-binaries` 404 / Offline                  | `rustup` on `PATH`                | `BuildFromSourceRustup`          | Displays fallback message, runs `rustup toolchain install nightly --force` with PTY, runs `cargo +nightly install <crate> --version <pinned_ver>` (pinned from `versions.jsonc`), exits 0.                           |
| **3. Binaries Repo Down + Missing Rustup** | `"open-source"`    | `r3bl-org/r3bl-binaries` 404 / Offline                  | No `rustup` on `PATH`             | `Err(NoViableUpgradeSource)`     | Does NOT attempt `cargo +nightly` (`+toolchain` unsupported without rustup proxy, and stable fails on `#![feature]`). Displays clean error that `rustup` is required to compile Nightly crates from source, exits 1. |
| **4. Binaries Repo Down + Zero Toolchain** | `"open-source"`    | `r3bl-org/r3bl-binaries` 404 / Offline                  | No `rustup`, no `cargo` on `PATH` | `Err(NoViableUpgradeSource)`     | Displays clean error: `"Upgrade failed: Binary release server unreachable and no local Rust build toolchain found. Please retry later: <bin> --upgrade"`, exits 1 cleanly without panic.                             |
| **5. Binaries Repo Down (Closed Source)**  | `"closed-source"`  | `r3bl-org/r3bl-binaries` 404 / Offline                  | Any (ignored)                     | `Err(NoViableUpgradeSource)`     | Does NOT attempt `crates.io` fallback. Displays clean error: `"Upgrade failed: Binary release server unreachable. Please retry later: <bin> --upgrade"`, exits 1.                                                    |
| **6. Local Dev Version Ahead of Manifest** | Open or Closed     | `r3bl.com/versions.jsonc` OK                            | Any                               | `UpgradeCheckStatus::UpToDate`   | Local `CARGO_PKG_VERSION` >= `versions.jsonc` `version`; strict `remote > local` comparison evaluates false so local dev binary is never downgraded.                                                                 |
| **7. Mid-Stream Network Drop**             | `"open-source"`    | Download aborts mid-stream                              | `rustup` on `PATH`                | Dynamic fallback to Source Build | Download stream error caught, temp files cleaned up, seamlessly transitions to `BuildFromSourceRustup` with `--version <pinned_ver>` from `versions.jsonc`.                                                          |
| **8. User Cancellation (Ctrl+C)**          | Open or Closed     | Interrupted during download or compilation              | Any                               | Interrupted                      | Cooperative `tokio::signal::ctrl_c()` caught, PTY session / HTTP stream terminated, temp files purged, original binary remains 100% intact.                                                                          |
| **9. First-Time Bootstrap Install**        | Open or Closed     | `r3bl.com` + `r3bl-org/r3bl-binaries` 200 OK            | None required                     | Script installer                 | `r3bl.com/install.sh` (Linux/macOS) or `r3bl.com/install.ps1` (Windows) reads `versions.jsonc`, extracts `.tar.gz` archive to `~/.cargo/bin` or `%USERPROFILE%\.cargo\bin`, sets perms.                              |
| **10. First-Time Install Outage (Open)**   | `"open-source"`    | `r3bl-org/r3bl-binaries` Down / Offline                 | Rust toolchain present            | Manual Cargo fallback            | `install.{sh,ps1}` runs `cargo install <crate> --version <pinned_ver> --force` (or manual `cargo install <crate> --force` if `r3bl.com` is also offline).                                                            |
| **11. MCP Server & `env-source` Stdout**   | Open or Closed     | Any upgrade or normal execution                         | Any                               | `UserFeedbackChannel::Stderr`    | All progress, spinners, and logs route exclusively to `stderr`. `stdout` remains 100% pure JSON-RPC (`mcp-server`) or shell export statements (`env-source`).                                                        |

### OS-Specific Verification Checklist

#### Linux (`x86_64-unknown-linux-gnu`)

Use the repository's `systemd-nspawn` test infrastructure (`cmdr/systemd-nspawn/` and
`/home/nazmul/github/notes/files/scripts/tests`) to validate installation and upgrade
behaviors across clean container environments for 3 major Linux distributions:

- **Target Distributions**:
    - **Arch Linux** (`cmdr-arch`): Rolling release, pacman package manager.
    - **Ubuntu / Debian** (`cmdr-ubuntu`): Ubuntu 24.04 LTS (Noble), apt package manager.
    - **Fedora** (`cmdr-fedora`): Fedora 41, dnf package manager.
- **Verification Steps in `systemd-nspawn` Containers**:
    - [ ] Run `./create-containers.fish all` to initialize fresh distro root filesystems.
    - [ ] Test `install.sh` bootstrap in clean containers without Rust or build toolchains
          installed.
    - [ ] Test `giti --upgrade`, `edi --upgrade`, `env-source --upgrade`,
          `cargo-rustdoc-fmt --upgrade`, and `rust-analyzer-mcp-server --upgrade`.
    - [ ] Verify in-place binary swap replaces inode cleanly via POSIX `rename`.
    - [ ] Test fallback source compilation with Linux PTY (`/dev/pts`) when GitHub is
          simulated offline.
    - [ ] Verify `--upgrade-behavior` explicitly selects `Auto`, `PromptToUpgrade`,
          `NotifyOnly`, or `Disable` without inspecting magic environment variables like
          `CI`.
    - [ ] Run `./cleanup.fish` after verification completes.
- **Follow-up Workspace Task**:
    - [ ] Consolidate and generalize `cmdr/systemd-nspawn/` and
          `/home/nazmul/github/notes/files/scripts/tests` into a root-level workspace test
          suite (`tests/nspawn/`) that tests all workspace binaries.

#### macOS Apple Silicon (`aarch64-apple-darwin`) & Intel (`x86_64-apple-darwin`)

- [ ] Test `install.sh` bootstrap on both M-series (ARM64) and Intel (x86_64) macOS.
- [ ] Verify automatic target triple detection distinguishes between
      `aarch64-apple-darwin` and `x86_64-apple-darwin`.
- [ ] Verify `.tar.gz` extraction and executable permissions.
- [ ] Test fallback source compilation with macOS PTY (`openpty`).
- [ ] Verify macOS quarantine / gatekeeper attributes do not prevent executing the
      upgraded binary.

#### Windows (`x86_64-pc-windows-msvc`)

- [ ] Test `install.ps1` bootstrap in standard Windows PowerShell.
- [ ] Verify `.tar.gz` archive extraction and `.exe` path resolution in
      `%USERPROFILE%\.cargo\bin`.
- [ ] Verify `self_replace` open file lock handling (renames active `.exe` to `.old`,
      writes new `.exe`, marks `.old` for deletion).
- [ ] Test fallback source compilation with Windows ConPTY (`CreatePseudoConsole`).
- [ ] Test Ctrl+C cancellation during download and compilation under Windows console.
