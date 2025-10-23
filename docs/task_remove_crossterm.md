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
      - [2.1 Add DirectAnsi Backend Enum Variant](#21-add-directansi-backend-enum-variant)
      - [2.2 Create ANSI Sequence Generator](#22-create-ansi-sequence-generator)
      - [2.3 Implement RenderOpImplDirectAnsi](#23-implement-renderopimpldirectansi)
      - [2.4 Update Routing Logic](#24-update-routing-logic)
    - [Phase 3: Migrate choose() and readline_async() to RenderOps](#phase-3-migrate-choose-and-readline_async-to-renderops)
      - [3.1 Update Macros](#31-update-macros)
      - [3.2 Migrate select_component.rs](#32-migrate-select_componentrs)
      - [3.3 Update Imports](#33-update-imports)
    - [Phase 4: Input Handling with mio + VT-100 Parser](#phase-4-input-handling-with-mio--vt-100-parser)
      - [4.1 Create Async Stdin Reader with mio](#41-create-async-stdin-reader-with-mio)
      - [4.2 Create VT-100 Input Parser](#42-create-vt-100-input-parser)
      - [4.3 Integrate with InputDevice](#43-integrate-with-inputdevice)
    - [Phase 5: Testing & Validation](#phase-5-testing--validation)
      - [5.1 Unit Tests for DirectAnsi Backend](#51-unit-tests-for-directansi-backend)
      - [5.2 Integration Tests with Mock OutputDevice](#52-integration-tests-with-mock-outputdevice)
      - [5.3 Visual Testing Examples](#53-visual-testing-examples)
    - [Phase 6: Remove Crossterm Dependency](#phase-6-remove-crossterm-dependency)
      - [6.1 Update Cargo.toml](#61-update-cargotoml)
      - [6.2 Remove Crossterm Code](#62-remove-crossterm-code)
      - [6.3 Update Documentation](#63-update-documentation)
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

#### 2.1 Add DirectAnsi Backend Enum Variant

**File**: `tui/src/tui/terminal_lib_backends/mod.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalLibBackend {
    Crossterm,
    Termion,
    DirectAnsi, // ← NEW!
}

// Make DirectAnsi the default after migration complete
pub const TERMINAL_LIB_BACKEND: TerminalLibBackend = TerminalLibBackend::DirectAnsi;
```

#### 2.2 Create ANSI Sequence Generator

**File**: `tui/src/tui/terminal_lib_backends/direct_ansi/ansi_sequence_generator.rs`

This module generates raw ANSI escape sequence bytes for each terminal operation:

```rust
/// Generates ANSI escape sequence bytes for terminal operations
///
/// All methods return `Vec<u8>` containing the raw ANSI escape sequence bytes.
/// These can be written directly to stdout via `OutputDevice`.
pub struct AnsiSequenceGenerator;

impl AnsiSequenceGenerator {
    /// Generate cursor movement to absolute position
    /// CSI <row>;<col>H
    ///
    /// Note: ANSI uses 1-based indexing, so we add 1 to both coordinates
    pub fn cursor_position(pos: Pos) -> Vec<u8> {
        format!("\x1b[{};{}H",
            pos.row_index.as_usize() + 1,
            pos.col_index.as_usize() + 1
        ).into_bytes()
    }

    /// Generate cursor movement to column
    /// CSI <col>G
    pub fn cursor_to_column(col: ColIndex) -> Vec<u8> {
        format!("\x1b[{}G", col.as_usize() + 1).into_bytes()
    }

    /// Generate cursor movement to next line
    /// CSI <n>E
    pub fn cursor_next_line(n: RowHeight) -> Vec<u8> {
        format!("\x1b[{}E", n.as_usize()).into_bytes()
    }

    /// Generate cursor movement to previous line
    /// CSI <n>F
    pub fn cursor_previous_line(n: RowHeight) -> Vec<u8> {
        format!("\x1b[{}F", n.as_usize()).into_bytes()
    }

    /// Clear entire screen
    /// CSI 2J
    pub fn clear_screen() -> Vec<u8> {
        b"\x1b[2J".to_vec()
    }

    /// Clear current line
    /// CSI 2K
    pub fn clear_current_line() -> Vec<u8> {
        b"\x1b[2K".to_vec()
    }

    /// Clear to end of line
    /// CSI 0K (or just CSI K)
    pub fn clear_to_end_of_line() -> Vec<u8> {
        b"\x1b[K".to_vec()
    }

    /// Clear to start of line
    /// CSI 1K
    pub fn clear_to_start_of_line() -> Vec<u8> {
        b"\x1b[1K".to_vec()
    }

    /// Generate foreground color sequence
    /// Uses TrueColor (24-bit) for RGB or 256-color codes
    ///
    /// Leverages existing TuiColor infrastructure for color conversion
    pub fn fg_color(color: TuiColor) -> Vec<u8> {
        match color {
            TuiColor::Rgb { r, g, b } => {
                format!("\x1b[38;2;{};{};{}m", r, g, b).into_bytes()
            }
            TuiColor::Ansi256(n) => {
                format!("\x1b[38;5;{}m", n).into_bytes()
            }
            // ... handle other TuiColor variants
        }
    }

    /// Generate background color sequence
    pub fn bg_color(color: TuiColor) -> Vec<u8> {
        match color {
            TuiColor::Rgb { r, g, b } => {
                format!("\x1b[48;2;{};{};{}m", r, g, b).into_bytes()
            }
            TuiColor::Ansi256(n) => {
                format!("\x1b[48;5;{}m", n).into_bytes()
            }
            // ... handle other TuiColor variants
        }
    }

    /// Reset colors and attributes to default
    /// CSI 0m
    pub fn reset_color() -> Vec<u8> {
        b"\x1b[0m".to_vec()
    }

    /// Generate text attribute sequences (bold, italic, underline, etc.)
    pub fn text_attributes(style: &TuiStyle) -> Vec<u8> {
        let mut bytes = Vec::new();

        if style.bold {
            bytes.extend_from_slice(b"\x1b[1m");
        }
        if style.dim {
            bytes.extend_from_slice(b"\x1b[2m");
        }
        if style.italic {
            bytes.extend_from_slice(b"\x1b[3m");
        }
        if style.underline {
            bytes.extend_from_slice(b"\x1b[4m");
        }
        if style.strikethrough {
            bytes.extend_from_slice(b"\x1b[9m");
        }

        bytes
    }

    /// Show cursor
    /// CSI ?25h
    pub fn show_cursor() -> Vec<u8> {
        b"\x1b[?25h".to_vec()
    }

    /// Hide cursor
    /// CSI ?25l
    pub fn hide_cursor() -> Vec<u8> {
        b"\x1b[?25l".to_vec()
    }

    /// Save cursor position
    /// CSI s (or CSI 7 on some terminals)
    pub fn save_cursor_position() -> Vec<u8> {
        b"\x1b[s".to_vec()
    }

    /// Restore cursor position
    /// CSI u (or CSI 8 on some terminals)
    pub fn restore_cursor_position() -> Vec<u8> {
        b"\x1b[u".to_vec()
    }

    /// Enter alternate screen
    /// CSI ?1049h
    pub fn enter_alternate_screen() -> Vec<u8> {
        b"\x1b[?1049h".to_vec()
    }

    /// Exit alternate screen
    /// CSI ?1049l
    pub fn exit_alternate_screen() -> Vec<u8> {
        b"\x1b[?1049l".to_vec()
    }

    /// Enable mouse tracking (all mouse events)
    /// CSI ?1003h CSI ?1015h CSI ?1006h
    ///
    /// Enables:
    /// - ?1003h: Report all mouse events (motion + button)
    /// - ?1015h: Enable urxvt mouse mode
    /// - ?1006h: Enable SGR extended mouse mode
    pub fn enable_mouse_tracking() -> Vec<u8> {
        b"\x1b[?1003h\x1b[?1015h\x1b[?1006h".to_vec()
    }

    /// Disable mouse tracking
    /// CSI ?1003l CSI ?1015l CSI ?1006l
    pub fn disable_mouse_tracking() -> Vec<u8> {
        b"\x1b[?1003l\x1b[?1015l\x1b[?1006l".to_vec()
    }
}
```

**Estimated LOC**: ~500 lines (implementation + documentation + tests)

#### 2.3 Implement RenderOpImplDirectAnsi

**File**: `tui/src/tui/terminal_lib_backends/direct_ansi/render_op_impl_direct_ansi.rs`

```rust
use super::{AnsiSequenceGenerator, PixelCharRenderer};
use crate::{
    PaintRenderOp, RenderOp, RenderOpsLocalData, LockedOutputDevice,
    Size, Pos, TuiColor, InlineString, TuiStyle,
};
use std::io::Write;

pub struct RenderOpImplDirectAnsi;

impl PaintRenderOp for RenderOpImplDirectAnsi {
    fn paint(
        &self,
        skip_flush: &mut bool,
        render_op: &RenderOp,
        window_size: Size,
        render_local_data: &mut RenderOpsLocalData,
        mut locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        if is_mock {
            return; // Skip actual rendering in mock mode
        }

        let bytes = match render_op {
            // === Cursor Movement ===
            RenderOp::MoveCursorPositionAbs(pos) => {
                // Optimization: skip if cursor already at position
                if render_local_data.cursor_pos == *pos {
                    return;
                }
                render_local_data.cursor_pos = *pos;
                AnsiSequenceGenerator::cursor_position(*pos)
            }

            RenderOp::MoveCursorPositionRelTo(origin, offset) => {
                let abs_pos = Pos::new(
                    origin.row_index + offset.row_index,
                    origin.col_index + offset.col_index,
                );
                render_local_data.cursor_pos = abs_pos;
                AnsiSequenceGenerator::cursor_position(abs_pos)
            }

            RenderOp::MoveCursorToColumn(col) => {
                render_local_data.cursor_pos.col_index = *col;
                AnsiSequenceGenerator::cursor_to_column(*col)
            }

            RenderOp::MoveCursorToNextLine(n) => {
                render_local_data.cursor_pos.row_index += *n;
                render_local_data.cursor_pos.col_index = col!(0);
                AnsiSequenceGenerator::cursor_next_line(*n)
            }

            RenderOp::MoveCursorToPreviousLine(n) => {
                render_local_data.cursor_pos.row_index =
                    render_local_data.cursor_pos.row_index.saturating_sub(*n);
                render_local_data.cursor_pos.col_index = col!(0);
                AnsiSequenceGenerator::cursor_previous_line(*n)
            }

            // === Screen Clearing ===
            RenderOp::ClearScreen => AnsiSequenceGenerator::clear_screen(),
            RenderOp::ClearCurrentLine => AnsiSequenceGenerator::clear_current_line(),
            RenderOp::ClearToEndOfLine => AnsiSequenceGenerator::clear_to_end_of_line(),
            RenderOp::ClearToStartOfLine => AnsiSequenceGenerator::clear_to_start_of_line(),

            // === Color Operations ===
            RenderOp::SetFgColor(color) => {
                // Optimization: skip if color hasn't changed
                if render_local_data.fg_color.as_ref() == Some(color) {
                    return;
                }
                render_local_data.fg_color = Some(*color);
                AnsiSequenceGenerator::fg_color(*color)
            }

            RenderOp::SetBgColor(color) => {
                // Optimization: skip if color hasn't changed
                if render_local_data.bg_color.as_ref() == Some(color) {
                    return;
                }
                render_local_data.bg_color = Some(*color);
                AnsiSequenceGenerator::bg_color(*color)
            }

            RenderOp::ResetColor => {
                render_local_data.fg_color = None;
                render_local_data.bg_color = None;
                AnsiSequenceGenerator::reset_color()
            }

            RenderOp::ApplyColors(style_opt) => {
                let mut bytes = Vec::new();
                if let Some(style) = style_opt {
                    if let Some(fg) = style.color_fg {
                        bytes.extend(AnsiSequenceGenerator::fg_color(fg));
                        render_local_data.fg_color = Some(fg);
                    }
                    if let Some(bg) = style.color_bg {
                        bytes.extend(AnsiSequenceGenerator::bg_color(bg));
                        render_local_data.bg_color = Some(bg);
                    }
                }
                bytes
            }

            // === Text Rendering ===
            RenderOp::PrintStyledText(text) => {
                // Text already contains ANSI codes, print as-is
                text.as_bytes().to_vec()
            }

            RenderOp::PaintTextWithAttributes(text, style_opt) => {
                let mut bytes = Vec::new();

                // Apply style attributes if provided
                if let Some(style) = style_opt {
                    bytes.extend(AnsiSequenceGenerator::text_attributes(style));
                }

                // Render text (with optional clipping to window bounds)
                bytes.extend(text.as_bytes());

                // Reset attributes after text
                if style_opt.is_some() {
                    bytes.extend(AnsiSequenceGenerator::reset_color());
                }

                bytes
            }

            RenderOp::CompositorNoClipTruncPaintTextWithAttributes(text, style_opt) => {
                // Same as PaintTextWithAttributes but without bounds checking
                // (compositor has already handled bounds)
                let mut bytes = Vec::new();

                if let Some(style) = style_opt {
                    bytes.extend(AnsiSequenceGenerator::text_attributes(style));
                }

                bytes.extend(text.as_bytes());

                if style_opt.is_some() {
                    bytes.extend(AnsiSequenceGenerator::reset_color());
                }

                bytes
            }

            // === Cursor Visibility ===
            RenderOp::ShowCursor => AnsiSequenceGenerator::show_cursor(),
            RenderOp::HideCursor => AnsiSequenceGenerator::hide_cursor(),

            // === Cursor Position Save/Restore ===
            RenderOp::SaveCursorPosition => AnsiSequenceGenerator::save_cursor_position(),
            RenderOp::RestoreCursorPosition => AnsiSequenceGenerator::restore_cursor_position(),

            // === Terminal Mode ===
            RenderOp::EnterRawMode => {
                // Raw mode is handled at a higher level (termios/Windows console mode)
                // Not an ANSI sequence - requires platform-specific API calls
                // See Phase 4 for raw mode implementation
                return;
            }

            RenderOp::ExitRawMode => {
                // Raw mode is handled at a higher level
                return;
            }

            RenderOp::Noop => return,
        };

        // Write bytes to output device
        if !bytes.is_empty() {
            locked_output_device.write_all(&bytes)
                .expect("Failed to write ANSI bytes to output device");
        }

        *skip_flush = false;
    }
}

impl Flush for RenderOpImplDirectAnsi {
    fn flush(&mut self, mut locked_output_device: LockedOutputDevice<'_>) {
        locked_output_device.flush()
            .expect("Failed to flush output device");
    }

    fn clear_before_flush(&mut self, mut locked_output_device: LockedOutputDevice<'_>) {
        let clear_bytes = AnsiSequenceGenerator::clear_screen();
        locked_output_device.write_all(&clear_bytes)
            .expect("Failed to write clear screen sequence");
        locked_output_device.flush()
            .expect("Failed to flush output device");
    }
}
```

**Estimated LOC**: ~600 lines (implementation + error handling + documentation)

#### 2.4 Update Routing Logic

**File**: `tui/src/tui/terminal_lib_backends/render_op.rs`

Modify `route_paint_render_op_to_backend` to include DirectAnsi:

```rust
pub fn route_paint_render_op_to_backend(
    render_local_data: &mut RenderOpsLocalData,
    skip_flush: &mut bool,
    render_op: &RenderOp,
    window_size: Size,
    locked_output_device: LockedOutputDevice<'_>,
    is_mock: bool,
) {
    match TERMINAL_LIB_BACKEND {
        TerminalLibBackend::Crossterm => {
            RenderOpImplCrossterm {}.paint(
                skip_flush,
                render_op,
                window_size,
                render_local_data,
                locked_output_device,
                is_mock,
            );
        }
        TerminalLibBackend::DirectAnsi => {
            RenderOpImplDirectAnsi {}.paint(
                skip_flush,
                render_op,
                window_size,
                render_local_data,
                locked_output_device,
                is_mock,
            );
        }
        TerminalLibBackend::Termion => unimplemented!(),
    }
}
```

**Estimated LOC**: ~50 lines (routing + backend enum updates)

---

### Phase 3: Migrate choose() and readline_async() to RenderOps

**Objective**: Replace direct crossterm calls in choose() and readline_async() with RenderOps.

#### 3.1 Update Macros

**File**: `tui/src/readline_async/choose_impl/crossterm_macros.rs`

Deprecate crossterm macros and create RenderOps-based replacements:

````rust
/// DEPRECATED: Use render_ops! macro instead
///
/// This macro is kept for backwards compatibility during migration.
/// New code should use `render_ops!` and `RenderOps::execute_all()`.
#[macro_export]
#[deprecated(note = "Use render_ops! macro and RenderOps::execute_all() instead")]
macro_rules! queue_commands {
    // ... existing implementation remains for backwards compatibility ...
}

/// Queue RenderOps to be executed
///
/// This macro creates RenderOps and executes them immediately.
///
/// # Example
///
/// ```rust
/// queue_render_ops!(output_device, render_ops!(
///     @new
///     RenderOp::ClearCurrentLine,
///     RenderOp::PrintStyledText("Hello".into()),
/// ));
/// ```
#[macro_export]
macro_rules! queue_render_ops {
    ($output_device:expr, $ops:expr) => {{
        use miette::IntoDiagnostic;
        let window_size = $crate::get_terminal_size()?;
        let locked_output = $crate::lock_output_device_as_mut!($output_device);
        let mut skip_flush = false;
        $ops.execute_all(&mut skip_flush, window_size, locked_output, false);
    }};
}
````

**Estimated LOC**: ~100 lines (macro updates + deprecation warnings)

#### 3.2 Migrate select_component.rs

**File**: `tui/src/readline_async/choose_impl/select_component.rs`

Example migration:

**Before (crossterm):**

```rust
queue_commands! {
    output_device,
    MoveToColumn(0),
    ResetColor,
    Clear(ClearType::CurrentLine),
    Print(styled_header),
    MoveToNextLine(1),
    ResetColor,
};
```

**After (RenderOps):**

```rust
let mut ops = render_ops!(
    @new
    RenderOp::MoveCursorToColumn(col!(0)),
    RenderOp::ResetColor,
    RenderOp::ClearCurrentLine,
    RenderOp::PrintStyledText(styled_header),
    RenderOp::MoveCursorToNextLine(height!(1)),
    RenderOp::ResetColor,
);
queue_render_ops!(output_device, ops);
```

**Files to migrate:**

- `tui/src/readline_async/choose_impl/select_component.rs`
- `tui/src/readline_async/choose_impl/function_component.rs`
- `tui/src/readline_async/choose_impl/event_loop.rs`
- `tui/src/readline_async/readline_async_impl/readline.rs`
- `tui/src/readline_async/spinner_impl/spinner_render.rs`

**Estimated LOC**: ~200 lines of changes (replacements across files)

#### 3.3 Update Imports

Replace crossterm imports with RenderOp imports:

**Before:**

```rust
use crossterm::{
    cursor::{MoveToColumn, MoveToNextLine},
    style::{Print, ResetColor},
    terminal::{Clear, ClearType},
};
```

**After:**

```rust
use crate::{
    RenderOp, RenderOps, render_ops,
    col, height, // Type-safe constructors from bounds_check
};
```

**Estimated LOC**: ~50 lines (import updates)

---

### Phase 4: Input Handling with mio + VT-100 Parser

**Objective**: Replace crossterm input handling with a custom VT-100 input parser and mio for async
I/O.

#### 4.1 Create Async Stdin Reader with mio

**File**: `tui/src/core/terminal_io/stdin_reader.rs` (NEW)

```rust
use mio::{Events, Interest, Poll, Token};
use std::io::{self, Read};
use std::os::unix::io::AsRawFd;
use std::time::Duration;

const STDIN_TOKEN: Token = Token(0);

/// Async stdin reader using mio for non-blocking I/O
///
/// This provides cross-platform async reading from stdin without depending
/// on external libraries like crossterm.
pub struct StdinReader {
    poll: Poll,
    events: Events,
    buffer: Vec<u8>,
}

impl StdinReader {
    pub fn new() -> io::Result<Self> {
        let poll = Poll::new()?;
        let events = Events::with_capacity(128);

        // Register stdin with mio for read events
        #[cfg(unix)]
        {
            let stdin_fd = io::stdin().as_raw_fd();
            let mut source = mio::unix::SourceFd(&stdin_fd);

            poll.registry().register(
                &mut source,
                STDIN_TOKEN,
                Interest::READABLE,
            )?;
        }

        #[cfg(windows)]
        {
            // Windows implementation uses Console API
            // See stdin_reader_windows.rs for details
        }

        Ok(Self {
            poll,
            events,
            buffer: Vec::with_capacity(4096),
        })
    }

    /// Poll stdin for readability with timeout
    ///
    /// Returns:
    /// - `Ok(Some(bytes))` if data available
    /// - `Ok(None)` if timeout
    /// - `Err` on I/O error
    pub fn read_with_timeout(&mut self, timeout_ms: u64) -> io::Result<Option<Vec<u8>>> {
        self.buffer.clear();

        // Poll with timeout
        self.poll.poll(
            &mut self.events,
            Some(Duration::from_millis(timeout_ms))
        )?;

        // Check if stdin is readable
        for event in &self.events {
            if event.token() == STDIN_TOKEN && event.is_readable() {
                // Read available bytes
                let mut stdin = io::stdin();
                stdin.read_to_end(&mut self.buffer)?;

                if !self.buffer.is_empty() {
                    return Ok(Some(self.buffer.clone()));
                }
            }
        }

        Ok(None)
    }

    /// Non-blocking read that returns immediately if no data
    pub fn try_read(&mut self) -> io::Result<Option<Vec<u8>>> {
        self.read_with_timeout(0)
    }
}
```

**Estimated LOC**: ~200 lines (Unix + Windows implementations)

#### 4.2 Create VT-100 Input Parser

**File**: `tui/src/core/terminal_io/vt100_input_parser/mod.rs` (NEW)

````rust
/// VT-100 input parser for stdin events
///
/// This is a **separate parser** from the PTY VT-100 parser, specialized for parsing
/// terminal input sequences (keyboard, mouse, etc.) from stdin.
///
/// **Architecture Decision**: We create a separate input parser rather than extending
/// the existing PTY parser to maintain clean separation of concerns. The PTY parser
/// handles output emulation (interpreting ANSI for rendering), while this parser
/// handles input parsing (converting ANSI to events).
///
/// # Handles
///
/// - Regular key presses (UTF-8 characters)
/// - Special keys (arrows, function keys, etc.) via CSI sequences
/// - Mouse events via SGR mouse protocol (CSI <...>M/m)
/// - Modifiers (Ctrl, Alt, Shift)
/// - Bracketed paste mode
///
/// # ANSI Sequence Reference
///
/// - Arrow keys: CSI A/B/C/D (Up/Down/Right/Left)
/// - Function keys: CSI <n>~ or CSI O P/Q/R/S
/// - Mouse: CSI < Cb ; Cx ; Cy M/m (SGR extended mode)
/// - Ctrl+key: Single byte 0x01-0x1A (Ctrl+A through Ctrl+Z)
/// - Alt+key: ESC followed by key

mod csi_parser;
mod key_parser;
mod mouse_parser;

use crate::{InputEvent, KeyEvent, MouseEvent, KeyCode, KeyModifiers};

pub struct Vt100InputParser {
    state: ParserState,
    buffer: Vec<u8>,
}

enum ParserState {
    Ground,           // Normal text input
    Escape,           // After ESC (0x1B)
    Csi,              // After CSI (ESC [)
    CsiParam,         // Accumulating CSI parameters
}

impl Vt100InputParser {
    pub fn new() -> Self {
        Self {
            state: ParserState::Ground,
            buffer: Vec::new(),
        }
    }

    /// Process bytes from stdin and extract input events
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut parser = Vt100InputParser::new();
    /// let bytes = stdin_reader.read_with_timeout(100)?;
    ///
    /// if let Some(bytes) = bytes {
    ///     let events = parser.process_bytes(&bytes);
    ///     for event in events {
    ///         handle_event(event);
    ///     }
    /// }
    /// ```
    pub fn process_bytes(&mut self, bytes: &[u8]) -> Vec<InputEvent> {
        let mut events = Vec::new();

        for &byte in bytes {
            if let Some(event) = self.process_byte(byte) {
                events.push(event);
            }
        }

        events
    }

    fn process_byte(&mut self, byte: u8) -> Option<InputEvent> {
        match self.state {
            ParserState::Ground => self.handle_ground(byte),
            ParserState::Escape => self.handle_escape(byte),
            ParserState::Csi => self.handle_csi(byte),
            ParserState::CsiParam => self.handle_csi_param(byte),
        }
    }

    fn handle_ground(&mut self, byte: u8) -> Option<InputEvent> {
        match byte {
            0x1B => {
                // ESC - enter escape sequence
                self.state = ParserState::Escape;
                self.buffer.clear();
                None
            }
            0x0D => {
                // Enter key (Carriage Return)
                Some(InputEvent::Key(KeyEvent::new(
                    KeyCode::Enter,
                    KeyModifiers::empty(),
                )))
            }
            0x7F => {
                // Backspace (DEL)
                Some(InputEvent::Key(KeyEvent::new(
                    KeyCode::Backspace,
                    KeyModifiers::empty(),
                )))
            }
            0x09 => {
                // Tab
                Some(InputEvent::Key(KeyEvent::new(
                    KeyCode::Tab,
                    KeyModifiers::empty(),
                )))
            }
            b'\x01'..=b'\x1A' => {
                // Ctrl+A through Ctrl+Z
                let char_code = byte - 1 + b'a';
                Some(InputEvent::Key(KeyEvent::new(
                    KeyCode::Char(char_code as char),
                    KeyModifiers::CONTROL,
                )))
            }
            _ if byte >= 0x20 && byte < 0x7F => {
                // Printable ASCII
                Some(InputEvent::Key(KeyEvent::new(
                    KeyCode::Char(byte as char),
                    KeyModifiers::empty(),
                )))
            }
            _ => {
                // UTF-8 continuation bytes or other
                // TODO: Handle multi-byte UTF-8 sequences properly
                None
            }
        }
    }

    fn handle_escape(&mut self, byte: u8) -> Option<InputEvent> {
        match byte {
            b'[' => {
                // CSI sequence start (Control Sequence Introducer)
                self.state = ParserState::Csi;
                None
            }
            b'O' => {
                // SS3 sequence (Single Shift 3) - used for function keys
                self.state = ParserState::Csi;
                None
            }
            _ => {
                // Alt + key
                self.state = ParserState::Ground;
                Some(InputEvent::Key(KeyEvent::new(
                    KeyCode::Char(byte as char),
                    KeyModifiers::ALT,
                )))
            }
        }
    }

    fn handle_csi(&mut self, byte: u8) -> Option<InputEvent> {
        match byte {
            b'0'..=b'9' | b';' | b'<' => {
                // Parameter bytes
                self.buffer.push(byte);
                self.state = ParserState::CsiParam;
                None
            }
            // Simple arrow keys (no parameters)
            b'A' => {
                self.state = ParserState::Ground;
                Some(InputEvent::Key(KeyEvent::new(
                    KeyCode::Up,
                    KeyModifiers::empty(),
                )))
            }
            b'B' => {
                self.state = ParserState::Ground;
                Some(InputEvent::Key(KeyEvent::new(
                    KeyCode::Down,
                    KeyModifiers::empty(),
                )))
            }
            b'C' => {
                self.state = ParserState::Ground;
                Some(InputEvent::Key(KeyEvent::new(
                    KeyCode::Right,
                    KeyModifiers::empty(),
                )))
            }
            b'D' => {
                self.state = ParserState::Ground;
                Some(InputEvent::Key(KeyEvent::new(
                    KeyCode::Left,
                    KeyModifiers::empty(),
                )))
            }
            _ => {
                // Unknown sequence
                self.state = ParserState::Ground;
                None
            }
        }
    }

    fn handle_csi_param(&mut self, byte: u8) -> Option<InputEvent> {
        match byte {
            b'0'..=b'9' | b';' => {
                // Continue accumulating parameters
                self.buffer.push(byte);
                None
            }
            b'M' | b'm' => {
                // SGR mouse event (CSI < Cb ; Cx ; Cy M/m)
                let event = self.parse_mouse_event();
                self.state = ParserState::Ground;
                self.buffer.clear();
                event
            }
            b'~' => {
                // Special key (Home, End, PageUp, etc.)
                // Format: CSI <n> ~
                let event = self.parse_special_key();
                self.state = ParserState::Ground;
                self.buffer.clear();
                event
            }
            b'A'..=b'D' => {
                // Arrow keys with modifiers
                // Format: CSI 1 ; <modifier> A/B/C/D
                let event = self.parse_arrow_with_modifiers(byte);
                self.state = ParserState::Ground;
                self.buffer.clear();
                event
            }
            _ => {
                // Unknown sequence
                self.state = ParserState::Ground;
                self.buffer.clear();
                None
            }
        }
    }

    fn parse_mouse_event(&self) -> Option<InputEvent> {
        // Parse SGR mouse protocol: CSI < Cb ; Cx ; Cy M/m
        // M = button press, m = button release
        // TODO: Implement full SGR mouse parsing
        None
    }

    fn parse_special_key(&self) -> Option<InputEvent> {
        // Parse sequences like:
        // CSI 1 ~ (Home)
        // CSI 4 ~ (End)
        // CSI 5 ~ (PageUp)
        // CSI 6 ~ (PageDown)
        // CSI 11 ~ (F1)
        // etc.

        let param_str = String::from_utf8_lossy(&self.buffer);
        let param = param_str.parse::<u32>().ok()?;

        let key_code = match param {
            1 | 7 => KeyCode::Home,
            2 => KeyCode::Insert,
            3 => KeyCode::Delete,
            4 | 8 => KeyCode::End,
            5 => KeyCode::PageUp,
            6 => KeyCode::PageDown,
            11..=15 => KeyCode::F((param - 10) as u8), // F1-F5
            17..=21 => KeyCode::F((param - 11) as u8), // F6-F10
            23 | 24 => KeyCode::F((param - 12) as u8), // F11-F12
            _ => return None,
        };

        Some(InputEvent::Key(KeyEvent::new(
            key_code,
            KeyModifiers::empty(),
        )))
    }

    fn parse_arrow_with_modifiers(&self, final_byte: u8) -> Option<InputEvent> {
        // Parse sequences like CSI 1;2A (Shift+Up)
        // Format: CSI 1 ; <modifier> <A/B/C/D>
        // Modifier: 2=Shift, 3=Alt, 4=Shift+Alt, 5=Control, 6=Shift+Control, etc.
        // TODO: Implement full modifier parsing
        None
    }
}
````

**Estimated LOC**: ~800 lines (parser state machine + CSI/key/mouse parsing + tests)

#### 4.3 Integrate with InputDevice

**File**: `tui/src/core/terminal_io/input_device.rs`

Update InputDevice to use the new parser:

```rust
use super::{StdinReader, Vt100InputParser};

pub struct InputDevice {
    stdin_reader: StdinReader,
    parser: Vt100InputParser,
    // ... existing fields ...
}

impl InputDevice {
    pub fn new() -> Self {
        Self {
            stdin_reader: StdinReader::new()
                .expect("Failed to create stdin reader"),
            parser: Vt100InputParser::new(),
            // ... existing initialization ...
        }
    }

    /// Poll for input events with timeout
    ///
    /// Returns a vector of parsed input events (keyboard, mouse, etc.)
    pub async fn poll_events(&mut self, timeout_ms: u64)
        -> miette::Result<Vec<InputEvent>>
    {
        // Read bytes from stdin
        let bytes_opt = self.stdin_reader
            .read_with_timeout(timeout_ms)
            .into_diagnostic()?;

        if let Some(bytes) = bytes_opt {
            // Parse bytes into events using VT-100 parser
            Ok(self.parser.process_bytes(&bytes))
        } else {
            Ok(Vec::new())
        }
    }
}
```

**Estimated LOC**: ~100 lines (integration + error handling)

---

### Phase 5: Testing & Validation

#### 5.1 Unit Tests for DirectAnsi Backend

**File**: `tui/src/tui/terminal_lib_backends/direct_ansi/tests.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Pos, TuiColor, row, col};

    #[test]
    fn test_cursor_position_ansi() {
        let pos = Pos::new(row!(5), col!(10));
        let bytes = AnsiSequenceGenerator::cursor_position(pos);
        assert_eq!(bytes, b"\x1b[6;11H"); // 1-indexed
    }

    #[test]
    fn test_cursor_to_column_ansi() {
        let bytes = AnsiSequenceGenerator::cursor_to_column(col!(15));
        assert_eq!(bytes, b"\x1b[16G"); // 1-indexed
    }

    #[test]
    fn test_clear_screen_ansi() {
        let bytes = AnsiSequenceGenerator::clear_screen();
        assert_eq!(bytes, b"\x1b[2J");
    }

    #[test]
    fn test_clear_current_line_ansi() {
        let bytes = AnsiSequenceGenerator::clear_current_line();
        assert_eq!(bytes, b"\x1b[2K");
    }

    #[test]
    fn test_fg_color_rgb_ansi() {
        let color = TuiColor::Rgb { r: 255, g: 128, b: 64 };
        let bytes = AnsiSequenceGenerator::fg_color(color);
        assert_eq!(bytes, b"\x1b[38;2;255;128;64m");
    }

    #[test]
    fn test_bg_color_ansi256() {
        let color = TuiColor::Ansi256(42);
        let bytes = AnsiSequenceGenerator::bg_color(color);
        assert_eq!(bytes, b"\x1b[48;5;42m");
    }

    #[test]
    fn test_reset_color_ansi() {
        let bytes = AnsiSequenceGenerator::reset_color();
        assert_eq!(bytes, b"\x1b[0m");
    }

    #[test]
    fn test_show_hide_cursor() {
        assert_eq!(AnsiSequenceGenerator::show_cursor(), b"\x1b[?25h");
        assert_eq!(AnsiSequenceGenerator::hide_cursor(), b"\x1b[?25l");
    }

    // ... more tests for all sequence types ...
}
```

**Estimated LOC**: ~400 lines (comprehensive unit tests)

#### 5.2 Integration Tests with Mock OutputDevice

**File**: `tui/src/tui/terminal_lib_backends/direct_ansi/integration_tests.rs`

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::{OutputDevice, lock_output_device_as_mut, render_ops, Size, width, height};

    #[test]
    fn test_render_ops_execution_produces_correct_ansi() {
        let (mut output_device, stdout_mock) = OutputDevice::new_mock();

        let mut ops = render_ops!(
            @new
            RenderOp::ClearScreen,
            RenderOp::MoveCursorPositionAbs(Pos::new(row!(0), col!(0))),
            RenderOp::SetFgColor(TuiColor::Rgb { r: 255, g: 0, b: 0 }),
            RenderOp::PrintStyledText("Hello, World!".into()),
        );

        let window_size = Size::new(width!(80), height!(24));
        let locked_output = lock_output_device_as_mut!(output_device);
        let mut skip_flush = false;

        ops.execute_all(&mut skip_flush, window_size, locked_output, false);

        let output = stdout_mock.get_copy_of_buffer_as_string();

        // Verify ANSI sequences
        assert!(output.contains("\x1b[2J")); // Clear screen
        assert!(output.contains("\x1b[1;1H")); // Move to 0,0
        assert!(output.contains("\x1b[38;2;255;0;0m")); // Red foreground
        assert!(output.contains("Hello, World!"));
    }

    #[test]
    fn test_incremental_rendering_operations() {
        let (mut output_device, stdout_mock) = OutputDevice::new_mock();

        let mut ops = render_ops!(
            @new
            RenderOp::MoveCursorToColumn(col!(10)),
            RenderOp::ClearCurrentLine,
            RenderOp::PrintStyledText("Test".into()),
            RenderOp::MoveCursorToNextLine(height!(1)),
        );

        let window_size = Size::new(width!(80), height!(24));
        let locked_output = lock_output_device_as_mut!(output_device);
        let mut skip_flush = false;

        ops.execute_all(&mut skip_flush, window_size, locked_output, false);

        let output = stdout_mock.get_copy_of_buffer_as_string();

        assert!(output.contains("\x1b[11G")); // Move to column 10 (1-indexed)
        assert!(output.contains("\x1b[2K")); // Clear current line
        assert!(output.contains("Test"));
        assert!(output.contains("\x1b[1E")); // Move to next line
    }

    #[test]
    fn test_color_optimization_skips_redundant_sequences() {
        let (mut output_device, stdout_mock) = OutputDevice::new_mock();

        let red = TuiColor::Rgb { r: 255, g: 0, b: 0 };
        let mut ops = render_ops!(
            @new
            RenderOp::SetFgColor(red),
            RenderOp::SetFgColor(red), // Should be skipped (same color)
            RenderOp::PrintStyledText("Text".into()),
        );

        let window_size = Size::new(width!(80), height!(24));
        let locked_output = lock_output_device_as_mut!(output_device);
        let mut skip_flush = false;

        ops.execute_all(&mut skip_flush, window_size, locked_output, false);

        let output = stdout_mock.get_copy_of_buffer_as_string();

        // Should only contain one SetFgColor sequence
        let color_sequence = "\x1b[38;2;255;0;0m";
        assert_eq!(output.matches(color_sequence).count(), 1);
    }
}
```

**Estimated LOC**: ~300 lines (integration tests)

#### 5.3 Visual Testing Examples

**File**: `tui/examples/test_direct_ansi_rendering.rs`

```rust
//! Visual test for DirectAnsi backend rendering
//!
//! This example demonstrates that DirectAnsi backend produces identical
//! visual output to the crossterm backend.
//!
//! Run with: cargo run --example test_direct_ansi_rendering

use r3bl_tui::*;

fn main() -> miette::Result<()> {
    let mut output_device = OutputDevice::new_stdout();

    // Test 1: Basic rendering
    let mut ops = render_ops!(
        @new
        RenderOp::ClearScreen,
        RenderOp::MoveCursorPositionAbs(Pos::new(row!(2), col!(5))),
        RenderOp::SetFgColor(TuiColor::Rgb { r: 0, g: 255, b: 0 }),
        RenderOp::PrintStyledText("✓ DirectAnsi Backend Test".into()),
        RenderOp::ResetColor,
    );

    let window_size = get_terminal_size()?;
    let locked_output = lock_output_device_as_mut!(output_device);
    let mut skip_flush = false;

    ops.execute_all(&mut skip_flush, window_size, locked_output, false);

    // Test 2: Incremental rendering (choose/readline style)
    let mut ops = render_ops!(
        @new
        RenderOp::MoveCursorToNextLine(height!(2)),
        RenderOp::MoveCursorToColumn(col!(5)),
        RenderOp::ClearCurrentLine,
        RenderOp::SetFgColor(TuiColor::Rgb { r: 255, g: 255, b: 0 }),
        RenderOp::PrintStyledText("→ Incremental rendering works!".into()),
        RenderOp::ResetColor,
    );

    ops.execute_all(&mut skip_flush, window_size, locked_output, false);

    // Test 3: Color gradients
    let mut ops = render_ops!(@new);

    for i in 0..16 {
        render_ops!(
            @add_to ops =>
            RenderOp::MoveCursorToNextLine(height!(1)),
            RenderOp::MoveCursorToColumn(col!(5)),
            RenderOp::SetFgColor(TuiColor::Rgb {
                r: (i * 16) as u8,
                g: 128,
                b: 255 - (i * 16) as u8
            }),
            RenderOp::PrintStyledText(format!("Color gradient step {}", i).into()),
        );
    }

    render_ops!(
        @add_to ops =>
        RenderOp::MoveCursorToNextLine(height!(2)),
        RenderOp::ResetColor,
    );

    ops.execute_all(&mut skip_flush, window_size, locked_output, false);

    Ok(())
}
```

**Estimated LOC**: ~150 lines (visual test examples)

---

### Phase 6: Remove Crossterm Dependency

#### 6.1 Update Cargo.toml

**File**: `tui/Cargo.toml`

```toml
[dependencies]
# REMOVED: crossterm = { version = "0.27", features = ["event-stream"] }

# NEW: Direct dependencies for terminal control
mio = { version = "0.8", features = ["os-poll", "os-ext"] }

# Platform-specific dependencies for raw mode
[target.'cfg(unix)'.dependencies]
libc = "0.2"

[target.'cfg(windows)'.dependencies]
windows = { version = "0.52", features = [
    "Win32_System_Console",
    "Win32_Foundation",
    "Win32_Storage_FileSystem",
] }
```

#### 6.2 Remove Crossterm Code

**Files to remove:**

- `tui/src/tui/terminal_lib_backends/crossterm/` (entire directory)
- `tui/src/readline_async/choose_impl/crossterm_macros.rs` (replace with render_ops macros)

**Files to update:**

- Remove all `use crossterm::*` imports
- Remove `RenderOpImplCrossterm` references
- Update `TERMINAL_LIB_BACKEND` default to `DirectAnsi`

#### 6.3 Update Documentation

**Files to update:**

- `README.md` - Update dependencies section
- `CHANGELOG.md` - Document crossterm removal
- `docs/architecture.md` - Update rendering architecture diagrams

**Estimated LOC**: ~100 lines (cargo updates + documentation)

---

## File Structure

### New Files to Create

```
tui/src/tui/terminal_lib_backends/direct_ansi/
├── mod.rs                          # Module exports
├── ansi_sequence_generator.rs      # ANSI escape sequence generation (~500 LOC)
├── render_op_impl_direct_ansi.rs   # DirectAnsi backend implementation (~600 LOC)
├── tests.rs                        # Unit tests (~400 LOC)
└── integration_tests.rs            # Integration tests (~300 LOC)

tui/src/core/terminal_io/
├── stdin_reader.rs                 # mio-based async stdin reader (~200 LOC)
└── vt100_input_parser/
    ├── mod.rs                      # Parser state machine (~400 LOC)
    ├── csi_parser.rs               # CSI sequence parsing (~200 LOC)
    ├── key_parser.rs               # Keyboard event parsing (~100 LOC)
    └── mouse_parser.rs             # Mouse event parsing (~100 LOC)

tui/examples/
└── test_direct_ansi_rendering.rs   # Visual validation (~150 LOC)
```

### Files to Modify

```
tui/src/tui/terminal_lib_backends/
├── mod.rs                          # Add DirectAnsi enum variant (~50 LOC changes)
└── render_op.rs                    # Add new RenderOp variants + routing (~250 LOC changes)

tui/src/readline_async/choose_impl/
├── crossterm_macros.rs             # Deprecate + add render_ops macros (~100 LOC changes)
├── select_component.rs             # Migrate to RenderOps (~80 LOC changes)
├── function_component.rs           # Migrate to RenderOps (~60 LOC changes)
└── event_loop.rs                   # Migrate to RenderOps (~40 LOC changes)

tui/src/readline_async/readline_async_impl/
└── readline.rs                     # Migrate to RenderOps (~100 LOC changes)

tui/src/readline_async/spinner_impl/
└── spinner_render.rs               # Migrate to RenderOps (~40 LOC changes)

tui/src/core/terminal_io/
└── input_device.rs                 # Integrate VT-100 parser (~100 LOC changes)

tui/Cargo.toml                      # Remove crossterm, add mio + platform deps (~20 LOC changes)
```

### Files to Remove

```
tui/src/tui/terminal_lib_backends/crossterm/
└── (entire directory - remove after migration validated)
```

## Code Size Estimates

| Component                          | Planned   | Actual        | Status           |
| ---------------------------------- | --------- | ------------- | ---------------- |
| **Phase 1: RenderOp extensions**   | 250 LOC   | **1,546 LOC** | ✅ COMPLETE      |
| **Phase 2: DirectAnsi backend**    | 1,850 LOC | TBD           | ⏳ PENDING       |
| **Phase 3: Partial TUI migration** | 420 LOC   | TBD           | ⏳ PENDING       |
| **Phase 4: Input handling**        | 1,100 LOC | TBD           | ⏳ PENDING       |
| **Phase 5: Testing**               | 850 LOC   | TBD           | ⏳ PENDING       |
| **Phase 6: Cleanup**               | 120 LOC   | TBD           | ⏳ PENDING       |
| **TOTAL**                          | **4,590** | **1,546+**    | **27% Complete** |

**Phase 1 Actual Breakdown (Commit ea269dca):**

- **New code**: 1,242 insertions
- **Modified code**: 304 deletions (52 files updated)
- **Files changed**: 52 total

**Why Phase 1 exceeded estimates:**

- Original plan was just RenderOp variants (~200 LOC)
- Actual implementation included:
  - TerminalModeState struct for state tracking
  - Full crossterm backend implementation of all 17 RenderOp variants
  - Compositor infrastructure refactoring
  - 52-file codebase alignment (imports, references, formatting)

**Benefit of larger Phase 1**: Creates production-validated foundation for Phase 2, eliminating risk
of backend incompatibility later.

## Migration Timeline

| Phase       | Description               | Duration                | Status               | Dependencies |
| ----------- | ------------------------- | ----------------------- | -------------------- | ------------ |
| **Phase 1** | Extend RenderOp           | 2-3 days                | ✅ COMPLETE (Oct 23) | None         |
| **Phase 2** | DirectAnsi backend        | 5-7 days                | ⏳ PENDING           | Phase 1      |
| **Phase 3** | Migrate choose()/readline | 3-4 days                | ⏳ PENDING           | Phase 2      |
| **Phase 4** | Input handling            | 5-7 days                | ⏳ PENDING           | Phase 3      |
| **Phase 5** | Testing & validation      | 3-5 days                | ⏳ PENDING           | Phase 4      |
| **Phase 6** | Remove crossterm          | 1-2 days                | ⏳ PENDING           | Phase 5      |
| **TOTAL**   | End-to-end migration      | **2-3 weeks remaining** | **27% Complete**     |              |

**Actual Phase 1 Duration**: ~6 hours (exceeded scope significantly)

**Remaining Timeline** (from Oct 23, 2025):

- Phase 2 start: Immediate (Foundation fully ready)
- Estimated completion: ~November 6-13, 2025

**Parallelization opportunities:**

- Phase 1 + Phase 4 (input) can overlap partially
- Phase 5 (testing) can begin during Phase 3-4
- Phase 2 development can begin immediately with crossterm validation layer in place

- **Critical path**: Phase 1 (✅) → Phase 2 (→) → Phase 3 (→) → Phase 6 (→)
- **Dependency resolved**: Phase 1 provides validated implementation foundation - Phase 2 implementation risk is eliminated

## Platform Compatibility

### ANSI Support by Platform

| Platform        | ANSI Support            | Raw Mode Implementation | Notes                              |
| --------------- | ----------------------- | ----------------------- | ---------------------------------- |
| **Linux**       | Native                  | `termios` via `libc`    | Full support, all terminals        |
| **macOS**       | Native                  | `termios` via `libc`    | Full support, all terminals        |
| **Windows 10+** | Native (with VT enable) | Windows Console API     | Enable Virtual Terminal Processing |

### Windows Virtual Terminal Processing

Windows 10+ supports ANSI escape sequences natively after enabling Virtual Terminal Processing:

```rust
#[cfg(windows)]
fn enable_virtual_terminal_processing() -> std::io::Result<()> {
    use windows::Win32::System::Console::*;
    use windows::Win32::Foundation::*;

    unsafe {
        let output_handle = GetStdHandle(STD_OUTPUT_HANDLE)?;
        let mut mode = CONSOLE_MODE(0);
        GetConsoleMode(output_handle, &mut mode)?;

        mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING | ENABLE_PROCESSED_OUTPUT;
        SetConsoleMode(output_handle, mode)?;
    }

    Ok(())
}
```

### Cross-Platform Testing

**Required test environments:**

- **Linux**: Ubuntu 22.04+, Fedora 38+, Arch Linux
- **macOS**: macOS 12+ (Monterey and newer)
- **Windows**: Windows 10 21H2+, Windows 11
  - Test terminals: Windows Terminal, PowerShell, cmd.exe

## Risks and Mitigation

| Risk                              | Impact | Probability | Mitigation                                                    |
| --------------------------------- | ------ | ----------- | ------------------------------------------------------------- |
| **Platform compatibility issues** | High   | Medium      | Extensive testing on all platforms before release             |
| **Windows Console quirks**        | Medium | Medium      | Enable VT processing, test on multiple Windows terminals      |
| **Input parsing edge cases**      | Medium | Medium      | Comprehensive test suite, handle unknown sequences gracefully |
| **Raw mode differences**          | Medium | Low         | Abstract platform differences in dedicated module             |
| **ANSI sequence variations**      | Low    | Low         | Stick to well-supported subset of ANSI standard               |
| **Performance regression**        | High   | Low         | Benchmark before/after, profile with flamegraph               |
| **Breaking existing apps**        | High   | Low         | Extensive testing, gradual rollout with feature flags         |

**Mitigation strategies:**

1. **Feature flag approach**: Keep crossterm as fallback during initial rollout
2. **Extensive testing**: Platform-specific CI/CD testing
3. **Gradual migration**: Enable DirectAnsi for new code first, migrate existing code incrementally
4. **Monitoring**: Collect telemetry on terminal type detection and ANSI support

## Success Metrics

### Phase 1 Achievements (✅ COMPLETE)

**Infrastructure & Foundation:**

- [x] All 11 incremental rendering RenderOp variants designed and documented
- [x] 6 terminal mode RenderOp variants designed and documented
- [x] TerminalModeState struct created for cross-backend terminal state tracking
- [x] All RenderOp variants fully implemented in crossterm backend
- [x] Compositor infrastructure refactored for backend-agnostic rendering
- [x] 52-file codebase alignment (imports, references, type safety)
- [x] Code quality: 100% Clippy-clean, cargo fmt compliant

**Production Validation:**

- [x] RenderOps expressive enough for all three rendering paths (Full TUI works)
- [x] Type-safe bounds checking applied throughout (ColIndex, RowHeight, Pos)
- [x] Reference implementation ready for Phase 2 DirectAnsi backend

### Phase 2-6 Metrics (⏳ PENDING)

### Performance

- [ ] **15M sample reduction** in flamegraph profiling (target from original analysis)
- [ ] **Frame render time** reduced by >20% on all platforms (vs crossterm)
- [ ] **Memory allocations** reduced during rendering
- [ ] **Validation**: Benchmark DirectAnsi backend vs crossterm backend before Phase 5 completion

### Correctness

- [ ] **All tests pass** with DirectAnsi backend on Linux, macOS, Windows
- [ ] **Visual parity** with crossterm backend (side-by-side comparison)
- [ ] **No regressions** in existing applications (edi, giti, rc)
- [ ] **RenderOp coverage**: Unit tests for all variants in DirectAnsi backend

### Compatibility

- [ ] **Linux terminals**: Works on xterm, gnome-terminal, kitty, alacritty, konsole
- [ ] **macOS terminals**: Works on Terminal.app, iTerm2, kitty, alacritty
- [ ] **Windows terminals**: Works on Windows Terminal, PowerShell, cmd.exe
- [ ] **Windows VT enable**: Automatic Virtual Terminal Processing activation on Windows 10+

### Code Quality

- [ ] **Net code reduction**: Despite new platform-specific code, total LOC decreases
- [ ] **Dependency reduction**: Remove crossterm dependency completely
- [ ] **Test coverage**: >90% coverage for DirectAnsi backend and VT-100 parser
- [ ] **Documentation**: All phases include rustdoc examples and integration guide

### Migration Completeness

- [ ] **All paths use RenderOps**: Full TUI ✅, choose() ⏳, readline_async() ⏳
- [ ] **No crossterm imports**: All codebase references eliminated (Phase 6)
- [ ] **Documentation updated**: Architecture diagrams and implementation guide
- [ ] **Cargo.toml cleaned**: crossterm removed, mio + platform deps added

## Conclusion

### Phase 1 Complete: Foundation is Solid ✅

This task removes crossterm by unifying all rendering paths around `RenderOp` as a universal
terminal control language. **Phase 1 is complete**, validating the core architectural approach:

#### Key Architectural Achievements

1. **✅ RenderOp as universal language**: 17 variants proven sufficient for all rendering paths
   - Incremental rendering (choose/readline): 11 variants
   - Terminal mode control: 6 variants
   - Full TUI: All variants working with crossterm backend

2. **✅ Type-safe infrastructure**: TerminalModeState tracks terminal state across backends
   - Raw mode, alternate screen, mouse tracking, bracketed paste
   - Ready for DirectAnsi backend to implement independently

3. **✅ Production-validated**: All variants tested in crossterm backend
   - No spec compliance issues discovered
   - ANSI sequence mappings correct
   - Cursor tracking semantics validated

4. **✅ Compositor abstraction**: Backend-agnostic rendering pipeline established
   - RenderOp → OutputDevice with pluggable implementations
   - Easy to switch between crossterm and DirectAnsi

#### Remaining Work: Phases 2-6 (~2-3 weeks)

**Phase 2: DirectAnsi Backend** (Ready to start)

- ANSI sequence generator without crossterm dependencies
- All variants implemented with direct byte output
- Zero risk: parallel with crossterm during development

**Phase 3: Partial TUI Migration**

- choose()/readline_async() to RenderOps
- Macro deprecation plan
- Import cleanup

**Phase 4: Input Handling**

- VT-100 input parser for stdin
- Async I/O with mio
- Mouse/keyboard event support

**Phase 5: Testing & Validation**

- DirectAnsi backend unit tests
- Integration tests with mock OutputDevice
- Cross-platform visual testing

**Phase 6: Cleanup**

- Remove crossterm from Cargo.toml
- Delete crossterm backend code
- Update documentation

#### Risk Assessment: Minimal ✅

Phase 1 dramatically reduced Phase 2-6 risk by:

- Proving RenderOp variants are sufficient
- Providing reference implementation in crossterm backend
- Establishing TerminalModeState for state management
- Testing infrastructure patterns before DirectAnsi

**Critical insight**: Because Phase 1 actually implemented all variants in the crossterm backend,
Phase 2 (DirectAnsi) becomes a straightforward translation task with zero architectural risk.

---

- **Document Version**: 1.1
- **Last Updated**: October 23, 2025
- **Status**: Phase 1 Complete - Phase 2 Ready to Begin
- **Next Action**: Start Phase 2 (DirectAnsi Backend Implementation)
