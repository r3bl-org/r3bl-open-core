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

> **Single Source of Truth (SSOT)**: This document is the technical SSOT for the shared
> upgrade engine (`r3bl_tui::core::script::upgrade`), centralized manifest
> (`https://r3bl.com/versions.json`), bootstrap installers (`https://r3bl.com/install.sh`,
> `https://r3bl.com/install.ps1`), and binary distribution repository
> (`https://github.com/r3bl-org/r3bl-binaries`). Proprietary crates in `r3bl-base` import
> `r3bl_tui` and reference this specification.

1. **Primary (Fast Binary Download — Open & Closed Source)**: 1-2 second HTTP download and
   atomic in-place binary swap via `self_replace` from pre-compiled GitHub Releases hosted
   on the dedicated history-free binary distribution repo
   (`https://github.com/r3bl-org/r3bl-binaries`).
2. **Fallback (Resilient Source Compilation — Open Source Only)**: For open-source crates
   (`"open_source": true` in `versions.json`), if `github.com/r3bl-org/r3bl-binaries` is
   down, blocked, or lacks a release asset for the host target triple, automatically fall
   back to building the pinned version from `crates.io` using the shared PTY session
   manager (`rustup toolchain install nightly --force` +
   `cargo +nightly install <crate> --version <ver>`) with live OSC 9;4 compilation
   progress tracking. Closed-source binaries (`"open_source": false`) have no source
   fallback and exit with a clean retry message.

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
  All pre-compiled `.tar.gz` (Linux/macOS) and `.zip` (Windows) archives for both
  open-source and closed-source tools are attached as GitHub Release assets on
  `github.com/r3bl-org/r3bl-binaries`. Because release assets live on GitHub's object
  storage CDN (`objects.githubusercontent.com`) outside `.git` history, the repository
  remains near 0 KB forever and never counts against GitHub's 1 GB Git repository quota.
- **Resilient Fallback for Open-Source Crates**: If GitHub CDN is unreachable and the
  crate is marked `"open_source": true`, the engine checks the local environment for
  `rustup` or `cargo` and automatically compiles the target version from `crates.io`.
- **Universal Branded Bootstrap Scripts (`r3bl.com`)**: Single-line first-time installs
  served directly from `r3bl.com`:
    - Linux/macOS: `curl -LsSf https://r3bl.com/install.sh | sh`
    - Windows: `irm https://r3bl.com/install.ps1 | iex`

### Source of Truth

**`r3bl.com/versions.json` as the Sole Version SOT**: Background version checks and
bootstrap scripts query `https://r3bl.com/versions.json` as the single authoritative
Source of Truth (SOT) for version numbers and crate visibility—never `crates.io` or
GitHub's rate-limited REST API:

```json
{
    "schema_version": 1,
    "crates": {
        "r3bl-cmdr": {
            "version": "0.0.25",
            "open_source": true,
            "released_on": "2026-10-17T00:00:00Z"
        },
        "r3bl-build-infra": {
            "version": "0.1.0",
            "open_source": true,
            "released_on": "2026-10-17T00:00:00Z"
        },
        "r3bl-rust-analyzer-mcp-server": {
            "version": "0.1.0",
            "open_source": true,
            "released_on": "2026-10-17T00:00:00Z"
        }
    }
}
```

