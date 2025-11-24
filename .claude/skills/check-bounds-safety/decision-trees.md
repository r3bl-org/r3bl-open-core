# Bounds Checking Decision Trees

This document provides decision trees and flowcharts for choosing the right bounds checking approach.

---

## Main Decision Tree: Which Trait Should I Use?

```
What are you trying to do?
│
├─ Access an array/buffer element?
│  │
│  └─ Use: ArrayBoundsCheck
│     Method: index.overflows(length)
│     Law: index < length
│     Example: buffer[index]
│
├─ Position a cursor in text?
│  │
│  └─ Use: CursorBoundsCheck
│     Method: length.check_cursor_position_bounds(pos)
│     Law: 0 <= position <= length
│     Example: Text editor cursor can be AFTER last char
│
├─ Check if content is visible in viewport?
│  │
│  └─ Use: ViewportBoundsCheck
│     Method: index.check_viewport_bounds(start, size)
│     Law: start <= index < start + size
│     Example: Scrolling, rendering optimization
│
├─ Validate a range for iteration?
│  │
│  └─ Use: RangeBoundsExt::check_range_is_valid_for_length
│     Method: range.check_range_is_valid_for_length(len)
│     Law: range.start < range.end && range.end <= length
│     Example: for i in 10..50 { buffer[i] }
│
├─ Check if index is within a specific range?
│  │
│  └─ Use: RangeBoundsExt::check_index_is_within
│     Method: range.check_index_is_within(index)
│     Law: range.start <= index < range.end (or <= for inclusive)
│     Example: VT-100 scroll regions
│
└─ Convert VT-100 inclusive range to Rust exclusive?
   │
   └─ Use: RangeConvertExt
      Method: inclusive_range.to_exclusive()
      Law: (a..=b) → (a..b+1)
      Example: VT-100 uses 1..=10, Rust uses 1..11
```

---

## Decision Tree: Array vs Cursor Bounds

```
Are you checking bounds for...?
│
├─ Reading/Writing array elements
│  │
│  ├─ Example: buffer[index] = value
│  │
│  └─ Use ArrayBoundsCheck
│     Why: Can't access element AT length
│     Law: index < length
│     ┌─────────────────┐
│     │ 0  1  2  3  4   │ ← Indices
│     │[a][b][c][d][e]  │ ← Buffer
│     └─────────────────┘
│     Length = 5
│     Valid indices: 0,1,2,3,4
│     Invalid: 5 (at length!)
│
└─ Positioning a cursor
   │
   ├─ Example: text|  (cursor after 'text')
   │
   └─ Use CursorBoundsCheck
      Why: Cursor CAN be at end
      Law: index <= length
      ┌─────────────────┐
      │ 0  1  2  3  4   │ ← Positions
      │ t  e  x  t  |   │ ← Text with cursor
      └─────────────────┘
      Length = 4 (4 chars)
      Valid positions: 0,1,2,3,4
      Position 4 is AFTER last char!
```

---

## Visual: Index vs Length

```
Index (0-based) vs Length (1-based)

Array: [a] [b] [c] [d] [e]
Index:  0   1   2   3   4    ← Start from 0
Length: 5                    ← Count of elements

Cursor positions in text "hello":
Position: 0   1   2   3   4   5
Text:     h   e   l   l   o   |
          ↑                   ↑
          Start               After last char
          (index 0)           (index = length)

Key insight:
- Array access:  index MUST be < length
- Cursor position: index CAN be = length
```

---

## Flowchart: Viewport Visibility

```
Is line visible in viewport?

Input:
- line_index (which line to check)
- viewport_start (first visible line)
- viewport_size (how many lines visible)

┌─────────────────────────────────────┐
│ Is line_index >= viewport_start?    │
└────────┬────────────────────┬───────┘
         │ NO                 │ YES
         ↓                    ↓
   ┌──────────┐     ┌─────────────────────────────────┐
   │ NOT      │     │ Is line_index < viewport_start  │
   │ VISIBLE  │     │              + viewport_size?   │
   └──────────┘     └────────┬────────────────┬───────┘
                             │ NO             │ YES
                             ↓                ↓
                       ┌──────────┐     ┌──────────┐
                       │ NOT      │     │ VISIBLE  │
                       │ VISIBLE  │     │          │
                       └──────────┘     └──────────┘

Example:
viewport_start = 10
viewport_size = 20
Visible range: lines 10-29

line 5:  NOT VISIBLE (< 10)
line 15: VISIBLE (10 <= 15 < 30)
line 35: NOT VISIBLE (>= 30)
```

