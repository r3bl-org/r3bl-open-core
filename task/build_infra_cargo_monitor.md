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

| Old (`check.fish`)       | New (`cargo monitor`)    |
| ------------------------ | ------------------------ |
| `./check.fish --watch`   | `cargo monitor`          |
| `./check.fish --watch-doc` | `cargo monitor doc`    |
| `./check.fish --watch-test` | `cargo monitor test`  |
| `./check.fish` (default) | `cargo monitor --once`   |
| `./check.fish --doc`     | `cargo monitor --once doc` |
| `./check.fish --test`    | `cargo monitor --once test` |
| `./check.fish --help`    | `cargo monitor --help`   |

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
│   ├── runner.rs                 # Execute cargo commands
│   ├── watcher.rs                # File watching with notify crate
│   ├── toolchain.rs              # Toolchain validation/repair
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
- [ ] Implement cargo command execution:
  - `cargo test --all-targets -q`
  - `cargo test --doc -q`
  - `cargo doc --no-deps`
- [ ] Capture stdout/stderr for ICE detection
- [ ] Measure execution duration
- [ ] Return structured `CheckResult`

### Phase 3: ICE and Stale Cache Detection

**Files to create:**
- `src/cargo_monitor/ice_detector.rs` - Corruption detection

**Tasks:**
- [ ] Port ICE detection logic from `check.fish`:
  - Check for `rustc-ice-*.txt` files
  - Detect exit code 101 with panic patterns
  - Detect stale cache parser errors
- [ ] Implement cleanup function:
  - Remove `target/` directory
  - Remove ICE dump files
- [ ] Implement retry logic (retry once after cleanup)

### Phase 4: Config Change Detection

**Files to create:**
- `src/cargo_monitor/config_hash.rs` - Config hashing

**Tasks:**
- [ ] Port config hash logic from `check_config_changed`:
  - Hash: `Cargo.toml`, `*/Cargo.toml`, `rust-toolchain.toml`, `.cargo/config.toml`
  - Store hash in `target/check/.config_hash`
  - Clean `target/check` on hash mismatch
- [ ] Use `sha2` crate for SHA256

### Phase 5: Toolchain Validation

**Files to create:**
- `src/cargo_monitor/toolchain.rs` - Toolchain management

**Tasks:**
- [ ] Port toolchain validation from `ensure_toolchain_installed`:
  - Read toolchain from `rust-toolchain.toml`
  - Check if toolchain is installed via `rustup toolchain list`
  - Check required components: `rust-analyzer`, `rust-src`
  - Verify `rustc` works
- [ ] Delegate to `rust-toolchain-sync-to-toml.fish` for installation
  - Or implement native rustup commands

### Phase 6: File Watcher

**Files to create:**
- `src/cargo_monitor/watcher.rs` - File watching

**Tasks:**
- [ ] Implement file watching with `notify` crate:
  - Watch directories: `cmdr/src/`, `analytics_schema/src/`, `tui/src/`
  - Watch config files: `Cargo.toml`, `rust-toolchain.toml`, etc.
  - Exclude: `target/`, `.git/`
- [ ] Implement debouncing (5 seconds default, configurable)
- [ ] Handle target directory auto-recovery:
  - Periodically check if `target/check` exists
  - Trigger rebuild if missing
- [ ] Implement single-instance enforcement:
  - Kill existing watch instances on startup
  - Use PID file or process detection

### Phase 7: Notifications

**Files to create:**
- `src/cargo_monitor/notification.rs` - Desktop notifications

**Tasks:**
- [ ] Port notification logic from `send_system_notification`:
  - macOS: `osascript` (AppleScript)
  - Linux: `gdbus` (GNOME) with `notify-send` fallback
- [ ] Implement auto-dismiss (5 seconds default)
- [ ] Notification triggers:
  - Watch mode: success and failure
  - Once mode: failure always, success only if duration > 1s

### Phase 8: Terminal UI

**Files to create:**
- `src/cargo_monitor/ui.rs` - Output formatting

**Tasks:**
- [ ] Implement colorized output:
  - Timestamps: `[HH:MM:SS AM/PM]`
  - Status icons: `✅`, `❌`, `🔄`, `👀`, `🧊`, `⚠️`
  - Color coding: green=success, red=failure, yellow=warning, cyan=info
- [ ] Progress indicators for long operations
- [ ] Duration formatting: `1m 30s`, `5s`, `0.5s`

### Phase 9: Integration and Testing

