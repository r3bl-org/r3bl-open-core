<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->
**Table of Contents**  *generated with [DocToc](https://github.com/thlorenz/doctoc)*

- [Phase 2: Modernize readline_async Cursor Handling](#phase-2-modernize-readline_async-cursor-handling)
  - [Overview](#overview)
  - [Goals](#goals)
  - [Implementation Plan](#implementation-plan)
    - [Step 0: Fix Off-by-One Bug [COMPLETE]](#step-0-fix-off-by-one-bug-complete)
    - [Step 1: Part A - Replace Crossterm Cursor Commands [COMPLETE]](#step-1-part-a---replace-crossterm-cursor-commands-complete)
    - [Step 2: Part B - Migrate term_size to Size [COMPLETE]](#step-2-part-b---migrate-term_size-to-size-complete)
    - [Step 3: Remaining Type-Safe Coordinates [DEFERRED]](#step-3-remaining-type-safe-coordinates-deferred)
    - [Step 4: Testing [COMPLETE]](#step-4-testing-complete)
  - [Critical Bug: Off-by-One in `move_from_beginning`](#critical-bug-off-by-one-in-move_from_beginning)
    - [Location](#location)
    - [Problem](#problem)
    - [Evidence](#evidence)
    - [Root Cause](#root-cause)
    - [Fix](#fix)
    - [Priority](#priority)
  - [Why This Matters](#why-this-matters)
  - [Part A: Replace Crossterm Cursor Commands with AnsiSequenceGenerator](#part-a-replace-crossterm-cursor-commands-with-ansisequencegenerator)
    - [Current State](#current-state)
    - [Target State](#target-state)
    - [Mapping Table](#mapping-table)
    - [Key Files](#key-files)
    - [Windows Compatibility](#windows-compatibility)
  - [Part B: Type-Safe Coordinates](#part-b-type-safe-coordinates)
    - [Current State](#current-state-1)
    - [Target State](#target-state-1)
    - [Migration Table](#migration-table)
    - [Benefits](#benefits)
    - [Key Files](#key-files-1)
  - [Implementation Order](#implementation-order)
  - [Testing Strategy](#testing-strategy)
  - [Out of Scope](#out-of-scope)
  - [References](#references)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Phase 2: Modernize readline_async Cursor Handling

## Overview

After fixing the `SharedWriter` indentation bug (GitHub #439), this task modernizes the cursor
handling code in `readline_async` to use newer patterns available in the codebase.

**Related PR**: Bug fix for MoveRight(0) interpreted as MoveRight(1)

## Goals

1. **Fix off-by-one bug** in `move_from_beginning` boundary cases
2. Replace crossterm cursor commands with `AnsiSequenceGenerator`
3. Replace raw `u16`/`usize` fields with type-safe coordinate newtypes

---

## Implementation Plan

### Step 0: Fix Off-by-One Bug [COMPLETE]

Fixed both `move_to_beginning` and `move_from_beginning` to calculate `line_height` from the
position directly, not `prev_pos`:

```rust
// Before (buggy):
let prev_pos = to - 1;
let line_height = self.line_height(prev_pos);

// After (fixed):
let line_height = self.line_height(to);
```

Updated regression test to verify correct behavior at row boundaries (to=80, to=240).

### Step 1: Part A - Replace Crossterm Cursor Commands [COMPLETE]

Replaced all `crossterm::cursor` commands with `CsiSequence`:

- `cursor::MoveToColumn(0)` → `CsiSequence::CursorHorizontalAbsolute(1)`
- `cursor::MoveUp(n)` → `CsiSequence::CursorUp(n)`
- `cursor::MoveDown(n)` → `CsiSequence::CursorDown(n)`
- `cursor::MoveRight(n)` → `CsiSequence::CursorForward(n)`
- `cursor::MoveTo(0, 0)` → `AnsiSequenceGenerator::cursor_position(row(0), col(0))`

Note: `Clear` commands retained from crossterm as they work fine with the hybrid approach.

### Step 2: Part B - Migrate term_size to Size [COMPLETE]

Migrated `term_size: (u16, u16)` to `term_size: Size`:

- `term_size.0` → `term_size.col_width.as_u16()` (or `.as_usize()`)
- Constructor converts tuple to `Size::new((width(x), height(y)))`
- `handle_resize` updated to use `Size::new`

### Step 3: Remaining Type-Safe Coordinates [DEFERRED]

The following fields remain as raw types for now:

- `line_cursor_grapheme: usize` - Would require extensive changes
- `current_column: u16` - Would require type conversions throughout
- `last_line_length: usize` - Would require type conversions

These can be migrated in a future iteration if needed.

### Step 4: Testing [COMPLETE]

- All 30 `line_state` tests pass
- Windows cross-compilation verified via `cargo rustc --target x86_64-pc-windows-gnu`

---

## Critical Bug: Off-by-One in `move_from_beginning`

### Location

`tui/src/readline_async/readline_async_impl/line_state.rs`, function `move_from_beginning`

### Problem

The `prev_pos = to - 1` logic causes incorrect cursor positioning when `to` is an exact multiple of
the terminal width:

```rust
let prev_pos = if one_idx.overflows(to_index.convert_to_length())
    == ArrayOverflowResult::Overflowed
{
    0
} else {
    to - 1  // ← This causes off-by-one at boundaries
};
let line_height = self.line_height(prev_pos);  // line_height = prev_pos / term_width
```

### Evidence

On an 80-column terminal, `to` is 0-based target column position:

| `to` | Expected Position | MoveDown Emitted | Actual Landing  |
| ---- | ----------------- | ---------------- | --------------- |
| 80   | Row 1, Col 0      | 0                | Row 0, Col 0 ❌ |
| 160  | Row 2, Col 0      | 1                | Row 1, Col 0 ❌ |
| 240  | Row 3, Col 0      | 2                | Row 2, Col 0 ❌ |

The cursor lands one row too high when `to` is exactly at a row boundary.

### Root Cause

`line_height(prev_pos)` calculates how many rows to move down:

- For `to=240`: `prev_pos=239`, `line_height=239/80=2`
- But position 240 is at Row 3, so we should move down 3 rows, not 2

### Fix

The fix should calculate `line_height` based on `to` directly for the row calculation, not
`prev_pos`. The `prev_pos` might have been intended for a different purpose (perhaps related to
character positioning), but it's incorrect for cursor row calculation.

```rust
// Possible fix (needs verification):
let line_height = to / self.term_size.0;  // Use `to` directly, not `prev_pos`
let line_remaining_len = to % self.term_size.0;
```

### Priority

**High** - This affects cursor positioning in multi-line input scenarios.

---

## Why This Matters

The bug we fixed (`MoveRight(0)` → `MoveRight(1)`) happened because:

- Raw numeric types don't prevent semantic errors
- Direct crossterm calls bypass our ANSI infrastructure

The refactoring prevents similar bugs through:

- Type safety (can't confuse positions with lengths)
- Centralized ANSI generation (consistent behavior)

---

## Part A: Replace Crossterm Cursor Commands with AnsiSequenceGenerator

### Current State

`tui/src/readline_async/readline_async_impl/line_state.rs` uses crossterm directly:

```rust
use crossterm::{QueueableCommand, cursor, terminal::{Clear, ClearType}};

// Examples of current usage:
term.queue(cursor::MoveToColumn(0))?;
term.queue(cursor::MoveRight(n))?;
term.queue(cursor::MoveUp(n))?;
term.queue(cursor::MoveDown(n))?;
```

### Target State

Use `AnsiSequenceGenerator` and `CsiSequence` from our codebase:

```rust
use crate::{AnsiSequenceGenerator, CsiSequence, ColIndex, RowHeight};

// Replacement patterns:
term.write_all(AnsiSequenceGenerator::cursor_to_column(col(0)).as_bytes())?;
term.write_all(CsiSequence::CursorForward(n).to_string().as_bytes())?;
term.write_all(AnsiSequenceGenerator::cursor_previous_line(height(n)).as_bytes())?;
term.write_all(AnsiSequenceGenerator::cursor_next_line(height(n)).as_bytes())?;
```

### Mapping Table

| Crossterm Command           | Replacement                                                  |
| --------------------------- | ------------------------------------------------------------ |
| `cursor::MoveToColumn(col)` | `AnsiSequenceGenerator::cursor_to_column(ColIndex)`          |
| `cursor::MoveRight(n)`      | `CsiSequence::CursorForward(n).to_string()`                  |
| `cursor::MoveLeft(n)`       | `CsiSequence::CursorBackward(n).to_string()`                 |
| `cursor::MoveUp(n)`         | `AnsiSequenceGenerator::cursor_previous_line(RowHeight)`     |
| `cursor::MoveDown(n)`       | `AnsiSequenceGenerator::cursor_next_line(RowHeight)`         |
| `cursor::MoveTo(row, col)`  | `AnsiSequenceGenerator::cursor_position(RowIndex, ColIndex)` |
| `cursor::Hide`              | `AnsiSequenceGenerator::hide_cursor()`                       |
| `cursor::Show`              | `AnsiSequenceGenerator::show_cursor()`                       |

### Key Files

- **Generator**: `tui/src/core/ansi/generator/ansi_sequence_generator.rs`
- **CSI Sequences**: `tui/src/core/ansi/vt_100_pty_output_parser/protocols/csi_codes/sequence.rs`
- **Target file**: `tui/src/readline_async/readline_async_impl/line_state.rs`

### Windows Compatibility

`AnsiSequenceGenerator` is **100% cross-platform**:

- It generates ANSI strings (pure Rust, no platform code)
- Output goes to `&mut dyn Write` (works with any backend)
- On Windows, crossterm's `OutputDevice` handles the actual terminal I/O
- The ANSI sequences themselves are universal

---

## Part B: Type-Safe Coordinates

### Current State

`LineState` uses raw numeric types:

```rust
pub struct LineState {
    pub line_cursor_grapheme: usize,  // Raw usize
    pub current_column: u16,          // Raw u16
    pub term_size: (u16, u16),        // Raw tuple
    pub last_line_length: usize,      // Raw usize
    // ...
}
```

### Target State

Use coordinate newtypes:

```rust
use crate::{ColIndex, Index, Size, ColWidth};

pub struct LineState {
    pub line_cursor_grapheme: Index,      // Type-safe index
    pub current_column: ColIndex,         // Type-safe column
    pub term_size: Size,                  // Named fields: col_width, row_height
    pub last_line_length: ColWidth,       // Type-safe width
    // ...
}
```

### Migration Table

| Current Field                 | New Type   | Rationale                           |
| ----------------------------- | ---------- | ----------------------------------- |
| `line_cursor_grapheme: usize` | `Index`    | Position within grapheme array      |
| `current_column: u16`         | `ColIndex` | Terminal column (0-based)           |
| `term_size: (u16, u16)`       | `Size`     | Contains `ColWidth` and `RowHeight` |
| `last_line_length: usize`     | `ColWidth` | Width measurement (1-based count)   |

### Benefits

- **Type safety**: Can't accidentally mix positions with lengths
- **Bounds checking**: Built-in via `ArrayBoundsCheck` trait
- **Self-documenting**: Types express intent
- **Already partially used**: Lines 136-164 already use `idx()` for bounds checks

### Key Files

- **Coordinate types**: `tui/src/core/coordinates/buffer_coords/`
- **Bounds checking**: `tui/src/core/coordinates/bounds_check/`

---

## Implementation Order

1. **Part A first**: Replace crossterm calls with `AnsiSequenceGenerator`
   - Lower risk, simpler changes
   - Can be done method by method

2. **Part B second**: Migrate to type-safe coordinates
   - Requires updating field types and all usages
   - More invasive but prevents future bugs

---

## Testing Strategy

1. All existing `line_state` tests must pass
2. Run `shell_async` example manually to verify cursor behavior
3. Test on Linux (DirectToAnsi) and verify Windows cross-compilation:
   ```bash
   cargo rustc -p r3bl_tui --target x86_64-pc-windows-gnu -- --emit=metadata
   ```

---

## Out of Scope

- **OffscreenBuffer**: Too heavyweight for `readline_async`'s simple use case
- **Full TUI migration**: `readline_async` should remain lightweight

---

## References

- GitHub Issue: #439 (SharedWriter indentation bug)
- `AnsiSequenceGenerator` docs: `tui/src/core/ansi/generator/ansi_sequence_generator.rs`
- Coordinate system docs: `tui/src/core/coordinates/mod.rs`
