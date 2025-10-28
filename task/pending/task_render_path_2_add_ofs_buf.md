<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Task: Add Viewport-Only OffscreenBuffer Mode for Path 2 Architectural Unification](#task-add-viewport-only-offscreenbuffer-mode-for-path-2-architectural-unification)
  - [Overview](#overview)
  - [Table of Contents](#table-of-contents)
  - [Executive Summary](#executive-summary)
    - [The Problem](#the-problem)
    - [The Solution](#the-solution)
    - [Key Benefits](#key-benefits)
    - [Timeline & Complexity](#timeline--complexity)
  - [Current State Analysis](#current-state-analysis)
    - [Path 1: Full-Terminal OffscreenBuffer (Existing)](#path-1-full-terminal-offscreenbuffer-existing)
    - [Path 2: Direct Interactive (Current)](#path-2-direct-interactive-current)
    - [Why Unification Matters](#why-unification-matters)
  - [Technical Design](#technical-design)
    - [3.1 OffscreenBuffer Extension](#31-offscreenbuffer-extension)
    - [3.2 Crossterm Command Adapter](#32-crossterm-command-adapter)
    - [3.3 Architecture Diagrams](#33-architecture-diagrams)
- [Implementation plan](#implementation-plan)
  - [Step 1: Extend OffscreenBuffer [PENDING] Extend OffscreenBuffer (3-4 days)](#step-1-extend-offscreenbuffer-pending-extend-offscreenbuffer-3-4-days)
  - [Step 2: Create Crossterm Command Adapter [PENDING] Create Crossterm Command Adapter (3-4 days)](#step-2-create-crossterm-command-adapter-pending-create-crossterm-command-adapter-3-4-days)
  - [Step 3: Refactor SelectComponent [PENDING] Refactor SelectComponent (3-4 days)](#step-3-refactor-selectcomponent-pending-refactor-selectcomponent-3-4-days)
  - [Step 4: Apply to Other Path 2 Components [PENDING] Apply to Other Path 2 Components (2-3 days)](#step-4-apply-to-other-path-2-components-pending-apply-to-other-path-2-components-2-3-days)
  - [Step 5: Documentation [PENDING] Documentation (1-2 days)](#step-5-documentation-pending-documentation-1-2-days)
  - [Detailed Code Examples](#detailed-code-examples)
    - [Example 1: Creating a Viewport Buffer](#example-1-creating-a-viewport-buffer)
    - [Example 2: Rendering Text to Viewport Buffer](#example-2-rendering-text-to-viewport-buffer)
    - [Example 3: Using the Helper Macro](#example-3-using-the-helper-macro)
    - [Example 4: Complete Component Refactor](#example-4-complete-component-refactor)
  - [Testing Strategy](#testing-strategy)
    - [Unit Tests](#unit-tests)
    - [Integration Tests](#integration-tests)
    - [Regression Tests](#regression-tests)
    - [Component Tests](#component-tests)
  - [Migration Guide](#migration-guide)
    - [For Developers Converting Path 2 Components](#for-developers-converting-path-2-components)
    - [Gotchas & Edge Cases](#gotchas--edge-cases)
  - [Future Possibilities](#future-possibilities)
    - [1. Automatic Scrolling](#1-automatic-scrolling)
    - [2. Terminal Resize Handling](#2-terminal-resize-handling)
    - [3. Optional Diffing for Viewport Mode](#3-optional-diffing-for-viewport-mode)
    - [4. Viewport Composition](#4-viewport-composition)
    - [5. Integration with Path 1](#5-integration-with-path-1)
  - [Open Questions & Decisions](#open-questions--decisions)
    - [1. Mutable Viewport Origin?](#1-mutable-viewport-origin)
    - [2. Viewport Larger Than Terminal?](#2-viewport-larger-than-terminal)
    - [3. Naming Convention](#3-naming-convention)
    - [4. Should Viewport Resize Be Supported?](#4-should-viewport-resize-be-supported)
    - [5. Performance: Full Paint vs Optional Diffing?](#5-performance-full-paint-vs-optional-diffing)
  - [References](#references)
    - [Code Files (Current Implementation)](#code-files-current-implementation)
    - [Related Memory Files](#related-memory-files)
    - [Documentation](#documentation)
    - [External References](#external-references)
  - [Appendix: Related Code Snippets](#appendix-related-code-snippets)
    - [Current OffscreenBuffer Constructor (for reference)](#current-offscreenbuffer-constructor-for-reference)
    - [Current Manual State Management (for reference)](#current-manual-state-management-for-reference)
  - [Version History](#version-history)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Task: Add Viewport-Only OffscreenBuffer Mode for Path 2 Architectural Unification

## Overview

This task unifies two separate architectural paths in the TUI rendering system by extending
`OffscreenBuffer` to support a viewport-only mode. Currently, Path 1 uses a full-terminal
`OffscreenBuffer` to accumulate render operations, then paints to the terminal, while Path 2
performs direct immediate-mode rendering to the terminal. By introducing a viewport-sized
`OffscreenBuffer` option, Path 2 components can use the same abstraction, eliminating code
duplication and enabling consistent rendering behavior across both paths.

**Status:** Specification & Implementation Plan **Estimated Effort:** 3-4 weeks (1-2 weeks core
implementation + testing + integration) **Priority:** Medium-High (technical debt + foundation for
future features)

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Current State Analysis](#current-state-analysis)
3. [Technical Design](#technical-design)
4. [Implementation Phases](#implementation-phases)
5. [Detailed Code Examples](#detailed-code-examples)
6. [Testing Strategy](#testing-strategy)
7. [Migration Guide](#migration-guide)
8. [Future Possibilities](#future-possibilities)
9. [Open Questions & Decisions](#open-questions--decisions)
10. [References](#references)

---

## Executive Summary

### The Problem

The R3BL TUI engine has two rendering paths:

- **Path 1 (Composed)**: Full-screen TUI applications using `RenderOpsIR` → `OffscreenBuffer` →
  diff-based rendering
- **Path 2 (Direct Interactive)**: Simple CLI tools using direct `crossterm` commands and manual
  state management

Path 2 components like `choose()` and `readline_async()` require extensive manual management that
should be automated:

- Manual space allocation (`allocate_viewport_height_space()`)
- Manual cursor position tracking (`move_cursor_back_to_start()`)
- Manual height/width calculations with clamping logic
- No automatic bounds checking
- No clipping by default

This complexity increases maintenance burden and limits features (scrolling, resize handling, etc.).

### The Solution

Extend `OffscreenBuffer` to support **viewport-only mode** alongside the existing full-terminal
mode. This enables:

- **Unified architecture**: Both Path 1 and Path 2 use the same buffer abstraction
- **Automatic state management**: Viewport buffer handles cursor, bounds, clipping automatically
- **Simplified component code**: Remove manual management code, use buffer API
- **Foundation for features**: Scrolling and resize handling become trivial
- **Backward compatible**: Path 1 remains unchanged, new mode is opt-in

### Key Benefits

| Aspect             | Before  | After     |
| ------------------ | ------- | --------- |
| State management   | Manual  | Automatic |
| Code complexity    | High    | Low       |
| Cursor tracking    | Manual  | Automatic |
| Bounds checking    | Manual  | Automatic |
| Feature foundation | Limited | Strong    |
| Type safety        | Medium  | High      |

### Timeline & Complexity

- **Phase 1 (OffscreenBuffer extension)**: ~3-4 days
- **Phase 2 (Crossterm adapter)**: ~3-4 days
- **Phase 3 (SelectComponent refactor)**: ~3-4 days
- **Phase 4 (Other components)**: ~2-3 days
- **Phase 5 (Documentation)**: ~1-2 days
- **Testing & validation**: Ongoing throughout

**Total:** 15-20 days of focused work

---

## Current State Analysis

### Path 1: Full-Terminal OffscreenBuffer (Existing)

**Architecture:**

```
Component
    ↓
RenderOpsIR (type-safe render operations)
    ↓
RenderPipeline (organize by Z-order)
    ↓
OffscreenBuffer (full terminal size: rows × cols)
    ↓
Diff detection (compare with previous frame)
    ↓
RenderOpsOutput (only operations for changed pixels)
    ↓
Terminal
```

**OffscreenBuffer characteristics:**

- Size: Full terminal width × height
- Contains: 2D grid of `PixelChar` (styled characters)
- Features:
  - Automatic cursor position tracking
  - Automatic bounds checking (can't write outside buffer)
  - Full terminal state representation
  - Supports diff-based rendering
  - Memory overhead: Fixed at terminal size

**Code location:** `tui/src/tui/terminal_lib_backends/offscreen_buffer/ofs_buf_core.rs`

### Path 2: Direct Interactive (Current)

**Architecture:**

```
Component (SelectComponent, Readline, etc.)
    ↓
Manual render logic
    ↓
Calculate dimensions
    ↓
Allocate space with println!()
    ↓
Queue crossterm commands
    ↓
Move cursor back manually
    ↓
Terminal
```

**Pain points in `choose()` implementation:**

1. **Manual space allocation** (`function_component.rs:allocate_viewport_height_space`):

```rust
// Must manually allocate lines before rendering
for _ in 0..*viewport_height {
    println!();  // Hacky space allocation
}

// Then move cursor back up
queue_commands! {
    self.get_output_device(),
    MoveToPreviousLine(*viewport_height),
};
```

2. **Manual height calculation** (`select_component.rs`):

```rust
// Must manually calculate header height
fn calculate_header_viewport_height(&self, state: &mut State) -> ChUnit {
    match state.header {
        Header::SingleLine(_) => ch(1),
        Header::MultiLine(ref lines) => ch(lines.len()),
    }
}

// Must manually calculate items height
fn calculate_items_viewport_height(&self, state: &mut State) -> ChUnit {
    if state.items.len() > usize(state.max_display_height) {
        state.max_display_height
    } else {
        ch(state.items.len())
    }
}
```

3. **Clamping logic in `choose_api.rs`** (lines 165-190):

```rust
let max_display_height = ch({
    match maybe_max_height {
        None => DEFAULT_HEIGHT,
        Some(row_height) => {
            let row_height = row_height.as_usize();
            if row_height == 0 {
                DEFAULT_HEIGHT
            } else {
                let num_items = from.len();
                if num_items < row_height {
                    num_items
                } else {
                    row_height
                }
            }
        }
    }
});
```

4. **Manual cursor management** (`select_component.rs:render`):

```rust
self.allocate_viewport_height_space(state)?;
// ... render content ...
render_helper::move_cursor_back_to_start(
    &mut self.output_device,
    render_context.items_viewport_height,
    render_context.header_viewport_height,
)?;
```

**Key issue:** Developers must manually implement state management that OffscreenBuffer provides for
free.

### Why Unification Matters

1. **Reduced cognitive load**: One buffer abstraction instead of two rendering models
2. **Code reuse**: All the buffer logic Path 1 uses becomes available to Path 2
3. **Automatic correctness**: Bounds checking and cursor tracking happen automatically
4. **Future features**: Scrolling, resize handling, composition become possible
5. **Type safety**: Buffer API provides stronger guarantees than imperative commands

---

## Technical Design

### 3.1 OffscreenBuffer Extension

#### Current Structure

```rust
// In tui/src/tui/terminal_lib_backends/offscreen_buffer/ofs_buf_core.rs

#[derive(Clone, PartialEq)]
pub struct OffscreenBuffer {
    pub buffer: PixelCharLines,        // 2D grid of styled characters
    pub window_size: Size,              // Terminal size in rows × cols
    pub cursor_pos: Pos,                // Current cursor position
    pub terminal_mode: TerminalModeState,
    memory_size: MemorySize,
    pub ansi_parser_support: AnsiParserSupport,
}
```

#### Proposed Extension

Add a `BufferMode` enum to support both full-terminal and viewport modes:

```rust
/// Specifies the operational mode of an OffscreenBuffer.
///
/// - `FullTerminal`: Buffer represents entire terminal. Used in Path 1 (composed
///   rendering). Size must match actual terminal dimensions.
/// - `Viewport`: Buffer represents a rectangular region within terminal. Used in
///   Path 2 (direct interactive) to manage partial-screen components. Includes
///   the position where this viewport should be painted on the terminal.
#[derive(Clone, Debug, PartialEq)]
pub enum BufferMode {
    /// Full terminal buffer. Used by Path 1 (composed rendering pipeline).
    FullTerminal { size: Size },

    /// Viewport-only buffer. Used by Path 2 (direct interactive components).
    /// The `origin` specifies where the top-left of this viewport should be
    /// positioned on the terminal screen (row, col).
    Viewport { size: Size, origin: Pos },
}

impl BufferMode {
    pub fn size(&self) -> Size {
        match self {
            BufferMode::FullTerminal { size } | BufferMode::Viewport { size, .. } => *size,
        }
    }

    pub fn is_full_terminal(&self) -> bool {
        matches!(self, BufferMode::FullTerminal { .. })
    }

    pub fn is_viewport(&self) -> bool {
        matches!(self, BufferMode::Viewport { .. })
    }

    pub fn origin(&self) -> Option<Pos> {
        match self {
            BufferMode::Viewport { origin, .. } => Some(*origin),
            BufferMode::FullTerminal { .. } => None,
        }
    }
}
```

#### Modified OffscreenBuffer Struct

```rust
#[derive(Clone, PartialEq)]
pub struct OffscreenBuffer {
    // Changed: now includes mode information
    pub mode: BufferMode,

    pub buffer: PixelCharLines,
    pub window_size: Size,  // Size of buffer (may be viewport size, not terminal size)
    pub cursor_pos: Pos,     // Relative to buffer origin
    pub terminal_mode: TerminalModeState,
    memory_size: MemorySize,
    pub ansi_parser_support: AnsiParserSupport,
}
```

#### New Constructors

````rust
impl OffscreenBuffer {
    /// Create a full-terminal buffer (existing Path 1 behavior).
    pub fn new_full_terminal(size: Size, terminal_mode: TerminalModeState) -> Self {
        let buffer = PixelCharLines::new(size);
        let memory_size = MemorySize::from(&buffer);

        Self {
            mode: BufferMode::FullTerminal { size },
            buffer,
            window_size: size,
            cursor_pos: Pos::ORIGIN,
            terminal_mode,
            memory_size,
            ansi_parser_support: AnsiParserSupport::new(),
        }
    }

    /// Create a viewport-only buffer (new Path 2 feature).
    ///
    /// # Arguments
    /// * `size` - Dimensions of the viewport (rows × cols)
    /// * `origin` - Position on terminal where this viewport should be painted (row, col)
    /// * `terminal_mode` - Terminal mode state
    ///
    /// # Example
    /// ```ignore
    /// // Create a 10×80 viewport positioned at row 5, col 0
    /// let viewport = OffscreenBuffer::new_viewport(
    ///     Size::new(10, 80),
    ///     Pos::new(5, 0),
    ///     TerminalModeState::default(),
    /// );
    /// ```
    pub fn new_viewport(
        size: Size,
        origin: Pos,
        terminal_mode: TerminalModeState,
    ) -> Self {
        let buffer = PixelCharLines::new(size);
        let memory_size = MemorySize::from(&buffer);

        Self {
            mode: BufferMode::Viewport { size, origin },
            buffer,
            window_size: size,
            cursor_pos: Pos::ORIGIN,  // Relative to viewport, not terminal
            terminal_mode,
            memory_size,
            ansi_parser_support: AnsiParserSupport::new(),
        }
    }

    /// Migrate from full-terminal to viewport mode.
    /// Used when a full-terminal buffer needs to be painted as a viewport.
    pub fn set_viewport_mode(&mut self, origin: Pos) {
        self.mode = BufferMode::Viewport {
            size: self.window_size,
            origin,
        };
    }
}
````

#### Viewport Paint Method

```rust
impl OffscreenBuffer {
    /// Paint this viewport buffer to the terminal at its configured origin position.
    ///
    /// This converts all `PixelChar` in the buffer to ANSI escape sequences
    /// and positions them at the correct terminal coordinates.
    ///
    /// # Errors
    /// Returns error if writing to terminal fails.
    pub fn paint_viewport_to_terminal(
        &self,
        output_device: &mut OutputDevice,
    ) -> miette::Result<()> {
        let origin = match self.mode {
            BufferMode::Viewport { origin, .. } => origin,
            BufferMode::FullTerminal { .. } => {
                return Err(miette::miette!(
                    "paint_viewport_to_terminal() called on full-terminal buffer. \
                     Use existing paint methods instead."
                ));
            }
        };

        let mut locked_device = lock_output_device_as_mut!(output_device);

        // Move cursor to viewport origin
        locked_device.queue(cursor::MoveTo(origin.col as u16, origin.row as u16))?;

        // Iterate through buffer lines
        for (buffer_row, line) in self.buffer.iter().enumerate() {
            // Paint each line
            let ansi_bytes = self.render_line_to_ansi(line);
            locked_device.write_all(ansi_bytes)?;

            // Move to next line (except after last line)
            if buffer_row < self.window_size.height.as_usize() - 1 {
                locked_device.queue(cursor::MoveToNextLine(1))?;
            }
        }

        locked_device.flush().into_diagnostic()?;
        Ok(())
    }

    /// Helper: Render a single line to ANSI bytes using PixelCharRenderer.
    fn render_line_to_ansi(&self, line: &PixelCharLine) -> &[u8] {
        let mut renderer = PixelCharRenderer::new();
        renderer.render_line(line.as_slice())
    }
}
```

### 3.2 Crossterm Command Adapter

Create a new module to translate crossterm commands to viewport buffer operations.

**File:** `tui/src/readline_async/viewport_buffer_adapter.rs`

````rust
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{OffscreenBuffer, OutputDevice};
use crossterm::terminal::ClearType;

/// Adapter for translating crossterm commands to ViewportBuffer operations.
///
/// This module provides a bridge between the imperative crossterm command model
/// and the buffer-based rendering model, enabling Path 2 components to benefit
/// from automatic state management.
pub mod adapter {
    use super::*;

    /// Execute a "move cursor to" command on a viewport buffer.
    ///
    /// # Arguments
    /// * `buffer` - The viewport buffer
    /// * `col` - Column position (0-based)
    /// * `row` - Row position (0-based)
    ///
    /// # Returns
    /// Error if position is outside buffer bounds.
    pub fn move_cursor_to(
        buffer: &mut OffscreenBuffer,
        col: u16,
        row: u16,
    ) -> miette::Result<()> {
        let col = col as usize;
        let row = row as usize;

        // Bounds check
        if row >= buffer.window_size.height.as_usize()
            || col >= buffer.window_size.width.as_usize() {
            return Err(miette::miette!(
                "Cursor position ({}, {}) outside viewport bounds ({}x{})",
                row, col,
                buffer.window_size.height.as_usize(),
                buffer.window_size.width.as_usize()
            ));
        }

        buffer.cursor_pos = Pos::new(row, col);
        Ok(())
    }

    /// Execute a "clear current line" command.
    pub fn clear_current_line(buffer: &mut OffscreenBuffer) -> miette::Result<()> {
        let row = buffer.cursor_pos.row;

        if row >= buffer.window_size.height.as_usize() {
            return Err(miette::miette!("Cursor row outside buffer bounds"));
        }

        // Clear the line by filling with spaces
        let width = buffer.window_size.width.as_usize();
        let line = &mut buffer.buffer[row];
        for col in 0..width {
            line[col] = PixelChar::Spacer;
        }

        Ok(())
    }

    /// Execute a "clear all" command on viewport.
    pub fn clear_all(buffer: &mut OffscreenBuffer) -> miette::Result<()> {
        for line in buffer.buffer.iter_mut() {
            for col in 0..buffer.window_size.width.as_usize() {
                line[col] = PixelChar::Spacer;
            }
        }
        Ok(())
    }

    /// Execute a "print text" command with attributes.
    pub fn print_text_with_attributes(
        buffer: &mut OffscreenBuffer,
        text: &str,
        style: Option<TuiStyle>,
    ) -> miette::Result<()> {
        let start_pos = buffer.cursor_pos;

        for (offset, ch) in text.chars().enumerate() {
            let col = start_pos.col + offset;
            let row = start_pos.row;

            // Bounds check
            if row >= buffer.window_size.height.as_usize()
                || col >= buffer.window_size.width.as_usize() {
                // Silently truncate to viewport bounds (typical terminal behavior)
                break;
            }

            let pixel = if let Some(s) = style {
                PixelChar::PlainText { display_char: ch, style: s }
            } else {
                PixelChar::PlainText {
                    display_char: ch,
                    style: TuiStyle::default()
                }
            };

            buffer.buffer[row][col] = pixel;
        }

        // Update cursor to end of printed text
        let new_col = (start_pos.col + text.len()).min(
            buffer.window_size.width.as_usize() - 1
        );
        buffer.cursor_pos = Pos::new(start_pos.row, new_col);

        Ok(())
    }

    /// Execute a "move to next line" command.
    pub fn move_to_next_line(buffer: &mut OffscreenBuffer, count: usize) -> miette::Result<()> {
        let new_row = buffer.cursor_pos.row + count;

        if new_row >= buffer.window_size.height.as_usize() {
            // Clamp to last row
            buffer.cursor_pos.row = buffer.window_size.height.as_usize() - 1;
        } else {
            buffer.cursor_pos.row = new_row;
        }

        buffer.cursor_pos.col = 0;
        Ok(())
    }

    /// Execute a "move to previous line" command.
    pub fn move_to_previous_line(buffer: &mut OffscreenBuffer, count: usize) -> miette::Result<()> {
        if buffer.cursor_pos.row < count {
            buffer.cursor_pos.row = 0;
        } else {
            buffer.cursor_pos.row -= count;
        }

        buffer.cursor_pos.col = 0;
        Ok(())
    }

    /// Execute a "move to column" command.
    pub fn move_to_column(buffer: &mut OffscreenBuffer, col: u16) -> miette::Result<()> {
        let col = col as usize;

        if col >= buffer.window_size.width.as_usize() {
            return Err(miette::miette!(
                "Column {} outside viewport width {}",
                col,
                buffer.window_size.width.as_usize()
            ));
        }

        buffer.cursor_pos.col = col;
        Ok(())
    }
}

/// Helper macro to simplify viewport buffer operations.
///
/// # Example
/// ```ignore
/// viewport_buffer_cmd!(move_cursor_to, &mut buffer, 0, 0)?;
/// viewport_buffer_cmd!(print_text_with_attributes, &mut buffer, "Hello", None)?;
/// viewport_buffer_cmd!(move_to_next_line, &mut buffer, 1)?;
/// ```
#[macro_export]
macro_rules! viewport_buffer_cmd {
    ($cmd:ident, $($arg:expr),* $(,)?) => {{
        $crate::readline_async::viewport_buffer_adapter::adapter::$cmd($($arg),*)
    }};
}
````

### 3.3 Architecture Diagrams

#### Before: Two Separate Paths

```
┌─────────────────────────────────────────────────────────────┐
│ Path 1: Composed Component Pipeline                          │
├─────────────────────────────────────────────────────────────┤
│ Component                                                     │
│     ↓                                                         │
│ RenderOpsIR (structured draw commands)                       │
│     ↓                                                         │
│ OffscreenBuffer (full terminal size)                         │
│     ↓                                                         │
│ Diff detection → RenderOpsOutput                             │
│     ↓                                                         │
│ Terminal                                                      │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ Path 2: Direct Interactive (Manual Management)               │
├─────────────────────────────────────────────────────────────┤
│ SelectComponent / Readline                                   │
│     ↓                                                         │
│ Manual render() logic                                        │
│     ↓                                                         │
│ Calculate height/width manually                              │
│     ↓                                                         │
│ Allocate space with println!()                               │
│     ↓                                                         │
│ Queue crossterm commands (MoveTo, Print, Clear, etc)        │
│     ↓                                                         │
│ Manually move cursor back                                    │
│     ↓                                                         │
│ Terminal                                                      │
└─────────────────────────────────────────────────────────────┘
```

#### After: Unified via Viewport Buffer

```
┌──────────────────────────────────────────────────────────────────┐
│ Path 1: Composed Component Pipeline                              │
├──────────────────────────────────────────────────────────────────┤
│ Component                                                         │
│     ↓                                                             │
│ RenderOpsIR                                                       │
│     ↓                                                             │
│ OffscreenBuffer (FullTerminal mode) ◄─── OffscreenBuffer API    │
│     ↓                                      (Unified abstraction)  │
│ RenderOpsOutput → Terminal                                       │
└──────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────┐
│ Path 2: Direct Interactive (Using Viewport Buffer)               │
├──────────────────────────────────────────────────────────────────┤
│ SelectComponent / Readline                                       │
│     ↓                                                             │
│ Simplified render() logic                                        │
│     ↓                                                             │
│ OffscreenBuffer (Viewport mode) ◄──────┐ (Automatic bounds,    │
│     ↓                                    │  cursor, clipping)    │
│ paint_viewport_to_terminal()              │                      │
│     ↓                                    │                       │
│ Terminal ◄──────────────────────────────┘                        │
└──────────────────────────────────────────────────────────────────┘
```

---

# Implementation plan

## Step 1: Extend OffscreenBuffer [PENDING] Extend OffscreenBuffer (3-4 days)

**Objective:** Add viewport mode support to OffscreenBuffer without breaking Path 1.

#### Tasks

1. **Create BufferMode enum**
   - File: `tui/src/tui/terminal_lib_backends/offscreen_buffer/buffer_mode.rs` (new)
   - Implement `FullTerminal` and `Viewport` variants
   - Add helper methods: `size()`, `is_viewport()`, `origin()`

2. **Modify OffscreenBuffer struct**
   - File: `tui/src/tui/terminal_lib_backends/offscreen_buffer/ofs_buf_core.rs`
   - Add `mode: BufferMode` field
   - Update existing constructor to use `FullTerminal` mode
   - Ensure backward compatibility

3. **Implement new constructors**
   - `new_full_terminal()` - Path 1 behavior (alias for `new()`)
   - `new_viewport()` - Path 2 behavior
   - Document when to use each

4. **Implement viewport paint method**
   - `paint_viewport_to_terminal()` - converts viewport buffer to ANSI and positions on terminal
   - Uses `PixelCharRenderer` for ANSI generation
   - Handles cursor positioning

5. **Update module exports**
   - File: `tui/src/tui/terminal_lib_backends/offscreen_buffer/mod.rs`
   - Export `BufferMode`
   - Update documentation

6. **Write unit tests**
   - Buffer creation in both modes
   - Viewport paint logic
   - Bounds checking
   - Backward compatibility with Path 1

#### Acceptance Criteria

- [ ] `BufferMode` enum compiles and is well-documented
- [ ] `new_viewport()` creates properly-sized viewport buffers
- [ ] `new_full_terminal()` maintains existing behavior
- [ ] `paint_viewport_to_terminal()` correctly positions buffer at origin
- [ ] All Path 1 tests still pass (no regression)
- [ ] New unit tests provide >80% coverage of new code

## Step 2: Create Crossterm Command Adapter [PENDING] Create Crossterm Command Adapter (3-4 days)

**Objective:** Bridge imperative crossterm commands to viewport buffer operations.

#### Tasks

1. **Create adapter module**
   - File: `tui/src/readline_async/viewport_buffer_adapter.rs` (new)
   - Implement adapter functions:
     - `move_cursor_to()`
     - `clear_current_line()`
     - `clear_all()`
     - `print_text_with_attributes()`
     - `move_to_next_line()`
     - `move_to_previous_line()`
     - `move_to_column()`

2. **Implement bounds checking**
   - All operations validate positions against buffer bounds
   - Graceful handling of out-of-bounds (clamp or error as appropriate)
   - Clear error messages

3. **Create helper macro**
   - `viewport_buffer_cmd!()` - simplify calling adapter functions
   - Document with examples

4. **Add integration tests**
   - Test each adapter function with valid inputs
   - Test error cases (out of bounds, invalid positions)
   - Test interaction between operations (e.g., move then print)

#### Acceptance Criteria

- [ ] All crossterm command mappings implemented
- [ ] Bounds checking works correctly
- [ ] Helper macro simplifies usage
- [ ] Integration tests cover all operations
- [ ] Error messages are clear and helpful

## Step 3: Refactor SelectComponent [PENDING] Refactor SelectComponent (3-4 days)

**Objective:** Remove manual state management from `choose()` by using viewport buffer.

#### Tasks

1. **Refactor SelectComponent.render()**
   - File: `tui/src/readline_async/choose_impl/select_component.rs`
   - Create viewport buffer sized for content
   - Use adapter functions instead of direct `queue_commands!`
   - Call `paint_viewport_to_terminal()` to output
   - Remove: `allocate_viewport_height_space()`, `move_cursor_back_to_start()`

2. **Simplify state struct**
   - Remove fields that buffer manages (cursor position, etc.)
   - Keep only application state

3. **Update choose() API**
   - Evaluate: should `maybe_max_height` parameter remain?
   - If kept: use as buffer size constraint
   - If removed: let buffer size based on content naturally
   - Document decision

4. **Update helper functions**
   - File: `tui/src/readline_async/choose_impl/function_component.rs`
   - Evaluate which helpers are still needed
   - Refactor or remove as appropriate

5. **Refactor render helper module**
   - `tui/src/readline_async/choose_impl/select_component.rs` (mod render_helper)
   - Simplify based on buffer taking over state management
   - Focus on what to render, not how to manage viewport

6. **Comprehensive testing**
   - Existing component tests should still pass
   - New tests for viewport buffer integration
   - Behavior regression testing (choose still works same way)

#### Code Example: Before & After

**Before (current):**

```rust
fn render(&mut self, state: &mut State) -> CommonResult<()> {
    let render_context = render_helper::RenderContext::new(self, state);

    // Manual space allocation
    self.allocate_viewport_height_space(state)?;

    // Manual header rendering with positioning
    render_helper::render_header(...)?;

    // Manual items rendering with cursor tracking
    render_helper::render_items(...)?;

    // Manual cursor restoration
    render_helper::move_cursor_back_to_start(
        &mut self.output_device,
        render_context.items_viewport_height,
        render_context.header_viewport_height,
    )?;

    lock_output_device_as_mut!(self.output_device).flush()?;
    Ok(())
}

// ~40 lines of helper functions for space allocation and cursor management
```

**After (with viewport buffer):**

```rust
fn render(&mut self, state: &mut State) -> CommonResult<()> {
    // Calculate viewport size based on content
    let viewport_height = calculate_viewport_height(state);
    let viewport_width = get_terminal_width();
    let viewport_size = Size::new(viewport_height, viewport_width);

    // Create buffer - everything else is automatic
    let mut buffer = OffscreenBuffer::new_viewport(
        viewport_size,
        Pos::ORIGIN,  // Or wherever cursor currently is
        TerminalModeState::default(),
    );

    // Render to buffer (no cursor management, no space allocation)
    render_to_buffer(&mut buffer, state, &self.style)?;

    // Paint to terminal (single call, all positioning handled)
    buffer.paint_viewport_to_terminal(&mut self.output_device)?;

    Ok(())
}

fn render_to_buffer(
    buffer: &mut OffscreenBuffer,
    state: &State,
    style: &StyleSheet,
) -> miette::Result<()> {
    // Render header
    let header_text = format_header(&state.header);
    viewport_buffer_cmd!(
        print_text_with_attributes,
        buffer,
        &header_text,
        Some(style.header_style)
    )?;
    viewport_buffer_cmd!(move_to_next_line, buffer, 1)?;

    // Render items
    for item in &state.items {
        let item_text = format_item(item);
        viewport_buffer_cmd!(
            print_text_with_attributes,
            buffer,
            &item_text,
            Some(style.item_style)
        )?;
        viewport_buffer_cmd!(move_to_next_line, buffer, 1)?;
    }

    Ok(())
}

// ~15 lines, much simpler and clearer intent
```

#### Acceptance Criteria

- [ ] SelectComponent compiles and tests pass
- [ ] `choose()` behaves identically to before (regression test)
- [ ] Manual state management code removed
- [ ] Code is cleaner and easier to understand
- [ ] No performance regression
- [ ] Memory usage acceptable

## Step 4: Apply to Other Path 2 Components [PENDING] Apply to Other Path 2 Components (2-3 days)

**Objective:** Extend viewport buffer pattern to other interactive components.

#### Affected Components

1. **Readline components**
   - `tui/src/readline_async/readline_async_impl/line_state.rs`
   - `tui/src/readline_async/readline_async_api.rs`

2. **Any form/input components** (if they exist)

#### Tasks

1. **Identify all Path 2 components**
   - Grep for direct crossterm usage
   - Map cursor management patterns
   - Document findings

2. **Refactor each component**
   - Apply same pattern as SelectComponent
   - Remove manual state management
   - Use viewport buffer API

3. **Ensure consistency**
   - All components use same buffer construction pattern
   - Same adapter function usage
   - Same error handling

#### Acceptance Criteria

- [ ] All Path 2 components identified and documented
- [ ] Each refactored to use viewport buffer
- [ ] Component tests pass for all
- [ ] Consistent patterns across codebase

## Step 5: Documentation [PENDING] Documentation (1-2 days)

**Objective:** Update documentation to reflect architectural unification.

#### Tasks

1. **Update `tui/src/lib.rs`**
   - Section: "Dual Rendering Paths"
   - Update to show both paths now use buffers
   - Explain viewport mode
   - Show architecture diagrams

2. **Add code examples**
   - How to create viewport buffer
   - How to render to it
   - How to paint to terminal

3. **Create migration guide**
   - For developers converting Path 2 components
   - Common patterns and idioms
   - Pitfalls to avoid

4. **Document API**
   - Viewport mode in OffscreenBuffer
   - Adapter functions
   - Helper macro

#### Acceptance Criteria

- [ ] Documentation updated and reviewed
- [ ] Examples compile and are correct
- [ ] Migration guide is clear and complete

---

## Detailed Code Examples

### Example 1: Creating a Viewport Buffer

```rust
use r3bl_tui::{OffscreenBuffer, BufferMode, Size, Pos, TerminalModeState};

// Create a 10×80 viewport at terminal position (row=5, col=0)
let size = Size::new(RowHeight::new(10), ColWidth::new(80));
let origin = Pos::new(5, 0);
let mode = TerminalModeState::default();

let mut buffer = OffscreenBuffer::new_viewport(size, origin, mode);

// Buffer is now ready to render to!
```

### Example 2: Rendering Text to Viewport Buffer

```rust
use r3bl_tui::{OffscreenBuffer, TuiStyle, fg_blue};
use r3bl_tui::readline_async::viewport_buffer_adapter::adapter;

fn render_menu(buffer: &mut OffscreenBuffer, items: &[&str]) -> miette::Result<()> {
    let header_style = TuiStyle::new().fg(fg_blue()).bold();

    // Print header
    adapter::print_text_with_attributes(
        buffer,
        "Select an option:",
        Some(header_style),
    )?;

    // Move to next line
    adapter::move_to_next_line(buffer, 1)?;

    // Print items
    for (idx, item) in items.iter().enumerate() {
        let prefix = if idx == 0 { " › " } else { "   " };
        let text = format!("{}{}", prefix, item);
        adapter::print_text_with_attributes(buffer, &text, None)?;
        adapter::move_to_next_line(buffer, 1)?;
    }

    Ok(())
}
```

### Example 3: Using the Helper Macro

```rust
use r3bl_tui::{OffscreenBuffer, TuiStyle};

fn render_with_macro(buffer: &mut OffscreenBuffer) -> miette::Result<()> {
    let style = TuiStyle::new().bold();

    viewport_buffer_cmd!(
        print_text_with_attributes,
        buffer,
        "Hello, World!",
        Some(style)
    )?;

    viewport_buffer_cmd!(move_to_next_line, buffer, 1)?;

    viewport_buffer_cmd!(move_to_column, buffer, 0)?;

    viewport_buffer_cmd!(
        print_text_with_attributes,
        buffer,
        "Press 'q' to quit.",
        None
    )?;

    Ok(())
}
```

### Example 4: Complete Component Refactor

**Before (Manual State Management):**

```rust
pub struct SelectComponent {
    pub output_device: OutputDevice,
    pub style: StyleSheet,
}

impl SelectComponent {
    fn render(&mut self, state: &mut State) -> CommonResult<()> {
        // Manual calculations
        let header_height = self.calculate_header_viewport_height(state);
        let items_height = self.calculate_items_viewport_height(state);
        let total_height = header_height + items_height;

        // Manual space allocation
        self.allocate_viewport_height_space(state)?;

        // Manual rendering
        self.render_header(state, &mut self.output_device)?;
        self.render_items(state, &mut self.output_device)?;

        // Manual cursor restoration
        self.move_cursor_back_to_start(total_height)?;

        Ok(())
    }

    fn allocate_viewport_height_space(&mut self, state: &State) -> CommonResult<()> {
        // Complex manual space allocation...
        for _ in 0..total_height {
            println!();
        }
        queue_commands! {
            self.output_device,
            MoveToPreviousLine(total_height),
        };
        Ok(())
    }

    fn move_cursor_back_to_start(&mut self, height: usize) -> CommonResult<()> {
        queue_commands! {
            self.output_device,
            MoveToPreviousLine(height),
        };
        Ok(())
    }

    // ... more manual management code ...
}
```

**After (Using Viewport Buffer):**

```rust
pub struct SelectComponent {
    pub output_device: OutputDevice,
    pub style: StyleSheet,
}

impl SelectComponent {
    fn render(&mut self, state: &mut State) -> CommonResult<()> {
        // Determine content size
        let height = calculate_content_height(state);
        let width = get_terminal_width();

        // Create buffer - everything else is automatic
        let size = Size::new(height, width);
        let mut buffer = OffscreenBuffer::new_viewport(
            size,
            Pos::ORIGIN,
            TerminalModeState::default(),
        );

        // Render to buffer
        self.render_content(&mut buffer, state)?;

        // Paint to terminal (one line!)
        buffer.paint_viewport_to_terminal(&mut self.output_device)?;

        Ok(())
    }

    fn render_content(
        &self,
        buffer: &mut OffscreenBuffer,
        state: &State,
    ) -> miette::Result<()> {
        // Simple, focused rendering logic
        viewport_buffer_cmd!(
            print_text_with_attributes,
            buffer,
            &format_header(&state.header),
            Some(self.style.header_style),
        )?;

        viewport_buffer_cmd!(move_to_next_line, buffer, 1)?;

        for item in &state.items {
            viewport_buffer_cmd!(
                print_text_with_attributes,
                buffer,
                &format_item(item),
                Some(self.style.item_style),
            )?;
            viewport_buffer_cmd!(move_to_next_line, buffer, 1)?;
        }

        Ok(())
    }
}
```

**Comparison:**

- **Before:** ~150 lines with manual state management
- **After:** ~80 lines, clear intent, no manual bookkeeping

---

## Testing Strategy

### Unit Tests

Create tests in `tui/src/tui/terminal_lib_backends/offscreen_buffer/tests/viewport_mode.rs`:

```rust
#[test]
fn test_viewport_buffer_creation() {
    let size = Size::new(10, 80);
    let origin = Pos::new(5, 0);
    let buffer = OffscreenBuffer::new_viewport(size, origin, TerminalModeState::default());

    assert!(buffer.mode.is_viewport());
    assert_eq!(buffer.window_size, size);
    assert_eq!(buffer.cursor_pos, Pos::ORIGIN);
}

#[test]
fn test_viewport_bounds_checking() {
    let size = Size::new(5, 20);
    let buffer = OffscreenBuffer::new_viewport(size, Pos::ORIGIN, TerminalModeState::default());

    // Should accept valid position
    adapter::move_cursor_to(&mut buffer, 0, 0).unwrap();

    // Should reject out-of-bounds
    assert!(adapter::move_cursor_to(&mut buffer, 0, 10).is_err());
    assert!(adapter::move_cursor_to(&mut buffer, 25, 0).is_err());
}

#[test]
fn test_print_and_cursor_advance() {
    let mut buffer = OffscreenBuffer::new_viewport(
        Size::new(5, 20),
        Pos::ORIGIN,
        TerminalModeState::default(),
    );

    adapter::print_text_with_attributes(&mut buffer, "Hello", None).unwrap();

    // Cursor should have advanced
    assert_eq!(buffer.cursor_pos.col, 5);
    assert_eq!(buffer.cursor_pos.row, 0);
}

#[test]
fn test_move_to_next_line_resets_column() {
    let mut buffer = OffscreenBuffer::new_viewport(
        Size::new(5, 20),
        Pos::ORIGIN,
        TerminalModeState::default(),
    );

    buffer.cursor_pos = Pos::new(0, 10);
    adapter::move_to_next_line(&mut buffer, 1).unwrap();

    assert_eq!(buffer.cursor_pos.row, 1);
    assert_eq!(buffer.cursor_pos.col, 0);  // Column reset
}

#[test]
fn test_clear_current_line() {
    let mut buffer = OffscreenBuffer::new_viewport(
        Size::new(5, 20),
        Pos::ORIGIN,
        TerminalModeState::default(),
    );

    // Write something
    adapter::print_text_with_attributes(&mut buffer, "Test", None).unwrap();

    // Clear it
    buffer.cursor_pos.row = 0;
    buffer.cursor_pos.col = 0;
    adapter::clear_current_line(&mut buffer).unwrap();

    // Should be spaces now
    for col in 0..20 {
        assert!(matches!(buffer.buffer[0][col], PixelChar::Spacer));
    }
}
```

### Integration Tests

Create tests in `tui/src/readline_async/tests/viewport_integration.rs`:

```rust
#[test]
fn test_select_component_with_viewport_buffer() {
    let mut state = State::default();
    state.items = vec![
        "Option 1".into(),
        "Option 2".into(),
        "Option 3".into(),
    ];

    let size = Size::new(5, 40);
    let mut buffer = OffscreenBuffer::new_viewport(
        size,
        Pos::ORIGIN,
        TerminalModeState::default(),
    );

    let component = SelectComponent::default();
    component.render_content(&mut buffer, &state).unwrap();

    // Verify buffer contains rendered content
    assert!(!buffer.buffer[0].is_empty());
    // ... more assertions ...
}

#[test]
fn test_buffer_respects_viewport_bounds() {
    let mut buffer = OffscreenBuffer::new_viewport(
        Size::new(3, 10),
        Pos::ORIGIN,
        TerminalModeState::default(),
    );

    // Try to render more content than buffer can hold
    for i in 0..5 {
        let _ = adapter::print_text_with_attributes(
            &mut buffer,
            &format!("Line {}", i),
            None,
        );
        if i < 4 {
            let _ = adapter::move_to_next_line(&mut buffer, 1);
        }
    }

    // Should not panic, should gracefully handle bounds
}
```

### Regression Tests

Ensure Path 1 still works:

```rust
#[test]
fn test_full_terminal_mode_unchanged() {
    let size = Size::new(24, 80);
    let buffer = OffscreenBuffer::new_full_terminal(size, TerminalModeState::default());

    assert!(buffer.mode.is_full_terminal());
    assert_eq!(buffer.window_size, size);

    // All Path 1 operations should still work
    // ... run existing Path 1 tests ...
}
```

### Component Tests

Verify SelectComponent behavior unchanged:

```rust
#[tokio::test]
async fn test_choose_still_works() {
    // Run existing choose() tests to ensure no regression
    // The UI interaction should be identical to before
}
```

---

## Migration Guide

### For Developers Converting Path 2 Components

#### Step 1: Replace Manual Rendering with Viewport Buffer

```rust
// Old
fn render(&mut self, state: &mut State) -> CommonResult<()> {
    self.allocate_viewport_height_space(state)?;
    render_header(&mut self.output_device, ...)?;
    render_items(&mut self.output_device, ...)?;
    move_cursor_back_to_start(&mut self.output_device, ...)?;
    Ok(())
}

// New
fn render(&mut self, state: &mut State) -> CommonResult<()> {
    let size = Size::new(
        calculate_content_height(state),
        get_terminal_width(),
    );
    let mut buffer = OffscreenBuffer::new_viewport(
        size,
        Pos::ORIGIN,
        TerminalModeState::default(),
    );

    self.render_to_buffer(&mut buffer, state)?;
    buffer.paint_viewport_to_terminal(&mut self.output_device)?;
    Ok(())
}
```

#### Step 2: Replace Queue Commands with Adapter Functions

```rust
// Old
queue_commands! {
    self.output_device,
    MoveToColumn(0),
    Print("Hello"),
    MoveToNextLine(1),
};

// New
viewport_buffer_cmd!(move_to_column, buffer, 0)?;
viewport_buffer_cmd!(
    print_text_with_attributes,
    buffer,
    "Hello",
    None,
)?;
viewport_buffer_cmd!(move_to_next_line, buffer, 1)?;
```

#### Step 3: Remove State Management Code

Delete these methods:

- `allocate_viewport_height_space()`
- `move_cursor_back_to_start()`
- `calculate_*_height()` functions
- Manual cursor position tracking

#### Common Patterns

**Pattern: Rendering multiple lines**

```rust
for item in items {
    viewport_buffer_cmd!(
        print_text_with_attributes,
        buffer,
        &format_item(item),
        Some(style),
    )?;
    viewport_buffer_cmd!(move_to_next_line, buffer, 1)?;
}
```

**Pattern: Clearing and redrawing**

```rust
viewport_buffer_cmd!(clear_all, buffer)?;
buffer.cursor_pos = Pos::ORIGIN;
// Render fresh content
```

**Pattern: Styled text**

```rust
let style = TuiStyle::new()
    .fg(fg_blue())
    .bold();

viewport_buffer_cmd!(
    print_text_with_attributes,
    buffer,
    "Styled Text",
    Some(style),
)?;
```

### Gotchas & Edge Cases

1. **Viewport sizing**
   - Always ensure viewport height ≥ content height
   - Account for padding/margins
   - Consider terminal resize (may need dynamic sizing)

2. **Cursor management**
   - Cursor position is relative to viewport, not terminal
   - `paint_viewport_to_terminal()` handles absolute positioning
   - Don't try to use absolute terminal coordinates

3. **Bounds checking**
   - Text longer than width silently truncates
   - Printing beyond height moves cursor to last line
   - Out-of-bounds operations return errors where appropriate

4. **Performance**
   - Viewport buffer allocates `PixelChar[][]` - acceptable for typical sizes
   - Full paint on every render (no diffing in viewport mode)
   - If performance issues arise, consider: (a) smaller viewport, (b) optional diffing

---

## Future Possibilities

### 1. Automatic Scrolling

Enable content larger than viewport to be scrolled:

```rust
// Pseudo-code
let mut buffer = OffscreenBuffer::new_viewport_with_scrolling(
    viewport_size,
    content_size,  // Can be larger than viewport
    origin,
);

// Automatically handles off-screen content
buffer.scroll(direction, amount)?;
buffer.paint_viewport_to_terminal(&mut output)?;  // Shows viewport window
```

### 2. Terminal Resize Handling

Automatically adapt to terminal resize events:

```rust
impl SelectComponent {
    async fn render_with_resize_handling(
        &mut self,
        state: &mut State,
    ) -> CommonResult<()> {
        loop {
            let terminal_size = get_current_terminal_size();

            // Buffer automatically resizes
            let mut buffer = OffscreenBuffer::new_viewport(
                Size::new(terminal_size.height, terminal_size.width),
                Pos::ORIGIN,
                TerminalModeState::default(),
            );

            self.render_to_buffer(&mut buffer, state)?;
            buffer.paint_viewport_to_terminal(&mut self.output_device)?;

            // Wait for event or resize
            select! {
                event = self.input_device.next() => {
                    match event {
                        Event::Resize => continue,  // Re-render
                        _ => break,
                    }
                }
            }
        }
        Ok(())
    }
}
```

### 3. Optional Diffing for Viewport Mode

For high-frequency redraws, enable diff-based optimization:

```rust
pub struct ViewportBufferWithDiff {
    current: OffscreenBuffer,
    previous: OffscreenBuffer,  // Track previous frame
}

impl ViewportBufferWithDiff {
    fn paint_diff_to_terminal(&mut self, output: &mut OutputDevice) -> Result<()> {
        let diff = self.compute_diff();
        // Only output changed pixels
        for change in diff {
            output.write_ansi_for_pixel(&change)?;
        }
        self.previous = self.current.clone();
        Ok(())
    }
}
```

### 4. Viewport Composition

Layer multiple viewports (headers, footers, content areas):

```rust
let mut header_buffer = OffscreenBuffer::new_viewport(
    Size::new(1, 80),
    Pos::new(0, 0),
    TerminalModeState::default(),
);

let mut content_buffer = OffscreenBuffer::new_viewport(
    Size::new(22, 80),
    Pos::new(1, 0),
    TerminalModeState::default(),
);

let mut footer_buffer = OffscreenBuffer::new_viewport(
    Size::new(1, 80),
    Pos::new(23, 0),
    TerminalModeState::default(),
);

// Render each independently, paint together
header_buffer.paint_viewport_to_terminal(&mut output)?;
content_buffer.paint_viewport_to_terminal(&mut output)?;
footer_buffer.paint_viewport_to_terminal(&mut output)?;
```

### 5. Integration with Path 1

Hybrid components that mix Path 1 (composed) and Path 2 (direct):

```rust
// Path 1: Use RenderOpsIR for complex layout
let mut main_render_ops = RenderPipeline::new();
main_render_ops.add(render_main_panel());
main_render_ops.add(render_sidebar());

// Path 2: Use viewport buffer for real-time input overlay
let mut overlay_buffer = OffscreenBuffer::new_viewport(...);
overlay_buffer.render_input_field(...)?;

// Compose both for final output
let main_buffer = execute_render_ops(main_render_ops)?;
overlay_buffer.paint_viewport_to_terminal(&mut output)?;
```

---

## Open Questions & Decisions

### 1. Mutable Viewport Origin?

**Question:** Should the viewport origin be mutable after buffer creation?

**Option A:** Immutable

- Pro: Simpler mental model, clear intent
- Con: Can't reposition viewport after creation

**Option B:** Mutable

- Pro: Flexible for dynamic positioning
- Con: More complex, potential for bugs

**Recommendation:** Start with immutable. Can add mutable method later if needed.

```rust
// Immutable (recommendation)
pub fn new_viewport(size: Size, origin: Pos, ...) -> Self { ... }

// Future: If needed, add
pub fn set_origin(&mut self, origin: Pos) { ... }
```

### 2. Viewport Larger Than Terminal?

**Question:** What happens if viewport size > terminal size?

**Option A:** Error on creation

- Pro: Fail fast, clear
- Con: Some use cases might need to handle dynamically

**Option B:** Allow silently, clamp on paint

- Pro: Flexible
- Con: Silent errors are bad

**Option C:** Allow, but require explicit confirmation

- Pro: Flexible + intentional
- Con: More API complexity

**Recommendation:** Option B for now. Add bounds checking with optional strict mode.

```rust
pub fn new_viewport(size: Size, origin: Pos, ...) -> miette::Result<Self> {
    // Warn if larger than terminal, but don't error
    if size.height > get_terminal_height() {
        eprintln!("Warning: viewport height exceeds terminal height");
    }
    Ok(Self { ... })
}
```

### 3. Naming Convention

**Question:** Should we use `new_viewport()` or `with_viewport_mode()`?

**Recommendation:** Use `new_viewport()` - clearer and more direct.

```rust
// Recommended
let buffer = OffscreenBuffer::new_viewport(size, origin, mode);

// Also provided for clarity
let buffer = OffscreenBuffer::new_full_terminal(size, mode);
```

### 4. Should Viewport Resize Be Supported?

**Question:** Should buffer be resizable after creation?

**Recommendation:** Not in Phase 1. Can add in Phase 2 if needed.

```rust
// Future method (Phase 2+)
pub fn resize_viewport(&mut self, new_size: Size) -> miette::Result<()> {
    self.buffer = PixelCharLines::new(new_size);
    self.window_size = new_size;
    Ok(())
}
```

### 5. Performance: Full Paint vs Optional Diffing?

**Question:** Should viewport mode support optional diffing like full-terminal mode?

**Phase 1 Recommendation:** No - keep simple. Full paint every time. **Phase 2+:** Consider if
performance testing shows need.

---

## References

### Code Files (Current Implementation)

- **OffscreenBuffer definition:**
  `tui/src/tui/terminal_lib_backends/offscreen_buffer/ofs_buf_core.rs`
- **SelectComponent (to refactor):** `tui/src/readline_async/choose_impl/select_component.rs`
- **Function component trait:** `tui/src/readline_async/choose_impl/function_component.rs`
- **Choose API:** `tui/src/readline_async/choose_api.rs`
- **PixelCharRenderer:** `tui/src/tui/terminal_lib_backends/direct_ansi/pixel_char_renderer.rs`
- **CLI text inline:** `tui/src/core/ansi/cli_text.rs`

### Related Memory Files

From previous architectural analysis:

- `claude_memories/rendering_path_architecture.md` - Overview of Path 1 vs Path 2
- `claude_memories/cli_text_inline_pixel_char_conversion.md` - How CliTextInline converts to
  PixelChar
- `claude_memories/pixel_char_renderer_ansi_generation.md` - PixelCharRenderer ANSI byte generation

### Documentation

- **Main lib docs:** `tui/src/lib.rs` section "Dual Rendering Paths"
- **CLAUDE.md project guidelines:** `/home/nazmul/github/r3bl-open-core/CLAUDE.md`
- **Bounds checking utilities:** `tui/src/core/units/bounds_check/mod.rs`

### External References

- **Crossterm docs:** https://docs.rs/crossterm/
- **VT100 ANSI spec:** https://vt100.net/
- **Unicode grapheme handling:** Unicode Segmentation crate

---

## Appendix: Related Code Snippets

### Current OffscreenBuffer Constructor (for reference)

```rust
impl OffscreenBuffer {
    pub fn new(window_size: Size, terminal_mode: TerminalModeState) -> Self {
        let buffer = PixelCharLines::new(window_size);
        let memory_size = MemorySize::from(&buffer);

        Self {
            buffer,
            window_size,
            cursor_pos: Pos::ORIGIN,
            terminal_mode,
            memory_size,
            ansi_parser_support: AnsiParserSupport::new(),
        }
    }
}
```

### Current Manual State Management (for reference)

```rust
fn allocate_viewport_height_space(&mut self, state: &mut S) -> miette::Result<()> {
    throws!({
        let viewport_height =
            self.calculate_items_viewport_height(state) +
            self.calculate_header_viewport_height(state);

        for _ in 0..*viewport_height {
            println!();
        }

        queue_commands! {
            self.get_output_device(),
            MoveToPreviousLine(*viewport_height),
        };
    });
    Ok(())
}
```

---

## Version History

| Date       | Version | Author                   | Changes                                       |
| ---------- | ------- | ------------------------ | --------------------------------------------- |
| 2025-01-25 | 1.0     | Nazmul (via Claude Code) | Initial specification and implementation plan |

---

**Document Status:** Ready for developer handoff **Last Updated:** 2025-01-25 **Next Review:** After
Phase 1 implementation
