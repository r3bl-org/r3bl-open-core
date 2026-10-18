# Task: Modernize choose API and readline onto Alternate Screen & Full TUI Primitives

<!-- BEGIN mktoc -->

- [Task: Modernize choose API and readline onto Alternate Screen & Full TUI Primitives](#task-modernize-choose-api-and-readline-onto-alternate-screen--full-tui-primitives)
- [Overview](#overview)
    - [Architectural Vision & The `fzf` Illusion](#architectural-vision--the-fzf-illusion)
    - [Roadmap Alignment & Dependencies](#roadmap-alignment--dependencies)
- [Design Specifications](#design-specifications)
    - [1. `choose` Component Design](#1-choose-component-design)
    - [2. `readline` App & Scrollback Design](#2-readline-app--scrollback-design)
    - [3. Full TUI Integration](#3-full-tui-integration)
- [Implementation Plan](#implementation-plan)
    - [Phase 1: Research & Virtual Scrollback Architecture](#phase-1-research--virtual-scrollback-architecture)
    - [Phase 2: Refactor `choose` to Alternate-Screen Component](#phase-2-refactor-choose-to-alternate-screen-component)
    - [Phase 3: Refactor `readline` to Alternate-Screen App](#phase-3-refactor-readline-to-alternate-screen-app)
    - [Phase 4: Cleanup, Deprecate Legacy Engines, & Validation](#phase-4-cleanup-deprecate-legacy-engines--validation)
- [Success Criteria](#success-criteria)

<!-- END mktoc -->

---

## Overview

Modernize the `choose` and `readline` APIs, which are the primary interactive entry points
into the `r3bl_tui` framework. Currently, both of these APIs maintain their own bespoke
cursor-positioning and inline terminal scrollback rendering engines, which do not use
`OffscreenBuffer` or `RenderOps` (the standard TUI rendering infrastructure).

We will consolidate their implementations to leverage standard `r3bl_tui` code. This
eliminates massive complexity and brittle terminal behavior (like screen tearing and
cursor jumping) while unifying the rendering architecture.

### Architectural Vision & The `fzf` Illusion

Moving to `OffscreenBuffer` and `RenderOps` shifts our mental model away from injecting
interactive components directly inline with the terminal's native scrollback buffer.

Instead, both tools transition to an **"alternate screen"** model, using full raw mode
(similar to `fzf` or `lazygit`):

1. While active, they own the terminal UI, routing all rendering through `OffscreenBuffer`
   and `compositor_render_ops_to_ofs_buf`.
2. Upon completion, they exit the alternate screen and dump the necessary output back to
   the cooked terminal, creating the seamless illusion that the user was working inline
   the whole time.

### Roadmap Alignment & Dependencies

This task is **Step 2** in the **v0.8.0 Unify Rendering Roadmap** in
[`prepare-v0.8.0-meta-task.md:117-128`](prepare-v0.8.0-meta-task.md#L117-L128):

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. task_unify_cli_and_styled_text.md                            │
│    (Extends TuiStyledText with builder methods for RenderOps)   │
└───────────────────────────────┬─────────────────────────────────┘
                                │
┌───────────────────────────────▼─────────────────────────────────┐
│ 2. task_refactor_interactive_apis_to_alternate_screen (THIS)    │
│    (Consolidates choose & readline onto TerminalWindow/App/     │
│     Component primitives; deletes legacy cursor engines)        │
└───────────────────────────────┬─────────────────────────────────┘
                                │
┌───────────────────────────────▼─────────────────────────────────┐
│ 3. remove-tokio.md                                              │
│    (Switches TerminalWindow & InputDevice to sync, dropping     │
│     Tokio from tui in one clean sweep!)                         │
└─────────────────────────────────────────────────────────────────┘
```

- **Prerequisite:
  [`task_unify_cli_and_styled_text.md`](task_unify_cli_and_styled_text.md)**: Extends
  `TuiStyledText` with builder methods so that `choose` and `readline` can emit
  `RenderOps` using ergonomic styling without clunky macros.
- **The Core Consolidation (This Task)**: Rewrites `choose` and `readline` as standard
  `Component` / `App` primitives running through
  [`TerminalWindow::main_event_loop`](file:///home/nazmul/github/roc-binaries/tui/src/tui/terminal_window/main_event_loop.rs).
- **The Downstream Enabler**: Once this task completes, **100% of all UI in `r3bl_tui`
  runs through `TerminalWindow::main_event_loop` and `InputDevice`**. This sets up
  [`remove-tokio.md`](remove-tokio.md) to make `TerminalWindow` synchronous and drop Tokio
  from `tui` in one clean, uninterrupted pass!

---

## Design Specifications

### 1. `choose` Component Design

- **Concept**: Provide a visual way to select items from a list using alternate screen
  mode.
- **Rendering**: Implements `Component` or `App`, rendering into `OffscreenBuffer` via
  `RenderOps` with `TuiStyledText`.
- **Timeout Support**: Configurable timeout that automatically closes the interactive
  session and returns default/empty selection.
- **The Illusion**: The list renders on the alternate screen. When the user selects an
  item and presses Enter, the alternate screen is restored, and the selection is printed
  to standard output in cooked mode.

### 2. `readline` App & Scrollback Design

- **Concept**: Provide a prompt while background tasks concurrently emit output.
- **Scrollback Handling**: Virtual scrollback buffer managed within the alternate screen
  using standard `r3bl_tui` viewports and flexbox layout.
- **The Illusion**: When the user exits the loop, the virtual scrollback history and
  prompt are dumped to the primary cooked terminal.

### 3. Full TUI Integration

- Both APIs run as standard Full TUI primitives through
  [`TerminalWindow::main_event_loop`](file:///home/nazmul/github/roc-binaries/tui/src/tui/terminal_window/main_event_loop.rs).
- Legacy inline cursor manipulation code in `tui/src/readline_async/choose_impl/` and
  `tui/src/readline_async/readline_async_impl/` is completely removed.

---

## Implementation Plan

### Phase 1: Research & Virtual Scrollback Architecture

- [ ] **Step 1.1: Analyze API Usage**
    - Audit call sites of `choose` and `readline_async` across `cmdr` and `tui/examples`.
- [ ] **Step 1.2: Design Virtual Scrollback Buffer**
    - Choose between dumping raw `OffscreenBuffer` lines vs. maintaining a virtual history
      vector flushed to stdout on exit.
- [ ] **Step 1.3: Verify Styling Prerequisite**
    - Ensure `task_unify_cli_and_styled_text.md` is complete for `TuiStyledText` builders.
- [ ] **Mandatory manual review:**
    - [ ] Architecture notes and prototype designs

---

### Phase 2: Refactor `choose` to Alternate-Screen Component

- [ ] **Step 2.1: Implement `ChooseComponent`**
    - Create standard `Component` implementation emitting `RenderOps` with
      `TuiStyledText`.
    - Support single/multiple selection, search filtering, and optional timeout.
- [ ] **Step 2.2: Implement Alternate-Screen Lifecycle**
    - Initialize alternate screen, execute via `TerminalWindow::main_event_loop`, and tear
      down cleanly.
    - In cooked mode, `println!` the chosen selection.
- [ ] **Step 2.3: Fast Local Verification**
    - Run `./check.fish --check`.
- [ ] **Mandatory manual review:**
    - [ ] `tui/src/readline_async/choose_api.rs`
    - [ ] `tui/src/readline_async/choose_impl/select_component.rs`

---

### Phase 3: Refactor `readline` to Alternate-Screen App

- [ ] **Step 3.1: Implement `ReadlineApp`**
    - Implement full TUI `App` managing prompt input line and scrollback viewport via
      `RenderOps`.
- [ ] **Step 3.2: Integrate `SharedWriter` Signal Queue**
    - Route concurrent `SharedWriter` outputs to `main_thread_channel`.
- [ ] **Step 3.3: Implement Cooked-Mode Exit Flush**
    - Dump the virtual scrollback to stdout upon exit.
- [ ] **Step 3.4: Fast Local Verification**
    - Run `./check.fish --check`.
- [ ] **Mandatory manual review:**
    - [ ] `tui/src/readline_async/readline_async_impl/readline_struct.rs`
    - [ ] `tui/src/readline_async/spinner.rs`
    - [ ] `tui/src/readline_async/readline_async_api.rs`

---

### Phase 4: Cleanup, Deprecate Legacy Engines, & Validation

- [ ] **Step 4.1: Delete Legacy Inline Cursor Engines**
    - Remove legacy cursor positioning hacks in `choose_impl` and `readline_async_impl`.
- [ ] **Step 4.2: Update Downstream Consumers in `cmdr`**
    - Update `giti` branch selection and `edi` prompt integration.
- [ ] **Step 4.3: Full Quality Check**
    - Run `./check.fish --fmt`
    - Run `./check.fish --clippy`
    - Run `./check.fish --test`
- [ ] **Mandatory manual review:**
    - [ ] All deleted files in `tui/src/readline_async/`
    - [ ] All modified files in `cmdr/`

---

## Success Criteria

1. **Alternate Screen Parity**: `choose` and `readline` operate cleanly in alternate
   screen mode with zero screen tearing or cursor jumping.
2. **Unified Styling & Rendering**: Both APIs emit standard `RenderOps` and render to
   `OffscreenBuffer` using `TuiStyledText`.
3. **Consolidated Engine**: Legacy cursor manipulation engines are deleted; all UI routes
   through `TerminalWindow::main_event_loop`.
4. **Clean Exit**: Both APIs exit cooked mode and print selections/scrollback to terminal
   history seamlessly.
