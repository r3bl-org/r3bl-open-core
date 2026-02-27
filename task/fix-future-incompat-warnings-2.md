# Eliminate `$crate::macro!()` Calls to Remove Future-Incompat Allow

## Context

The previous task (`task/done/fix-future-incompat-warnings.md`) migrated ~173 `use crate::macro_name;`
imports to `#[macro_use]` textual propagation. This eliminated the lint for regular code, but
`$crate::macro!()` calls inside macro bodies remain. These are required for cross-crate
correctness, so `tui/src/lib.rs` still has:

```rust
#![allow(macro_expanded_macro_exports_accessed_by_absolute_paths)]
```

And `.cargo/config.toml` still has `[future-incompat-report] frequency = "never"`.

This plan restructures the 5 remaining macro-calls-macro chains so that NO `$crate::macro!()`
calls exist, allowing both workarounds to be removed entirely.

## All `$crate::macro!()` Call Sites (5 Chains)

| # | Calling macro | `$crate::` target | File |
|---|---------------|-------------------|------|
| 1 | `new_style!` | `$crate::apply_style!` | `core/tui_style/tui_style_lite.rs` |
| 1 | `apply_style!` (self-recursion) | `$crate::apply_style!` | same file |
| 2 | `box_start!` | `$crate::box_props!` | `tui/rsx/layout_macros.rs` |
| 2 | `box_start!` | `$crate::get_tui_styles!` | same file |
| 3 | `queue_terminal_command!` | `$crate::crossterm_op!` | `tui/terminal_lib_backends/crossterm_backend/crossterm_paint_render_op_impl.rs` |
| 3 | `flush_now!` | `$crate::crossterm_op!` | same file |
| 3 | `disable_raw_mode_now!` | `$crate::crossterm_op!` | same file |
| 3 | `enable_raw_mode_now!` | `$crate::crossterm_op!` | same file |
| 4 | `queue_commands!` | `$crate::lock_output_device_as_mut!` | `readline_async/choose_impl/crossterm_macros.rs` |
| 4 | `execute_commands!` (x2) | `$crate::lock_output_device_as_mut!` | same file |
| 5 | `render_pipeline!` `@join_and_drop` | `$crate::render_pipeline!` | `tui/terminal_lib_backends/render_pipeline.rs` |

Note: `$crate::TypeName` paths (e.g., `$crate::RenderPipeline::default()`) do NOT trigger
the lint. Only `$crate::macro_name!()` calls do.

## Chain 1: Merge `apply_style!` into `new_style!`

**File:** `tui/src/core/tui_style/tui_style_lite.rs`

**Strategy:** Fold all TT-munching arms into `new_style!` using `@apply` internal dispatch.
Self-recursion uses `new_style!(@apply ...)` without `$crate::` — this works because if the
outer `new_style!()` call resolved, the macro is already in scope for inner calls.

**Before:**
```rust
#[macro_export]
macro_rules! new_style {
    ($($rem:tt)*) => {{
        let mut style = $crate::TuiStyle::default();
        $crate::apply_style!(style, $($rem)*);  // ← $crate:: macro call
        style
    }};
}

#[macro_export]
macro_rules! apply_style {
    ($style:ident, bold $($rem:tt)*) => {{
        $style.attribs.bold = Some($crate::tui_style_attrib::Bold);
        $crate::apply_style!($style, $($rem)*);  // ← $crate:: self-recursion
    }};
    // ... 14 more arms ...
    ($style:ident,) => {};
}
```

**After:**
```rust
#[macro_export]
macro_rules! new_style {
    // Entry point.
    ($($rem:tt)*) => {{
        #[allow(unused_mut)]
        let mut style = $crate::TuiStyle::default();
        new_style!(@apply style, $($rem)*);  // ← no $crate::
        style
    }};
    // Internal TT-munching arms (moved from apply_style!).
    (@apply $style:ident, bold $($rem:tt)*) => {{
        $style.attribs.bold = Some($crate::tui_style_attrib::Bold);
        new_style!(@apply $style, $($rem)*);  // ← no $crate::
    }};
    // ... all other arms with same pattern ...
    (@apply $style:ident,) => {};
}
```

**Cleanup:**
- Delete `apply_style!` macro entirely.
- Update 7 test call sites in same file: `apply_style!(s, bold)` → `let s = new_style!(bold)`.
- Update the `#[macro_use]` comment in `lib.rs` (remove apply_style from list).

## Chain 2: Inline `box_props!` and `get_tui_styles!` in `box_start!`

**File:** `tui/src/tui/rsx/layout_macros.rs`

**Strategy:** `box_props!` just constructs a `FlexBoxProps` struct literal. `get_tui_styles!`
`@from:` arm just calls `stylesheet.find_styles_by_ids(...)`. Inline both directly into
`box_start!`.

**Before (line 24-29):**
```rust
$arg_surface.box_start($crate::box_props! {
    id:                     $arg_id,
    dir:                    $arg_dir,
    requested_size_percent: $arg_requested_size_percent,
    maybe_styles:           $crate::get_tui_styles! { @from: $arg_surface.stylesheet, [$($args)*.into()] }
})?
```

**After:**
```rust
$arg_surface.box_start($crate::FlexBoxProps {
    id:                     $arg_id,
    dir:                    $arg_dir,
    requested_size_percent: $arg_requested_size_percent,
    maybe_styles:           $arg_surface.stylesheet.find_styles_by_ids(&[$($args)*.into()])
})?
```

