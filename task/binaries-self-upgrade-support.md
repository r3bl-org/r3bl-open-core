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
`r3bl-rust-analyzer-mcp-server`) and future closed-source binaries (`r3bl-base`), to a
unified, hybrid distribution and self-upgrade architecture.

> **Architecture & Schema Reference**:
>
> - **Code Schema**: [`JsoncManifest`] in
>   `cmdr/src/self_upgrade/parser/jsonc_manifest_entry_point.rs` defines the Rust schema,
>   JSONC parser (`JsoncManifest::try_parse_jsonc`), and comment-preserving CST updater
>   (`JsoncManifest::try_update_crate_in_jsonc`) for `https://r3bl.com/versions.jsonc`.
> - **Architecture Specification**: This document is the technical specification for the
>   shared client-side upgrade engine (`r3bl_cmdr::self_upgrade`), centralized manifest
>   schema (`https://r3bl.com/versions.jsonc`), and binary distribution repository
>   (`https://github.com/r3bl-org/r3bl-binaries`).
> - **Maintainer Release Pipeline (`r3bl-base`)**: The private release orchestrator
>   (`release.fish`), manifest publisher CLI (`r3bl-manifest-update`), and canonical
>   authoring of `versions.jsonc`, `install.sh`, and `install.ps1` live in `../r3bl-base`
>   and are tracked in
>   [`../r3bl-base/task/task_release_pipeline.md`](../../r3bl-base/task/task_release_pipeline.md).

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
`cmdr/src/self_upgrade/parser/jsonc_manifest_entry_point.rs`)**: Dedicated-thread version
checks (bounded by a `2s` timeout and a `1h` `mtime` cache cooldown per crate) and
bootstrap scripts query `https://r3bl.com/versions.jsonc` for version numbers and crate
distribution, never `crates.io` or GitHub's rate-limited REST API.

Because we use **JSONC** (JSON with Comments and trailing commas) across the codebase,
`r3bl_cmdr` parses `versions.jsonc` via `jsonc-parser` (`features = ["serde", "cst"]`):

- **Deserialization (`JsoncManifest::try_parse_jsonc`)**: Parses JSONC with `//` line
  comments, `/* ... */` block comments, and trailing commas, ignoring unknown future
  fields so additive schema extensions never break older binaries in the wild.
- **Comment-Preserving Mutation (`JsoncManifest::try_update_crate_in_jsonc`)**: Uses
  `jsonc_parser::cst::CstRootNode` (Concrete Syntax Tree) so release tooling can update a
  crate's version entry in `versions.jsonc` while preserving 100% of existing comments and
  formatting.
    - See
      [`r3bl-base/task/task_release_pipeline.md`](../../r3bl-base/task/task_release_pipeline.md)
      for the `r3bl-base` CLI binary (`r3bl-manifest-update`) that calls
      `JsoncManifest::try_update_crate_in_jsonc()` and deploys `versions.jsonc` to
      `r3bl.com` and `github.com/r3bl-org/r3bl-binaries`.

```jsonc
{
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

- **Schema Fields (defined in [`JsoncManifest`] & [`CrateVersionMetadata`])**:
    - **Additive-Only Forward Compatibility**: Neither `JsoncManifest` nor
      `CrateVersionMetadata` uses `#[serde(deny_unknown_fields)]`, so any new fields added
      in the future are safely ignored by existing binaries in the wild.
    - `version` (`CrateVersion` struct with
      `pub major: u16, pub minor: u16, pub patch: u16` and
      `#[serde(try_from = "String", into = "String")]`): Validated semantic version parsed
      via slice pattern matching (`[major_str, minor_str, patch_str]`) at deserialization
      time, with explicit `Ord` / `PartialOrd` comparing
      `(self.major, self.minor, self.patch)` in SemVer precedence order.
    - `crate_distribution` (`CrateDistribution` with
      `#[serde(rename_all = "kebab-case")]`): Enum witness indicating distribution and
      licensing model:
        - `"open-source"` (`CrateDistribution::OpenSource`): Open-source crates in
          `r3bl-open-core` eligible for `cargo install` source compilation fallback.
        - `"closed-source"` (`CrateDistribution::ClosedSource`): Closed-source crates in
          `r3bl-base` distributed exclusively as pre-compiled binaries.
    - `released_on` (`chrono::DateTime<chrono::Utc>`): ISO 8601 / RFC 3339 UTC timestamp
      (`YYYY-MM-DDTHH:MM:SSZ`) recording when the release was published (also used by
      commercial binaries in `r3bl-base` to verify the user's `updates_until` maintenance
      window before upgrading).
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
│     ├─► Parses `version` (pinned), `crate_distribution`, and `released_on`   │
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
│ HEAD https://github.com/r3bl-org/r3bl-binaries/releases/download/            │
│      v{pinned_ver}-{crate}/{crate}-v{pinned_ver}-{target}.tar.gz             │
│                                                                              │
│ ├─► Returns 200 / 302:                                                       │
│ │   └─► `UpgradeSource::GithubReleaseArtifact`                               │
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
│ ├─► [GithubReleaseArtifact]                                                  │
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
`r3bl-base`) depend on both `r3bl_tui` and `r3bl-cmdr`:

