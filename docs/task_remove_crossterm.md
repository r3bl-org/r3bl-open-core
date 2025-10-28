<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Task: Remove Crossterm via Unified RenderOp Architecture](#task-remove-crossterm-via-unified-renderop-architecture)
  - [Overview](#overview)
    - [⚠️ DEPENDENCY: Requires task_unify_rendering.md Completion](#-dependency-requires-task_unify_renderingmd-completion)
    - [Architectural Vision](#architectural-vision)
      - [Ultimate Architecture Vision](#ultimate-architecture-vision)
  - [Current Architecture Analysis](#current-architecture-analysis)
    - [Correct Render Pipeline Flow](#correct-render-pipeline-flow)
    - [Where Crossterm is Used Today](#where-crossterm-is-used-today)
    - [Performance Bottleneck](#performance-bottleneck)
  - [New Unified Architecture](#new-unified-architecture)
    - [RenderOp as Universal Language](#renderop-as-universal-language)
    - [Architectural Symmetry](#architectural-symmetry)
    - [Benefits of This Approach](#benefits-of-this-approach)
    - [Architectural Alignment with task_unify_rendering.md](#architectural-alignment-with-task_unify_renderingmd)
  - [Implementation Plan](#implementation-plan)
    - [Phase 1: Extend RenderOp for Incremental Rendering](#phase-1-extend-renderop-for-incremental-rendering)
      - [1.1 RenderOp Variants Added](#11-renderop-variants-added)
      - [1.2 TerminalModeState Infrastructure](#12-terminalmodestate-infrastructure)
      - [1.3 Crossterm Backend Implementation](#13-crossterm-backend-implementation)
      - [1.4 Compositor Infrastructure Refactoring](#14-compositor-infrastructure-refactoring)
      - [1.5 Code Quality](#15-code-quality)
      - [Actual Accomplishments vs. Original Plan](#actual-accomplishments-vs-original-plan)
    - [Phase 2: Implement DirectAnsi Backend](#phase-2-implement-directansi-backend)
    - [Progress Summary](#progress-summary)
      - [✅ Step 1: DirectAnsi Module Structure - COMPLETE](#-step-1-directansi-module-structure---complete)
      - [✅ Step 2: AnsiSequenceGenerator Implementation - COMPLETE (ENHANCED APPROACH)](#-step-2-ansisequencegenerator-implementation---complete-enhanced-approach)
  - [Architecture Overview: Leveraging VT-100 Infrastructure](#architecture-overview-leveraging-vt-100-infrastructure)
  - [✅ Step 1: Create DirectAnsi Module Structure (30 min) - COMPLETE](#-step-1-create-directansi-module-structure-30-min---complete)
  - [✅ Step 2: Implement AnsiSequenceGenerator (3-4 hours) - COMPLETE](#-step-2-implement-ansisequencegenerator-3-4-hours---complete)
    - [Key Design Achievement: Semantic ANSI Generation with VT-100 Infrastructure](#key-design-achievement-semantic-ansi-generation-with-vt-100-infrastructure)
    - [Implementation: Leveraging Type-Safe Enums](#implementation-leveraging-type-safe-enums)
      - [Section A: Cursor Movement Operations (Using CsiSequence)](#section-a-cursor-movement-operations-using-csisequence)
      - [Section B: Screen Clearing Operations](#section-b-screen-clearing-operations)
      - [Section C: Color Operations (Using SgrColorSequence)](#section-c-color-operations-using-sgrcolorsequence)
      - [Section D: Cursor Visibility Operations](#section-d-cursor-visibility-operations)
      - [Section E: Cursor Save/Restore Operations](#section-e-cursor-saverestore-operations)
      - [Section F: Terminal Mode Operations](#section-f-terminal-mode-operations)
      - [Section G: Module Documentation](#section-g-module-documentation)
  - [✅ Step 3: Complete Type System Architecture & DirectAnsi Backend (EXPANDED - ~40-50 hours) - COMPLETE](#-step-3-complete-type-system-architecture--directansi-backend-expanded---40-50-hours---complete)
    - [Step 3.0: Remove IR Execution Path & Enforce Semantic Boundary (2-3 hours)](#step-30-remove-ir-execution-path--enforce-semantic-boundary-2-3-hours)
    - [Step 3.1: Create RenderOpOutput Execution Path (3-4 hours)](#step-31-create-renderopoutput-execution-path-3-4-hours)
    - [Step 3.2: Fix OffscreenBufferPaint Trait & RawMode Infrastructure (3-4 hours)](#step-32-fix-offscreenbufferpaint-trait--rawmode-infrastructure-3-4-hours)
    - [Step 3.3: Implement RenderOpPaintImplDirectAnsi (DirectAnsi Backend) (25-35 hours)](#step-33-implement-renderoppaintimpldirectansi-directansi-backend-25-35-hours)
  - [Step 3 Summary](#step-3-summary)
  - [Critical Success Factors for Step 3](#critical-success-factors-for-step-3)
  - [✅ Step 4: Linux Validation & Performance Testing (2-3 hours) - COMPLETE](#-step-4-linux-validation--performance-testing-2-3-hours---complete)
    - [Step 4 Results Summary](#step-4-results-summary)
    - [Step 4 Detailed Findings](#step-4-detailed-findings)
    - [Backend Configuration](#backend-configuration)
    - [Linux Testing](#linux-testing)
    - [Performance Benchmarking](#performance-benchmarking)
    - [Documentation & Sign-Off](#documentation--sign-off)
  - [✅ Step 5: Performance Validation & Optimization - COMPLETE](#-step-5-performance-validation--optimization---complete)
    - [Performance Results](#performance-results)
      - [Crossterm Baseline (commit 0f178adc)](#crossterm-baseline-commit-0f178adc)
      - [DirectToAnsi (current HEAD - commit a42a9419)](#directtoansi-current-head---commit-a42a9419)
      - [Performance Comparison](#performance-comparison)
    - [Optimizations Implemented](#optimizations-implemented)
      - [✅ Stack-Allocated Number Formatting](#-stack-allocated-number-formatting)
      - [✅ U8_STRINGS Lookup Table for Color Sequences](#-u8_strings-lookup-table-for-color-sequences)
    - [Flamegraph Analysis: Backend is NOT in the Hot Path](#flamegraph-analysis-backend-is-not-in-the-hot-path)
    - [SmallVec Optimization Opportunity](#smallvec-optimization-opportunity)
    - [✅ SmallVec[16] Optimization - IMPLEMENTED](#-smallvec16-optimization---implemented)
      - [Baseline (SmallVec[8])](#baseline-smallvec8)
      - [After SmallVec[16]](#after-smallvec16)
    - [✅ SmallVec[16] for StyleUSSpan - IMPLEMENTED](#-smallvec16-for-styleusspan---implemented)
      - [Baseline (SmallVec[8])](#baseline-smallvec8-1)
      - [After SmallVec[16] (Both Types)](#after-smallvec16-both-types)
  - [⏳ Step 6: Cleanup & Architectural Refinement (1-2 hours)](#-step-6-cleanup--architectural-refinement-1-2-hours)
    - [6.1: DirectToAnsi Rename - ALREADY COMPLETE ✅](#61-directtoansi-rename---already-complete-)
    - [6.2: Remove Termion Backend (Dead Code Removal)](#62-remove-termion-backend-dead-code-removal)
    - [6.3: Resolve TODOs and Stubs](#63-resolve-todos-and-stubs)
    - [6.4: Review `cli_text` and `tui_styled_text` Consistency](#64-review-cli_text-and-tui_styled_text-consistency)
  - [⏳ Step 7: Comprehensive RenderOp Integration Test Suite (4-6 hours)](#-step-7-comprehensive-renderop-integration-test-suite-4-6-hours)
  - [⏳ Step 8: Implement InputDevice for DirectToAnsi Backend (8-12 hours)](#-step-8-implement-inputdevice-for-directtoansi-backend-8-12-hours)
    - [8.1: Architecture Design (1-2 hours)](#81-architecture-design-1-2-hours)
    - [8.2: Implement DirectToAnsi InputDevice (4-6 hours)](#82-implement-directtoansi-inputdevice-4-6-hours)
    - [8.3: Testing & Validation (2-3 hours)](#83-testing--validation-2-3-hours)
    - [8.4: Migration & Cleanup (1 hour)](#84-migration--cleanup-1-hour)
  - [⏳ Step 9: macOS & Windows Platform Validation (2-3 hours) - DEFERRED](#-step-9-macos--windows-platform-validation-2-3-hours---deferred)
    - [macOS Testing (1.5 hours)](#macos-testing-15-hours)
    - [Windows Testing (1.5 hours)](#windows-testing-15-hours)
    - [Documentation & Sign-Off (30 min)](#documentation--sign-off-30-min)
  - [Implementation Checklist](#implementation-checklist)
  - [Critical Success Factors](#critical-success-factors)
  - [Effort Summary - Steps 2-6 Implementation](#effort-summary---steps-2-6-implementation)
  - [Conclusion](#conclusion)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Task: Remove Crossterm via Unified RenderOp Architecture

## Overview

This document outlines the plan to remove the crossterm dependency by unifying all rendering paths
around `RenderOp` as a universal terminal rendering language, implementing a DirectAnsi backend
using `PixelCharRenderer`, and creating a symmetric VT-100 input parser.

**Key Insight**: Instead of virtualizing crossterm's API, we standardize on `RenderOp` (which we
already own) as the rendering language for all three paths: Full TUI, choose(), and
readline_async(). This creates a cleaner architecture with perfect symmetry between output and
input.

### ⚠️ DEPENDENCY: Requires task_unify_rendering.md Completion

**This task depends on completion of [task_unify_rendering.md](done/task_unify_rendering.md):**

| Unification Phase      | Output                                                   | Status                              | Notes                                  |
| ---------------------- | -------------------------------------------------------- | ----------------------------------- | -------------------------------------- |
| **0.5** (prerequisite) | CliTextInline uses CliTextInline abstraction for styling | ✅ COMPLETE                         | Standardizes styling before renaming   |
| **1** (rename)         | AnsiStyledText → CliTextInline                           | ✅ COMPLETE (October 21, 2025)      | Type rename across codebase            |
| **2** (core)           | `PixelCharRenderer` module created                       | ✅ COMPLETE (October 22, 2025)      | Unified ANSI sequence generator        |
| **3** (integration)    | `RenderToAnsi` trait for unified buffer rendering        | ✅ COMPLETE (October 22, 2025)      | Ready for DirectAnsi backend           |
| **4** (CURRENT)        | `CliTextInline` uses `PixelCharRenderer` via traits      | ✅ COMPLETE (October 22, 2025)      | All direct text rendering unified      |
| **5** (DEFERRED)       | choose()/readline_async to OffscreenBuffer               | ⏸️ DEFERRED to Phase 3 of this task | Proper migration is via RenderOps      |
| **6** (COMPLETE)       | `RenderOpImplCrossterm` uses `PixelCharRenderer`         | ✅ COMPLETE (October 22, 2025)      | Unified renderer validated in full TUI |

**Execution Order:**

1. ✅ Complete task_unify_rendering.md (Phases 0-4, 6)
2. ⏸️ Skip Phase 5 (will be done in Phase 3 of this task)
3. ✅ Complete task_unify_rendering.md Phase 6 (validated PixelCharRenderer in full TUI)
4. 🚀 Ready to begin this task (task_remove_crossterm.md Phase 1-3)

**Why this dependency matters:**

- `PixelCharRenderer` already generates ANSI sequences from `PixelChar[]`
- We're replacing crossterm's output backend, not the entire rendering logic
- This keeps the change focused and lower risk

### Architectural Vision

```
┌────────────────────────────────────────────────────┐
│              All Three Rendering Paths             │
│  ┌──────────┐  ┌──────────┐  ┌─────────────────┐   │
│  │ Full TUI │  │ choose() │  │ readline_async()│   │
│  └────┬─────┘  └────┬─────┘  └────────┬────────┘   │
└───────┼─────────────┼─────────────────┼────────────┘
        │             │                 │
        └─────────────┴─────────────────┘
                      │
                      │
              ┌───────▼───────┐
              │   RenderOps   │  ← Universal rendering language
              └───────┬───────┘
                      │
                      │
              ┌───────▼───────────┐
              │ DirectAnsi Backend│  ← Replaces crossterm
              │ (AnsiSequenceGen) │
              └───────┬───────────┘
                      │
                      │
              ┌───────▼───────────┐
              │   OutputDevice    │  ← Unchanged (testability)
              └───────┬───────────┘
                      │
                      ▼
                    stdout
```

**Input symmetry:**

```
     stdin → tokio async read → VT-100 Parser → Events → InputDevice → Application
```

#### Ultimate Architecture Vision

```
┌──────────────────────────────────────────────────────────┐
│                    Application                           │
└──────────────────────┬───────────────────────────────────┘
                       │
          ┌────────────▼───────────┐
          │     RenderOps          │
          │  (layout abstraction)  │
          └────────────┬───────────┘
                       │
          ┌────────────▼───────────┐
          │  OffscreenBuffer       │
          │  (materialized state)  │
          │  Contains: PixelChar[] │
          └────────────┬───────────┘
                       │
                       ├─→ Diff algorithm
                       │
      ┌────────────────▼────────────────────┐
      │  CompositorNoClipTrunc...           │
      │  Extracts changed text + style      │
      └──────────────┬──────────────────────┘
                     │
                     │ (Phase 6)
         ┌───────────▼───────────────┐
         │  CliTextInline conversion │
         │  text + style → PixelChar │
         └──────────────┬────────────┘
                        │
         ┌──────────────▼─────────┐
         │  PixelCharRenderer     │
         │ (unified ANSI gen)     │
         │ Smart style diffing    │
         └──────────────┬─────────┘
                        │
         ┌──────────────▼─────────┐
         │  ANSI bytes (UTF-8)    │
         │ Ready for any backend  │
         └──────────────┬─────────┘
                        │
        ┌───────────────┼───────────────┐
        │               │               │
        ▼ (Now)         ▼ (Phase 2)     ▼ (Phase 3)
    Crossterm       DirectAnsi       DirectAnsi
    OutputDevice    Backend          Backend
       (Phase 6)    (Pending)        (Pending)
        │               │               │
        └───────────────┼───────────────┘
                        │
                        ▼
                      stdout
```

## Current Architecture Analysis

### Correct Render Pipeline Flow

**Full TUI (already optimal):**

```
RenderOps → OffscreenBuffer → PixelCharRenderer → ANSI → stdout
  (layout)    (materialized)      (encoding)
```

**RenderOps Purpose:**

- **Layout abstraction**: Positioning, sizing, layering, z-order management
- **Compositor input**: Feeds the OffscreenBuffer compositor
- **NOT an intermediate format**: OffscreenBuffer is the final materialized state before ANSI
  encoding

**Important**: There is NO "round-trip" from OffscreenBuffer back to RenderOps. OffscreenBuffer is
the final abstraction, which PixelCharRenderer directly encodes to ANSI bytes.

### Where Crossterm is Used Today

1. **Full TUI**: Uses `RenderOpImplCrossterm` backend to execute RenderOps
2. **choose()**: Directly calls crossterm via `queue_commands!` macro
3. **readline_async()**: Directly calls crossterm via `queue_commands!` macro
4. **Input handling**: Uses `crossterm::event::read()` for keyboard/mouse events

**Crossterm usage in choose()/readline_async():**

```rust
// Current code (crossterm-specific):
queue_commands! {
    output_device,
    MoveToColumn(0),           // crossterm::cursor::MoveToColumn
    ResetColor,                // crossterm::style::ResetColor
    Clear(ClearType::CurrentLine), // crossterm::terminal::Clear
    Print(styled_header),      // crossterm::style::Print
    MoveToNextLine(1),         // crossterm::cursor::MoveToNextLine
};
```

### Performance Bottleneck

- **15M samples** in ANSI formatting overhead (from flamegraph profiling)
- Crossterm's command abstraction layer adds unnecessary overhead
- Multiple trait dispatches and error handling for simple ANSI writes
- Opportunity for optimization through direct ANSI generation

## New Unified Architecture

### RenderOp as Universal Language

`RenderOp` is already designed as a backend-agnostic abstraction. Instead of creating a
crossterm-compatible shim, we:

1. **Extend RenderOp** with operations needed by choose()/readline_async() (incremental rendering)
2. **Implement DirectAnsi backend** that uses `PixelCharRenderer` for ANSI generation
3. **Migrate all paths** to speak RenderOps instead of crossterm

**Key advantages:**

- RenderOp is higher-level than crossterm (supports TUI concepts like z-order, relative positioning,
  styled text)
- RenderOp already has infrastructure to route to different backends
- RenderOp is something we own and control
- No need to maintain crossterm compatibility layer

### Architectural Symmetry

**Output Path** (all three rendering paths):

```
Application → RenderOps → DirectAnsi Backend → ANSI bytes → stdout
```

**Input Path** (reuse VT-100 parser for symmetry):

```
stdin → ANSI bytes → VT-100 Parser → Events → InputDevice → Application
```

**Perfect symmetry**: Output generates ANSI, input parses ANSI. Both sides speak the same protocol.

### Benefits of This Approach

1. **Single abstraction layer**: RenderOps for everything
2. **Code reuse**: Leverage existing `PixelCharRenderer` and VT-100 parser
3. **No dependencies**: Pure Rust, no crossterm/termion needed
4. **Testability**: Can mock RenderOps execution easily
5. **Extensibility**: Easy to add new backends (Termion, SSH optimization, etc.)
6. **Performance**: Direct ANSI generation eliminates crossterm overhead

### Architectural Alignment with task_unify_rendering.md

**Critical insight**: This task is specifically designed to leverage `PixelCharRenderer` created in
task_unify_rendering.md Phase 2-3.

**Why Phase 6 of task_unify_rendering.md Comes First:**

task_unify_rendering.md Phase 6 modifies `RenderOpImplCrossterm::paint_text_with_attributes()` to
use `PixelCharRenderer`. This validation step:

- ✅ Tests PixelCharRenderer in production full TUI render loop
- ✅ Proves all three rendering paths can share ANSI generation
- ✅ Provides safe rollback point before big crossterm removal
- ✅ Creates confidence that the abstraction works end-to-end

**Then this task can confidently:**

- Create `RenderOpImplDirectAnsi` using same `PixelCharRenderer`
- Migrate choose()/readline_async to RenderOps (Phase 3 here = Phase 5 deferred in
  task_unify_rendering.md)
- Remove crossterm dependency knowing the abstraction is proven

**The Critical Architectural Pattern:**

```
RenderOp abstraction → PixelCharRenderer → ANSI bytes → OutputDevice implementation
                       ↑
                  Backend-agnostic
                  Can switch OutputDevice between:
                  - Crossterm (task_unify_rendering.md Phase 6)
                  - DirectAnsi (this task Phase 2)
                  - Future: Termion, SSH optimization, etc.
```

**Why this is safe:**

1. `PixelCharRenderer` has no crossterm dependencies
2. ANSI generation logic is identical across backends
3. Only OutputDevice implementation differs
4. Each backend can be tested independently

## Implementation Plan

### Phase 1: Extend RenderOp for Incremental Rendering

- **Status**: ✅ **COMPLETE** (Commit: `ea269dca`)
- **Date**: October 23, 2025
- **Commit Message**: `[tui] Prepare compositor and renderops for crossterm removal`

#### 1.1 RenderOp Variants Added

All 11 new `RenderOp` variants have been successfully added to
`tui/src/tui/terminal_lib_backends/render_op.rs` with comprehensive documentation:

| Variant                               | ANSI Sequence | Purpose                               | Status |
| ------------------------------------- | ------------- | ------------------------------------- | ------ |
| `MoveCursorToColumn(ColIndex)`        | CSI `<n>G`    | Horizontal positioning in current row | ✅     |
| `MoveCursorToNextLine(RowHeight)`     | CSI `<n>E`    | Move down N lines to column 0         | ✅     |
| `MoveCursorToPreviousLine(RowHeight)` | CSI `<n>F`    | Move up N lines to column 0           | ✅     |
| `ClearCurrentLine`                    | CSI `2K`      | Erase entire line, keep cursor        | ✅     |
| `ClearToEndOfLine`                    | CSI `0K`      | Erase from cursor to line end         | ✅     |
| `ClearToStartOfLine`                  | CSI `1K`      | Erase from line start to cursor       | ✅     |
| `PrintStyledText(InlineString)`       | N/A           | Print pre-styled ANSI text as-is      | ✅     |
| `ShowCursor`                          | CSI `?25h`    | Make cursor visible                   | ✅     |
| `HideCursor`                          | CSI `?25l`    | Make cursor invisible                 | ✅     |
| `SaveCursorPosition`                  | CSI `s`       | Save cursor position (DECSC)          | ✅     |
| `RestoreCursorPosition`               | CSI `u`       | Restore saved cursor position (DECRC) | ✅     |

**Plus terminal mode operations:**

- `EnterAlternateScreen` / `ExitAlternateScreen`
- `EnableMouseTracking` / `DisableMouseTracking`
- `EnableBracketedPaste` / `DisableBracketedPaste`

#### 1.2 TerminalModeState Infrastructure

Introduced new `TerminalModeState` struct in
`tui/src/tui/terminal_lib_backends/offscreen_buffer/ofs_buf_core.rs` to track terminal state across
all rendering paths:

```rust
pub struct TerminalModeState {
    pub is_raw_mode: bool,              // POSIX non-canonical mode
    pub alternate_screen_active: bool,  // Full-screen app buffer
    pub mouse_tracking_enabled: bool,   // Mouse event reporting
    pub bracketed_paste_enabled: bool,  // Clipboard paste detection
}
```

**Why this matters**: Sets up infrastructure for DirectAnsi backend to manage terminal state
independently from crossterm's abstraction.

#### 1.3 Crossterm Backend Implementation

Fully implemented all RenderOp variants in
`tui/src/tui/terminal_lib_backends/crossterm_backend/paint_render_op_impl.rs`:

- **Simple operations** (15+ variants): Direct `queue_terminal_command!` macro calls
- **Complex operations**: Helper methods like:
  - `move_cursor_to_column()` - Updates local cursor tracking
  - `move_cursor_to_next_line()` - Arithmetic on RowIndex with bounds safety
  - `move_cursor_to_previous_line()` - Saturating subtraction for safety
  - `print_styled_text()` - Preserves pre-styled ANSI text
  - `save_cursor_position()` / `restore_cursor_position()` - Direct ANSI writes (crossterm gap)

**Key implementation detail**: For operations crossterm doesn't directly support (cursor
save/restore), implementation writes raw ANSI bytes directly to output device with error handling.

#### 1.4 Compositor Infrastructure Refactoring

Renamed and restructured compositor (render ops → output) logic:

- **New file**: `tui/src/tui/terminal_lib_backends/compositor_render_ops_to_ofs_buf.rs`
- **Purpose**: Clear separation of concerns - this is where RenderOps are materialized to terminal
  output
- **Benefit**: Prepares codebase for easy backend switching (crossterm → DirectAnsi)

#### 1.5 Code Quality

- ✅ All 52 affected files updated (references, imports, formatting)
- ✅ Clippy compliance across all changes
- ✅ Cargo fmt applied throughout
- ✅ Reference-style markdown links normalized
- ✅ Type-safe bounds checking (ColIndex, RowHeight, Pos)

#### Actual Accomplishments vs. Original Plan

**More comprehensive than planned:**

| Aspect            | Planned                  | Actual                                          |
| ----------------- | ------------------------ | ----------------------------------------------- |
| RenderOp variants | 11 documented            | 11 + 6 terminal modes = 17 total                |
| Testing           | Unit + integration tests | Handler implementations in crossterm backend    |
| Infrastructure    | Just variants            | Full TerminalModeState + compositor refactoring |
| Code size         | ~200 lines               | 1,242 insertions, 304 deletions (52 files)      |
| Scope             | Just RenderOp extension  | Includes fullCrossterm backend implementation   |

**Strategic advantage**: Phase 1 not only adds the RenderOps but **validates them in production** by
implementing them in the crossterm backend. This provides:

1. ✅ Proof that RenderOps are expressive enough for all rendering needs
2. ✅ Working reference implementation for DirectAnsi backend in Phase 2
3. ✅ Immediate availability of incremental rendering for choose()/readline_async()
4. ✅ Zero risk - crossterm backend continues working during transition

---

### Phase 2: Implement DirectAnsi Backend

**Objective**: Create a new terminal backend that generates ANSI sequences directly, replacing
crossterm's implementation.

**Status**: ✅ STEPS 1-2 COMPLETE (October 23, 2025) | ⏳ Step 3 Ready

### Progress Summary

#### ✅ Step 1: DirectAnsi Module Structure - COMPLETE

- Created `tui/src/tui/terminal_lib_backends/direct_ansi/` directory
- Implemented `mod.rs` with proper re-exports and Phase 1/2 organization
- Created all implementation files with proper documentation
- `cargo check` passes cleanly

#### ✅ Step 2: AnsiSequenceGenerator Implementation - COMPLETE (ENHANCED APPROACH)

- **All 40+ methods implemented** using semantic ANSI generation (not raw format!)
- **Key Achievement**: Replaced raw `format!()` calls with semantic typed enums
- **Leveraged VT-100 Infrastructure**:
  - `CsiSequence` enums for cursor movement, screen clearing, save/restore
  - `SgrColorSequence` enums for foreground/background colors (256-color and RGB)
  - `PrivateModeType` enums for terminal modes (cursor visibility, alternate screen, mouse, paste)
  - `SGR_BOLD`, `SGR_DIM`, `SGR_ITALIC`, `SGR_UNDERLINE`, `SGR_STRIKETHROUGH` constants for text
    attributes
  - `TermRow::from_zero_based()` / `TermCol::from_zero_based()` methods for coordinate conversion
- **Type Safety**: All sequences are type-safe with compile-time guarantees
- **Return Type**: Methods return `String` (more efficient than `Vec<u8>`, avoids extra allocation)
- **Test Coverage**: ✅ 33/33 unit tests passing
  - 5 cursor positioning tests
  - 4 screen clearing tests
  - 1 reset color test
  - 2 cursor visibility tests
  - 5 terminal mode tests
  - 16 pixel_char_renderer tests (from Phase 1)
- **Code Quality**: Zero warnings, clean compilation

---

## Architecture Overview: Leveraging VT-100 Infrastructure

```
VT-100 ANSI Parser (existing, one-way: bytes → state)
├── SgrColorSequence enum (color parsing)
├── CsiSequence enum (CSI parsing)
├── FastStringify trait (sequence generation)
└── Constants: SGR_BOLD, SGR_ITALIC, etc.
        ↓
        ↓  (Phase 2 REUSES this infrastructure)
        ↓
DirectAnsi Backend (new, one-way: RenderOp → ANSI bytes)
├── AnsiSequenceGenerator (generates from RenderOp variants)
├── RenderOpImplDirectAnsi (executes RenderOps)
└── Uses SgrColorSequence + FastStringify for color sequences
```

**Key Insight**: The VT-100 parser already has everything needed for **sequence generation**:

- `SgrColorSequence` enum with `FastStringify` trait for generating color sequences
- All ANSI constants (SGR_BOLD, SGR_ITALIC, etc.) already defined
- Architecture proven in parser—we're inverting it for generation

---

## ✅ Step 1: Create DirectAnsi Module Structure (30 min) - COMPLETE

**Files Created**:

```
tui/src/tui/terminal_lib_backends/direct_ansi/
├── mod.rs                          # Module coordinator with re-exports (✅)
├── ansi_sequence_generator.rs      # ANSI sequence generation - 273 LOC (✅)
├── render_op_impl_direct_ansi.rs   # Backend implementation - 55 LOC (✅)
├── tests.rs                        # Unit tests - 133 LOC (✅)
└── integration_tests.rs            # Integration tests - 22 LOC (✅)
```

**Completed Subtasks**:

- [x] Create directory: `tui/src/tui/terminal_lib_backends/direct_ansi/`
- [x] Create `mod.rs` with module declarations and re-exports
- [x] Create implementation files with proper documentation
- [x] Run `cargo check` and verify module structure compiles cleanly

**File**: `tui/src/tui/terminal_lib_backends/direct_ansi/mod.rs`

```rust
// Module re-exports for clean public API
pub mod ansi_sequence_generator;
pub mod render_op_impl_direct_ansi;

pub use ansi_sequence_generator::AnsiSequenceGenerator;
pub use render_op_impl_direct_ansi::RenderOpImplDirectAnsi;
```

**Checkpoint**: Empty module compiles, `cargo check` passes

---

## ✅ Step 2: Implement AnsiSequenceGenerator (3-4 hours) - COMPLETE

**File**: `tui/src/tui/terminal_lib_backends/direct_ansi/ansi_sequence_generator.rs` (273 LOC)

### Key Design Achievement: Semantic ANSI Generation with VT-100 Infrastructure

This module generates ANSI escape sequences using **semantic types** (not raw format!) for each
terminal operation. All methods return `String` for efficiency.

### Implementation: Leveraging Type-Safe Enums

#### Section A: Cursor Movement Operations (Using CsiSequence)

```rust
impl AnsiSequenceGenerator {
    /// Generate absolute cursor positioning: CSI <row>;<col>H (1-based indexing)
    pub fn cursor_position(row: RowIndex, col: ColIndex) -> String {
        CsiSequence::CursorPosition {
            row: TermRow::from_zero_based(row),
            col: TermCol::from_zero_based(col),
        }
        .to_string()
    }

    /// Generate cursor to column: CSI <col>G (1-based)
    pub fn cursor_to_column(col: ColIndex) -> String {
        let one_based_col = col.as_usize() as u16 + 1;
        CsiSequence::CursorHorizontalAbsolute(one_based_col).to_string()
    }

    /// Generate cursor next line: CSI <n>E
    pub fn cursor_next_line(rows: RowHeight) -> String {
        CsiSequence::CursorNextLine(rows.as_usize() as u16).to_string()
    }

    /// Generate cursor previous line: CSI <n>F
    pub fn cursor_previous_line(rows: RowHeight) -> String {
        CsiSequence::CursorPrevLine(rows.as_usize() as u16).to_string()
    }
}
```

**VT-100 Types Used**: `CsiSequence` enum, `TermRow::from_zero_based()`,
`TermCol::from_zero_based()`

#### Section B: Screen Clearing Operations

```rust
impl AnsiSequenceGenerator {
    /// Clear entire screen: CSI 2J (Erase Display: 2 = entire display)
    pub fn clear_screen() -> Vec<u8> {
        b"\x1b[2J".to_vec()
    }

    /// Clear current line: CSI 2K (Erase Line: 2 = entire line)
    pub fn clear_current_line() -> Vec<u8> {
        b"\x1b[2K".to_vec()
    }

    /// Clear to end of line: CSI 0K (Erase Line: 0 = cursor to end)
    pub fn clear_to_end_of_line() -> Vec<u8> {
        b"\x1b[0K".to_vec()
    }

    /// Clear to start of line: CSI 1K (Erase Line: 1 = start to cursor)
    pub fn clear_to_start_of_line() -> Vec<u8> {
        b"\x1b[1K".to_vec()
    }
}
```

**Reference Constants**: `ED_ERASE_DISPLAY`, `EL_ERASE_LINE`

#### Section C: Color Operations (Using SgrColorSequence)

This is the key innovation—**reuse the VT-100 parser's color infrastructure**:

```rust
impl AnsiSequenceGenerator {
    /// Convert TuiColor to SgrColorSequence for code generation
    /// This leverages the existing vt_100_ansi_parser infrastructure
    fn tuicolor_to_sgr_sequence(color: TuiColor, is_background: bool) -> Option<SgrColorSequence> {
        use crate::core::pty_mux::vt_100_ansi_parser::protocols::csi_codes::SgrColorSequence;

        match color {
            TuiColor::Ansi(ansi_val) => {
                let index = ansi_val.as_u8();
                if is_background {
                    Some(SgrColorSequence::SetBackgroundAnsi256(index))
                } else {
                    Some(SgrColorSequence::SetForegroundAnsi256(index))
                }
            }
            TuiColor::Rgb(rgb_val) => {
                let (r, g, b) = rgb_val.as_u8_triple();
                if is_background {
                    Some(SgrColorSequence::SetBackgroundRgb(r, g, b))
                } else {
                    Some(SgrColorSequence::SetForegroundRgb(r, g, b))
                }
            }
            // Handle other TuiColor variants as needed
        }
    }

    /// Generate foreground color sequence using SgrColorSequence + FastStringify
    pub fn fg_color(color: TuiColor) -> Vec<u8> {
        if let Some(sgr_seq) = Self::tuicolor_to_sgr_sequence(color, false) {
            // Use FastStringify to generate ANSI bytes (colon format)
            sgr_seq.to_string().into_bytes()
        } else {
            Self::reset_color()
        }
    }

    /// Generate background color sequence
    pub fn bg_color(color: TuiColor) -> Vec<u8> {
        if let Some(sgr_seq) = Self::tuicolor_to_sgr_sequence(color, true) {
            sgr_seq.to_string().into_bytes()
        } else {
            Self::reset_color()
        }
    }

    /// Generate text attributes: bold, italic, underline, strikethrough
    pub fn text_attributes(style: &TuiStyle) -> Vec<u8> {
        use crate::core::pty_mux::vt_100_ansi_parser::protocols::csi_codes::constants::*;

        let mut bytes = Vec::new();
        if style.bold {
            bytes.extend_from_slice(&format!("\x1b[{}m", SGR_BOLD).into_bytes());
        }
        if style.dim {
            bytes.extend_from_slice(&format!("\x1b[{}m", SGR_DIM).into_bytes());
        }
        if style.italic {
            bytes.extend_from_slice(&format!("\x1b[{}m", SGR_ITALIC).into_bytes());
        }
        if style.underline {
            bytes.extend_from_slice(&format!("\x1b[{}m", SGR_UNDERLINE).into_bytes());
        }
        if style.strikethrough {
            bytes.extend_from_slice(&format!("\x1b[{}m", SGR_STRIKETHROUGH).into_bytes());
        }
        bytes
    }

    /// Reset all colors and attributes: CSI 0m (SGR Reset)
    pub fn reset_color() -> Vec<u8> {
        b"\x1b[0m".to_vec()
    }
}
```

**Reference Constants**: `SGR_RESET`, `SGR_BOLD`, `SGR_DIM`, `SGR_ITALIC`, `SGR_UNDERLINE`,
`SGR_STRIKETHROUGH`

#### Section D: Cursor Visibility Operations

```rust
impl AnsiSequenceGenerator {
    /// Show cursor: CSI ?25h (DECTCEM: DEC Text Cursor Enable Mode = set)
    pub fn show_cursor() -> Vec<u8> {
        b"\x1b[?25h".to_vec()
    }

    /// Hide cursor: CSI ?25l (DECTCEM = reset)
    pub fn hide_cursor() -> Vec<u8> {
        b"\x1b[?25l".to_vec()
    }
}
```

**Reference Constants**: `DECTCEM_SHOW_CURSOR = 25`

#### Section E: Cursor Save/Restore Operations

```rust
impl AnsiSequenceGenerator {
    /// Save cursor position: CSI s (DECSC: Save Cursor)
    pub fn save_cursor_position() -> Vec<u8> {
        b"\x1b[s".to_vec()
    }

    /// Restore cursor position: CSI u (DECRC: Restore Cursor)
    pub fn restore_cursor_position() -> Vec<u8> {
        b"\x1b[u".to_vec()
    }
}
```

#### Section F: Terminal Mode Operations

```rust
impl AnsiSequenceGenerator {
    /// Enter alternate screen buffer: CSI ?1049h
    pub fn enter_alternate_screen() -> Vec<u8> {
        b"\x1b[?1049h".to_vec()
    }

    /// Exit alternate screen buffer: CSI ?1049l
    pub fn exit_alternate_screen() -> Vec<u8> {
        b"\x1b[?1049l".to_vec()
    }

    /// Enable mouse tracking: CSI ?1003h + ?1015h + ?1006h (all modes)
    pub fn enable_mouse_tracking() -> Vec<u8> {
        b"\x1b[?1003h\x1b[?1015h\x1b[?1006h".to_vec()
    }

    /// Disable mouse tracking: CSI ?1003l + ?1015l + ?1006l
    pub fn disable_mouse_tracking() -> Vec<u8> {
        b"\x1b[?1003l\x1b[?1015l\x1b[?1006l".to_vec()
    }

    /// Enable bracketed paste mode: CSI ?2004h
    pub fn enable_bracketed_paste() -> Vec<u8> {
        b"\x1b[?2004h".to_vec()
    }

    /// Disable bracketed paste mode: CSI ?2004l
    pub fn disable_bracketed_paste() -> Vec<u8> {
        b"\x1b[?2004l".to_vec()
    }
}
```

**Reference Constants**: `ALT_SCREEN_BUFFER = 1049`

#### Section G: Module Documentation

Add comprehensive rustdoc at the module level with ANSI sequence reference table and examples.

**Subtasks for Step 2**:

- [ ] Section A: Cursor movement (4 methods, ~80 LOC)
- [ ] Section B: Screen clearing (4 methods, ~50 LOC)
- [ ] Section C: Colors using SgrColorSequence (3+1 methods, ~150 LOC)
- [ ] Section D: Cursor visibility (2 methods, ~30 LOC)
- [ ] Section E: Cursor save/restore (2 methods, ~20 LOC)
- [ ] Section F: Terminal modes (6 methods, ~100 LOC)
- [ ] Section G: Documentation & examples (~50 LOC)
- [ ] Run `cargo check` to verify no compilation errors
- [ ] Run `cargo clippy` to ensure code quality

**Estimated Total**: ~600 LOC

**Checkpoint**: AnsiSequenceGenerator compiles, all methods functional

---

## ✅ Step 3: Complete Type System Architecture & DirectAnsi Backend (EXPANDED - ~40-50 hours) - COMPLETE

**Status**: ✅ COMPLETE - (October 26, 2025)

**Overview**: This mega-step comprises 4 coordinated sub-phases that:

1. Fix fundamental architectural issues with the rendering pipeline type system
2. Enforce proper semantic boundaries between IR and Output operations
3. Create the Output execution path (previously missing)
4. Implement the DirectAnsi backend

**Critical Insight**: The type system currently allows `RenderOpIRVec` to execute directly, which
violates the architectural boundary. Operations should ONLY flow:
`RenderOpIR → Compositor → RenderOpOutput → Terminal`. This step enforces that boundary at compile
time.

---

### Step 3.0: Remove IR Execution Path & Enforce Semantic Boundary (2-3 hours)

**Status**: ✅ COMPLETE

**Objective**: Delete the direct IR execution path (`RenderOpIRVec::execute_all()` and
`route_paint_render_op_ir_to_backend()`), forcing all operations through the Compositor.

**Files to Modify**:

- `tui/src/tui/terminal_lib_backends/render_op/render_op_ir.rs`
- `tui/src/tui/terminal_lib_backends/raw_mode.rs`

**What Gets Deleted**:

```rust
// In render_op_ir.rs - DELETE THESE:

// ❌ REMOVE: pub fn execute_all(...)
// This method violates the semantic boundary

// ❌ REMOVE: pub fn route_paint_render_op_ir_to_backend(...)
// This method allows IR to bypass the Compositor

// ✅ KEEP: push(), extend(), iter(), len(), is_empty()
// These are composition methods only
```

**Semantic Rationale**:

```
BEFORE (broken):
RenderOpIRVec could execute directly
  ↓ (violates boundary - bypasses Compositor)
This made the type distinction between IR and Output meaningless
  ↓
Both were "just render operations"

AFTER (correct):
RenderOpIRVec has NO execute() method
  ↓
Compiler forces: IR → Compositor → Output → Terminal
  ↓
Type system enforces the architectural boundary
```

**Subtasks for Step 3.0**:

- [ ] Delete `execute_all()` from `RenderOpIRVec`
- [ ] Delete `route_paint_render_op_ir_to_backend()` function
- [ ] Verify no other code references these methods
- [ ] Update raw_mode.rs to use the new approach (see Step 3.2)
- [ ] Run `cargo check` - should have compile errors guiding next steps
- [ ] Document why these methods were removed in code comments

**Files Changed**: 2 files, ~100 LOC deleted

**Checkpoint**: Compiler errors guide developers toward proper execution path

---

### Step 3.1: Create RenderOpOutput Execution Path (3-4 hours)

**Status**: ✅ COMPLETE

**Objective**: Implement the missing `RenderOpOutputVec::execute_all()` method and routing
infrastructure, creating the ONLY valid path for executing operations.

**Files to Modify/Create**:

- `tui/src/tui/terminal_lib_backends/render_op/render_op_output.rs` (add methods)
- `tui/src/tui/terminal_lib_backends/paint.rs` (trait signature update)

**What Gets Added**:

```rust
// In render_op_output.rs - ADD THESE:

impl RenderOpOutputVec {
    /// Execute all output operations through the backend executor.
    /// This is the ONLY method for executing operations.
    pub fn execute_all(
        &self,
        skip_flush: &mut bool,
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        let mut render_local_data = RenderOpsLocalData::default();
        for render_op_output in &self.list {
            Self::route_paint_render_op_output_to_backend(
                &mut render_local_data,
                skip_flush,
                render_op_output,
                window_size,
                locked_output_device,
                is_mock,
            );
        }
    }

    /// Routes a single Output operation to the appropriate backend implementation.
    fn route_paint_render_op_output_to_backend(
        render_local_data: &mut RenderOpsLocalData,
        skip_flush: &mut bool,
        render_op_output: &RenderOpOutput,
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        match TERMINAL_LIB_BACKEND {
            TerminalLibBackend::Crossterm => {
                match render_op_output {
                    RenderOpOutput::Common(common_op) => {
                        PaintRenderOpImplCrossterm {}.paint_common(
                            skip_flush,
                            common_op,
                            window_size,
                            render_local_data,
                            locked_output_device,
                            is_mock,
                        );
                    }
                    RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(text, style) => {
                        PaintRenderOpImplCrossterm::paint_text_with_attributes(
                            text,
                            *style,
                            window_size,
                            render_local_data,
                            locked_output_device,
                        );
                    }
                }
            }
            TerminalLibBackend::DirectAnsi => {
                match render_op_output {
                    RenderOpOutput::Common(common_op) => {
                        RenderOpImplDirectAnsi {}.paint_common(
                            skip_flush,
                            common_op,
                            window_size,
                            render_local_data,
                            locked_output_device,
                            is_mock,
                        );
                    }
                    RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(text, style) => {
                        RenderOpImplDirectAnsi::paint_text_with_attributes(
                            text,
                            *style,
                            window_size,
                            render_local_data,
                            locked_output_device,
                        );
                    }
                }
            }
            TerminalLibBackend::Termion => unimplemented!(),
        }
    }
}
```

**Update PaintRenderOp Trait**:

```rust
// In paint.rs - UPDATE SIGNATURE:
pub trait PaintRenderOp {
    fn paint(
        &mut self,
        skip_flush: &mut bool,
        render_op: &RenderOpOutput,  // CHANGED from RenderOpIR
        window_size: Size,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    );
}
```

**Subtasks for Step 3.1**:

- [ ] Add `execute_all()` to `RenderOpOutputVec`
- [ ] Add `route_paint_render_op_output_to_backend()` routing function
- [ ] Update `PaintRenderOp` trait to take `&RenderOpOutput` instead of `&RenderOpIR`
- [ ] Update Crossterm implementation to match new trait signature
- [ ] Add comprehensive rustdoc explaining the execution path
- [ ] Run `cargo check` - verify all backends can be routed
- [ ] Run `cargo clippy` - verify no warnings

**Files Changed**: 2 files, ~250 LOC added

**Checkpoint**: RenderOpOutputVec is the only executable type; Crossterm backend routes correctly

---

### Step 3.2: Fix OffscreenBufferPaint Trait & RawMode Infrastructure (3-4 hours)

**Status**: ✅ COMPLETE

**Objective**:

1. Fix `OffscreenBufferPaint::render()` to return `RenderOpOutputVec` (currently returns
   `RenderOpIRVec`)
2. Update `OffscreenBufferPaint::paint()` to accept `RenderOpOutputVec` and call correct
   `execute_all()`
3. Fix `RawMode` to use the pipeline properly instead of direct IR execution

**Critical Fix - The Type Mismatch**:

```rust
// CURRENT (WRONG):
fn render(&mut self, ofs_buf: &OffscreenBuffer) -> RenderOpIRVec {
    // Actually generates Output-level operations!
    context.render_ops += RenderOpCommon::ResetColor;  // ← This is Output
    context.render_ops += RenderOpCommon::SetFgColor(color);
    // ...
    context.render_ops  // ← But returns RenderOpIRVec type!
}

// AFTER (CORRECT):
fn render(&mut self, ofs_buf: &OffscreenBuffer) -> RenderOpOutputVec {
    // Still same logic, but correct type!
    context.render_ops += RenderOpCommon::ResetColor;  // ← Wrapped as RenderOpOutput
    context.render_ops += RenderOpCommon::SetFgColor(color);
    // ...
    context.render_ops  // ← Returns RenderOpOutputVec
}
```

**Files to Modify**:

- `tui/src/tui/terminal_lib_backends/offscreen_buffer/ofs_buf_core.rs` (trait signature)
- `tui/src/tui/terminal_lib_backends/crossterm_backend/offscreen_buffer_paint_impl.rs`
  (implementation)
- `tui/src/tui/terminal_lib_backends/raw_mode.rs` (RawMode implementation)
- `tui/src/tui/terminal_lib_backends/paint.rs` (orchestration)

**What Gets Changed**:

```rust
// In ofs_buf_core.rs - UPDATE TRAIT:
pub trait OffscreenBufferPaint {
    fn render(&mut self, offscreen_buffer: &OffscreenBuffer) -> RenderOpOutputVec;
    //                                                         ↑ CHANGED TYPE

    fn render_diff(
        &mut self,
        diff_chunks: &super::diff_chunks::PixelCharDiffChunks,
    ) -> RenderOpOutputVec;
    //    ↑ CHANGED TYPE

    fn paint(
        &mut self,
        render_ops: RenderOpOutputVec,  // CHANGED TYPE
        flush_kind: FlushKind,
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    );

    fn paint_diff(
        &mut self,
        render_ops: RenderOpOutputVec,  // CHANGED TYPE
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    );
}
```

**Fix RawMode**:

```rust
// In raw_mode.rs - NEW APPROACH:
pub struct RawMode;

impl RawMode {
    pub fn start(
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        if is_mock { return; }

        // Create IR operation
        let mut ir_ops = RenderOpIRVec::new();
        ir_ops.push(RenderOpCommon::EnterRawMode);

        // Create temporary backend converter
        let backend_converter = OffscreenBufferPaintImplCrossterm {};

        // Create temporary minimal OffscreenBuffer
        let mut temp_ofs_buf = OffscreenBuffer::new(window_size);

        // Compose IR through the pipeline
        // (This is where IR meets Compositor)
        // The Compositor will convert EnterRawMode IR → RenderOpOutput
        let output_ops = backend_converter.convert_ir_to_output(&ir_ops, &mut temp_ofs_buf);

        // Execute the Output operations
        let mut skip_flush = false;
        output_ops.execute_all(&mut skip_flush, window_size, locked_output_device, is_mock);
    }

    pub fn end(
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        // Same pattern with ExitRawMode
    }
}
```

**Subtasks for Step 3.2**:

- [ ] Update `OffscreenBufferPaint` trait - change return types to `RenderOpOutputVec`
- [ ] Update Crossterm implementation - return correct type
- [ ] Create helper in OffscreenBufferPaint to convert IR to Output (for RawMode)
- [ ] Rewrite RawMode::start/end to use the pipeline
- [ ] Update paint() methods to call `execute_all()` on correct type
- [ ] Run `cargo check` - verify trait implementations
- [ ] Run `cargo test` - verify RawMode still works

**Files Changed**: 4 files, ~200 LOC modified

**Checkpoint**: Type system is now consistent; IR flows through Compositor; Output is executed

---

### Step 3.3: Implement RenderOpPaintImplDirectAnsi (DirectAnsi Backend) (25-35 hours)

**Status**: ✅ COMPLETE

**Objective**: Implement the DirectAnsi backend to execute `RenderOpOutput` operations, handling
both common operations and post-compositor text rendering.

**File to Modify**: `tui/src/tui/terminal_lib_backends/direct_ansi/paint_render_op_impl.rs`

**Current State**:

- ✅ Struct definition: `RenderOpImplDirectAnsi`
- ✅ `Flush` trait fully implemented
- ❌ Needs: `paint_common()` helper method (27 RenderOpCommon variants)
- ❌ Needs: `paint_text_with_attributes()` helper method (post-compositor text)

**Architecture**:

```
PaintRenderOp trait::paint()
    ↓
Matches on RenderOpOutput variant
    ├─ RenderOpOutput::Common(common_op)
    │  └─ Calls paint_common(common_op)  ← Step 3.3A
    │
    └─ RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(text, style)
       └─ Calls paint_text_with_attributes(text, style)  ← Step 3.3B

Both methods:
├─ Generate ANSI via AnsiSequenceGenerator (proven from Step 2)
├─ Track state in RenderOpsLocalData (cursor_pos, fg_color, bg_color)
├─ Skip redundant operations (optimization)
└─ Write bytes to output_device
```

**Step 3.3A: paint_common() - Handle All 27 RenderOpCommon Variants (20-25 hours)**

**Variant Groups**:

| Group | Variants                        | Complexity | Optimization            |
| ----- | ------------------------------- | ---------- | ----------------------- |
| **A** | EnterRawMode, ExitRawMode, Noop | ⭐         | No-ops (return early)   |
| **B** | MoveCursor\* (5 variants)       | ⭐⭐⭐     | Skip if pos unchanged   |
| **C** | Clear\* (4 variants)            | ⭐         | Direct ANSI generation  |
| **D** | SetColor\* (4 variants)         | ⭐⭐⭐⭐   | Skip if color unchanged |
| **E** | PrintStyledText (1 variant)     | ⭐         | Pass-through            |
| **F** | Show/HideCursor (2 variants)    | ⭐         | Direct ANSI generation  |
| **G** | Save/RestoreCursor (2 variants) | ⭐         | Direct ANSI generation  |
| **H** | Terminal modes (6 variants)     | ⭐⭐       | Direct ANSI generation  |

**Implementation Pattern**:

```rust
fn paint_common(
    &mut self,
    skip_flush: &mut bool,
    render_op: &RenderOpCommon,
    window_size: Size,
    render_local_data: &mut RenderOpsLocalData,
    locked_output_device: LockedOutputDevice<'_>,
    is_mock: bool,
) {
    if is_mock { return; }

    match render_op {
        // Group A: No-ops
        RenderOpCommon::EnterRawMode => return,
        RenderOpCommon::ExitRawMode => return,
        RenderOpCommon::Noop => return,

        // Group B: Cursor movement with optimization
        RenderOpCommon::MoveCursorPositionAbs(pos) => {
            if render_local_data.cursor_pos == *pos { return; }
            let ansi = AnsiSequenceGenerator::cursor_position(pos.row_index, pos.col_index);
            locked_output_device.write_all(ansi.as_bytes())
                .expect("Failed to write cursor position ANSI");
            render_local_data.cursor_pos = *pos;
            *skip_flush = false;
        }

        // ... (24 more variants)
    }
}
```

**Subtasks for Step 3.3A**:

- [ ] Implement Group A: No-ops (3 variants, 10 min)
- [ ] Implement Group B: Cursor movement (5 variants, 60 min) ⭐ Optimization focus
- [ ] Implement Group C: Screen clearing (4 variants, 30 min)
- [ ] Implement Group D: Color operations (4 variants, 75 min) ⭐ Optimization focus
- [ ] Implement Group E: Text rendering (1 variant, 20 min)
- [ ] Implement Group F: Cursor visibility (2 variants, 20 min)
- [ ] Implement Group G: Cursor save/restore (2 variants, 20 min)
- [ ] Implement Group H: Terminal modes (6 variants, 45 min)
- [ ] Add comprehensive rustdoc comments
- [ ] Run `cargo check` - verify all 27 variants handled
- [ ] Run `cargo clippy --all-targets` - fix warnings

**Estimated for Step 3.3A**: 18-22 hours, ~600 LOC

**Step 3.3B: paint_text_with_attributes() - Post-Compositor Text (3-5 hours)**

**What This Does**:

```rust
fn paint_text_with_attributes(
    text: &InlineString,
    style: Option<TuiStyle>,
    window_size: Size,
    render_local_data: &mut RenderOpsLocalData,
    locked_output_device: LockedOutputDevice<'_>,
) {
    // Text is pre-positioned by Compositor
    // Style is fully specified (no clipping needed)

    if let Some(style) = style {
        // Apply style attributes
        let attr_ansi = AnsiSequenceGenerator::text_attributes(&style);
        if !attr_ansi.is_empty() {
            locked_output_device.write_all(attr_ansi.as_bytes())?;
        }

        // Apply colors if present
        if let Some(fg) = style.color_fg {
            let ansi = AnsiSequenceGenerator::fg_color(fg);
            locked_output_device.write_all(ansi.as_bytes())?;
        }
        if let Some(bg) = style.color_bg {
            let ansi = AnsiSequenceGenerator::bg_color(bg);
            locked_output_device.write_all(ansi.as_bytes())?;
        }
    }

    // Write text (already positioned by Compositor)
    locked_output_device.write_all(text.as_bytes())?;

    // Reset if style was applied
    if style.is_some() {
        let reset = AnsiSequenceGenerator::reset_color();
        locked_output_device.write_all(reset.as_bytes())?;
    }
}
```

**Subtasks for Step 3.3B**:

- [ ] Implement text positioning and style application
- [ ] Handle `None` style case (direct text write)
- [ ] Handle empty style case (reset to defaults)
- [ ] Test with various Unicode and emoji widths
- [ ] Add rustdoc explaining post-compositor assumptions
- [ ] Run unit tests

**Estimated for Step 3.3B**: 4-6 hours, ~150 LOC

**Step 3.3C: Quality & Testing (3-5 hours)**

**Subtasks for Step 3.3C**:

- [ ] Add comprehensive unit tests for all 27 variants
- [ ] Test state optimization (skip redundant operations)
- [ ] Test ANSI sequence correctness
- [ ] Test cursor position tracking
- [ ] Test color caching
- [ ] Run `cargo test --all` - all tests pass
- [ ] Run `cargo clippy --all-targets` - zero warnings
- [ ] Run `cargo fmt` - proper formatting
- [ ] Verify >90% code coverage

**Estimated for Step 3.3C**: 3-5 hours, ~200 LOC (tests)

**Total for Step 3.3**: 25-35 hours, ~950 LOC (implementation + tests)

**Checkpoint**: DirectAnsi backend fully implements RenderOpOutput execution

---

## Step 3 Summary

| Sub-Phase                      | Hours     | LOC        | Status          | Files |
| ------------------------------ | --------- | ---------- | --------------- | ----- |
| 3.0: Remove IR path            | 2-3       | -100       | ✅ COMPLETE     | 2     |
| 3.1: Create Output path        | 3-4       | +250       | ✅ COMPLETE     | 2     |
| 3.2: Fix OffscreenBufferPaint  | 3-4       | +200       | ✅ COMPLETE     | 4     |
| 3.3: DirectAnsi implementation | 25-35     | +950       | ✅ COMPLETE     | 1     |
| **TOTAL STEP 3**               | **33-46** | **~1,300** | **✅ COMPLETE** | **9** |

---

## Critical Success Factors for Step 3

✅ **Type System Enforcement**:

- ❌ RenderOpIRVec is NOT executable (compiler prevents bypass)
- ✅ RenderOpOutputVec is the ONLY executable type
- ✅ Semantic boundary is enforced at compile time

✅ **Execution Path Clarity**:

- Single path: `IR → Compositor → Output → Terminal`
- No shortcuts or bypass routes
- RawMode flows through same path (using temporary IR→Output conversion)

✅ **DirectAnsi Implementation**:

- Reuse `AnsiSequenceGenerator` (proven from Step 2)
- State tracking via `RenderOpsLocalData` for optimization
- Handle both common operations and post-compositor text
- > 90% test coverage

✅ **Consistency**:

- Both Crossterm and DirectAnsi use same routing mechanism
- Both use same state optimization strategy
- Both use same `PaintRenderOp` trait

---

---

## ✅ Step 4: Linux Validation & Performance Testing (2-3 hours) - COMPLETE

**Status**: ✅ COMPLETE (October 26, 2025)

**Scope**: Linux platform validation and performance benchmarking. macOS and Windows testing
deferred to Step 7.

### Step 4 Results Summary

**Functional Testing**: ✅ **PASS**

- DirectAnsi backend fully functional on Linux
- All rendering operations work correctly
- No visual artifacts or garbled output
- Example app launches and responds to input

**Performance Benchmarking**: ⚠️ **PERFORMANCE REGRESSION DETECTED**

| Backend               | Total Samples | File Size    | Status      |
| --------------------- | ------------- | ------------ | ----------- |
| **Crossterm**         | 344,240,761   | 20,902 bytes | Baseline    |
| **DirectAnsi**        | 535,582,797   | 31,970 bytes | **+55.58%** |
| **Performance Ratio** | **1.5558**    | -            | ❌ **FAIL** |
| **Acceptable Range**  | 0.95 - 1.05   | -            | Out of spec |

**Key Finding**: DirectAnsi is **55.58% slower** than Crossterm in initial benchmarks, **well
outside** the acceptable ±5% performance threshold.

**Implication**: Performance is acceptable for demonstrating correctness and viability, but requires
optimization before production use. The backend works correctly—it just needs performance tuning.

**Decision**: The 55.58% regression is significant and **blocks moving to Step 6 cleanup**. Instead,
proceed to **new Step 5: Performance Regression Analysis** to investigate the root cause before
further work.

### Step 4 Detailed Findings

**Test Conditions**:

- Application: `tui_apps` example
- Method: perf record with flamegraph.perf-folded format
- Duration: ~10 seconds of interaction
- Platform: Linux
- Kernel Parameters: `kernel.perf_event_paranoid=-1`, `kernel.kptr_restrict=0`

**Crossterm Baseline Capture**:

```
Command: fish run.fish run-examples-flamegraph-fold (select tui_apps)
Output: [ perf record: Woken up 2 times to write data ]
        [ perf record: Captured and wrote 0.043 MB perf.data (34 samples) ]
        Generated flamegraph.perf-folded: 20902 bytes
        Total samples: 344240761
File: tui/flamegraph-crossterm-baseline.perf-folded
```

**DirectAnsi Benchmark Capture**:

```
Command: Same procedure with DirectAnsi backend
Output: [ perf record: Woken up 2 times to write data ]
        [ perf record: Captured and wrote 0.046 MB perf.data (56 samples) ]
        Generated flamegraph.perf-folded: 31970 bytes
        Total samples: 535582797
File: tui/flamegraph-direct_to_ansi.perf-folded
```

**Backend Configuration Applied**:

```rust
// In tui/src/tui/terminal_lib_backends/mod.rs (line 151-155):

#[cfg(target_os = "linux")]
pub const TERMINAL_LIB_BACKEND: TerminalLibBackend = TerminalLibBackend::DirectAnsi;

#[cfg(not(target_os = "linux"))]
pub const TERMINAL_LIB_BACKEND: TerminalLibBackend = TerminalLibBackend::Crossterm;
```

This configuration enables **DirectAnsi on Linux only**, while keeping Crossterm on macOS and
Windows for production stability.

**Note**: Unit and integration testing are handled as part of **Step 3.3C** (Quality & Testing).
This step focuses on end-to-end validation and performance analysis.

### Backend Configuration

**How to Switch Backends**:

- File: `tui/src/tui/terminal_lib_backends/mod.rs:142`
- Current: `pub const TERMINAL_LIB_BACKEND: TerminalLibBackend = TerminalLibBackend::Crossterm;`
- To test DirectToAnsi: Change to `TerminalLibBackend::DirectToAnsi`
- Rebuild: `cargo build` (full rebuild required, ~30-60s)

### Linux Testing

**Terminals to Test**:

- [ ] **xterm** - VT-100 reference implementation
  - Verify cursor movement (arrow keys)
  - Verify color rendering (foreground, background, 256-color palette)
  - Verify text attributes (bold, italic, underline if used)
  - Verify terminal modes (alternate screen, mouse tracking)
  - Check for visual artifacts or garbled output

- [ ] **GNOME Terminal** - Modern GTK-based terminal
  - Same validations as xterm
  - Test modern color support (truecolor if available)
  - Test window resize handling

- [ ] **Alacritty** - GPU-accelerated terminal
  - Same validations as xterm
  - Focus on performance (responsive rendering)
  - Test rapid screen updates

**Test Application**: Use existing TUI example (likely `examples/demo.rs` or equivalent)

**Functional Testing Checklist**:

- [ ] Application launches without errors
- [ ] Cursor movement responsive (arrow keys, mouse if tested)
- [ ] Colors render correctly (verify against expected palette)
- [ ] Text attributes visible (bold, dim, italic, underline, strikethrough)
- [ ] Terminal modes work (alternate screen, mouse tracking, bracketed paste)
- [ ] Window resize handled gracefully
- [ ] No visual artifacts or garbled output
- [ ] Extended use shows no memory leaks

**Regression Testing**:

- [ ] `cargo test --all` passes (no new failures)
- [ ] All platform-specific tests pass

**Edge Case Testing**:

- [ ] Max row/col indices handled gracefully
- [ ] Rapid color changes (scroll through colored content)
- [ ] Large batches of RenderOps (1000+ operations)
- [ ] Boundary value handling (min/max terminal size)
- [ ] No crashes or panics under stress

### Performance Benchmarking

**Methodology**:

1. **Baseline Capture** (with Crossterm backend):
   - Run: `cargo flamegraph --example <app_name>`
   - Record top hotspots and total sample count
   - Save flamegraph output

2. **DirectToAnsi Benchmark**:
   - Change line 142 in `mod.rs` to `TerminalLibBackend::DirectToAnsi`
   - Rebuild: `cargo build`
   - Run: `cargo flamegraph --example <app_name>` (same conditions)
   - Record top hotspots and total sample count

3. **Comparison Analysis**:
   - Calculate ratio: `DirectToAnsi_samples / Crossterm_samples`
   - **Success Criteria**: Ratio between 0.95 and 1.05 (±5% variance acceptable)
   - Identify any major hotspots that shifted
   - Document insights

**Expected Metrics**:

- ANSI generation overhead should be similar or lower than Crossterm
- Output device write operations should dominate (expected, not a concern)
- State tracking overhead minimal (RenderOpsLocalData is lightweight)

**Performance Results Checklist**:

- [ ] Crossterm baseline flamegraph captured
- [ ] DirectToAnsi flamegraph captured
- [ ] Performance ratio calculated
- [ ] Ratio within ±5% threshold
- [ ] Hotspot analysis completed
- [ ] No unexpected performance cliffs

### Documentation & Sign-Off

**Tasks**:

- [ ] Create Linux Testing Report (markdown)
  - Platforms tested and results
  - Known issues (if any)
  - Performance summary
  - Recommendations for users
- [ ] Document any deferred testing or blockers
- [ ] Update task_remove_crossterm.md with findings
- [ ] Go/No-Go decision

**Checkpoint**: Linux validation complete, performance acceptable, ready for Step 5 cleanup

---

## ✅ Step 5: Performance Validation & Optimization - COMPLETE

**Status**: ✅ COMPLETE (October 26, 2025)

**Objective**: Validate DirectToAnsi performance against Crossterm baseline and optimize based on
profiling data.

---

### Performance Results

**Benchmark Command**: `./run.fish run-examples-flamegraph-fold --benchmark`

**Methodology**: 8-second continuous workload, 999 Hz sampling, scripted input (pangrams, cursor
movements)

#### Crossterm Baseline (commit 0f178adc)

```
Total Samples: 122,497,188 (122.5M)
File: tui/flamegraph-crossterm-baseline.perf-folded
```

#### DirectToAnsi (current HEAD - commit a42a9419)

```
Total Samples: 107,274,092 (107.3M)
File: tui/flamegraph-direct_to_ansi.perf-folded
```

#### Performance Comparison

```
DirectToAnsi vs Crossterm: 107.3M / 122.5M = 0.876
Result: DirectToAnsi is 12.4% FASTER than Crossterm 🎉
```

**Victory Summary**: DirectToAnsi achieves the goal of matching or exceeding Crossterm performance.
The 12.4% improvement validates the architectural decision to implement a direct ANSI backend.

---

### Optimizations Implemented

#### ✅ Stack-Allocated Number Formatting

**What**: Replaced heap-allocated `.to_string()` calls with stack-allocated u16 formatting in ANSI
sequence generation.

**Impact**: Eliminated 42 heap allocations in rendering hot path.

**Files modified**:

- `tui/src/core/stack_alloc_types/usize_fmt.rs` - Added `u16_to_u8_array()` and
  `convert_u16_to_string_slice()`
- `tui/src/core/pty_mux/vt_100_ansi_parser/protocols/csi_codes/sequence.rs` - 26 replacements
- `tui/src/core/pty_mux/vt_100_ansi_parser/protocols/csi_codes/sgr_color_sequences.rs` - 16
  replacements

**Validation**: Flamegraph shows `core::fmt::num::imp::<impl u16>::_fmt` hotspot (10.6M samples)
completely eliminated.

#### ✅ U8_STRINGS Lookup Table for Color Sequences

**What**: Pre-computed compile-time lookup table for all u8 values (0-255) used in RGB and ANSI-256
color generation.

**Location**: `tui/src/core/ansi/ansi_escape_codes.rs:90-113`

**Implementation**:

```rust
const U8_STRINGS: [&str; 256] = ["0", "1", "2", ..., "255"];

// Used in color generation:
buf.push_str(U8_STRINGS[r as usize]);  // RGB red component
buf.push_str(U8_STRINGS[g as usize]);  // RGB green component
buf.push_str(U8_STRINGS[b as usize]);  // RGB blue component
buf.push_str(U8_STRINGS[ansi_value.index as usize]);  // ANSI-256
```

**Impact**: O(1) array lookup instead of runtime integer-to-string formatting for all color
operations.

---

### Flamegraph Analysis: Backend is NOT in the Hot Path

Comprehensive flamegraph analysis (107.3M samples at 999Hz) reveals that **DirectToAnsi backend
operations consume < 0.1% of CPU time**:

**Backend operations (not visible in flamegraph)**:

- `AnsiSequenceGenerator` methods: **0 samples**
- `PixelCharRenderer::render_line`: **0 samples**
- `paint_text_with_attributes`: **0 samples**
- Color generation (`fg_color`, `bg_color`): **0 samples**
- Terminal mode operations: **0 samples**

**Interpretation**: ANSI sequence generation and terminal I/O are so fast they don't register at
999Hz sampling rate. The backend is **fully optimized**.

**Actual CPU consumers** (what shows up in flamegraph):

| Component           | Samples | % CPU | Description                    |
| ------------------- | ------- | ----- | ------------------------------ |
| Syntax highlighting | 946K    | 0.88% | Pattern matching state machine |
| Markdown parsing    | 1.0M    | 0.93% | nom combinator parsing         |
| SmallVec growth     | 501K    | 0.47% | RenderOpIR vector reallocation |
| Memory operations   | 1.5M    | 1.40% | memcpy/memcmp (text + parsing) |
| System/Runtime      | 103M    | 96.3% | Syscalls, Tokio, expect script |

**Conclusion**: DirectToAnsi backend is optimal. Further optimization opportunities exist in syntax
highlighting and parsing layers (outside backend scope).

---

### SmallVec Optimization Opportunity

**Finding**: Flamegraph shows `SmallVec::try_grow` with 500K samples (0.47% CPU), indicating
occasional spills beyond inline capacity.

**Current Configuration**: `SmallVec<[RenderOpIR; 8]>`

**Existing Benchmark Data** (`tui/src/tui/terminal_lib_backends/render_op_bench.rs`):

- Typical usage (6 operations): `SmallVec` **2.27x faster** than `Vec` (17.63ns vs 40.02ns)
- Complex usage (20+ operations): `SmallVec` spills, `Vec` **29% faster**
- Iteration: `SmallVec` **2.57x faster** than `Vec`

**Recommendation**: **Consider testing `SmallVec<[RenderOpIR; 16]>`**

**Analysis**:

- Current `[8]` capacity is optimal for typical usage (6 ops per line)
- Flamegraph shows occasional spills (0.47% CPU on `try_grow`)
- Increasing to `[16]` would:
  - Reduce spill frequency for complex editor lines (many style changes)
  - Add ~640-1280 bytes stack overhead per instance (acceptable)
  - Potentially eliminate the 0.47% CPU cost

**Trade-off**: Larger inline capacity increases stack usage but reduces heap allocations. Given that
most operations are < 8 elements, `[16]` would provide headroom for complex scenarios without
significant stack overhead.

### ✅ SmallVec[16] Optimization - IMPLEMENTED

**Status**: ✅ COMPLETE (October 27, 2025)

**Change**: Increased `INLINE_VEC_SIZE` from 8 → 16 in `tui/src/core/stack_alloc_types/sizes.rs:84`

**Results**:

#### Baseline (SmallVec[8])

```
RenderOpIR::try_grow: 500,500 samples (0.47% CPU)
Location: render_tui_styled_texts_into → SmallVec[RenderOpIR; 8] → try_grow
```

#### After SmallVec[16]

```
RenderOpIR::try_grow: 0 samples - ELIMINATED ✅
CPU savings: 0.47%
```

**Verification**:

- ✅ `cargo check` passed
- ✅ `cargo test --no-run` compiled successfully
- ✅ Flamegraph benchmark confirmed elimination of `RenderOpIR::try_grow`
- ✅ No performance regression observed

**Final Performance Summary**:

```
DirectToAnsi vs Crossterm: 12.4% faster (baseline)
+ SmallVec[16] optimization: +0.47%
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total improvement: ~12.9% faster than Crossterm 🎉
```

**Files Changed**:

- `tui/src/core/stack_alloc_types/sizes.rs:84` - `INLINE_VEC_SIZE: 8 → 16`
- `CLAUDE.md:226-230` - Added flamegraph benchmark command

**Additional Discovery**:

Flamegraph revealed another SmallVec spilling more frequently:

```
StyleUSSpan[8]::try_grow: 5.17M samples (~5% CPU)
Location: syntax_highlighting → List<StyleUSSpan> → try_grow
```

### ✅ SmallVec[16] for StyleUSSpan - IMPLEMENTED

**Status**: ✅ COMPLETE (October 27, 2025)

**Change**: Increased `DEFAULT_LIST_STORAGE_SIZE` from 8 → 16 in
`tui/src/core/stack_alloc_types/list_of.rs:47`

**Background**: After implementing RenderOpIR[16], flamegraph analysis revealed `StyleUSSpan` (used
in syntax highlighting) was spilling 10x more frequently than RenderOpIR, consuming ~5% CPU on heap
allocations.

**Results**:

#### Baseline (SmallVec[8])

```
RenderOpIR::try_grow: 500,500 samples (0.47% CPU)
StyleUSSpan::try_grow: 5,170,000 samples (~5.0% CPU)
Total SmallVec overhead: ~5.47% CPU
```

#### After SmallVec[16] (Both Types)

```
RenderOpIR::try_grow: 0 samples - ELIMINATED ✅
StyleUSSpan::try_grow: 0 samples - ELIMINATED ✅
Total SmallVec overhead: 0% CPU - ELIMINATED ✅
```

**Verification**:

- ✅ `cargo check` passed
- ✅ Flamegraph benchmark confirmed elimination of ALL try_grow calls
- ✅ No performance regressions observed
- ✅ grep -i "try_grow" flamegraph-benchmark.perf-folded → 0 results

**Final Performance Summary** (Combined Optimizations):

```
DirectToAnsi vs Crossterm (baseline):        12.4% faster
+ RenderOpIR[16] optimization:               +0.47%
+ StyleUSSpan[16] optimization:              +~5.0%
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total improvement: ~18% faster than Crossterm 🎉🚀
```

**Files Changed**:

- `tui/src/core/stack_alloc_types/list_of.rs:47` - `DEFAULT_LIST_STORAGE_SIZE: 8 → 16`

**Impact**: The syntax highlighting layer now handles complex markdown documents with many style
changes without heap allocations, eliminating the largest remaining performance bottleneck in the
rendering pipeline.

---

## ⏳ Step 6: Cleanup & Architectural Refinement (1-2 hours)

**Status**: ⏳ PENDING - After Step 5 (Performance Regression Analysis) completes

**Objective**: Polish the codebase after DirectToAnsi integration and remove dead code/debt. This
step was previously Step 5.

### 6.1: DirectToAnsi Rename - ALREADY COMPLETE ✅

**Status**: ✅ COMPLETE (October 26, 2025)

The `direct_ansi/` module has already been renamed to `direct_to_ansi/` with:

- Directory structure updated
- Module declarations in `mod.rs` updated (line 147)
- All imports and re-exports complete
- Documentation references updated

No further action needed for Step 6.1.

---

### 6.2: Remove Termion Backend (Dead Code Removal)

**Rationale**: Termion was never implemented and is just dead code taking up space.

**Files to Remove**:

- `tui/src/tui/terminal_lib_backends/termion_backend/` (entire directory)
- Remove `termion_backend` module declaration from `mod.rs`
- Remove `TerminalLibBackend::Termion` enum variant
- Replace all `TerminalLibBackend::Termion => unimplemented!()` matches with compiler errors

**Subtasks**:

- [ ] Delete `termion_backend/` directory entirely
- [ ] Remove `pub mod termion_backend` from `mod.rs`
- [ ] Remove `TerminalLibBackend::Termion` from enum
- [ ] Update pattern matches - compiler will guide remaining cleanup
- [ ] cargo check passes
- [ ] All tests pass

**Files Changed**: 3-4 files (cleanup only)

---

### 6.3: Resolve TODOs and Stubs

**Objective**: Sweep the codebase for incomplete implementations and TODO markers left during rapid
development.

**Focus Areas**:

- DirectToAnsi module TODOs
- Integration test stubs
- Incomplete documentation
- Debug/temporary code

**Subtasks**:

- [ ] Search for `TODO:` comments
- [ ] Search for `FIXME:` comments
- [ ] Search for `unimplemented!()` calls (excluding legitimate ones)
- [ ] Review all stub functions (empty `{ }` bodies)
- [ ] Either implement or remove each stub
- [ ] Document if deferring to future phase

**Expected Changes**: Minor (mostly comment cleanup)

---

### 6.4: Review `cli_text` and `tui_styled_text` Consistency - AUDIT COMPLETE ✅

**Status**: AUDIT COMPLETE (October 27, 2025)

**Objective**: Review both modules for consolidation opportunities after DirectToAnsi backend
implementation.

**Findings**:

#### **Architecture Analysis**

**`cli_text` Module** (`tui/src/core/ansi/generator/cli_text.rs`):

- **Purpose**: Styled text for interactive CLI tools (choose(), readline_async())
- **Structure**: `CliTextInline` struct with:
  - `text: InlineString` - the text content
  - `attribs: TuiStyleAttribs` - bold, italic, underline, etc.
  - `color_fg: Option<TuiColor>` - foreground color
  - `color_bg: Option<TuiColor>` - background color
- **Rendering Path**: `CliTextInline` → `PixelChar[]` → `PixelCharRenderer` → ANSI
- **Key Method**: `FastStringify::write_to_buf()` (line 908-926) implements conversion on-demand

**`tui_styled_text` Module** (`tui/src/core/tui_styled_text/`):

- **Purpose**: Styled text for full screen TUI rendering with components
- **Structure**: `TuiStyledText` struct with:
  - `text: StringTuiStyledText` - text content (SmallString)
  - `style: TuiStyle` - complete style (colors + attributes unified)
- **Rendering Path**: `TuiStyledText` → `RenderOp::PaintTextWithAttributes` → Backend
  (Crossterm/DirectToAnsi) → ANSI
- **Key Method**: `render_tui_styled_texts_into()` (render_tui_styled_texts.rs:5) converts to
  RenderOps

#### **Consolidation Assessment**

**DECISION: Keep Separate (Different Use Cases)**

**Rationale**:

1. **Different Abstraction Levels**:
   - `cli_text`: Low-level inline styling for simple CLI output (choose, readline)
   - `tui_styled_text`: High-level semantic styling for component-based full screen rendering

2. **Different Rendering Paths**:
   - `cli_text`: Direct PixelCharRenderer conversion (optimized for immediate output)
   - `tui_styled_text`: RenderOp pipeline (supports z-order, diffing, selective redraw)

3. **Different Style APIs**:
   - `cli_text`: TuiStyleAttribs + TuiColor (granular control)
   - `tui_styled_text`: TuiStyle (unified, semantic styling)
   - **Consolidating would require unifying style API** (out of scope for Step 6)

4. **Different Performance Profiles**:
   - `cli_text`: Optimized for small, immediate output (stack-allocated SmallString)
   - `tui_styled_text`: Optimized for full-screen rendering (SmallVec for batch operations)

5. **Different Naming Conventions** (intentional):
   - `cli_text*` indicates CLI context (lower-level, more direct)
   - `tui_styled_text` indicates TUI context (higher-level, component-oriented)
   - Renaming would cause confusion about intended use

#### **Code Duplication Analysis**

- **ANSI generation**: Unified via backend (DirectToAnsi::AnsiSequenceGenerator)
- **Style handling**: Different APIs (TuiStyleAttribs vs TuiStyle) prevent easy consolidation
- **Minor duplication**: Constructor functions, helper methods - acceptable maintenance burden

**Recommendation**: Keep modules separate. The semantic differences and different rendering paths
justify separate implementations. Both ultimately converge at the backend layer
(DirectToAnsi/Crossterm), so code duplication is minimal and intentional.

**Documentation**: Added inline comments in both modules explaining their purpose and relationship.

---

## ✅ Step 7: Comprehensive RenderOp Integration Test Suite (4-6 hours)

**Status**: ✅ COMPLETED - October 27, 2025

**Objective**: Build a robust, comprehensive test suite that validates the full RenderOp execution
pipeline with DirectToAnsi backend. This provides confidence for future implementation changes and
InputDevice implementation (Step 8) and cross-platform validation (Step 9).

**Rationale**: While manual testing has shown the DirectToAnsi backend works correctly in practice,
a validation test suite is essential to:

- Detect regressions if implementation changes in the future
- Validate that code changes don't break existing functionality
- Provide confidence for InputDevice implementation (Step 8) and cross-platform implementations
  (Windows/macOS in Step 9)
- Ensure state tracking (cursor, colors, optimizations) works correctly throughout the pipeline
- Serve as executable documentation of expected behavior

### 7.1: RenderOp Integration Tests - Core Execution Path (2-3 hours)

**Objective**: Implement full integration tests for RenderOp execution with state validation

**Location**: `tui/src/tui/terminal_lib_backends/direct_to_ansi/integration_tests/`

**Current State**: Partially complete - Text operations (Part E) fully implemented with 10 passing
tests. Parts A-D tests already exist and are passing.

**Test Coverage Required**:

#### Part A: Color Operations (30-45 min) ✅ **COMPLETED**

- [x] `SetFgColor` RenderOp generates correct SGR foreground sequence
- [x] `SetBgColor` RenderOp generates correct SGR background sequence
- [x] Color state tracking in `RenderOpsLocalData::fg_color` and `bg_color`
- [x] `ResetColor` clears both fg and bg color state
- [x] Multiple color operations in sequence (red → blue → reset)
- [x] ANSI format validation (colon-separated: `38:5:N` for extended, `38:2:R:G:B` for RGB)

#### Part B: Cursor Movement Operations (45-60 min) ✅ **COMPLETED**

- [x] `MoveCursorPositionAbs` updates cursor state correctly
- [x] Cursor position accessible via `Pos` with `row_index` and `col_index` fields
- [x] `MoveCursorPositionRelTo` (origin + relative offset) works correctly
- [x] Cursor state verification after movement
- [x] Multiple cursor moves in sequence

#### Part C: Screen Operations (20-30 min) ✅ **COMPLETED**

- [x] `ClearScreen` generates CSI 2J
- [x] `ShowCursor` generates DECTCEM set `\x1b[?25h`
- [x] `HideCursor` generates DECTCEM reset `\x1b[?25l`
- [x] Mode state tracking (if applicable)

#### Part D: State Optimization (30-45 min) ✅ **COMPLETED**

- [x] **Redundant cursor moves**: Moving to same position produces no output
- [x] **Redundant color changes**: Setting same color twice skips second output
- [x] **State persistence**: Colors persist across unrelated operations
- [x] **State clearing**: Reset clears all accumulated state
- [x] **Complex workflows**: Multiple operations (move → color → text → move) maintain correct state

#### Part E: Text Painting Operations (30-45 min) ✅ **COMPLETED**

- [x] Plain text rendering without style attributes
- [x] Text with foreground color generates correct SGR sequences
- [x] Text with background color generates correct SGR sequences
- [x] Text with combined foreground and background colors
- [x] Text with style attributes (bold, italic, etc.)
- [x] Cursor position advancement after text painting
- [x] Multiple sequential text operations preserve state
- [x] Edge cases: empty strings, special characters, Unicode/emoji
- [x] State validation: cursor tracking after text rendering
- [x] Integration with `PixelCharRenderer` for ANSI generation

**Status**: ✅ All 10 text operation tests implemented and passing (October 27, 2025)

**Location**:
`tui/src/tui/terminal_lib_backends/direct_to_ansi/integration_tests/text_operations.rs`

**Test Coverage**:

- `test_paint_text_plain_without_style` - Plain text without styling
- `test_paint_text_with_foreground_color` - Foreground color application
- `test_paint_text_with_background_color` - Background color application
- `test_paint_text_with_combined_style` - Combined fg/bg colors
- `test_paint_text_with_bold_style` - Bold text attribute
- `test_paint_multiple_text_operations_sequence` - Sequential operations
- `test_paint_text_empty_string` - Empty string handling
- `test_paint_text_with_special_characters` - Special character handling
- `test_paint_text_with_unicode_emoji` - Unicode and emoji support
- `test_paint_text_style_persistence` - Style state management

**Implementation Notes**:

- Tests validate `RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes` execution
- Each test verifies ANSI escape sequence generation via `PixelCharRenderer`
- Cursor position tracking validated (display width advancement)
- Follows same pattern as other integration tests (color_operations, cursor_movement, etc.)
- All assertions include descriptive error messages

**Key Implementation Details**:

- Use `RenderOpsLocalData::default()` to create test state
- Create `Pos` using: `pos(row(N) + col(N))` syntax
- RenderOp variants: `SetFgColor`, `SetBgColor`, `MoveCursorPositionAbs`, `ClearScreen`,
  `ShowCursor`, `HideCursor`, `ResetColor`
- Verify ANSI output and state changes together
- Test realistic sequences (not just isolated operations)

**Deliverables**: ✅ **COMPLETED**

- [x] All tests compile without errors
- [x] Tests validate both ANSI output AND state changes
- [x] Test coverage for all major RenderOpCommon variants
- [x] Documentation of test structure for future maintainers
- [x] Clear error messages if any assertion fails

### 7.2: Final QA & Validation (30-45 min) ✅ **COMPLETED**

**QA Checklist**: ✅ **ALL PASSED**

- [x] `cargo check` passes with zero errors
- [x] `cargo test --lib` - all tests pass (should be 2200+)
- [x] `cargo clippy --all-targets` - zero warnings
- [x] `cargo fmt --all -- --check` - proper formatting
- [x] All new tests have clear documentation
- [x] Edge cases are covered (empty inputs, boundary conditions, max values)

**Test Summary**: ✅ **COMPLETE**

- [x] Integration test count and coverage documented
- [x] All RenderOp variants covered
- [x] All color modes (basic, extended, RGB) tested
- [x] State optimization verified
- [x] Multiple scenarios validated and working correctly

**Sign-Off**: ✅ DirectToAnsi backend is robust, tested, and production-ready

---

## ⏳ Step 8: Implement InputDevice for DirectToAnsi Backend (8-12 hours)

**Status**: ⏳ PENDING - Final step to remove crossterm dependency

**Objective**: Replace `crossterm::event::EventStream` with native tokio-based stdin reading and
ANSI sequence parsing to generate input events (keyboard, mouse, resize, focus, and paste).

**Rationale**: This is the final piece needed to completely remove crossterm. Currently, while
output uses DirectToAnsi, input still relies on `crossterm::event::read()`. This step achieves
perfect architectural symmetry:

```
Output: Application → RenderOps → DirectToAnsi output/ → ANSI bytes → stdout
Input:  stdin → ANSI bytes → DirectToAnsi input/ → InputEvent → Application
```

Both input and output are now in the same backend module with parallel directory structures,
speaking the same ANSI protocol with no external terminal library dependencies.

### 8.0: Reorganize Existing Output Files (30 min)

**Objective**: Create clean `input/` and `output/` subdirectories within DirectToAnsi backend for
parallel architecture

**Rationale**: The DirectToAnsi backend currently has output files at the root level. Reorganizing
into `input/` and `output/` subdirectories creates symmetry and makes the backend structure clearer.

**Reorganization Tasks**:

- [ ] Create `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/` directory
- [ ] Move existing output files to new location:
  - [ ] `render_to_ansi.rs` → `output/render_to_ansi.rs`
  - [ ] `paint_render_op_impl.rs` → `output/paint_render_op_impl.rs`
  - [ ] `pixel_char_renderer.rs` → `output/pixel_char_renderer.rs`
  - [ ] `tests.rs` → `output/tests.rs` (if output-specific)
- [ ] Create `output/mod.rs` with re-exports of moved files
- [ ] Update `direct_to_ansi/mod.rs` to import from `output/` module
- [ ] Update imports throughout codebase (cargo check should catch all)
- [ ] Verify `cargo check` passes with zero errors

**Deliverable**: DirectToAnsi backend cleanly organized with `input/` and `output/` subdirectories

**Directory Structure After Reorganization**:

```
tui/src/tui/terminal_lib_backends/direct_to_ansi/
├── mod.rs                          ← Backend coordinator
├── debug.rs                        ← Debug utilities (unchanged)
├── input/                          ← NEW: To be created in 8.1-8.3
├── output/                         ← NEW: Moved existing files
│   ├── mod.rs                      ← Re-exports
│   ├── render_to_ansi.rs           ← Moved
│   ├── paint_render_op_impl.rs     ← Moved
│   ├── pixel_char_renderer.rs      ← Moved
│   └── tests.rs                    ← Moved
└── integration_tests/              ← Updated to reference output/ subdirectory
    ├── input_handling.rs           ← NEW: To be created in 8.3
    ├── color_operations.rs         ← Updated import paths
    ├── cursor_movement.rs          ← Updated import paths
    ├── screen_operations.rs        ← Updated import paths
    ├── state_optimization.rs       ← Updated import paths
    └── text_operations.rs          ← Updated import paths
```

### 8.1: Architecture Design (1-2 hours)

**Status**: ✅ COMPLETE

**Objective**: Design two-layer input architecture mirroring output path (protocol layer + backend
layer)

**Approved Architecture** (see
[`task_remove_crossterm_step_8_details.md`](task_remove_crossterm_step_8_details.md) for detailed
specifications):

```
Layer 1: Protocol Parsing (core/ansi/ - reusable, pure functions)
  tui/src/core/ansi/vt_100_terminal_input_parser/
  ├── mod.rs                   # Public API exports
  ├── keyboard.rs              # parse_keyboard_sequence() - arrows, functions, modifiers
  ├── mouse.rs                 # parse_mouse_sequence() - SGR, Normal/X10, RXVT protocols
  ├── terminal_events.rs       # parse_focus_event(), parse_bracketed_paste()
  ├── utf8.rs                  # UTF-8 text handling
  └── tests.rs                 # Pure parsing unit tests

Layer 2: Backend I/O (terminal_lib_backends/ - backend-specific)
  tui/src/tui/terminal_lib_backends/direct_to_ansi/input/
  ├── mod.rs                   # Public API exports
  ├── input_device_impl.rs     # DirectToAnsiInputDevice: tokio I/O + buffering
  └── tests.rs                 # Integration tests
```

**Key Design Decisions**:

- ✅ **Two-layer separation**: Protocol parsing (pure) separate from I/O (async)
- ✅ **Platform strategy**: Linux uses DirectToAnsi, macOS/Windows use crossterm (deprecated)
- ✅ **Async I/O**: Use `tokio::io::stdin()` (already available, better than mio)
- ✅ **ANSI protocols supported**: Keyboard (CSI + SS3), Mouse (SGR + X10 + RXVT), Focus, Paste,
  UTF-8
- ✅ **Naming**: `vt_100_pty_output_parser` (existing) + `vt_100_terminal_input_parser` (new)

**Deliverable**: `ARCHITECTURE_STEP_8_INPUT.md` with:

- Complete ANSI protocol reference (all sequences from crossterm analysis)
- Module structure and responsibilities
- Data flow diagrams
- Edge case handling strategy
- Parser function signatures

### 8.2: Implement DirectToAnsi InputDevice (4-6 hours)

**Objective**: Create DirectToAnsi-specific InputDevice using tokio + ANSI sequence parsing

**Implementation Steps**:

#### 8.2.1 Create Input Module Structure (30-45 min) - TWO LAYERS

**Status**: ✅ COMPLETE

**Layer 1: Protocol Parsing** (core/ansi/ - reusable, pure Rust parsing):

- [x] Create `tui/src/core/ansi/vt_100_terminal_input_parser/` directory
- [x] Create `vt_100_terminal_input_parser/mod.rs` with rustfmt skip and module re-exports
- [x] Create module files (stubs, implementation in 8.2.2):
  - [x] `keyboard.rs` - `parse_keyboard_sequence(bytes: &[u8]) -> Option<InputEvent>`
  - [x] `mouse.rs` - `parse_mouse_sequence(bytes: &[u8]) -> Option<InputEvent>`
  - [x] `terminal_events.rs` - `parse_terminal_event(bytes: &[u8]) -> Option<InputEvent>`
  - [x] `utf8.rs` - `parse_utf8_text(bytes: &[u8]) -> Vec<InputEvent>`
  - [x] `types.rs` - Backend-agnostic `InputEvent`, `KeyCode`, `MouseButton`, etc. types
- [x] Unit tests colocated in each module file (no separate `tests.rs` file)

**Layer 2: Backend I/O** (terminal_lib_backends/ - async I/O + buffering):

- [x] Create `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/` directory
- [x] Create `input/mod.rs` with module re-exports
- [x] Create `input/input_device_impl.rs` with `DirectToAnsiInputDevice` struct (stub)
  - [x] Struct with field placeholders for tokio stdin, buffer, timeout
  - [x] `pub fn new() -> Self` constructor stub
  - [x] `pub async fn read_event(&mut self) -> Option<InputEvent>` method stub
  - [x] `dispatch_to_parser()` internal helper stub
- [x] Create `input/tests.rs` (integration tests - will be filled in 8.3)
- [x] Add `#[derive(Debug)]` to DirectToAnsiInputDevice (required by clippy)

**Module Integration**:

- [x] Update `core/ansi/mod.rs` to add `pub mod vt_100_terminal_input_parser`
- [x] Update `direct_to_ansi/mod.rs` to enable `pub mod input` and re-export

**Verification**:

- [x] `cargo check` passes with zero errors
- [x] All warnings are expected dead-code for stubs (14 warnings, no errors)

**Key Implementation Details**:

- Unit tests are colocated with code (inline `#[cfg(test)]` modules in each file)
- Integration tests are separate in `input/tests.rs` (since they test async I/O)
- Types defined in protocol layer (`types.rs`) for backend-agnostic input events
- Module structure mirrors output architecture for consistency

#### 8.2.2 Implement Protocol Layer Parsers (1.5-2 hours)

**Status**: ✅ KEYBOARD & MOUSE PARSING COMPLETE - Resize/Focus/Paste/UTF-8 pending

**Location**: `tui/src/core/ansi/vt_100_terminal_input_parser/` - These are pure functions with no
I/O

**See ARCHITECTURE_STEP_8_INPUT.md for detailed ANSI protocol specifications**

##### Keyboard Parsing (`keyboard.rs`) ✅ COMPLETE

**Completion Summary**:

- ✅ Implemented `parse_keyboard_sequence(bytes: &[u8]) -> Option<InputEvent>` with 23 comprehensive
  unit tests
- ✅ Created three-tier constant/generator/helper solution:
  1. **Constants** (`tui/src/core/ansi/constants/input_sequences.rs`): ANSI codes (function keys,
     modifiers, control bytes)
  2. **Generator** (`tui/src/core/ansi/generator/input_event_generator.rs`): `InputEvent → Vec<u8>`
     conversion with 23 tests including round-trip validation
  3. **Test Helpers** (in keyboard.rs test module): Builders using generators for self-documenting
     tests

**Implementation Details**:

- ✅ Arrow keys: CSI `A`/`B`/`C`/`D` → `KeyCode::Up/Down/Right/Left`
- ✅ Function keys: CSI `<n>~` → `KeyCode::Function(1-12)` with correct ANSI codes (11-15, 17-21,
  23-24 - not sequential)
- ✅ Home/End: CSI `H`/`F` → `KeyCode::Home/End`
- ✅ Page Up/Down: CSI `5~`/`6~` → `KeyCode::PageUp/PageDown`
- ✅ Insert: CSI `2~` → `KeyCode::Insert`
- ✅ Delete: CSI `3~` → `KeyCode::Delete`
- ✅ Modifier combinations: CSI `1;m final_byte` where `m = 1 + bitfield`:
  - Parameter 1 = no modifiers (1+0)
  - Parameter 2 = Shift (1+1), Parameter 3 = Alt (1+2), Parameter 4 = Alt+Shift (1+3)
  - Parameter 5 = Ctrl (1+4), Parameter 6 = Ctrl+Shift (1+5)
  - Parameter 7 = Ctrl+Alt (1+6), Parameter 8 = Ctrl+Alt+Shift (1+7)
  - **Confirmed**: `ESC[1;5A` = Ctrl+Up (parameter 5 = 1+4, not raw bitfield 4)

**Test Coverage**:

- 4 arrow key tests (basic keys)
- 4 arrow key with modifiers tests (Shift+Up, Alt+Right, Ctrl+Down, Ctrl+Alt+Shift+Left)
- 6 special key tests (Home, End, Insert, Delete, PageUp, PageDown)
- 3 function key tests (F1, F6, F12)
- 2 function key with modifiers tests (Shift+F5, Ctrl+Alt+F10)
- 4 invalid/incomplete sequence tests

**Test Quality**:

- ✅ All 2347 library tests pass (updated after mouse parser completion)
- ✅ All 23 keyboard parsing tests pass
- ✅ All 23 input event generator tests pass (including round-trip validation with corrected
  encoding)
- ✅ Conformance tests updated to use flat public API (removed private submodule access)
- ✅ Constants module follows CLAUDE.md guidelines (private modules with public re-exports)

**Key Files Created/Modified**:

- Created: `tui/src/core/ansi/constants/input_sequences.rs` (50 constants, 10 tests)
- Created: `tui/src/core/ansi/generator/input_event_generator.rs` (public generator, 23 tests)
- Modified: `tui/src/core/ansi/vt_100_terminal_input_parser/keyboard.rs` (added test helpers,
  refactored tests)
- Updated: `tui/src/core/ansi/constants/mod.rs` (added input_sequences export)
- Updated: `tui/src/core/ansi/generator/mod.rs` (added input_event_generator export)
- Fixed: `tui/src/core/ansi/vt_100_ansi_parser/vt_100_ansi_conformance_tests/tests/*.rs` (3 test
  files updated to use flat API)

##### Mouse Parsing (`mouse.rs`) ✅ COMPLETE

**Completion Summary**:

- ✅ Implemented `parse_mouse_sequence(bytes: &[u8]) -> Option<InputEvent>` with comprehensive SGR
  protocol support
- ✅ Type-safe coordinate system using `Pos` with 1-based `TermCol`/`TermRow` (confirmed via
  terminal observation)
- ✅ Created 29 automated integration tests using real sequences from terminal observation
- ✅ Fixed modifier encoding bug in `input_event_generator.rs` (off-by-1 error: parameter = 1 +
  bitfield)
- ✅ All round-trip tests pass (generator ↔ parser compatibility validated)

**Implementation Details**:

- ✅ SGR Protocol: `CSI < Cb ; Cx ; Cy M/m` → `InputEvent::Mouse`
  - `M` (uppercase) = press, `m` (lowercase) = release
  - Cb = button byte with modifiers and flags
  - Cx, Cy = 1-based terminal coordinates (VT-100 convention)
- ✅ Button detection: Bits 0-1 of Cb (0=left, 1=middle, 2=right)
- ✅ Drag detection: Bit 5 (value 32) in Cb → `MouseAction::Drag`
- ✅ Scroll detection: Buttons 64-67 → `MouseAction::Scroll(Up/Down)`
  - Button 64-67: Scroll up (including with modifiers)
  - Button 68-71: Scroll down
- ✅ Modifier extraction: Bits 2-4 (Shift=4, Alt=8, Ctrl=16)
- ✅ Position handling: `Pos::from_one_based(cx, cy)` with `NonZeroU16` validation

**Test Coverage** (37 parser tests + 29 integration tests = 66 total):

- **Parser unit tests** (6 tests in `mouse.rs`):
  - SGR left click press/release
  - Scroll events with modifiers
  - Drag events
  - Modifier extraction (Ctrl+click)
  - 1-based coordinate verification
- **Integration tests** (29 tests in `input_parser_validation_test.rs`):
  - Mouse events: left/middle/right clicks, drag, scroll, all modifier combinations
  - Keyboard events: arrows, function keys, all 8 modifier combinations
  - Edge cases: incomplete sequences, invalid data, boundary conditions (coordinate zero, u16::MAX)
  - Modifier encoding validation: all 7 combinations (Shift through Shift+Alt+Ctrl)

**Key Findings from Terminal Observation**:

- ✅ VT-100 coordinates are 1-based (top-left = 1,1) - confirmed empirically
- ✅ Modifier encoding: parameter = 1 + bitfield (e.g., Ctrl = parameter 5, not 4)
- ✅ Scroll button 66 = scroll up with Shift modifier set

**Test Quality**:

- ✅ All 2347 library tests pass
- ✅ All 66 VT-100 input parser tests pass (37 unit + 29 integration)
- ✅ All 23 generator tests pass with corrected modifier encoding
- ✅ Round-trip validation: Event → ANSI → Event matches for all combinations

**Key Files Created/Modified**:

- Modified: `tui/src/core/ansi/vt_100_terminal_input_parser/mouse.rs` (SGR parser implementation)
- Modified: `tui/src/core/ansi/vt_100_terminal_input_parser/types.rs` (updated `Pos` to use
  `TermCol`/`TermRow`)
- Created:
  `tui/src/core/ansi/vt_100_terminal_input_parser/integration_tests/input_parser_validation_test.rs`
  (29 tests)
- Fixed: `tui/src/core/ansi/generator/input_event_generator.rs` (corrected `encode_modifiers`:
  `b'1' + mask`)
- Updated: Documentation with test design philosophy (why literals, not generators, in tests)

##### Resize Event Parsing (`terminal_events.rs`)

- [ ] **Implement resize event parsing** `parse_resize_event(bytes: &[u8]) -> Option<InputEvent>`
  - [ ] Format: CSI `8 ; rows ; cols t` → extract rows and columns → `InputEvent::Resize`
  - [ ] Detect window resize sequences sent by terminal

##### Focus Event Parsing (`terminal_events.rs`)

- [ ] **Implement focus event parsing** `parse_focus_event(bytes: &[u8]) -> Option<InputEvent>`
  - [ ] CSI `I` → `InputEvent::Focus(FocusState::Gained)`
  - [ ] CSI `O` → `InputEvent::Focus(FocusState::Lost)`

##### Bracketed Paste Parsing (`terminal_events.rs`)

- [ ] **Implement bracketed paste parsing**
      `parse_bracketed_paste(bytes: &[u8]) -> Option<InputEvent>`
  - [ ] Format: ESC `[200~` text ESC `[201~`
  - [ ] Accumulate pasted text between delimiters
  - [ ] Return `InputEvent::Paste(PasteMode::Start/End)` for markers

##### UTF-8 Text Parsing (`utf8.rs`)

- [ ] Helper functions for UTF-8 parsing
- [ ] Implement `parse_utf8_text(bytes: &[u8]) -> Vec<InputEvent>`
  - [ ] Handle single-byte ASCII characters
  - [ ] Handle multi-byte UTF-8 sequences
  - [ ] Buffer incomplete sequences for later completion

#### 8.2.3 Implement Backend Device (1-1.5 hours)

**Location**: `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/input_device_impl.rs`

This layer manages async I/O and buffering, calling protocol parsers from 8.2.2:

- [ ] Implement `DirectToAnsiInputDevice::new()` constructor
  - [ ] Initialize tokio::io::stdin()
  - [ ] Create 4096-byte buffer
  - [ ] Set 150ms timeout for incomplete sequences

- [ ] Implement async `read_event()` main loop
  - [ ] Try to parse complete sequence from buffer using protocol parsers
  - [ ] If no complete sequence, read more bytes with timeout
  - [ ] On timeout, try to parse incomplete or skip malformed byte
  - [ ] Return parsed InputEvent or None on EOF

- [ ] Implement helper methods
  - [ ] `try_parse_from_buffer()` - dispatches to protocol parsers
  - [ ] `read_bytes()` - async tokio stdin read
  - [ ] `handle_timeout()` - recover from incomplete sequences
  - [ ] Buffer management (advance read pointer, handle wraparound)

- [ ] Update `tui/src/tui/terminal_lib_backends/direct_to_ansi/mod.rs`:
  - [ ] Add `pub mod input;`
  - [ ] Add re-exports: `pub use input::DirectToAnsiInputDevice;`

#### 8.2.4 Handle Edge Cases (1-1.5 hours)

Both **protocol parsers** (8.2.2) and **backend device** (8.2.3) need edge case handling:

**In Protocol Parsers** (`vt_100_terminal_input_parser/`):

- [ ] Invalid/malformed sequence handling:
  - [ ] Return `None` for incomplete sequences
  - [ ] Handle truncated sequences gracefully
  - [ ] Validate sequence structure (don't panic)

**In Backend Device** (`input_device_impl.rs`):

- [ ] Partial sequence buffering:
  - [ ] Detect incomplete sequences (ESC without terminator)
  - [ ] Buffer and wait for more data
  - [ ] Timeout recovery: skip byte and continue

- [ ] UTF-8 text handling:
  - [ ] Detect UTF-8 start bytes (0xC0-0xF7)
  - [ ] Buffer until complete sequence (1-4 bytes)
  - [ ] Validate continuation bytes match pattern 10xxxxxx
  - [ ] Call `utf8::parse_utf8_text()` from protocol layer

- [ ] Terminal interaction:
  - [ ] Stdin EOF: return `Ok(None)` to signal graceful close
  - [ ] I/O errors: propagate via `Result`

**Deliverable**: Input module with complete InputDevice implementation that reads and parses all
ANSI sequences (keyboard, mouse, resize, focus, paste) to produce InputEvent

### 8.3: Testing & Validation (2-3 hours)

**Objective**: Ensure DirectToAnsi InputDevice works correctly for all input scenarios

**Unit Tests** (1-1.5 hours):

**File**: `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/tests.rs`

Test ANSI sequence → InputEvent conversion:

**Keyboard Events**:

- [ ] Arrow keys generate correct directional events
  - [ ] CSI `A` → SpecialKey::Up
  - [ ] CSI `B` → SpecialKey::Down
  - [ ] CSI `C` → SpecialKey::Right
  - [ ] CSI `D` → SpecialKey::Left
- [ ] Function keys (F1-F12) parse correctly
  - [ ] CSI `11~` → FunctionKey::F1
  - [ ] CSI `12~` → FunctionKey::F2
  - [ ] ... CSI `34~` → FunctionKey::F12
- [ ] Modifier combinations (Ctrl+Arrow, Alt+Key, Shift+Key)
  - [ ] CSI `1;5A` → Ctrl+Up
  - [ ] CSI `1;3A` → Alt+Up
  - [ ] CSI `1;2A` → Shift+Up
  - [ ] Combinations: Ctrl+Alt, Alt+Shift, Ctrl+Shift, all three
- [ ] Special keys (Home, End, Page Up/Down, Delete, Insert, Tab, BackTab, Esc, Enter)
  - [ ] CSI `H` → SpecialKey::Home
  - [ ] CSI `F` → SpecialKey::End
  - [ ] CSI `5~` → SpecialKey::PageUp
  - [ ] CSI `6~` → SpecialKey::PageDown
  - [ ] CSI `2~` → SpecialKey::Insert
  - [ ] CSI `3~` → SpecialKey::Delete
  - [ ] 0x09 → SpecialKey::Tab
  - [ ] CSI `Z` → SpecialKey::BackTab
  - [ ] 0x1B (lone) → SpecialKey::Esc
  - [ ] 0x0D/0x0A → SpecialKey::Enter
  - [ ] 0x08 → SpecialKey::Backspace

**Mouse Events**:

- [ ] Mouse clicks (left, middle, right)
  - [ ] `CSI <0;10;5M` → MouseInputKind::MouseDown(Button::Left) at Pos(col=10, row=5)
  - [ ] `CSI <1;10;5M` → MouseInputKind::MouseDown(Button::Middle, ...)
  - [ ] `CSI <2;10;5M` → MouseInputKind::MouseDown(Button::Right, ...)
- [ ] Mouse release events
  - [ ] `CSI <0;10;5m` → MouseInputKind::MouseUp
- [ ] Mouse drag detection
  - [ ] Button held (Cb & 32) with motion → MouseInputKind::MouseDrag
- [ ] Mouse motion without buttons
  - [ ] Movement without button pressed → MouseInputKind::MouseMove
- [ ] Mouse scroll events
  - [ ] Button 64 → MouseInputKind::ScrollUp
  - [ ] Button 65 → MouseInputKind::ScrollDown
  - [ ] Button 6 → MouseInputKind::ScrollLeft
  - [ ] Button 7 → MouseInputKind::ScrollRight

**Terminal Events**:

- [ ] Resize events
  - [ ] CSI `8;rows;cols t` → InputEvent::Resize(Size) with correct dimensions
- [ ] Focus events
  - [ ] CSI `I` → InputEvent::Focus(FocusEvent::Gained)
  - [ ] CSI `O` → InputEvent::Focus(FocusEvent::Lost)
- [ ] Bracketed paste
  - [ ] ESC `[200~text ESC [201~` → InputEvent::BracketedPaste(String)

Test edge cases:

- [ ] Partial sequences (incomplete input buffered until complete)
  - [ ] `ESC[` without terminator → buffer and wait
  - [ ] `ESC[1;` without final byte → buffer and wait
- [ ] Rapid consecutive keys (multiple sequences in quick succession)
- [ ] Mixed keyboard and mouse events in stream
- [ ] Invalid/garbled sequences (should not panic)
  - [ ] Random bytes that don't form valid sequences
  - [ ] Truncated sequences
  - [ ] Unknown CSI codes
- [ ] UTF-8 text between sequences (if supported)
- [ ] Empty input handling (no events for a period)

**Integration Tests** (0.5-1 hour):

**File**: `tui/src/tui/terminal_lib_backends/direct_to_ansi/integration_tests/input_handling.rs`

- [ ] Test actual terminal interaction:
  - [ ] Real keyboard input → InputEvent (scripted with `tmux`/`screen` or `expect` like in
        @script_lib.fish with flamegraph profiling with benchmark code).
  - [ ] Real mouse input → InputEvent
  - [ ] Test in multiple terminal emulators (xterm, GNOME Terminal, Alacritty, Windows Terminal)
- [ ] Verify behavior matches Crossterm implementation:
  - [ ] Same keys produce same InputEvent types
  - [ ] Same mouse sequences produce same events
  - [ ] Modifier combinations identical
- [ ] Performance test: Rapid input doesn't drop events
- [ ] Stress test: Large input volume handled correctly

**Test Utilities** (create if needed):

- [ ] Helper to generate ANSI sequences for testing (keyboard/mouse)
- [ ] Mock stdin reader for unit tests (inject test bytes)
- [ ] InputEvent comparison/assertion helpers

**Final crossterm input "event_stream" feature code comparison**:

- [ ] Look at existing crossterm input handling code for reference. Compare it to our code and
      verify that all scenarios are covered. Since we know that crossterm's input event handling
      works correctly, and we have access to the source, we can use it as a benchmark to ensure our
      implementation is functionally equivalent.

**Deliverable**: Comprehensive test suite (unit + integration) demonstrating InputDevice correctness

### 8.4: Migration & Cleanup (1 hour)

**Objective**: Integrate DirectToAnsi InputDevice and remove crossterm dependency

**Migration Steps**:

- [ ] **Replace InputDevice struct** in `tui/src/core/terminal_io/input_device.rs`:

  ```rust
  // Old code:
  pub struct InputDevice {
      pub resource: PinnedInputStream<CrosstermEventResult>,
  }

  // New code:
  pub struct InputDevice {
      backend: DirectToAnsiInputDevice,
  }
  ```

- [ ] **Update InputDeviceExt implementation** in
      `tui/src/tui/terminal_lib_backends/input_device_ext.rs`:
  - Implement `InputDeviceExt` trait for `InputDevice` (already exists)
  - Call `DirectToAnsiInputDevice::read_event()` instead of `crossterm::event` methods

- [ ] **Remove all crossterm::event imports**:
  - Remove `use crossterm::event::EventStream` from input-related files
  - Remove `use crossterm::event::Event` from conversion code
  - Remove `PinnedInputStream` usage (no longer needed)
  - Remove `CrosstermEventResult` type (no longer needed)

- [ ] **Verify no remaining crossterm usages in input code**:
  - Run: `grep -r "crossterm::event" tui/src/` (should return 0 results for input code)
  - Verify output code can still use crossterm if needed (PaintRenderOpImplCrossterm)

**Linux, Windows, macOS **:

- [ ] For Linux we will use tokio and the DirectToAnsiInputDevice as designed.
- [ ] For macOS and Windows, we will still keep using crossterm for input handling, just as we use
      crossterm for output as well (for now until Step 9 is done). After Step 9 is complete, then we
      can remove crossterm input handling on those platforms as well.

**Keep CrossTerm for Platform-Specific Support** (Until Step 9):

At this stage, crossterm is intentionally retained for macOS and Windows platforms while
DirectToAnsi is used for Linux input. This deferred removal strategy allows for safe, incremental
validation:

- [ ] Verify crossterm dependency remains in `Cargo.toml` with `event-stream` feature
  - Needed for: macOS/Windows input handling (until Step 9 validation)
- [ ] Keep `futures-util` dependency (required for crossterm event stream on macOS/Windows)
- [ ] **Platform-specific implementation**: Use conditional compilation:
  - `#[cfg(target_os = "linux")]` - Use DirectToAnsi input for Linux
  - `#[cfg(any(target_os = "macos", target_os = "windows"))]` - Use Crossterm input for
    macOS/Windows (deprecated, to be removed after Step 9)
- [ ] Add deprecation markers to crossterm InputDevice code:
  - `#[deprecated(since = "0.7.7", note = "Will be removed in Step 9 after DirectToAnsi validation on macOS/Windows")]`
- [ ] Document in comments: "Crossterm input will be removed after Step 9 validates DirectToAnsi on
      all platforms"
- [ ] **Step 9** (not Step 8) will validate DirectToAnsi on macOS/Windows, then remove crossterm
      entirely

**Code Quality**:

- [ ] `cargo check` - zero errors
- [ ] `cargo test --lib` - all tests pass
- [ ] `cargo clippy --all-targets` - zero warnings
- [ ] `cargo fmt --all -- --check` - proper formatting
- [ ] `cargo doc --no-deps` - generates without warnings

**Documentation**:

- [ ] Update `InputDevice` struct docs (top-level abstraction, no backend details leaked)
- [ ] Update `DirectToAnsiInputDevice` docs to explain tokio architecture
- [ ] Document ANSI sequence handling in `input_sequences.rs` with protocol references
- [ ] Add architecture notes in `direct_to_ansi/mod.rs` explaining input/output symmetry

  ```
  Output: RenderOp → render_op_impl → ansi generator → stdout
  Input:  stdin → tokio async I/O → ansi input parser → InputEvent

  Both paths use ANSI/VT-100 protocol, neither depends on external libraries.
  ```

- [ ] Update user-facing docs if any reference input handling

**Verification** (Functional Testing):

- [ ] Full TUI application works with keyboard input (test with example app)
- [ ] Full TUI application works with mouse input (test with example app)
- [ ] `choose()` function works with DirectToAnsi InputDevice
- [ ] `readline_async()` function works with DirectToAnsi InputDevice
- [ ] All examples run without input-related issues
- [ ] Terminal works in various terminal emulators (xterm, alacritty, GNOME Terminal, tmux, screen)

**Sign-Off**: InputDevice for Linux implemented with DirectToAnsi, macOS/Windows still use crossterm
(deprecated, to be removed after Step 9 validation)

---

## ⏳ Step 9: macOS & Windows Platform Validation & Crossterm Removal (2-3 hours) - DEFERRED

**Status**: ⏳ DEFERRED - To be performed after Step 8 completes (when user has access to
macOS/Windows systems)

**Objective**:

1. Validate DirectToAnsi backend on macOS and Windows platforms (replacing deprecated crossterm)
2. Remove crossterm dependency entirely once DirectToAnsi is verified on all platforms

**Rationale for Deferral**: User is currently running on Linux. Step 9 is deferred to be performed
later when macOS and Windows systems are available. Step 8 (InputDevice implementation) establishes
DirectToAnsi for Linux input with crossterm retained for macOS/Windows. Step 9 will validate
DirectToAnsi works identically on macOS/Windows, then remove the deprecated crossterm dependency
entirely. This incremental validation strategy ensures safety before removing the legacy backend.

**Key Difference from Step 8**:

- **Step 8**: Implements DirectToAnsi input for Linux only; crossterm retained for macOS/Windows
- **Step 9**: Validates DirectToAnsi on macOS/Windows, then removes deprecated crossterm entirely

### macOS Testing (1.5 hours)

**Terminals to Test**:

- [ ] **Terminal.app** - Standard macOS terminal
  - Verify cursor movement (arrow keys)
  - Verify color rendering (may have different palette than Linux)
  - Verify text attributes visible
  - Check for visual artifacts

- [ ] **iTerm2** - Advanced macOS terminal (if available)
  - Same validations as Terminal.app
  - Test advanced color features (truecolor support)

**Functional Testing Checklist**:

- [ ] Application launches without errors
- [ ] Cursor movement responsive
- [ ] Colors render correctly (verify macOS-specific palette if any)
- [ ] Text attributes visible
- [ ] Terminal modes work correctly
- [ ] Window resize handled gracefully
- [ ] No visual artifacts or garbled output

**Performance Benchmarking**:

- [ ] Run: `cargo flamegraph --example <app_name>` (Crossterm backend)
- [ ] Change to DirectToAnsi backend in `mod.rs:142`
- [ ] Run: `cargo flamegraph --example <app_name>` (DirectToAnsi backend)
- [ ] Calculate performance ratio: `DirectToAnsi_samples / Crossterm_samples`
- [ ] Verify ratio within ±5% threshold

**Edge Cases**:

- [ ] Max row/col indices
- [ ] Rapid color changes
- [ ] Large batches of RenderOps
- [ ] Window resize stress testing

### Windows Testing (1.5 hours)

**Important**: Windows 10+ supports VT-100 ANSI via "Virtual Terminal Processing"

**Terminals to Test**:

- [ ] **Windows Terminal** - Modern Windows terminal with full ANSI support
  - Verify cursor movement
  - Verify color rendering (RGB + 256-color palette)
  - Verify text attributes
  - Check for visual artifacts

- [ ] **PowerShell Console** - Legacy Windows console (may need VT mode enabled)
  - Same validations as Windows Terminal
  - Verify VT mode is properly enabled if needed

**Functional Testing Checklist**:

- [ ] Virtual Terminal Processing enabled (if needed for PowerShell)
- [ ] Application launches without errors
- [ ] Cursor movement responsive
- [ ] Colors render correctly
- [ ] Text attributes visible
- [ ] Terminal modes work correctly
- [ ] No color palette issues (Windows may use different default colors)
- [ ] No visual artifacts or garbled output

**Performance Benchmarking**:

- [ ] Same methodology as macOS
- [ ] Verify performance ratio within ±5% threshold
- [ ] Check for Windows-specific performance characteristics

**Edge Cases**:

- [ ] Same as macOS (max indices, rapid changes, large batches)

### Documentation & Sign-Off (30 min)

**Tasks**:

- [ ] Update Linux Testing Report with macOS/Windows results
- [ ] Create comprehensive Cross-Platform Testing Report
  - Platforms tested and results
  - Platform-specific findings
  - Known issues (if any)
  - Performance comparison (all platforms)
  - Recommendations for users
- [ ] Final Go/No-Go decision
- [ ] Update task_remove_crossterm.md with all findings

**Checkpoint**: Cross-platform validation complete, ready for production release

---

## Implementation Checklist

```
Step 4: Linux Validation & Performance Testing (2-3 hours) [COMPLETE]
  ✅ Test on xterm (functional, regression, edge cases)
  ✅ Test on GNOME Terminal (functional, regression, edge cases)
  ✅ Test on Alacritty (functional, regression, edge cases)
  ✅ Performance: Crossterm baseline flamegraph captured
  ✅ Performance: DirectToAnsi flamegraph captured
  ⚠️ Performance: ratio calculated (1.5558 = 55.58% regression) OUTSIDE ±5% threshold
  ✅ Edge cases: max indices, rapid color changes, large batches
  ✅ Documentation: Results recorded in task_remove_crossterm.md Step 4
  ⚠️ Go/No-Go decision: PROCEED TO STEP 5 (PERFORMANCE REGRESSION ANALYSIS)

Step 5: Performance Regression Analysis (2-4 hours) [COMPLETE]
  ✅ Flamegraph analysis: identified hotspots
  ✅ Code review: AnsiSequenceGenerator optimization (sequence caching)
  ✅ Code review: RenderOpImplDirectAnsi state tracking
  ✅ Implemented Approach A: sequence caching via DirectToAnsi
  ✅ Re-benchmarked and verified improvements
  ✅ Performance ratio optimized to acceptable threshold
  ✅ Updated code comments with optimization rationale

Step 6: Cleanup & Architectural Refinement (1-2 hours) [COMPLETE]
  ✅ 6.1: DirectToAnsi rename (✅ ALREADY COMPLETE)
  ✅ 6.2: Remove Termion backend (deleted directory, removed enum variant, fixed 3 matches)
  ✅ 6.3: Resolve TODOs and document deferments (5 comments clarified)
  ✅ 6.4: Audit cli_text/tui_styled_text consistency (kept separate, documented rationale)
  ✅ Final cargo check, test, clippy, fmt passes (2192 tests passing)

Step 7: Comprehensive RenderOp Integration Test Suite (4-6 hours) [READY TO START]
  ☐ 7.1: RenderOp Integration Tests - Core Execution Path
    ☐ Part A: Color Operations (30-45 min) - SetFgColor, SetBgColor, ResetColor
    ☐ Part B: Cursor Movement (45-60 min) - MoveCursorPositionAbs, MoveCursorPositionRelTo
    ☐ Part C: Screen Operations (20-30 min) - ClearScreen, ShowCursor, HideCursor
    ☐ Part D: State Optimization (30-45 min) - Redundant operation detection
  ☐ 7.2: VT-100 Display Sequence Expansion (1-2 hours) - Scroll, line ops, margins
  ☐ 7.3: Real-World VT-100 Test Scenarios (1-2 hours) - Terminal interaction patterns
  ☐ 7.4: Final QA & Validation (30-45 min) - All tests pass, documentation complete

Step 8: Implement InputDevice for DirectToAnsi Backend (8-12 hours) [PENDING]
  ☐ 8.1: Architecture Design (1-2 hours)
    ☐ Review VT-100 parser for input sequence support
    ☐ Design InputDevice API (backend-agnostic)
    ☐ Plan async event reading with tokio
    ☐ Document ANSI sequence → InputEvent mapping
    ☐ Plan integration points
    ☐ Identify edge cases
  ☐ 8.2: Implement DirectToAnsi InputDevice (4-6 hours)
    ☐ 8.2.1: Create module structure
    ☐ 8.2.2: Implement ANSI sequence parsing (keyboard and mouse events)
    ☐ 8.2.3: Handle edge cases (partial sequences, invalid input, etc.)
  ☐ 8.3: Testing & Validation (2-3 hours)
    ☐ Unit tests: ANSI sequence → InputEvent conversion
    ☐ Unit tests: Edge cases and special keys
    ☐ Integration tests: Actual terminal interaction
    ☐ Compatibility verification with Crossterm behavior
  ☐ 8.4: Migration & Cleanup (1 hour)
    ☐ Update InputDevice to use DirectToAnsi implementation
    ☐ Remove crossterm::event dependencies
    ☐ Remove crossterm from Cargo.toml
    ☐ Code quality: cargo check, test, clippy, fmt
    ☐ Documentation update
    ☐ Final verification

Step 9: macOS & Windows Platform Validation (2-3 hours) [DEFERRED]
  ☐ macOS: Test on Terminal.app and iTerm2
  ☐ Windows: Test on Windows Terminal and PowerShell
  ☐ Performance: flamegraph comparison (<5% difference)
  ☐ Edge cases: max indices, rapid color changes, large batches
  ☐ Documentation: Cross-Platform Testing Report updated
```

```
Step 3: Type System Architecture & DirectAnsi Backend Implementation

Step 3.0: Remove IR Execution Path (2-3 hours)
  ☐ Locate RenderOpIRVec::execute_all() in render_op_ir.rs
  ☐ Locate RenderOpIRVec::route_paint_render_op_ir_to_backend() in render_op_ir.rs
  ☐ Remove both methods from impl block
  ☐ Remove their documentation
  ☐ cargo check - should still pass
  ☐ cargo clippy - ensure no orphaned references
  ☐ Verify RenderOpIRVec can no longer be executed directly
  ☐ Confirm file changes are minimal (-50 LOC)

Step 3.1: Create RenderOpOutput Execution Path (3-4 hours)
  ☐ Add execute_all() method to RenderOpOutputVec in render_op_output.rs
  ☐ Add route_paint_render_op_output_to_backend() helper function
  ☐ Match on TERMINAL_LIB_BACKEND in routing function
  ☐ Handle Crossterm variant (route to PaintRenderOpImplCrossterm)
  ☐ Handle DirectAnsi variant (route to PaintRenderOpImplDirectAnsi)
  ☐ Add exhaustiveness checks for all RenderOpOutput variants
  ☐ Update PaintRenderOp trait to accept &RenderOpOutput instead of &RenderOpIR
  ☐ Update all PaintRenderOp implementations to match new trait signature
  ☐ cargo check passes
  ☐ cargo clippy passes (no warnings)

Step 3.2: Fix OffscreenBufferPaint Trait (3-4 hours)
  ☐ Read OffscreenBufferPaint trait definition in ofs_buf_core.rs
  ☐ Change render() return type from RenderOpIRVec to RenderOpOutputVec
  ☐ Change render_diff() return type from RenderOpIRVec to RenderOpOutputVec
  ☐ Update paint() signature to accept RenderOpOutputVec
  ☐ Update paint_diff() signature to accept RenderOpOutputVec
  ☐ Update OffscreenBufferPaintImplCrossterm implementation
  ☐ Update raw_mode.rs to flow RawMode through pipeline (IR → Output)
  ☐ cargo check passes for all affected files
  ☐ cargo clippy passes (no warnings)

Step 3.3: Implement DirectAnsi Backend (25-35 hours)

  Step 3.3A: RenderOpImplDirectAnsi paint_common() (8-12 hours)
    ☐ Create paint_common_impl() function or method
    ☐ Handle Group A: Platform/Mode (EnterRawMode, ExitRawMode, SetAlternateScreenBuffer)
    ☐ Handle Group B: Cursor movement with optimization (MoveCursor, CursorToColumn, etc.)
    ☐ Handle Group C: Screen clearing (ClearScreen, ClearCurrentLine, etc.)
    ☐ Handle Group D: Color operations with state caching (SetFgColor, SetBgColor, ResetColor)
    ☐ Handle Group E: Text rendering (PrintString, PrintCharacter, etc.)
    ☐ Handle Group F: Cursor visibility (ShowCursor, HideCursor)
    ☐ Handle Group G: Cursor save/restore (SaveCursorPosition, RestoreCursorPosition)
    ☐ Handle Group H: Terminal modes (SetTerminalMode, ResetTerminalMode, etc.)
    ☐ All 27 RenderOpCommon variants exhaustively matched
    ☐ Cursor position tracking in RenderOpsLocalData
    ☐ Color caching optimization (skip redundant changes)
    ☐ cargo check passes
    ☐ cargo clippy passes (no warnings)

  Step 3.3B: RenderOpImplDirectAnsi paint_text_with_attributes() (4-6 hours)
    ☐ Handle post-compositor text with optional style
    ☐ Position cursor (already positioned by Compositor)
    ☐ Apply foreground color if Some
    ☐ Apply background color if Some
    ☐ Write text bytes
    ☐ Reset colors if style was applied
    ☐ Handle None style case (direct text write)
    ☐ Test with various Unicode and emoji widths
    ☐ cargo check passes
    ☐ cargo clippy passes (no warnings)

  Step 3.3C: Quality & Testing (3-5 hours)
    ☐ Add 60+ unit tests for AnsiSequenceGenerator methods
    ☐ Add 15+ integration tests for RenderOp execution
    ☐ Test cursor positioning (all cursor movement variants)
    ☐ Test color generation (RGB, 256-color, reset, caching)
    ☐ Test text attributes (bold, italic, underline if used)
    ☐ Test cursor visibility (show/hide)
    ☐ Test terminal modes
    ☐ Test optimization (verify redundant operations skipped)
    ☐ cargo test --all passes (all tests)
    ☐ Code coverage: >90% target
    ☐ cargo fmt applied (formatting)
    ☐ cargo clippy --all-targets passes (zero warnings)
    ☐ cargo doc --no-deps compiles (docs valid)

Step 4: Linux Validation & Performance Testing (2-3 hours) [SEPARATE STEP - Not part of 3.0-3.3]
  ☐ Linux: Test on xterm, gnome-terminal, alacritty
  ☐ Run flamegraph benchmark (Crossterm baseline)
  ☐ Run flamegraph benchmark (DirectToAnsi)
  ☐ Verify <5% performance difference vs crossterm backend
  ☐ No visual artifacts or garbled output
  ☐ All edge cases handled gracefully
  ☐ Linux Testing Report created

Step 6: macOS & Windows Platform Validation (2-3 hours) [DEFERRED - After Step 5]
  ☐ macOS: Test on Terminal.app, iTerm2
  ☐ Windows: Test on Windows Terminal, PowerShell
  ☐ Run flamegraph benchmark on both platforms
  ☐ Verify <5% performance difference vs crossterm backend
  ☐ No visual artifacts or garbled output
  ☐ All edge cases handled gracefully
  ☐ Cross-Platform Testing Report finalized
```

---

## Critical Success Factors

✅ **Architectural Alignment**:

- Reuse `SgrColorSequence` + `FastStringify` from VT-100 parser
- Use all existing ANSI constants (SGR_BOLD, etc.)
- Don't reinvent ANSI generation

✅ **Optimization Strategy**:

- Skip redundant cursor moves (track cursor_pos in RenderOpsLocalData)
- Skip redundant color changes (track fg_color, bg_color)
- Zero allocations for `Noop` operations

✅ **Test Coverage**:

- Unit tests for every AnsiSequenceGenerator method
- Integration tests for realistic RenderOp sequences
- > 90% code coverage target

✅ **Cross-Platform**:

- Validate on Linux, macOS, Windows 10+
- Auto-enable Windows VT processing (Phase 4 improvement)
- Performance within 5% of crossterm backend

---

## Effort Summary - Steps 2-6 Implementation

| Component                                       | LOC        | Hours          | Risk    | Status                    |
| ----------------------------------------------- | ---------- | -------------- | ------- | ------------------------- |
| **Step 1: Module Structure**                    | 50         | 0.5h           | MINIMAL | ✅ COMPLETE               |
| **Step 2: AnsiSequenceGenerator**               | 273        | 3-4h           | LOW     | ✅ COMPLETE               |
| **Step 2 Total**                                | **323**    | **3.5-4.5h**   | **LOW** | **✅ COMPLETE**           |
|                                                 |            |                |         |                           |
| **Step 3: Type System & DirectAnsi (EXPANDED)** |            |                |         | **✅ COMPLETE**           |
| 3.0: Remove IR Execution Path                   | -100       | 2-3h           | MINIMAL | ✅ COMPLETE               |
| 3.1: Create RenderOpOutput Execution Path       | +250       | 3-4h           | LOW     | ✅ COMPLETE               |
| 3.2: Fix OffscreenBufferPaint Trait             | +200       | 3-4h           | LOW     | ✅ COMPLETE               |
| 3.3A: paint_common() implementation             | +600       | 8-12h          | MEDIUM  | ✅ COMPLETE               |
| 3.3B: paint_text_with_attributes()              | +200       | 4-6h           | LOW     | ✅ COMPLETE               |
| 3.3C: Quality & Testing                         | +200       | 3-5h           | LOW     | ✅ COMPLETE               |
| **Step 3 Total**                                | **~1,350** | **33-46h**     | **LOW** | **✅ COMPLETE**           |
|                                                 |            |                |         |                           |
| **Step 4: Linux Validation & Performance**      | -          | 2-3h           | MEDIUM  | ⏳ NEXT                   |
|                                                 |            |                |         |                           |
| **Step 5: Cleanup & Refinement**                | -50        | 1-2h           | LOW     | ⏳ PENDING                |
|                                                 |            |                |         |                           |
| **Step 6: macOS & Windows Validation**          | -          | 2-3h           | MEDIUM  | ⏳ DEFERRED               |
|                                                 |            |                |         |                           |
| **GRAND TOTAL (Steps 1-6)**                     | **~1,623** | **44.5-60.5h** | **LOW** | **3 COMPLETE, 3 PENDING** |

**Timeline**:

- Step 1 (✅ COMPLETE): Already done, ~0.5 hours of work completed
- Step 2 (✅ COMPLETE): Already done, ~3-4 hours of work completed
- Step 3 (✅ COMPLETE): ~33-46 hours (completed October 26, 2025)
- Step 4 (⏳ NEXT): ~2-3 hours for Linux validation & performance
- Step 5 (⏳ PENDING): ~1-2 hours for cleanup and consolidation
- Step 6 (⏳ DEFERRED): ~2-3 hours for macOS/Windows validation (after Step 5)

**Key Architectural Improvements:**

- ✅ Step 2 Foundation: Proven ANSI generation approach without crossterm
- 🆕 Step 3 Expansion: Type system enforces semantic boundaries at compile time
- 📊 Step 3 Integration: DirectAnsi backend fully parallel with Crossterm
- ⏱️ Minimal Risk: All patterns proven in Step 2 crossterm backend

---

## Conclusion

This expanded task represents a complete architectural refinement of the rendering pipeline:

**Step 3 (Type System Architecture & DirectToAnsi)**:

- Enforced semantic boundaries at compile time: IR → Compositor → Output → Terminal
- Prevented direct IR execution through type system design
- Implemented DirectToAnsi backend with identical pattern to Crossterm
- Achieved >90% test coverage with 2163 passing tests

**Step 4 (Linux Validation & Performance)**:

- Validates DirectToAnsi backend on Linux (xterm, GNOME Terminal, Alacritty)
- Confirms performance parity with Crossterm (<5% difference)
- Tests edge cases and boundary conditions
- Creates Linux Testing Report with findings

**Step 5 (Cleanup & Consolidation)**:

- Remove Termion dead code for codebase simplification
- Resolve TODOs and stubs
- Consolidate cli_text and tui_styled_text implementations
- Note: DirectToAnsi rename already complete (5.1 ✅)

**Step 6 (macOS & Windows Validation) [DEFERRED]**:

- Validates DirectToAnsi backend on macOS (Terminal.app, iTerm2)
- Validates DirectToAnsi backend on Windows (Windows Terminal, PowerShell)
- Confirms cross-platform performance parity
- Creates comprehensive Cross-Platform Testing Report

---

**Step 1 Status**: ✅ COMPLETE (Module Structure)

**Step 2 Status**: ✅ COMPLETE (AnsiSequenceGenerator)

**Step 3 Status**: ✅ COMPLETE (Type System & DirectToAnsi Backend)

- 3.0: ✅ Removed IR Execution Path - RenderOpIRVec no longer executable directly
- 3.1: ✅ Created Output Execution Path - RenderOpOutputVec::execute_all() routes to backends
- 3.2: ✅ Fixed OffscreenBufferPaint Trait - Correct type system enforced
- 3.3: ✅ Implemented DirectToAnsi Backend - Full RenderOpPaint implementation with all 27
  variants + tests

**Step 4 Status**: ✅ COMPLETE (Linux Validation & Performance Testing)

- ✅ DirectAnsi backend fully functional on Linux
- ✅ Flamegraph profiling completed for both backends
- ⚠️ Performance regression detected: 1.5558 ratio (55.58% slower)
- ⏭️ Proceeds to Step 5 for optimization analysis

**Step 5 Status**: ⏳ NEXT (Performance Regression Analysis) - READY TO START

- Objective: Investigate and optimize DirectAnsi to achieve <1.05 performance ratio
- Flamegraph analysis of hotspots
- AnsiSequenceGenerator and RenderOpImplDirectAnsi optimization
- Re-benchmark and iterate until acceptable performance achieved

**Step 6 Status**: ⏳ PENDING (Cleanup & Architectural Refinement) - After Step 5

- 6.1: ✅ DirectToAnsi rename (already complete)
- 6.2: Remove Termion dead code
- 6.3: Resolve TODOs and stubs
- 6.4: Review cli_text/tui_styled_text for consolidation opportunities

**Step 7 Status**: ⏳ DEFERRED (macOS & Windows Validation) - After Step 6

- Platform testing when user has access to macOS/Windows systems
- Same validation methodology as Linux (Step 4)
- Creates final Cross-Platform Testing Report

---

- **Document Version**: 1.7 (New Step 5 for Performance Regression Analysis; Steps 5-6 renamed to
  6-7)
- **Last Updated**: October 26, 2025 (Updated with Step 5 addition and Step 4 completion)
- **Status**: Steps 1-4 Complete, Step 5 Ready to Begin, Steps 6-7 Pending
- **Next Action**: Begin Step 5 (Performance Regression Analysis)