- **Schema Fields**:
    - `version`: Target semantic version string pinned for the crate.
    - `open_source`: Boolean indicating whether the crate is published on `crates.io` and
      eligible for `cargo install` source compilation fallback (`true` for open-source
      crates in `r3bl-open-core`, `false` for closed-source crates in `r3bl-base`).
    - `released_on`: ISO 8601 UTC timestamp (`YYYY-MM-DDTHH:MM:SSZ`) recording when the
      release was published (also used by commercial binaries in `r3bl-base` to verify the
      user's `updates_until` maintenance window before upgrading).
- **Strictly Newer Version Comparison (`remote_version > local_version`)**:
    - Both remote (`versions.json`) and local (`CARGO_PKG_VERSION`) strings are parsed as
      `(major, minor, patch)` semantic version tuples and compared using
      `remote_version > local_version` (rather than `!=`). This prevents local development
      builds (whose `Cargo.toml` version has been bumped ahead of `versions.json`) from
      falsely triggering an upgrade that would downgrade the local binary.
- **Pinned Version in `crates.io` Compilation Fallback (Open Source Only)**:
    - When `open_source == true` and the pre-compiled binary cannot be downloaded from
      `github.com/r3bl-org/r3bl-binaries`, the fallback compilation codepath **never**
      installs an unpinned "latest" version from `crates.io`. Instead, it **always**
      passes `--version <pinned_ver>` using the exact version `<pinned_ver>` read from
      `r3bl.com/versions.json`:
      `cargo +nightly install <crate_name> --version <pinned_ver>`.
    - Closed-source binaries (`open_source == false`) have no fallback and only download
      pre-compiled binaries from `github.com/r3bl-org/r3bl-binaries`.
- **Direct CDN Asset Checking**: When an upgrade is available, a lightweight HTTP `HEAD`
  request directly checks the `r3bl-org/r3bl-binaries` GitHub Release asset download URL:
  `https://github.com/r3bl-org/r3bl-binaries/releases/download/v{ver}-{crate}/{crate}-v{ver}-{target}.tar.gz`
  (Linux/macOS) or `.zip` (Windows). This hits GitHub's global CDN directly with zero REST
  API overhead.

### CLI Flag

**Dedicated `--upgrade` Flag Across All Binaries**: All workspace binaries provide a
top-level `--upgrade` CLI argument to run standalone self-upgrades without running the
main application:

- Supports `giti --upgrade`, `edi --upgrade`, `env-source --upgrade`,
  `cargo-rustdoc-fmt --upgrade` (and `cargo rustdoc-fmt --upgrade`), and
  `rust-analyzer-mcp-server --upgrade`.
- When passed, the binary bypasses normal application startup, executes the upgrade flow,
  and exits (`0` on success, `1` on error).

## Lifecycle Flowchart

```text
╭──────────────────────────────────────────────────────────────────────────────╮
│ Invocation: Normal Run OR Dedicated Standalone `<binary> --upgrade`          │
╰──────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭──────────────────────────────────────────────────────────────────────────────╮
│ 1. Version Check & SOT Evaluation (`r3bl.com/versions.json`)                 │
│                                                                              │
│ GET https://r3bl.com/versions.json                                           │
│ ├─► Parses `version` (pinned), `open_source`, and `released_on`              │
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
│     └─► NotifyOnly (CI=true / R3BL_NO_AUTO_UPGRADE=1): Print notification    │
╰──────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭──────────────────────────────────────────────────────────────────────────────╮
│ 3. Strategy Resolution (`resolve_upgrade_strategy`)                          │
│                                                                              │
│ Internal `get_host_target_triple()` detects OS & Architecture                │
│ HEAD https://github.com/r3bl-org/r3bl-binaries/releases/download/                 │
│      v{pinned_ver}-{crate}/{crate}-v{pinned_ver}-{target}.tar.gz (or .zip)   │
│                                                                              │
│ ├─► Returns 200 / 302:                                                       │
│ │   └─► `UpgradeSource::GithubReleaseArtifact`                               │
│ │                                                                            │
│ └─► Returns 404 / 5xx / Network Error / Offline:                             │
│     ├─► `open_source == false` ───────────► Error: `NoViableUpgradeSource`   │
│     ├─► `open_source == true`:                                               │
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
│ │   (If stream fails mid-way & `open_source == true`, falls back to source). │
│ │                                                                            │
│ ├─► [BuildFromSourceRustup] (open_source == true only; uses pinned_ver)      │
│ │   Run `rustup toolchain install nightly --force` in PTY session,           │
│ │   then `cargo +nightly install <crate> --version <pinned_ver>` (OSC 9;4).  │
│ │                                                                            │
│ ├─► [BuildFromSourceCargo] (open_source == true only; uses pinned_ver)       │
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

- [ ] `types.rs`:
    - [ ] `CrateVersionMetadata`: Deserialized entry from `r3bl.com/versions.json`
          (`version`, `open_source`, `released_on`).
    - [ ] `UpgradeConfig`: Per-binary configuration (`crate_name`, `policy`,
          `output_target`).
    - [ ] `UpgradeSource`: `GithubReleaseArtifact`, `BuildFromSourceRustup`,
          `BuildFromSourceCargo`.
    - [ ] `UpgradePolicy`: `InteractivePrompt` (interactive TUI apps), `AutoUpgrade`
          (headless / agent tooling), and `NotifyOnly`.
    - [ ] `UpgradeOutputTarget`: `Stdout` (default CLI/TUI), `Stderr` (MCP server and
          stdout-piped tools like `env-source`), and `Quiet` (structured tracing only).
    - [ ] `UpgradeCheckResult`: `NotChecked`, `Checking`, `UpToDate`, `UpgradeAvailable`,
          `FailedCheck`.
- [ ] `target_triple.rs`:
    - [ ] `get_host_target_triple()`: Automatically resolves platform triple
          (`x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`,
          `x86_64-pc-windows-msvc`, `x86_64-pc-windows-gnu`).
- [ ] `github_release.rs`:
    - [ ] Constructs direct asset download URL on `github.com/r3bl-org/r3bl-binaries`:
          `https://github.com/r3bl-org/r3bl-binaries/releases/download/v<version>-<crate>/<crate>-v<version>-<target>.tar.gz`
          (for Unix) or `.zip` (for Windows), plus `.sha256`.
    - [ ] Performs lightweight HTTP `HEAD` asset accessibility check.
- [ ] `strategy_resolver.rs`:
    - [ ] `resolve_upgrade_strategy(crate_name, pinned_version, open_source)`: Evaluates
          `github.com/r3bl-org/r3bl-binaries` asset availability; if unavailable and
          `open_source == true`, checks local toolchain presence (`rustup` / `cargo`) on
          `$PATH`. If `open_source == false`, returns `Err(NoViableUpgradeSource)`.
- [ ] `version_check.rs`:
    - [ ] `try_get_crate_metadata_from_r3bl_versions_json(crate_name)`: Fetches
          `https://r3bl.com/versions.json` and extracts `CrateVersionMetadata` (`version`,
          `open_source`, `released_on`).
    - [ ] Compares semantic versions strictly (`remote_version > local_version`) so local
          development builds with bumped `Cargo.toml` versions never trigger a downgrade.
    - [ ] `start_background_version_check(config)`: Non-blocking async task running the
          SOT version check.
    - [ ] `start_background_version_check_sync(config)`: Spawns a detached OS thread with
          a lightweight `current_thread` Tokio runtime for synchronous binaries (such as
          `rust-analyzer-mcp-server` which intentionally runs without `#[tokio::main]`).
    - [ ] Updates atomic state `UpgradeCheckResult` and caches the resolved
          `CrateVersionMetadata` (`version`, `open_source`, `released_on`).
- [ ] `source_build.rs`:
    - [ ] Generalized PTY session manager (`PtySessionBuilder` with
          `DefaultPtySessionConfig + PtySessionConfigToken::CaptureOsc`) for `rustup`
          toolchain updates and
          `cargo +nightly install <crate> --version <pinned_version_from_versions_json>`.
    - [ ] Never installs an unpinned version from `crates.io`; always pins `--version` to
          the exact version resolved from `r3bl.com/versions.json`.
    - [ ] `OscPtyEvent::ProgressUpdate` (`OSC 9;4`) compilation escape sequence handler
          and live `Spinner` updater.
- [ ] `run_upgrade.rs`:
    - [ ] `handle_upgrade_at_exit()` / `handle_upgrade_at_exit_sync()`: Coordinates exit
          phase based on policy and output target.
    - [ ] `run_upgrade()` / `run_upgrade_sync()`: Executes the resolved `UpgradeSource`
          (streaming download from `github.com/r3bl-org/r3bl-binaries` + `.sha256`
          verification + archive extraction via `flate2`/`tar` + `self_replace` on current
          binary and updating any sibling binaries in the unpacked archive, OR PTY/OSC
          `rustup` + `cargo +nightly install <crate> --version <pinned_ver>` when
          `open_source == true`).
    - [ ] `run_upgrade_cli_arg_interceptor()`: Helper for standalone `--upgrade` CLI
          invocations.
    - [ ] Listens for cooperative `tokio::signal::ctrl_c()`.
- [ ] `ui_strings.rs`:
    - [ ] Parameterized messages for GitHub download progress (% + MB), fallback
          transition notices, source compilation progress, success, and failure/retry
          notices.
- [ ] `mod.rs`: Private modules and public barrel exports re-exported at `r3bl_tui::*`.

### 2. r3bl-cmdr

**Interactive Command-Line & Developer Productivity Tools (`giti`, `edi`, `env-source`)**:

- [ ] Add `--upgrade` CLI argument to `giti` (`cmdr/src/giti/clap_config.rs`), `edi`
      (`cmdr/src/edi/clap_config.rs`), and `env-source`
      (`cmdr/src/env_source/clap_config.rs`).
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

- [ ] Add `--upgrade` CLI argument to `CLIArg` in
      `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs` (compatible with both
      `cargo-rustdoc-fmt --upgrade` and `cargo rustdoc-fmt --upgrade` via
      `strip_cargo_subcommand_injection()`).
- [ ] Intercept `--upgrade` at startup in `build-infra/src/bin/cargo-rustdoc-fmt.rs`.
- [ ] Spawn background version check at startup in `cargo-rustdoc-fmt.rs`.
- [ ] At exit, invoke `r3bl_tui::handle_upgrade_at_exit()` with
      `UpgradePolicy::AutoUpgrade` (or `NotifyOnly` when `CI=true` or
      `R3BL_NO_AUTO_UPGRADE=1`) and `UpgradeOutputTarget::Stdout`.
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
- [ ] Add `--upgrade` CLI argument to `CLIArg` in
      `rust-analyzer-mcp-server/src/cli_arg.rs` (making `workspace` optional when
      `--upgrade` is passed).
- [ ] Intercept `--upgrade` at startup in `rust-analyzer-mcp-server/src/main.rs`.
- [ ] Spawn background version check at startup in `main.rs`.
- [ ] At shutdown/exit, invoke `r3bl_tui::handle_upgrade_at_exit_sync()` with
      `UpgradePolicy::AutoUpgrade` (or `NotifyOnly` when `CI=true` or
      `R3BL_NO_AUTO_UPGRADE=1`) and `UpgradeOutputTarget::Stderr` to guarantee `stdout`
      JSON-RPC message transport remains 100% uncorrupted.
- [ ] Release archive for `r3bl-rust-analyzer-mcp-server` on
      `github.com/r3bl-org/r3bl-binaries` bundles `rust-analyzer-mcp-server`.

### 5. Bootstrap Scripts & Manifest on `r3bl.com`

**Centralized Installers (`r3bl.com/install.sh`, `r3bl.com/install.ps1`) & Version
Manifest (`r3bl.com/versions.json`)**:

- [ ] Hosted in `../r3bl_website/` so they are served directly from `https://r3bl.com/`
      (with canonical copies maintained in `r3bl-open-core` and synced to
      `../r3bl_website/`):
    - [ ] `https://r3bl.com/versions.json`: Central JSON manifest mapping crate names to
          `{ "version", "open_source", "released_on" }`.
    - [ ] `https://r3bl.com/install.sh`: POSIX shell bootstrap script for Linux & macOS
          (`curl -LsSf https://r3bl.com/install.sh | sh`).
    - [ ] `https://r3bl.com/install.ps1`: PowerShell bootstrap script for Windows
          (`irm https://r3bl.com/install.ps1 | iex`).
- [ ] Supports optional target crate argument (`cmdr` (default), `build-infra`,
      `mcp-server`, `all`, or any crate entry present in `versions.json`).
- [ ] **Bootstrap Execution Flow**:
    - [ ]   1. Fetches `https://r3bl.com/versions.json` to resolve the target crate's
           pinned `version` and `open_source` boolean.
    - [ ]   2. Downloads the pre-compiled archive and `.sha256` checksum from
           `https://github.com/r3bl-org/r3bl-binaries/releases/download/v<ver>-<crate>/<crate>-v<ver>-<target>.tar.gz`
           (or `.zip`), verifies the checksum, and extracts all executables inside the
           archive into `~/.cargo/bin` (or `$env:USERPROFILE\.cargo\bin`).
    - [ ]   3. If the binary download fails and `"open_source": true`, checks for `cargo`
           on `$PATH` and runs `cargo install <crate> --version <ver> --force` (strictly
           using the `<ver>` pinned in `versions.json`). If `"open_source": false`, prints
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
           fleet artifacts), strips symbols, and packages archives with `.sha256`
           checksums into `dist/`.
    - [ ]   5. **Dry-Run Validation**: For open-source crates, runs
           `cargo publish -p <crate> --dry-run --allow-dirty --no-verify`.
    - [ ]   6. **Interactive Confirmation**: Prompts for explicit user approval before
           publishing.
    - [ ]   7. **Atomic Publish, Binary Upload & `versions.json` Update**:
        - [ ] Bumps `Cargo.toml`, regenerates `README.md` via `cargo readme` (except
              `rust-analyzer-mcp-server` which uses a hand-crafted `README.md`), creates
              git commit/tag, pushes to remote, and publishes open-source crates to
              `crates.io`.
        - [ ] Creates the GitHub Release on `github.com/r3bl-org/r3bl-binaries` using:
              `gh release create <tag> --repo r3bl-org/r3bl-binaries --title "<tag>" --notes-file docs/release-notes/<crate>/v<version>.md`
              with attached `dist/*` binary assets (`.tar.gz`, `.zip`, `.sha256`).
        - [ ] Updates `../r3bl_website/versions.json` with the new `version`,
              `open_source`, and `released_on` UTC timestamp (plus syncs `install.sh` and
              `install.ps1` to `../r3bl_website/`).