- [x] `constants.rs`:
    - [x] Centralized constants: `toolchain_bin` (`rustup`, `cargo`), `toolchain_cli_args`
          (`toolchain`, `install`, `nightly`, `+nightly`, `--version`, `--force`),
          `upgrade_cli_args` (`UPGRADE_ARG_LONG`, `UPGRADE_BEHAVIOR_ARG_LONG`),
          `manifest_timeouts` (`IMMEDIATE_MANIFEST_TIMEOUT`,
          `DEDICATED_THREAD_MANIFEST_TIMEOUT`, `DEDICATED_THREAD_CHECK_COOLDOWN`),
          `check_cooldown_cache` (`CACHE_TOP_LEVEL_DIR`, `UPGRADE_CHECK_CACHE_SUBDIR`,
          `LAST_CHECK_FILE_EXT`), `archive`, `versions_manifest_keys`, and
          `upgrade_reasons`.
- [x] `parser/`:
    - [x] `jsonc_manifest_entry_point.rs`: `JsoncManifest` schema for
          `https://r3bl.com/versions.jsonc`, providing `try_parse_jsonc()` (ignoring
          unknown future fields) and comment-preserving `try_update_crate_in_jsonc()` via
          `jsonc-parser` CST.
    - [x] Domain newtypes and enums: `CrateDistribution` (`OpenSource` / `ClosedSource`,
          `#[serde(rename_all = "kebab-case")]`), `CrateName(pub String)`,
          `CrateVersion { pub major: u16, pub minor: u16, pub patch: u16 }` (parsed via
          slice pattern matching with `#[serde(try_from = "String", into = "String")]` and
          explicit tuple `.cmp()` `Ord`), and `CrateMetadata`.
- [x] `config/`:
    - [x] `crate_identity.rs`: `CrateIdentity` (`crate_name: CrateName`,
          `current_version: CrateVersion`).
    - [x] `cli_arg.rs`: Opaque `UpgradeCLIArgs` (`--upgrade`, `--upgrade-behavior` backed
          by private `CliUpgradeBehavior`) with `From<UpgradeCLIArgs> for UpgradePolicy`
          at the CLI boundary.
    - [x] `upgrade_policy.rs`: Pure domain model with `UpgradePolicy`
          (`Enabled(UpgradeStrategy)`, `Disabled`), `UpgradeStrategy` (`Immediate`,
          `DedicatedThread(UpgradeActionOnExit)`), and `UpgradeActionOnExit`
          (`PromptToUpgrade`, `Auto`, `NotifyOnly`).
    - [x] `upgrade_config.rs`: Immutable `UpgradeConfig` (`crate_identity`,
          `user_feedback_channel`, `upgrade_policy`, `jsonc_manifest_url`,
          `binaries_release_base_url`, `cooldown_policy`) and `CooldownPolicy`
          (`DefaultCacheDir`, `CustomTimestampFilePath(PathBuf)`, `Disabled`).
    - [x] `user_feedback_channel.rs`: `UserFeedbackChannel` (`Stdout`, `Stderr`, `Quiet`).