**Cleanup:**
- `box_props!` macro: check if used elsewhere. If only in `box_start!`, remove it.
  If used externally, keep but mark `#[deprecated]`.
- `get_tui_styles!` macro: keep as-is (it has ~7 direct call sites in non-macro code that
  don't trigger the lint).

## Chain 3: Convert `crossterm_op!` to Function

**File:** `tui/src/tui/terminal_lib_backends/crossterm_backend/crossterm_paint_render_op_impl.rs`

**Strategy:** `crossterm_op!` does `match result { Ok => tracing::info!(...), Err => tracing::error!(...) }`
with a conditional `DEBUG_TUI_SHOW_TERMINAL_BACKEND` check. Convert to two functions
(with/without `is_mock` parameter). Callers change `$crate::crossterm_op!(...)` to
`$crate::crossterm_op(...)` — function calls don't trigger the lint.

**New functions (replace macro):**
```rust
/// Executes a crossterm operation with optional mock support and debug logging.
pub fn crossterm_op_with_mock(
    is_mock: bool,
    log_msg: &str,
    result: Result<(), impl std::fmt::Display>,
    success_msg: &str,
    error_msg: &str,
) {
    use crate::tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND;
    if !is_mock {
        crossterm_op(log_msg, result, success_msg, error_msg);
    }
}

pub fn crossterm_op(
    log_msg: &str,
    result: Result<(), impl std::fmt::Display>,
    success_msg: &str,
    error_msg: &str,
) {
    use crate::tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND;
    match result {
        Ok(_) => {
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::info!(message = success_msg, details = %log_msg);
            });
        }
        Err(err) => {
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::error!(message = error_msg, details = %log_msg, error = %err);
            });
        }
    }
}
```

**Callers change from:**
```rust
$crate::crossterm_op!($arg_log_msg, QueueableCommand::queue($writer, $command), ...)
```
**To:**
```rust
$crate::crossterm_op($arg_log_msg, QueueableCommand::queue($writer, $command), ...)
```

**Cleanup:**
- Delete `crossterm_op!` macro (only used by sibling macros in same file).

**Note on `tracing::info!`/`tracing::error!`:** These macros capture `file!()` and `line!()`
at their call site. Moving them into a function means the logged source location will point
to the function, not the original caller. This is acceptable — these are internal debug logs
gated behind `DEBUG_TUI_SHOW_TERMINAL_BACKEND`.

## Chain 4: Inline Lock in `queue_commands!`/`execute_commands!`

**File:** `tui/src/readline_async/choose_impl/crossterm_macros.rs`

**Strategy:** Replace `$crate::lock_output_device_as_mut!($output_device)` with
`&mut *$output_device.lock()` directly in the 3 call sites.

**`queue_commands!` — before:**
```rust
$crate::lock_output_device_as_mut!($output_device),
```
**After:**
```rust
&mut *$output_device.lock(),
```

**`execute_commands!` — before (2 sites):**
```rust
$crate::lock_output_device_as_mut!($output_device),  // queue line
$crate::lock_output_device_as_mut!($output_device).flush()  // flush line
```
**After:**
```rust
&mut *$output_device.lock(),  // queue line
(&mut *$output_device.lock()).flush()  // flush line
```

**Keep:** `lock_output_device_as_mut!` macro stays — it has ~40 direct call sites in regular
code that don't trigger the lint.

## Chain 5: Self-Recursion in `render_pipeline!`

**File:** `tui/src/tui/terminal_lib_backends/render_pipeline.rs`

**Strategy:** The `@join_and_drop` arm calls `$crate::render_pipeline!()`. Drop `$crate::` —
if the caller has `render_pipeline!` in scope (which it must, since it invoked the macro),
the inner call resolves too.

**Before (line 165):**
```rust
let mut pipeline = $crate::render_pipeline!();
```
**After:**
```rust
let mut pipeline = render_pipeline!();
```

## Final Cleanup

### Remove `#![allow(...)]` from `tui/src/lib.rs`

Delete lines 2582-2595:
```rust
// Allow `$crate::macro_name!()` inside ...
#![allow(macro_expanded_macro_exports_accessed_by_absolute_paths)]
```

### Remove `[future-incompat-report]` from `.cargo/config.toml`

Delete:
```toml
[future-incompat-report]
frequency = "never"
```

## Execution Order

1. **Chain 1** — merge `apply_style!` into `new_style!` → `./check.fish --check`
2. **Chain 2** — inline `box_props!` and `get_tui_styles!` in `box_start!` → `./check.fish --check`
3. **Chain 3** — convert `crossterm_op!` to function → `./check.fish --check`
4. **Chain 4** — inline lock in `queue_commands!`/`execute_commands!` → `./check.fish --check`
5. **Chain 5** — remove `$crate::` from `render_pipeline!` self-call → `./check.fish --check`
6. **Cleanup** — remove `#![allow(...)]` and `[future-incompat-report]` → `./check.fish --check`
7. **Full verification** → `./check.fish --full`

## Verification

1. `./check.fish --check` — zero warnings (no future-incompat report)
2. `./check.fish --clippy` — clippy passes
3. `./check.fish --test` — all tests pass
4. `cargo check -p r3bl_tui --all-targets` — examples compile
5. `./check.fish --full` — complete suite passes
6. Confirm no `#![allow(macro_expanded_macro_exports_accessed_by_absolute_paths)]` in any `.rs` file
7. Confirm no `[future-incompat-report]` in `.cargo/config.toml`
8. `grep -r '\$crate::[a-z_]*!' tui/src/` — no `$crate::macro!()` calls remain