### 7. Website & Documentation

**Consolidated Showcase & Installation Guides Across All Mediums**:

- [ ] **Crate READMEs & `lib.rs` Modules**: Each binary crate (`cmdr`, `build-infra`,
      `rust-analyzer-mcp-server`, and root `r3bl-open-core`) provides:
    - [ ]   1. Primary: 1-line bootstrap script (`https://r3bl.com/install.sh` /
           `https://r3bl.com/install.ps1`).
    - [ ]   2. Feature docs & `--upgrade` CLI flag instructions.
    - [ ]   3. End-of-file fallback instructions: `cargo install <crate> --force`.
- [ ] **R3BL Website (`r3bl.com` in `../r3bl_website/`)**:
    - [ ] Host `versions.json`, `install.sh`, and `install.ps1` at the root of
          `../r3bl_website/` so they are served at
          `https://r3bl.com/{versions.json,install.sh,install.ps1}`.
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

- [ ] Add `self_replace = "1.5"` and `tar = "0.4"` to `tui/Cargo.toml` (alongside existing
      `flate2` and `sha2`).
- [ ] Create `tui/src/core/script/upgrade/types.rs` defining `CrateVersionMetadata`,
      `UpgradeConfig`, `UpgradeSource`, `UpgradePolicy`, `UpgradeOutputTarget`, and
      `UpgradeCheckResult`.