- [x] `upgrade_journey_public_api.rs` & `upgrade_check_cache.rs`:
    - [x] `upgrade_journey!` macro, `UpgradeJourney<State>` typestate machine across 9
          states (`NotStarted`, `Disabled`, `CooldownSkipped`, `Checking`, `UpToDate`,
          `UpgradeAvailable`, `CheckFailed`, `Upgrading`, `Completed`), `StartupOutcome`
          (`UpgradedAndExit(ImmediateUpgradeResult)`, `ContinueApplication`,
          `AlreadyStarted`), and `SINGLETON` (`UpgradeCheckStatus` +
          `thread_check_handle: Option<JoinHandle<()>>` with
          `UpgradeCheckStatus::handle_upgrade_on_exit()`).
- [x] `check/`:
    - [x] `check_entry_point.rs`: `check_for_upgrade(journey: UpgradeJourney<Checking>)`.
    - [x] `manifest_query.rs`: `try_get_jsonc_manifest_from_url(url, timeout)`.
    - [x] `cooldown.rs`: `1h` `mtime` stamp check and touch helpers
          (`<cache_dir>/r3bl/upgrade-check/<crate_name>.last_check`).
- [x] `upgrade/`:
    - [x] `target.rs`: `get_host_target_triple()` (`x86_64-unknown-linux-gnu`,
          `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`,
          `aarch64-pc-windows-msvc`).
    - [x] `bin_name.rs`: `get_current_bin_name()`.
    - [x] `release.rs`: Constructs direct asset download URL on
          `github.com/r3bl-org/r3bl-binaries` and performs lightweight HTTP `HEAD` asset
          accessibility check.
    - [x] `upgrade_source.rs`: `UpgradeSource` (`GithubReleaseArtifact`,
          `BuildFromSourceRustup`) with `UpgradeSource::try_resolve` and
          `UpgradeSource::try_fallback_to_source`.
    - [x] `build.rs`: Generalized PTY session manager (`PtySessionBuilder` with
          `DefaultPtySessionConfig + PtySessionConfigToken::CaptureOsc`) for `rustup`
          toolchain updates and
          `cargo +nightly install <crate> --version <pinned_version_from_versions_jsonc> --force`.
    - [x] `download.rs`: Streams binary archive from `github.com/r3bl-org/r3bl-binaries`,
          verifies SHA-256 checksum, updates `Spinner`, and checks for
          `tokio::signal::ctrl_c()`.
    - [x] `swap.rs`: Unpacks `.tar.gz` and performs atomic in-place binary swapping via
          `self_replace` on current executable and updates any sibling binaries.
    - [x] `prompt.rs`: Interactive exit prompt (`choose`) returning
          `PromptUpgradeDecision` (`UpgradeNow`, `DeclinedByUser`,
          `SkippedNonInteractive`).
    - [x] `ui_strings.rs`, `feedback_device.rs`, `upgrade_entry_point.rs`
          (`perform_upgrade`), and `upgrade_error.rs` (`UpgradeError`).
- [x] `mod.rs`: Private modules and public barrel exports re-exported at `r3bl_cmdr::*`.

### 2. r3bl-cmdr (Binaries: `giti`, `edi`, `env-source`)

**Interactive Command-Line & Developer Productivity Tools (`giti`, `edi`, `env-source`)**:

- [ ] Flatten `UpgradeCLIArgs` (`--upgrade` and `--upgrade-behavior`) into `giti`
      (`cmdr/src/giti/clap_config.rs`), `edi` (`cmdr/src/edi/clap_config.rs`), and
      `env-source` (`cmdr/src/env_source/clap_config.rs`).
- [ ] Invoke `upgrade_journey!(channel, cli_arg.upgrade_args).start()` at startup in
      `cmdr/src/bin/giti.rs`, `cmdr/src/bin/edi.rs`, and `cmdr/src/bin/env-source.rs`
      (exiting immediately on `StartupOutcome::UpgradedAndExit`), and invoke
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
      `UpgradeActionOnExit::Auto` as the default (with `UpgradeActionOnExit::PromptToUpgrade`
      available via `--upgrade-behavior prompt-to-upgrade`) and
      `UserFeedbackChannel::Stdout`.
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
      `StartupOutcome::UpgradedAndExit`) and call
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
      `StartupOutcome::UpgradedAndExit`), and invoke
      `UpgradeCheckStatus::handle_upgrade_on_exit()` at shutdown to guarantee `stdout`
      JSON-RPC message transport remains 100% uncorrupted.
