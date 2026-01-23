# Release Plan for r3bl_tui v0.7.7, r3bl-cmdr v0.0.25, and r3bl-build-infra v0.0.1

## Overview

This release involves significant documentation updates and a new crate publication:
- **tui v0.7.7**: DirectToAnsi backend, VT100 parser, RRT pattern, PTY testing, benchmark testing
- **cmdr v0.0.25**: Removal of `ch` binary
- **build-infra v0.0.1**: First release of `cargo-rustdoc-fmt` tool (NEW)

## Files to Modify

| File | Change Type | Priority |
|------|-------------|----------|
| `CHANGELOG.md` | Add entries for global-config, tui v0.7.7, cmdr v0.0.25, build-infra v0.0.1 | HIGH |
| `README.md` | Add Claude Code section, fix script names, document workflows | HIGH |
| `tui/src/lib.rs` | Add DirectToAnsi, RRT, VT100 parser docs (match existing style) | HIGH |
| `cmdr/src/lib.rs` | Verify no `ch` references | MEDIUM |
| `build-infra/Cargo.toml` | Verify ready for publish (version, metadata) | HIGH |
| `build-infra/src/lib.rs` | Ensure docs are crates.io ready | HIGH |
| `tui/Cargo.toml` | Bump 0.7.6 → 0.7.7 | HIGH |
| `cmdr/Cargo.toml` | Bump 0.0.24 → 0.0.25, update tui dep | HIGH |
| `docs/release-guide.md` | Add build-infra to release workflow (line ~15) | HIGH |
| `tui/README.md` | Regenerate with `cargo readme` | HIGH |
| `cmdr/README.md` | Regenerate with `cargo readme` | HIGH |
| `build-infra/README.md` | Regenerate with `cargo readme` | HIGH |

## Key Findings

### Current State
- **tui**: v0.7.6 in Cargo.toml → bumping to v0.7.7
- **cmdr**: v0.0.24 in Cargo.toml → bumping to v0.0.25
- **Binaries in cmdr**: `giti`, `edi`, `rc` (NO `ch` binary - it was removed)
- **CHANGELOG.md**: Single file at repo root (not per-crate)

### Major Changes Since Last Release

**tui:**
1. **DirectToAnsi backend** - Our own input/output device implementation (no Crossterm on Linux, only on macOS/Windows)
2. **Full VT100 parser engine** - Complete VT100 input and output parser for terminal escape sequences
3. **Resilient Reactor Thread (RRT) pattern** - Generic infrastructure for dedicated worker threads
4. **PTY testing infrastructure** - "Real world" testing with pseudo-terminals
5. **Benchmark testing** - Performance regression detection framework
6. Enhanced readline_async: Tab, BackTab, navigation keys, FnKey support
7. Type-safe editor state methods in readline_async
8. mio_poller refactoring with thread liveness and generation tracking

**cmdr:**
1. `ch` binary was removed (was added in v0.0.22, documented in v0.0.24, now gone)

**build-infra (NEW - first release):**
1. `cargo-rustdoc-fmt` - Cargo subcommand for formatting rustdoc comments (markdown tables, reference-style links)

**global-config/workflow:**
1. r3bl-vscode-extensions + r3bl extension pack integration with Claude Code
2. Comprehensive scripts: `check.fish`, `rust-toolchain*.fish`, `bootstrap.sh`
3. Claude Code skills and slash commands documented

---

## Implementation Tasks

### Task 1: Update CHANGELOG.md [COMPLETE]

**File:** `/home/nazmul/github/roc/CHANGELOG.md`

#### 1a. Update global-config (next) section (line ~180)

Add to the existing "Changed" section or create new entries:

**Changed:**
- **Documentation:** Updated README.md with comprehensive Claude Code integration section documenting `.claude/` directory structure, available skills, and slash commands
- **Documentation:** Fixed script name inconsistencies (`rust-toolchain-validate.fish` not `rust-toolchain-install-validate.fish`)
- **Documentation:** Added tmux-r3bl-dev.fish development dashboard documentation
- **Documentation:** Clarified VSCode extensions installation workflow

**Removed:**
- Deprecated `setup-dev-tools.sh` references (if applicable - verify with user)

#### 1b. Enhance tui v0.7.7 (next) section

Add these items to the existing v0.7.7 section:

