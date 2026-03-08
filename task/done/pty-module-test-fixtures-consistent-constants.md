# Consolidate PTY Test Marker Constants

## Problem

Local `const` declarations of `TEST_RUNNING`, `CONTROLLED_STARTING`, `CONTROLLED_READY`, and
`PTY_CONTROLLED_ENV_VAR` shadow (or should be) global constants in `constants.rs`. If the global
values are ever changed, local shadows silently diverge, causing test protocol mismatches.

Some files shadow these names with **different values**, making the name misleading.

The `generate_pty_test!` macro itself uses bare string literals for protocol markers instead of
referencing the shared constants.

## Source of Truth

`tui/src/core/test_fixtures/pty_test_fixtures/constants.rs`

**Currently has:**
- `pub const CONTROLLED_READY`
- `pub const LINE_PREFIX`

**Needs to add:**
- `pub const TEST_RUNNING: &str = "TEST_RUNNING";`
- `pub const CONTROLLED_STARTING: &str = "CONTROLLED_STARTING";`
- `pub const PTY_CONTROLLED_ENV_VAR: &str = "R3BL_PTY_TEST_CONTROLLED";`

Re-exported to crate root via barrel exports:
`constants.rs` -> `pty_test_fixtures/mod.rs` -> `test_fixtures/mod.rs` -> `core/mod.rs` ->
crate root.

## Part 0: Add constants to `constants.rs` and update the macro

### 0a. Add 3 new constants to `constants.rs`

Add `TEST_RUNNING`, `CONTROLLED_STARTING`, and `PTY_CONTROLLED_ENV_VAR` with doc comments.

### 0b. Update `generate_pty_test!` macro to use global constants

The macro currently has bare string literals and a local const. Change to use the globals
via `$crate::` paths:

| Line | Current | Change to |
|:-----|:--------|:----------|
| 242 | `const PTY_CONTROLLED_ENV_VAR: &str = "R3BL_PTY_TEST_CONTROLLED";` | Remove; use `$crate::PTY_CONTROLLED_ENV_VAR` |
| 249 | `println!("TEST_RUNNING");` | `println!("{}", $crate::TEST_RUNNING);` |
| 255 | `println!("CONTROLLED_STARTING");` | `println!("{}", $crate::CONTROLLED_STARTING);` |

**Not changed** (standard conventions, not domain-specific protocol values):
- `"1"` (env var truthy value)
- `"RUST_BACKTRACE"` (standard Rust env var)
- `"--test-threads"`, `"1"`, `"--nocapture"` (standard cargo test CLI flags)

**Files: 2** (`constants.rs`, `generate_pty_test.rs`)

## Part A: Remove local const declarations that exactly shadow the globals

Remove the local `const` lines and their doc comments. Add the constant names to each file's
existing `use crate::{...}` import.

**16 files, 42+ local const declarations to remove.**

| # | File | Remove |
|:--|:-----|:-------|
| 1 | `readline_async/.../pty_ctrl_navigation_test.rs` | `TEST_RUNNING`, `CONTROLLED_STARTING`, `CONTROLLED_READY` |
| 2 | `readline_async/.../pty_ctrl_w_test.rs` | `TEST_RUNNING`, `CONTROLLED_STARTING`, `CONTROLLED_READY` |
| 3 | `readline_async/.../pty_ctrl_u_test.rs` | `TEST_RUNNING`, `CONTROLLED_STARTING`, `CONTROLLED_READY` |
| 4 | `readline_async/.../pty_ctrl_d_eof_test.rs` | `TEST_RUNNING`, `CONTROLLED_STARTING`, `CONTROLLED_READY` |
| 5 | `readline_async/.../pty_alt_navigation_test.rs` | `TEST_RUNNING`, `CONTROLLED_STARTING`, `CONTROLLED_READY` |
| 6 | `readline_async/.../pty_alt_kill_test.rs` | `TEST_RUNNING`, `CONTROLLED_STARTING`, `CONTROLLED_READY` |
| 7 | `core/ansi/.../pty_sigwinch_test.rs` | `TEST_RUNNING`, `CONTROLLED_STARTING`, `CONTROLLED_READY` |
| 8 | `core/ansi/.../pty_keyboard_modifiers_test.rs` | `TEST_RUNNING`, `CONTROLLED_STARTING`, `CONTROLLED_READY` |
| 9 | `core/ansi/.../pty_new_keyboard_features_test.rs` | `TEST_RUNNING`, `CONTROLLED_STARTING`, `CONTROLLED_READY` |
| 10 | `core/ansi/.../pty_input_device_test.rs` | `CONTROLLED_STARTING`, `CONTROLLED_READY` |
| 11 | `core/ansi/.../pty_bracketed_paste_test.rs` | `CONTROLLED_STARTING`, `CONTROLLED_READY` |
| 12 | `core/ansi/.../pty_utf8_text_test.rs` | `CONTROLLED_STARTING`, `CONTROLLED_READY` |
| 13 | `core/ansi/.../pty_terminal_events_test.rs` | `CONTROLLED_STARTING`, `CONTROLLED_READY` |
| 14 | `core/ansi/.../pty_mouse_events_test.rs` | `CONTROLLED_STARTING`, `CONTROLLED_READY` |
| 15 | `core/terminal_io/.../backend_compat_output_test.rs` | `CONTROLLED_READY` |
| 16 | `core/terminal_io/.../backend_compat_input_test.rs` | `CONTROLLED_READY` |