- [ ] Release archive for `r3bl-rust-analyzer-mcp-server` on
      `github.com/r3bl-org/r3bl-binaries` bundles `rust-analyzer-mcp-server`.

### 5. Bootstrap Scripts & Manifest on `r3bl.com`

**Centralized Installers (`r3bl.com/install.sh`, `r3bl.com/install.ps1`) & Version
Manifest (`r3bl.com/versions.jsonc`)**:

- Authored canonically in `../r3bl-base` and deployed/published by the `r3bl-base` release
  pipeline to both `../r3bl_website/` (`https://r3bl.com/`) and `../r3bl-binaries/`
  (`https://github.com/r3bl-org/r3bl-binaries`). See
  [`../r3bl-base/task/task_release_pipeline.md`](../../r3bl-base/task/task_release_pipeline.md)
  for the complete authoring and deployment implementation plan:
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

**Maintainer Release Pipeline in `r3bl-base`
(`../r3bl-base/task/task_release_pipeline.md`)**:

- [x] **Dedicated Binary Distribution Repository (`github.com/r3bl-org/r3bl-binaries`)**:
    - [x] Created at `https://github.com/r3bl-org/r3bl-binaries` (and cloned locally at
          `../r3bl-binaries`) with a single `README.md` commit on `main` and repository
          metadata configured. Used to host GitHub Release binary archives (`.tar.gz` and
          `.sha256`) and mirror `versions.jsonc`, `install.sh`, and `install.ps1` for all
          R3BL open-source and closed-source tools.
- [ ] **Release Orchestration & Manifest Publisher (`../r3bl-base`)**:
    - [ ] Tracked in
          [`../r3bl-base/task/task_release_pipeline.md`](../../r3bl-base/task/task_release_pipeline.md)
          (`r3bl-manifest-update` CLI binary calling
          `r3bl_cmdr::JsoncManifest::try_update_crate_in_jsonc()`, canonical
          `versions.jsonc` / `install.sh` / `install.ps1`, and `release.fish` deploying to
          `../r3bl-binaries` and `../r3bl_website`).

### 7. Website & Documentation

**Consolidated Showcase & Installation Guides Across All Mediums**:

- [ ] **Crate READMEs & `lib.rs` Modules**: Each binary crate (`cmdr`, `build-infra`,
      `rust-analyzer-mcp-server`, and root `r3bl-open-core`) provides:
    - [ ] 1. Primary: 1-line bootstrap script (`https://r3bl.com/install.sh` /
          `https://r3bl.com/install.ps1`).
    - [ ] 2. Feature docs & `--upgrade` / `--upgrade-behavior` CLI flag instructions.
    - [ ] 3. End-of-file fallback instructions: `cargo install <crate> --force`.
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
      and `UpgradeAvailable` (`action_on_exit: UpgradeActionOnExit`) carry narrowed witnesses
      unpacked from `config.get_upgrade_policy()`. All `into_*` transition methods are
      `pub(super)` so external crates cannot forge state transitions:

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
    - Calling `.start(self) -> StartupOutcome` atomically claims `SINGLETON` if its status
      is `UpgradeCheckStatus::NotStarted`. Any subsequent `.start()` call in the same
      process leaves `SINGLETON` untouched, spawns zero threads, logs via
      `DEBUG_SELF_UPGRADE`, and returns `StartupOutcome::AlreadyStarted`.
    - `StartupOutcome::UpgradedAndExit(ImmediateUpgradeResult)` returns a `Copy` domain
      enum (`Succeeded | Failed`, with `exit_code(self) -> std::process::ExitCode`)
      instead of `miette::Result<()>` so `UserFeedbackChannel` remains the single output
      emitter (preventing duplicate `stderr` errors and `Quiet` leaks in
      `fn main() -> CommonResult`):

        ```rust
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum ImmediateUpgradeResult {
            Succeeded,
            Failed,
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum StartupOutcome {
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
      using standard fail-fast `ScopedMutex` locking in production (and
      `lock_raw_poison_safe` + `handle.join()` drain inside
      `#[cfg(test)] pub(crate) fn reset_for_testing()`):

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
let outcome = journey.start(); // handle StartupOutcome::UpgradedAndExit(Succeeded | Failed)
  │
  ├── [Duplicate .start() when SINGLETON != NotStarted]
  │     └── 0 threads spawned, SINGLETON unchanged ──► StartupOutcome::AlreadyStarted
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

