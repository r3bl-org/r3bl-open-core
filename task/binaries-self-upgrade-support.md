# Task: Binary Self-Upgrade & Bootstrap

<!-- prettier-ignore-start -->
<!-- BEGIN mktoc -->

- [Task: Binary Self-Upgrade & Bootstrap](#task-binary-self-upgrade--bootstrap)
- [Overview](#overview)
    - [Dual-Engine Strategy](#dual-engine-strategy)
    - [Source of Truth](#source-of-truth)
    - [CLI Flag](#cli-flag)
- [Lifecycle Flowchart](#lifecycle-flowchart)
- [Architecture](#architecture)
    - [1. r3bl_tui](#1-r3bl_tui)
    - [2. r3bl-cmdr](#2-r3bl-cmdr)
    - [3. r3bl-build-infra](#3-r3bl-build-infra)
    - [4. r3bl-rust-analyzer-mcp-server](#4-r3bl-rust-analyzer-mcp-server)
    - [5. Bootstrap Scripts & Manifest on `r3bl.com`](#5-bootstrap-scripts--manifest-on-r3blcom)
    - [6. Release Script & `github.com/r3bl-org/r3bl-binaries`](#6-release-script--githubcomr3bl-orgr3bl-binaries)
    - [7. Website & Documentation](#7-website--documentation)
- [Implementation Plan](#implementation-plan)
    - [Phase 1: Shared Upgrade Module](#phase-1-shared-upgrade-module)
    - [Phase 2: Bootstrap, Manifest & Release Script](#phase-2-bootstrap-manifest--release-script)
    - [Phase 3: Refactor cmdr](#phase-3-refactor-cmdr)
    - [Phase 4: Integrate build-infra](#phase-4-integrate-build-infra)
    - [Phase 5: Integrate mcp-server](#phase-5-integrate-mcp-server)
    - [Phase 6: Docs & Website](#phase-6-docs--website)
    - [Phase 7: Verification & Testing](#phase-7-verification--testing)
- [Cross-Platform Test Matrix](#cross-platform-test-matrix)
    - [Test Scenario Matrix](#test-scenario-matrix)
    - [OS-Specific Verification Checklist](#os-specific-verification-checklist) -
      [Linux (`x86_64-unknown-linux-gnu`)](#linux-x86_64-unknown-linux-gnu) -
      [macOS Apple Silicon (`aarch64-apple-darwin`) & Intel (`x86_64-apple-darwin`)](#macos-apple-silicon-aarch64-apple-darwin--intel-x86_64-apple-darwin) -
      [Windows (`x86_64-pc-windows-msvc`)](#windows-x86_64-pc-windows-msvc)

<!-- END mktoc -->
<!-- prettier-ignore-end -->

## Overview

Transition all R3BL binary tools—both open-source (`r3bl-cmdr`, `r3bl-build-infra`,
`r3bl-rust-analyzer-mcp-server`) and future closed-source binaries (`r3bl-base`)—to a
unified, hybrid distribution and self-upgrade architecture.

> **Single Source of Truth (SSOT)**:
> - **Code Schema SSOT**: [`VersionsManifest`] in `tui/src/core/script/upgrade/versions_manifest.rs` is the
>   authoritative Rust schema definition, JSONC parser (`VersionsManifest::from_jsonc_str`), and
>   comment-preserving CST updater (`VersionsManifest::update_crate_in_jsonc_str`) for
>   `https://r3bl.com/versions.jsonc`.
> - **Architectural SSOT**: This document is the technical specification for the shared
>   upgrade engine (`r3bl_tui::core::script::upgrade`), centralized manifest
>   (`https://r3bl.com/versions.jsonc`), bootstrap installers (`https://r3bl.com/install.sh`,
>   `https://r3bl.com/install.ps1`), and binary distribution repository
>   (`https://github.com/r3bl-org/r3bl-binaries`). Proprietary crates in `r3bl-base` import
>   `r3bl_tui` and reference this specification.

1. **Primary (Fast Binary Download — Open & Closed Source)**: 1-2 second HTTP download and
   atomic in-place binary swap via `self_replace` from pre-compiled GitHub Releases hosted
   on the dedicated history-free binary distribution repo
   (`https://github.com/r3bl-org/r3bl-binaries`).
2. **Fallback (Resilient Source Compilation — Open Source Only)**: For open-source crates
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

### Source of Truth

**`r3bl.com/versions.jsonc` as the Sole Version SOT (Schema SSOT: [`VersionsManifest`] in `tui/src/core/script/upgrade/versions_manifest.rs`)**:
Background version checks and bootstrap scripts query `https://r3bl.com/versions.jsonc`
as the single authoritative Source of Truth (SOT) for version numbers and crate
distribution—never `crates.io` or GitHub's rate-limited REST API.

Because we use **JSONC** (JSON with Comments and trailing commas) across the codebase,
`r3bl_tui` parses `versions.jsonc` via `jsonc-parser` (`features = ["serde", "cst"]`):
- **Deserialization (`VersionsManifest::from_jsonc_str`)**: Parses JSONC with `//` line
  comments, `/* ... */` block comments, and trailing commas directly into
  `VersionsManifest`.
- **Comment-Preserving Mutation (`VersionsManifest::update_crate_in_jsonc_str`)**: Uses
  `jsonc_parser::cst::CstRootNode` (Concrete Syntax Tree) so release tooling can update a
  crate's version entry in `versions.jsonc` while preserving 100% of existing comments and
  formatting.

```jsonc
{
    // Schema version for forward compatibility (`ManifestSchemaVersion`).
    "schema_version": 1,
    "crates": {
        // Open-source interactive CLI suite (`giti`, `edi`, `env-source`).
        "r3bl-cmdr": {
            "version": "0.0.25",
            "crate_distribution": "open-source",
            "released_on": "2026-10-17T00:00:00Z"
        },
        // Open-source build & documentation tooling (`cargo-rustdoc-fmt`).
        "r3bl-build-infra": {
            "version": "0.1.0",
            "crate_distribution": "open-source",
            "released_on": "2026-10-17T00:00:00Z"
        },
        // Open-source Rust Analyzer MCP server (`rust-analyzer-mcp-server`).
        "r3bl-rust-analyzer-mcp-server": {
            "version": "0.1.0",
            "crate_distribution": "open-source",
            "released_on": "2026-10-17T00:00:00Z"
        }
    }
}
```

- **Schema Fields (defined in [`VersionsManifest`] & [`CrateVersionMetadata`])**:
    - `schema_version` (`ManifestSchemaVersion(pub u32)`): Schema version integer (`1`).
    - `version` (`CrateVersion(pub String)`): Target semantic version string pinned for
      the crate.
    - `crate_distribution` (`CrateDistribution` with `#[serde(rename_all = "kebab-case")]`):
      Enum witness indicating distribution and licensing model:
        - `"open-source"` (`CrateDistribution::OpenSource`): Open-source crates in
          `r3bl-open-core` eligible for `cargo install` source compilation fallback.
        - `"closed-source"` (`CrateDistribution::ClosedSource`): Closed-source crates in
          `r3bl-base` distributed exclusively as pre-compiled binaries.
    - `released_on` (`chrono::DateTime<chrono::Utc>`): ISO 8601 / RFC 3339 UTC timestamp
      (`YYYY-MM-DDTHH:MM:SSZ`) recording when the release was published (also used by
      commercial binaries in `r3bl-base` to verify the user's `updates_until` maintenance
      window before upgrading).
- **Strictly Newer Version Comparison (`remote_version > local_version`)**:
    - Both remote (`versions.jsonc`) and local (`CARGO_PKG_VERSION`) strings are parsed as
      `(major, minor, patch)` semantic version tuples and compared using
      `remote_version > local_version` (rather than `!=`). This prevents local development
      builds (whose `Cargo.toml` version has been bumped ahead of `versions.jsonc`) from
      falsely triggering an upgrade that would downgrade the local binary.
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

**Dedicated `--upgrade` and `--upgrade-policy` Flags Across All Binaries**: All workspace
binaries provide top-level CLI arguments to run standalone self-upgrades or explicitly
control exit upgrade behavior (with zero magic environment variable inspection):

- `--upgrade`: Supports `giti --upgrade`, `edi --upgrade`, `env-source --upgrade`,
  `cargo-rustdoc-fmt --upgrade` (and `cargo rustdoc-fmt --upgrade`), and
  `rust-analyzer-mcp-server --upgrade`. When passed, the binary bypasses normal
  application startup, executes the upgrade flow, and exits (`0` on success, `1` on
  error).
- `--upgrade-policy <policy>`: Explicit `clap::ValueEnum` override (`interactive-prompt` /
  `prompt`, `auto-upgrade` / `auto`, `notify-only` / `notify`, `never` / `off`). When
  omitted, falls back to the binary's `default_policy`.

## Lifecycle Flowchart

```text
╭──────────────────────────────────────────────────────────────────────────────╮
│ Invocation: Normal Run OR Dedicated Standalone `<binary> --upgrade`          │
╰──────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭──────────────────────────────────────────────────────────────────────────────╮
│ 1. Version Check & SOT Evaluation (`r3bl.com/versions.jsonc`)                │
│                                                                              │
│ GET https://r3bl.com/versions.jsonc                                          │
│ ├─► Parses `version` (pinned), `crate_distribution`, and `released_on`       │
│ ├─► Compares `(major, minor, patch)`: `remote_version > local_version`       │
│ └─► Stores state in `AtomicU8 (UpgradeCheckResult)`                          │
╰──────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭──────────────────────────────────────────────────────────────────────────────╮
│ 2. Trigger Point                                                             │
│                                                                              │
│ ├─► Mode A: Dedicated `--upgrade` CLI flag passed at startup                 │
│ │   └─► Immediately runs upgrade flow, then exits (0 on success, 1 on error) │
│ │                                                                            │
│ └─► Mode B: Application exit phase (`handle_upgrade_at_exit`)                │
│     ├─► InteractivePrompt (giti, edi): Prompt user ("Yes, upgrade" / "No")   │
│     ├─► AutoUpgrade (cargo-rustdoc-fmt, MCP server): Run upgrade             │
│     ├─► NotifyOnly: Print notification                                       │
│     └─► Never: Suppress exit upgrade notifications and actions entirely      │
│     (Controlled explicitly via `--upgrade-policy <policy>`, no magic env)    │
╰──────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭──────────────────────────────────────────────────────────────────────────────╮
│ 3. Strategy Resolution (`resolve_upgrade_strategy`)                          │
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
│     │   ├─► `rustup` on $PATH ──► `UpgradeSource::BuildFromSourceRustup`     │
│     │   ├─► `cargo`  on $PATH ──► `UpgradeSource::BuildFromSourceCargo`      │
│     │   └─► Neither on $PATH  ──► Error: `NoViableUpgradeSource`             │
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
│ ├─► [BuildFromSourceCargo] (OpenSource only; uses pinned_ver)                │
│ │   Run `cargo +nightly install <crate> --version <pinned_ver>` (OSC 9;4).   │
│ │                                                                            │
│ └─► [All Attempts Fail]                                                      │
│     Print clean error with retry instructions: `<binary> --upgrade`          │
╰──────────────────────────────────────────────────────────────────────────────╯
```

## Architecture

### 1. r3bl_tui

**Shared Upgrade Engine**: The unified upgrade module is housed inside
`r3bl_tui::core::script::upgrade`:

- [x] `constants.rs`:
    - [x] Centralized string constants: `toolchain_binaries` (`rustup`, `cargo`),
          `toolchain_args` (`toolchain`, `install`, `nightly`, `+nightly`, `--version`,
          `--force`), `archive` (`NON_BINARY_EXTENSIONS`, `DEFAULT_ARCHIVE_FILENAME`,
          `EXTRACTED_DIR_NAME`, `UPGRADE_TMP_SUFFIX`), `manifest_keys` (`schema_version`,
          `crates`, `version`, `crate_distribution`, `released_on`), and
          `upgrade_reasons`.
- [x] `versions_manifest.rs`:
    - [x] `VersionsManifest`: Single Source of Truth (SSOT) for the
          `https://r3bl.com/versions.jsonc` schema, providing `from_jsonc_str()` and
          comment-preserving `update_crate_in_jsonc_str()` via `jsonc-parser` CST.
    - [x] Domain newtypes and enums: `ManifestSchemaVersion(pub u32)`,
          `CrateDistribution` (`OpenSource` / `ClosedSource`,
          `#[serde(rename_all = "kebab-case")]`), `CrateName(pub String)`,
          `CrateVersion(pub String)`.
    - [x] `CrateVersionMetadata`: Deserialized entry from `r3bl.com/versions.jsonc`
          (`version: CrateVersion`, `crate_distribution: CrateDistribution`,
          `released_on: chrono::DateTime<chrono::Utc>`).
- [x] `upgrade_types.rs`:
    - [x] `UpgradeConfig`: Per-binary configuration (`crate_name: CrateName`,
          `local_version: CrateVersion`, `policy: UpgradePolicy`,
          `output_target: UpgradeOutputTarget`), resolving `policy` deterministically via
          `cli_policy.unwrap_or(default_policy)`.
    - [x] `UpgradeSource`: `GithubReleaseArtifact`, `BuildFromSourceRustup`,
          `BuildFromSourceCargo`.
    - [x] `UpgradePolicy`: `InteractivePrompt` (interactive TUI apps), `AutoUpgrade`
          (headless / agent tooling), `NotifyOnly`, and `Never` (derives
          `clap::ValueEnum` for `--upgrade-policy <policy>`).
    - [x] `UpgradeOutputTarget`: `Stdout` (default CLI/TUI), `Stderr` (MCP server and
          stdout-piped tools like `env-source`), and `Quiet` (structured tracing only).
    - [x] `UpgradeCheckResult`: `NotChecked`, `Checking`, `UpToDate`, `UpgradeAvailable`,
          `FailedCheck`.
    - [x] `UpgradeError`: Error types for strategy resolution, checksum mismatch, platform
          support, and user cancellation.
- [x] `target_triple.rs`:
    - [x] `get_host_target_triple()`: Automatically resolves platform triple
          (`x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`,
          `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`).
- [x] `github_release.rs`:
    - [x] Constructs direct asset download URL on `github.com/r3bl-org/r3bl-binaries`:
          `https://github.com/r3bl-org/r3bl-binaries/releases/download/v<version>-<crate>/<crate>-v<version>-<target>.tar.gz`,
          plus `.sha256`.
    - [x] Performs lightweight HTTP `HEAD` asset accessibility check.
- [x] `strategy_resolver.rs`:
    - [x] `resolve_upgrade_strategy(crate_name, pinned_version, distribution)`: Evaluates
          `github.com/r3bl-org/r3bl-binaries` asset availability; if unavailable and
          `distribution == CrateDistribution::OpenSource`, checks local toolchain presence
          (`rustup` / `cargo`) on `$PATH`. If `CrateDistribution::ClosedSource`, returns
          `Err(NoViableUpgradeSource)`.
- [x] `version_check.rs`:
    - [x] `try_get_crate_metadata_from_r3bl_versions_json(crate_name)`: Fetches
          `https://r3bl.com/versions.jsonc` and parses `CrateVersionMetadata` via
          `VersionsManifest::from_jsonc_str`.
    - [x] Compares semantic versions strictly (`remote_version > local_version`) so local
          development builds with bumped `Cargo.toml` versions never trigger a downgrade.
    - [x] `start_background_version_check(config)`: Non-blocking async task running the
          SOT version check (no-op if `config.policy == UpgradePolicy::Never`).
    - [x] `start_background_version_check_sync(config)`: Spawns a detached OS thread with
          a lightweight `current_thread` Tokio runtime for synchronous binaries (such as
          `rust-analyzer-mcp-server` which intentionally runs without `#[tokio::main]`).
    - [x] Updates atomic state `UpgradeCheckResult` and caches the resolved
          `CrateVersionMetadata` (`version`, `crate_distribution`, `released_on`).
- [x] `source_build.rs`:
    - [x] Generalized PTY session manager (`PtySessionBuilder` with
          `DefaultPtySessionConfig + PtySessionConfigToken::CaptureOsc`) for `rustup`
          toolchain updates and
          `cargo +nightly install <crate> --version <pinned_version_from_versions_jsonc> --force`.
    - [x] Never installs an unpinned version from `crates.io`; always pins `--version` to
          the exact version resolved from `r3bl.com/versions.jsonc`.
    - [x] `OscPtyEvent::ProgressUpdate` (`OSC 9;4`) compilation escape sequence handler
          and live `Spinner` updater.
- [x] `run_upgrade.rs`:
    - [x] `handle_upgrade_at_exit()` / `handle_upgrade_at_exit_sync()`: Coordinates exit
          phase based on policy and output target.
    - [x] `run_upgrade()` / `run_upgrade_sync()`: Executes the resolved `UpgradeSource`
          (streaming `.tar.gz` download from `github.com/r3bl-org/r3bl-binaries` +
          `.sha256` verification + archive extraction via `flate2`/`tar` + `self_replace`
          on current binary and updating any sibling binaries in the unpacked archive, OR
          PTY/OSC `rustup` + `cargo +nightly install <crate> --version <pinned_ver>` when
          `CrateDistribution::OpenSource`).
    - [x] `run_upgrade_cli_arg_interceptor()`: Helper for standalone `--upgrade` CLI
          invocations.
    - [x] Listens for cooperative `tokio::signal::ctrl_c()`.
- [x] `ui_strings.rs`:
    - [x] Parameterized messages for GitHub download progress (% + MB), fallback
          transition notices, source compilation progress, success, and failure/retry
          notices.
- [x] `mod.rs`: Private modules and public barrel exports re-exported at `r3bl_tui::*`.

### 2. r3bl-cmdr

**Interactive Command-Line & Developer Productivity Tools (`giti`, `edi`, `env-source`)**:

- [ ] Add `--upgrade` and `--upgrade-policy` CLI arguments to `giti`
      (`cmdr/src/giti/clap_config.rs`), `edi` (`cmdr/src/edi/clap_config.rs`), and
      `env-source` (`cmdr/src/env_source/clap_config.rs`).
- [ ] Intercept `--upgrade` at startup in `cmdr/src/bin/giti.rs`, `cmdr/src/bin/edi.rs`,
      and `cmdr/src/bin/env-source.rs` to run standalone upgrade and exit.
- [ ] **Special handling for `env-source`**:
    - [ ] `env-source` is a synchronous, sub-2ms shell startup tool whose `stdout` is
          piped into `source` / `Invoke-Expression`
          (`env-source -i ~/.profile -o fish | source`).
    - [ ] Normal `env-source -i ...` runs must **not** spawn network version checks or
          print anything to `stdout` (preserving its ~1.8 ms execution time and pure shell
          export stream on `stdout`).
    - [ ] `env-source --upgrade` runs the standalone upgrade flow on-demand via
          `run_upgrade_sync()` with `UpgradeOutputTarget::Stderr`.
- [ ] Refactor `cmdr/src/analytics_client/upgrade_check.rs` and `ui_str.rs` (used by
      `giti` and `edi`) to delegate to `r3bl_tui::core::script::upgrade::*` using
      `UpgradePolicy::InteractivePrompt` and `UpgradeOutputTarget::Stdout`.
- [ ] Preserve `cmdr`-specific interactive prompts, `ExitContext` handling, emojis, and
      lolcat greetings.
- [ ] Release archive for `r3bl-cmdr` on `github.com/r3bl-org/r3bl-binaries` bundles all 3
      binaries: `giti`, `edi`, and `env-source`. When any one of the 3 binaries upgrades,
      it uses `self_replace` on itself and updates the other 2 sibling binaries in the
      same directory so the entire suite stays on the same version.

### 3. r3bl-build-infra

**Build Infrastructure & Documentation Tools (`cargo-rustdoc-fmt`)**:

- [ ] Add `--upgrade` and `--upgrade-policy` CLI arguments to `CLIArg` in
      `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs` (compatible with both
      `cargo-rustdoc-fmt --upgrade` and `cargo rustdoc-fmt --upgrade` via
      `strip_cargo_subcommand_injection()`).
- [ ] Intercept `--upgrade` at startup in `build-infra/src/bin/cargo-rustdoc-fmt.rs`.
- [ ] Spawn background version check at startup in `cargo-rustdoc-fmt.rs`.
- [ ] At exit, invoke `r3bl_tui::handle_upgrade_at_exit()` with
      `UpgradeConfig::new("r3bl-build-infra", env!("CARGO_PKG_VERSION"), args.upgrade_policy, UpgradePolicy::AutoUpgrade, UpgradeOutputTarget::Stdout)`.
- [ ] Release archive for `r3bl-build-infra` on `github.com/r3bl-org/r3bl-binaries`
      bundles `cargo-rustdoc-fmt` (and any future binaries added to `build-infra`, such as
      `spawny` in `task/build-infra-spawny.md`).

### 4. r3bl-rust-analyzer-mcp-server

**Model Context Protocol Semantic Analysis Server (`rust-analyzer-mcp-server`)**:

- [ ] **Pure `std::thread` Compatibility**: `rust-analyzer-mcp-server` intentionally runs
      without a global `#[tokio::main]` async runtime on its MCP/LSP bridge threads. It
      uses the synchronous entry points (`start_background_version_check_sync`,
      `handle_upgrade_at_exit_sync`, `run_upgrade_sync`) which isolate async HTTP/PTY I/O
      inside a dedicated background/on-demand thread.
- [ ] Add `--upgrade` and `--upgrade-policy` CLI arguments to `CLIArg` in
      `rust-analyzer-mcp-server/src/cli_arg.rs` (making `workspace` optional when
      `--upgrade` is passed).
- [ ] Intercept `--upgrade` at startup in `rust-analyzer-mcp-server/src/main.rs`.
- [ ] Spawn background version check at startup in `main.rs`.
- [ ] At shutdown/exit, invoke `r3bl_tui::handle_upgrade_at_exit_sync()` with
      `UpgradeConfig::new("r3bl-rust-analyzer-mcp-server", env!("CARGO_PKG_VERSION"), args.upgrade_policy, UpgradePolicy::AutoUpgrade, UpgradeOutputTarget::Stderr)`
      to guarantee `stdout` JSON-RPC message transport remains 100% uncorrupted.
- [ ] Release archive for `r3bl-rust-analyzer-mcp-server` on
      `github.com/r3bl-org/r3bl-binaries` bundles `rust-analyzer-mcp-server`.

### 5. Bootstrap Scripts & Manifest on `r3bl.com`

**Centralized Installers (`r3bl.com/install.sh`, `r3bl.com/install.ps1`) & Version
Manifest (`r3bl.com/versions.jsonc`)**:

- [ ] Hosted in `../r3bl_website/` so they are served directly from `https://r3bl.com/`
      (with canonical copies maintained in `r3bl-open-core` and synced to
      `../r3bl_website/`):
    - [ ] `https://r3bl.com/versions.jsonc`: Central JSONC manifest (with preserved
          comments) mapping crate names to
          `{ "version", "crate_distribution", "released_on" }`.
    - [ ] `https://r3bl.com/install.sh`: POSIX shell bootstrap script for Linux & macOS
          (`curl -LsSf https://r3bl.com/install.sh | sh`).
    - [ ] `https://r3bl.com/install.ps1`: PowerShell bootstrap script for Windows
          (`irm https://r3bl.com/install.ps1 | iex`).
- [ ] Supports optional target crate argument (`cmdr` (default), `build-infra`,
      `mcp-server`, `all`, or any crate entry present in `versions.jsonc`).
- [ ] **Bootstrap Execution Flow**:
    - [ ]   1. Fetches `https://r3bl.com/versions.jsonc` to resolve the target crate's
           pinned `version` and `crate_distribution` (`"open-source"` vs
           `"closed-source"`).
    - [ ]   2. Downloads the pre-compiled `.tar.gz` archive and `.sha256` checksum from
           `https://github.com/r3bl-org/r3bl-binaries/releases/download/v<ver>-<crate>/<crate>-v<ver>-<target>.tar.gz`,
           verifies the checksum, and extracts all executables inside the archive into
           `~/.cargo/bin` (or `$env:USERPROFILE\.cargo\bin`).
    - [ ]   3. If the binary download fails and `"crate_distribution": "open-source"`,
           checks for `cargo` on `$PATH` and runs
           `cargo install <crate> --version <ver> --force` (strictly using the `<ver>`
           pinned in `versions.jsonc`). If `"crate_distribution": "closed-source"`, prints
           a clean error without attempting a `crates.io` build.
- [ ] **First-Time Install Caveat During Outages**:
    - [ ] For open-source crates, all documentation also provides the manual source
          compilation fallback at the end: `cargo install <crate> --force`.

### 6. Release Script & `github.com/r3bl-org/r3bl-binaries`

**Automated End-to-End Release Automation (`release.fish`)**:

- [x] **Dedicated Binary Distribution Repository (`github.com/r3bl-org/r3bl-binaries`)**:
    - [x] Created at `https://github.com/r3bl-org/r3bl-binaries` (and cloned locally at
          `../r3bl-binaries`) with a single `README.md` commit on `main` and repository
          metadata configured. Used exclusively to host GitHub Release binary archives for
          all R3BL open-source and closed-source tools.
- [ ] `release.fish`: Automated, end-to-end release manager supporting both binary and
      library crates:
    - [ ]   1. **Version Validation**: Fails fast if target version is not strictly
           greater than current version.
    - [ ]   2. **CHANGELOG.md & Release Notes Verification**: Fails fast if `CHANGELOG.md`
           lacks an entry for `### v<version>` or if the standalone release notes file
           `docs/release-notes/<crate>/v<version>.md` does not exist.
    - [ ]   3. **Quality Checks**: Executes `./check.fish --full`.
    - [ ]   4. **Multi-Platform Binary Packaging**: For binary crates (`r3bl-cmdr`,
           `r3bl-build-infra`, `r3bl-rust-analyzer-mcp-server`), builds Linux native
           binaries, cross-compiles Windows `.exe` binaries (and optionally collects macOS
           fleet artifacts), strips symbols, and packages `.tar.gz` archives with `.sha256`
           checksums into `dist/`.
    - [ ]   5. **Dry-Run Validation**: For open-source crates, runs
           `cargo publish -p <crate> --dry-run --allow-dirty --no-verify`.
    - [ ]   6. **Interactive Confirmation**: Prompts for explicit user approval before
           publishing.
    - [ ]   7. **Atomic Publish, Binary Upload & `versions.jsonc` Update**:
        - [ ] Bumps `Cargo.toml`, regenerates `README.md` via `cargo readme` (except
              `rust-analyzer-mcp-server` which uses a hand-crafted `README.md`), creates
              git commit/tag, pushes to remote, and publishes open-source crates to
              `crates.io`.
        - [ ] Creates the GitHub Release on `github.com/r3bl-org/r3bl-binaries` using:
              `gh release create <tag> --repo r3bl-org/r3bl-binaries --title "<tag>" --notes-file docs/release-notes/<crate>/v<version>.md`
              with attached `dist/*` binary assets (`.tar.gz` and `.sha256`).
        - [ ] Updates `../r3bl_website/versions.jsonc` (preserving all comments) with the
              new `version`, `crate_distribution`, and `released_on` UTC timestamp (plus
              syncs `install.sh` and `install.ps1` to `../r3bl_website/`).

### 7. Website & Documentation

**Consolidated Showcase & Installation Guides Across All Mediums**:

- [ ] **Crate READMEs & `lib.rs` Modules**: Each binary crate (`cmdr`, `build-infra`,
      `rust-analyzer-mcp-server`, and root `r3bl-open-core`) provides:
    - [ ]   1. Primary: 1-line bootstrap script (`https://r3bl.com/install.sh` /
           `https://r3bl.com/install.ps1`).
    - [ ]   2. Feature docs & `--upgrade` / `--upgrade-policy` CLI flag instructions.
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
- [x] Create `tui/src/core/script/upgrade/versions_manifest.rs` defining
      `VersionsManifest` (the SSOT for `versions.jsonc` with `from_jsonc_str` and
      CST-based `update_crate_in_jsonc_str`), `ManifestSchemaVersion`, `CrateDistribution`,
      `CrateName`, `CrateVersion`, and `CrateVersionMetadata`.
- [x] Create `tui/src/core/script/upgrade/upgrade_types.rs` defining `UpgradeConfig`,
      `UpgradeSource`, `UpgradePolicy`, `UpgradeOutputTarget`, `UpgradeCheckResult`, and
      `UpgradeError`.
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
      `https://r3bl.com/versions.jsonc` as the sole SOT via
      `VersionsManifest::from_jsonc_str`, comparing versions strictly via
      `(major, minor, patch)` (`remote_version > local_version`), with both async and
      sync-thread variants and `AtomicU8` state management.
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
      comments and trailing commas), CST comment-preserving updates, strict
      `remote > local` version comparison, `upgrade_types`, `versions_manifest`, strategy
      resolver (both `CrateDistribution::OpenSource` and `CrateDistribution::ClosedSource`),
      target triple, and UI strings.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `tui/Cargo.toml`
    - [ ] `tui/src/core/script/upgrade/constants.rs`
    - [ ] `tui/src/core/script/upgrade/upgrade_types.rs`
    - [ ] `tui/src/core/script/upgrade/versions_manifest.rs`
    - [ ] `tui/src/core/script/upgrade/target_triple.rs`
    - [ ] `tui/src/core/script/upgrade/github_release.rs`
    - [ ] `tui/src/core/script/upgrade/strategy_resolver.rs`
    - [ ] `tui/src/core/script/upgrade/source_build.rs`
    - [ ] `tui/src/core/script/upgrade/version_check.rs`
    - [ ] `tui/src/core/script/upgrade/run_upgrade.rs`
    - [ ] `tui/src/core/script/upgrade/ui_strings.rs`
    - [ ] `tui/src/core/script/upgrade/mod.rs`
    - [ ] `tui/src/core/script/mod.rs`

### Phase 2: Bootstrap, Manifest & Release Script

**Bootstrap scripts (`install.sh`, `install.ps1`), `versions.jsonc`, and `release.fish`**:

- [x] Create and configure `https://github.com/r3bl-org/r3bl-binaries` repository with
      metadata, topics, and initial `README.md` on `main`.
- [ ] Create `../r3bl_website/versions.jsonc` initialized with schema v1 (`version`,
      `crate_distribution`, `released_on` and explanatory comments for `r3bl-cmdr`,
      `r3bl-build-infra`, and `r3bl-rust-analyzer-mcp-server`).
- [ ] Create `install.sh` (at repo root and synced to `../r3bl_website/install.sh`) for
      Linux and macOS bootstrap installation via `https://r3bl.com/install.sh`.
- [ ] Create `install.ps1` (at repo root and synced to `../r3bl_website/install.ps1`) for
      Windows PowerShell bootstrap installation via `https://r3bl.com/install.ps1`.
- [ ] Create `release.fish` at repo root implementing:
    - [ ] `<crate>` and `<version>` argument parsing with interactive fallback.
    - [ ] Version validation against `<crate>/Cargo.toml`.
    - [ ] `CHANGELOG.md` entry and `docs/release-notes/<crate>/v<version>.md`
          verification.
    - [ ] Binary multi-platform compilation & packaging into `dist/` (Linux `.tar.gz` and
          Windows `.tar.gz` cross-compiled directly on Linux) with `.sha256` checksums.
    - [ ] `cargo publish --dry-run --allow-dirty --no-verify` and explicit user
          confirmation prompt.
    - [ ] Automated version bump, git commit, git tag (`v<version>-<crate>`), git push,
          `cargo publish`, binary upload via
          `gh release create --repo r3bl-org/r3bl-binaries --notes-file docs/release-notes/<crate>/v<version>.md`
          with attached `dist/*` assets, and updating `../r3bl_website/versions.jsonc`
          (preserving comments).
- [ ] Test `release.fish` validation guardrails (`--dry-run`, missing changelog/release
      notes, invalid version).
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `../r3bl_website/versions.jsonc`
    - [ ] `install.sh`
    - [ ] `../r3bl_website/install.sh`
    - [ ] `install.ps1`
    - [ ] `../r3bl_website/install.ps1`
    - [ ] `release.fish`

### Phase 3: Refactor cmdr

**Integration into `r3bl-cmdr` suite (`giti`, `edi`, `env-source`)**:

- [ ] Add `--upgrade` and `--upgrade-policy` CLI flags to `giti`
      (`cmdr/src/giti/clap_config.rs`), `edi` (`cmdr/src/edi/clap_config.rs`), and
      `env-source` (`cmdr/src/env_source/clap_config.rs`).
- [ ] Intercept `--upgrade` at startup in `cmdr/src/bin/giti.rs`, `cmdr/src/bin/edi.rs`,
      and `cmdr/src/bin/env-source.rs` to run standalone upgrade and exit.
- [ ] Refactor `cmdr/src/analytics_client/upgrade_check.rs` and `ui_str.rs` to delegate to
      `r3bl_tui::core::script::upgrade::*`.
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

### Phase 4: Integrate build-infra

**Integration into `r3bl-build-infra` (`cargo-rustdoc-fmt`)**:

- [ ] Add `--upgrade` and `--upgrade-policy` CLI flags to `CLIArg` in
      `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`.
- [ ] Intercept `--upgrade` at startup in `build-infra/src/bin/cargo-rustdoc-fmt.rs`.
- [ ] Integrate background check and exit upgrade handler with
      `UpgradeConfig::new("r3bl-build-infra", env!("CARGO_PKG_VERSION"), args.upgrade_policy, UpgradePolicy::AutoUpgrade, UpgradeOutputTarget::Stdout)`.
- [ ] Ensure `--upgrade-policy` explicitly controls exit behavior without inspecting
      magic environment variables like `CI`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `build-infra/src/bin/cargo-rustdoc-fmt.rs`
    - [ ] `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`
    - [ ] `build-infra/Cargo.toml`

### Phase 5: Integrate mcp-server

**Integration into `r3bl-rust-analyzer-mcp-server`**:

- [ ] Add `--upgrade` and `--upgrade-policy` CLI flags to `CLIArg` in
      `rust-analyzer-mcp-server/src/cli_arg.rs`.
- [ ] Intercept `--upgrade` at startup in `rust-analyzer-mcp-server/src/main.rs` via
      `run_upgrade_sync()`.
- [ ] Integrate background check (`start_background_version_check_sync`) and shutdown
      upgrade handler (`handle_upgrade_at_exit_sync` with
      `UpgradeConfig::new("r3bl-rust-analyzer-mcp-server", env!("CARGO_PKG_VERSION"), args.upgrade_policy, UpgradePolicy::AutoUpgrade, UpgradeOutputTarget::Stderr)`).
- [ ] Ensure `--upgrade-policy` explicitly controls shutdown behavior without inspecting
      magic environment variables like `CI`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `rust-analyzer-mcp-server/src/cli_arg.rs`
    - [ ] `rust-analyzer-mcp-server/src/main.rs`
    - [ ] `rust-analyzer-mcp-server/Cargo.toml`

### Phase 6: Docs & Website

**Documentation, skills, release guides, and website updates**:

- [ ] Update `cmdr/README.md` and `cmdr/src/lib.rs`:
    - [ ] Add 1-line bootstrap install commands (`https://r3bl.com/install.sh` and
          `https://r3bl.com/install.ps1`).
    - [ ] Document `--upgrade` and `--upgrade-policy` CLI arguments for `giti`, `edi`, and
          `env-source`.
    - [ ] Add end-of-file fallback instructions: `cargo install r3bl-cmdr --force`.
- [ ] Update `build-infra/README.md` and `build-infra/src/lib.rs`:
    - [ ] Add bootstrap install commands (`https://r3bl.com/install.sh` /
          `https://r3bl.com/install.ps1` with `build-infra`).
    - [ ] Document `cargo-rustdoc-fmt --upgrade` / `cargo rustdoc-fmt --upgrade` and
          `--upgrade-policy`.
    - [ ] Add end-of-file fallback instructions: `cargo install r3bl-build-infra --force`.
- [ ] Update `rust-analyzer-mcp-server/README.md` and
      `rust-analyzer-mcp-server/src/lib.rs`:
    - [ ] Add bootstrap install commands (`https://r3bl.com/install.sh` /
          `https://r3bl.com/install.ps1` with `mcp-server`).
    - [ ] Document `rust-analyzer-mcp-server --upgrade` and `--upgrade-policy` CLI
          arguments.
    - [ ] Add end-of-file fallback instructions:
          `cargo install r3bl-rust-analyzer-mcp-server --force`.
- [ ] Update root `README.md`:
    - [ ] Consolidate all binary tools with quick 1-line bootstrap install commands from
          `r3bl.com`.
    - [ ] Document `--upgrade` and `--upgrade-policy` usage across tools.
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
      this task file as the SSOT for the shared upgrade engine, `r3bl.com/versions.jsonc`,
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

### Phase 7: Verification & Testing

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

| Scenario                                     | Crate Distribution   | Network State                                           | Local Toolchain                    | Target Resolved                  | Expected Outcome & Verification                                                                                                                                                            |
| :------------------------------------------- | :------------------- | :------------------------------------------------------ | :--------------------------------- | :------------------------------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **1. Fast Binary Upgrade**                   | Open or Closed       | `r3bl.com/versions.jsonc` + `r3bl-org/r3bl-binaries` OK | Any / None needed                  | `GithubReleaseArtifact`          | Streams `.tar.gz` from `github.com/r3bl-org/r3bl-binaries`, verifies `.sha256`, swaps binary via `self_replace` (and updates sibling binaries), exits 0.                                   |
| **2. Binaries Repo Down + Full Rustup**      | `"open-source"`      | `r3bl-org/r3bl-binaries` 404 / Offline                  | `rustup` + `cargo` on `$PATH`      | `BuildFromSourceRustup`          | Displays fallback message, runs `rustup toolchain install nightly --force` with PTY, runs `cargo +nightly install <crate> --version <pinned_ver>` (pinned from `versions.jsonc`), exits 0. |
| **3. Binaries Repo Down + Standalone Cargo** | `"open-source"`      | `r3bl-org/r3bl-binaries` 404 / Offline                  | `cargo` on `$PATH` (no `rustup`)   | `BuildFromSourceCargo`           | Displays fallback message, runs `cargo +nightly install <crate> --version <pinned_ver>` (pinned from `versions.jsonc`), does not error on missing `rustup`, exits 0.                       |
| **4. Binaries Repo Down + Zero Toolchain**   | `"open-source"`      | `r3bl-org/r3bl-binaries` 404 / Offline                  | No `rustup`, no `cargo` on `$PATH` | `Err(NoViableUpgradeSource)`     | Displays clean error: `"Upgrade failed: GitHub release unreachable and no local Rust build toolchain found. Please retry later: <bin> --upgrade"`, exits 1 cleanly without panic.          |
| **5. Binaries Repo Down (Closed Source)**    | `"closed-source"`    | `r3bl-org/r3bl-binaries` 404 / Offline                  | Any (ignored)                      | `Err(NoViableUpgradeSource)`     | Does NOT attempt `crates.io` fallback. Displays clean error: `"Upgrade failed: Binary release server unreachable. Please retry later: <bin> --upgrade"`, exits 1.                          |
| **6. Local Dev Version Ahead of SOT**        | Open or Closed       | `r3bl.com/versions.jsonc` OK                            | Any                                | `UpgradeCheckResult::UpToDate`   | Local `CARGO_PKG_VERSION` >= `versions.jsonc` `version`; strict `remote > local` comparison evaluates false so local dev binary is never downgraded.                                       |
| **7. Mid-Stream Network Drop**               | `"open-source"`      | Download aborts mid-stream                              | `rustup` or `cargo` on `$PATH`     | Dynamic fallback to Source Build | Download stream error caught, temp files cleaned up, seamlessly transitions to `BuildFromSourceRustup` or `BuildFromSourceCargo` with `--version <pinned_ver>` from `versions.jsonc`.      |
| **8. User Cancellation (Ctrl+C)**            | Open or Closed       | Interrupted during download or compilation              | Any                                | Interrupted                      | Cooperative `tokio::signal::ctrl_c()` caught, PTY session / HTTP stream terminated, temp files purged, original binary remains 100% intact.                                                |
| **9. First-Time Bootstrap Install**          | Open or Closed       | `r3bl.com` + `r3bl-org/r3bl-binaries` 200 OK            | None required                      | Script installer                 | `r3bl.com/install.sh` (Linux/macOS) or `r3bl.com/install.ps1` (Windows) reads `versions.jsonc`, extracts `.tar.gz` archive to `~/.cargo/bin` or `$env:USERPROFILE\.cargo\bin`, sets perms. |
| **10. First-Time Install Outage (Open)**     | `"open-source"`      | `r3bl-org/r3bl-binaries` Down / Offline                 | Rust toolchain present             | Manual Cargo fallback            | `install.{sh,ps1}` runs `cargo install <crate> --version <pinned_ver> --force` (or manual `cargo install <crate> --force` if `r3bl.com` is also offline).                                  |
| **11. MCP Server & `env-source` Stdout**     | Open or Closed       | Any upgrade or normal execution                         | Any                                | `UpgradeOutputTarget::Stderr`    | All progress, spinners, and logs route exclusively to `stderr`. `stdout` remains 100% pure JSON-RPC (`mcp-server`) or shell export statements (`env-source`).                              |

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
    - [ ] Verify `--upgrade-policy` explicitly selects `AutoUpgrade`, `InteractivePrompt`,
          `NotifyOnly`, or `Never` without inspecting magic environment variables like `CI`.
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
      `$env:USERPROFILE\.cargo\bin`.
- [ ] Verify `self_replace` open file lock handling (renames active `.exe` to `.old`,
      writes new `.exe`, marks `.old` for deletion).
- [ ] Test fallback source compilation with Windows ConPTY (`CreatePseudoConsole`).
- [ ] Test Ctrl+C cancellation during download and compilation under Windows console.
