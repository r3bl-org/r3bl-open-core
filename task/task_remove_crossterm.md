<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Task: Remove Crossterm via Unified RenderOp Architecture](#task-remove-crossterm-via-unified-renderop-architecture)
  - [Overview](#overview)
    - [Dependency: Requires task_unify_rendering.md Completion](#dependency-requires-task_unify_renderingmd-completion)
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
  - [Implementation Plan](#implementation-plan)
  - [Step 0: Prerequisite Setup [COMPLETE]](#step-0-prerequisite-setup-complete)
  - [Step 1: Extend RenderOp for Incremental Rendering [COMPLETE]](#step-1-extend-renderop-for-incremental-rendering-complete)
    - [Key Accomplishments:](#key-accomplishments)
  - [Step 2: Implement DirectAnsi Backend [COMPLETE]](#step-2-implement-directansi-backend-complete)
    - [Step 2.1: Create DirectAnsi Module Structure [COMPLETE]](#step-21-create-directansi-module-structure-complete)
    - [Step 2.2: Implement AnsiSequenceGenerator [COMPLETE]](#step-22-implement-ansisequencegenerator-complete)
  - [Step 3: Complete Type System Architecture & DirectAnsi Backend [COMPLETE]](#step-3-complete-type-system-architecture--directansi-backend-complete)
    - [Step 3.0: Remove IR Execution Path & Enforce Semantic Boundary [COMPLETE]](#step-30-remove-ir-execution-path--enforce-semantic-boundary-complete)
    - [Step 3.1: Create RenderOpOutput Execution Path [COMPLETE]](#step-31-create-renderopoutput-execution-path-complete)
    - [Step 3.2: Fix OffscreenBufferPaint Trait & RawMode Infrastructure [COMPLETE]](#step-32-fix-offscreenbufferpaint-trait--rawmode-infrastructure-complete)
    - [Step 3.3: Implement RenderOpPaintImplDirectAnsi (DirectAnsi Backend) [COMPLETE]](#step-33-implement-renderoppaintimpldirectansi-directansi-backend-complete)
  - [Step 4: Linux Validation & Performance Testing [COMPLETE]](#step-4-linux-validation--performance-testing-complete)
    - [Key Findings:](#key-findings)
  - [Step 5: Performance Validation & Optimization [COMPLETE]](#step-5-performance-validation--optimization-complete)
    - [Performance Results](#performance-results)
      - [Baseline & Results](#baseline--results)
    - [Optimizations Implemented](#optimizations-implemented)
      - [Stack-Allocated Number Formatting [COMPLETE]](#stack-allocated-number-formatting-complete)
      - [U8_STRINGS Lookup Table for Color Sequences [COMPLETE]](#u8_strings-lookup-table-for-color-sequences-complete)
      - [SmallVec[16] Optimization [COMPLETE]](#smallvec16-optimization-complete)
      - [StyleUSSpan[16] Optimization [COMPLETE]](#styleusspan16-optimization-complete)
  - [Step 6: Cleanup & Architectural Refinement [COMPLETE]](#step-6-cleanup--architectural-refinement-complete)
    - [6.1: DirectToAnsi Rename [COMPLETE]](#61-directtoansi-rename-complete)
    - [6.2: Remove Termion Backend (Dead Code Removal) [COMPLETE]](#62-remove-termion-backend-dead-code-removal-complete)
    - [6.3: Review `cli_text` and `tui_styled_text` Consistency [COMPLETE]](#63-review-cli_text-and-tui_styled_text-consistency-complete)
  - [Step 7: Comprehensive RenderOp Integration Test Suite [COMPLETE]](#step-7-comprehensive-renderop-integration-test-suite-complete)
    - [Summary](#summary)
    - [Part A: Color Operations [COMPLETE]](#part-a-color-operations-complete)
    - [Part B: Cursor Movement Operations [COMPLETE]](#part-b-cursor-movement-operations-complete)
    - [Part C: Screen Operations [COMPLETE]](#part-c-screen-operations-complete)
    - [Part D: State Optimization [COMPLETE]](#part-d-state-optimization-complete)
    - [Part E: Text Painting Operations [COMPLETE]](#part-e-text-painting-operations-complete)
    - [Final QA [COMPLETE]](#final-qa-complete)
  - [Step 8: Implement InputDevice for DirectToAnsi Backend [COMPLETE - Linux]](#step-8-implement-inputdevice-for-directtoansi-backend-complete---linux)
    - [Architecture](#architecture)
    - [Step 8.0: Reorganize Existing Output Files [COMPLETE]](#step-80-reorganize-existing-output-files-complete)
    - [Step 8.1: Architecture Design [COMPLETE]](#step-81-architecture-design-complete)
    - [Step 8.2: Implement Protocol Layer Parsers [COMPLETE]](#step-82-implement-protocol-layer-parsers-complete)
      - [Keyboard Parsing [COMPLETE]](#keyboard-parsing-complete)
      - [SS3 Keyboard Support [COMPLETE]](#ss3-keyboard-support-complete)
      - [Kitty Keyboard Protocol (CSI u) Support [COMPLETE]](#kitty-keyboard-protocol-csi-u-support-complete)
      - [Mouse Parsing [COMPLETE]](#mouse-parsing-complete)
      - [Terminal Events & OSC Parsing [COMPLETE]](#terminal-events--osc-parsing-complete)
      - [UTF-8 Text Parsing [COMPLETE]](#utf-8-text-parsing-complete)
    - [Step 8.2.1: Crossterm Feature Parity Analysis [COMPLETE]](#step-821-crossterm-feature-parity-analysis-complete)
    - [Step 8.2.2: Architecture Insight - Mio Poller, MaybeMore & Zero-Latency ESC [COMPLETE]](#step-822-architecture-insight---mio-poller-maybemore--zero-latency-esc-complete)
    - [Step 8.3: Backend Device Implementation [COMPLETE]](#step-83-backend-device-implementation-complete)
    - [Step 8.4: Testing & Validation [COMPLETE]](#step-84-testing--validation-complete)
    - [Step 8.5: Migration & Cleanup [COMPLETE]](#step-85-migration--cleanup-complete)
    - [Step 8.6: Resolve TODOs and Stubs [PENDING]](#step-86-resolve-todos-and-stubs-pending)
  - [Step 9: macOS & Windows Platform Validation & Crossterm Removal [PENDING]](#step-9-macos--windows-platform-validation--crossterm-removal-pending)
    - [macOS Drivers & Testing [PENDING]](#macos-drivers--testing-pending)
    - [Windows Drivers & Testing [PENDING]](#windows-drivers--testing-pending)
    - [Crossterm Dependency Removal [PENDING]](#crossterm-dependency-removal-pending)
  - [Implementation Checklist](#implementation-checklist)
  - [Critical Success Factors](#critical-success-factors)
  - [Effort Summary - Steps 1-7 Implementation](#effort-summary---steps-1-7-implementation)
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

### Dependency: Requires task_unify_rendering.md Completion

**This task depends on completion of [task_unify_rendering.md](done/task_unify_rendering.md):**

| Unification Phase      | Output                                                   | Status                                 | Notes                                  |
| ---------------------- | -------------------------------------------------------- | -------------------------------------- | -------------------------------------- |
| **0.5** (prerequisite) | CliTextInline uses CliTextInline abstraction for styling | [COMPLETE] COMPLETE                    | Standardizes styling before renaming   |
| **1** (rename)         | AnsiStyledText → CliTextInline                           | [COMPLETE] COMPLETE (October 21, 2025) | Type rename across codebase            |
| **2** (core)           | `PixelCharRenderer` module created                       | [COMPLETE] COMPLETE (October 22, 2025) | Unified ANSI sequence generator        |
| **3** (integration)    | `RenderToAnsi` trait for unified buffer rendering        | [COMPLETE] COMPLETE (October 22, 2025) | Ready for DirectAnsi backend           |
| **4** (CURRENT)        | `CliTextInline` uses `PixelCharRenderer` via traits      | [COMPLETE] COMPLETE (October 22, 2025) | All direct text rendering unified      |
| **5** (DEFERRED)       | choose()/readline_async to OffscreenBuffer               | ⏸️ DEFERRED to Future Work (Step 9+)   | Proper migration is via RenderOps      |
| **6** (COMPLETE)       | `RenderOpImplCrossterm` uses `PixelCharRenderer`         | [COMPLETE] COMPLETE (October 22, 2025) | Unified renderer validated in full TUI |

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
                     │ (Current)
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
        ▼ (Now)         ▼ (Steps 2-5)   ▼ (Future)
    Crossterm       DirectAnsi       DirectAnsi
    OutputDevice    Backend          Backend
       (Current)    (Steps 2-5)      (Future)
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

### Where Crossterm is Used Today

1. **Full TUI**: Uses `RenderOpImplCrossterm` backend to execute RenderOps
2. **choose()**: Directly calls crossterm via `queue_commands!` macro
3. **readline_async()**: Directly calls crossterm via `queue_commands!` macro
4. **Input handling**: Uses `crossterm::event::read()` for keyboard/mouse events

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

## Implementation Plan

## Step 0: Prerequisite Setup [COMPLETE]

All prerequisites, repository dependencies, and RenderOp design requirements are satisfied.

## Step 1: Extend RenderOp for Incremental Rendering [COMPLETE]

- **Status**: [COMPLETE] **COMPLETE** (Commit: `ea269dca`)
- **Date**: October 23, 2025
- **Commit Message**: `[tui] Prepare compositor and renderops for crossterm removal`

All 11 new `RenderOp` variants have been successfully added to
`tui/src/tui/terminal_lib_backends/render_op.rs` with comprehensive documentation.

### Key Accomplishments:

- [COMPLETE] Added 11 new RenderOp variants for incremental rendering
- [COMPLETE] Implemented TerminalModeState infrastructure for tracking terminal state
- [COMPLETE] Fully implemented all RenderOp variants in Crossterm backend
- [COMPLETE] Renamed and restructured compositor logic
- [COMPLETE] Code quality: All 52 affected files updated, clippy compliant
- [COMPLETE] Type-safe bounds checking (ColIndex, RowHeight, Pos)

## Step 2: Implement DirectAnsi Backend [COMPLETE]

**Status**: [COMPLETE] STEPS 1-2 COMPLETE (October 23, 2025) | [WORK_IN_PROGRESS] Step 3 Ready

### Step 2.1: Create DirectAnsi Module Structure [COMPLETE]

- Created `tui/src/tui/terminal_lib_backends/direct_ansi/` directory
- Implemented `mod.rs` with proper re-exports and Step 2.1 organization
- Created all implementation files with proper documentation
- `cargo check` passes cleanly

### Step 2.2: Implement AnsiSequenceGenerator [COMPLETE]

- **All 40+ methods implemented** using semantic ANSI generation (not raw format!)
- **Key Achievement**: Replaced raw `format!()` calls with semantic typed enums
- **Leveraged VT-100 Infrastructure**: CsiSequence, SgrColorSequence, PrivateModeType enums
- **Type Safety**: All sequences are type-safe with compile-time guarantees
- **Test Coverage**: [COMPLETE] 33/33 unit tests passing

## Step 3: Complete Type System Architecture & DirectAnsi Backend [COMPLETE]

**Status**: [COMPLETE] COMPLETE - (October 26, 2025)

### Step 3.0: Remove IR Execution Path & Enforce Semantic Boundary [COMPLETE]

**Objective**: Delete the direct IR execution path, forcing all operations through the Compositor.

### Step 3.1: Create RenderOpOutput Execution Path [COMPLETE]

**Objective**: Implement the missing `RenderOpOutputVec::execute_all()` method and routing
infrastructure.

### Step 3.2: Fix OffscreenBufferPaint Trait & RawMode Infrastructure [COMPLETE]

**Objective**: Fix `OffscreenBufferPaint::render()` to return `RenderOpOutputVec` and update RawMode
to use the pipeline properly.

### Step 3.3: Implement RenderOpPaintImplDirectAnsi (DirectAnsi Backend) [COMPLETE]

**Objective**: Implement the DirectAnsi backend to execute `RenderOpOutput` operations.

**Status**: [COMPLETE] COMPLETE

- [COMPLETE] DirectAnsi backend fully implements RenderOpOutput execution
- [COMPLETE] All 27 RenderOpCommon variants handled
- [COMPLETE] Post-compositor text rendering integrated
- [COMPLETE] State tracking via RenderOpsLocalData for optimization
- [COMPLETE] Comprehensive unit and integration test coverage

## Step 4: Linux Validation & Performance Testing [COMPLETE]

**Status**: [COMPLETE] COMPLETE (October 26, 2025)

**Scope**: Linux platform validation and performance benchmarking. macOS and Windows testing
deferred to Step 9.

### Key Findings:

**Functional Testing**: [COMPLETE] **PASS**

- DirectAnsi backend fully functional on Linux
- All rendering operations work correctly
- No visual artifacts or garbled output

**Performance Benchmarking**: [COMPLETE] **PASS**

| Backend             | Total Samples | Status   |
| ------------------- | ------------- | -------- |
| **Crossterm**       | 344,240,761   | Baseline |
| **DirectAnsi (v1)** | 535,582,797   | +55.58%  |

**Result**: Performance regression detected, but improvement planned for Step 5.

## Step 5: Performance Validation & Optimization [COMPLETE]

**Status**: [COMPLETE] COMPLETE (October 26, 2025)

### Performance Results

**Benchmark Command**: `./run.fish run-examples-flamegraph-fold --benchmark`

**Methodology**: 8-second continuous workload, 999 Hz sampling, scripted input (pangrams, cursor
movements)

#### Baseline & Results

```
DirectToAnsi vs Crossterm: 107.3M / 122.5M = 0.876
Result: DirectToAnsi is 12.4% FASTER than Crossterm [COMPLETE]
```

**Victory Summary**: DirectToAnsi achieves the goal of matching or exceeding Crossterm performance.

### Optimizations Implemented

#### Stack-Allocated Number Formatting [COMPLETE]

- Replaced heap-allocated `.to_string()` calls with stack-allocated u16 formatting
- Eliminated 42 heap allocations in rendering hot path
- Impact: Removed `core::fmt::num::imp::<impl u16>::_fmt` hotspot entirely

#### U8_STRINGS Lookup Table for Color Sequences [COMPLETE]

- Pre-computed compile-time lookup table for all u8 values (0-255)
- O(1) array lookup instead of runtime integer-to-string formatting
- Impact: All color operations now optimal

#### SmallVec[16] Optimization [COMPLETE]

- Increased INLINE_VEC_SIZE from 8 → 16
- Eliminated 0.47% CPU cost from RenderOpIR spillage

#### StyleUSSpan[16] Optimization [COMPLETE]

- Increased DEFAULT_LIST_STORAGE_SIZE from 8 → 16
- Eliminated ~5.0% CPU cost from StyleUSSpan spillage

**Final Performance Summary**:

```
DirectToAnsi vs Crossterm (baseline):        12.4% faster
+ SmallVec[16] optimization:                 +0.47%
+ StyleUSSpan[16] optimization:              +~5.0%
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total improvement: ~18% faster than Crossterm [COMPLETE][COMPLETE]
```

## Step 6: Cleanup & Architectural Refinement [COMPLETE]

**Status**: [COMPLETE] COMPLETE (October 28, 2025)

**Objective**: Polish the codebase after DirectToAnsi integration and remove dead code/debt.

### 6.1: DirectToAnsi Rename [COMPLETE]

**Status**: [COMPLETE] COMPLETE (October 26, 2025)

The `direct_ansi/` module has already been renamed to `direct_to_ansi/` with:

- Directory structure updated
- Module declarations in `mod.rs` updated
- All imports and re-exports complete
- Documentation references updated

### 6.2: Remove Termion Backend (Dead Code Removal) [COMPLETE]

**Status**: [COMPLETE] COMPLETE (October 28, 2025)

**Rationale**: Termion was never implemented and was dead code.

**Finding**: Already removed - termion_backend directory and TerminalLibBackend::Termion variant no longer exist in codebase. Only documentation references remain (as a "future possibility" comment).

### 6.3: Review `cli_text` and `tui_styled_text` Consistency [COMPLETE]

**Status**: AUDIT COMPLETE (October 27, 2025)

**Finding**: Keep Separate (Different Use Cases)

**Rationale**:

1. **Different Abstraction Levels**: `cli_text` is low-level, `tui_styled_text` is high-level
2. **Different Rendering Paths**: `cli_text` uses direct PixelCharRenderer, `tui_styled_text` uses
   RenderOp pipeline
3. **Different Style APIs**: Consolidating would require unifying style API (out of scope)
4. **Different Performance Profiles**: Each optimized for its use case
5. **Intentional Naming**: `cli_text*` vs `tui_styled_text` conveys intended use

**Recommendation**: Keep modules separate due to semantic differences and different rendering paths.

## Step 7: Comprehensive RenderOp Integration Test Suite [COMPLETE]

**Status**: [COMPLETE] COMPLETED - October 27, 2025

**Objective**: Build a robust, comprehensive test suite that validates the full RenderOp execution
pipeline with DirectToAnsi backend.

### Summary

- [COMPLETE] All tests compile without errors
- [COMPLETE] Tests validate both ANSI output AND state changes
- [COMPLETE] Test coverage for all major RenderOpCommon variants
- [COMPLETE] Clear error messages if any assertion fails

### Part A: Color Operations [COMPLETE]

- [COMPLETE] SetFgColor RenderOp generates correct SGR foreground sequence
- [COMPLETE] SetBgColor RenderOp generates correct SGR background sequence
- [COMPLETE] Color state tracking validated
- [COMPLETE] ResetColor clears both fg and bg color state
- [COMPLETE] Multiple color operations in sequence tested
- [COMPLETE] ANSI format validation (colon-separated format)

### Part B: Cursor Movement Operations [COMPLETE]

- [COMPLETE] MoveCursorPositionAbs updates cursor state correctly
- [COMPLETE] Cursor position accessible via `Pos`
- [COMPLETE] MoveCursorPositionRelTo works correctly
- [COMPLETE] Cursor state verification after movement
- [COMPLETE] Multiple cursor moves in sequence tested

### Part C: Screen Operations [COMPLETE]

- [COMPLETE] ClearScreen generates CSI 2J
- [COMPLETE] ShowCursor generates DECTCEM set
- [COMPLETE] HideCursor generates DECTCEM reset
- [COMPLETE] Mode state tracking tested

### Part D: State Optimization [COMPLETE]

- [COMPLETE] Redundant cursor moves produce no output
- [COMPLETE] Redundant color changes skip second output
- [COMPLETE] State persistence across unrelated operations
- [COMPLETE] State clearing works correctly
- [COMPLETE] Complex workflows maintain correct state

### Part E: Text Painting Operations [COMPLETE]

- [COMPLETE] Plain text rendering without style attributes
- [COMPLETE] Text with foreground color
- [COMPLETE] Text with background color
- [COMPLETE] Text with combined colors
- [COMPLETE] Text with style attributes
- [COMPLETE] Cursor position advancement
- [COMPLETE] Multiple sequential text operations
- [COMPLETE] Edge cases: empty strings, special characters, Unicode/emoji
- [COMPLETE] State validation: cursor tracking
- [COMPLETE] Integration with PixelCharRenderer

### Final QA [COMPLETE]

- [COMPLETE] `cargo check` passes with zero errors
- [COMPLETE] `cargo test --lib` - all tests pass
- [COMPLETE] `cargo clippy --all-targets` - zero warnings
- [COMPLETE] `cargo fmt --all -- --check` - proper formatting
- [COMPLETE] All new tests have clear documentation
- [COMPLETE] Edge cases are covered

**Sign-Off**: [COMPLETE] DirectToAnsi backend is robust, tested, and production-ready

## Step 8: Implement InputDevice for DirectToAnsi Backend [COMPLETE - Linux]

**Status**: [COMPLETE] Linux InputDevice implementation complete with Crossterm feature parity, Kitty CSI u support, OSC handling, MaybeMore stream tracking, and resilient circuit-breaker recovery.

**Objective**: Replace `crossterm::event::EventStream` on Linux with native `mio`-based non-blocking stdin polling and a decoupled chunk framer / chunk decoder pipeline.

**Rationale**: Completes the DirectToAnsi pipeline on Linux, delivering high-performance, pure-Rust terminal input handling without Crossterm dependency.

### Architecture

```
Layer 1: Protocol Parsing & Framing (core/ansi/vt_100_terminal_input_parser/)
  ├── chunk_framer/            # Slices raw streams, CircuitBreaker, DrainState, ByteOffset
  │   ├── circuit_breaker/     # Runaway payload protection & safety limits
  │   └── mod.rs               # ChunkFramer state machine with MaybeMore
  ├── chunk_decoder/           # Pure, zero-allocation decoders from byte chunks to VT100InputEventIR
  │   ├── keyboard/            # CSI, SS3, CSI u (Kitty), Alt disambiguation
  │   ├── mouse/               # SGR, X10, RXVT mouse decoders
  │   ├── terminal_events/     # Resize, focus, bracketed paste, OSC queries/reports
  │   └── utf8/                # Character input decoding
  ├── ir_event_types.rs        # Intermediate event representations (including Ignored)
  └── maybe_more.rs            # KernelDrained vs KernelMayHaveMore stream status

Layer 2: Backend I/O & Actor Polling (terminal_lib_backends/direct_to_ansi/input/)
  ├── mio_poller/              # Dedicated OS thread with mio::Poll on /dev/tty
  │   ├── mio_poll_worker.rs   # Event loop polling stdin, signals, and software interrupts
  │   └── handler_stdin.rs     # Non-blocking reads evaluating MaybeMore
  ├── input_device_impl.rs     # DirectToAnsiInputDevice channel consumer
  ├── input_device_public_api.rs # Public API & single-instance lifecycle guard
  ├── paste_state_machine.rs   # Bracketed paste reassembly
  └── protocol_conversion.rs   # Conversion from IR events to InputEvent
```

### Step 8.0: Reorganize Existing Output Files [COMPLETE]

**Objective**: Create clean `input/` and `output/` subdirectories within DirectToAnsi backend

**Status**: [COMPLETE] COMPLETE

**Directory Structure After Reorganization**:

```
tui/src/tui/terminal_lib_backends/direct_to_ansi/
├── mod.rs                          ← Backend coordinator
├── debug.rs                        ← Debug utilities
├── input/                          ← Input handling (mio poller, subscriber, conversion)
├── output/                         ← Output handling (RenderOp painter, pixel renderer)
│   ├── mod.rs
│   ├── render_to_ansi.rs
│   ├── paint_render_op_impl.rs
│   ├── pixel_char_renderer.rs
│   └── tests.rs
└── integration_tests/              ← Tests
```

### Step 8.1: Architecture Design [COMPLETE]

**Status**: [COMPLETE] COMPLETE

**Approved Architecture**:

- [COMPLETE] **Two-layer separation**: Framing and decoding (pure) separate from I/O polling (mio)
- [COMPLETE] **Platform strategy**: Linux uses DirectToAnsi, macOS/Windows use crossterm (pending Step 9)
- [COMPLETE] **Async I/O**: Dedicated `mio` worker thread polling `/dev/tty` non-blocking with OS pipe / channel dispatch to tokio tasks (replaces problematic `tokio::io::stdin()`)
- [COMPLETE] **Stream availability**: Evaluated via `MaybeMore` (`KernelDrained` vs `KernelMayHaveMore`) at I/O read boundary
- [COMPLETE] **ANSI protocols supported**: Keyboard (CSI + SS3 + CSI u Kitty), Mouse (SGR + X10 + RXVT), Focus, Paste, OSC 10-19 color reports, OSC 52 clipboard, UTF-8
- [COMPLETE] **Resiliency**: Runaway OSC payload protection via `CircuitBreaker` and `DrainState`
- [COMPLETE] **Naming**: `vt_100_pty_output_parser` (existing) + `vt_100_terminal_input_parser` (new)

### Step 8.2: Implement Protocol Layer Parsers [COMPLETE]

**Status**: [COMPLETE] **PROTOCOL PARSERS COMPLETE - CROSSTERM FEATURE PARITY ACHIEVED**

#### Keyboard Parsing [COMPLETE]

- [COMPLETE] Implemented `parse_keyboard_sequence(bytes: &[u8]) -> Option<(InputEvent, usize)>`
- [COMPLETE] Arrow keys: CSI A/B/C/D → KeyCode::Up/Down/Right/Left
- [COMPLETE] Function keys: CSI <n>~ → KeyCode::Function(1-12)
- [COMPLETE] Home/End: CSI H/F → KeyCode::Home/End
- [COMPLETE] Modified Home/End: CSI 1;m H/F → KeyCode::Home/End with modifiers (xterm standard)
- [COMPLETE] Modified Function keys F1-F4: CSI 1;m P/Q/R/S → KeyCode::Function(1-4) with modifiers
- [COMPLETE] Backtab: CSI Z → KeyCode::BackTab
- [COMPLETE] Modifier combinations: CSI 1;m final_byte
- [COMPLETE] All critical keyboard sequences handled and covered by unit/roundtrip tests

#### SS3 Keyboard Support [COMPLETE]

- [COMPLETE] Implemented `parse_ss3_sequence()` for application mode (vim, less, emacs)
- [COMPLETE] Arrow keys: ESC O A/B/C/D → KeyCode::Up/Down/Right/Left
- [COMPLETE] Function keys F1-F4: ESC O P/Q/R/S → KeyCode::Function(1-4)
- [COMPLETE] Critical for vim/application mode compatibility

#### Kitty Keyboard Protocol (CSI u) Support [COMPLETE]

- [COMPLETE] Implemented `parse_csi_u()` in `chunk_decoder/keyboard/csi_u.rs`
- [COMPLETE] Disambiguates `Alt+[` (`ESC [`) from standard CSI sequences
- [COMPLETE] Decodes Kitty progressive enhancement sequences (`CSI <codepoint> ; <modifiers> u`)
- [COMPLETE] Terminal mode controller pushes `PushKeyboardEnhancementFlags` on startup and pops on shutdown
- [COMPLETE] Tested with unit and roundtrip tests

#### Mouse Parsing [COMPLETE]

**SGR Protocol** [COMPLETE]:

- [COMPLETE] Implemented `parse_sgr_mouse()` for modern terminals
- [COMPLETE] Button detection: bits 0-1 of Cb (0=left, 1=middle, 2=right)
- [COMPLETE] Drag detection: bit 5 in Cb
- [COMPLETE] Scroll detection: buttons 64-67
- [COMPLETE] 1-based coordinate handling
- [COMPLETE] 6 unit tests passing

**X10 Protocol** [COMPLETE]:

- [COMPLETE] Implemented `parse_x10_mouse()` for legacy xterm/screen/tmux
- [COMPLETE] Format: ESC [ M Cb Cx Cy (6 bytes fixed)
- [COMPLETE] Button decoding and coordinate conversion
- [COMPLETE] 12 unit tests passing

**RXVT Protocol** [COMPLETE]:

- [COMPLETE] Implemented `parse_rxvt_mouse()` for rxvt/urxvt terminals
- [COMPLETE] Format: ESC [ Cb ; Cx ; Cy M (semicolon-separated)
- [COMPLETE] Same button encoding as X10
- [COMPLETE] 13 unit tests passing

#### Terminal Events & OSC Parsing [COMPLETE]

- [COMPLETE] Implemented `parse_terminal_event()` dispatcher
- [COMPLETE] Resize events: CSI 8 ; rows ; cols t
- [COMPLETE] Focus events: CSI I (gained) / CSI O (lost)
- [COMPLETE] Bracketed paste: ESC[200~ (start) / ESC[201~ (end)
- [COMPLETE] OSC Sequence parsing: OSC 10, 11, 12, 13, 14, 17, 19 dynamic color query responses and OSC 52 clipboard transfers
- [COMPLETE] `Alt+]` vs OSC disambiguation via lexical scanning and `MaybeMore`
- [COMPLETE] `VT100InputEventIR::Ignored` variant for consumed protocol responses
- [COMPLETE] Circuit breaker and runaway OSC payload protection (1 MiB cap with drain state)

#### UTF-8 Text Parsing [COMPLETE]

- [COMPLETE] Implemented `parse_utf8_text()` for character input
- [COMPLETE] 1-byte ASCII (0x00-0x7F)
- [COMPLETE] 2-byte sequences (0xC0-0xDF)
- [COMPLETE] 3-byte sequences (0xE0-0xEF)
- [COMPLETE] 4-byte sequences (0xF0-0xF7)
- [COMPLETE] Invalid/incomplete handling
- [COMPLETE] 13 unit tests passing

### Step 8.2.1: Crossterm Feature Parity Analysis [COMPLETE]

**Mouse Protocol Support**:

| Protocol       | Status              | Use Case                                |
| -------------- | ------------------- | --------------------------------------- |
| **SGR**        | [COMPLETE] COMPLETE | Modern standard (kitty, alacritty, etc) |
| **Normal/X10** | [COMPLETE] COMPLETE | Legacy xterm, screen, tmux              |
| **RXVT**       | [COMPLETE] COMPLETE | rxvt/urxvt terminals                    |

**Keyboard Sequence Support**:

| Sequence Type | Status              | Use Case                                           |
| ------------- | ------------------- | -------------------------------------------------- |
| **CSI**       | [COMPLETE] COMPLETE | Arrow keys, function keys, modifiers (normal mode) |
| **SS3**       | [COMPLETE] COMPLETE | Arrow keys, F1-F4 in application mode              |
| **Kitty**     | [COMPLETE] COMPLETE | Advanced: CSI u progressive enhancement, Alt+[     |

**Terminal Compatibility Matrix**:

| Terminal         | Keyboard    | Mouse Protocol | Status           |
| ---------------- | ----------- | -------------- | ---------------- |
| xterm (normal)   | CSI         | SGR            | [COMPLETE] WORKS |
| xterm (app mode) | SS3         | X10            | [COMPLETE] WORKS |
| vim              | SS3         | SGR            | [COMPLETE] WORKS |
| less             | SS3         | SGR            | [COMPLETE] WORKS |
| urxvt            | CSI/SS3     | RXVT           | [COMPLETE] WORKS |
| kitty            | CSI / CSI u | SGR            | [COMPLETE] WORKS |
| alacritty        | CSI / CSI u | SGR            | [COMPLETE] WORKS |
| screen           | SS3         | X10            | [COMPLETE] WORKS |
| tmux             | SS3         | SGR/X10        | [COMPLETE] WORKS |

### Step 8.2.2: Architecture Insight - Mio Poller, MaybeMore & Zero-Latency ESC [COMPLETE]

**The ESC Key & Escape Collision Problem**: How do we distinguish between an isolated ESC key press and the start of an ANSI sequence (`ESC [`, `ESC O`, `ESC ]`) without adding arbitrary timer delays?

**The Solution: Dedicated Mio Poller + `MaybeMore` + Circuit Breaker**:

1. **Dedicated Worker Thread**: `MioPollWorker` monitors `/dev/tty` with `mio::Poll` using non-blocking reads, multiplexing stdin with OS signals (`SIGWINCH`) and software interrupts.
2. **Deterministic Stream Availability (`MaybeMore`)**:
   - Evaluated directly at the I/O read boundary: `MaybeMore::from_read_count(bytes_read, capacity)`.
   - **`KernelDrained`**: If `bytes_read < capacity`, the kernel read queue is empty. A lone `ESC` byte represents an intentional ESC key press and is emitted immediately (0ms delay, no timeout).
   - **`KernelMayHaveMore`**: If `bytes_read == capacity`, more packet fragments may be in flight; incomplete prefixes are held in the framer accumulator.
3. **Resilient Circuit Breaker & Drain State**:
   - If an unrecognized or malformed sequence arrives (e.g., unsupported CSI sequence, runaway OSC transfer), the `CircuitBreaker` transitions to `DrainState` or discards the chunk without freezing the input event loop. Subsequent keystrokes are never blocked.

### Step 8.3: Backend Device Implementation [COMPLETE]

**Status**: [COMPLETE] **Step 8.3 FULLY COMPLETE** - All parsers integrated

**Location**: `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/input_device_impl.rs`

**DirectToAnsiInputDevice Structure**:

```rust
pub struct DirectToAnsiInputDevice {
    channel_receiver: Option<tokio::sync::mpsc::Receiver<InputEvent>>,
    _subscriber_guard: Option<SubscriberGuard>,
}
```

**Main Event Loop & Poller Thread Architecture**: [COMPLETE] COMPLETE

- Asynchronous `next(&mut self) -> Option<InputEvent>` reading from subscriber channel
- Background `MioPollWorker` running dedicated `mio::Poll` on `/dev/tty`
- Chunk framing, circuit breaking, and protocol decoding performed before channel broadcast
- Zero-latency ESC key detection and packet fragmentation reassembly via `MaybeMore`

**Parser Integration**: [COMPLETE] **ALL COMPLETE**

- [COMPLETE] Keyboard parser (CSI + SS3 + CSI u) with unit and roundtrip tests
- [COMPLETE] Mouse parser (SGR + X10 + RXVT) with 51 total tests
- [COMPLETE] Terminal events parser (resize, focus, paste, OSC color & clipboard)
- [COMPLETE] UTF-8 text parser with 13 tests

**Test Status**: [COMPLETE] **All input parser unit, property, and PTY integration tests passing**

- [COMPLETE] Input parser unit tests
- [COMPLETE] PTY integration tests (process isolation)
- [COMPLETE] Input event generator roundtrip tests
- [COMPLETE] DirectToAnsiInputDevice lifecycle and multi-instance tests

### Step 8.4: Testing & Validation [COMPLETE]

**Step 8.4.0: Pre-Testing Fixes + PTY Integration Tests** - [COMPLETE] COMPLETE

**Key Fixes Implemented**:

1. [COMPLETE] Generator bug fix (encode_modifiers correction)
2. [COMPLETE] Terminal event parsing implementation
3. [COMPLETE] DirectToAnsiInputDevice unit tests
4. [COMPLETE] PTY integration tests (4 tests)

**Step 8.4.1: Backend Unit Tests** - [COMPLETE] COMPLETE

- [COMPLETE] Expand backend unit tests
- [COMPLETE] Buffer management & ByteOffset coordinate tests
- [COMPLETE] Parser dispatch coverage
- [COMPLETE] ESC key detection & MaybeMore lookahead tests
- [COMPLETE] Incomplete sequence handling & circuit breaker tests
- [COMPLETE] EOF & error handling tests

### Step 8.5: Migration & Cleanup [COMPLETE]

**Objective**: Integrate DirectToAnsiInputDevice into application event loop

**Status**: [COMPLETE] COMPLETE

**Tasks**:

- [COMPLETE] Update `InputDevice` enum to support `DirectToAnsi(DirectToAnsiInputDevice)`
- [COMPLETE] Add platform-specific backend selection (`#[cfg(target_os = "linux")]`)
- [COMPLETE] Update application event loop to use new input device (`InputDevice::new()`)
- [COMPLETE] Remove crossterm `EventStream` usage in DirectToAnsi paths on Linux
- [COMPLETE] Update documentation and PTY integration tests

### Step 8.6: Resolve TODOs and Stubs [PENDING]

**Objective**: Sweep the codebase for incomplete implementations and TODO markers as final validation before crossterm removal.

**Subtasks**:

- [ ] Search for `TODO:` comments related to DirectToAnsi/RenderOp
- [ ] Search for `FIXME:` comments in input/output paths
- [ ] Search for `unimplemented!()` calls in render pipeline
- [ ] Review all stub functions in DirectToAnsi backend
- [ ] Either implement or remove each stub
- [ ] Verify no lingering crossterm references in DirectToAnsi code paths
- [ ] Run full test suite to ensure no regressions

## Step 9: macOS & Windows Platform Validation & Crossterm Removal [PENDING]

**Status**: [WORK_IN_PROGRESS] PENDING - Linux DirectToAnsi is complete; macOS and Windows native drivers are required to eliminate Crossterm entirely.

**Objective**: Implement native non-Crossterm input and raw-mode drivers for macOS and Windows, validate cross-platform parity, and remove `crossterm` from `Cargo.toml`.

### macOS Drivers & Testing [PENDING]

- [ ] Implement Darwin-compatible tty poller:
  - Darwin's `kqueue(2)` fails with `EINVAL` on `/dev/tty` / PTY file descriptors.
  - Implement a `select(2)` / `poll(2)` polling driver (similar to `filedescriptor`).
- [ ] Implement `SIGWINCH` signal delivery via `signal-hook` with self-pipe trick.
- [ ] Validate DirectToAnsi rendering operations on macOS fleet.
- [ ] Validate DirectToAnsi input handling on macOS fleet.

### Windows Drivers & Testing [PENDING]

- [ ] Implement native Windows Console input driver:
  - Windows lacks `/dev/tty` and POSIX termios.
  - Implement Win32 console reader using `ReadConsoleInputW` / ConPTY virtual terminal input processing (`ENABLE_VIRTUAL_TERMINAL_INPUT`).
- [ ] Implement Windows console raw mode toggling via `GetConsoleMode` / `SetConsoleMode`.
- [ ] Validate DirectToAnsi rendering operations on Windows fleet.
- [ ] Validate DirectToAnsi input handling on Windows fleet.

### Crossterm Dependency Removal [PENDING]

- [ ] Verify zero remaining `crossterm` usages in the codebase.
- [ ] Remove `crossterm` dependency from `tui/Cargo.toml`.
- [ ] Remove `TerminalLibBackend::Crossterm` or retain as optional feature flag.
- [ ] Update documentation and crate architecture diagrams.
- [ ] Final validation across all platforms.

## Implementation Checklist

- [x] Step 0: Prerequisites complete
- [x] Step 1: RenderOp extension complete
- [x] Step 2: DirectAnsi backend module structure
- [x] Step 3: Type system and DirectAnsi implementation
- [x] Step 4: Linux validation complete
- [x] Step 5: Performance optimization complete
- [x] Step 6: Cleanup and refinement complete
- [x] Step 7: Comprehensive test suite complete
- [x] Step 8: InputDevice implementation for Linux complete
  - [x] Step 8.0-8.5: Complete
  - [ ] Step 8.6: Resolve TODOs and Stubs (optional final sweep)
- [ ] Step 9: macOS & Windows native drivers and Crossterm removal

## Critical Success Factors

1. **Architecture Soundness**:
   - [COMPLETE] RenderOp is proven to work for all rendering paths
   - [COMPLETE] DirectAnsi backend matches Crossterm performance (18% faster)
   - [COMPLETE] Input/output symmetry via ANSI protocol
   - [COMPLETE] Decoupled ChunkFramer / ChunkDecoder / CircuitBreaker architecture

2. **Code Quality**:
   - [COMPLETE] Full test coverage for all major components
   - [COMPLETE] Clippy compliance across codebase
   - [COMPLETE] Zero regressions from refactoring

3. **Platform Support**:
   - [COMPLETE] Linux fully validated (RenderOp output + DirectToAnsiInputDevice)
   - [WORK_IN_PROGRESS] macOS native driver pending (kqueue workaround via select)
   - [WORK_IN_PROGRESS] Windows native driver pending (Win32 Console / ConPTY)

4. **Performance**:
   - [COMPLETE] DirectAnsi is 18% faster than Crossterm
   - [COMPLETE] Zero memory leaks
   - [COMPLETE] Zero-latency ESC key disambiguation (0ms delay) via MaybeMore

## Effort Summary - Steps 1-8 Implementation

| Step      | Component                         | Status                         | Time          | Lines |
| --------- | --------------------------------- | ------------------------------ | ------------- | ----- |
| 1         | RenderOp extension                | [COMPLETE] COMPLETE            | 4-5h          | 1242  |
| 2         | DirectAnsi module + ANSI gen      | [COMPLETE] COMPLETE            | 3-4h          | 600   |
| 3         | Type system + backend impl        | [COMPLETE] COMPLETE            | 33-46h        | 1300  |
| 4         | Linux validation                  | [COMPLETE] COMPLETE            | 2-3h          | 0     |
| 5         | Performance optimization          | [COMPLETE] COMPLETE            | 3-4h          | 150   |
| 6         | Cleanup & refinement              | [COMPLETE] COMPLETE            | 1-2h          | 50    |
| 7         | Test suite                        | [COMPLETE] COMPLETE            | 4-6h          | 400   |
| 8         | InputDevice implementation (Linux)| [COMPLETE] COMPLETE            | 8-12h         | 800+  |
| 9         | macOS & Windows native drivers    | [WORK_IN_PROGRESS] PENDING     | 10-15h        | TBD   |
| **TOTAL** | **Steps 1-8**                     | **~60-80 hours**               | **~5500 LOC** |       |

## Conclusion

The unified RenderOp architecture and DirectToAnsi backend are now production-ready on Linux:

- [COMPLETE] Output path fully implemented with DirectAnsi backend (18% performance improvement)
- [COMPLETE] Comprehensive test coverage across all RenderOps
- [COMPLETE] Input protocol parsers complete with Crossterm feature parity, Kitty CSI u, and OSC support
- [COMPLETE] Resilient circuit breaker and zero-latency ESC disambiguation via MaybeMore
- [COMPLETE] Production-ready Linux InputDevice integrated into application event loop
- [WORK_IN_PROGRESS] Final step is implementing native drivers for macOS (select poller) and Windows (ConPTY) to drop Crossterm from `Cargo.toml`.
