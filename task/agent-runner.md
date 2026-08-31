# Task: Agent Runner Coding Harness Orchestrator

<!-- prettier-ignore-start -->
<!-- BEGIN mktoc -->

- [Overview](#overview)
    - [Vision](#vision)
    - [Evolution from Chi](#evolution-from-chi)
    - [Core Value Proposition](#core-value-proposition)
- [Architecture & Mental Model](#architecture--mental-model)
    - [Terminal Layout & Workspace Modes](#terminal-layout--workspace-modes)
    - [Sidequest Branching Mental Model](#sidequest-branching-mental-model)
    - [Context Scraping & Transcript Pipeline](#context-scraping--transcript-pipeline)
- [Core Components & Subsystems](#core-components--subsystems)
    - [1. PTY Process Orchestrator](#1-pty-process-orchestrator)
    - [2. Tabbed Terminal Multiplexer](#2-tabbed-terminal-multiplexer)
    - [3. Rich Markdown Prompt Composer](#3-rich-markdown-prompt-composer)
    - [4. Context Extractor & Transcript Engine](#4-context-extractor--transcript-engine)
    - [5. Session Cloner & Sidequest Coordinator](#5-session-cloner--sidequest-coordinator)
- [Implementation Plan](#implementation-plan)
    - [Phase 1: Agent Harness Configuration & PTY Session Wrapper](#phase-1-agent-harness-configuration--pty-session-wrapper)
    - [Phase 2: Tabbed Multiplexer & Process Indicators](#phase-2-tabbed-multiplexer--process-indicators)
    - [Phase 3: Rich Markdown Prompt Composer](#phase-3-rich-markdown-prompt-composer)
    - [Phase 4: Terminal Scraping & Transcript Engine](#phase-4-terminal-scraping--transcript-engine)
    - [Phase 5: Session Cloning & Ephemeral Sidequests](#phase-5-session-cloning--ephemeral-sidequests)
    - [Phase 6: CLI Binary, Integration & Verification](#phase-6-cli-binary-integration--verification)

<!-- END mktoc -->
<!-- prettier-ignore-end -->

## Overview

### Vision

`agent-runner` is a specialized terminal multiplexer and orchestration workbench for
command-line coding agents (`agy-cli`, `claude-code`, etc.), implemented as a binary in
`cmdr`. It eliminates the limitations of primitive terminal input prompts by providing a
dedicated Markdown composition environment, multi-tab session management, context
scraping, and non-destructive experimentation branches ("sidequests").

### Evolution from Chi

The legacy `chi` initiative ([`prd_chi.md`]) explored integrating helper tools around
Claude Code by reading local SQLite history. `agent-runner` supersedes `chi` with a
universal, harness-agnostic architecture:

1. **Harness Neutrality**: Orchestrates any terminal agent (`agy-cli`, `claude-code`, or
   custom agent binaries) running inside a managed PTY.
2. **First-Class Tabs**: Native terminal tabs powered by [`PTYMux`] and [`ProcessManager`].
3. **Structured Prompt Composition**: Full markdown authoring via [`EditorComponent`]
   before transmitting payloads to the active agent.
4. **Context Cloning (Sidequests)**: Instant creation of ephemeral child sessions to test
   hypotheses without polluting the parent conversation history.

### Core Value Proposition

- **No More Terminal Input Frustration**: Craft multi-paragraph, bulleted instructions
  with code snippets using the full editing power of [`edi`].
- **Parallel Exploration**: Run long-running agent tasks in background tabs while
  interacting with another session.
- **Risk-Free Branching**: Test speculative instructions or alternate refactoring plans
  in temporary sidequest tabs.
- **Clean Context Captures**: Automatic scraping of terminal canvas output and user prompts
  into structured conversation transcripts.

---

## Architecture & Mental Model

### Terminal Layout & Workspace Modes

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ [1: agy-cli (main)]*   [2: sidequest-borrow-fix]   [+] New Tab   [?] Help    │
├─────────────────────────────────────────────────────────────────────────────┤
│ Headless VT-100 Terminal Display (Active Tab Canvas)                        │
│                                                                             │
│ > Antigravity CLI v2.0                                                      │
│ > Analyzing repository structure...                                         │
│ > Modified 3 files in tui/src/core/pty/                                     │
│ > Running cargo check... [OK]                                               │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│ Rich Prompt Composer (Toggleable Slide-Over / Split View)                   │
│                                                                             │
│ Please refactor `process_manager.rs` to implement graceful child termination│
│ following these requirements:                                               │
│ - Send SIGTERM first with 2-second deadline                                 │
│ - Escalate to SIGKILL if the process fails to exit                          │
│                                                                             │
│ [Ctrl+Enter] Submit Prompt   [Esc] Hide Composer   [Alt+1..9] Switch Tab    │
╰─────────────────────────────────────────────────────────────────────────────╯
```

### Sidequest Branching Mental Model

```text
┌──────────────────────────────────────┐
│ Main Session Tab                     │
│ Context: Clean, focused on goal      │
└──────────────────┬───────────────────┘
                   │ User hits [Ctrl+B] (Spawn Sidequest)
                   ▼
┌──────────────────────────────────────┐
│ Cloned Ephemeral Session Tab         │
│ Context: Seeded from checkpoint      │
│ Purpose: Tangent / hypothesis check  │
└──────────────────┬───────────────────┘
                   │
         ┌─────────┴─────────┐
         ▼                   ▼
┌─────────────────┐ ┌─────────────────┐
│ Discard / Close │ │ Copy Key Learn  │
│ (Zero pollution)│ │ Back to Parent  │
└─────────────────┘ └─────────────────┘
```

### Context Scraping & Transcript Pipeline

```text
PTY Child Process (e.g., agy-cli)
       │
       ▼ Raw stdout / ANSI stream
OfsBufVT100 Headless Canvas
       │
       ▼ Canvas line scraping (ANSI stripping)
Transcript Engine ◄── User Prompts from Markdown Composer
       │
       ▼
Session Transcript Archive (~/.local/share/r3bl-cmdr/sessions/<id>/)
```

---

## Core Components & Subsystems

### 1. PTY Process Orchestrator

- **Module**: `cmdr/src/agent_runner/orchestrator/`
- Wraps [`pty_session`] to spawn agent CLI harnesses in dedicated pseudo-terminals.
- Manages window size synchronization (`SIGWINCH`), raw terminal mode toggling, and
  clean signal forwarding (`SIGINT`, `SIGTERM`).
- Supports configurable harness presets:
  - `agy-cli`: Antigravity CLI executable with flags.
  - `claude-code`: Claude Code executable with flags.
  - `custom`: User-specified binary and arguments.

### 2. Tabbed Terminal Multiplexer

- **Module**: `cmdr/src/agent_runner/mux/`
- Built on [`PTYMux`] and [`ProcessManager`].
- Each tab maintains an independent [`OfsBufVT100`] canvas.
- Tab bar shows:
  - Tab index (1..9 for direct `Alt+N` switching).
  - Process label and icon.
  - Live execution indicator (`● Running`, `✓ Idle`, `! Alert`).
- Instant switching between tabs with zero visual flicker.

### 3. Rich Markdown Prompt Composer

- **Module**: `cmdr/src/agent_runner/composer/`
- Leverages [`EditorComponent`] and syntax themes from [`edi`].
- Provides dedicated multi-line editing with modal navigation, text selection, and undo.
- Template injection engine:
  - Quick insert for git diff snapshots.
  - Quick insert for file contents (`@filename`).
  - Standard prompt scaffolds (refactor, bugfix, review).
- Submission router:
  - Formats markdown text into single-line escaped strings or bracketed paste sequences.
  - Transmits payload to the active tab's PTY input channel.

### 4. Context Extractor & Transcript Engine

- **Module**: `cmdr/src/agent_runner/context/`
- Extracts visible text rows from the active canvas buffer (`OfsBuf`).
- Correlates submitted prompt text with subsequent agent responses.
- Generates clean Markdown transcripts stripped of raw terminal escape codes.
- Saves session state to `~/.local/share/r3bl-cmdr/agent_sessions/<session_id>/`.

### 5. Session Cloner & Sidequest Coordinator

- **Module**: `cmdr/src/agent_runner/sidequest/`
- Captures current session state (harness configuration, working directory, transcript
  snapshot).
- Spawns a new PTY session in a dedicated tab flagged as `Ephemeral`.
- Seeds the child agent with context from the parent session if supported by the harness
  (e.g., initial prompt with state summary).
- Closing an ephemeral tab cleans up all temporary state without affecting the primary
  session.

---

## Implementation Plan

### Phase 1: Agent Harness Configuration & PTY Session Wrapper

- [ ] Define `HarnessConfig` and `AgentLauncher` in `cmdr/src/agent_runner/orchestrator/`.
- [ ] Implement support for `agy-cli` and `claude-code` profile auto-detection.
- [ ] Wrap `r3bl_tui::core::pty_session` with lifecycle monitoring and exit status capture.
- [ ] Add unit tests for harness command-line construction and environment propagation.
- [ ] Mandatory manual review:
    - [ ] `cmdr/src/agent_runner/orchestrator/config.rs`
    - [ ] `cmdr/src/agent_runner/orchestrator/launcher.rs`
    - [ ] `cmdr/src/agent_runner/orchestrator/session_wrapper.rs`

### Phase 2: Tabbed Multiplexer & Process Indicators

- [ ] Adapt [`PTYMux`] and [`ProcessManager`] for multi-agent session lifecycle.
- [ ] Implement `TabBarComponent` rendering tab labels, active indicators, and shortcut hints.
- [ ] Implement keyboard shortcuts:
    - [ ] `Alt+1` through `Alt+9`: Direct tab selection.
    - [ ] `Ctrl+T`: Create new session tab.
    - [ ] `Ctrl+W`: Close active session tab.
- [ ] Add integration test verifying multi-tab concurrent output isolation.
- [ ] Mandatory manual review:
    - [ ] `cmdr/src/agent_runner/mux/tab_manager.rs`
    - [ ] `cmdr/src/agent_runner/mux/tab_bar_component.rs`
    - [ ] `cmdr/src/agent_runner/mux/input_router.rs`

### Phase 3: Rich Markdown Prompt Composer

- [ ] Embed `EditorComponent` into a toggleable bottom/overlay split pane.
- [ ] Add keybinding `Ctrl+Space` or `Ctrl+I` to open and focus the prompt composer.
- [ ] Implement template expansion engine (git status, file injection, system prompts).
- [ ] Implement prompt transmission via bracketed paste or sanitized PTY write on `Ctrl+Enter`.
- [ ] Add unit tests for prompt formatting and escape code handling.
- [ ] Mandatory manual review:
    - [ ] `cmdr/src/agent_runner/composer/mod.rs`
    - [ ] `cmdr/src/agent_runner/composer/editor_pane.rs`
    - [ ] `cmdr/src/agent_runner/composer/template_engine.rs`

### Phase 4: Terminal Scraping & Transcript Engine

- [ ] Implement canvas line scraper reading from `OfsBufVT100`.
- [ ] Implement ANSI stripper and text normalizer.
- [ ] Implement `TranscriptRecorder` pairing composed prompts with agent terminal output.
- [ ] Add disk persistence writing JSONL and Markdown transcripts per session.
- [ ] Mandatory manual review:
    - [ ] `cmdr/src/agent_runner/context/canvas_scraper.rs`
    - [ ] `cmdr/src/agent_runner/context/transcript.rs`
    - [ ] `cmdr/src/agent_runner/context/storage.rs`

### Phase 5: Session Cloning & Ephemeral Sidequests

- [ ] Implement `SidequestCoordinator` to snapshot active session parameters.
- [ ] Implement `Ctrl+B` action to spawn a sidequest tab seeded with parent context.
- [ ] Implement sidequest close workflow with prompt to copy key findings back to parent.
- [ ] Add automated test simulating sidequest creation, execution, and cleanup.
- [ ] Mandatory manual review:
    - [ ] `cmdr/src/agent_runner/sidequest/coordinator.rs`
    - [ ] `cmdr/src/agent_runner/sidequest/state_snapshot.rs`

### Phase 6: CLI Binary, Integration & Verification

- [ ] Add `agent-runner` binary definition to `cmdr/Cargo.toml`.
- [ ] Implement CLI argument parsing (`agent-runner [--harness <name>] [WORKING_DIR]`).
- [ ] Run `./check.fish --check` across the workspace.
- [ ] Run `./check.fish --clippy` across the workspace.
- [ ] Run `./check.fish --test` across the workspace.
- [ ] Run `./check.fish --quick-doc` to verify rustdoc navigation.
- [ ] Perform interactive smoke test running an agent CLI inside `agent-runner`.
- [ ] Mandatory manual review:
    - [ ] Complete modified file checklist across all crates.

[`prd_chi.md`]: file:///home/nazmul/github/roc/task/pending/prd_chi.md
[`PTYMux`]: file:///home/nazmul/github/roc/tui/src/core/pty/pty_mux/mux.rs
[`ProcessManager`]: file:///home/nazmul/github/roc/tui/src/core/pty/pty_mux/process_manager.rs
[`EditorComponent`]: file:///home/nazmul/github/roc/tui/src/core/tui_core/tui_style/mod.rs
[`edi`]: file:///home/nazmul/github/roc/cmdr/src/edi/mod.rs
[`pty_session`]: file:///home/nazmul/github/roc/tui/src/core/pty/pty_session/mod.rs
[`OfsBufVT100`]: file:///home/nazmul/github/roc/tui/src/core/pty/pty_mux/mod.rs