**Added:**
- **Resilient Reactor Thread (RRT) pattern** - Generic infrastructure for managing dedicated worker threads that:
  - Block on I/O (stdin, sockets, signals) using epoll/mio
  - Broadcast events to async consumers via broadcast channels
  - Handle graceful shutdown when all consumers disconnect
  - Support thread restart/reuse with generation tracking
- `ThreadSafeGlobalState<W, E>` - Thread-safe singleton pattern for RRT
- `ThreadLiveness` - Running state + generation tracking for safe thread reuse
- `SubscriberGuard<W, E>` - Manages subscriber lifecycle with waker access

**Major Infrastructure Upgrades:**
- **OffscreenBuffer VT100 implementation** - Complete in-memory terminal emulator enabling snapshot testing
  - Full VT100 ANSI escape sequence support (cursor, erase, scroll, SGR, etc.)
  - VTE parser integration with custom `Performer` implementation
  - Enables visual verification of terminal output in PTY tests
- **PTY testing infrastructure** - Real-world testing in pseudo-terminals, not mocks
  - `generate_pty_test!` macro for single-feature tests
  - `spawn_controlled_in_pty` for multi-backend comparison tests
  - Controller/Controlled pattern for isolation
  - Backend compatibility tests (DirectToAnsi vs Crossterm)

**Enhanced readline_async API:**
- Tab and BackTab (Shift+Tab) key support
- Navigation keys support (arrow keys, Home, End, etc.)
- FnKey support (F1-F12)
- Type-safe editor state methods via `ReadlineAsyncContext`
- Extended `ReadlineEvent` enum with new variants

**Changed:**
- Refactored mio_poller module for improved clarity and thread reuse semantics
- Reduced DirectToAnsi input device complexity
- Thread liveness tracking integrated with mio_poller for restart capability

#### 1c. Add cmdr v0.0.25 section

**New section after v0.0.24:**

```markdown
### v0.0.25 (YYYY-MM-DD)

Removed the experimental `ch` (Claude Code prompt history) binary to focus on core `giti` and `edi` functionality.

- Removed:
  - `ch` binary and all associated code (Claude Code prompt history recall tool)
  - `ch` module from library exports

- Changed:
  - Updated documentation to reflect only `giti` and `edi` as the main binaries
```

#### 1d. Add build-infra v0.0.1 section (NEW CRATE)

**Add new section for r3bl-build-infra:**

```markdown
## `r3bl-build-infra`

### v0.0.1 (YYYY-MM-DD)

Initial release of the R3BL build infrastructure crate, providing cargo subcommands for Rust documentation maintenance.

- Added:
  - `cargo-rustdoc-fmt` binary - Cargo subcommand for formatting rustdoc comments
    - Markdown table alignment in `///` and `//!` doc comments
    - Inline-to-reference link conversion for cleaner documentation
    - Workspace-aware processing (specific files, directories, or entire workspace)
    - Git integration (auto-detect changed files, staged/unstaged, from latest commit)
    - Check mode for CI verification (`--check` flag)
    - Selective formatting (tables only, links only, or both)
  - Modular library API for programmatic use
```

---

### Task 2: Update Main README.md [COMPLETE]

**File:** `/home/nazmul/github/roc/README.md`

**Completed:**
- 2a. Fixed script name inconsistency (rust-toolchain-install-validate.fish → rust-toolchain-validate.fish)
- 2b. Added Claude Code Integration section with skills table and slash commands
- Regenerated TOC with doctoc

**Notes:**
- 2c/2d: VSCode extensions and tmux documentation already exist in README
- 2e: setup-dev-tools.sh is deprecated (bootstrap.sh handles rustup, nushell no longer needed)
- Minor doc debt: tmux section mentions "r3bl-dev" session but script uses "r3bl"

#### 2a. Fix Script Name Inconsistencies (HIGH PRIORITY)

- **Line ~1691**: References `rust-toolchain-install-validate.fish` but actual file is `rust-toolchain-validate.fish`
- **Lines ~1760-1761**: Same issue in unified script architecture diagram
- Update all references to use correct script names

#### 2b. Add Claude Code Integration Section (HIGH PRIORITY)

Add new section documenting `.claude/` directory structure:

```markdown
## Claude Code Integration

