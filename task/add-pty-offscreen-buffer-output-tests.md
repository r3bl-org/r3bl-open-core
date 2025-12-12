# Add Rendered Output Integration Tests (StdoutMock → OffscreenBuffer)

<!-- cspell:words GHIJ -->

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Overview](#overview)
  - [Background](#background)
  - [Architecture Decision: StdoutMock → OffscreenBuffer (Not Real PTY)](#architecture-decision-stdoutmock-%E2%86%92-offscreenbuffer-not-real-pty)
  - [Why Both Test Layers](#why-both-test-layers)
  - [Module Organization](#module-organization)
- [Implementation Plan](#implementation-plan)
  - [Step 0: Create Test Infrastructure](#step-0-create-test-infrastructure)
    - [Step 0.0: Create Rendered Output Test Helper Module](#step-00-create-rendered-output-test-helper-module)
    - [Step 0.1: Add OffscreenBuffer Assertion Helpers](#step-01-add-offscreenbuffer-assertion-helpers)
  - [Step 1: High-Value Tests - Cursor Movement [HIGH PRIORITY]](#step-1-high-value-tests---cursor-movement-high-priority)
    - [Step 1.0: Create `cursor_movement_rendered.rs`](#step-10-create-cursor_movement_renderedrs)
    - [Step 1.1: `test_move_cursor_absolute_origin_rendered`](#step-11-test_move_cursor_absolute_origin_rendered)
    - [Step 1.2: `test_move_cursor_absolute_5_10_rendered`](#step-12-test_move_cursor_absolute_5_10_rendered)
    - [Step 1.3: `test_move_cursor_relative_to_rendered`](#step-13-test_move_cursor_relative_to_rendered)
    - [Step 1.4: `test_move_cursor_to_next_line_rendered`](#step-14-test_move_cursor_to_next_line_rendered)
    - [Step 1.5: `test_move_cursor_to_previous_line_rendered`](#step-15-test_move_cursor_to_previous_line_rendered)
  - [Step 2: High-Value Tests - Text Operations [HIGH PRIORITY]](#step-2-high-value-tests---text-operations-high-priority)
    - [Step 2.0: Create `text_operations_rendered.rs`](#step-20-create-text_operations_renderedrs)
    - [Step 2.1: `test_paint_text_with_foreground_color_rendered`](#step-21-test_paint_text_with_foreground_color_rendered)
    - [Step 2.2: `test_paint_text_with_background_color_rendered`](#step-22-test_paint_text_with_background_color_rendered)
    - [Step 2.3: `test_paint_text_with_combined_style_rendered`](#step-23-test_paint_text_with_combined_style_rendered)
    - [Step 2.4: `test_paint_text_with_bold_style_rendered`](#step-24-test_paint_text_with_bold_style_rendered)
    - [Step 2.5: `test_paint_text_with_unicode_emoji_rendered`](#step-25-test_paint_text_with_unicode_emoji_rendered)
  - [Step 3: Medium-Value Tests - Screen Operations [MEDIUM PRIORITY]](#step-3-medium-value-tests---screen-operations-medium-priority)
    - [Step 3.0: Create `screen_operations_rendered.rs`](#step-30-create-screen_operations_renderedrs)
    - [Step 3.1: `test_clear_screen_rendered`](#step-31-test_clear_screen_rendered)
    - [Step 3.2: `test_clear_current_line_rendered`](#step-32-test_clear_current_line_rendered)
    - [Step 3.3: `test_clear_to_end_of_line_rendered`](#step-33-test_clear_to_end_of_line_rendered)
    - [Step 3.4: `test_clear_to_start_of_line_rendered`](#step-34-test_clear_to_start_of_line_rendered)
  - [Step 4: Update Module Documentation](#step-4-update-module-documentation)
    - [Step 4.0: Update `integration_tests/mod.rs`](#step-40-update-integration_testsmodrs)
    - [Step 4.1: Add Module-Level Docs for Each `*_rendered.rs`](#step-41-add-module-level-docs-for-each-_renderedrs)
  - [Step 5: Verify and Document](#step-5-verify-and-document)
    - [Step 5.0: Run All Tests](#step-50-run-all-tests)
    - [Step 5.1: Update Testing Strategy in `mod.rs`](#step-51-update-testing-strategy-in-modrs)
- [Notes](#notes)
  - [Tests NOT Worth Adding as Rendered](#tests-not-worth-adding-as-rendered)
  - [Dependencies](#dependencies)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Overview

## Background

The `direct_to_ansi/output/integration_tests/` module currently uses `StdoutMock` to test the output
painting pipeline. These tests verify that `RenderOpOutput` variants generate correct ANSI escape
sequences by comparing raw byte output.

We now have `OffscreenBuffer.apply_bytes()` capability that can parse ANSI sequences and render them
to a buffer. This enables **behavioral correctness testing**: verifying that output produces the
correct visual result, not just the correct bytes.

## Architecture Decision: StdoutMock → OffscreenBuffer (Not Real PTY)

We considered two approaches for behavioral testing:

| Approach                     | Platform       | What it adds over StdoutMock       |
| :--------------------------- | :------------- | :--------------------------------- |
| StdoutMock → OffscreenBuffer | Cross-platform | Verifies rendered result           |
| Real PTY → OffscreenBuffer   | Linux-only     | Tests full I/O stack (stdout, TTY) |

**Decision: Use StdoutMock → OffscreenBuffer.**

Rationale:

- **Same behavioral coverage** — both verify the rendered result is correct
- **Cross-platform** — runs on Linux, macOS, and Windows CI
- **Faster** — no PTY setup overhead
- **Simpler** — reuses existing `StdoutMock` infrastructure

Real PTY would only add value for testing I/O behavior (buffering, syscalls, raw mode effects), but
we already verify bytes are correct. The "real stdout" doesn't transform bytes in raw mode.

## Why Both Test Layers

| Aspect           | StdoutMock (existing)       | OffscreenBuffer (new)      |
| :--------------- | :-------------------------- | :------------------------- |
| **Tests**        | "Did we emit correct bytes" | "Did it render correctly?" |
| **Speed**        | Fast (~ms)                  | Fast (~ms)                 |
| **Platform**     | Cross-platform              | Cross-platform             |
| **Catches**      | Typos, wrong SGR codes      | Semantic bugs, off-by-one  |
| **Failure info** | "Wrong CSI parameter"       | "Text at wrong position"   |

**Decision: Keep both.** They test complementary properties:

- StdoutMock: Spec compliance (bytes match expected format)
- OffscreenBuffer: Behavioral correctness (visual result is correct)

## Module Organization

New rendered tests will be in separate `*_rendered.rs` modules:

```
direct_to_ansi/output/integration_tests/
├── color_operations.rs           # StdoutMock (existing)
├── cursor_movement.rs            # StdoutMock (existing)
├── cursor_movement_rendered.rs   # OffscreenBuffer (NEW)
├── screen_operations.rs          # StdoutMock (existing)
├── screen_operations_rendered.rs # OffscreenBuffer (NEW)
├── state_optimization.rs         # StdoutMock (existing, NO rendered equivalent)
├── text_operations.rs            # StdoutMock (existing)
├── text_operations_rendered.rs   # OffscreenBuffer (NEW)
├── test_helpers.rs               # Existing helpers
├── test_helpers_rendered.rs      # New helpers for OffscreenBuffer (NEW)
└── mod.rs
```

# Implementation Plan

## Step 0: Create Test Infrastructure

### Step 0.0: Create Rendered Output Test Helper Module

Create `test_helpers_rendered.rs` with utilities for StdoutMock → OffscreenBuffer testing:

```rust
// test_helpers_rendered.rs

/// Execute render operations via StdoutMock and render result to OffscreenBuffer
pub fn execute_and_render_to_buffer(
    ops: Vec<RenderOpOutput>,
    buffer_size: Size,
) -> OffscreenBuffer {
    // 1. Create StdoutMock output device
    // 2. Execute ops via RenderOpPaintImplDirectToAnsi
    // 3. Get captured bytes from StdoutMock
    // 4. Create OffscreenBuffer with buffer_size
    // 5. Call buffer.apply_bytes(captured_bytes)
    // 6. Return buffer for assertions
}

/// Execute a single render operation and render to buffer
pub fn execute_single_and_render(
    op: RenderOpOutput,
    buffer_size: Size,
) -> OffscreenBuffer {
    execute_and_render_to_buffer(vec![op], buffer_size)
}
```

### Step 0.1: Add OffscreenBuffer Assertion Helpers

```rust
/// Assert character at position
pub fn assert_char_at(buffer: &OffscreenBuffer, row: RowIndex, col: ColIndex, expected: char);

/// Assert foreground color at position
pub fn assert_fg_color_at(buffer: &OffscreenBuffer, row: RowIndex, col: ColIndex, expected: TuiColor);

/// Assert background color at position
pub fn assert_bg_color_at(buffer: &OffscreenBuffer, row: RowIndex, col: ColIndex, expected: TuiColor);

/// Assert cell is empty (default/cleared)
pub fn assert_cell_empty(buffer: &OffscreenBuffer, row: RowIndex, col: ColIndex);

/// Assert entire row is empty
pub fn assert_row_empty(buffer: &OffscreenBuffer, row: RowIndex);

/// Assert text string starting at position
pub fn assert_text_at(buffer: &OffscreenBuffer, row: RowIndex, col: ColIndex, expected: &str);
```

## Step 1: High-Value Tests - Cursor Movement [HIGH PRIORITY]

### Step 1.0: Create `cursor_movement_rendered.rs`

Create new module for rendered cursor movement tests.

### Step 1.1: `test_move_cursor_absolute_origin_rendered`

Verify cursor at (0,0) places character at top-left:

```rust
#[test]
fn test_move_cursor_absolute_origin_rendered() {
    // Move to (0,0), print 'X'
    // Assert: buffer[0][0] == 'X'
}
```

### Step 1.2: `test_move_cursor_absolute_5_10_rendered`

Verify 0-based to 1-based coordinate conversion:

```rust
#[test]
fn test_move_cursor_absolute_5_10_rendered() {
    // Move to (5,10), print 'X'
    // Assert: buffer[5][10] == 'X'
    // Assert: buffer[0][0] is empty (didn't accidentally go to origin)
}
```

### Step 1.3: `test_move_cursor_relative_to_rendered`

Verify origin + offset calculation:

```rust
#[test]
fn test_move_cursor_relative_to_rendered() {
    // MoveCursorPositionRelTo(origin=(5,3), relative=(2,7))
    // Print 'X'
    // Assert: buffer[7][10] == 'X' (5+2=7 row, 3+7=10 col)
}
```

### Step 1.4: `test_move_cursor_to_next_line_rendered`

Verify CNL (Cursor Next Line) moves down and resets column:

```rust
#[test]
fn test_move_cursor_to_next_line_rendered() {
    // Move to (5,10)
    // MoveCursorToNextLine(3)
    // Print 'X'
    // Assert: buffer[8][0] == 'X' (row 5+3=8, column reset to 0)
}
```

### Step 1.5: `test_move_cursor_to_previous_line_rendered`

Verify CPL (Cursor Previous Line) moves up and resets column:

```rust
#[test]
fn test_move_cursor_to_previous_line_rendered() {
    // Move to (10,15)
    // MoveCursorToPreviousLine(3)
    // Print 'X'
    // Assert: buffer[7][0] == 'X' (row 10-3=7, column reset to 0)
}
```

## Step 2: High-Value Tests - Text Operations [HIGH PRIORITY]

### Step 2.0: Create `text_operations_rendered.rs`

Create new module for rendered text painting tests.

### Step 2.1: `test_paint_text_with_foreground_color_rendered`

Verify foreground color applies to text cells:

```rust
#[test]
fn test_paint_text_with_foreground_color_rendered() {
    // Move to (0,0)
    // Paint "Hello" with fg=red
    // Assert: buffer[0][0..5] all have fg_color == red
    // Assert: buffer[0][0..5] contain "Hello"
}
```

### Step 2.2: `test_paint_text_with_background_color_rendered`

Verify background color applies to text cells:

```rust
#[test]
fn test_paint_text_with_background_color_rendered() {
    // Move to (0,0)
    // Paint "World" with bg=blue
    // Assert: buffer[0][0..5] all have bg_color == blue
}
```

### Step 2.3: `test_paint_text_with_combined_style_rendered`

Verify both fg and bg colors apply:

```rust
#[test]
fn test_paint_text_with_combined_style_rendered() {
    // Paint "Test" with fg=white, bg=blue
    // Assert: cells have both colors
}
```

### Step 2.4: `test_paint_text_with_bold_style_rendered`

Verify bold attribute applies:

```rust
#[test]
fn test_paint_text_with_bold_style_rendered() {
    // Paint "Bold" with bold=true
    // Assert: cells have bold attribute
}
```

### Step 2.5: `test_paint_text_with_unicode_emoji_rendered`

Verify emoji width handling (emoji should occupy 2 columns):

```rust
#[test]
fn test_paint_text_with_unicode_emoji_rendered() {
    // Paint "A👋B"
    // Assert: buffer[0][0] == 'A'
    // Assert: buffer[0][1] == '👋' (or first half of wide char)
    // Assert: buffer[0][2] is continuation (or empty for wide char)
    // Assert: buffer[0][3] == 'B'
}
```

## Step 3: Medium-Value Tests - Screen Operations [MEDIUM PRIORITY]

### Step 3.0: Create `screen_operations_rendered.rs`

Create new module for rendered screen operation tests.

### Step 3.1: `test_clear_screen_rendered`

Verify entire buffer is cleared:

```rust
#[test]
fn test_clear_screen_rendered() {
    // Fill buffer with 'X' characters
    // ClearScreen
    // Assert: all cells empty
}
```

### Step 3.2: `test_clear_current_line_rendered`

Verify only current row is cleared:

```rust
#[test]
fn test_clear_current_line_rendered() {
    // Fill rows 0-5 with text
    // Move to row 2
    // ClearCurrentLine
    // Assert: row 2 is empty
    // Assert: rows 0,1,3,4,5 still have text
}
```

### Step 3.3: `test_clear_to_end_of_line_rendered`

Verify partial line clear (cursor to EOL):

```rust
#[test]
fn test_clear_to_end_of_line_rendered() {
    // Fill row 0 with "ABCDEFGHIJ"
    // Move to (0, 5)
    // ClearToEndOfLine
    // Assert: buffer[0][0..5] == "ABCDE"
    // Assert: buffer[0][5..] is empty
}
```

### Step 3.4: `test_clear_to_start_of_line_rendered`

Verify partial line clear (start to cursor):

```rust
#[test]
fn test_clear_to_start_of_line_rendered() {
    // Fill row 0 with "ABCDEFGHIJ"
    // Move to (0, 5)
    // ClearToStartOfLine
    // Assert: buffer[0][0..5] is empty (or up to and including cursor)
    // Assert: buffer[0][6..] == "GHIJ" (or from cursor+1)
}
```

## Step 4: Update Module Documentation

### Step 4.0: Update `integration_tests/mod.rs`

Add the new rendered modules to mod.rs and update the Testing Strategy documentation to reflect the
two-tier approach (StdoutMock + OffscreenBuffer).

### Step 4.1: Add Module-Level Docs for Each `*_rendered.rs`

Each new rendered module should have documentation explaining:

- What it tests (behavioral correctness via OffscreenBuffer)
- How it complements the StdoutMock tests
- Link to the corresponding StdoutMock module

## Step 5: Verify and Document

### Step 5.0: Run All Tests

Ensure both StdoutMock and rendered tests pass:

```bash
cargo test -p r3bl_tui integration_tests
```

### Step 5.1: Update Testing Strategy in `mod.rs`

Document the dual-layer testing approach in the module documentation.

# Notes

## Tests NOT Worth Adding as Rendered

These remain StdoutMock-only:

| Test Category          | Reason                                               |
| :--------------------- | :--------------------------------------------------- |
| `state_optimization`   | Need to count operations, not observe final state    |
| `color_operations`     | Colors without text have no observable buffer effect |
| `ShowCursor`           | Cursor visibility not in OffscreenBuffer             |
| `HideCursor`           | Cursor visibility not in OffscreenBuffer             |
| `EnterAlternateScreen` | Mode switch, not buffer content                      |
| `ExitAlternateScreen`  | Mode switch, not buffer content                      |

## Dependencies

- `OffscreenBuffer.apply_bytes()` must support all ANSI sequences we generate
- Cross-platform (no PTY dependency)
