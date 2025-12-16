# Plan: ANSI Sequence Generator INPUT/OUTPUT Symmetry Refactoring

## Summary

Refactor the ANSI sequence generation infrastructure to create clear INPUT/OUTPUT symmetry:
- Rename output generator for clarity
- Create new input generator (test + doc only) with shared constants
- Consolidate duplicated test helpers across codebase

## Phase 1: Rename Output Generator

### Step 1.1: Rename file
- `tui/src/core/ansi/generator/ansi_sequence_generator.rs` → `ansi_sequence_generator_output.rs`

### Step 1.2: Update mod.rs
In `tui/src/core/ansi/generator/mod.rs`:
```rust
// Change from:
mod ansi_sequence_generator;
pub use ansi_sequence_generator::*;

// To:
mod ansi_sequence_generator_output;
pub use ansi_sequence_generator_output::*;
```

### Step 1.3: Update imports
- Most imports use re-exported `AnsiSequenceGenerator` from crate root — no changes needed
- Verify no direct module path imports exist

---

## Phase 2: Create Input Generator

### Step 2.1: Create new file
Create `tui/src/core/ansi/generator/ansi_sequence_generator_input.rs`:

```rust
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI escape sequence generator for terminal INPUT (test fixtures only).
//!
//! Provides input sequence generation for testing. Creates symmetry with
//! [`ansi_sequence_generator_output`] for output sequences.

use crate::input_sequences::{
    ANSI_CSI_BRACKET, ANSI_ESC, ANSI_FUNCTION_KEY_TERMINATOR, ANSI_PARAM_SEPARATOR,
    ANSI_SS3_O, ARROW_DOWN_FINAL, ARROW_LEFT_FINAL, ARROW_RIGHT_FINAL, ARROW_UP_FINAL,
    BACKTAB_FINAL, SPECIAL_END_FINAL, SPECIAL_HOME_FINAL,
    SS3_F1_FINAL, SS3_F2_FINAL, SS3_F3_FINAL, SS3_F4_FINAL,
};

// ==================== Pre-computed Sequence Constants ====================

pub const SEQ_ARROW_UP: &[u8] = &[ANSI_ESC, ANSI_CSI_BRACKET, ARROW_UP_FINAL];
pub const SEQ_ARROW_DOWN: &[u8] = &[ANSI_ESC, ANSI_CSI_BRACKET, ARROW_DOWN_FINAL];
pub const SEQ_ARROW_RIGHT: &[u8] = &[ANSI_ESC, ANSI_CSI_BRACKET, ARROW_RIGHT_FINAL];
pub const SEQ_ARROW_LEFT: &[u8] = &[ANSI_ESC, ANSI_CSI_BRACKET, ARROW_LEFT_FINAL];
pub const SEQ_HOME: &[u8] = &[ANSI_ESC, ANSI_CSI_BRACKET, SPECIAL_HOME_FINAL];
pub const SEQ_END: &[u8] = &[ANSI_ESC, ANSI_CSI_BRACKET, SPECIAL_END_FINAL];
pub const SEQ_BACKTAB: &[u8] = &[ANSI_ESC, ANSI_CSI_BRACKET, BACKTAB_FINAL];
pub const SEQ_F1: &[u8] = &[ANSI_ESC, ANSI_SS3_O, SS3_F1_FINAL];
pub const SEQ_F2: &[u8] = &[ANSI_ESC, ANSI_SS3_O, SS3_F2_FINAL];
pub const SEQ_F3: &[u8] = &[ANSI_ESC, ANSI_SS3_O, SS3_F3_FINAL];
pub const SEQ_F4: &[u8] = &[ANSI_ESC, ANSI_SS3_O, SS3_F4_FINAL];

// ==================== Helper Functions ====================

/// Builds CSI sequence: `ESC [ <final>`
#[must_use]
pub const fn csi(final_byte: u8) -> [u8; 3] {
    [ANSI_ESC, ANSI_CSI_BRACKET, final_byte]
}

/// Builds SS3 sequence: `ESC O <final>`
#[must_use]
pub const fn ss3(final_byte: u8) -> [u8; 3] {
    [ANSI_ESC, ANSI_SS3_O, final_byte]
}

/// Builds CSI tilde sequence: `ESC [ <code> ~`
#[must_use]
pub fn csi_tilde(code: u16) -> Vec<u8> {
    let mut seq = vec![ANSI_ESC, ANSI_CSI_BRACKET];
    seq.extend(code.to_string().as_bytes());
    seq.push(ANSI_FUNCTION_KEY_TERMINATOR);
    seq
}

/// Builds CSI with modifier: `ESC [ 1 ; <mod+1> <final>`
#[must_use]
pub fn csi_modified(modifier: u8, final_byte: u8) -> Vec<u8> {
    let param = 1 + modifier;
    vec![ANSI_ESC, ANSI_CSI_BRACKET, b'1', ANSI_PARAM_SEPARATOR, b'0' + param, final_byte]
}

// ==================== Re-exports ====================

pub use crate::core::ansi::vt_100_terminal_input_parser::test_fixtures::{
    generate_keyboard_sequence,
    generate_mouse_sequence_bytes,
    generate_resize_sequence,
    generate_focus_sequence,
    generate_paste_sequence,
};
```

### Step 2.2: Update generator/mod.rs
```rust
#[cfg(any(test, doc))]
mod ansi_sequence_generator_input;

#[cfg(any(test, doc))]
pub use ansi_sequence_generator_input::*;
```

---

## Phase 3: Update lib.rs Re-exports

Add test+doc exports at crate root:
```rust
#[cfg(any(test, doc))]
pub use core::ansi::generator::ansi_sequence_generator_input::*;
```

---

## Phase 4: Consolidate Test Helpers

### Step 4.1: Update `stateful_parser.rs` tests

**4.1a: Update `test_fixtures` module** — add imports from shared input generator:
```rust
#[cfg(test)]
mod test_fixtures {
    pub use super::StatefulInputParser;
    pub use crate::ansi_sequence_generator_input::{
        SEQ_ARROW_UP, SEQ_ARROW_DOWN, SEQ_ARROW_RIGHT, SEQ_ARROW_LEFT,
        SEQ_HOME, SEQ_END, ANSI_ESC, ASCII_DEL,
    };
    // ... rest of imports
}
```

**4.1b: Update `tests_esc_disambiguation` module** (lines 188-229) — replace manual byte arrays:
```rust
// BEFORE (line 191):
parser.advance(&[ANSI_ESC, b'[', b'A'], false);

// AFTER:
parser.advance(SEQ_ARROW_UP, false);
```

Update all four arrow key tests:
- `arrow_up_complete_sequence()` — use `SEQ_ARROW_UP`
- `arrow_down_complete_sequence()` — use `SEQ_ARROW_DOWN`
- `arrow_right_complete_sequence()` — use `SEQ_ARROW_RIGHT`
- `arrow_left_complete_sequence()` — use `SEQ_ARROW_LEFT`

**4.1c: Update `tests_chunked_input` module** — update any manual sequences similarly

### Step 4.2: Update `backend_compat_input_test.rs`
Remove local `generate_test_sequences` module helpers, use shared:
```rust
use crate::{csi, ss3, csi_tilde, csi_modified, SEQ_ARROW_UP, ...};
```

### Step 4.3: Update `keyboard.rs` test helpers

**File**: `tui/src/core/ansi/vt_100_terminal_input_parser/keyboard.rs`

**4.3a: Remove redundant helper functions** (lines 1061-1097):
- `arrow_key_sequence()` — replace with `csi()` or `SEQ_ARROW_*` constants
- `function_key_sequence()` — replace with `ss3()` or `csi_tilde()`

**4.3b: Update Alt+key tests** (lines 1708-1776) — replace manual sequences:
```rust
// BEFORE:
&[ANSI_ESC, b'b']  // Alt+b

// AFTER (if helper added):
alt_char(b'b')  // or keep manual if clearer
```

---

## Phase 5: Verification

1. `cargo check -p r3bl_tui` — compilation
2. `cargo test -p r3bl_tui` — all tests pass
3. `cargo clippy -p r3bl_tui` — no warnings

---

## Critical Files

| File | Action |
|------|--------|
| `tui/src/core/ansi/generator/ansi_sequence_generator.rs` | Rename to `*_output.rs` |
| `tui/src/core/ansi/generator/mod.rs` | Update module decls, add input generator |
| `tui/src/core/ansi/generator/ansi_sequence_generator_input.rs` | Create (new file) |
| `tui/src/lib.rs` | Add `#[cfg(any(test, doc))]` re-exports |
| `tui/src/tui/.../stateful_parser.rs` | Update test_fixtures imports, replace manual sequences |
| `tui/src/core/terminal_io/backend_compat_tests/backend_compat_input_test.rs` | Remove duplicated `csi()`, `ss3()`, `csi_tilde()`, `csi_modified()` helpers |
| `tui/src/core/ansi/vt_100_terminal_input_parser/keyboard.rs` | Remove `arrow_key_sequence()`, `function_key_sequence()` helpers |

---

## Key Constraint

**`ansi_sequence_generator_input.rs` uses `#[cfg(any(test, doc))]`** — available for tests AND documentation, but not included in production builds.