- [ ] Create `tui/src/core/script/upgrade/target_triple.rs` implementing
      `get_host_target_triple()`.
- [ ] Create `tui/src/core/script/upgrade/github_release.rs` implementing
      `github.com/r3bl-org/r3bl-binaries` asset URL formatting and HTTP `HEAD`
      availability check.
- [ ] Create `tui/src/core/script/upgrade/strategy_resolver.rs` implementing
      `resolve_upgrade_strategy(crate_name, pinned_version, open_source)` and local
      `$PATH` inspection (gating source build fallback on `open_source == true`).
- [ ] Move and generalize PTY/OSC `rustup` & `cargo` build engine from
      `cmdr/src/analytics_client/upgrade_check.rs` into
      `tui/src/core/script/upgrade/source_build.rs` (always passing
      `--version <pinned_version>` from `versions.json` to `cargo +nightly install`).
- [ ] Create `tui/src/core/script/upgrade/version_check.rs` querying
      `https://r3bl.com/versions.json` as the sole SOT, comparing versions strictly via
      `(major, minor, patch)` (`remote_version > local_version`), with both async and
      sync-thread variants and `AtomicU8` state management.
- [ ] Create `tui/src/core/script/upgrade/run_upgrade.rs` implementing
      `handle_upgrade_at_exit()`, `handle_upgrade_at_exit_sync()`,
      `run_upgrade_cli_arg_interceptor()`, `run_upgrade_sync()`, binary streaming download
      from `github.com/r3bl-org/r3bl-binaries` with `.sha256` check, archive extraction,
      `self_replace` on the running executable plus updating sibling binaries in the
      archive, and automatic source fallback execution (pinned to
      `--version <pinned_ver>`) when `open_source == true`.
