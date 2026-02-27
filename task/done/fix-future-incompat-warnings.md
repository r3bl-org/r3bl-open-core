<!-- cspell:words incompat -->

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Overview

The tui crate has 77 `#[macro_export]` macros imported via `use crate::macro_name;` at ~173+ call
sites across 68+ files. This pattern triggers the
`macro_expanded_macro_exports_accessed_by_absolute_paths` future-incompat lint
([rust-lang/rust#52234][lint-issue]). Currently suppressed by two workarounds:

1. `#![allow(macro_expanded_macro_exports_accessed_by_absolute_paths)]` in 4 `lib.rs` files
2. `[future-incompat-report] frequency = "never"` in `.cargo/config.toml` (lines 29-37)

This task removes both workarounds by migrating from `use crate::macro_name;` imports to
`#[macro_use]` textual propagation for all macros consumed within the tui crate. All 77 macros keep
`#[macro_export]` since `r3bl_tui` is a library crate and any macro may be used by downstream
consumers.

[lint-issue]: https://github.com/rust-lang/rust/issues/52234

## Strategy

The lint fires when `#[macro_export]` macros are accessed via absolute paths
(`use crate::macro_name;`). The fix: propagate macros textually via `#[macro_use]` on module
declarations, then delete all `use crate::macro_name;` imports for macros.

- **All 77 macros keep `#[macro_export]`** - `r3bl_tui` is a library crate; any macro is part of
  the public API and may be used by downstream consumers.
- **Within tui itself**: replace `use crate::macro_name;` with `#[macro_use]` textual scoping.
- **`$crate::` paths inside macros**: Continue to work - `$crate` resolves to the defining crate
  regardless of import mechanism.

## Macro names to remove from imports

Complete list from the 77 `#[macro_export]` macros:

```
ok, throws, throws_with_return, assert_eq2, assert_eq2_og, console_log,
with_mut, send_signal, timed, inline_string, tiny_inline_string, inline_vec,
join_fmt, join_with_index_fmt, join, join_with_index, pad_fmt, list,
render_list, tui_color, new_style, apply_style, get_tui_style, get_tui_styles,
tui_stylesheet, tui_styled_text, tui_styled_texts, pc, req_size_pc,
generate_index_type_impl, generate_length_type_impl, command,
bail_command_ran_and_failed, with_saved_pwd, fs_paths, fs_paths_exist,
try_create_temp_dir_and_cd, try_write_file, generate_pty_test, key_press,
crossterm_keyevent, lock_output_device_as_mut, generate_impl_display_for_fast_stringify,
telemetry_record, create_fmt, fmt_option, parse_list, set_mimalloc_in_main,
run_with_safe_stack, surface, unwrap_or_err, box_start, box_end, box_props,
box_start_with_component, box_start_with_surface_renderer,
render_component_in_current_box, render_component_in_given_box,
render_pipeline, queue_terminal_command, flush_now, disable_raw_mode_now,
enable_raw_mode_now, crossterm_op, cli_text_line, cli_text_lines,
rla_println, rla_print, rla_println_prefixed, early_return_if_paused,
empty_check_early_return, multiline_disabled_check_early_return,
queue_commands, queue_commands_no_lock, execute_commands, execute_commands_no_lock
```

# Implementation plan

## Step 0: Reorder modules in `core/mod.rs`

`#[macro_use]` uses textual scoping: macros are only visible to modules declared AFTER them.
Currently `decl_macros` (line 8) comes after `color_wheel` (line 5), which uses `ok!`/`throws!`.

**File**: `tui/src/core/mod.rs`

Move `decl_macros` to the FIRST position. Group all macro-defining modules before their consumers:

```rust
// Macro-defining modules FIRST (order matters for #[macro_use]).
#[macro_use] pub mod decl_macros;        // ok!, throws!, assert_eq2!, etc.
#[macro_use] pub mod stack_alloc_types;  // inline_string!, inline_vec!, etc.
#[macro_use] pub mod tui_style;          // tui_color!, new_style!, get_tui_style!
#[macro_use] pub mod tui_styled_text;    // tui_styled_text!, tui_styled_texts!
#[macro_use] pub mod coordinates;        // pc!, generate_index_type_impl!, etc.
#[macro_use] pub mod script;             // command!, with_saved_pwd!, etc.
#[macro_use] pub mod terminal_io;        // key_press!, etc.
#[macro_use] pub mod test_fixtures;      // generate_pty_test!
#[macro_use] pub mod common;             // telemetry_record!, etc.
#[macro_use] pub mod log;                // create_fmt!
#[macro_use] pub mod misc;               // fmt_option!
#[macro_use] pub mod heap_alloc_types;   // parse_list!

// Consumer-only modules.
pub mod ansi;
pub mod color_wheel;
pub mod glyphs;
pub mod graphemes;
pub mod osc;
pub mod pty;
pub mod pty_mux;
pub mod resilient_reactor_thread;
pub mod storage;
pub mod term;
```

Also update the `pub use` re-export block to match the new ordering.

**Note**: Some "consumer-only" modules (like `ansi`) also define macros (`cli_text_line!`,
`cli_text_lines!`) but those macros are only used within `ansi` itself or by external crates, not by
sibling modules. If sibling modules DO use them, move `ansi` to the macro-defining group.

## Step 1: Add `#[macro_use]` in `lib.rs`

**File**: `tui/src/lib.rs`

Add `#[macro_use]` to the `core` module declaration so macros propagate to `readline_async` and
`tui` modules:

```rust
#[macro_use]
pub mod core;
pub mod network_io;
#[macro_use] pub mod readline_async;  // defines rla_println!, etc.
#[macro_use] pub mod tui;             // defines box_start!, render_pipeline!, etc.
```

### Step 1.0: Submodule `#[macro_use]` chains inside `readline_async/`

- `readline_async/mod.rs`: `#[macro_use] pub mod choose_impl;` (for crossterm macros),
  `#[macro_use] pub mod readline_async_api;` (for rla_println), etc.

### Step 1.1: Submodule `#[macro_use]` chains inside `tui/`

- `tui/mod.rs`: `#[macro_use] pub mod rsx;` (for box_start, etc.),
  `#[macro_use] pub mod editor;`, `#[macro_use] pub mod layout;`,
  `#[macro_use] pub mod terminal_lib_backends;`

Each intermediate `mod.rs` in the hierarchy must also add `#[macro_use]` to the submodule that
defines macros (e.g., `rsx/mod.rs` needs `#[macro_use] pub mod layout_macros;`).

## Step 2: Remove `use crate::macro_name` imports (~173 sites)

This is the bulk of the work. For each file in `tui/src/` that imports macros via
`use crate::{...}`:

1. **Remove macro names** from `use crate::{...}` lines
2. **Delete empty import lines** (if only macros were imported)
3. **Keep non-macro imports** (types, functions, constants)

### Step 2.0: Write a helper script

Write a helper script (Python or shell) that:

1. Reads each `.rs` file under `tui/src/`
2. Parses `use crate::{...}` lines
3. Removes any identifier matching a macro name from the import list
4. Deletes the entire `use` line if it becomes empty
5. Preserves formatting and other imports

### Step 2.1: Run the helper script and verify

Run the script, then `./check.fish --check` after each batch.

## Step 3: Cleanup workarounds

### Step 3.0: Remove future-incompat suppression from `.cargo/config.toml`

Delete lines 29-37:

```toml
# TODO: task/fix-future-incompat-warnings.md will fix the following workaround
# Suppress the `macro_expanded_macro_exports_accessed_by_absolute_paths` ...
[future-incompat-report]
frequency = "never"
```

### Step 3.1: Remove lint allow from 4 `lib.rs` files

Remove from each:

```rust
#![allow(macro_expanded_macro_exports_accessed_by_absolute_paths)]
```

**Files:**

- `tui/src/lib.rs`
- `cmdr/src/lib.rs`
- `analytics_schema/src/lib.rs`
- `build-infra/src/lib.rs`

### Step 3.2: Update XMARK comments in `lib.rs` files

The XMARK comments currently mention the `allow` attribute - update them to reflect that it's no
longer needed.

## Step 4: Final verification

1. `./check.fish --check` - typecheck all crates (no `macro_expanded_...` warnings)
2. `./check.fish --build` - full build succeeds
3. `./check.fish --clippy` - clippy passes (unused imports, etc.)
4. `./check.fish --test` - all tests pass
5. `cargo fmt --all` - no reformatting triggered
6. `./check.fish --full` - no future-incompat report appears
7. Confirm `.cargo/config.toml` no longer has `[future-incompat-report]`
8. Confirm no `#![allow(macro_expanded_macro_exports_accessed_by_absolute_paths)]` in any file