## Part B: Replace bare string literals with const references

Replace bare `"TEST_RUNNING"`, `"CONTROLLED_STARTING"`, and `"CONTROLLED_READY"` string
literals in code (not doc comments) with the global constant.

**11 files, ~17 replacements.**

| # | File | Lines | Change |
|:--|:-----|:------|:-------|
| 1-6 | readline tests (same 6 from Part A) | controlled fns | `println!("TEST_RUNNING")` -> `println!("{TEST_RUNNING}")` |
| 10 | `pty_input_device_test.rs` | 243 | `println!("TEST_RUNNING")` -> `println!("{TEST_RUNNING}")` |
| 11 | `pty_bracketed_paste_test.rs` | 163 | same |
| 12 | `pty_utf8_text_test.rs` | 128 | same |
| 13 | `pty_terminal_events_test.rs` | 141 | same |
| 14 | `pty_mouse_events_test.rs` | 169 | same |
| 17 | `pty_shared_writer_no_blank_line_test.rs` | 231, 232, 349 | `"TEST_RUNNING"` -> `TEST_RUNNING`, `"CONTROLLED_STARTING"` -> `CONTROLLED_STARTING` |
| 18 | `pty_multiline_output_test.rs` | 204, 205, 342 | same |

Also in files 7-9 (sigwinch, keyboard_modifiers, new_keyboard_features): the controller
functions use `contains("TEST_RUNNING")` bare literals - replace with `contains(TEST_RUNNING)`.

## Part C: Rename local constants that shadow global names with DIFFERENT values

These shadow the global name but with **different values** - the most dangerous case.

### C1: `CONTROLLED_READY` shadows (4 files)

| # | File | Current | Rename to |
|:--|:-----|:--------|:----------|
| 19 | `pty_mio_poller_thread_lifecycle_test.rs:54` | `const CONTROLLED_READY = "LIFECYCLE_TEST_READY"` | `LIFECYCLE_READY` |
| 20 | `pty_mio_poller_singleton_test.rs:45` | `const CONTROLLED_READY = "SINGLETON_TEST_READY"` | `SINGLETON_READY` |
| 21 | `pty_mio_poller_subscribe_test.rs:55` | `const CONTROLLED_READY = "SUBSCRIBE_TEST_READY"` | `SUBSCRIBE_READY` |
| 22 | `pty_mio_poller_thread_reuse_test.rs:85` | `const CONTROLLED_READY = "REUSE_TEST_READY"` | `REUSE_READY` |

These tests use `spawn_controlled_in_pty` (not `generate_pty_test!`) with custom handshake
protocols. Renaming eliminates the shadowing hazard while preserving their unique marker
semantics.

### C2: `PTY_CONTROLLED_ENV_VAR` shadows (2 files)

| # | File | Current | Rename to |
|:--|:-----|:--------|:----------|
| 23 | `backend_compat_output_test.rs:149` | `const PTY_CONTROLLED_ENV_VAR = "R3BL_PTY_OUTPUT_TEST_CONTROLLED"` | `OUTPUT_TEST_ENV_VAR` |
| 24 | `backend_compat_input_test.rs:83` | `const PTY_CONTROLLED_ENV_VAR = "R3BL_PTY_INPUT_TEST_CONTROLLED"` | `INPUT_TEST_ENV_VAR` |

These backend compat tests use intentionally different env var names to avoid collisions
when running tests in parallel. The const NAME should not shadow the global.

## Part D: Leave alone (no changes)

- **Doc comments** in `lib.rs:572,585`, `pty_sigwinch_test.rs:35`,
  `backend_compat_output_test.rs:65`, `pty_input_device_test.rs:6,25,32,35,223` -
  human-readable documentation, not code.
- **Standard CLI/env literals** in the macro: `"RUST_BACKTRACE"`, `"1"`,
  `"--test-threads"`, `"--nocapture"` - these are standard conventions, not PTY protocol values.

## Summary

| Part | Files | Const removals | String literal fixes | Renames | Const additions |
|:-----|:------|:---------------|:---------------------|:--------|:----------------|
| 0 | 2 | 1 (local) | 2 | - | 3 |
| A | 16 | 42 | - | - | - |
| B | 11 | - | ~17 | - | - |
| C | 6 | - | - | 6 | - |
| **Total** | **24 unique files** | **43** | **~19** | **6** | **3** |

Parts A and B overlap (same files get both treatments). Net: **24 files touched**.

## Execution Order

1. **Part 0** first - add constants and update the macro (foundation for everything else)
2. **Part A + B together** per file - remove local consts, fix bare literals, add imports
3. **Part C** last - rename the intentionally-different-value constants
4. Run `./check.fish --full` to verify