- [ ] Create `tui/src/core/script/upgrade/ui_strings.rs` providing parameterized
      formatters.
- [ ] Create `tui/src/core/script/upgrade/mod.rs` with private modules and public barrel
      exports.
- [ ] Wire `upgrade` into `tui/src/core/script/mod.rs` and export at `r3bl_tui::*`.
- [ ] Add unit tests in `tui/src/core/script/upgrade/` for `versions.json` parsing, strict
      `remote > local` version comparison, types, strategy resolver (both
      `open_source: true` and `open_source: false`), target triple, and UI strings.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/Cargo.toml`
    - [ ] `tui/src/core/script/upgrade/types.rs`
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

**Bootstrap scripts (`install.sh`, `install.ps1`), `versions.json`, and `release.fish`**:

- [x] Create and configure `https://github.com/r3bl-org/r3bl-binaries` repository with
      metadata, topics, and initial `README.md` on `main`.
- [ ] Create `../r3bl_website/versions.json` initialized with schema v1 (`version`,
      `open_source`, `released_on` for `r3bl-cmdr`, `r3bl-build-infra`, and
      `r3bl-rust-analyzer-mcp-server`).
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
          Windows `.zip` cross-compiled directly on Linux) with `.sha256` checksums.
    - [ ] `cargo publish --dry-run --allow-dirty --no-verify` and explicit user
          confirmation prompt.
    - [ ] Automated version bump, git commit, git tag (`v<version>-<crate>`), git push,
          `cargo publish`, binary upload via
          `gh release create --repo r3bl-org/r3bl-binaries --notes-file docs/release-notes/<crate>/v<version>.md`
          with attached `dist/*` assets, and updating `../r3bl_website/versions.json`.