**Tasks:**
- [ ] Add module to `lib.rs`
- [ ] Update `build-infra/CLAUDE.md` with new binary info
- [ ] Write unit tests for:
  - CLI parsing
  - Config hashing
  - ICE detection patterns
  - Duration formatting
- [ ] Write integration tests:
  - One-off mode execution
  - Watch mode (with mock watcher)
- [ ] Manual testing:
  - `cargo install --path build-infra --force`
  - Test all CLI combinations
  - Test ICE recovery
  - Test config change detection

## Feature Parity Checklist

From `check.fish`, ensure these features are implemented:

- [ ] **Watch modes**: `--watch`, `--watch-test`, `--watch-doc` → default, `test`, `doc`
- [ ] **One-off modes**: default, `--test`, `--doc` → `--once`, `--once test`, `--once doc`
- [ ] **Single instance enforcement**: Kill existing watch instances
- [ ] **Config change detection**: Hash config files, clean on change
- [ ] **Toolchain validation**: Validate before running, auto-install if needed
- [ ] **ICE detection**: Exit code 101, panic patterns, `rustc-ice-*.txt`
- [ ] **Stale cache detection**: Parser errors with punctuation tokens
- [ ] **ICE recovery**: Clean target, retry once
- [ ] **Target directory recovery**: Rebuild if `target/check` is missing
- [ ] **Debouncing**: 5-second window to prevent rapid re-runs
- [ ] **Desktop notifications**: Success/failure with auto-dismiss
- [ ] **Timestamps**: 12-hour format with AM/PM
- [ ] **Duration tracking**: Per-check and total duration
- [ ] **Colorized output**: Status icons and colors
- [ ] **CARGO_TARGET_DIR**: Use `target/check` to avoid IDE conflicts

## Nice-to-Have Enhancements

These could be added after initial parity:

- [ ] **Parallel checks**: Run tests and docs concurrently
- [ ] **Custom watch directories**: `--watch-dir` flag
- [ ] **Custom debounce**: `--debounce` flag
- [ ] **Quiet mode**: `--quiet` for minimal output
- [ ] **JSON output**: `--json` for machine-readable results
- [ ] **Clippy integration**: Add clippy as a check type
- [ ] **Build-only mode**: Just compile without tests
- [ ] **Nextest support**: Use cargo-nextest if available

## Implementation Notes

### Cargo Plugin Convention

Like `cargo-rustdoc-fmt`, the binary must handle Cargo's subcommand injection:
- Direct: `cargo-monitor --once doc` → args = `["cargo-monitor", "--once", "doc"]`
- Via Cargo: `cargo monitor --once doc` → args = `["cargo-monitor", "monitor", "--once", "doc"]`

Use the existing `strip_cargo_subcommand_injection()` pattern from `cargo-rustdoc-fmt.rs`.

### Target Directory Isolation

Use `CARGO_TARGET_DIR=target/check` to avoid lock contention with IDEs. This is critical for
watch mode where the IDE might also be compiling.

### Async Considerations

The watcher needs to be async or threaded to:
1. Watch for file changes
2. Run checks (blocking cargo commands)
3. Handle Ctrl+C gracefully
4. Send notifications

Consider using `tokio` (already a dependency) with `tokio::process::Command` for subprocess
management.

## Exit Codes

| Code | Meaning                           |
| ---- | --------------------------------- |
| 0    | All checks passed                 |
| 1    | One or more checks failed         |
| 2    | Toolchain installation failed     |
| 130  | Interrupted (Ctrl+C)              |

## Timeline Estimate

| Phase | Description              | Estimate |
| ----- | ------------------------ | -------- |
| 1     | Core Infrastructure      | 2 hours  |
| 2     | Check Runner             | 2 hours  |
| 3     | ICE Detection            | 1 hour   |
| 4     | Config Hash              | 1 hour   |
| 5     | Toolchain Validation     | 2 hours  |
| 6     | File Watcher             | 3 hours  |
| 7     | Notifications            | 1 hour   |
| 8     | Terminal UI              | 1 hour   |
| 9     | Integration & Testing    | 3 hours  |
| **Total** |                      | **~16 hours** |

## References

- `check.fish` - Source of truth for functionality
- `script_lib.fish` - Shared utilities (notifications, toolchain, etc.)
- `build-infra/src/bin/cargo-rustdoc-fmt.rs` - Existing binary pattern
- `build-infra/CLAUDE.md` - Binary installation workflow
