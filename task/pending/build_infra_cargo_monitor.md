<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

**Table of Contents** _generated with [DocToc](https://github.com/thlorenz/doctoc)_

- [Task: Implement `cargo monitor` Binary](#task-implement-cargo-monitor-binary)
  - [Overview](#overview)
  - [CLI Design](#cli-design)
    - [Mapping from `check.fish`](#mapping-from-checkfish)
  - [Architecture](#architecture)
    - [Module Structure](#module-structure)
    - [Key Dependencies to Add](#key-dependencies-to-add)
  - [Implementation Phases](#implementation-phases)
    - [Phase 1: Core Infrastructure](#phase-1-core-infrastructure)
    - [Phase 2: Check Runner](#phase-2-check-runner)
    - [Phase 3: ICE and Stale Cache Detection](#phase-3-ice-and-stale-cache-detection)
    - [Phase 4: Config Change Detection](#phase-4-config-change-detection)
    - [Phase 5: Toolchain Validation](#phase-5-toolchain-validation)
    - [Phase 6: File Watcher](#phase-6-file-watcher)
    - [Phase 7: Notifications](#phase-7-notifications)
    - [Phase 8: Terminal UI](#phase-8-terminal-ui)
    - [Phase 9: Integration and Testing](#phase-9-integration-and-testing)
  - [Feature Parity Checklist](#feature-parity-checklist)
  - [Nice-to-Have Enhancements](#nice-to-have-enhancements)
  - [Implementation Notes](#implementation-notes)
    - [Cargo Plugin Convention](#cargo-plugin-convention)
    - [Target Directory Isolation (tmpfs)](#target-directory-isolation-tmpfs)
    - [Performance Optimizations](#performance-optimizations)
    - [Async Considerations](#async-considerations)
  - [Exit Codes](#exit-codes)
  - [References](#references)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Task: Implement `cargo monitor` Binary

## Overview

Create a new cargo subcommand `cargo monitor` in `build-infra/` that replaces the functionality of
`check.fish`. The binary will be a Rust implementation providing continuous code verification with
watch mode as the default behavior.

## CLI Design

```bash
# Watch modes (default behavior - no flag needed)
cargo monitor              # watch all: tests + doctests + docs
cargo monitor doc          # watch docs only
cargo monitor test         # watch tests + doctests only

# One-off modes (explicit flag)
cargo monitor --once       # run all checks once
cargo monitor --once doc   # build docs once
cargo monitor --once test  # run tests + doctests only once

# Help
cargo monitor --help
cargo monitor help
```

### Mapping from `check.fish`

| Old (`check.fish`)          | New (`cargo monitor`)       |
| --------------------------- | --------------------------- |
| `./check.fish --watch`      | `cargo monitor`             |
| `./check.fish --watch-doc`  | `cargo monitor doc`         |
| `./check.fish --watch-test` | `cargo monitor test`        |
| `./check.fish` (default)    | `cargo monitor --once`      |
| `./check.fish --doc`        | `cargo monitor --once doc`  |
| `./check.fish --test`       | `cargo monitor --once test` |
| `./check.fish --help`       | `cargo monitor --help`      |

## Architecture

### Module Structure

```
build-infra/src/
├── bin/
│   ├── cargo-rustdoc-fmt.rs      # Existing
│   └── cargo-monitor.rs          # NEW: Binary entry point
├── cargo_monitor/                 # NEW: Implementation module
│   ├── mod.rs                    # Module root, public re-exports
│   ├── cli_arg.rs                # CLI argument parsing (clap)
│   ├── types.rs                  # CheckType, CheckResult, etc.
│   ├── runner.rs                 # Execute cargo commands + two-stage doc builds
│   ├── watcher.rs                # File watching with notify crate
│   ├── toolchain.rs              # Toolchain validation/repair/corruption detection
│   ├── ice_detector.rs           # ICE and stale cache detection
│   ├── config_hash.rs            # Config change detection
│   ├── notification.rs           # Desktop notifications
│   └── ui.rs                     # Terminal output formatting
├── cargo_rustdoc_fmt/            # Existing
├── common/                       # Existing shared utilities
└── lib.rs                        # Add cargo_monitor module
```

### Key Dependencies to Add

```toml
# In build-infra/Cargo.toml [dependencies]
notify = "8"                      # File watching (cross-platform)
notify-debouncer-mini = "0.6"     # Debouncing for file events
crossbeam-channel = "0.5"         # Channel for watcher events
which = "7"                       # Find executables (inotifywait, etc.)
sha2 = "0.10"                     # SHA256 for config hash
dirs = "6"                        # Standard directories
```

## Implementation Phases

### Phase 1: Core Infrastructure

**Files to create:**

- `src/bin/cargo-monitor.rs` - Binary entry point
- `src/cargo_monitor/mod.rs` - Module root
- `src/cargo_monitor/cli_arg.rs` - CLI parsing
- `src/cargo_monitor/types.rs` - Core types

**Tasks:**

- [ ] Add `[[bin]]` section to Cargo.toml for `cargo-monitor`
- [ ] Create CLI argument structure with clap
  - Subcommands: `doc`, `test`, (none = all)
  - Flags: `--once`, `--verbose`, `--help`
- [ ] Define core types:
  - `CheckType`: `All`, `Test`, `Doc`
  - `RunMode`: `Watch`, `Once`
  - `CheckResult`: Success, Failure, Ice

### Phase 2: Check Runner

**Files to create:**

- `src/cargo_monitor/runner.rs` - Execute cargo commands

**Tasks:**

- [ ] Implement cargo command execution with `ionice_wrapper` pattern:
  - `cargo test --all-targets -q`
  - `cargo test --doc -q`
  - `cargo doc --no-deps` (quick mode)
  - `cargo doc` (full mode with deps)
- [ ] Capture stdout/stderr for ICE detection
- [ ] Measure execution duration
- [ ] Return structured `CheckResult`
- [ ] Implement cross-platform `ionice_wrapper`:
  - Linux: `ionice -c2 -n0` for highest I/O priority in best-effort class
  - macOS: Run command directly (no ionice available)

**Two-Stage Doc Builds (--watch-doc / `cargo monitor doc`):**

The doc build mode uses a sophisticated two-stage architecture to provide fast feedback while still
building complete documentation:

```
Stage 1: Quick Build (BLOCKING)
├── Target: r3bl_tui only (--no-deps)
├── Duration: ~3-5 seconds
├── Staging dir: /tmp/roc/target/check-doc-staging-quick
├── rsync to serving dir (no --delete)
└── Notify: "Quick Docs Ready"

Stage 2: Full Build (BACKGROUND/FORKED)
├── Target: All crates + dependencies
├── Duration: ~90 seconds
├── Staging dir: /tmp/roc/target/check-doc-staging-full
├── rsync to serving dir (--delete only if orphans detected)
└── Notify: "Full Docs Built"
```

**Why Quick Blocks but Full Forks:**

- Cargo uses a global package cache lock (`~/.cargo/.package-cache`)
- Running both simultaneously causes "Blocking waiting for file lock" messages
- Quick build is fast (~3-5s), so blocking is acceptable for immediate feedback
- Full build is slow (~90s), so forking lets user continue editing

**Orphan File Cleanup (full builds only):**

- Long-running watch sessions accumulate stale docs from renamed/deleted files
- Detection: compare file counts - if serving > staging, orphans exist
- Cleanup: full builds use rsync `--delete` to remove orphaned files
- Quick builds never delete (would wipe dependency docs)

**Log File:**

- Background builds log to `/tmp/roc/check.log` for debugging
- Use `log_and_print` pattern for dual stdout + file output

### Phase 3: ICE and Stale Cache Detection

**Files to create:**

- `src/cargo_monitor/ice_detector.rs` - Corruption detection

**Tasks:**

- [ ] Port ICE detection logic from `check.fish`:
  - Check for `rustc-ice-*.txt` files in current directory
  - Detect exit code 101 with panic patterns:
    - `internal compiler error`
    - `thread 'rustc' panicked`
    - `rustc ICE`
    - `panicked at 'mir_`
  - Detect stale cache parser errors:
    - Pattern: `` expected.*found `[^a-zA-Z0-9]` `` (single punctuation token)
    - Combined with `could not compile` message
- [ ] Implement cleanup function:
  - Remove all target directories (main + staging)
  - Remove ICE dump files (`rustc-ice-*.txt`)
- [ ] Implement retry logic (retry once after cleanup)

### Phase 4: Config Change Detection

**Files to create:**

- `src/cargo_monitor/config_hash.rs` - Config hashing

**Tasks:**

- [ ] Port config hash logic from `check_config_changed`:
  - Hash: `Cargo.toml`, `*/Cargo.toml`, `rust-toolchain.toml`, `.cargo/config.toml`
  - Store hash in `/tmp/roc/target/check/.config_hash`
  - Clean `/tmp/roc/target/check` on hash mismatch
- [ ] Use `sha2` crate for SHA256
- [ ] Algorithm:
  1. Concatenate all config file contents
  2. Compute SHA256 hash of concatenated content
  3. Compare with stored hash
  4. If different: clean target dir, store new hash, rebuild
  5. If same: proceed without cleaning (artifacts are valid)

### Phase 5: Toolchain Validation

**Files to create:**

- `src/cargo_monitor/toolchain.rs` - Toolchain management

**Tasks:**

- [ ] Port toolchain validation from `ensure_toolchain_installed`:
  - Read toolchain from `rust-toolchain.toml`
  - Check if toolchain is installed via `rustup toolchain list`
  - Check required components: `rust-analyzer`, `rust-src`
  - Verify `rustc` works
- [ ] Port corruption detection from `is_toolchain_corrupted`:
  - Detect "Missing manifest in toolchain" errors
  - Detect toolchains that appear installed but are broken internally
  - Symptoms: repeated "syncing channel updates" loops
- [ ] Implement `force_remove_corrupted_toolchain`:
  - Try `rustup toolchain uninstall` first
  - Fall back to direct folder deletion (`~/.rustup/toolchains/<name>`)
  - Clear rustup caches (`~/.rustup/downloads/`, `~/.rustup/tmp/`)
- [ ] Delegate to `rust-toolchain-sync-to-toml.fish` for installation
  - Or implement native rustup commands
- [ ] On sync failure: Show last 30 lines of output (not silently suppressed)

### Phase 6: File Watcher

**Files to create:**

- `src/cargo_monitor/watcher.rs` - File watching

**Tasks:**

- [ ] Implement file watching with `notify` crate:
  - Watch directories: `cmdr/src/`, `analytics_schema/src/`, `tui/src/`
  - Watch config files: `Cargo.toml`, `*/Cargo.toml`, `rust-toolchain.toml`, `.cargo/config.toml`
  - Exclude: `target/`, `.git/`
- [ ] Implement sliding window debounce (2 seconds default):
  - Wait for first event (blocks until file change)
  - Start sliding window with 2-second timeout
  - If new event arrives: reset window (loop back)
  - If timeout expires: quiet period detected, run checks
  - Each new change resets the window, coalescing rapid saves
- [ ] Handle target directory auto-recovery:
  - Periodically check if `/tmp/roc/target/check` exists (every 10 seconds)
  - Trigger rebuild if missing (handles `cargo clean`, manual `rm -rf`, etc.)
- [ ] Implement single-instance enforcement:
  - Kill existing watch instances on startup
  - Also kill orphaned inotifywait/fswatch processes
  - Use PID file or process detection

### Phase 7: Notifications

**Files to create:**

- `src/cargo_monitor/notification.rs` - Desktop notifications

**Tasks:**

- [ ] Port notification logic from `send_system_notification`:
  - macOS: `osascript` (AppleScript) - always available
  - Linux: `gdbus` (GNOME) for reliable auto-dismiss, with `notify-send` fallback
- [ ] Implement auto-dismiss (5 seconds / 5000ms default)
- [ ] For GNOME: Use `gdbus` to send notification, then `CloseNotification` after timeout (GNOME
      ignores notify-send's `--expire-time`)
- [ ] Notification triggers:
  - Watch mode: success and failure
  - Once mode: failure always, success only if duration > 1s (`NOTIFICATION_THRESHOLD_SECS`)

### Phase 8: Terminal UI

**Files to create:**

- `src/cargo_monitor/ui.rs` - Output formatting

**Tasks:**

- [ ] Implement colorized output:
  - Timestamps: `[HH:MM:SS AM/PM]` (12-hour format)
  - Status icons: `✅`, `❌`, `🔄`, `👀`, `🧊`, `⚠️`, `🔪`, `🧹`, `📝`, `🔨`, `🔀`
  - Color coding: green=success, red=failure, yellow=warning, cyan=info, brblack=debug
- [ ] Progress indicators for long operations
- [ ] Duration formatting via `format_duration`:
  - Sub-second: `0.5s`
  - Seconds only: `5s`
  - Minutes and seconds: `2m 30s`
  - Hours, minutes, seconds: `1h 5m 30s`

### Phase 9: Integration and Testing

**Tasks:**

- [ ] Add module to `lib.rs`
- [ ] Update `build-infra/CLAUDE.md` with new binary info
- [ ] Write unit tests for:
  - CLI parsing
  - Config hashing
  - ICE detection patterns
  - Duration formatting
  - Toolchain corruption detection patterns
- [ ] Write integration tests:
  - One-off mode execution
  - Watch mode (with mock watcher)
  - Two-stage doc build orchestration
- [ ] Manual testing:
  - `cargo install --path build-infra --force`
  - Test all CLI combinations
  - Test ICE recovery
  - Test config change detection
  - Test toolchain corruption recovery

## Feature Parity Checklist

From `check.fish`, ensure these features are implemented:

- [ ] **Watch modes**: `--watch`, `--watch-test`, `--watch-doc` → default, `test`, `doc`
- [ ] **One-off modes**: default, `--test`, `--doc` → `--once`, `--once test`, `--once doc`
- [ ] **Single instance enforcement**: Kill existing watch instances + orphaned watchers
- [ ] **Config change detection**: SHA256 hash config files, clean on change
- [ ] **Toolchain validation**: Validate before running, auto-install if needed
- [ ] **Toolchain corruption detection**: "Missing manifest" detection, force-remove, reinstall
- [ ] **ICE detection**: Exit code 101, panic patterns, `rustc-ice-*.txt`
- [ ] **Stale cache detection**: Parser errors with single punctuation tokens
- [ ] **ICE recovery**: Clean all target dirs, retry once
- [ ] **Target directory recovery**: Rebuild if `/tmp/roc/target/check` is missing (check every 10s)
- [ ] **Sliding window debounce**: 2-second quiet window to prevent rapid re-runs
- [ ] **Desktop notifications**: Success/failure with 5-second auto-dismiss
- [ ] **Timestamps**: 12-hour format with AM/PM
- [ ] **Duration tracking**: Per-check and total duration
- [ ] **Colorized output**: Status icons and colors
- [ ] **tmpfs target directory**: Use `/tmp/roc/target/check` for ~2-3x faster builds
- [ ] **Parallel jobs**: `CARGO_BUILD_JOBS=28` for maximum parallelism (60% faster docs)
- [ ] **Two-stage doc builds**: Quick (blocking) + Full (background) with separate staging dirs
- [ ] **Orphan file cleanup**: Detect and remove stale doc files in full builds
- [ ] **Cross-platform ionice**: `ionice_wrapper` for Linux I/O priority, passthrough on macOS
- [ ] **Log file**: `/tmp/roc/check.log` for background build debugging

## Nice-to-Have Enhancements

These could be added after initial parity:

- [ ] **Custom watch directories**: `--watch-dir` flag
- [ ] **Custom debounce**: `--debounce` flag (default: 2s)
- [ ] **Quiet mode**: `--quiet` for minimal output
- [ ] **JSON output**: `--json` for machine-readable results
- [ ] **Clippy integration**: Add clippy as a check type
- [ ] **Build-only mode**: Just compile without tests
- [ ] **Nextest support**: Use cargo-nextest if available

**Note on parallel checks:** The current implementation cannot run tests and docs concurrently
because cargo uses a global package cache lock. This is why quick doc builds block before forking
full doc builds to background.

## Implementation Notes

### Cargo Plugin Convention

Like `cargo-rustdoc-fmt`, the binary must handle Cargo's subcommand injection:

- Direct: `cargo-monitor --once doc` → args = `["cargo-monitor", "--once", "doc"]`
- Via Cargo: `cargo monitor --once doc` → args = `["cargo-monitor", "monitor", "--once", "doc"]`

Use the existing `strip_cargo_subcommand_injection()` pattern from `cargo-rustdoc-fmt.rs`.

### Target Directory Isolation (tmpfs)

Use `CARGO_TARGET_DIR=/tmp/roc/target/check` to:

1. Avoid lock contention with IDEs (which use `target/`)
2. Build in RAM (tmpfs) for ~2-3x faster builds
3. Eliminate SSD wear from frequent rebuilds

**Trade-off:** Build cache is lost on reboot - first post-reboot build is cold (~2-4 min),
subsequent builds use cached artifacts (fast, ~9 seconds for 2,700+ tests).

**Directory Structure:**

```
/tmp/roc/
├── target/
│   └── check/                    # Main target dir (tests, serving docs)
│       ├── doc/                  # Served docs (browser loads from here)
│       └── .config_hash          # Config change detection hash
├── target/check-doc-staging-quick/   # Quick doc build staging
│   └── doc/
└── target/check-doc-staging-full/    # Full doc build staging
    └── doc/
```

### Performance Optimizations

1. **tmpfs builds**: All I/O happens in RAM, no SSD/HDD seeks
2. **Parallel jobs**: `CARGO_BUILD_JOBS=28` forces maximum CPU utilization
3. **ionice priority**: `ionice -c2 -n0` gives highest I/O priority (Linux only)
4. **Incremental compilation**: Rust only recompiles what changed
5. **Cached test binaries**: Re-running tests just executes the binary

### Async Considerations

The watcher needs to be async or threaded to:

1. Watch for file changes
2. Run checks (blocking cargo commands)
3. Handle Ctrl+C gracefully
4. Send notifications
5. Fork background doc builds

Consider using `tokio` (already a dependency) with `tokio::process::Command` for subprocess
management.

## Exit Codes

| Code | Meaning                       |
| ---- | ----------------------------- |
| 0    | All checks passed             |
| 1    | One or more checks failed     |
| 2    | Toolchain installation failed |
| 130  | Interrupted (Ctrl+C)          |

## References

- `check.fish` - Source of truth for functionality
- `script_lib.fish` - Shared utilities (notifications, toolchain, ionice, etc.)
- `rust-toolchain-sync-to-toml.fish` - Nuclear toolchain reinstall
- `rust-toolchain-validate.fish` - Toolchain validation (quick and complete modes)
- `build-infra/src/bin/cargo-rustdoc-fmt.rs` - Existing binary pattern
- `build-infra/CLAUDE.md` - Binary installation workflow