- [ ] Test `release.fish` validation guardrails (`--dry-run`, missing changelog/release
      notes, invalid version).
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `../r3bl_website/versions.json`
    - [ ] `install.sh`
    - [ ] `../r3bl_website/install.sh`
    - [ ] `install.ps1`
    - [ ] `../r3bl_website/install.ps1`
    - [ ] `release.fish`

### Phase 3: Refactor cmdr

**Integration into `r3bl-cmdr` suite (`giti`, `edi`, `env-source`)**:

- [ ] Add `--upgrade` CLI flag to `giti` (`cmdr/src/giti/clap_config.rs`), `edi`
      (`cmdr/src/edi/clap_config.rs`), and `env-source`
      (`cmdr/src/env_source/clap_config.rs`).
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

- [ ] Add `--upgrade` CLI flag to `CLIArg` in
      `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`.
- [ ] Intercept `--upgrade` at startup in `build-infra/src/bin/cargo-rustdoc-fmt.rs`.
- [ ] Integrate background check and exit upgrade handler (`UpgradePolicy::AutoUpgrade`,
      `UpgradeOutputTarget::Stdout`).
- [ ] Ensure CI / environment guardrail (`CI=true` or `R3BL_NO_AUTO_UPGRADE=1`) falls back
      to `UpgradePolicy::NotifyOnly`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `build-infra/src/bin/cargo-rustdoc-fmt.rs`
    - [ ] `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`
    - [ ] `build-infra/Cargo.toml`