| File                                                      | Responsibility in Typestate Architecture                                                                                                                                                                                                                                                                                                                                   |
| :-------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **`cmdr/src/self_upgrade/constants.rs`**                  | Defines `upgrade_cli_args::{UPGRADE_ARG_LONG, UPGRADE_BEHAVIOR_ARG_LONG}`, `manifest_timeouts::{IMMEDIATE_MANIFEST_TIMEOUT (5s), DEDICATED_THREAD_MANIFEST_TIMEOUT (2s), DEDICATED_THREAD_CHECK_COOLDOWN (1h)}`, and `check_cooldown_cache`.                                                                                                                               |
| **`cmdr/src/self_upgrade/config/cli_arg.rs`**             | Opaque `clap` CLI boundary adapter (`UpgradeCLIArgs` with private fields + private `CliUpgradeBehavior`) and `From<UpgradeCLIArgs> for UpgradePolicy`.                                                                                                                                                                                                                     |
| **`cmdr/src/self_upgrade/config/upgrade_policy.rs`**      | Pure domain model: `UpgradePolicy` (`Enabled(UpgradeStrategy)`, `Disabled`), `UpgradeStrategy` (`Immediate`, `DedicatedThread(UpgradeActionOnExit)` with `manifest_timeout()`), and `UpgradeActionOnExit` (`PromptToUpgrade`, `Auto`, `NotifyOnly`).                                                                                                                       |
| **`cmdr/src/self_upgrade/config/upgrade_config.rs`**      | Immutable `UpgradeConfig` (`crate_identity`, `user_feedback_channel`, `upgrade_policy`, URLs, `cooldown_policy`) + `CooldownPolicy` (`DefaultCacheDir`, `CustomTimestampFilePath(PathBuf)`, `Disabled`) constructed via `UpgradeConfig::new()`. `upgrade_config!` macro is deleted.                                                                                        |
| **`cmdr/src/self_upgrade/check/cooldown.rs`**             | `CooldownStatus` (`Active`, `ExpiredOrMissing`), `check_cooldown_status(&UpgradeConfig)`, and `record_successful_check(&UpgradeConfig)` using `<cache_dir>/r3bl/upgrade-check/<crate_name>.last_check` `mtime`.                                                                                                                                                            |
| **`cmdr/src/self_upgrade/check/manifest_query.rs`**       | `try_get_jsonc_manifest_from_url(url: &str, timeout: Duration)` parameterized by `UpgradeStrategy::manifest_timeout()`.                                                                                                                                                                                                                                                    |
| **`cmdr/src/self_upgrade/check/check_entry_point.rs`**    | `check_for_upgrade(journey: UpgradeJourney<Checking>)`, `check_for_immediate_upgrade(journey: UpgradeJourney<Checking>)`, and pure `extract_crate_metadata_from_manifest_result` preserving full `format!("{err:#}")` diagnostic chains and touching the `1h` stamp on `UpToDate`.                                                                                         |
| **`cmdr/src/self_upgrade/upgrade/prompt.rs`**             | `PromptUpgradeDecision` (`UpgradeNow`, `DeclinedByUser`, `SkippedNonInteractive`) returned by `prompt_user_to_upgrade` (eliminating boolean blindness on non-interactive terminals).                                                                                                                                                                                       |
| **`cmdr/src/self_upgrade/upgrade/bin_name.rs`**           | `get_current_bin_name()` (replaces deleted `metadata_resolver.rs` and obsolete `get_remote_crate_metadata()`).                                                                                                                                                                                                                                                             |
| **`cmdr/src/self_upgrade/upgrade_journey_public_api.rs`** | Hygiene-hardened `upgrade_journey!` macro (with migrated architectural docs), `UpgradeJourney<State>`, 9 typestate structs (`NotStarted` through `Completed`), `pub(super) fn into_*` transitions, `UpgradeResolution`, `ImmediateUpgradeResult`, `StartupOutcome`, `.start(self)` (1 OS thread via `Builder::spawn`), and `Upgrading::execute_action_on_exit`.           |
| **`cmdr/src/self_upgrade/upgrade_check_cache.rs`**        | Private `UpgradeSingletonState` (`status` + `thread_check_handle`), 9-variant `UpgradeCheckStatus` (dead `try_get_crate_metadata` and `is_upgrade_available` removed), atomic single-start claim, deadlock-safe `join()` + `UpgradeAvailable -> Upgrading -> Completed` exit handling, and poison-safe + thread-draining `#[cfg(test)] pub(crate) fn reset_for_testing()`. |
| **`cmdr/src/self_upgrade/mod.rs`**                        | Module-level narrative and ASCII diagram covering all 3 binary execution paths, Scenario 2a/2b/2c (`thread_check_handle` join + `1h` `mtime` cooldown), `5s` vs. `2s` timeouts, `ImmediateUpgradeResult`, and `StartupOutcome::AlreadyStarted`.                                                                                                                            |

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
      `ImmediateUpgradeResult`, and `StartupOutcome::AlreadyStarted`.
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
      `StartupOutcome::AlreadyStarted`.
