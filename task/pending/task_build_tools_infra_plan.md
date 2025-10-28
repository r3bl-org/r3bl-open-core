# Build Tools Infrastructure Plan

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Overview](#overview)
  - [Task Description](#task-description)
  - [Current State](#current-state)
    - [Technical Issues](#technical-issues)
  - [Goals](#goals)
- [Implementation plan](#implementation-plan)
  - [Decision Analysis & Rationale](#decision-analysis--rationale)
    - [Options Considered](#options-considered)
      - [Option 1: Python Rewrite](#option-1-python-rewrite)
      - [Option 2: Rust Rewrite [COMPLETE] **SELECTED**](#option-2-rust-rewrite--selected)
  - [Game-Changing Factors](#game-changing-factors)
    - [1. Team is Rust Experts](#1-team-is-rust-experts)
    - [2. TUI Dogfooding = Product Development](#2-tui-dogfooding--product-development)
    - [3. Open Source Product Potential](#3-open-source-product-potential)
    - [4. Infrastructure Already Prepared](#4-infrastructure-already-prepared)
  - [Architecture](#architecture)
    - [Project Structure](#project-structure)
    - [Dependency Stack](#dependency-stack)
  - [Solving the Chicken-Egg Problem](#solving-the-chicken-egg-problem)
    - [Option A: Bootstrap Script [COMPLETE] **RECOMMENDED**](#option-a-bootstrap-script--recommended)
    - [Option B: Checked-In Pre-Built Binaries](#option-b-checked-in-pre-built-binaries)
    - [Option C: `cargo-xtask` Pattern](#option-c-cargo-xtask-pattern)
  - [Step 0: Proof of Concept [PENDING]](#step-0-proof-of-concept-pending)
  - [Step 1: Feature Parity [PENDING]](#step-1-feature-parity-pending)
  - [Step 2: TUI Excellence [PENDING]](#step-2-tui-excellence-pending)
  - [Step 3: Open Source Product [PENDING]](#step-3-open-source-product-pending)
  - [Command Migration Map](#command-migration-map)
    - [From `run.fish` → `r3bl-build`](#from-runfish-%E2%86%92-r3bl-build)
  - [Example TUI Implementations](#example-tui-implementations)
    - [1. Interactive Example Picker](#1-interactive-example-picker)
    - [2. Multi-Pane Dashboard](#2-multi-pane-dashboard)
    - [3. Log Viewer with Filtering](#3-log-viewer-with-filtering)
  - [Future `cargo-xtask-tui` API](#future-cargo-xtask-tui-api)
  - [Testing Strategy](#testing-strategy)
    - [Unit Tests](#unit-tests)
    - [Integration Tests](#integration-tests)
    - [TUI Testing](#tui-testing)
  - [Migration Timeline](#migration-timeline)
    - [Week 1: POC](#week-1-poc)
    - [Week 2: Feature Parity](#week-2-feature-parity)
    - [Week 3: TUI Excellence](#week-3-tui-excellence)
    - [Weeks 4-5: Product Extraction](#weeks-4-5-product-extraction)
    - [Week 6: Launch](#week-6-launch)
  - [Success Metrics](#success-metrics)
    - [Technical](#technical)
    - [User Experience](#user-experience)
    - [Product](#product)
  - [Risks and Mitigations](#risks-and-mitigations)
    - [Risk 1: TUI complexity slows development](#risk-1-tui-complexity-slows-development)
    - [Risk 2: pty_mux not mature enough](#risk-2-pty_mux-not-mature-enough)
    - [Risk 3: Team resistance to new tool](#risk-3-team-resistance-to-new-tool)
    - [Risk 4: Bootstrap script too complex](#risk-4-bootstrap-script-too-complex)
    - [Risk 5: Product extraction takes too long](#risk-5-product-extraction-takes-too-long)
  - [Open Questions](#open-questions)
  - [Next Steps](#next-steps)
  - [References](#references)
  - [Conclusion](#conclusion)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Overview

## Task Description

Rewrite the complex Fish shell scripts (`run.fish`, `script_lib.fish`, `rust-toolchain-*.fish`) into
a Rust-based build tool called `r3bl-build`. This decision is driven by team expertise alignment,
the opportunity to stress-test and showcase `r3bl_tui`, and the potential to extract generic
components into an open-source product.

## Current State

The project currently uses complex Fish shell scripts that are difficult to maintain and test:

- **`script_lib.fish`**: ~960 lines with complex toolchain management, Docker operations, profiling
- **`run.fish`**: ~950 lines with 30+ commands covering builds, tests, docs, examples, deployment
- **`rust-toolchain-*.fish`**: Multiple specialized scripts for toolchain synchronization

### Technical Issues

- [BLOCKED] Poor IDE support for Fish shell
- [BLOCKED] Limited testing frameworks
- [BLOCKED] Verbose error handling
- [BLOCKED] Difficult to maintain and extend
- [BLOCKED] No type safety

## Goals

1. Rewrite build scripts in Rust for better maintainability and testability
2. Create feature parity with existing Fish scripts
3. Build delightful TUI experiences for developer tools
4. Establish proof-of-concept for `cargo-xtask-tui` open-source product
5. Stress-test and showcase the `r3bl_tui` crate capabilities
6. Reduce build script complexity and improve IDE support

# Implementation plan

## Decision Analysis & Rationale

### Options Considered

#### Option 1: Python Rewrite

**Pros:**

- [COMPLETE] Excellent IDE support (PyCharm, VS Code with mypy, ruff)
- [COMPLETE] Rich testing with pytest
- [COMPLETE] Rapid iteration (no compilation)
- [COMPLETE] Type hints with mypy
- [COMPLETE] Rich CLI libraries (typer, click, rich)

**Cons:**

- [BLOCKED] Runtime dependency (Python 3.11+)
- [BLOCKED] Team lacks Python expertise
- [BLOCKED] Not dogfooding (Python for Rust project feels inconsistent)
- [BLOCKED] Cannot leverage TUI crate
- [BLOCKED] No product potential

**Effort:** [PENDING] MEDIUM (1-1.5 weeks)

---

#### Option 2: Rust Rewrite [COMPLETE] **SELECTED**

**Pros:**

- [COMPLETE] **Team Expertise:** Rust experts can write idiomatic, high-quality code
- [COMPLETE] **TUI Dogfooding:** Stress-test `r3bl_tui` crate with real-world complexity
- [COMPLETE] **Product Potential:** Can extract `cargo-xtask-tui` as open-source product
- [COMPLETE] **Existing Infrastructure:** `script/` module and `pty_mux` already prepared
- [COMPLETE] **Type Safety:** Compile-time error catching
- [COMPLETE] **Excellent Testing:** Full unit/integration test support
- [COMPLETE] **Great IDE Support:** rust-analyzer provides amazing tooling
- [COMPLETE] **Binary Distribution:** Single binary, no runtime dependencies
- [COMPLETE] **Workspace Integration:** Can directly use types/code from other crates

**Cons:**

- [BLOCKED] Chicken-egg complexity (solvable via bootstrap script)
- [BLOCKED] Compilation time for changes
- [BLOCKED] Higher initial effort

**Effort:** [BLOCKED] HIGH (2-3 weeks initially, but with long-term ROI)

---

## Game-Changing Factors

### 1. Team is Rust Experts

- No need to learn Python while rewriting complex tooling
- Can leverage expertise for better error handling, type design, async patterns
- Write idiomatic code from day 1

### 2. TUI Dogfooding = Product Development

This transforms the project from "script replacement" to "product development":

- [COMPLETE] **Stress-test** the TUI crate with real-world complexity
- [COMPLETE] **Create a showcase** for potential users
- [COMPLETE] **Discover pain points** in the API before users do
- [COMPLETE] **Build a reference implementation** demonstrating best practices

**Example TUI Use Cases:**

```rust
// Multi-pane watch dashboard (using pty_mux)
$ r3bl-build dev-dashboard
┌─────────────────────┬─────────────────────┐
│ bacon test          │ bacon clippy        │
│ ✓ 342 tests passed  │ ⚠ 3 warnings        │
├─────────────────────┼─────────────────────┤
│ cargo doc           │ ./check.fish        │
│ 📚 Generating...    │ ✓ All checks passed │
└─────────────────────┴─────────────────────┘

// Interactive example runner with fuzzy search
$ r3bl-build run-examples
╔══════════════════════════════════════════╗
║ Select Example (type to filter):        ║
╠══════════════════════════════════════════╣
║ > demo_1                                 ║
║   demo_2_complex_layout                  ║
║   flamegraph_profiling                   ║
║   async_task_runner                      ║
╚══════════════════════════════════════════╝
```

### 3. Open Source Product Potential

**Competitive Analysis:**

- [`cargo-xtask`](https://github.com/matklad/cargo-xtask) - Simple, no TUI
- [`cargo-make`](https://github.com/sagiegurari/cargo-make) - TOML-based, no TUI
- [`just`](https://github.com/casey/just) - Make-like, no TUI

**Our Differentiator:** Rich TUI experience + Rust-native + workspace-aware

Can extract generic parts into `cargo-xtask-tui` crate - **first to market** with this approach.

### 4. Infrastructure Already Prepared

Existing modules built in preparation:

- [COMPLETE] **`script/` module:** Subprocess management, shell execution
- [COMPLETE] **`pty_mux`:** Multiplexed PTY handling for parallel tasks

**Example Usage:**

```rust
use r3bl_tui::pty_mux::PtySession;
use r3bl_tui::script::CommandBuilder;

async fn run_dev_dashboard() {
    let mut mux = PtyMux::new();

    // Top-left: tests
    mux.spawn_pane(CommandBuilder::new("bacon")
        .arg("test")
        .arg("--headless"));

    // Top-right: clippy
    mux.spawn_pane(CommandBuilder::new("bacon")
        .arg("clippy"));

    // Bottom-left: docs
    mux.spawn_pane(CommandBuilder::new("bacon")
        .arg("doc"));

    // Bottom-right: health check
    mux.spawn_pane(CommandBuilder::new("watch")
        .arg("-n")
        .arg("60")
        .arg("./check.fish"));

    mux.run_all().await?;
}
```

---

## Architecture

### Project Structure

```
build-infra-tools/
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs              # Entry point, clap CLI
│   ├── lib.rs               # Reusable library
│   ├── commands/            # Command implementations
│   │   ├── mod.rs
│   │   ├── build.rs         # cargo build wrapper
│   │   ├── test.rs          # nextest + doctest
│   │   ├── watch.rs         # File watching
│   │   ├── toolchain.rs     # Rust toolchain mgmt
│   │   ├── clippy.rs        # Linting operations
│   │   ├── docs.rs          # Documentation generation
│   │   ├── examples.rs      # Example runner
│   │   ├── docker.rs        # Docker operations
│   │   └── dashboard.rs     # TUI dashboard (pty_mux)
│   ├── tui/                 # TUI components
│   │   ├── mod.rs
│   │   ├── example_picker.rs   # Fuzzy search for examples
│   │   ├── log_viewer.rs       # Real-time log streaming
│   │   └── dashboard_layout.rs # 4-pane layout
│   └── utils/               # Shared utilities
│       ├── cargo.rs         # Cargo metadata parsing
│       ├── process.rs       # Process management
│       └── file_watch.rs    # File system watching
└── tests/
    ├── integration/
    │   ├── test_build.rs
    │   ├── test_toolchain.rs
    │   └── test_commands.rs
    └── fixtures/
        └── sample_workspace/
```

### Dependency Stack

```toml
[dependencies]
# Dogfood our own crates 🐶
r3bl_tui = { path = "../tui" }
r3bl_core = { path = "../core", optional = true }

# CLI & Config
clap = { version = "4", features = ["derive", "cargo"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"

# Async runtime (for pty_mux)
tokio = { version = "1", features = ["full"] }

# Process management
which = "6"                 # Find binaries in PATH
cargo_metadata = "0.18"     # Parse Cargo.toml/workspace
notify = "6"                # File system watching

# Error handling
anyhow = "1"
thiserror = "1"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

[dev-dependencies]
assert_cmd = "2"            # CLI testing
predicates = "3"            # Assertions for CLI tests
tempfile = "3"              # Temporary directories
```

---

## Solving the Chicken-Egg Problem

### Option A: Bootstrap Script [COMPLETE] **RECOMMENDED**

Keep a minimal `bootstrap.fish` that ONLY:

1. Checks if `r3bl-build` binary exists
2. If not: `cargo build --release --package build-infra-tools`
3. Copies binary to `./bin/r3bl-build`
4. All future operations use `./bin/r3bl-build`

**Implementation:**

```fish
#!/usr/bin/env fish
# bootstrap.fish - ONLY builds the build tool

if not test -f ./bin/r3bl-build
    echo "Building r3bl-build for the first time..."
    cargo build --release --package build-infra-tools
    mkdir -p ./bin
    cp target/release/r3bl-build ./bin/
end

./bin/r3bl-build $argv
```

**Usage:**

```bash
# First time (or after changes to build-infra-tools)
./bootstrap.fish install-cargo-tools

# After that, use the binary directly
./bin/r3bl-build all
./bin/r3bl-build dev-dashboard
```

---

### Option B: Checked-In Pre-Built Binaries

```
bin/
├── r3bl-build-x86_64-linux
├── r3bl-build-aarch64-linux
└── r3bl-build-x86_64-macos
```

**Usage:**

```bash
./bin/r3bl-build-$(uname -m)-$(uname -s | tr '[:upper:]' '[:lower:]') all
```

**Pros:**

- No bootstrap needed
- Instant execution

**Cons:**

- Must maintain binaries for multiple platforms
- Large binary sizes in git (consider Git LFS)
- Unusual pattern (though `rustc` itself uses this)

---

### Option C: `cargo-xtask` Pattern

```
xtask/          # Not in workspace, separate Cargo.toml
├── Cargo.toml  # Minimal deps, fast to compile
└── src/
    └── main.rs
```

**Usage:**

```bash
cargo xtask install-cargo-tools
cargo xtask dev-dashboard
```

**Pros:**

- Idiomatic Rust pattern
- Used by rust-analyzer, tokio, etc.
- No bootstrap script needed

**Cons:**

- Separate from workspace
- Harder to share code with main crates

**Precedents:**

- [rust-analyzer](https://github.com/rust-lang/rust-analyzer/tree/master/xtask)
- [tokio](https://github.com/tokio-rs/tokio/tree/master/xtask)

---

## Step 0: Proof of Concept [PENDING]

**Goal:** Validate approach and TUI integration

**Tasks:**

- [ ] Set up `build-infra-tools` crate structure
- [ ] Implement basic CLI with `clap`
- [ ] Port 3-5 critical commands:
  - [ ] `build` - Basic cargo build
  - [ ] `test` - Run tests
  - [ ] `clippy` - Linting
  - [ ] `install-cargo-tools` - Tool installation
  - [ ] `run-examples` - Example execution
- [ ] **Build ONE TUI feature:** Interactive example picker with fuzzy search
- [ ] Create bootstrap script
- [ ] Write basic integration tests

**Success Criteria:**

- Fish → Rust parity for core commands
- One delightful TUI experience working
- Can build and run via bootstrap script

---

## Step 1: Feature Parity [PENDING]

**Goal:** Complete migration from Fish scripts

**Tasks:**

- [ ] Port remaining 25+ commands from `run.fish`
- [ ] Port utility functions from `script_lib.fish`
- [ ] Implement toolchain management commands
- [ ] Add Docker operations
- [ ] Implement profiling/flamegraph commands
- [ ] Comprehensive error handling with `anyhow`/`thiserror`
- [ ] Unit tests for all modules
- [ ] Integration tests for command execution
- [ ] Documentation for all commands

**Success Criteria:**

- Complete feature parity with Fish scripts
- All tests passing
- Can deprecate Fish scripts

---

## Step 2: TUI Excellence [PENDING]

**Goal:** Create delightful user experiences

**Tasks:**

- [ ] **Dev Dashboard** using `pty_mux`:
  - [ ] 4-pane layout (nextest, clippy, docs, health check)
  - [ ] Real-time output streaming
  - [ ] Keyboard navigation between panes
  - [ ] Pane resizing
- [ ] **Enhanced Example Picker:**
  - [ ] Fuzzy search with highlighting
  - [ ] Preview of example description
  - [ ] Recent examples history
- [ ] **Interactive Log Viewer:**
  - [ ] Syntax highlighting for Rust logs
  - [ ] Log level filtering
  - [ ] Search functionality
  - [ ] Follow mode (tail -f)
- [ ] **Progress Indicators:**
  - [ ] Rich progress bars for builds
  - [ ] Spinners for long operations
  - [ ] Color-coded status messages
- [ ] **Interactive Command Palette:**
  - [ ] Fuzzy search all commands
  - [ ] Command history
  - [ ] Favorites/bookmarks

**Success Criteria:**

- TUI features are more delightful than CLI equivalents
- No regressions in functionality
- Positive user feedback (internal team testing)

---

## Step 3: Open Source Product [PENDING]

**Goal:** Extract and publish reusable components

**Tasks:**

- [ ] Identify generic vs project-specific code
- [ ] Extract to `cargo-xtask-tui` crate:
  - [ ] Core abstractions for task definitions
  - [ ] TUI components (dashboard, pickers, viewers)
  - [ ] Process management utilities
  - [ ] File watching utilities
- [ ] Create comprehensive documentation:
  - [ ] API documentation
  - [ ] User guide
  - [ ] Tutorial: "Building Your First TUI Build Tool"
  - [ ] Examples for common use cases
- [ ] Polish and testing:
  - [ ] 100% public API documentation
  - [ ] Example projects using the crate
  - [ ] Performance benchmarks
- [ ] Marketing:
  - [ ] Blog post: "Building Delightful Build Tools with Rust TUI"
  - [ ] Reddit post to /r/rust
  - [ ] Submit to This Week in Rust
  - [ ] HN launch post

**Success Criteria:**

- Published to crates.io
- Positive community reception
- At least 2-3 external projects using it

---

## Command Migration Map

### From `run.fish` → `r3bl-build`

| Fish Command                   | Rust Command                    | Priority | Complexity |
| ------------------------------ | ------------------------------- | -------- | ---------- |
| `all`                          | `r3bl-build all`                | P0       | Medium     |
| `build`                        | `r3bl-build build`              | P0       | Low        |
| `test_workspace`               | `r3bl-build test`               | P0       | Low        |
| `clippy`                       | `r3bl-build clippy`             | P0       | Medium     |
| `docs`                         | `r3bl-build docs`               | P0       | Low        |
| `install-cargo-tools`          | `r3bl-build install-tools`      | P0       | High       |
| `toolchain-update`             | `r3bl-build toolchain update`   | P0       | High       |
| `toolchain-sync`               | `r3bl-build toolchain sync`     | P0       | High       |
| `toolchain-validate`           | `r3bl-build toolchain validate` | P0       | Medium     |
| `run-examples`                 | `r3bl-build examples run` (TUI) | P1       | High       |
| `run-binaries`                 | `r3bl-build binaries run` (TUI) | P1       | Medium     |
| `dev-dashboard`                | `r3bl-build dashboard` (TUI)    | P1       | High       |
| `log`                          | `r3bl-build log` (TUI)          | P1       | Medium     |
| `watch-all-tests`              | `r3bl-build watch tests`        | P2       | Medium     |
| `watch-clippy`                 | `r3bl-build watch clippy`       | P2       | Medium     |
| `watch-check`                  | `r3bl-build watch check`        | P2       | Medium     |
| `bench`                        | `r3bl-build bench`              | P2       | Low        |
| `run-examples-flamegraph-svg`  | `r3bl-build flamegraph svg`     | P2       | High       |
| `run-examples-flamegraph-fold` | `r3bl-build flamegraph fold`    | P2       | High       |
| `docker-build`                 | `r3bl-build docker build`       | P2       | Medium     |
| `build-server`                 | `r3bl-build sync`               | P3       | Medium     |
| `upgrade-deps`                 | `r3bl-build upgrade-deps`       | P3       | Low        |
| `audit-deps`                   | `r3bl-build audit`              | P3       | Low        |
| `unmaintained-deps`            | `r3bl-build unmaintained`       | P3       | Low        |
| `rustfmt`                      | `r3bl-build fmt`                | P3       | Low        |
| `serve-docs`                   | `r3bl-build docs serve`         | P3       | Low        |

---

## Example TUI Implementations

### 1. Interactive Example Picker

```rust
use r3bl_tui::prelude::*;

pub struct ExamplePicker {
    examples: Vec<Example>,
    filter: String,
    selected: usize,
}

impl ExamplePicker {
    pub fn run() -> Result<Option<Example>> {
        let examples = discover_examples()?;
        let mut picker = Self {
            examples,
            filter: String::new(),
            selected: 0,
        };

        // TUI event loop
        picker.render_loop()
    }

    fn filtered_examples(&self) -> Vec<&Example> {
        self.examples
            .iter()
            .filter(|e| e.name.contains(&self.filter))
            .collect()
    }
}
```

---

### 2. Multi-Pane Dashboard

```rust
use r3bl_tui::pty_mux::{PtyMux, PaneConfig};

pub async fn run_dev_dashboard() -> Result<()> {
    let mut mux = PtyMux::new()
        .with_layout(Layout::Grid { rows: 2, cols: 2 });

    // Top-left: Continuous testing
    mux.add_pane(PaneConfig {
        title: "Tests".into(),
        command: "bacon".into(),
        args: vec!["test".into(), "--headless".into()],
        auto_restart: true,
    });

    // Top-right: Linting
    mux.add_pane(PaneConfig {
        title: "Clippy".into(),
        command: "bacon".into(),
        args: vec!["clippy".into(), "--headless".into()],
        auto_restart: true,
    });

    // Bottom-left: Documentation
    mux.add_pane(PaneConfig {
        title: "Docs".into(),
        command: "bacon".into(),
        args: vec!["doc".into(), "--headless".into()],
        auto_restart: true,
    });

    // Bottom-right: Health check
    mux.add_pane(PaneConfig {
        title: "Health".into(),
        command: "watch".into(),
        args: vec!["-n".into(), "60".into(), "./check.fish".into()],
        auto_restart: true,
    });

    mux.run().await
}
```

---

### 3. Log Viewer with Filtering

```rust
use r3bl_tui::prelude::*;

pub struct LogViewer {
    logs: Vec<LogEntry>,
    filter_level: LogLevel,
    follow_mode: bool,
    search_term: String,
}

impl LogViewer {
    pub fn watch(path: &Path) -> Result<()> {
        let mut viewer = Self {
            logs: Vec::new(),
            filter_level: LogLevel::Debug,
            follow_mode: true,
            search_term: String::new(),
        };

        // Watch file for changes
        let watcher = FileWatcher::new(path)?;

        // TUI event loop with live updates
        viewer.render_loop_with_watcher(watcher)
    }

    fn filtered_logs(&self) -> impl Iterator<Item = &LogEntry> {
        self.logs
            .iter()
            .filter(|e| e.level >= self.filter_level)
            .filter(|e| {
                self.search_term.is_empty()
                    || e.message.contains(&self.search_term)
            })
    }
}
```

---

## Future `cargo-xtask-tui` API

**Vision:** Make it trivial for any Rust project to add rich TUI build tools.

```rust
// User's xtask/src/main.rs
use cargo_xtask_tui::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    XTask::new("my-project")
        // Simple commands
        .command(
            "build",
            Command::cargo("build").with_description("Build the project")
        )
        .command(
            "test",
            Command::cargo("test").with_description("Run all tests")
        )

        // TUI commands
        .tui_command(
            "examples",
            TuiCommand::example_picker()
                .with_preview()
                .with_fuzzy_search()
        )

        // Dashboard
        .dashboard(|d| {
            d.pane("tests", "cargo test --all-targets")
             .pane("clippy", "cargo clippy --workspace")
             .pane("docs", "cargo doc --no-deps")
             .pane("watch", "watch -n 60 ./check.sh")
        })

        // Run the CLI
        .run()
        .await
}
```

**Features to provide:**

- [COMPLETE] Example picker component
- [COMPLETE] Log viewer component
- [COMPLETE] Multi-pane dashboard with `pty_mux`
- [COMPLETE] Progress indicators and spinners
- [COMPLETE] Fuzzy search utilities
- [COMPLETE] File watching utilities
- [COMPLETE] Process management utilities
- [COMPLETE] Common command patterns (cargo, rustup, etc.)

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_examples() {
        let examples = discover_examples().unwrap();
        assert!(!examples.is_empty());
    }

    #[test]
    fn test_example_filtering() {
        let picker = ExamplePicker::new();
        picker.set_filter("demo");
        assert!(picker.filtered_examples()
            .all(|e| e.name.contains("demo")));
    }
}
```

### Integration Tests

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_build_command() {
    Command::cargo_bin("r3bl-build")
        .unwrap()
        .arg("build")
        .assert()
        .success()
        .stdout(predicate::str::contains("Finished"));
}

#[test]
fn test_toolchain_validate() {
    Command::cargo_bin("r3bl-build")
        .unwrap()
        .args(&["toolchain", "validate", "quick"])
        .assert()
        .success();
}
```

### TUI Testing

```rust
// Mock terminal for TUI testing
#[test]
fn test_example_picker_navigation() {
    let mut terminal = MockTerminal::new();
    let picker = ExamplePicker::new();

    // Simulate key presses
    terminal.send_key(Key::Down);
    terminal.send_key(Key::Down);
    terminal.send_key(Key::Enter);

    let selected = picker.get_selected();
    assert_eq!(selected.unwrap().name, "example_2");
}
```

---

## Migration Timeline

### Week 1: POC

- Days 1-2: Project setup, bootstrap script
- Days 3-4: Port core commands (build, test, clippy)
- Day 5: Build example picker TUI
- Weekend: Testing and refinement

### Week 2: Feature Parity

- Days 1-2: Port toolchain management
- Days 3-4: Port remaining commands
- Day 5: Comprehensive testing
- Weekend: Documentation

### Week 3: TUI Excellence

- Days 1-2: Multi-pane dashboard
- Days 3-4: Log viewer and command palette
- Day 5: Polish and bug fixes
- Weekend: User testing (internal team)

### Weeks 4-5: Product Extraction

- Days 1-3: Identify and extract generic code
- Days 4-7: Create cargo-xtask-tui crate
- Days 8-10: Documentation and examples
- Weekend: Prep for launch

### Week 6: Launch

- Days 1-2: Final testing
- Day 3: Publish to crates.io
- Days 4-5: Marketing (blog posts, Reddit, HN)

---

## Success Metrics

### Technical

- [ ] 100% feature parity with Fish scripts
- [ ] All tests passing (unit + integration)
- [ ] No performance regressions
- [ ] Binary size < 20MB
- [ ] Compilation time < 2 minutes

### User Experience

- [ ] Positive feedback from team members
- [ ] TUI features used regularly
- [ ] Faster than Fish equivalents (subjective)
- [ ] No major bugs reported in first month

### Product

- [ ] `cargo-xtask-tui` published to crates.io
- [ ] 50+ GitHub stars in first month
- [ ] 2+ external projects using it
- [ ] Featured in This Week in Rust

---

## Risks and Mitigations

### Risk 1: TUI complexity slows development

**Mitigation:** Phased approach - get CLI working first, add TUI incrementally

### Risk 2: pty_mux not mature enough

**Mitigation:** This is actually a benefit - we'll discover issues and fix them

### Risk 3: Team resistance to new tool

**Mitigation:** Keep Fish scripts during transition, allow parallel usage

### Risk 4: Bootstrap script too complex

**Mitigation:** Keep it minimal - just build and copy binary

### Risk 5: Product extraction takes too long

**Mitigation:** Phase 4 is optional - we get value even if we don't extract

---

## Open Questions

- [ ] Should we use `cargo-xtask` pattern or standalone crate?
- [ ] Which TUI features should be prioritized?
- [ ] Should we check in pre-built binaries or rely on bootstrap?
- [ ] What's the minimum viable TUI experience for Phase 1?
- [ ] How much of `script_lib.fish` logic needs porting?
- [ ] Should we support non-interactive mode for CI/CD?
- [ ] What should the default behavior be (TUI vs CLI)?

---

## Next Steps

1. **Decision:** Choose bootstrap strategy (Option A recommended)
2. **Scaffold:** Create `build-infra-tools` crate structure
3. **POC:** Implement 3-5 core commands + example picker
4. **Validate:** Ensure TUI integration works smoothly
5. **Iterate:** Gather feedback and adjust plan

---

## References

- [cargo-xtask pattern](https://github.com/matklad/cargo-xtask)
- [rust-analyzer xtask](https://github.com/rust-lang/rust-analyzer/tree/master/xtask)
- [tokio xtask](https://github.com/tokio-rs/tokio/tree/master/xtask)
- [cargo-make](https://github.com/sagiegurari/cargo-make)
- [just task runner](https://github.com/casey/just)

---

## Conclusion

This is an opportunity to:

1. **Solve a real problem:** Complex Fish scripts are unmaintainable
2. **Dogfood our technology:** Stress-test and showcase `r3bl_tui`
3. **Create value:** Build something genuinely useful for the Rust community
4. **Demonstrate expertise:** Show what's possible with Rust TUI

**Let's build this.** [COMPLETE]