### Phase 5: Integrate mcp-server

**Integration into `r3bl-rust-analyzer-mcp-server`**:

- [ ] Add `--upgrade` CLI flag to `CLIArg` in `rust-analyzer-mcp-server/src/cli_arg.rs`.
- [ ] Intercept `--upgrade` at startup in `rust-analyzer-mcp-server/src/main.rs` via
      `run_upgrade_sync()`.
- [ ] Integrate background check (`start_background_version_check_sync`) and shutdown
      upgrade handler (`handle_upgrade_at_exit_sync` with `UpgradePolicy::AutoUpgrade`,
      `UpgradeOutputTarget::Stderr`).
- [ ] Ensure CI / environment guardrail (`CI=true` or `R3BL_NO_AUTO_UPGRADE=1`) falls back
      to `UpgradePolicy::NotifyOnly`.
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
    - [ ] Document `--upgrade` standalone CLI argument for `giti`, `edi`, and
          `env-source`.
    - [ ] Add end-of-file fallback instructions: `cargo install r3bl-cmdr --force`.
- [ ] Update `build-infra/README.md` and `build-infra/src/lib.rs`:
    - [ ] Add bootstrap install commands (`https://r3bl.com/install.sh` /
          `https://r3bl.com/install.ps1` with `build-infra`).
    - [ ] Document `cargo-rustdoc-fmt --upgrade` / `cargo rustdoc-fmt --upgrade`.
    - [ ] Add end-of-file fallback instructions: `cargo install r3bl-build-infra --force`.
- [ ] Update `rust-analyzer-mcp-server/README.md` and
      `rust-analyzer-mcp-server/src/lib.rs`:
    - [ ] Add bootstrap install commands (`https://r3bl.com/install.sh` /
          `https://r3bl.com/install.ps1` with `mcp-server`).
    - [ ] Document `rust-analyzer-mcp-server --upgrade` standalone CLI argument.
    - [ ] Add end-of-file fallback instructions:
          `cargo install r3bl-rust-analyzer-mcp-server --force`.
- [ ] Update root `README.md`:
    - [ ] Consolidate all binary tools with quick 1-line bootstrap install commands from
          `r3bl.com`.
    - [ ] Document `--upgrade` usage across tools.
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
      `../r3bl_website/versions.json`.
- [ ] Update `docs/release-guide.md` to reflect binary artifact generation, upload to
      `github.com/r3bl-org/r3bl-binaries`, and `versions.json` updates via `release.fish`.
- [x] Update `../r3bl-base/task/prd_monetization_approach.md` (Section 7) to reference
      this task file as the SSOT for the shared upgrade engine, `r3bl.com/versions.json`,
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
visibility (`open_source: true` vs. `false`), and toolchain environments.

### Test Scenario Matrix