- [x] Consolidate `UpgradePolicy` into immutable `UpgradeConfig` (Variant 2A):
    - Move `upgrade_policy: UpgradePolicy` + `get_upgrade_policy(&self) -> UpgradePolicy`
      into `UpgradeConfig` (`cmdr/src/self_upgrade/config/upgrade_config.rs`) and update
      `UpgradeConfig::new(...)` and rustdoc in `upgrade_config.rs` and
      `upgrade_policy.rs`.
    - Collapse `NotStarted` into a unit struct (`pub struct NotStarted;`), simplify
      `UpgradeJourney::<NotStarted>::new(config: UpgradeConfig)`, dispatch `.start(self)`
      via `self.config.get_upgrade_policy()` (keeping narrowed
      `strategy: UpgradeStrategy` in `Checking` and `action_on_exit: UpgradeActionOnExit` in
      `UpgradeAvailable`), construct `CrateVersion` at compile time via
      `const { CrateVersion::from_cargo_parts(...) }` in `upgrade_journey!` (using Cargo's
      pre-split `CARGO_PKG_VERSION_{MAJOR,MINOR,PATCH,PRE}` while keeping runtime
      `FromStr` slice-pattern validation for `versions.jsonc` / tests), and update unit
      tests across `self_upgrade`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `cmdr/src/self_upgrade/constants.rs`
    - [x] `cmdr/src/self_upgrade/mod.rs`
    - [x] `cmdr/src/self_upgrade/config/mod.rs`
    - [x] `cmdr/src/self_upgrade/config/cli_arg.rs`
    - [x] `cmdr/src/self_upgrade/config/upgrade_policy.rs`
    - [ ] `cmdr/src/self_upgrade/config/upgrade_config.rs`
    - [ ] `cmdr/src/self_upgrade/config/crate_identity.rs`
    - [ ] `cmdr/src/self_upgrade/parser/crate_version.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey_public_api.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_check_cache.rs`
    - [ ] `cmdr/src/self_upgrade/check/mod.rs`
    - [ ] `cmdr/src/self_upgrade/check/check_entry_point.rs`
    - [ ] `cmdr/src/self_upgrade/check/manifest_query.rs`
    - [ ] `cmdr/src/self_upgrade/check/cooldown.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/mod.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/upgrade_entry_point.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/upgrade_source.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/download.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/swap.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/build.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/release.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/target.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/prompt.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/ui_strings.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/feedback_device.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/bin_name.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/upgrade_error.rs`
    - [ ] `cmdr/src/lib.rs`
    - [ ] `cmdr/Cargo.toml`
    - [ ] `tui/src/core/common/mod.rs`
    - [ ] `tui/src/core/common/run_in_tokio.rs`
    - [ ] `tui/src/core/script/mod.rs`
    - [ ] `tui/src/tui/mod.rs`
    - [ ] `tui/src/readline_async/choose_impl/mod.rs`
    - [ ] `tui/Cargo.toml`

### Phase 3: Bootstrap, Manifest & Release Script

**Maintainer release pipeline (`../r3bl-base/task/task_release_pipeline.md`)**:

- [x] Create and configure `https://github.com/r3bl-org/r3bl-binaries` repository with
      metadata, topics, and initial `README.md` on `main`.
