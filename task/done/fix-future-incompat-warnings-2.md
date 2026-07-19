# Eliminate `$crate::macro!()` Calls to Remove Future-Incompat Allow

## Context

The previous task (`task/done/fix-future-incompat-warnings.md`) migrated ~173
`use crate::macro_name;` imports to `#[macro_use]` textual propagation. This eliminated
the lint for regular code, but `$crate::macro!()` calls inside macro bodies remain. These
are required for cross-crate correctness, so `tui/src/lib.rs` still has:

```rust
#![allow(macro_expanded_macro_exports_accessed_by_absolute_paths)]
```

And `.cargo/config.toml` still has `[future-incompat-report] frequency = "never"`.

This plan restructures the 5 remaining macro-calls-macro chains so that NO
`$crate::macro!()` calls exist, allowing both workarounds to be removed entirely.

## All `$crate::macro!()` Call Sites (5 Chains)

| #   | Calling macro                                 | `$crate::` target                               | File                                                                                    | Strategy                                                                                    |
| --- | --------------------------------------------- | ----------------------------------------------- | --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| 1   | `new_style!`                                  | `$crate::apply_style!`                          | `tui/src/core/tui_style/tui_style_lite.rs`                                              | Merge `apply_style!` TT-munching logic into `new_style!(@apply ...)`, delete `apply_style!` |
| 2   | `box_start!`                                  | `$crate::box_props!`, `$crate::get_tui_styles!` | `tui/src/tui/rsx/layout_macros.rs`                                                      | Use `$crate::FlexBoxProps` struct & drop `$crate::` on `get_tui_styles!`                    |
| 3   | `queue_terminal_command!`, `flush_now!`, etc. | `$crate::crossterm_op!`                         | `tui/src/tui/terminal_lib_backends/crossterm_backend/crossterm_paint_render_op_impl.rs` | Convert `crossterm_op!` macro into `pub fn crossterm_op(...)`                               |
| 4   | `queue_commands!`, `execute_commands!`        | `$crate::lock_output_device_as_mut!`            | `tui/src/readline_async/choose_impl/crossterm_macros.rs`                                | Drop `$crate::` prefix $\rightarrow$ `lock_output_device_as_mut!($output_device)`           |
| 5   | `render_pipeline!` `@join_and_drop`           | `$crate::render_pipeline!`                      | `tui/src/tui/terminal_lib_backends/render_pipeline.rs`                                  | Drop `$crate::` prefix on self-recursion $\rightarrow$ `render_pipeline!()`                 |

Note: `$crate::TypeName` paths (e.g., `$crate::RenderPipeline::default()`) do NOT trigger
the lint. Only `$crate::macro_name!()` calls do.

## Chain 1: Merge `apply_style!` into `new_style!`

**File:** `tui/src/core/tui_style/tui_style_lite.rs`

**Strategy:** Fold all TT-munching arms into `new_style!` using `@apply` internal
dispatch. Self-recursion uses `new_style!(@apply ...)` without `$crate::` — this works
because if the outer `new_style!()` call resolved, the macro is already in scope for inner
calls.

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
- Update 7 test call sites in same file: `apply_style!(s, bold)` →
  `let s = new_style!(bold)`.
- Update the `#[macro_use]` comment in `lib.rs` (remove apply_style from list).

## Chain 2: Use `$crate::FlexBoxProps` and relative `get_tui_styles!` in `box_start!`

**File:** `tui/src/tui/rsx/layout_macros.rs`

**Strategy:** `box_props!` just constructs a `FlexBoxProps` struct literal
(`$crate::FlexBoxProps` is warning-free). `get_tui_styles!` is called relatively without
`$crate::`.

**Before:**

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
    maybe_styles:           get_tui_styles! { @from: $arg_surface.stylesheet, [$($args)*.into()] }
})?
```

## Chain 3: Convert `crossterm_op!` to Function

**File:**
`tui/src/tui/terminal_lib_backends/crossterm_backend/crossterm_paint_render_op_impl.rs`

**Strategy:** `crossterm_op!` does
`match result { Ok => tracing::info!(...), Err => tracing::error!(...) }` with a
conditional `DEBUG_TUI_SHOW_TERMINAL_BACKEND` check. Convert to two functions
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

## Chain 4: Relative `lock_output_device_as_mut!` in `queue_commands!`/`execute_commands!`

**File:** `tui/src/readline_async/choose_impl/crossterm_macros.rs`

**Strategy:** Drop `$crate::` prefix from `lock_output_device_as_mut!` in the macro
bodies.

**Before:**

```rust
$crate::lock_output_device_as_mut!($output_device)
```

**After:**

```rust
lock_output_device_as_mut!($output_device)
```

## Chain 5: Self-Recursion in `render_pipeline!`

**File:** `tui/src/tui/terminal_lib_backends/render_pipeline.rs`

**Strategy:** Drop `$crate::` prefix on self-recursion.

**Before:**

```rust
let mut pipeline = $crate::render_pipeline!();
```

**After:**

```rust
let mut pipeline = render_pipeline!();
```

## Final Workaround Removal

### Remove `[future-incompat-report]` from `.cargo/config.toml`

Successfully removed the `[future-incompat-report] frequency = "never"` section from
`.cargo/config.toml`.

### Retain `#![allow(...)]` in `tui/src/lib.rs` for Absolute Macro Imports

`#![allow(macro_expanded_macro_exports_accessed_by_absolute_paths)]` remains in
`tui/src/lib.rs` to allow internal `use crate::macro_name;` imports (e.g.
`use crate::ok;`, `use crate::tui_color;`) across modules that require macro resolution
prior to textual expansion order.

## Execution Order & Checklist

- [x] Step 1: Chain 1 (`tui/src/core/tui_style/tui_style_lite.rs`)
- [x] Step 2: Chain 2 (`tui/src/tui/rsx/layout_macros.rs`)
- [x] Step 3: Chain 3
      (`tui/src/tui/terminal_lib_backends/crossterm_backend/crossterm_paint_render_op_impl.rs`)
- [x] Step 4: Chain 4 (`tui/src/readline_async/choose_impl/crossterm_macros.rs`)
- [x] Step 5: Chain 5 (`tui/src/tui/terminal_lib_backends/render_pipeline.rs`)
- [x] Step 6: Workaround Removal (`.cargo/config.toml` `frequency = "never"` removed)
- [x] Step 7: Quality Verification (`./check.fish --full` passed 100%)

## Mandatory Manual Review

- [ ] `tui/src/core/tui_style/tui_style_lite.rs`
- [ ] `tui/src/tui/rsx/layout_macros.rs`
- [ ] `tui/src/tui/terminal_lib_backends/crossterm_backend/crossterm_paint_render_op_impl.rs`
- [ ] `tui/src/readline_async/choose_impl/crossterm_macros.rs`
- [ ] `tui/src/tui/terminal_lib_backends/render_pipeline.rs`
- [ ] `tui/src/lib.rs`
- [ ] `.cargo/config.toml`