This project is configured for optimal use with [Claude Code](https://claude.ai/claude-code).

### Project Instructions
The `CLAUDE.md` file at the repo root provides project-specific instructions that Claude Code follows automatically.

### Available Skills (.claude/skills/)
Claude Code autonomously discovers and applies these coding patterns:
- `check-code-quality` - Comprehensive quality checklist
- `run-clippy` - Linting and formatting
- `write-documentation` - Rustdoc conventions
- `organize-modules` - Module organization patterns
- `check-bounds-safety` - Type-safe Index/Length patterns
- `analyze-performance` - Flamegraph analysis
- `design-philosophy` - Core design principles

### Slash Commands
Invoke skills directly:
- `/check` - Run code quality checks
- `/docs` - Documentation formatting
- `/clippy` - Linting
- `/fix-intradoc-links` - Fix rustdoc links
- `/check-regression` - Performance regression detection
- `/analyze-logs` - Log file analysis
- `/r3bl-task` - Task management
```

#### 2c. Fix VSCode Extensions Installation (MEDIUM PRIORITY)

- Update references to `./install.sh` - clarify it's in the r3bl-vscode-extensions repo
- Add correct installation workflow:
  1. Clone r3bl-vscode-extensions repo
  2. Run install.sh from that repo
  3. Or manually install each extension

#### 2d. Document tmux-r3bl-dev.fish (MEDIUM PRIORITY)

Add dedicated section for the tmux development dashboard script:
- What it does (launches tmux with bacon, watch, etc.)
- How to use it
- Configuration options

#### 2e. Clarify setup-dev-tools.sh Status (MEDIUM PRIORITY)

Either:
- Remove if deprecated (bootstrap.sh replaced it)
- Or document its purpose if still needed

---

### Task 3: Update cmdr/src/lib.rs [COMPLETE]

**File:** `/home/nazmul/github/roc/cmdr/src/lib.rs`

**Status: No changes needed - `ch` binary already removed**
- Documentation only mentions `giti` and `edi` ✓
- Module exports: `analytics_client`, `common`, `edi`, `giti`, `rc` (no `ch`) ✓
- Cargo.toml only has binaries: `giti`, `edi`, `rc` ✓
- No `ch` module export visible

---

### Task 4: Update tui/src/lib.rs [COMPLETE]

**File:** `/home/nazmul/github/roc/tui/src/lib.rs`

**Documentation Style Guide** (match existing sections like "Type-safe bounds checking", "Grapheme support"):
- Main heading with opening paragraph explaining the concept
- Subsections (##) for specific topics: "The Challenge", "The Solution", "Key Features", "Architecture", "Learn More"
- Code examples with clear comments
- Bullet lists for features/benefits
- "Learn More" section linking to module docs

**Major additions needed:**

#### 4a. Add DirectToAnsi Backend Section (comprehensive)

```markdown
# DirectToAnsi Backend (Linux-Native)

The R3BL TUI engine includes a custom terminal I/O backend that bypasses Crossterm on Linux,
providing direct ANSI escape sequence handling for maximum performance and control.

## Why DirectToAnsi?

On Linux, we've replaced Crossterm with our own implementation:
- **Direct control**: No intermediary library between our code and the terminal
- **Optimized for our use case**: Tailored to TUI app patterns
- **Full VT100 compliance**: Complete input and output parser engine

Crossterm is still used on macOS and Windows where platform-specific APIs differ significantly.

## Architecture

The backend consists of two main components:
- **Input Device**: Uses `mio` (epoll on Linux) for async I/O multiplexing
- **Output Device**: `PixelCharRenderer` with smart attribute diffing (~30% output reduction)

## Key Features

- Zero-latency ESC key detection
- Full keyboard modifier support (Ctrl, Alt, Shift)
- Mouse event handling with bracketed paste
- SIGWINCH signal integration for terminal resize
- Thread lifecycle management via RRT pattern

## Learn More

See [`mod@tui::terminal_lib_backends::direct_to_ansi`] for implementation details.
```

#### 4b. Add RRT Pattern Section

```markdown
# Resilient Reactor Thread (RRT) Pattern

The RRT pattern provides generic infrastructure for managing dedicated worker threads
that handle blocking I/O operations. This powers the DirectToAnsi backend's mio_poller.

## The Problem

Async executors (like Tokio) use thread pools that shouldn't block. Terminal input
requires blocking on stdin, which would starve other async tasks.

## The Solution

RRT dedicates a thread to blocking I/O that:
1. Blocks on I/O (stdin, sockets, signals) using epoll/mio
2. Broadcasts events to async consumers via channels
3. Handles graceful shutdown when consumers disconnect
4. Supports thread restart/reuse with generation tracking

## Key Components

- [`ThreadSafeGlobalState<W, E>`] - Thread-safe singleton for RRT instances
- [`ThreadLiveness`] - Running state + generation for safe thread reuse
- [`SubscriberGuard`] - Manages subscriptions with automatic cleanup on drop

## Learn More

See [`mod@core::resilient_reactor_thread`] for the generic implementation.
```

#### 4c. Enhance VT100 Parser Section (Major Upgrade)

**Key change to highlight:** The `OffscreenBuffer` now has a complete VT100 ANSI implementation that serves as an **in-memory terminal emulator**. This upgrade enabled snapshot testing for PTY tests.

```markdown
# VT100/ANSI Escape Sequence Engine

The R3BL TUI engine includes a complete VT100 parser for both **input** and **output**:

## Input Parsing
- Keyboard events (with modifiers: Ctrl, Alt, Shift)
- Mouse events (clicks, scrolling, movement)
- Terminal events (resize via SIGWINCH)
- Bracketed paste sequences

## Output Parsing & In-Memory Terminal Emulation

The [`OffscreenBuffer`] implements a full VT100 terminal emulator that can:
- Parse ANSI escape sequences from any source (PTY output, test data, etc.)
- Maintain cursor position, scroll regions, and text attributes
- Render to an in-memory character grid

This enables **snapshot testing** - capturing the visual state of terminal output
for verification without needing a real terminal.

## Implementation

The output parser uses the `vte` crate with a custom [`Performer`] implementation
that routes VT100 operations to [`OffscreenBuffer`] methods:

| VT100 Operation | OffscreenBuffer Method |
|-----------------|------------------------|
| Print character | `put_char_at_cursor()` |
| Cursor movement | `move_cursor_*()` |
| Erase operations | `erase_*()` |
| Scroll regions | `set_scroll_region()`, `scroll_*()` |
| SGR (colors/styles) | `set_current_style()` |

See [`mod@core::ansi::vt_100_pty_output_parser`] for the output parser and
[`mod@tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl`] for
the OffscreenBuffer VT100 implementation.
```

#### 4d. PTY Testing Infrastructure Section (CRITICAL - Major Investment)

**This is a key differentiator of the crate and deserves prominent documentation.**

```markdown
# PTY-Based Integration Testing

The R3BL TUI library includes a comprehensive PTY (pseudo-terminal) testing
infrastructure that validates code in **real terminal environments**, not mocks.

## Why PTY Testing?

Unit tests with mocked I/O miss real-world issues:
- Race conditions between input and output
- Terminal buffering behavior
- Signal handling (SIGWINCH, SIGINT)
- Raw mode edge cases
- Platform-specific terminal quirks

PTY tests spawn actual processes in pseudo-terminals, sending real byte sequences
and observing real terminal behavior. **This catches bugs that hypothetical scenarios miss.**

## Architecture: Controller/Controlled Pattern

```text
┌─────────────────────────┐     PTY I/O      ┌─────────────────────────┐
│ Controller Process      │◄────────────────►│ Controlled Process      │
│ (Test runner)           │                  │ (Code under test)       │
│                         │                  │                         │
│ • Spawns controlled     │   stdin ────────►│ • Runs in raw mode      │
│ • Sends input bytes     │                  │ • Executes test logic   │
│ • Reads output          │◄──── stdout ─────│ • Produces output       │
│ • Verifies assertions   │                  │                         │
└─────────────────────────┘                  └─────────────────────────┘
```

## Two Testing Approaches

### 1. Single-Feature Tests: [`generate_pty_test!`]

For testing one specific behavior (e.g., Ctrl+W word deletion, SIGWINCH handling):

```rust
generate_pty_test!(
    test_ctrl_w_deletes_word,
    controller_fn,    // Sends input, verifies output
    controlled_fn     // Runs in PTY, executes behavior
);
```

### 2. Backend Comparison Tests: [`spawn_controlled_in_pty`]

For verifying DirectToAnsi and Crossterm produce **identical results**:

```rust
// Run same test with DirectToAnsi backend
let (_, pty_direct) = spawn_controlled_in_pty("direct_to_ansi", ...);
let output_direct = capture_output(&pty_direct);

// Run same test with Crossterm backend
let (_, pty_crossterm) = spawn_controlled_in_pty("crossterm", ...);
let output_crossterm = capture_output(&pty_crossterm);

// Compare: both backends must produce identical output
assert_eq!(output_direct, output_crossterm);
```

## Snapshot Testing with OffscreenBuffer

The upgraded [`OffscreenBuffer`] enables snapshot testing:
1. Controlled process renders to terminal (via PTY)
2. Controller captures raw ANSI output
3. [`OffscreenBuffer`] parses ANSI into in-memory grid
4. Grid state is compared against expected snapshot

This validates the **visual correctness** of terminal output.

## Test Categories

The test suite covers:
- Bracketed paste handling
- Keyboard modifiers (Ctrl, Alt, Shift combinations)
- Mouse events (clicks, scrolling, movement)
- Terminal resize (SIGWINCH)
- UTF-8 text input
- Raw mode enable/disable cycles
- mio_poller thread lifecycle and reuse
- Backend compatibility (DirectToAnsi vs Crossterm)

See [`mod@core::test_fixtures::pty_test_fixtures`] for the testing infrastructure.
```

#### 4e. Add Performance/Benchmark Testing Section

```markdown
# Performance Regression Detection

The crate includes infrastructure for detecting performance regressions:
- Flamegraph-based profiling with `.perf-folded` file generation
- Baseline comparison for detecting regressions
- Integration with the `/check-regression` Claude Code skill

See [`analyze-performance` skill] for usage details.
```

#### 4f. Enhance readline_async Section

Add to existing section:
- Tab/BackTab (Shift+Tab) key support
- Navigation keys (arrows, Home, End, PageUp, PageDown)
- FnKey support (F1-F12)
- Type-safe editor state methods via `ReadlineAsyncContext`

---

### Task 5: Prepare build-infra for First Release [COMPLETE]

**File:** `/home/nazmul/github/roc/build-infra/`

**Changes made:**
- Improved description to be search-optimized
- Updated keywords for better discoverability
- Updated tui dependency from 0.7.6 → 0.7.7
- Enhanced lib.rs installation instructions for crates.io

#### 5a. Cargo.toml Required Fields for crates.io

**Currently present (verify these are accurate):**

| Field | Current Value | Required? | Notes |
|-------|--------------|-----------|-------|
| `name` | `"r3bl-build-infra"` | ✅ Required | Package name on crates.io |
| `version` | `"0.0.1"` | ✅ Required | Semver version |
| `edition` | `"2024"` | Recommended | Rust edition (requires Rust 1.84+) |
| `license` | `"Apache-2.0"` | ✅ Required | SPDX identifier |
| `description` | `"Build infrastructure tools..."` | ✅ Required | Short description for crates.io listing |
| `repository` | GitHub URL | Recommended | Links to source code |
| `documentation` | docs.rs URL | Recommended | Auto-generated by docs.rs |
| `homepage` | `"https://r3bl.com"` | Optional | Project homepage |
| `readme` | `"README.md"` | Recommended | Displayed on crates.io |
| `keywords` | 5 keywords | Recommended | Max 5, each ≤20 chars, for search |
| `categories` | 2 categories | Recommended | From [crates.io/category_slugs](https://crates.io/category_slugs) |
| `authors` | Name + email | Optional | Package authors |

**Action items:**

1. **Improve `description`** - Current is generic. Consider:
   ```toml
   description = "Cargo subcommand for formatting rustdoc markdown tables and converting inline links to reference-style"
   ```

2. **Verify `keywords`** - Current: `["build-tools", "rustdoc", "markdown", "formatting", "cargo"]`
   - Good selection, covers key search terms

3. **Verify `categories`** - Current: `["development-tools", "command-line-utilities"]`
   - Valid slugs from crates.io category list

4. **Update tui dependency** - Change from path-only to versioned:
   ```toml
   r3bl_tui = { path = "../tui", version = "0.7.7" }  # After tui is published
   ```

#### 5b. README Structure (crates.io landing page)

Use "Specific Description + README Roadmap" strategy for maximum discoverability NOW while showing vision:

**Description** (search-optimized, first ~100 chars matter):
```toml
description = "Cargo subcommand for formatting rustdoc comments: markdown table alignment and reference-style links"
```

**Keywords** (current tool focused):
```toml
keywords = ["rustdoc", "markdown", "formatting", "cargo", "documentation"]
```

**README structure:**
```markdown
# r3bl-build-infra

Cargo subcommands for Rust development workflows.

## Available Tools

### `cargo-rustdoc-fmt` (v0.0.1)
Format markdown tables and convert inline links to reference-style in rustdoc comments.

- Workspace-aware processing
- Git integration (auto-detect changed files)
- Check mode for CI (`--check`)

## Coming Soon 🚀

### `cargo-monitor`
Unified development workflow automation (Rust port of `check.fish` and `rust-toolchain*.fish`):

**Watch Mode (default):**
- Continuous compilation, testing, and doc building on file changes
- Sliding window debounce (2-second quiet period) to coalesce rapid saves
- Single-instance enforcement (kills orphaned watchers)

**Optimized Builds:**
- **tmpfs target directory** (`/tmp/roc/target/check`) - ~2-3x faster builds in RAM
- **`$CARGO_TARGET_DIR` isolation** - Separate from IDE's `target/` to avoid lock contention
- **Parallel compiler frontend** - Leverages `-Z threads=N` for maximum parallelism
- **`CARGO_BUILD_JOBS=28`** - Maximum CPU utilization

**Toolchain Management:**
- Automated nightly validation (ensuring stable nightly for parallel frontend)
- Corruption detection ("Missing manifest" errors, broken toolchains)
- Force-remove and reinstall corrupted toolchains
- Config change detection (SHA256 hash of Cargo.toml files)

**Resilience:**
- ICE (Internal Compiler Error) detection and auto-recovery
- Stale cache detection (parser errors from corrupted incremental builds)
- Target directory auto-recovery (rebuilds if `target/` is deleted)

**Two-Stage Doc Builds:**
- Quick build (blocking, ~3-5s) for immediate feedback
- Full build (background, ~90s) with complete dependency docs
- Orphan file cleanup for long-running sessions

**Cross-Platform:**
- Linux: `ionice` for I/O priority, `inotifywait` for file watching
- macOS: Native file watching, no ionice

See the [cargo-monitor implementation plan][cargo-monitor-plan] for details.

[cargo-monitor-plan]: https://github.com/r3bl-org/r3bl-open-core/blob/v0.0.1-build-infra/task/pending/build_infra_cargo_monitor.md

## Installation
\`\`\`bash
cargo install r3bl-build-infra
\`\`\`
```

#### 5c. Verify lib.rs documentation

Ensure lib.rs has:
- Crate-level documentation (`//!`) matching the README content
- Clear explanation of what `cargo-rustdoc-fmt` does
- Installation: `cargo install r3bl-build-infra`
- Usage examples: `cargo rustdoc-fmt --workspace`
- Feature list (table formatting, link conversion, git integration, check mode)

#### 5d. Generate README.md

```bash
cd build-infra && cargo readme > README.md
```

---

### Task 6: Update release-guide.md [COMPLETE]

**File:** `/home/nazmul/github/roc/docs/release-guide.md`

**Changes made:**
- Updated v0.7.6-tui → v0.7.7-tui (all occurrences)
- Updated v0.0.24-cmdr → v0.0.25-cmdr (all occurrences)
- Added new build-infra section with v0.0.1-build-infra

The release-guide.md tracks the **latest published versions** in the "Full workflow" section. Update version numbers AND add the new build-infra section.

#### 6a. Update existing version numbers

| Crate | Line | Current | New |
|-------|------|---------|-----|
| tui | ~49-50 | `v0.7.6-tui` | `v0.7.7-tui` |
| cmdr | ~68-69 | `v0.0.24-cmdr` | `v0.0.25-cmdr` |

#### 6b. Add new build-infra section (after cmdr, before the final "Push" section)

Insert after line 74 (`cd ..` ending cmdr section), before line 76 (`# Push the git commit...`):

```sh
cd build-infra
# 1. Update version in Cargo.toml (for self, optionally for dep: `r3bl_tui`)
#    and this file
# 2. Update CHANGELOG.md (don't forget to update TOC)
# 3. Run "Dependi: Update All dependencies to the latest version" in vscode
#    w/ the Cargo.toml file open. Don't use `cargo-edit`
#    <https://github.com/killercup/cargo-edit> and `cargo upgrade`.
cargo update --verbose # Update Cargo.lock file (not Cargo.toml)
cargo build; cargo test; cargo doc --no-deps; cargo clippy --fix --allow-dirty --allow-staged; cargo fmt --all
# Generate the crates.io landing page for this crate
cargo readme > README.md
cargo publish --dry-run --allow-dirty
git add -A
git commit -S -m "v0.0.1-build-infra"
git tag -a v0.0.1-build-infra -m "v0.0.1-build-infra"
cargo publish
# Don't forget to test the release by running `cargo install r3bl-build-infra`
# Then verify: `cargo rustdoc-fmt --help`
git push ; git push --tags # Push tags & commits
cd ..
```

#### 6c. Update the "Overview" section (~line 95)

Add `build-infra` to the list of crates:
```
Starting at the root folder of the project, eg `~/github/r3bl-open-core/`, the following steps are
applied to each crate (`tui`, `cmdr`, `build-infra`, `analytics_schema`):
```

**Order in release-guide.md:**
1. analytics_schema (top - no deps)
2. tui (depends on analytics_schema)
3. cmdr (depends on tui)
4. build-infra (depends on tui) ← NEW

---

### Task 7: Update Cargo.toml Versions [COMPLETE]

**Files:**
- tui/Cargo.toml: 0.7.6 → 0.7.7 ✓
- cmdr/Cargo.toml: 0.0.24 → 0.0.25 ✓
- cmdr's r3bl_tui dep: 0.7.6 → 0.7.7 ✓
- build-infra's r3bl_tui dep: 0.7.6 → 0.7.7 ✓ (already updated in Task 5)

---

### Task 8: Generate README.md Files [COMPLETE]

Generated via `cargo readme > README.md`:
- tui/README.md ✓
- cmdr/README.md ✓
- build-infra/README.md ✓

After lib.rs updates:

```bash
cd tui && cargo readme > README.md
cd ../cmdr && cargo readme > README.md
cd ../build-infra && cargo readme > README.md
```

---

### Task 9: Run Release Verification [COMPLETE]

**Results:**
- `cargo build --workspace` ✓
- `cargo test --workspace --lib` ✓ (2682 tests passed)
- `cargo clippy --workspace -- -D warnings` ✓ (fixed 2 doc backtick issues)
- `cargo doc --workspace --no-deps` ✓
- `cargo publish --dry-run` for analytics_schema ✓
- `cargo publish --dry-run` for tui ✓

**Manual steps remaining:**
- User to run `cargo publish` manually for each crate

---

## Execution Order

**Phase 1: Documentation Updates**
1. **CHANGELOG.md** - Update all sections:
   - global-config (next): Documentation & workflow updates
   - tui v0.7.7 (next): DirectToAnsi, VT100, RRT, PTY testing, benchmark
   - cmdr v0.0.25: `ch` binary removal
   - build-infra v0.0.1: NEW - first release
2. **Main README.md** - Major updates:
   - Fix script name inconsistencies
   - Add Claude Code Integration section
   - Document development scripts and workflows
3. **docs/release-guide.md** - Add build-infra to release workflow

**Phase 2: Crate Documentation**
4. **cmdr/src/lib.rs** - Verify no `ch` references remain
5. **tui/src/lib.rs** - Add DirectToAnsi, RRT, VT100, PTY, benchmark sections (match existing style)
6. **build-infra/src/lib.rs** - Verify crates.io ready

**Phase 3: Version Bumps**
7. **Cargo.toml files** - Bump versions:
   - tui: 0.7.6 → 0.7.7
   - cmdr: 0.0.24 → 0.0.25 (update tui dep)
   - build-infra: verify 0.0.1 (update tui dep)

**Phase 4: Generate & Verify**
8. **Generate READMEs** - `cargo readme > README.md` in tui/, cmdr/, build-infra/
9. **Update CHANGELOG.md TOC** - Run `doctoc CHANGELOG.md`
10. **Verify** - Run build/test/clippy/doc cycle in all crates
11. **Dry run publish** - `cargo publish --dry-run` in each crate

## Complete Release Sequence

### Step 1: Make All Changes (no publishing yet) [COMPLETE]
- [x] Update CHANGELOG.md (global-config, tui v0.7.7, cmdr v0.0.25, build-infra v0.0.1)
- [x] Update main README.md (Claude Code section, script fixes)
- [x] Update docs/release-guide.md (add build-infra)
- [x] Update tui/src/lib.rs (DirectToAnsi, RRT, VT100, PTY testing sections)
- [x] Verify cmdr/src/lib.rs (no `ch` references)
- [x] Update build-infra description, README with roadmap
- [x] Bump versions in Cargo.toml files
- [x] Run `cargo readme > README.md` in tui/, cmdr/, build-infra/
- [x] Run `doctoc CHANGELOG.md`
- [x] Run full verification: `cargo build && cargo test && cargo clippy && cargo doc --no-deps`
- [x] Run `cargo publish --dry-run` in analytics_schema, tui

**Note:** User runs all `cargo publish` commands manually. Claude tracks progress here.

### Step 2: Publish analytics_schema (if changed) [SKIPPED - no changes]
```bash
cd analytics_schema
cargo publish --dry-run --allow-dirty
git add -A && git commit -S -m "v0.0.3-analytics_schema"  # or current version
git tag -a v0.0.3-analytics_schema -m "v0.0.3-analytics_schema"
cargo publish
git push && git push --tags
cd ..
```
**Note:** Only if analytics_schema has changes. Currently at v0.0.3.

### Step 3: Publish r3bl_tui (others depend on this) [COMPLETE]
```bash
cd tui
cargo publish --dry-run --allow-dirty   # ✅ DONE
git add -A && git commit -S -m "v0.7.7-tui"   # ✅ DONE
git tag -a v0.7.7-tui -m "v0.7.7-tui"   # ✅ DONE
cargo publish   # ✅ DONE
git push && git push --tags   # ✅ DONE
cd ..
```

### Step 4: Publish r3bl-cmdr (depends on tui) [COMPLETE]
```bash
cd cmdr
cargo publish --dry-run --allow-dirty   # ✅ DONE
git add -A && git commit -S -m "v0.0.25-cmdr"   # ✅ DONE
git tag -a v0.0.25-cmdr -m "v0.0.25-cmdr"   # ✅ DONE
cargo publish   # ✅ DONE
git push && git push --tags   # ✅ DONE
cd ..
```

### Step 5: Publish r3bl-build-infra (depends on tui) - FIRST RELEASE 🎉 [WORK_IN_PROGRESS]
```bash
cd build-infra
cargo publish --dry-run --allow-dirty   # ✅ DONE
git add -A && git commit -S -m "v0.0.1-build-infra"   # ← Next
git tag -a v0.0.1-build-infra -m "v0.0.1-build-infra"
cargo publish
# Test: cargo install r3bl-build-infra && cargo rustdoc-fmt --help
git push && git push --tags
cd ..
```

### Step 6: Create GitHub Releases
For each tag, create a GitHub release at https://github.com/r3bl-org/r3bl-open-core/releases/new:
- **v0.7.7-tui**: Copy release notes from CHANGELOG.md
- **v0.0.25-cmdr**: Include `cargo install r3bl-cmdr` instructions
- **v0.0.1-build-infra**: Include `cargo install r3bl-build-infra` instructions, highlight "Coming Soon" roadmap

### Dependency Chain
```
analytics_schema (v0.0.3) ─┐
                          ├──► tui (v0.7.7) ─┬──► cmdr (v0.0.25)
                          │                  │
                          │                  └──► build-infra (v0.0.1)
                          │
                          └──► cmdr (also depends directly)
```

---

## Verification

After all changes:
1. Run `/check` skill to verify code quality
2. Run `cargo doc --no-deps --open` to preview documentation
3. Run `cargo publish --dry-run --allow-dirty` in each crate (tui, cmdr, build-infra)
4. Review generated README.md files for accuracy
5. Test `cargo install r3bl-build-infra` works (after publish)
6. Test `cargo rustdoc-fmt --help` works after install

