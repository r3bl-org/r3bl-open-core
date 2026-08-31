# Task: Interactive Rust Dojo & r3bl-runner Notebook Engine

<!-- prettier-ignore-start -->
<!-- BEGIN mktoc -->

- [Overview](#overview)
    - [Vision](#vision)
    - [Dual-Layer Architecture](#dual-layer-architecture)
    - [Key Capabilities](#key-capabilities)
- [Mental Model & UI Layout](#mental-model--ui-layout)
    - [Split-Pane Terminal Workspace](#split-pane-terminal-workspace)
    - [Code Block Cell Lifecycle & Visual States](#code-block-cell-lifecycle--visual-states)
    - [Curriculum & Golden Master Storage Flow](#curriculum--golden-master-storage-flow)
- [Architecture & Design](#architecture--design)
    - [1. Notebook Parsing & Cell State (`r3bl-runner`)](#1-notebook-parsing--cell-state-r3bl-runner)
    - [2. Execution Engine (`r3bl-runner`)](#2-execution-engine-r3bl-runner)
    - [3. Notebook Editor Component (`rust-dojo`)](#3-notebook-editor-component-rust-dojo)
    - [4. Output Panel Component (`rust-dojo`)](#4-output-panel-component-rust-dojo)
    - [5. Lesson & Curriculum Manager (`rust-dojo`)](#5-lesson--curriculum-manager-rust-dojo)
- [Implementation Plan](#implementation-plan)
    - [Phase 1: Notebook Data Model & Markdown Cell Parser](#phase-1-notebook-data-model--markdown-cell-parser)
    - [Phase 2: Code Execution Engine & PTY Subprocess Runner](#phase-2-code-execution-engine--pty-subprocess-runner)
    - [Phase 3: Split-Pane UI & Cell Status Highlighting](#phase-3-split-pane-ui--cell-status-highlighting)
    - [Phase 4: Curriculum Engine & Golden Master Isolation](#phase-4-curriculum-engine--golden-master-isolation)
    - [Phase 5: CLI Entry Point & cmdr Integration](#phase-5-cli-entry-point--cmdr-integration)
    - [Phase 6: Verification & Quality Audit](#phase-6-verification--quality-audit)

<!-- END mktoc -->
<!-- prettier-ignore-end -->

## Overview

### Vision

`rust-dojo` is an interactive terminal learning and experimentation environment for Rust,
shipped as a binary in the `cmdr` crate. It provides an immediate, low cognitive load
playground for mastering Rust concepts (borrow checker katas, lifetime puzzles, async
patterns, trait design) directly within the developer's terminal.

### Dual-Layer Architecture

The implementation is split into two cleanly separated layers:

1. **`r3bl-runner` (Engine Layer)**: A reusable notebook execution engine. It parses
   Markdown documents into narrative text sections and executable code cells, manages
   execution states, and coordinates running code blocks via isolated subprocesses or
   REPLs.
2. **`rust-dojo` (Application Layer)**: The user-facing application in `cmdr`. It provides
   the split-pane TUI experience, curriculum management with isolated user workspaces, and
   keyboard navigation.

### Key Capabilities

- **Split-Pane TUI**: Interactive Markdown editor on the left; live, streaming execution
  output panel on the right.
- **Context-Aware Execution**: Place the caret in any `rust ` fenced code block and press
  `Ctrl+Enter` to compile and run that specific block.
- **Visual Block Status**: Real-time status indicators (unrun, running, success, failure)
  rendered on code block borders or gutters.
- **Live Output Streaming**: Compiler diagnostics and program stdout/stderr stream in
  real-time via a PTY session with full ANSI color fidelity.
- **Golden Master Curriculum**: Bundled lessons compiled into the binary via
  `include_str!`, cloned automatically to user storage to prevent accidental mutation of
  master files.
- **Freeform Mode**: Ability to open and run arbitrary external `.md` notebooks.

---

## Mental Model & UI Layout

### Split-Pane Terminal Workspace

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ rust-dojo: 01_ownership_basics.md                           [?] Help [q] Quit│
├─────────────────────────────────────────────┬───────────────────────────────┤
│ # Lesson 1: Understanding Ownership         │ Execution Output (PTY)        │
│                                             │                               │
│ In Rust, each value has an owner. When the  │ Compiling playground v0.1.0...│
│ owner goes out of scope, the value is       │    Finished dev [unoptimized] │
│ dropped automatically.                      │     Running `target/debug/...`│
│                                             │                               │
│ ┌─ [Rust Cell 1] ───────────────── [OK] ──┐ │ Hello, R3BL Dojo!             │
│ │ let s1 = String::from("hello");         │ │ s1 is: hello                  │
│ │ println!("s1 is: {}", s1);              │ │ s2 is: hello                  │
│ └─────────────────────────────────────────┘ │                               │
│                                             │ Process exited with code: 0   │
│ Now try to move s1 to s2 and print both:    │                               │
│                                             │                               │
│ ┌─ [Rust Cell 2: Caret Active] ── [FAIL] ─┐ │ Compiling playground v0.1.0...│
│ │ let s1 = String::from("hello");         │ │ error[E0382]: borrow of       │
│ │ let s2 = s1;                            │ │ moved value: `s1`             │
│ │ println!("s1: {}, s2: {}", s1, s2);     │ │  --> src/main.rs:4:26         │
│ └─────────────────────────────────────────┘ │                               │
├─────────────────────────────────────────────┴───────────────────────────────┤
│ [Ctrl+Enter] Run Cell   [Alt+Left/Right] Switch Pane   [Tab] Next Cell      │
╰─────────────────────────────────────────────────────────────────────────────╯
```

### Code Block Cell Lifecycle & Visual States

```text
                   ┌──────────────┐
                   │    Unrun     │ (Muted gray or cyan border)
                   └──────┬───────┘
                          │ User triggers [Ctrl+Enter]
                          ▼
                   ┌──────────────┐
                   │   Running    │ (Pulsing yellow border / spinner)
                   └──────┬───────┘
                          │ Process completes
             ┌────────────┴────────────┐
             ▼                         ▼
      ┌──────────────┐          ┌──────────────┐
      │   Success    │          │   Failure    │
      │ (Green / 0)  │          │ (Red / !0)   │
      └──────────────┘          └──────────────┘
```

### Curriculum & Golden Master Storage Flow

```text
┌───────────────────────────────┐
│ Embedded Binary Resources     │
│ (include_str! master lessons) │
└──────────────┬────────────────┘
               │ First access / clone
               ▼
┌───────────────────────────────┐
│ User Workspace Directory      │
│ ~/.local/share/r3bl-cmdr/dojo │
│ ├── 01_ownership.md (mutable) │
│ ├── 02_borrowing.md (mutable) │
│ └── ...                       │
└───────────────────────────────┘
```

---

## Architecture & Design

### 1. Notebook Parsing & Cell State (`r3bl-runner`)

- **Module**: `cmdr/src/rust_dojo/runner/model/`
- **Data Structures**:
    - `NotebookDocument`: Contains an ordered list of `NotebookBlock` elements.
    - `NotebookBlock`: Enum of `Text(String)` and `CodeCell(CodeCellState)`.
    - `CodeCellState`:
        - `id`: Unique identifier (UUID or cell index).
        - `language`: Target language tag (e.g., `"rust"`).
        - `code`: Mutable source text content.
        - `status`: `CellExecutionStatus` (`Unrun`, `Running`, `Success`,
          `Failure { exit_code: i32 }`).
        - `output_log`: Cached output text from the most recent run.
- **Markdown Parser**:
    - Leverages `pulldown-cmark` or existing R3BL parser to detect fenced code blocks.
    - Maps code block ranges to line indices within the editor buffer for seamless
      bi-directional synchronization.

### 2. Execution Engine (`r3bl-runner`)

- **Module**: `cmdr/src/rust_dojo/runner/engine/`
- **Trait Definition**:
    ```rust
    #[async_trait]
    pub trait CodeExecutionEngine: Send + Sync {
        async fn execute(
            &self,
            code: &str,
            output_channel: tokio::sync::mpsc::Sender<PtyOutputEvent>,
        ) -> miette::Result<CellExecutionStatus>;
    }
    ```
- **Cargo Subprocess Engine (Initial Implementation)**:
    - Spawns in a temporary directory on `tmpfs` (or system temp dir).
    - Scaffolds a minimal `Cargo.toml` and `src/main.rs`.
    - Automatically injects `fn main()` wrapper if the snippet contains only bare
      statements.
    - Executes `cargo run --color=always` inside an R3BL [`pty_session`].
    - Emits real-time output events to the channel for direct display in the output panel.
- **Future REPL Hook (Option C Hybrid)**:
    - Architectural abstraction allows plugging in an `evcxr` REPL backend without
      touching UI code.

### 3. Notebook Editor Component (`rust-dojo`)

- **Module**: `cmdr/src/rust_dojo/ui/editor_pane.rs`
- Integrates [`EditorComponent`] and [`EditorEngine`] from `r3bl_tui`.
- Renders Markdown syntax highlighting with custom block decorations:
    - Detects bounding lines of fenced code blocks.
    - Draws dynamic border styles and gutter indicators reflecting `CellExecutionStatus`.
    - Exposes keyboard action `ActionRunCurrentCell` bound to `Ctrl+Enter`.

### 4. Output Panel Component (`rust-dojo`)

- **Module**: `cmdr/src/rust_dojo/ui/output_pane.rs`
- Uses [`OfsBufVT100`] and PTY reader tasks to stream process output.
- Bypasses terminfo, rendering truecolor ANSI output directly to the pane canvas.
- Supports independent vertical scrolling and horizontal 2D panning.

### 5. Lesson & Curriculum Manager (`rust-dojo`)

- **Module**: `cmdr/src/rust_dojo/curriculum/`
- Embedded master files:
    - `01_ownership.md`
    - `02_borrowing.md`
    - `03_lifetimes.md`
    - `04_traits.md`
    - `05_async_basics.md`
- Resolves workspace directory (`dirs::data_local_dir() / r3bl-cmdr / dojo`).
- Clones master on demand if local working copy is missing.
- Saves working edits back to the workspace copy on cell execution or exit.

---

## Implementation Plan

### Phase 1: Notebook Data Model & Markdown Cell Parser

- [ ] Implement `NotebookDocument`, `NotebookBlock`, and `CodeCellState` in
      `cmdr/src/rust_dojo/runner/model/`.
- [ ] Implement markdown parser to partition markdown documents into text blocks and code
      cells.
- [ ] Implement serializer to save edited cells back to standard markdown format.
- [ ] Add unit tests verifying parsing and serialization round-trips.
- [ ] Mandatory manual review:
    - [ ] `cmdr/src/rust_dojo/runner/model/notebook.rs`
    - [ ] `cmdr/src/rust_dojo/runner/model/parser.rs`
    - [ ] `cmdr/src/rust_dojo/runner/model/serializer.rs`

### Phase 2: Code Execution Engine & PTY Subprocess Runner

- [ ] Define `CodeExecutionEngine` trait in `cmdr/src/rust_dojo/runner/engine/traits.rs`.
- [ ] Implement `CargoExecutionEngine` in
      `cmdr/src/rust_dojo/runner/engine/cargo_runner.rs`:
    - [ ] Temporary project directory scaffolding with cached build artifacts.
    - [ ] Automatic `fn main()` wrapper injection for bare statement blocks.
    - [ ] Execution via `r3bl_tui::core::pty_session` with real-time ANSI streaming.
- [ ] Implement cancel/interrupt support (e.g., `Ctrl+C` to terminate a runaway child
      process).
- [ ] Add integration tests running sample Rust code snippets via PTY.
- [ ] Mandatory manual review:
    - [ ] `cmdr/src/rust_dojo/runner/engine/traits.rs`
    - [ ] `cmdr/src/rust_dojo/runner/engine/cargo_runner.rs`
    - [ ] `cmdr/src/rust_dojo/runner/engine/temp_workspace.rs`

### Phase 3: Split-Pane UI & Cell Status Highlighting

- [ ] Implement two-pane layout container using `r3bl_tui::Layout`.
- [ ] Implement `NotebookEditorPane` wrapping `EditorComponent`:
    - [ ] Block detection and caret-to-cell mapping.
    - [ ] Custom styling for code block borders based on `CellExecutionStatus`.
    - [ ] Keybinding dispatcher for `Ctrl+Enter` (run active cell).
- [ ] Implement `OutputPane`:
    - [ ] Attach to PTY output event receiver.
    - [ ] Display compilation diagnostics and runtime output.
    - [ ] Add clear, scroll, and copy output actions.
- [ ] Mandatory manual review:
    - [ ] `cmdr/src/rust_dojo/ui/layout.rs`
    - [ ] `cmdr/src/rust_dojo/ui/editor_pane.rs`
    - [ ] `cmdr/src/rust_dojo/ui/output_pane.rs`

### Phase 4: Curriculum Engine & Golden Master Isolation

- [ ] Embed initial lesson files in `cmdr/src/rust_dojo/curriculum/lessons/`:
    - [ ] `01_ownership.md`
    - [ ] `02_borrowing.md`
    - [ ] `03_lifetimes.md`
- [ ] Implement `CurriculumManager`:
    - [ ] Resolve user local data directory.
    - [ ] Safe golden master cloning without overwriting existing user edits.
    - [ ] Lesson selection menu / list view.
- [ ] Add unit tests for workspace isolation and persistence.
- [ ] Mandatory manual review:
    - [ ] `cmdr/src/rust_dojo/curriculum/manager.rs`
    - [ ] `cmdr/src/rust_dojo/curriculum/lessons/`

### Phase 5: CLI Entry Point & cmdr Integration

- [ ] Add `rust-dojo` CLI binary definition in `cmdr/Cargo.toml`.
- [ ] Implement clap CLI parser (`rust-dojo [LESSON_OR_FILE]`).
- [ ] Connect application lifecycle and terminal raw mode management.
- [ ] Update `cmdr` README and documentation.
- [ ] Mandatory manual review:
    - [ ] `cmdr/src/bin/rust_dojo.rs`
    - [ ] `cmdr/src/rust_dojo/app.rs`
    - [ ] `cmdr/Cargo.toml`

### Phase 6: Verification & Quality Audit

- [ ] Run `./check.fish --check` across the workspace.
- [ ] Run `./check.fish --clippy` across the workspace.
- [ ] Run `./check.fish --test` to ensure all unit and integration tests pass.
- [ ] Run `./check.fish --quick-doc` to verify documentation builds with no broken links.
- [ ] Perform manual end-to-end interactive testing with sample katas.
- [ ] Mandatory manual review:
    - [ ] Complete modified file checklist across all crates.

[`pty_session`]: file:///home/nazmul/github/roc/tui/src/core/pty/pty_session/mod.rs
[`EditorComponent`]: file:///home/nazmul/github/roc/tui/src/core/tui_core/tui_style/mod.rs
[`EditorEngine`]: file:///home/nazmul/github/roc/tui/src/core/tui_core/tui_style/mod.rs
[`OfsBufVT100`]: file:///home/nazmul/github/roc/tui/src/core/pty/pty_mux/mod.rs