| Scenario                                     | Crate Visibility     | Network State                                          | Local Toolchain                    | Target Resolved                  | Expected Outcome & Verification                                                                                                                                                           |
| :------------------------------------------- | :------------------- | :----------------------------------------------------- | :--------------------------------- | :------------------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **1. Fast Binary Upgrade**                   | Open or Closed       | `r3bl.com/versions.json` + `r3bl-org/r3bl-binaries` OK | Any / None needed                  | `GithubReleaseArtifact`          | Streams `.tar.gz` (Unix) or `.zip` (Windows) from `github.com/r3bl-org/r3bl-binaries`, verifies `.sha256`, swaps binary via `self_replace` (and updates sibling binaries), exits 0.       |
| **2. Binaries Repo Down + Full Rustup**      | `open_source: true`  | `r3bl-org/r3bl-binaries` 404 / Offline                 | `rustup` + `cargo` on `$PATH`      | `BuildFromSourceRustup`          | Displays fallback message, runs `rustup toolchain install nightly --force` with PTY, runs `cargo +nightly install <crate> --version <pinned_ver>` (pinned from `versions.json`), exits 0. |
| **3. Binaries Repo Down + Standalone Cargo** | `open_source: true`  | `r3bl-org/r3bl-binaries` 404 / Offline                 | `cargo` on `$PATH` (no `rustup`)   | `BuildFromSourceCargo`           | Displays fallback message, runs `cargo +nightly install <crate> --version <pinned_ver>` (pinned from `versions.json`), does not error on missing `rustup`, exits 0.                       |
| **4. Binaries Repo Down + Zero Toolchain**   | `open_source: true`  | `r3bl-org/r3bl-binaries` 404 / Offline                 | No `rustup`, no `cargo` on `$PATH` | `Err(NoViableUpgradeSource)`     | Displays clean error: `"Upgrade failed: GitHub release unreachable and no local Rust build toolchain found. Please retry later: <bin> --upgrade"`, exits 1 cleanly without panic.         |
| **5. Binaries Repo Down (Closed Source)**    | `open_source: false` | `r3bl-org/r3bl-binaries` 404 / Offline                 | Any (ignored)                      | `Err(NoViableUpgradeSource)`     | Does NOT attempt `crates.io` fallback. Displays clean error: `"Upgrade failed: Binary release server unreachable. Please retry later: <bin> --upgrade"`, exits 1.                         |
| **6. Local Dev Version Ahead of SOT**        | Open or Closed       | `r3bl.com/versions.json` OK                            | Any                                | `UpgradeCheckResult::UpToDate`   | Local `CARGO_PKG_VERSION` >= `versions.json` `version`; strict `remote > local` comparison evaluates false so local dev binary is never downgraded.                                       |
| **7. Mid-Stream Network Drop**               | `open_source: true`  | Download aborts mid-stream                             | `rustup` or `cargo` on `$PATH`     | Dynamic fallback to Source Build | Download stream error caught, temp files cleaned up, seamlessly transitions to `BuildFromSourceRustup` or `BuildFromSourceCargo` with `--version <pinned_ver>` from `versions.json`.      |
| **8. User Cancellation (Ctrl+C)**            | Open or Closed       | Interrupted during download or compilation             | Any                                | Interrupted                      | Cooperative `tokio::signal::ctrl_c()` caught, PTY session / HTTP stream terminated, temp files purged, original binary remains 100% intact.                                               |
| **9. First-Time Bootstrap Install**          | Open or Closed       | `r3bl.com` + `r3bl-org/r3bl-binaries` 200 OK           | None required                      | Script installer                 | `r3bl.com/install.sh` (Linux/macOS) or `r3bl.com/install.ps1` (Windows) reads `versions.json`, extracts archive to `~/.cargo/bin` or `$env:USERPROFILE\.cargo\bin`, sets perms.           |
| **10. First-Time Install Outage (Open)**     | `open_source: true`  | `r3bl-org/r3bl-binaries` Down / Offline                | Rust toolchain present             | Manual Cargo fallback            | `install.{sh,ps1}` runs `cargo install <crate> --version <pinned_ver> --force` (or manual `cargo install <crate> --force` if `r3bl.com` is also offline).                                 |
| **11. MCP Server & `env-source` Stdout**     | Open or Closed       | Any upgrade or normal execution                        | Any                                | `UpgradeOutputTarget::Stderr`    | All progress, spinners, and logs route exclusively to `stderr`. `stdout` remains 100% pure JSON-RPC (`mcp-server`) or shell export statements (`env-source`).                             |

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
    - [ ] Verify `CI=true` and `R3BL_NO_AUTO_UPGRADE=1` suppress automated upgrades and
          fall back to `NotifyOnly`.
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
- [ ] Verify `.zip` archive extraction and `.exe` path resolution in
      `$env:USERPROFILE\.cargo\bin`.
- [ ] Verify `self_replace` open file lock handling (renames active `.exe` to `.old`,
      writes new `.exe`, marks `.old` for deletion).
- [ ] Test fallback source compilation with Windows ConPTY (`CreatePseudoConsole`).
- [ ] Test Ctrl+C cancellation during download and compilation under Windows console.
