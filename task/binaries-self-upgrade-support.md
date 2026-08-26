Task: Pre-Compiled Binary Self-Upgrade & Bootstrap Support Across R3BL Binaries

<!-- BEGIN mktoc -->

- [Overview](#overview)
    - [Current State: Crates.io & Rustup](#current-state-cratesio--rustup)
    - [Archived Rustup Code](#archived-rustup-code)
    - [New Model: GitHub Releases](#new-model-github-releases)
- [Lifecycle Flowchart](#lifecycle-flowchart)
- [Architecture](#architecture)
    - [1. `r3bl_tui`](#1-r3bl_tui)
    - [2. `r3bl-cmdr`](#2-r3bl-cmdr)
    - [3. `r3bl-build-infra`](#3-r3bl-build-infra)
    - [4. `r3bl-rust-analyzer-mcp-server`](#4-r3bl-rust-analyzer-mcp-server)
    - [5. Bootstrap Scripts](#5-bootstrap-scripts)
    - [6. `release.fish` Script](#6-releasefish-script)
- [Implementation Plan](#implementation-plan)
    - [Phase 1: Shared Upgrade Module (`r3bl_tui`)](#phase-1-shared-upgrade-module-r3bl_tui)
    - [Phase 2: Bootstrap & `release.fish`](#phase-2-bootstrap--releasefish)
    - [Phase 3: Refactor `cmdr`](#phase-3-refactor-cmdr)
    - [Phase 4: Integrate `build-infra`](#phase-4-integrate-build-infra)
    - [Phase 5: Integrate `mcp-server`](#phase-5-integrate-mcp-server)
    - [Phase 6: Docs & Website](#phase-6-docs--website)
    - [Phase 7: Verification & Testing](#phase-7-verification--testing)
      <!-- END mktoc -->

## Overview

Transition all R3BL binary tools (`r3bl-cmdr`, `r3bl-build-infra`, and
`r3bl-rust-analyzer-mcp-server`) from source-based `crates.io` compilation via
`rustup`/`cargo` to **pre-compiled binary distribution via GitHub Releases**.

### Current State: Crates.io & Rustup

Currently, binary tools like `r3bl-cmdr` rely on source compilation via `crates.io`:

1. The binary performs a non-blocking background check against the `crates.io` API.
2. If an update is available, upon exit it prompts the user and spawns a Pseudoterminal
   (PTY) child process using `r3bl_tui::core::pty::PtySessionBuilder`.
3. The PTY session executes `rustup toolchain install nightly --force` followed by
   `cargo +nightly install r3bl-cmdr`, parsing raw stdout lines from `rustup` and terminal
   OSC 9;4 compilation progress escape sequences from `cargo` to animate a live TUI
   `Spinner`.

While clever and visually responsive, this approach has significant real-world drawbacks:

- End-user machines are required to have `rustup`, `cargo`, Rust nightly, GCC, and system
  libraries installed.
- Every upgrade requires 1-2 minutes of CPU-heavy compilation building 200+ dependencies.
- Headless tools (like `cargo-rustdoc-fmt`) and AI agents calling
  `rust-analyzer-mcp-server` cannot easily or reliably run multi-minute source
  compilations.

### Archived Rustup Code

The complete, generalized source code for the PTY session manager, OSC 9;4 progress
parser, and `rustup` toolchain updater has been safely preserved and archived for
reference:

- [GitHub Repo](https://github.com/nazmulidris/rust-scratch/tree/main/rustup-upgrade)
- [Local Repo](file:///home/nazmul/github/rust-scratch/rustup-upgrade/)

### New Model: GitHub Releases

We are pivoting to direct pre-compiled binary distribution:

1. **Instant Upgrades**: 1-2 second HTTP download and atomic binary swap via the
   [`self_replace`](https://crates.io/crates/self_replace) crate from crates.io (instead
   of 1-2 minutes compiling 200+ crate dependencies on user machines).
2. **Zero Toolchain Dependencies**: Users and AI coding agents do not need Rust nightly,
   `rustup`, `cargo`, GCC, or system dev headers installed.
3. **Deterministic Multi-Platform Releases**: Pre-compiled, optimized release binaries for
   Linux, macOS, and Windows eliminate local toolchain or linker incompatibilities.
4. **Universal Bootstrap Scripts**: Single-line first-time installs via `install.sh`
   (Linux/macOS) and `install.ps1` (Windows).
5. **One-Time Transition Bridge**: Existing `v0.0.26` users compile `v0.0.27` once from
   `crates.io`, and from `v0.0.27+` onward receive all subsequent upgrades instantly via
   GitHub Releases.
6. **Simpler Client Code**: Replaces complex PTY child process spawning, OSC 9;4
   compilation escape sequence parsing, and `rustup` CLI parsing in `r3bl_tui` with clean
   HTTP streaming and cross-platform atomic binary replacement.

---

## Lifecycle Flowchart

```text
╭──────────────────────────────────────────────────────────────────────────────╮
│ 1. Startup & Background Version Check (Non-blocking)                         │
│                                                                              │
│ User or caller launches binary (cmdr, cargo-rustdoc-fmt, MCP server)         │
│    │                                                                         │
│    ├─► [Foreground] Executes primary job immediately                         │
│    │                                                                         │
│    └─► [Background Tokio Task] start_background_version_check(crate_name)    │
│             │                                                                │
│             ▼                                                                │
│          Queries GitHub Releases API for newest tag matching v*-<crate>      │
│             │                                                                │
│             ├─► Compares remote version with local CARGO_PKG_VERSION         │
│             │                                                                │
│             └─► Stores state in AtomicU8 (UpgradeCheckResult)                │
╰──────────────────────────────────────────────────────────────────────────────╯
                                     │
                                     ▼
╭──────────────────────────────────────────────────────────────────────────────╮
│ 2. Exit Phase (handle_upgrade_at_exit)                                       │
│                                                                              │
│ Primary job finishes in foreground                                           │
│    │                                                                         │
│    ├─► Check UpgradePolicy & environment:                                    │
│    │       - If CI=true or R3BL_NO_AUTO_UPGRADE=1: fallback to NotifyOnly    │
│    │                                                                         │
│    ├─► Policy: InteractivePrompt (r3bl-cmdr)                                 │
│    │     - If terminal is interactive AND UpgradeAvailable:                  │
│    │          Prompts user via choose() ("Yes, upgrade now" / "No, thanks")  │
│    │          If "Yes": proceed to step 3                                    │
│    │     - If terminal is non-interactive: exit immediately                  │
│    │                                                                         │
│    ├─► Policy: AutoUpgrade (cargo-rustdoc-fmt, rust-analyzer-mcp-server)     │
│    │       - If UpgradeAvailable: proceed directly to step 3                 │
│    │                                                                         │
│    └─► Policy: NotifyOnly                                                    │
│            - Prints/logs notification without executing upgrade              │
╰──────────────────────────────────────────────────────────────────────────────╯
                                     │
                                     ▼
╭──────────────────────────────────────────────────────────────────────────────╮
│ 3. Download & Progress Tracking (run_upgrade_from_github)                    │
│                                                                             │
│ - Resolve target asset URL:                                                  │
│   Linux / macOS: `<crate>-v<version>-<target>.tar.gz`                        │
│   Windows:       `<crate>-v<version>-<target>.zip`                           │
│ - Live Spinner Progress Stages:                                              │
│   a. "Connecting to GitHub Releases..."                                      │
│   b. "Downloading <crate> v<version>... <pct>% (<dl> MB / <total> MB)"       │
│      Streams body in 8-64 KB chunks, computes progress against Content-      │
│      Length, and throttles UI updates to 50ms / percentage increments.       │
│   c. "Extracting binaries (<bin_list>)..."                                   │
│   d. "Installing binaries via self_replace..."                               │
│   e. "Upgrade complete."                                                     │
│ - Handles cooperative Ctrl+C cancellation via tokio::signal::ctrl_c().       │
╰──────────────────────────────────────────────────────────────────────────────╯
                                     │
                                     ▼
╭──────────────────────────────────────────────────────────────────────────────╮
│ 4. In-Place Atomic Binary Replacement & Stream Routing                       │
│                                                                             │
│ - Unpack binary to temp file and call `self_replace::self_replace()`.        │
│ - Works seamlessly across Linux, macOS, and Windows (handles open .exe lock) │
│ - Output Stream Routing:                                                     │
│   - Stdout: Live spinner and formatted banners on stdout (CLI / TUI).        │
│   - Stderr: Live spinner and messages on stderr (MCP server; preserves       │
│     clean stdout for JSON-RPC message transport).                            │
│   - Quiet: Suppress terminal output; log via structured tracing.             │
╰─────────────────────────────────────────────────────────────────────────────╯
```

---

## Architecture

### 1. `r3bl_tui`

- `types.rs`:
    - `UpgradePolicy`: `InteractivePrompt` (for interactive TUI apps), `AutoUpgrade` (for
      headless / agent tooling), and `NotifyOnly`.
    - `UpgradeOutputTarget`: `Stdout` (default CLI/TUI), `Stderr` (for MCP servers to
      preserve stdout JSON-RPC transport), and `Quiet` (structured tracing only).
    - `UpgradeCheckResult`: Atomic status representing `NotChecked`, `Checking`,
      `UpToDate`, `UpgradeAvailable`, and `FailedCheck`.
- `github_release.rs`:
    - Resolves target platform triple (`x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`,
      `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`, `x86_64-pc-windows-gnu`).
    - Queries `https://api.github.com/repos/r3bl-org/r3bl-open-core/releases` for releases
      matching tag pattern `v*-<crate_short_name>`.
    - Extracts asset download URL for the current platform.
- `version_check.rs`:
    - Spawns background async task querying GitHub Releases API without blocking startup.
    - Updates atomic state `UpgradeCheckResult`.
- `run_upgrade.rs`:
    - `handle_upgrade_at_exit()`: Coordinates exit phase based on policy and output
      target.
    - `run_upgrade_from_github()`: Streams asset archive download in chunks via `reqwest`,
      accumulates received bytes, computes percentage and MB metrics against
      `Content-Length`, throttles live updates to the `Spinner` (50ms cadence or %
      changes), unpacks archives, and calls `self_replace::self_replace()` to swap
      binaries in-place.
    - Cooperative cancellation: listens for `tokio::signal::ctrl_c()`, cleans up temp
      files, and exits cleanly.
- `ui_strings.rs`:
    - Parameterized messages for upgrade availability, connecting status, download
      percentage / MB metrics, extraction, installation, success, failure, and goodbye
      greetings.
- `mod.rs`: Barrel exports re-exported at `r3bl_tui::*`.

### 2. `r3bl-cmdr`

- Replace existing PTY/OSC/rustup/cargo install code in `upgrade_check.rs` and `ui_str.rs`
  with calls to the shared `r3bl_tui` GitHub Releases module.
- Delegate background version checking and upgrade execution to `r3bl_tui::*` using
  `UpgradePolicy::InteractivePrompt` and `UpgradeOutputTarget::Stdout`.
- Preserve cmdr-specific interactive prompts, exit context handling, emojis, and lolcat
  greetings.
- Release bundles all 3 binaries: `giti`, `edi`, and `rc`.

### 3. `r3bl-build-infra`

- Spawn background version check at startup in `cargo-rustdoc-fmt.rs`.
- At exit, invoke `r3bl_tui::handle_upgrade_at_exit()` with `UpgradePolicy::AutoUpgrade`
  (or `NotifyOnly` when `CI=true` or `R3BL_NO_AUTO_UPGRADE=1`) and
  `UpgradeOutputTarget::Stdout`.
- Release bundles `cargo-rustdoc-fmt`.

### 4. `r3bl-rust-analyzer-mcp-server`

- Spawn background version check at startup in `main.rs`.
- At shutdown/exit, invoke `r3bl_tui::handle_upgrade_at_exit()` with
  `UpgradePolicy::AutoUpgrade` (or `NotifyOnly` when `CI=true` or
  `R3BL_NO_AUTO_UPGRADE=1`) and `UpgradeOutputTarget::Stderr` to ensure stdout JSON-RPC
  message transport remains completely clean and unbroken.
- Release bundles `rust-analyzer-mcp-server`.

### 5. Bootstrap Scripts

- `install.sh`: POSIX shell bootstrap script for Linux & macOS
  (`curl -sSf .../install.sh | sh`).
- `install.ps1`: PowerShell bootstrap script for Windows (`irm .../install.ps1 | iex`).
- Supports optional target crate argument (`cmdr` (default), `build-infra`, `mcp-server`,
  or `all`).

### 6. `release.fish` Script

- `release.fish`: Automated, end-to-end release manager supporting both binary and library
  crates with built-in guardrails:
    1. **Version Validation**: Fails fast if target version is not strictly greater than
       the current version in `<crate>/Cargo.toml`.
    2. **CHANGELOG.md Verification**: Fails fast if `CHANGELOG.md` does not contain
       release notes for `### v<version>`.
    3. **Automated GitHub Release Notes Extraction**: Parses the markdown changelog block
       for `v<version>` from `CHANGELOG.md` and formats it into the GitHub Release body
       along with 1-line bootstrap install commands (`install.sh` / `install.ps1`).
    4. **Quality Checks**: Executes `./check.fish --full`.
    5. **Multi-Platform Binary Packaging (Binary crates only)**: Builds Linux native
       binaries, cross-compiles Windows `.exe` binaries directly on Linux, strips symbols,
       and packages `.tar.gz` and `.zip` archives with `.sha256` checksums into `dist/`.
    6. **Dry-Run Validation**: Runs
       `cargo publish -p <crate> --dry-run --allow-dirty --no-verify`.
    7. **Interactive User Confirmation**: Shows summary (crate, version bump, changelog
       excerpt, assets) and prompts for explicit user approval before publishing.
    8. **Atomic Publish & Release**: Bumps `Cargo.toml`, creates git commit and tag
       (`v<version>-<crate>`), pushes to remote, publishes to `crates.io`, and creates the
       GitHub Release with attached `dist/*` assets.

---

## Implementation Plan

### Phase 1: Shared Upgrade Module (`r3bl_tui`)

- [ ] Add `self_replace = "1.5"` dependency to `tui/Cargo.toml`.
- [ ] Create `tui/src/core/script/upgrade/types.rs` defining `UpgradePolicy`,
      `UpgradeOutputTarget`, and `UpgradeCheckResult`.
- [ ] Create `tui/src/core/script/upgrade/github_release.rs` implementing target triple
      detection, GitHub Releases API querying for tag pattern `v*-<crate>`, and asset URL
      extraction.
- [ ] Create `tui/src/core/script/upgrade/version_check.rs` implementing
      `start_background_version_check()` and `get_bin_name_from_current_exe()`.
- [ ] Create `tui/src/core/script/upgrade/run_upgrade.rs` implementing
      `handle_upgrade_at_exit()`, `run_upgrade_from_github()` (streaming chunks via
      `reqwest`, byte accumulation, `Content-Length` comparison, throttled `Spinner`
      progress updates with percentage and MB metrics, cooperative `Ctrl+C` cancellation),
      archive extraction, and atomic binary replacement via `self_replace`.
- [ ] Create `tui/src/core/script/upgrade/ui_strings.rs` providing parameterized status,
      connecting, download progress (percentage + MB), extraction, installation, success,
      and failure formatters.
- [ ] Create `tui/src/core/script/upgrade/mod.rs` with private modules and public barrel
      exports.
- [ ] Wire `upgrade` into `tui/src/core/script/mod.rs` and export at `r3bl_tui::*`.
- [ ] Add unit tests in `tui/src/core/script/upgrade/` for `types`, `github_release`,
      `version_check`, and `ui_strings`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/Cargo.toml`
    - [ ] `tui/src/core/script/upgrade/types.rs`
    - [ ] `tui/src/core/script/upgrade/github_release.rs`
    - [ ] `tui/src/core/script/upgrade/version_check.rs`
    - [ ] `tui/src/core/script/upgrade/run_upgrade.rs`
    - [ ] `tui/src/core/script/upgrade/ui_strings.rs`
    - [ ] `tui/src/core/script/upgrade/mod.rs`
    - [ ] `tui/src/core/script/mod.rs`

### Phase 2: Bootstrap & `release.fish`

- [ ] Create `install.sh` at repo root for Linux and macOS bootstrap installation.
- [ ] Create `install.ps1` at repo root for Windows PowerShell bootstrap installation.
- [ ] Create `release.fish` at repo root implementing: - `<crate>` and `<version>`
      argument parsing with interactive fallback. - Version validation against
      `<crate>/Cargo.toml`. - `CHANGELOG.md` entry verification. - Automated extraction of
      `CHANGELOG.md` section for GitHub Release notes. - Binary multi-platform compilation
      & packaging into `dist/` (Linux `.tar.gz` and Windows `.zip` cross-compiled directly
      on Linux) with `.sha256` checksums. - `cargo publish --dry-run` and explicit user
      confirmation prompt. - Automated version bump, git commit, git tag
      (`v<version>-<crate>`), git push, `cargo publish`, and `gh release create` with
      attached assets.
- [ ] Test `release.fish` validation guardrails (`--dry-run`, missing changelog, invalid
      version).
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `install.sh`
    - [ ] `install.ps1`
    - [ ] `release.fish`

### Phase 3: Refactor `cmdr`

- [ ] Refactor `cmdr/src/analytics_client/upgrade_check.rs` to remove existing
      PTY/OSC/rustup/cargo functions and delegate to `r3bl_tui::*` using
      `UpgradePolicy::InteractivePrompt` and `UpgradeOutputTarget::Stdout`.
- [ ] Refactor `cmdr/src/analytics_client/ui_str.rs` to remove existing `upgrade_install`
      functions and delegate to `r3bl_tui::*`.
- [ ] Run `cmdr` unit tests and verify `giti`, `edi`, and `rc` binaries compile and
      function.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `cmdr/src/analytics_client/upgrade_check.rs`
    - [ ] `cmdr/src/analytics_client/ui_str.rs`
    - [ ] `cmdr/Cargo.toml`

### Phase 4: Integrate `build-infra`

- [ ] Integrate background check and exit upgrade handler (`UpgradePolicy::AutoUpgrade`,
      `UpgradeOutputTarget::Stdout`) into `build-infra/src/bin/cargo-rustdoc-fmt.rs`.
- [ ] Ensure CI / environment guardrail (`CI=true` or `R3BL_NO_AUTO_UPGRADE=1`) falls back
      to `UpgradePolicy::NotifyOnly`.
- [ ] Update `build-infra/Cargo.toml` dependencies if needed.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `build-infra/src/bin/cargo-rustdoc-fmt.rs`
    - [ ] `build-infra/Cargo.toml`

### Phase 5: Integrate `mcp-server`

- [ ] Integrate background check and shutdown upgrade handler
      (`UpgradePolicy::AutoUpgrade`, `UpgradeOutputTarget::Stderr`) into
      `rust-analyzer-mcp-server/src/main.rs`.
- [ ] Ensure output target is strictly `UpgradeOutputTarget::Stderr` / structured tracing
      so that stdout JSON-RPC message transport remains completely clean and unbroken.
- [ ] Ensure CI / environment guardrail (`CI=true` or `R3BL_NO_AUTO_UPGRADE=1`) falls back
      to `UpgradePolicy::NotifyOnly`.
- [ ] Update `rust-analyzer-mcp-server/Cargo.toml` dependencies if needed.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `rust-analyzer-mcp-server/src/main.rs`
    - [ ] `rust-analyzer-mcp-server/Cargo.toml`

### Phase 6: Docs & Website

- [ ] Update `cmdr/README.md` and `cmdr/src/lib.rs` with pre-compiled bootstrap install
      instructions (`install.sh` and `install.ps1`).
- [ ] Update `build-infra/README.md` and `build-infra/src/lib.rs` with pre-compiled
      bootstrap install instructions.
- [ ] Update `rust-analyzer-mcp-server/README.md` and
      `rust-analyzer-mcp-server/src/lib.rs` with pre-compiled bootstrap install
      instructions.
- [ ] Update root `README.md` with the new single-line bootstrap install commands
      (`install.sh` and `install.ps1`).
- [ ] Update `r3bl.com` website in `../r3bl_website/index.html` with new install
      instructions for all binary tools (`r3bl-cmdr`, `r3bl-build-infra`,
      `r3bl-rust-analyzer-mcp-server`).
- [ ] Update `.agents/skills/release-crate/SKILL.md` to incorporate
      `./release.fish <crate> <version>`.
- [ ] Update `docs/release-guide.md` to reflect binary artifact generation and release
      steps via `release.fish`.
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
    - [ ] `.agents/skills/release-crate/SKILL.md`
    - [ ] `docs/release-guide.md`

### Phase 7: Verification & Testing

- [ ] Audit workspace to ensure no orphaned PTY/rustup upgrade code, unused imports, or
      dead functions remain.
- [ ] Run `./check.fish --check` across the entire workspace.
- [ ] Run `./check.fish --build` across the entire workspace.
- [ ] Run `./check.fish --clippy` across the entire workspace.
- [ ] Run `./check.fish --test` across the entire workspace.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `task/binaries-self-upgrade-support.md`
