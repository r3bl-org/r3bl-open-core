# Task: Binary Self-Upgrade & Bootstrap

<!-- prettier-ignore-start -->
<!-- BEGIN mktoc -->

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
    - [5. Bootstrap Scripts](#5-bootstrap-scripts)
    - [6. Release Script](#6-release-script)
    - [7. Website & Documentation](#7-website--documentation)
- [Implementation Plan](#implementation-plan)
    - [Phase 1: Shared Upgrade Module](#phase-1-shared-upgrade-module)
    - [Phase 2: Bootstrap & Release Script](#phase-2-bootstrap--release-script)
    - [Phase 3: Refactor cmdr](#phase-3-refactor-cmdr)
    - [Phase 4: Integrate build-infra](#phase-4-integrate-build-infra)
    - [Phase 5: Integrate mcp-server](#phase-5-integrate-mcp-server)
    - [Phase 6: Docs & Website](#phase-6-docs--website)
    - [Phase 7: Verification & Testing](#phase-7-verification--testing)
- [Cross-Platform Test Matrix](#cross-platform-test-matrix)
    - [Test Scenario Matrix](#test-scenario-matrix)
    - [OS-Specific Verification Checklist](#os-specific-verification-checklist)

<!-- END mktoc -->
<!-- prettier-ignore-end -->

## Overview

Transition all R3BL binary tools (`r3bl-cmdr`, `r3bl-build-infra`, and
`r3bl-rust-analyzer-mcp-server`) to a hybrid upgrade architecture.

1. **Primary (Fast Binary Download)**: 1-2 second HTTP download and atomic in-place binary
   swap via `self_replace` from pre-compiled GitHub Releases (instead of 1-2 minutes
   compiling 200+ crate dependencies on user machines).
2. **Fallback (Resilient Source Compilation)**: If GitHub is down, blocked, or release
   assets are unavailable, automatically and resiliently fall back to building from source
   using the shared PTY session manager (`rustup toolchain install nightly` +
   `cargo +nightly install`) with live OSC 9;4 compilation progress tracking.

### Dual-Engine Strategy

**Fast Binary Downloads + Resilient Source Compilation**: This hybrid architecture
provides the ideal balance between instant binary upgrades and robust fallback
compilation:

- **Instant Upgrades by Default**: 99% of upgrades take 1-2 seconds with zero toolchain
  dependencies on user machines.
- **Zero Toolchain Dependencies for End Users**: Users and AI coding agents using
  `install.sh` or `install.ps1` do not need Rust nightly, `rustup`, `cargo`, GCC, or
  system dev headers.
- **Deterministic Multi-Platform Releases**: Pre-compiled release binaries for Linux,
  macOS, and Windows eliminate local toolchain or linker incompatibilities.
- **Resilient Fallback for Developers**: If GitHub goes down or network conditions block
  GitHub CDN, the system checks the local environment for `rustup` or `cargo` and
  automatically initiates a source build.
- **Universal Bootstrap Scripts**: Single-line first-time installs via `install.sh`
  (Linux/macOS) and `install.ps1` (Windows).

### Source of Truth

**`crates.io` API SOT & Rate-Limit Prevention**: Background version checks query
`crates.io` as the authoritative Source of Truth (SOT) to prevent GitHub API
rate-limiting:

- **`crates.io` API as Version SOT**: Background version checks query
  `https://crates.io/api/v1/crates/{crate_name}` for `max_version`. This avoids GitHub
  unauthenticated REST API rate limits (60 requests per hour per IP).
- **Direct CDN Asset Checking**: When an upgrade is available, a lightweight HTTP `HEAD`
  request directly checks the GitHub Release asset download URL
  (`https://github.com/.../download/v{ver}-{crate}/{crate}-v{ver}-{target}.tar.gz` for
  Linux/macOS, or `.zip` for Windows). This hits GitHub/AWS CloudFront CDN directly with
  zero REST API overhead.

### CLI Flag

**Dedicated `--upgrade` Flag Across All Binaries**: All binaries provide a top-level
`--upgrade` CLI argument to run standalone self-upgrades without running the main
application:

- Supports `giti --upgrade`, `edi --upgrade`, `rc --upgrade`,
  `cargo-rustdoc-fmt --upgrade`, and `rust-analyzer-mcp-server --upgrade`.
- When passed, the binary bypasses normal application startup, executes the upgrade flow
  synchronously, and exits.

---

## Lifecycle Flowchart

```text
╭──────────────────────────────────────────────────────────────────────────────╮
│ Invocation: Normal Run OR Dedicated Standalone `<binary> --upgrade`          │
╰──────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭──────────────────────────────────────────────────────────────────────────────╮
│ 1. Version Check & SOT Evaluation (crates.io API)                            │
│                                                                              │
│ GET https://crates.io/api/v1/crates/{crate_name}                            │
│ ├─► Compares remote `max_version` with local `CARGO_PKG_VERSION`             │
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
│     ├─► InteractivePrompt (cmdr): Prompt user ("Yes, upgrade" / "No")        │
│     ├─► AutoUpgrade (cargo-rustdoc-fmt, MCP server): Run upgrade             │
│     └─► NotifyOnly (CI=true / R3BL_NO_AUTO_UPGRADE=1): Print notification    │
╰──────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭──────────────────────────────────────────────────────────────────────────────╮
│ 3. Strategy Resolution (`resolve_upgrade_strategy`)                          │
│                                                                              │
│ Internal `get_host_target_triple()` detects OS & Architecture                │
│ HEAD https://github.com/.../download/v{ver}-{crate}/{crate}-v{ver}-{target} │
│                                                                              │
│ ├─► Returns 200 / 302:                                                       │
│ │   └─► `UpgradeSource::GithubReleaseArtifact`                               │
│ │                                                                            │
│ └─► Returns 404 / 5xx / Network Error / Offline:                             │
│     ├─► `rustup` on $PATH ──► `UpgradeSource::BuildFromSourceRustup`         │
│     ├─► `cargo`  on $PATH ──► `UpgradeSource::BuildFromSourceCargo`          │
│     └─► Neither on $PATH  ──► Error: `NoViableUpgradeSource`                 │
╰──────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭──────────────────────────────────────────────────────────────────────────────╮
│ 4. Execution Engine                                                          │
│                                                                              │
│ ├─► [GithubReleaseArtifact]                                                  │
│ │   Stream download with % and MB spinner, unpack archive, atomic swap via   │
│ │   `self_replace`. (If stream fails mid-way, falls back to source build).   │
│ │                                                                            │
│ ├─► [BuildFromSourceRustup]                                                  │
│ │   Run `rustup toolchain install nightly --force` in PTY session,           │
│ │   followed by `cargo +nightly install <crate>` with OSC 9;4 progress.      │
│ │                                                                            │
│ ├─► [BuildFromSourceCargo]                                                   │
│ │   Run `cargo +nightly install <crate>` with OSC 9;4 progress in PTY.       │
│ │                                                                            │
│ └─► [All Attempts Fail]                                                      │
│     Print clean error with retry instructions: `<binary> --upgrade`          │
╰──────────────────────────────────────────────────────────────────────────────╯
```

---

## Architecture

### 1. r3bl_tui

**Shared Upgrade Engine**: The unified upgrade module is housed inside
`r3bl_tui::core::script::upgrade`:

- `types.rs`:
    - `UpgradeSource`: `GithubReleaseArtifact`, `BuildFromSourceRustup`,
      `BuildFromSourceCargo`.
    - `UpgradePolicy`: `InteractivePrompt` (interactive TUI apps), `AutoUpgrade` (headless
      / agent tooling), and `NotifyOnly`.
    - `UpgradeOutputTarget`: `Stdout` (default CLI/TUI), `Stderr` (MCP server to preserve
      stdout JSON-RPC transport), and `Quiet` (structured tracing only).
    - `UpgradeCheckResult`: `NotChecked`, `Checking`, `UpToDate`, `UpgradeAvailable`,
      `FailedCheck`.
- `target_triple.rs`:
    - `get_host_target_triple()`: Automatically resolves platform triple
      (`x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`,
      `x86_64-pc-windows-msvc`, `x86_64-pc-windows-gnu`).
- `github_release.rs`:
    - Constructs direct asset download URL (`<crate>-v<version>-<target>.tar.gz` for Unix,
      `.zip` for Windows).
    - Performs lightweight HTTP `HEAD` asset accessibility check.
- `strategy_resolver.rs`:
    - `resolve_upgrade_strategy(crate_name, target_version)`: Evaluates network
      availability and checks local toolchain presence on `$PATH`.
- `version_check.rs`:
    - `start_background_version_check(crate_name)`: Non-blocking async task querying
      `crates.io` API.
    - Updates atomic state `UpgradeCheckResult`.
- `source_build.rs`:
    - Generalized PTY session manager for `rustup` toolchain updates and `cargo` install.
    - OSC 9;4 compilation escape sequence parser and live `Spinner` updater.
- `run_upgrade.rs`:
    - `handle_upgrade_at_exit()`: Coordinates exit phase based on policy and output
      target.
    - `run_upgrade()`: Executes the resolved `UpgradeSource` (streaming GitHub download +
      `self_replace` OR PTY/OSC `rustup` + `cargo install`) with dynamic fallback if
      GitHub fails mid-stream.
    - `run_upgrade_cli_arg_interceptor()`: Helper for standalone `--upgrade` CLI
      invocations.
    - Listens for cooperative `tokio::signal::ctrl_c()`.
- `ui_strings.rs`:
    - Parameterized messages for GitHub download progress (% + MB), fallback transition
      notices, source compilation progress, success, and failure/retry notices.
- `mod.rs`: Private modules and public barrel exports re-exported at `r3bl_tui::*`.

### 2. r3bl-cmdr

**Interactive Command-Line Workspace Tools (`giti`, `edi`, `rc`)**:

- Add `--upgrade` CLI argument to `giti`, `edi`, `rc` in their respective
  `clap_config.rs`.
- Intercept `--upgrade` at startup in `giti.rs`, `edi.rs`, `rc.rs` to run standalone
  upgrade and exit.
- Refactor `upgrade_check.rs` and `ui_str.rs` to delegate to
  `r3bl_tui::core::script::upgrade::*`.
- Delegate background version checking and upgrade execution to `r3bl_tui::*` using
  `UpgradePolicy::InteractivePrompt` and `UpgradeOutputTarget::Stdout`.
- Preserve cmdr-specific interactive prompts, exit context handling, emojis, and lolcat
  greetings.
- Release bundles all 3 binaries: `giti`, `edi`, and `rc`.

### 3. r3bl-build-infra

**Build Infrastructure & Testing Tools (`cargo-rustdoc-fmt`, `spawny`)**:

- Add `--upgrade` CLI argument to `cargo-rustdoc-fmt` and `spawny`.
- Intercept `--upgrade` at startup in `cargo-rustdoc-fmt.rs` and `spawny.rs`.
- Spawn background version check at startup in both binaries.
- At exit, invoke `r3bl_tui::handle_upgrade_at_exit()` with `UpgradePolicy::AutoUpgrade`
  (or `NotifyOnly` when `CI=true` or `R3BL_NO_AUTO_UPGRADE=1`) and
  `UpgradeOutputTarget::Stdout`.
- Release bundles all build-infra binaries: `cargo-rustdoc-fmt` and `spawny`.

### 4. r3bl-rust-analyzer-mcp-server

**Model Context Protocol Semantic Analysis Server (`rust-analyzer-mcp-server`)**:

- Add `--upgrade` CLI argument in `clap_config.rs`.
- Intercept `--upgrade` at startup in `main.rs`.
- Spawn background version check at startup in `main.rs`.
- At shutdown/exit, invoke `r3bl_tui::handle_upgrade_at_exit()` with
  `UpgradePolicy::AutoUpgrade` (or `NotifyOnly` when `CI=true` or
  `R3BL_NO_AUTO_UPGRADE=1`) and `UpgradeOutputTarget::Stderr` to ensure stdout JSON-RPC
  message transport remains clean.
- Release bundles `rust-analyzer-mcp-server`.

### 5. Bootstrap Scripts

**Zero-Dependency Platform Installers (`install.sh`, `install.ps1`)**:

- `install.sh`: POSIX shell bootstrap script for Linux & macOS
  (`curl -sSf .../install.sh | sh`).
- `install.ps1`: PowerShell bootstrap script for Windows (`irm .../install.ps1 | iex`).
- Supports optional target crate argument (`cmdr` (default), `build-infra`, `mcp-server`,
  or `all`).
- **First-Time Install Caveat During GitHub Outages**:
    - Bootstrap scripts fetch installers and release archives from GitHub.
    - If GitHub is down, new users cannot use `install.sh` or `install.ps1`.
    - All documentation must provide the standard source compilation fallback at the end:
      `cargo install <crate> --force` (where `<crate>` is `r3bl-cmdr`, `r3bl-build-infra`,
      or `r3bl-rust-analyzer-mcp-server`).

### 6. Release Script

**Automated End-to-End Release Automation (`release.fish`)**:

- `release.fish`: Automated, end-to-end release manager supporting both binary and library
  crates:
    1. **Version Validation**: Fails fast if target version is not strictly greater than
       current version.
    2. **CHANGELOG.md Verification**: Fails fast if `CHANGELOG.md` lacks entry for
       `### v<version>`.
    3. **Automated GitHub Release Notes Extraction**: Parses markdown changelog block for
       `v<version>`.
    4. **Quality Checks**: Executes `./check.fish --full`.
    5. **Multi-Platform Binary Packaging**: Builds Linux native binaries, cross-compiles
       Windows `.exe` binaries directly on Linux, strips symbols, and packages archives
       with `.sha256` checksums into `dist/`.
    6. **Dry-Run Validation**: Runs
       `cargo publish -p <crate> --dry-run --allow-dirty --no-verify`.
    7. **Interactive Confirmation**: Prompts for explicit user approval before publishing.
    8. **Atomic Publish & Release**: Bumps `Cargo.toml`, creates git commit/tag, pushes to
       remote, publishes to `crates.io`, and creates GitHub Release with attached `dist/*`
       assets.

### 7. Website & Documentation

**Consolidated Showcase & Installation Guides Across All Mediums**:

- **Crate READMEs & `lib.rs` Modules**: Each crate (`cmdr`, `build-infra` [for both
  `cargo-rustdoc-fmt` and `spawny`], `rust-analyzer-mcp-server`, and root
  `r3bl-open-core`) provides:
    1. Primary: 1-line bootstrap script (`install.sh` / `install.ps1`).
    2. Feature docs & `--upgrade` CLI flag instructions.
    3. End-of-file fallback instructions: `cargo install <crate> --force`.
- **R3BL Website Homepage (`r3bl.com`)**:
    - Update `../r3bl_website/index.html` to consolidate the entire tool suite
      (`r3bl-cmdr`, `r3bl-build-infra` [`cargo-rustdoc-fmt` + `spawny`],
      `r3bl-rust-analyzer-mcp-server`) with copy-pasteable bootstrap commands.
    - Replace placeholder text with modern product highlights and clear architecture
      diagrams.
    - Clean up and modernize `../r3bl_website/css/styles.css` and root variable styling.
    - Provide fallback source compilation instructions (`cargo install <crate> --force`).

---

## Implementation Plan

### Phase 1: Shared Upgrade Module

**Module implementation in `r3bl_tui`**:

- [ ] Add `self_replace = "1.5"` dependency to `tui/Cargo.toml`.
- [ ] Create `tui/src/core/script/upgrade/types.rs` defining `UpgradeSource`,
      `UpgradePolicy`, `UpgradeOutputTarget`, and `UpgradeCheckResult`.
- [ ] Create `tui/src/core/script/upgrade/target_triple.rs` implementing
      `get_host_target_triple()`.
- [ ] Create `tui/src/core/script/upgrade/github_release.rs` implementing asset URL
      formatting and HTTP `HEAD` availability check.
- [ ] Create `tui/src/core/script/upgrade/strategy_resolver.rs` implementing
      `resolve_upgrade_strategy(crate_name, target_version)` and local `$PATH` inspection.
- [ ] Move and generalize PTY/OSC `rustup` & `cargo` build engine into
      `tui/src/core/script/upgrade/source_build.rs`.
- [ ] Create `tui/src/core/script/upgrade/version_check.rs` wrapping `crates.io` check and
      `AtomicU8` state management.
- [ ] Create `tui/src/core/script/upgrade/run_upgrade.rs` implementing
      `handle_upgrade_at_exit()`, `run_upgrade_cli_arg_interceptor()`, binary streaming
      download with `self_replace`, and automatic source fallback execution.
- [ ] Create `tui/src/core/script/upgrade/ui_strings.rs` providing parameterized
      formatters.
- [ ] Create `tui/src/core/script/upgrade/mod.rs` with private modules and public barrel
      exports.
- [ ] Wire `upgrade` into `tui/src/core/script/mod.rs` and export at `r3bl_tui::*`.
- [ ] Add unit tests in `tui/src/core/script/upgrade/` for types, strategy resolver,
      target triple, and UI strings.
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

### Phase 2: Bootstrap & Release Script

**Bootstrap scripts (`install.sh`, `install.ps1`) and `release.fish`**:

- [ ] Create `install.sh` at repo root for Linux and macOS bootstrap installation.
- [ ] Create `install.ps1` at repo root for Windows PowerShell bootstrap installation.
- [ ] Create `release.fish` at repo root implementing:
    - `<crate>` and `<version>` argument parsing with interactive fallback.
    - Version validation against `<crate>/Cargo.toml`.
    - `CHANGELOG.md` entry verification.
    - Automated extraction of `CHANGELOG.md` section for GitHub Release notes.
    - Binary multi-platform compilation & packaging into `dist/` (Linux `.tar.gz` and
      Windows `.zip` cross-compiled directly on Linux) with `.sha256` checksums.
    - `cargo publish --dry-run` and explicit user confirmation prompt.
    - Automated version bump, git commit, git tag (`v<version>-<crate>`), git push,
      `cargo publish`, and `gh release create` with attached assets.
- [ ] Test `release.fish` validation guardrails (`--dry-run`, missing changelog, invalid
      version).
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `install.sh`
    - [ ] `install.ps1`
    - [ ] `release.fish`

### Phase 3: Refactor cmdr

**Integration into `r3bl-cmdr` suite (`giti`, `edi`, `rc`)**:

- [ ] Add `--upgrade` CLI flag to `giti`, `edi`, and `rc` in their respective
      `clap_config.rs`.
- [ ] Intercept `--upgrade` at startup in `giti.rs`, `edi.rs`, `rc.rs` to run standalone
      upgrade and exit.
- [ ] Refactor `cmdr/src/analytics_client/upgrade_check.rs` and `ui_str.rs` to delegate to
      `r3bl_tui::core::script::upgrade::*`.
- [ ] Run `cmdr` unit tests and verify `giti`, `edi`, and `rc` compile and function.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `cmdr/src/analytics_client/upgrade_check.rs`
    - [ ] `cmdr/src/analytics_client/ui_str.rs`
    - [ ] `cmdr/src/bin/giti.rs`
    - [ ] `cmdr/src/bin/edi.rs`
    - [ ] `cmdr/src/bin/rc.rs`
    - [ ] `cmdr/src/giti/clap_config.rs`
    - [ ] `cmdr/src/edi/clap_config.rs`
    - [ ] `cmdr/Cargo.toml`

### Phase 4: Integrate build-infra

**Integration into `r3bl-build-infra` (`cargo-rustdoc-fmt`, `spawny`)**:

- [ ] Add `--upgrade` CLI flag to `cargo-rustdoc-fmt` in
      `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`.
- [ ] Add `--upgrade` CLI flag to `spawny` in `build-infra/src/spawny/cli/`.
- [ ] Intercept `--upgrade` at startup in `build-infra/src/bin/cargo-rustdoc-fmt.rs` and
      `build-infra/src/bin/spawny.rs`.
- [ ] Integrate background check and exit upgrade handler (`UpgradePolicy::AutoUpgrade`,
      `UpgradeOutputTarget::Stdout`).
- [ ] Ensure CI / environment guardrail (`CI=true` or `R3BL_NO_AUTO_UPGRADE=1`) falls back
      to `UpgradePolicy::NotifyOnly`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `build-infra/src/bin/cargo-rustdoc-fmt.rs`
    - [ ] `build-infra/src/bin/spawny.rs`
    - [ ] `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`
    - [ ] `build-infra/src/spawny/cli/`
    - [ ] `build-infra/Cargo.toml`

### Phase 5: Integrate mcp-server

**Integration into `r3bl-rust-analyzer-mcp-server`**:

- [ ] Add `--upgrade` CLI flag to `rust-analyzer-mcp-server` in `clap_config.rs`.
- [ ] Intercept `--upgrade` at startup in `rust-analyzer-mcp-server/src/main.rs`.
- [ ] Integrate background check and shutdown upgrade handler
      (`UpgradePolicy::AutoUpgrade`, `UpgradeOutputTarget::Stderr`).
- [ ] Ensure CI / environment guardrail (`CI=true` or `R3BL_NO_AUTO_UPGRADE=1`) falls back
      to `UpgradePolicy::NotifyOnly`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `rust-analyzer-mcp-server/src/main.rs`
    - [ ] `rust-analyzer-mcp-server/Cargo.toml`

### Phase 6: Docs & Website

**Documentation, skills, release guides, and website updates**:

- [ ] Update `cmdr/README.md` and `cmdr/src/lib.rs`:
    - Add 1-line bootstrap install commands (`install.sh` and `install.ps1`).
    - Document `--upgrade` standalone CLI argument.
    - Add end-of-file fallback instructions: `cargo install r3bl-cmdr --force`.
- [ ] Update `build-infra/README.md` and `build-infra/src/lib.rs`:
    - Add bootstrap install commands (`install.sh build-infra` /
      `install.ps1 build-infra`).
    - Document `--upgrade` standalone CLI argument across both `cargo-rustdoc-fmt` and
      `spawny`.
    - Document features and commands for both `cargo-rustdoc-fmt` and `spawny`.
    - Add end-of-file fallback instructions: `cargo install r3bl-build-infra --force`.
- [ ] Update `rust-analyzer-mcp-server/README.md` and
      `rust-analyzer-mcp-server/src/lib.rs`:
    - Add bootstrap install commands (`install.sh mcp-server` / `install.ps1 mcp-server`).
    - Document `--upgrade` standalone CLI argument.
    - Add end-of-file fallback instructions:
      `cargo install r3bl-rust-analyzer-mcp-server --force`.
- [ ] Update root `README.md`:
    - Consolidate all binary tools with quick 1-line bootstrap install commands.
    - Document `--upgrade` usage across tools.
    - Add fallback `cargo install <crate> --force` instructions.
- [ ] Update `r3bl.com` website in `../r3bl_website/index.html`:
    - Replace placeholder content with product showcase for `r3bl-cmdr`,
      `r3bl-build-infra` (`cargo-rustdoc-fmt` + `spawny`), and
      `r3bl-rust-analyzer-mcp-server`.
    - Provide prominent copy-pasteable bootstrap install commands (`install.sh` and
      `install.ps1`).
    - Provide fallback source compilation instructions (`cargo install <crate> --force`).
    - Clean up and modernize `../r3bl_website/css/styles.css` and `css/root_vars.css`.
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
    - [ ] `../r3bl_website/css/styles.css`
    - [ ] `../r3bl_website/css/root_vars.css`
    - [ ] `.agents/skills/release-crate/SKILL.md`
    - [ ] `docs/release-guide.md`

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

---

## Cross-Platform Test Matrix

Comprehensive validation matrix ensuring correct behavior across all supported operating
systems (Linux, macOS Apple Silicon, macOS Intel, and Windows), network states, and
toolchain environments.

### Test Scenario Matrix

| Scenario                                 | Network State                              | Local Toolchain                    | Target Resolved                  | Expected Outcome & Verification                                                                                                                                                   |
| :--------------------------------------- | :----------------------------------------- | :--------------------------------- | :------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **1. Fast Binary Upgrade**               | GitHub CDN 200 OK                          | Any / None needed                  | `GithubReleaseArtifact`          | Streams `.tar.gz` (Unix) or `.zip` (Windows), verifies `.sha256`, in-place atomic replace via `self_replace`, exits 0. Running binary updated instantly.                          |
| **2. GitHub Down + Full Rustup**         | GitHub 404 / Offline                       | `rustup` + `cargo` on `$PATH`      | `BuildFromSourceRustup`          | Displays fallback message, runs `rustup toolchain install nightly --force` with PTY, runs `cargo +nightly install <crate>` with OSC 9;4 progress spinner, exits 0.                |
| **3. GitHub Down + Standalone Cargo**    | GitHub 404 / Offline                       | `cargo` on `$PATH` (no `rustup`)   | `BuildFromSourceCargo`           | Displays fallback message, runs `cargo +nightly install <crate>` (or `cargo install`), does not error on missing `rustup`, exits 0.                                               |
| **4. GitHub Down + Zero Rust Toolchain** | GitHub 404 / Offline                       | No `rustup`, no `cargo` on `$PATH` | `Err(NoViableUpgradeSource)`     | Displays clean error: `"Upgrade failed: GitHub release unreachable and no local Rust build toolchain found. Please retry later: <bin> --upgrade"`, exits 1 cleanly without panic. |
| **5. Mid-Stream Network Drop**           | GitHub connection aborts during download   | `rustup` or `cargo` on `$PATH`     | Dynamic fallback to Source Build | Download stream error caught, temp files cleaned up, seamlessly transitions to `BuildFromSourceRustup` or `BuildFromSourceCargo`.                                                 |
| **6. User Cancellation (Ctrl+C)**        | Interrupted during download or compilation | Any                                | Interrupted                      | Cooperative `tokio::signal::ctrl_c()` caught, PTY session / HTTP stream terminated, temp files purged, original binary remains 100% intact.                                       |
| **7. First-Time Bootstrap Install**      | GitHub CDN 200 OK                          | None required                      | Script installer                 | `install.sh` (Linux/macOS) / `install.ps1` (Windows) extracts binary to `~/.cargo/bin` or `$env:USERPROFILE\.cargo\bin`, sets permissions, runs immediately.                      |
| **8. First-Time Install GitHub Outage**  | GitHub Down / Offline                      | Rust toolchain present             | Manual Cargo fallback            | `cargo install <crate> --force` compiles from crates.io and installs cleanly.                                                                                                     |
| **9. MCP Server Stdio Transport Safety** | Any upgrade execution                      | Any                                | `UpgradeOutputTarget::Stderr`    | All progress, spinners, and logs route exclusively to `stderr`. `stdout` remains 100% pure JSON-RPC stream with zero corrupted frames.                                            |

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
    - [ ] Test `giti --upgrade`, `edi --upgrade`, `rc --upgrade`,
          `cargo-rustdoc-fmt --upgrade`, and `rust-analyzer-mcp-server --upgrade`.
    - [ ] Verify in-place binary swap replaces inode cleanly via POSIX `rename`.
    - [ ] Test fallback source compilation with Linux PTY (`/dev/pts`) when GitHub is
          simulated offline.
    - [ ] Verify `CI=true` and `R3BL_NO_AUTO_UPGRADE=1` suppress automated upgrades and
          fall back to `NotifyOnly`.
    - [ ] Run `./cleanup.fish` after verification completes.
- **Follow-up Workspace Task**:
    - Consolidate and generalize `cmdr/systemd-nspawn/` and
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

<!-- cspell:words USERPROFILE -->
