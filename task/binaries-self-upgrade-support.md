# Task: Self-Upgrade Support Across R3BL Binaries

## Overview

Add self-upgrade capabilities to binary crates in the workspace (`r3bl-build-infra` and
`r3bl-rust-analyzer-mcp-server`) while sharing the core upgrade machinery across all R3BL
tools. Currently, `r3bl-cmdr` (`cmdr/src/analytics_client/upgrade_check.rs`) contains a
complete implementation for background crates.io version checks and interactive PTY-driven
upgrades (`rustup` toolchain update + `cargo install` with OSC progress and spinner).

By extracting this reusable infrastructure into `r3bl_tui`
(`tui/src/core/script/upgrade/`), we create a unified, maintainable facility that any R3BL
binary can use with minimal boilerplate.

### Architecture & Crate Responsibilities

1. **`r3bl_tui` (`tui/src/core/script/upgrade/`)**:
    - `version_check.rs`: Background async task querying crates.io for the latest release
      version, storing results in `UpgradeCheckResult` using `AtomicU8` with
      `AtomicU8Ext`.
    - `run_upgrade.rs`: PTY-driven upgrade execution running
      `rustup toolchain install nightly --force` followed by
      `cargo +nightly install <crate_name>`, with real-time OSC progress parsing, animated
      spinner, and Ctrl+C cancellation.
    - `ui_strings.rs`: Parameterized messages for installation success, failure, command
      execution errors, and upgrade notifications.
    - `mod.rs`: Barrel exports re-exported at `r3bl_tui::*`.

2. **`r3bl-cmdr` (`cmdr/src/analytics_client/`)**:
    - Refactor `upgrade_check.rs` and `ui_str.rs` to delete duplicated PTY/OSC/spinner
      execution code.
    - Delegate version checking and upgrade execution to `r3bl_tui::*`.
    - Preserve cmdr-specific interactive prompts, exit context handling, emojis, and
      lolcat greetings.

3. **`r3bl-build-infra` (`build-infra/`)**:
    - Add `--upgrade` flag to `cargo-rustdoc-fmt` CLI args.
    - Spawn background version check at startup in `cargo-rustdoc-fmt.rs`.
    - Execute upgrade directly when `--upgrade` flag is passed.
    - Print notification on stdout at the end of normal formatting runs if an update is
      available.

4. **`r3bl-rust-analyzer-mcp-server` (`rust-analyzer-mcp-server/`)**:
    - Add `--upgrade` flag to `CLIArg`.
    - When `--upgrade` is passed, run `r3bl_tui::run_upgrade_with_spinner` directly in the
      terminal.
    - During standard MCP stdio operation, ensure background upgrade checks only emit
      diagnostics or notifications to stderr (or structured tracing) so that stdout
      JSON-RPC message transport remains completely clean and unbroken.

## Implementation plan

### Phase 1: Shared Upgrade Module in `r3bl_tui`

- [ ] Create `tui/src/core/script/upgrade/mod.rs` with private modules and public barrel
      exports.
- [ ] Create `tui/src/core/script/upgrade/version_check.rs` implementing
      `UpgradeCheckResult`, `start_background_version_check()`, and
      `get_bin_name_from_current_exe()`.
- [ ] Create `tui/src/core/script/upgrade/run_upgrade.rs` implementing
      `run_upgrade_with_spinner()`, `run_rustup_update()`,
      `run_cargo_install_with_progress()`, and `handle_osc_event()`.
- [ ] Create `tui/src/core/script/upgrade/ui_strings.rs` providing parameterized status
      and notification formatters.
- [ ] Wire `upgrade` into `tui/src/core/script/mod.rs` and export at `r3bl_tui::*`.
- [ ] Add unit tests in `tui/src/core/script/upgrade/` for `version_check`, `run_upgrade`
      (`extract_rustup_progress`), and `ui_strings`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/src/core/script/upgrade/mod.rs`
    - [ ] `tui/src/core/script/upgrade/version_check.rs`
    - [ ] `tui/src/core/script/upgrade/run_upgrade.rs`
    - [ ] `tui/src/core/script/upgrade/ui_strings.rs`
    - [ ] `tui/src/core/script/mod.rs`
    - [ ] `tui/Cargo.toml`

### Phase 2: Refactor `r3bl-cmdr` to Use Shared Module

- [ ] Refactor `cmdr/src/analytics_client/upgrade_check.rs` to delete extracted
      PTY/OSC/spinner functions and delegate to `r3bl_tui::*`.
- [ ] Refactor `cmdr/src/analytics_client/ui_str.rs` to remove the duplicated
      `upgrade_install` module and delegate to
      `r3bl_tui::upgrade_available_notification()`.
- [ ] Run `cmdr` unit tests and verify `giti`, `edi`, and `rc` binaries compile and
      function.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `cmdr/src/analytics_client/upgrade_check.rs`
    - [ ] `cmdr/src/analytics_client/ui_str.rs`
    - [ ] `cmdr/Cargo.toml`

### Phase 3: Integrate Self-Upgrade into `r3bl-build-infra`

- [ ] Add `--upgrade` flag to `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`.
- [ ] Integrate background check and `--upgrade` execution handler into
      `build-infra/src/bin/cargo-rustdoc-fmt.rs`.
- [ ] Update `build-infra/Cargo.toml` dependencies if needed.
- [ ] Add CLI argument parsing tests for `--upgrade` flag in `build-infra`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`
    - [ ] `build-infra/src/bin/cargo-rustdoc-fmt.rs`
    - [ ] `build-infra/Cargo.toml`

### Phase 4: Integrate Self-Upgrade into `r3bl-rust-analyzer-mcp-server`

- [ ] Add `--upgrade` flag to `rust-analyzer-mcp-server/src/cli_arg.rs`.
- [ ] Integrate `--upgrade` execution handler into `rust-analyzer-mcp-server/src/main.rs`.
- [ ] Integrate background version check into `rust-analyzer-mcp-server/src/main.rs` with
      logging restricted to stderr / structured tracing to preserve MCP stdio JSON-RPC
      protocol integrity.
- [ ] Update `rust-analyzer-mcp-server/Cargo.toml` dependencies if needed.
- [ ] Add CLI argument parsing tests for `--upgrade` in `rust-analyzer-mcp-server`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `rust-analyzer-mcp-server/src/cli_arg.rs`
    - [ ] `rust-analyzer-mcp-server/src/main.rs`
    - [ ] `rust-analyzer-mcp-server/Cargo.toml`

### Phase 5: Workspace Audit, Verification, and Testing

- [ ] Audit workspace to ensure no orphaned legacy upgrade code, unused imports, or dead
      functions remain.
- [ ] Run `./check.fish --check` across the entire workspace.
- [ ] Run `./check.fish --build` across the entire workspace.
- [ ] Run `./check.fish --clippy` across the entire workspace.
- [ ] Run `./check.fish --test` across the entire workspace.
- [ ] Verify `cargo rustdoc-fmt --upgrade` and `rust-analyzer-mcp-server --upgrade` CLI
      help and invocation.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `task/binaries-self-upgrade-support.md`