---

## Flowchart: Range Validation

```
Is range valid for buffer?

Input:
- range (e.g., 10..50)
- buffer_length

┌─────────────────────────────────────┐
│ Is range.start < range.end?         │
└────────┬────────────────────┬───────┘
         │ NO                 │ YES
         ↓                    ↓
   ┌──────────┐     ┌─────────────────────────────┐
   │ INVALID  │     │ Is range.end <= buffer_len? │
   │ (empty)  │     └────────┬────────────┬───────┘
   └──────────┘              │ NO         │ YES
                             ↓            ↓
                       ┌──────────┐  ┌────────┐
                       │ INVALID  │  │ VALID  │
                       │ (overflow)│  │        │
                       └──────────┘  └────────┘

Example:
buffer_length = 100

Range 10..50:
  ✅ 10 < 50 AND 50 <= 100 → VALID

Range 50..10:
  ❌ 50 >= 10 → INVALID (empty range)

Range 10..150:
  ❌ 150 > 100 → INVALID (overflows buffer)
```

---

## Comparison Table: When to Use Each

| Scenario                          | Trait                 | Method                                     | Example                                       |
|-----------------------------------|-----------------------|-------------------------------------------|-----------------------------------------------|
| Buffer read/write                 | `ArrayBoundsCheck`    | `index.overflows(length)`                 | `buffer[idx]`                                 |
| Text cursor position              | `CursorBoundsCheck`   | `len.check_cursor_position_bounds(pos)`   | Cursor after last char                        |
| Scroll region check               | `ViewportBoundsCheck` | `index.check_viewport_bounds(start, sz)`  | Line visible on screen?                       |
| Iterator range validation         | `RangeBoundsExt`      | `range.check_range_is_valid_for_length()` | `for i in 10..50 { ... }`                     |
| Scroll region membership          | `RangeBoundsExt`      | `range.check_index_is_within(index)`      | Is cursor in scrollable region?               |
| VT-100 range conversion           | `RangeConvertExt`     | `inclusive.to_exclusive()`                | Convert `1..=10` to `1..11`                   |

---

## Edge Cases Reference

### Array Access Edge Cases

```rust
// Length 0 (empty array)
let length = len(0);
let index = idx(0);
index.overflows(length) // → Overflows! Can't access empty array

// Maximum valid index
let length = len(10);
let index = idx(9);  // Last valid index
index.overflows(length) // → Within

let index = idx(10);  // At length
index.overflows(length) // → Overflows!
```

### Cursor Position Edge Cases

```rust
// Empty text (length 0)
let length = len(0);
let cursor = idx(0);
length.check_cursor_position_bounds(cursor) // → Within! Cursor at start of empty text

// Cursor at end
let length = len(5);
let cursor = idx(5);  // After last character
length.check_cursor_position_bounds(cursor) // → Within! Valid cursor position
```

### Viewport Edge Cases

```rust
// Zero-sized viewport
let viewport_size = len(0);
// Nothing is visible!

// Item exactly at viewport start
let index = idx(10);
let viewport_start = idx(10);
let viewport_size = len(20);
index.check_viewport_bounds(viewport_start, viewport_size) // → true (10 is visible)

// Item exactly at viewport end
let index = idx(29);  // Last visible
index.check_viewport_bounds(idx(10), len(20)) // → true

let index = idx(30);  // First non-visible
index.check_viewport_bounds(idx(10), len(20)) // → false
```

---

## Quick Reference Card

```
┌─────────────────────────────────────────────────────────────┐
│             Type-Safe Bounds Checking                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Index types (0-based):   Index, RowIndex, ColIndex        │
│  Length types (1-based):  Length, RowHeight, ColWidth      │
│                                                             │
│  Constructors: idx(), row(), col(), len(), width(), height()│
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Array Access:     index < length                    │   │
│  │ Cursor Position:  index <= length                   │   │
│  │ Viewport:         start <= index < start + size     │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  Never use raw usize for indices or lengths!                │
│  Use .is_zero() instead of == 0                             │
│  Compiler prevents comparing incompatible types!            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Summary

**Golden Rules:**

1. **Always use Index types for indices** (row(), col(), idx())
2. **Always use Length types for lengths** (width(), height(), len())
3. **Use the right trait for the right scenario** (see decision tree)
4. **Remember the laws:**
   - Array: `index < length`
   - Cursor: `index <= length`
   - Viewport: `start <= index < start + size`

When in doubt, consult `tui/src/core/units/bounds_check/mod.rs` for comprehensive documentation!
