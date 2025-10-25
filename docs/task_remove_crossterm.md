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
  - [Step 1: Create DirectAnsi Module Structure (30 min) ✅ COMPLETE](#step-1-create-directansi-module-structure-30-min--complete)
  - [Step 2: Implement AnsiSequenceGenerator (3-4 hours) ✅ COMPLETE](#step-2-implement-ansisequencegenerator-3-4-hours--complete)
    - [Key Design Achievement: Semantic ANSI Generation with VT-100 Infrastructure](#key-design-achievement-semantic-ansi-generation-with-vt-100-infrastructure)
    - [Implementation: Leveraging Type-Safe Enums](#implementation-leveraging-type-safe-enums)
      - [Section A: Cursor Movement Operations (Using CsiSequence)](#section-a-cursor-movement-operations-using-csisequence)
      - [Section B: Screen Clearing Operations](#section-b-screen-clearing-operations)
      - [Section C: Color Operations (Using SgrColorSequence)](#section-c-color-operations-using-sgrcolorsequence)
      - [Section D: Cursor Visibility Operations](#section-d-cursor-visibility-operations)
      - [Section E: Cursor Save/Restore Operations](#section-e-cursor-saverestore-operations)
      - [Section F: Terminal Mode Operations](#section-f-terminal-mode-operations)
      - [Section G: Module Documentation](#section-g-module-documentation)
  - [Step 3: Complete Type System Architecture & DirectAnsi Backend (EXPANDED - ~40-50 hours)](#step-3-complete-type-system-architecture--directansi-backend-expanded)
    - [Architectural Overview: Enforcing Semantic Boundaries](#architectural-overview-enforcing-semantic-boundaries)
    - [Step 3.0: Remove IR Execution Path (2-3 hours)](#step-30-remove-ir-execution-path-2-3-hours)
    - [Step 3.1: Create RenderOpOutput Execution Path (3-4 hours)](#step-31-create-renderopoutput-execution-path-3-4-hours)
    - [Step 3.2: Fix OffscreenBufferPaint Trait (3-4 hours)](#step-32-fix-offscreenbufferpaint-trait-3-4-hours)
    - [Step 3.3: Implement DirectAnsi Backend (25-35 hours)](#step-33-implement-directansi-backend-25-35-hours)
      - [Step 3.3A: RenderOpImplDirectAnsi paint_common() (8-12 hours)](#step-33a-renderopimpldirectansi-paint_common-8-12-hours)
      - [Step 3.3B: RenderOpImplDirectAnsi paint_text_with_attributes() (4-6 hours)](#step-33b-renderopimpldirectansi-paint_text_with_attributes-4-6-hours)
      - [Step 3.3C: Quality & Testing (3-5 hours)](#step-33c-quality--testing-3-5-hours)
    - [Step 3 Summary](#step-3-summary)
    - [Critical Success Factors for Step 3](#critical-success-factors-for-step-3)
  - [Step 4: Cross-Platform Validation & Performance (2-3 hours)](#step-4-cross-platform-validation--performance-2-3-hours)
  - [Implementation Checklist](#implementation-checklist)
  - [Effort Summary (Updated with Step 4 Discovery)](#effort-summary-updated-with-step-4-discovery)
  - [File Structure](#file-structure)
    - [New Files to Create](#new-files-to-create)
    - [Files to Modify](#files-to-modify)
    - [Files to Remove](#files-to-remove)
  - [Code Size Estimates](#code-size-estimates)
  - [Migration Timeline](#migration-timeline)
  - [Platform Compatibility](#platform-compatibility)
    - [ANSI Support by Platform](#ansi-support-by-platform)
    - [Windows Virtual Terminal Processing](#windows-virtual-terminal-processing)
    - [Cross-Platform Testing](#cross-platform-testing)
  - [Risks and Mitigation](#risks-and-mitigation)
  - [Success Metrics](#success-metrics)
    - [Phase 1 Achievements (✅ COMPLETE)](#phase-1-achievements--complete)
    - [Phase 2-6 Metrics (⏳ PENDING)](#phase-2-6-metrics--pending)
    - [Performance](#performance)
    - [Correctness](#correctness)
    - [Compatibility](#compatibility)
    - [Code Quality](#code-quality)
    - [Migration Completeness](#migration-completeness)
  - [Conclusion](#conclusion)
    - [Phase 1 Complete: Foundation is Solid ✅](#phase-1-complete-foundation-is-solid-)
      - [Key Architectural Achievements](#key-architectural-achievements)
      - [Remaining Work: Phases 2-6 (~2-3 weeks)](#remaining-work-phases-2-6-2-3-weeks)
      - [Risk Assessment: Minimal ✅](#risk-assessment-minimal-)

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
     stdin → mio async read → VT-100 Parser → Events → InputDevice → Application
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

## Step 1: Create DirectAnsi Module Structure (30 min) ✅ COMPLETE

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

## Step 2: Implement AnsiSequenceGenerator (3-4 hours) ✅ COMPLETE

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

## Step 3: Complete Type System Architecture & DirectAnsi Backend (EXPANDED - ~40-50 hours)

**Status**: ⏳ PENDING - COMPREHENSIVE ARCHITECTURAL FIX + BACKEND IMPL

**Overview**: This mega-step comprises 4 coordinated sub-phases that:
1. Fix fundamental architectural issues with the rendering pipeline type system
2. Enforce proper semantic boundaries between IR and Output operations
3. Create the Output execution path (previously missing)
4. Implement the DirectAnsi backend

**Critical Insight**: The type system currently allows `RenderOpIRVec` to execute directly, which violates the architectural boundary. Operations should ONLY flow: `RenderOpIR → Compositor → RenderOpOutput → Terminal`. This step enforces that boundary at compile time.

---

### Step 3.0: Remove IR Execution Path & Enforce Semantic Boundary (2-3 hours)

**Status**: ⏳ PENDING

**Objective**: Delete the direct IR execution path (`RenderOpIRVec::execute_all()` and `route_paint_render_op_ir_to_backend()`), forcing all operations through the Compositor.

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

**Status**: ⏳ PENDING

**Objective**: Implement the missing `RenderOpOutputVec::execute_all()` method and routing infrastructure, creating the ONLY valid path for executing operations.

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

**Status**: ⏳ PENDING

**Objective**:
1. Fix `OffscreenBufferPaint::render()` to return `RenderOpOutputVec` (currently returns `RenderOpIRVec`)
2. Update `OffscreenBufferPaint::paint()` to accept `RenderOpOutputVec` and call correct `execute_all()`
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
- `tui/src/tui/terminal_lib_backends/crossterm_backend/offscreen_buffer_paint_impl.rs` (implementation)
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

### Step 3.3: Implement RenderOpImplDirectAnsi (DirectAnsi Backend) (25-35 hours)

**Status**: ⏳ PENDING

**Objective**: Implement the DirectAnsi backend to execute `RenderOpOutput` operations, handling both common operations and post-compositor text rendering.

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

| Group | Variants | Complexity | Optimization |
|-------|----------|------------|--------------|
| **A** | EnterRawMode, ExitRawMode, Noop | ⭐ | No-ops (return early) |
| **B** | MoveCursor* (5 variants) | ⭐⭐⭐ | Skip if pos unchanged |
| **C** | Clear* (4 variants) | ⭐ | Direct ANSI generation |
| **D** | SetColor* (4 variants) | ⭐⭐⭐⭐ | Skip if color unchanged |
| **E** | PrintStyledText (1 variant) | ⭐ | Pass-through |
| **F** | Show/HideCursor (2 variants) | ⭐ | Direct ANSI generation |
| **G** | Save/RestoreCursor (2 variants) | ⭐ | Direct ANSI generation |
| **H** | Terminal modes (6 variants) | ⭐⭐ | Direct ANSI generation |

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

| Sub-Phase | Hours | LOC | Status | Files |
|-----------|-------|-----|--------|-------|
| 3.0: Remove IR path | 2-3 | -100 | ⏳ | 2 |
| 3.1: Create Output path | 3-4 | +250 | ⏳ | 2 |
| 3.2: Fix OffscreenBufferPaint | 3-4 | +200 | ⏳ | 4 |
| 3.3: DirectAnsi implementation | 25-35 | +950 | ⏳ | 1 |
| **TOTAL STEP 3** | **33-46** | **~1,300** | **⏳ PENDING** | **9** |

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
- >90% test coverage

✅ **Consistency**:
- Both Crossterm and DirectAnsi use same routing mechanism
- Both use same state optimization strategy
- Both use same `PaintRenderOp` trait

---

---

## Step 4: Cross-Platform Validation & Performance (2-3 hours)

**Status**: ⏳ PENDING - After Step 3 completes

**Note**: Unit and integration testing are handled as part of **Step 3.3C** (Quality & Testing). This step focuses on end-to-end validation and performance analysis.

**Linux Testing**:

- [ ] Test on xterm
- [ ] Test on gnome-terminal
- [ ] Test on alacritty
- [ ] Verify cursor movement
- [ ] Verify color rendering
- [ ] Verify no garbled output

**macOS Testing**:

- [ ] Test on Terminal.app
- [ ] Test on iTerm2 (if available)
- [ ] Same validations as Linux

**Windows Testing**:

- [ ] Test on Windows Terminal
- [ ] Test on PowerShell console
- [ ] Verify Virtual Terminal Processing works
- [ ] Verify color output

**Performance**:

- [ ] Run flamegraph benchmark
- [ ] Compare vs crossterm backend
- [ ] Target: <5% difference in flamegraph samples

**Edge Cases**:

- [ ] Max row/col indices
- [ ] Rapid color changes
- [ ] Large batches of RenderOps
- [ ] Boundary value handling

**Checkpoint**: Runs on all platforms, no regressions, <5% performance difference

---

## Implementation Checklist

```
Phase 3: Type System Architecture & DirectAnsi Backend Implementation

STEP 3.0: Remove IR Execution Path (2-3 hours)
  ☐ Locate RenderOpIRVec::execute_all() in render_op_ir.rs
  ☐ Locate RenderOpIRVec::route_paint_render_op_ir_to_backend() in render_op_ir.rs
  ☐ Remove both methods from impl block
  ☐ Remove their documentation
  ☐ cargo check - should still pass
  ☐ cargo clippy - ensure no orphaned references
  ☐ Verify RenderOpIRVec can no longer be executed directly
  ☐ Confirm file changes are minimal (-50 LOC)

STEP 3.1: Create RenderOpOutput Execution Path (3-4 hours)
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

STEP 3.2: Fix OffscreenBufferPaint Trait (3-4 hours)
  ☐ Read OffscreenBufferPaint trait definition in ofs_buf_core.rs
  ☐ Change render() return type from RenderOpIRVec to RenderOpOutputVec
  ☐ Change render_diff() return type from RenderOpIRVec to RenderOpOutputVec
  ☐ Update paint() signature to accept RenderOpOutputVec
  ☐ Update paint_diff() signature to accept RenderOpOutputVec
  ☐ Update OffscreenBufferPaintImplCrossterm implementation
  ☐ Update raw_mode.rs to flow RawMode through pipeline (IR → Output)
  ☐ cargo check passes for all affected files
  ☐ cargo clippy passes (no warnings)

STEP 3.3: Implement DirectAnsi Backend (25-35 hours)

  STEP 3.3A: RenderOpImplDirectAnsi paint_common() (8-12 hours)
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

  STEP 3.3B: RenderOpImplDirectAnsi paint_text_with_attributes() (4-6 hours)
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

  STEP 3.3C: Quality & Testing (3-5 hours)
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

STEP 4: Cross-Platform Validation (2-3 hours) [SEPARATE STEP - Not part of 3.0-3.3]
  ☐ Linux: Test on xterm, gnome-terminal, alacritty
  ☐ macOS: Test on Terminal.app, iTerm2
  ☐ Windows: Test on Windows Terminal, PowerShell
  ☐ Run flamegraph benchmark
  ☐ Verify <5% performance difference vs crossterm backend
  ☐ No visual artifacts or garbled output
  ☐ All edge cases handled gracefully
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

## Effort Summary - Phase 3 Implementation

| Component                                      | LOC        | Hours      | Risk        | Status         |
| ---------------------------------------------- | ---------- | ---------- | ----------- | -------------- |
| **Phase 2: COMPLETE**                          |            |            |             |                |
| Step 1: Module Structure                       | 50         | 0.5h       | MINIMAL     | ✅ COMPLETE    |
| Step 2: AnsiSequenceGenerator                  | 273        | 3-4h       | LOW         | ✅ COMPLETE    |
| **Phase 2 Total**                              | **323**    | **3.5-4.5h** | **LOW**     | **✅ COMPLETE** |
|                                                |            |            |             |                |
| **Phase 3: Type System & DirectAnsi (EXPANDED)** |          |            |             |                |
| 3.0: Remove IR Execution Path                  | -100       | 2-3h       | MINIMAL     | ⏳ PENDING     |
| 3.1: Create RenderOpOutput Execution Path      | +250       | 3-4h       | LOW         | ⏳ PENDING     |
| 3.2: Fix OffscreenBufferPaint Trait            | +200       | 3-4h       | LOW         | ⏳ PENDING     |
| 3.3A: paint_common() implementation            | +600       | 8-12h      | MEDIUM      | ⏳ PENDING     |
| 3.3B: paint_text_with_attributes()             | +200       | 4-6h       | LOW         | ⏳ PENDING     |
| 3.3C: Quality & Testing                        | +200       | 3-5h       | LOW         | ⏳ PENDING     |
| **Phase 3 Total**                              | **~1,350** | **33-46h** | **LOW**     | **⏳ PENDING** |
|                                                |            |            |             |                |
| **Phase 4: Cross-Platform Validation**         | -          | 2-3h       | MEDIUM      | ⏳ PENDING     |
|                                                |            |            |             |                |
| **GRAND TOTAL (Phases 2-4)**                   | **~1,673** | **38.5-53.5h** | **LOW**     | **2 COMPLETE, 2 PENDING** |

**Timeline**:
- Phase 2 (✅ COMPLETE): Already done, ~4 hours of work completed
- Phase 3 (⏳ PENDING): 2-3 weeks (4-5 hours/day)
- Phase 4 (⏳ PENDING): 1 week for cross-platform validation

**Key Architectural Improvements:**

- ✅ Phase 2 Foundation: Proven ANSI generation approach without crossterm
- 🆕 Phase 3 Expansion: Type system enforces semantic boundaries at compile time
- 📊 Phase 3 Integration: DirectAnsi backend fully parallel with Crossterm
- ⏱️ Minimal Risk: All patterns proven in Phase 2 crossterm backend

---

## Conclusion

This expanded Step 3 represents a major architectural refinement that makes the type system enforce semantic boundaries, preventing compile-time errors that could occur at runtime. By implementing the four sub-phases (3.0-3.3) in sequence, we ensure:

1. **Type Safety**: The compiler prevents IR from being executed directly
2. **Semantic Correctness**: All operations flow through the proper pipeline (IR → Compositor → Output → Terminal)
3. **Backend Symmetry**: Both Crossterm and DirectAnsi backends use identical routing and optimization patterns
4. **Production Quality**: >90% test coverage with comprehensive unit and integration tests

**Phase 2 Status**: ✅ COMPLETE (Steps 1-2, Module Structure + AnsiSequenceGenerator)

**Phase 3 Status**: ⏳ PENDING (Steps 3.0-3.3, Type System Architecture & DirectAnsi Backend Implementation)

**Phase 4 Status**: ⏳ PENDING (Cross-Platform Validation & Performance)

---

- **Document Version**: 1.2 (Architectural Refactoring Edition)
- **Last Updated**: October 25, 2025
- **Status**: Expanded Step 3 Ready for Implementation
- **Next Action**: Begin Step 3.0 (Remove IR Execution Path)