- [ ] Complete
      [`../r3bl-base/task/task_release_pipeline.md`](../../r3bl-base/task/task_release_pipeline.md)
      in `../r3bl-base` (private maintainer repository):
    - [ ] Phase 1: Build `r3bl-manifest-update` CLI binary in `../r3bl-base` wrapping
          `r3bl_cmdr::JsoncManifest::try_update_crate_in_jsonc()` and deploying/pushing to
          `../r3bl_website` and `../r3bl-binaries`.
    - [ ] Phase 2: Author canonical `versions.jsonc`, `install.sh`, and `install.ps1` in
          `../r3bl-base` and deploy them to `../r3bl-binaries` and `../r3bl_website`.
    - [ ] Phase 3: Build `release.fish` in `../r3bl-base` to orchestrate multi-platform
          binary packaging, GitHub Release uploads to `r3bl-org/r3bl-binaries`, and
          `versions.jsonc` publishing.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `../r3bl-base/task/task_release_pipeline.md`
    - [ ] `../r3bl-base/README.md`
    - [ ] `../r3bl-base/task/prd_monetization_approach.md`

### Phase 4: Refactor cmdr

**Integration into `r3bl-cmdr` suite (`giti`, `edi`, `env-source`)**:

- [ ] Flatten `UpgradeCLIArgs` (`--upgrade` and `--upgrade-behavior`) into `giti`
      (`cmdr/src/giti/clap_config.rs`), `edi` (`cmdr/src/edi/clap_config.rs`), and
      `env-source` (`cmdr/src/env_source/clap_config.rs`).
- [ ] Start `upgrade_journey!` at startup in `cmdr/src/bin/giti.rs`,
      `cmdr/src/bin/edi.rs`, and `cmdr/src/bin/env-source.rs` (exiting on
      `StartupOutcome::UpgradedAndExit(ImmediateUpgradeResult)`), and invoke
      `UpgradeCheckStatus::handle_upgrade_on_exit()` before exit:
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
      `StartupOutcome::UpgradedAndExit(ImmediateUpgradeResult)`) and call
      `UpgradeCheckStatus::handle_upgrade_on_exit()` before exit.
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
      `StartupOutcome::UpgradedAndExit(ImmediateUpgradeResult)`) and invoke
      `UpgradeCheckStatus::handle_upgrade_on_exit()` at shutdown.
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
- [x] Update `../r3bl-base/task/prd_monetization_approach.md` (Section 7) to reference
      this task file for the shared upgrade engine, `r3bl.com/versions.jsonc`,
      `r3bl.com/install.{sh,ps1}`, and `github.com/r3bl-org/r3bl-binaries`.
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
    - [ ] `../r3bl-base/task/prd_monetization_approach.md`

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
| **1. Fast Binary Upgrade**                 | Open or Closed     | `r3bl.com/versions.jsonc` + `r3bl-org/r3bl-binaries` OK | Any / None needed                 | `GithubReleaseArtifact`          | Streams `.tar.gz` from `github.com/r3bl-org/r3bl-binaries`, verifies `.sha256`, swaps binary via `self_replace` (and updates sibling binaries), exits 0.                                                             |
| **2. Binaries Repo Down + Full Rustup**    | `"open-source"`    | `r3bl-org/r3bl-binaries` 404 / Offline                  | `rustup` on `PATH`                | `BuildFromSourceRustup`          | Displays fallback message, runs `rustup toolchain install nightly --force` with PTY, runs `cargo +nightly install <crate> --version <pinned_ver>` (pinned from `versions.jsonc`), exits 0.                           |
| **3. Binaries Repo Down + Missing Rustup** | `"open-source"`    | `r3bl-org/r3bl-binaries` 404 / Offline                  | No `rustup` on `PATH`             | `Err(NoViableUpgradeSource)`     | Does NOT attempt `cargo +nightly` (`+toolchain` unsupported without rustup proxy, and stable fails on `#![feature]`). Displays clean error that `rustup` is required to compile Nightly crates from source, exits 1. |
| **4. Binaries Repo Down + Zero Toolchain** | `"open-source"`    | `r3bl-org/r3bl-binaries` 404 / Offline                  | No `rustup`, no `cargo` on `PATH` | `Err(NoViableUpgradeSource)`     | Displays clean error: `"Upgrade failed: GitHub release unreachable and no local Rust build toolchain found. Please retry later: <bin> --upgrade"`, exits 1 cleanly without panic.                                    |
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
