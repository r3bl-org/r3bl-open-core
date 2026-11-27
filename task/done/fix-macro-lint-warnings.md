# Task: Fix Macro Lint Warnings

<!-- BEGIN mktoc -->

- [Task: Fix Macro Lint Warnings](#task-fix-macro-lint-warnings)
- [Overview](#overview)
    - [Why `#[macro_use]` Goes on `mod` Declarations (Not `pub use *`)](#why-macro_use-goes-on-mod-declarations-not-pub-use-)
    - [Architectural Rules for Implementation](#architectural-rules-for-implementation)
    - [Complete List of 79 `#[macro_export]` Macros in `r3bl_tui`](#complete-list-of-79-macro_export-macros-in-r3bl_tui)
- [Implementation plan](#implementation-plan)
    - [Phase 1: Inline Macro Cross-Calls](#phase-1-inline-macro-cross-calls)
    - [Phase 2: Add Textual Macro Propagation](#phase-2-add-textual-macro-propagation)
    - [Phase 3: Remove Core ANSI Imports](#phase-3-remove-core-ansi-imports)
    - [Phase 4: Remove Core Foundation Imports](#phase-4-remove-core-foundation-imports)
    - [Phase 5: Remove Readline and Network Imports](#phase-5-remove-readline-and-network-imports)
    - [Phase 6: Remove TUI Macro Imports](#phase-6-remove-tui-macro-imports)
    - [Phase 7: Remove Allow and Update Docs](#phase-7-remove-allow-and-update-docs)
    - [Phase 8: Minimal #[macro_use] Scope and Uniform #[macro_export] Policy](#phase-8-minimal-macro_use-scope-and-uniform-macro_export-policy)
    - [Phase 9: Document Private Items Directly and Remove Conditional Visibility](#phase-9-document-private-items-directly-and-remove-conditional-visibility)
        - [Phase 9.1: Enable `--document-private-items` Across Tooling and Crates](#phase-91-enable---document-private-items-across-tooling-and-crates)
        - [Phase 9.2: Collapse Dual-Arm Conditional Visibility Across 29 Coordinator Files](#phase-92-collapse-dual-arm-conditional-visibility-across-29-coordinator-files)
        - [Phase 9.3: Fix Newly Surfaced Private-Item Intra-Doc Link Warnings (63 Files, 98 Warnings)](#phase-93-fix-newly-surfaced-private-item-intra-doc-link-warnings-63-files-98-warnings)
        - [Phase 9.4: Update Agent Instructions and Skills & Run Final Verification](#phase-94-update-agent-instructions-and-skills--run-final-verification)
    - [Phase 10: Tighten Barrel `pub mod` Submodules and Remove `#[doc(inline)]` Shims](#phase-10-tighten-barrel-pub-mod-submodules-and-remove-docinline-shims)
        - [Phase 10.1: Targeted Cleanup of `#[doc(inline)]` Coordinators (`coordinates`, `graphemes`, `pty`, `md_parser`)](#phase-101-targeted-cleanup-of-docinline-coordinators-coordinates-graphemes-pty-md_parser)
        - [Phase 10.2: Crate-Wide `r3bl_tui` Barrel `pub mod` Audit and Tightening](#phase-102-crate-wide-r3bl_tui-barrel-pub-mod-audit-and-tightening)
        - [Phase 10.3: Update Module Organization Docs/Skills & Run Final Verification](#phase-103-update-module-organization-docsskills--run-final-verification)

<!-- END mktoc -->

## Overview

`r3bl_tui` currently triggers the
`macro_expanded_macro_exports_accessed_by_absolute_paths` future-incompatibility lint
([rust-lang/rust#52234][lint-issue]) across 257 source files when compiled with
`--all-targets`, masked by
`#![allow(macro_expanded_macro_exports_accessed_by_absolute_paths)]` in `tui/src/lib.rs`.
Cargo surfaces this in its future-incompatibility report on every build because `rustc`
plans to turn the lint into a hard compiler error.

### Why `#[macro_use]` Goes on `mod` Declarations (Not `pub use *`)

1. **Invalid on `pub use *`**: Placing `#[macro_use]` before `pub use foo::*;` is a Rust
   syntax error (`#[macro_use] is not supported on use statements`). `#[macro_export]`
   already places all 79 macros at the `r3bl_tui` crate root automatically for external
   crates (`cmdr`, integration tests, benchmarks, examples, and doctests), so `pub use *`
   never exports `macro_rules!` macros anyway.
2. **Valid on `mod` declarations**: Placing `#[macro_use]` on `mod foo;` / `pub mod foo;`
   propagates `macro_rules!` definitions textually in source declaration order throughout
   `r3bl_tui` without going through crate-root path resolution.
3. **Why imports and cross-calls must also be removed**: Adding `#[macro_use]` alone makes
   macros available in textual scope, but leaving `use crate::<macro>;` imports in
   consumer files or `$crate::<macro>!()` calls inside macro bodies still resolves those
   macros via absolute crate-root paths and continues triggering the lint. Both must be
   removed.

### Architectural Rules for Implementation

- **Keep `#[macro_export]` on all 79 macros**: `r3bl_tui` is a public library crate;
  external crates and doctests continue importing macros via `use r3bl_tui::<macro>;`
  (cross-crate macro imports do not trigger this lint).
- **No bulk scripts**: Per `AGENTS.md` Rule C.1, scripts (`python`, `sed`, `awk`, `perl`,
  `bash`, `fish`, or disposable Rust binaries) are strictly prohibited. Perform all edits
  file-by-file using native file-editing tools (`edit_file`), optionally parallelized via
  subagents (`batch-refactor-with-sub-agents` skill) with `Model: "inherit"`.
- **Preserve non-macro imports**: Only strip macro names from `use crate::{...};`
  statements (including inner `#[cfg(test)] mod tests` blocks). Keep regular functions and
  types such as `ch`, `vp_col`, `vp_row`, `vp_width`, `vp_height`, `vp_pos`, `vp_caret`,
  `c_col`, `c_row`, `c_index`, `c_len`, `usize`, and `cli_text_inline` intact. If a
  `use crate::...;` statement only imported macros, delete the entire `use` statement.

### Complete List of 79 `#[macro_export]` Macros in `r3bl_tui`

```text
assert_eq2, assert_eq2_og, bail_command_ran_and_failed, box_end, box_props,
box_start, cli_text_line, cli_text_lines, command, console_log,
convert_u16_to_ascii_str_slice, convert_usize_to_ascii_str_slice, create_fmt,
crossterm_keyevent, define_ansi_const, early_return_if_paused,
empty_check_early_return, execute_commands, execute_commands_no_lock,
flush_now, fmt_option, format_no_alloc, fs_paths, fs_paths_exist,
generate_async_isolated_process_test, generate_canvas_index_type_impl,
generate_canvas_length_type_impl, generate_impl_display_for_fast_stringify,
generate_index_type_impl, generate_isolated_process_test,
generate_length_type_impl, generate_pty_test, get_tui_style, get_tui_styles,
inline_string, inline_vec, join, join_fmt, join_with_index,
join_with_index_fmt, key_press, list, multiline_disabled_check_early_return,
new_style, ok, pad_fmt, parse_list, pc, queue_commands,
queue_commands_no_lock, queue_terminal_command,
render_component_in_current_box, render_component_in_given_box, render_list,
render_pipeline, req_size_pc, retry_until_success_test,
retry_until_success_test_async, rla_print, rla_println, rla_println_prefixed,
run_with_safe_stack, scoped_mutex, send_signal, set_mimalloc_in_main, surface,
telemetry_record, throws, throws_with_return, timed, tiny_inline_string,
try_create_temp_dir_and_cd, tui_color, tui_styled_text, tui_styled_texts,
tui_stylesheet, unwrap_or_err, with_mut, with_saved_pwd
```

[lint-issue]: https://github.com/rust-lang/rust/issues/52234

## Implementation plan

### Phase 1: Inline Macro Cross-Calls

Eliminate the only 2 `$crate::<macro>!()` invocations inside `#[macro_export]` macro
bodies by inlining the underlying `$crate::Type` / method expressions directly
(`$crate::Type` does not trigger the lint and preserves cross-crate hygiene without
requiring callers to import helper macros).

- [x] Update `req_size_pc!` in `tui/src/core/coordinates/percent_spec/req_size_pc.rs`
      (lines 35-36): replace `$crate::pc!($arg_width)?` and `$crate::pc!($arg_height)?`
      with `$crate::Pc::try_from($arg_width)?` and `$crate::Pc::try_from($arg_height)?`.
- [x] Update `box_start!` in `tui/src/tui/rsx/layout_macros.rs` (line 28): replace
      `$crate::get_tui_styles! { @from: $arg_surface.stylesheet, [$($args)*.into()] }`
      with `$arg_surface.stylesheet.find_styles_by_ids(&[$($args)*.into()])`.
- [x] Run `./check.fish --check` to verify compilation.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/src/core/coordinates/percent_spec/req_size_pc.rs`
    - [ ] `tui/src/tui/rsx/layout_macros.rs`

### Phase 2: Add Textual Macro Propagation

Add `#[macro_use]` to module declarations and reorder macro-defining modules before
sibling consumer modules so textual macro scoping resolves all 79 macros across every
target (`lib`, `tests`, `benches`).

- [x] Update top-level module declarations in `tui/src/lib.rs` (lines 3568-3571): add
      `#[macro_use]` to `pub mod core;`, `pub mod readline_async;`, and `pub mod tui;`
      (`pub mod network_io;` defines no macros).
- [x] Reorder and annotate `tui/src/core/mod.rs` in strict topological dependency order
      (critical: `test_fixtures` only uses `ok!` from `decl_macros`, whereas `common`,
      `script`, `terminal_io`, and `ansi` tests invoke `generate_isolated_process_test!`
      and `generate_pty_test!` from `test_fixtures`):
    1. `#[macro_use] pub mod decl_macros;`
    2. `#[macro_use] pub mod stack_alloc_types;`
    3. `#[macro_use] pub mod test_fixtures;`
    4. `#[macro_use] pub mod common;`
    5. `#[macro_use] pub mod tui_style;`
    6. `#[macro_use] pub mod tui_styled_text;`
    7. `#[macro_use] pub mod coordinates;`
    8. `#[macro_use] pub mod script;`
    9. `#[macro_use] pub mod terminal_io;`
    10. `#[macro_use] pub mod log;`
    11. `#[macro_use] pub mod misc;`
    12. `#[macro_use] pub mod heap_alloc_types;`
    13. `#[macro_use] pub mod ansi;`
    14. Consumer-only modules (`notification`, `color_wheel`, `glyphs`, `graphemes`,
        `pty`, `resilient_reactor_thread`, `storage`).
- [x] Add `#[macro_use]` and topologically reorder `core` submodule coordinators:
    - `tui/src/core/decl_macros/mod.rs`: `#[macro_use] pub mod macros;`
    - `tui/src/core/stack_alloc_types/mod.rs`: `#[macro_use]` on `constructors`,
      `fast_int_fmt`, `into_existing`, `list_of`, `make_new`, `memory_allocator`, and
      `render_list` (place before `items` and `sizes`).
    - `tui/src/core/test_fixtures/mod.rs`: move `#[macro_use]` submodules
      `isolated_process_fixtures`, `pty_test_fixtures`, and `retry_fixtures` before
      `output_device_fixtures` and `tcp_stream_fixtures`.
    - `tui/src/core/test_fixtures/isolated_process_fixtures/mod.rs`:
      `#[macro_use] pub mod generate_isolated_process_test;`
    - `tui/src/core/test_fixtures/pty_test_fixtures/mod.rs`: move
      `#[macro_use] mod generate_pty_test;` before `pty_test_child`, `pty_test_watchdog`,
      `single_thread_safe_controlled_child`, and `controlled_ready_signal` siblings.
    - `tui/src/core/common/mod.rs`: move `#[macro_use] pub mod fast_strings;`,
      `#[macro_use] pub mod scoped_mutex;`, and `#[macro_use] pub mod telemetry;` to the
      top before `string_repeat_cache`, `time_duration`, and other consumer submodules.
    - `tui/src/core/common/fast_strings/mod.rs`: `#[macro_use] pub mod fast_stringify;`
      and `#[macro_use] pub mod format_no_alloc;` (before `hex_slice`).
    - `tui/src/core/common/scoped_mutex/mod.rs`: move `#[macro_use] mod mutex_ext;` first
      (before `deadlock_prevention` and `scoped_mutex_public_api`).
    - `tui/src/core/tui_style/mod.rs`: move `#[macro_use] pub mod tui_color;`,
      `#[macro_use] pub mod tui_style_lite;`, and `#[macro_use] pub mod tui_stylesheet;`
      (in that order, since `tui_style_lite` tests call `tui_color!`) before
      `tui_style_impl` and other siblings.
    - `tui/src/core/tui_styled_text/mod.rs`: `#[macro_use] pub mod tui_styled_text_impl;`
      and `#[macro_use] pub mod tui_styled_texts_impl;` (before
      `vec_vs_smallvec_bench_tests`).
    - `tui/src/core/coordinates/mod.rs`: place `#[macro_use] pub mod viewport_coords;`,
      `#[macro_use] pub mod canvas;`, and `#[macro_use] pub mod percent_spec;` before
      `bounds_check`, `byte`, `primitives`, and `vt_100_ansi_coords`.
    - `tui/src/core/coordinates/viewport_coords/mod.rs`: move
      `#[macro_use] pub mod index_and_length_impl_macros;` to first position (before
      `col_index`, `col_width`, `index`, `length`, `row_height`, `row_index`).
    - `tui/src/core/coordinates/canvas/mod.rs`: `#[macro_use] mod canvas_coords;`
    - `tui/src/core/coordinates/canvas/canvas_coords/mod.rs`: move
      `#[macro_use] mod macros;` to first position (before `c_col` through `c_width`).
    - `tui/src/core/coordinates/percent_spec/mod.rs`: `#[macro_use] pub mod pc;` and
      `#[macro_use] pub mod req_size_pc;`.
    - `tui/src/core/script/mod.rs`: move `#[macro_use] mod command_impl;`,
      `#[macro_use] mod directory_change;`, `#[macro_use]` on both `#[cfg]` arms of
      `fs_path`, and `#[macro_use] mod temp_dir;` to the top before consumer submodules.
    - `tui/src/core/script/command_impl/mod.rs`: move
      `#[macro_use] pub mod command_runner;` before `command_run_result`.
    - `tui/src/core/terminal_io/mod.rs`: move `#[macro_use] mod key_press;` to first
      position; in `tui/src/core/terminal_io/key_press.rs` (line 325), add `#[macro_use]`
      to inline `pub mod convert_key_event` so `crossterm_keyevent!` propagates outward.
    - `tui/src/core/log/mod.rs`: move `#[macro_use] pub mod tracing_init;` to first
      position.
    - `tui/src/core/misc/mod.rs`: move `#[macro_use] pub mod formatter;` to first
      position.
    - `tui/src/core/heap_alloc_types/mod.rs`: `#[macro_use] pub mod parse_list;`
    - `tui/src/core/ansi/mod.rs`: place `#[macro_use] pub mod constants;` and
      `#[macro_use]` on both `#[cfg]` arms of `generator` first before `color`,
      `detect_color`, `osc`, `terminal_raw_mode`, and `vt_100_*`.
    - `tui/src/core/ansi/constants/mod.rs`: move `#[macro_use] mod macros;` to first
      position (before `csi`, `da`, `dsr`, `esc`, `generic`, `input_sequences`,
      `osc_constants`, `sgr`).
    - `tui/src/core/ansi/generator/mod.rs`: `#[macro_use] mod cli_text;`
- [x] Add `#[macro_use]` and topologically reorder `readline_async` and `tui` submodule
      coordinators:
    - `tui/src/readline_async/mod.rs`: `#[macro_use] pub mod readline_async_api;`,
      `#[macro_use] pub mod choose_impl;`, `#[macro_use] pub mod readline_async_impl;`
    - `tui/src/readline_async/choose_impl/mod.rs`:
      `#[macro_use] pub mod crossterm_macros;`
    - `tui/src/readline_async/readline_async_impl/mod.rs`: move `#[macro_use]` on both
      `#[cfg]` arms of `line_state` to first position (before `event_conversion`,
      `line_control_task`, `readline_struct`, and `readline_async_integration_tests`).
    - `tui/src/readline_async/readline_async_impl/line_state/mod.rs`:
      `#[macro_use] mod core;`
    - `tui/src/tui/mod.rs`: move `#[macro_use] pub mod rsx;`,
      `#[macro_use] pub mod layout;`, `#[macro_use] pub mod terminal_lib_backends;`, and
      `#[macro_use] pub mod editor;` to the top before `animator`, `cmd_line_args`,
      `dialog`, `global_constants`, `md_parser`, `syntax_highlighting`, and
      `terminal_window`.
    - `tui/src/tui/rsx/mod.rs`: `#[macro_use] pub mod layout_macros;` and
      `#[macro_use] pub mod render_macros;`
    - `tui/src/tui/layout/mod.rs`: move `#[macro_use] pub mod layout_error;` first, then
      `#[macro_use] pub mod surface;`, before `flex_box` and `props`.
    - `tui/src/tui/terminal_lib_backends/mod.rs`: add `#[macro_use]` to both `#[cfg]` arms
      of `render_pipeline` and both `#[cfg]` arms of `crossterm_backend`.
    - `tui/src/tui/terminal_lib_backends/crossterm_backend/mod.rs`: move `#[macro_use]` on
      both `#[cfg]` arms of `crossterm_paint_render_op_impl` before `debug` and
      `input_device_impl`.
    - `tui/src/tui/editor/mod.rs`: move `#[macro_use] pub mod editor_engine;` before
      `default_content`, `editor_buffer`, `editor_component`, `zero_copy_gap_buffer`, and
      `test_fixtures_editor`.
    - `tui/src/tui/editor/editor_engine/mod.rs`: move
      `#[macro_use] pub mod editor_macros;` to first position (before `caret_mut` and
      `content_mut`).
- [x] Run `./check.fish --check` to confirm all module reorderings compile cleanly.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/src/lib.rs`
    - [ ] `tui/src/core/mod.rs`
    - [ ] `tui/src/core/decl_macros.rs`
    - [ ] `tui/src/core/stack_alloc_types/mod.rs`
    - [ ] `tui/src/core/test_fixtures/mod.rs`
    - [ ] `tui/src/core/test_fixtures/isolated_process_fixtures/mod.rs`
    - [ ] `tui/src/core/test_fixtures/pty_test_fixtures/mod.rs`
    - [ ] `tui/src/core/common/mod.rs`
    - [ ] `tui/src/core/common/fast_strings/mod.rs`
    - [ ] `tui/src/core/common/scoped_mutex/mod.rs`
    - [ ] `tui/src/core/tui_style/mod.rs`
    - [ ] `tui/src/core/tui_styled_text/mod.rs`
    - [ ] `tui/src/core/coordinates/mod.rs`
    - [ ] `tui/src/core/coordinates/viewport_coords/mod.rs`
    - [ ] `tui/src/core/coordinates/canvas/mod.rs`
    - [ ] `tui/src/core/coordinates/canvas/canvas_coords/mod.rs`
    - [ ] `tui/src/core/coordinates/percent_spec/mod.rs`
    - [ ] `tui/src/core/script/mod.rs`
    - [ ] `tui/src/core/script/command_impl/mod.rs`
    - [ ] `tui/src/core/terminal_io/mod.rs`
    - [ ] `tui/src/core/terminal_io/key_press.rs`
    - [ ] `tui/src/core/log/mod.rs`
    - [ ] `tui/src/core/misc/mod.rs`
    - [ ] `tui/src/core/heap_alloc_types/mod.rs`
    - [ ] `tui/src/core/ansi/mod.rs`
    - [ ] `tui/src/core/ansi/constants/mod.rs`
    - [ ] `tui/src/core/ansi/generator/mod.rs`
    - [ ] `tui/src/readline_async/mod.rs`
    - [ ] `tui/src/readline_async/choose_impl/mod.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/mod.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/line_state/mod.rs`
    - [ ] `tui/src/tui/mod.rs`
    - [ ] `tui/src/tui/rsx/mod.rs`
    - [ ] `tui/src/tui/layout/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/crossterm_backend/mod.rs`
    - [ ] `tui/src/tui/editor/mod.rs`
    - [ ] `tui/src/tui/editor/editor_engine/mod.rs`

### Phase 3: Remove Core ANSI Imports

Remove `use crate::<macro>;` imports from all 53 flagged files under `tui/src/core/ansi/`
using native file-editing tools (`edit_file` / subagent batches).

- [x] Strip macro names from `use crate::...;` imports across `tui/src/core/ansi/**`
      (including inner `#[cfg(test)]` modules).
- [x] Run `./check.fish --check` and
      `cargo check -p r3bl_tui --all-targets -q --message-format=short` to verify both
      `lib` and `#[cfg(test)]` scopes compile.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/src/core/ansi/color/ansi_value.rs`
    - [ ] `tui/src/core/ansi/color/rgb_value.rs`
    - [ ] `tui/src/core/ansi/constants/csi.rs`
    - [ ] `tui/src/core/ansi/constants/da.rs`
    - [ ] `tui/src/core/ansi/constants/dsr.rs`
    - [ ] `tui/src/core/ansi/constants/esc.rs`
    - [ ] `tui/src/core/ansi/constants/generic.rs`
    - [ ] `tui/src/core/ansi/constants/input_sequences.rs`
    - [ ] `tui/src/core/ansi/constants/osc_constants.rs`
    - [ ] `tui/src/core/ansi/constants/sgr.rs`
    - [ ] `tui/src/core/ansi/detect_color/detect_color_integration_tests/pty_test_color_detection.rs`
    - [ ] `tui/src/core/ansi/detect_color/detect_color_support.rs`
    - [ ] `tui/src/core/ansi/generator/cli_text.rs`
    - [ ] `tui/src/core/ansi/generator/da.rs`
    - [ ] `tui/src/core/ansi/generator/dsr.rs`
    - [ ] `tui/src/core/ansi/generator/esc.rs`
    - [ ] `tui/src/core/ansi/generator/sgr_code.rs`
    - [ ] `tui/src/core/ansi/osc/osc_codes.rs`
    - [ ] `tui/src/core/ansi/osc/osc_sender.rs`
    - [ ] `tui/src/core/ansi/osc/pty_osc_progress_scanner.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_basic_enable_disable.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_flag_verification.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_input_behavior.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_multiple_cycles.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_poison_recovery.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_unix.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_char_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_clear_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_line_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_mode_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_scroll_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_clear_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/protocols/csi_codes/sequence.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/protocols/csi_codes/sgr_attribute_sequences.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/protocols/csi_codes/sgr_color_sequences.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/test_fixtures_ofs_buf_vt_100.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/validation_tests/observe_real_interactive_terminal_input_events.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_bracketed_paste_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_csi_u_keyboard_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_input_device_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_keyboard_modifiers_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_keypad_and_special_keys_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_mio_poller_singleton_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_mio_poller_subscribe_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_mio_poller_thread_lifecycle_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_mio_poller_thread_reuse_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_mouse_events_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_osc_color_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_osc_safety_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_osc_sender_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_sigwinch_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_terminal_events_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_utf8_text_test.rs`

### Phase 4: Remove Core Foundation Imports

Remove `use crate::<macro>;` imports from the remaining 90 flagged files across
`tui/src/core/` (`color_wheel`, `common`, `coordinates`, `graphemes`, `log`, `misc`,
`pty`, `resilient_reactor_thread`, `script`, `stack_alloc_types`, `storage`,
`terminal_io`, `test_fixtures`, `tui_styled_text`, and `tui_style`).

- [x] Strip macro names from `use crate::...;` imports across the remaining 90
      `tui/src/core/**` files (preserving constructor functions like `ch`, `vp_col`,
      `vp_row`, `c_col`, `c_row`, etc.).
- [x] Run `./check.fish --check` and
      `cargo check -p r3bl_tui --all-targets -q --message-format=short` to verify
      compilation.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/src/core/color_wheel/color_wheel_impl.rs`
    - [ ] `tui/src/core/color_wheel/gradients/ansi_256.rs`
    - [ ] `tui/src/core/color_wheel/gradients/truecolor.rs`
    - [ ] `tui/src/core/color_wheel/lolcat.rs`
    - [ ] `tui/src/core/common/fast_strings/fast_stringify.rs`
    - [ ] `tui/src/core/common/fast_strings/hex_slice.rs`
    - [ ] `tui/src/core/common/scoped_mutex/deadlock_prevention/policy_impl.rs`
    - [ ] `tui/src/core/common/scoped_mutex/scoped_mutex_public_api.rs`
    - [ ] `tui/src/core/common/string_repeat_cache.rs`
    - [ ] `tui/src/core/common/time_duration.rs`
    - [ ] `tui/src/core/coordinates/canvas/canvas_coords/c_col.rs`
    - [ ] `tui/src/core/coordinates/canvas/canvas_coords/c_height.rs`
    - [ ] `tui/src/core/coordinates/canvas/canvas_coords/c_index.rs`
    - [ ] `tui/src/core/coordinates/canvas/canvas_coords/c_length.rs`
    - [ ] `tui/src/core/coordinates/canvas/canvas_coords/c_row.rs`
    - [ ] `tui/src/core/coordinates/canvas/canvas_coords/c_width.rs`
    - [ ] `tui/src/core/coordinates/percent_spec/req_size_pc.rs`
    - [ ] `tui/src/core/coordinates/primitives/ch_unit.rs`
    - [ ] `tui/src/core/coordinates/viewport_coords/col_index.rs`
    - [ ] `tui/src/core/coordinates/viewport_coords/col_width.rs`
    - [ ] `tui/src/core/coordinates/viewport_coords/index.rs`
    - [ ] `tui/src/core/coordinates/viewport_coords/length.rs`
    - [ ] `tui/src/core/coordinates/viewport_coords/row_height.rs`
    - [ ] `tui/src/core/coordinates/viewport_coords/row_index.rs`
    - [ ] `tui/src/core/graphemes/gc_string/owned/gc_string_owned_non_editor_impl.rs`
    - [ ] `tui/src/core/graphemes/gc_string/owned/gc_string_owned.rs`
    - [ ] `tui/src/core/graphemes/unicode_segment/seg_index.rs`
    - [ ] `tui/src/core/graphemes/unicode_segment/seg_length.rs`
    - [ ] `tui/src/core/graphemes/unicode_segment/segment_builder.rs`
    - [ ] `tui/src/core/log/custom_event_formatter.rs`
    - [ ] `tui/src/core/log/log_integration_tests/pty_test_custom_formatter.rs`
    - [ ] `tui/src/core/log/log_public_api.rs`
    - [ ] `tui/src/core/log/rolling_file_appender_impl.rs`
    - [ ] `tui/src/core/log/tracing_init.rs`
    - [ ] `tui/src/core/misc/calc_str_len.rs`
    - [ ] `tui/src/core/misc/string_helper.rs`
    - [ ] `tui/src/core/pty/e2e_tests/osc_capture_test.rs`
    - [ ] `tui/src/core/pty/pty_mux/input_router/keyboard_command.rs`
    - [ ] `tui/src/core/pty/pty_mux/mux.rs`
    - [ ] `tui/src/core/pty/pty_mux/output_renderer.rs`
    - [ ] `tui/src/core/pty/pty_mux/process_manager.rs`
    - [ ] `tui/src/core/pty/pty_session/threads/reader.rs`
    - [ ] `tui/src/core/pty/pty_session/threads/writer.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/process_isolated_tests/fixtures.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/process_isolated_tests/mod.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_input_sender.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_integration_tests/double_panic_prevention_test.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_integration_tests/pty_test_production_factory_restart.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_integration_tests/pty_test_production_poll_error.rs`
    - [ ] `tui/src/core/script/command_impl/command_runner.rs`
    - [ ] `tui/src/core/script/command_impl/command_run_result.rs`
    - [ ] `tui/src/core/script/crates_api.rs`
    - [ ] `tui/src/core/script/directory_change.rs`
    - [ ] `tui/src/core/script/directory_create.rs`
    - [ ] `tui/src/core/script/download.rs`
    - [ ] `tui/src/core/script/environment.rs`
    - [ ] `tui/src/core/script/env_source/conformance_tests/tests/test_golden_formatters.rs`
    - [ ] `tui/src/core/script/env_source/conformance_tests/tests/test_subshell_unix.rs`
    - [ ] `tui/src/core/script/fs_path.rs`
    - [ ] `tui/src/core/script/git/branch_ops.rs`
    - [ ] `tui/src/core/script/git/file_ops.rs`
    - [ ] `tui/src/core/script/git/git_test_fixtures.rs`
    - [ ] `tui/src/core/script/github_api.rs`
    - [ ] `tui/src/core/script/git/status_ops.rs`
    - [ ] `tui/src/core/script/package_manager.rs`
    - [ ] `tui/src/core/script/permissions.rs`
    - [ ] `tui/src/core/script/temp_dir.rs`
    - [ ] `tui/src/core/stack_alloc_types/into_existing.rs`
    - [ ] `tui/src/core/stack_alloc_types/items.rs`
    - [ ] `tui/src/core/storage/kv.rs`
    - [ ] `tui/src/core/terminal_io/backend_compat_tests/backend_compat_input_test.rs`
    - [ ] `tui/src/core/terminal_io/backend_compat_tests/backend_compat_output_test.rs`
    - [ ] `tui/src/core/terminal_io/backend_compat_tests/pty_non_blocking_stdout_no_panic_test.rs`
    - [ ] `tui/src/core/terminal_io/backend_compat_tests/pty_terminal_mode_test.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/capabilities_integration_tests/test_disclaimer.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/capabilities_integration_tests/test_piped_stdin.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/capabilities_integration_tests/test_piped_stdout.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/capabilities_integration_tests/test_pty_is_interactive.rs`
    - [ ] `tui/src/core/terminal_io/modifier_keys_mask.rs`
    - [ ] `tui/src/core/terminal_io/output_device.rs`
    - [ ] `tui/src/core/terminal_io/shared_writer.rs`
    - [ ] `tui/src/core/terminal_io/terminal_mode_controller.rs`
    - [ ] `tui/src/core/test_fixtures/output_device_fixtures/stdout_mock.rs`
    - [ ] `tui/src/core/test_fixtures/pty_test_fixtures/pty_test_child/pty_test_child_impl.rs`
    - [ ] `tui/src/core/test_fixtures/tcp_stream_fixtures/mock_async_stream.rs`
    - [ ] `tui/src/core/tui_styled_text/tui_styled_texts_impl.rs`
    - [ ] `tui/src/core/tui_styled_text/vec_vs_smallvec_bench_tests.rs`
    - [ ] `tui/src/core/tui_style/tui_color.rs`
    - [ ] `tui/src/core/tui_style/tui_style_impl.rs`
    - [ ] `tui/src/core/tui_style/tui_style_lite.rs`
    - [ ] `tui/src/core/tui_style/tui_stylesheet.rs`

### Phase 5: Remove Readline and Network Imports

Remove `use crate::<macro>;` imports from all 36 flagged files across
`tui/src/network_io/` and `tui/src/readline_async/`.

- [x] Strip macro names from `use crate::...;` imports across `tui/src/network_io/**` (2
      files) and `tui/src/readline_async/**` (34 files).
- [x] Run `./check.fish --check` and
      `cargo check -p r3bl_tui --all-targets -q --message-format=short` to verify
      compilation.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/src/network_io/json_serde.rs`
    - [ ] `tui/src/network_io/length_prefix_protocol.rs`
    - [ ] `tui/src/readline_async/choose_api.rs`
    - [ ] `tui/src/readline_async/choose_impl/choose_integration_tests/pty_shared_writer_pause_test.rs`
    - [ ] `tui/src/readline_async/choose_impl/choose_integration_tests/pty_test_select_component.rs`
    - [ ] `tui/src/readline_async/choose_impl/event_loop.rs`
    - [ ] `tui/src/readline_async/choose_impl/function_component.rs`
    - [ ] `tui/src/readline_async/choose_impl/scroll.rs`
    - [ ] `tui/src/readline_async/choose_impl/select_component.rs`
    - [ ] `tui/src/readline_async/choose_impl/state.rs`
    - [ ] `tui/src/readline_async/choose_impl/style.rs`
    - [ ] `tui/src/readline_async/readline_async_api.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/event_conversion.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/line_control_task.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/line_state/core.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/line_state/cursor.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/line_state/event_handlers.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/line_state/output.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/line_state/render.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_alt_navigation_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_concurrent_input_output_deadlock_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_ctrl_d_delete_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_ctrl_d_eof_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_ctrl_navigation_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_ctrl_u_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_ctrl_w_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_editor_state_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_multiline_output_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_readline_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_shared_writer_no_blank_line_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/test_pty_alt_word_deletion.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_struct.rs`
    - [ ] `tui/src/readline_async/spinner_impl/spinner_integration_tests/pty_spinner_test.rs`
    - [ ] `tui/src/readline_async/spinner_impl/spinner_print.rs`
    - [ ] `tui/src/readline_async/spinner_impl/spinner_render.rs`
    - [ ] `tui/src/readline_async/spinner.rs`

### Phase 6: Remove TUI Macro Imports

Remove `use crate::<macro>;` imports from all 78 flagged files under `tui/src/tui/`.

- [x] Strip macro names from `use crate::...;` imports across `tui/src/tui/**`.
- [x] Run `./check.fish --check` and
      `cargo check -p r3bl_tui --all-targets -q --message-format=short` to verify
      compilation.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/src/tui/animator/animator_struct.rs`
    - [ ] `tui/src/tui/dialog/dialog_buffer/dialog_buffer_struct.rs`
    - [ ] `tui/src/tui/dialog/dialog_component/dialog_component_struct.rs`
    - [ ] `tui/src/tui/dialog/dialog_component/dialog_event.rs`
    - [ ] `tui/src/tui/dialog/dialog_engine/dialog_engine_api.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/buffer_config_struct.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/buffer_struct.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/caret_locate.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/clipboard/clipboard_service_impl.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/clipboard/clipboard_service.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/history/editor_history.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/render_cache.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/selection/selection_container.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/selection/selection_line.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/selection/selection_state_machine.rs`
    - [ ] `tui/src/tui/editor/editor_component/editor_component_struct.rs`
    - [ ] `tui/src/tui/editor/editor_component/editor_event.rs`
    - [ ] `tui/src/tui/editor/editor_engine/caret_mut.rs`
    - [ ] `tui/src/tui/editor/editor_engine/content_mut.rs`
    - [ ] `tui/src/tui/editor/editor_engine/engine_internal_api.rs`
    - [ ] `tui/src/tui/editor/editor_engine/engine_public_api.rs`
    - [ ] `tui/src/tui/editor/editor_engine/engine_struct.rs`
    - [ ] `tui/src/tui/editor/editor_engine/scroll_editor_content.rs`
    - [ ] `tui/src/tui/editor/editor_engine/validate_buffer_mut.rs`
    - [ ] `tui/src/tui/editor/test_fixtures_editor.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_adapters.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_delete_ops.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_insert_ops.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_seg_builder_ops.rs`
    - [ ] `tui/src/tui/layout/flex_box.rs`
    - [ ] `tui/src/tui/layout/props.rs`
    - [ ] `tui/src/tui/layout/surface.rs`
    - [ ] `tui/src/tui/md_parser/block/parse_fenced_code_block.rs`
    - [ ] `tui/src/tui/md_parser/block/parse_smart_list_block.rs`
    - [ ] `tui/src/tui/md_parser/convert_to_plain_text.rs`
    - [ ] `tui/src/tui/md_parser/extended/parse_metadata_kcsv.rs`
    - [ ] `tui/src/tui/md_parser/extended/parse_metadata_kv.rs`
    - [ ] `tui/src/tui/md_parser/fragment/parse_fragments_in_a_line.rs`
    - [ ] `tui/src/tui/md_parser/fragment/plain_parser_catch_all.rs`
    - [ ] `tui/src/tui/md_parser/parse_markdown.rs`
    - [ ] `tui/src/tui/md_parser/single_line/parse_heading_in_single_line.rs`
    - [ ] `tui/src/tui/md_parser/single_line/parse_markdown_text_in_single_line.rs`
    - [ ] `tui/src/tui/md_parser/single_line/parse_null_padded_line.rs`
    - [ ] `tui/src/tui/md_parser/single_line/take_text_between_in_single_line.rs`
    - [ ] `tui/src/tui/md_parser/single_line/take_text_in_single_line.rs`
    - [ ] `tui/src/tui/rsx/layout_macros.rs`
    - [ ] `tui/src/tui/syntax_highlighting/convert_syntect_to_styled_text.rs`
    - [ ] `tui/src/tui/syntax_highlighting/intermediate_types.rs`
    - [ ] `tui/src/tui/syntax_highlighting/md_parser_syn_hi/md_parser_stylesheet.rs`
    - [ ] `tui/src/tui/syntax_highlighting/md_parser_syn_hi/md_parser_syn_hi_impl.rs`
    - [ ] `tui/src/tui/syntax_highlighting/pattern_matcher.rs`
    - [ ] `tui/src/tui/syntax_highlighting/r3bl_syntect_theme.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/compositor_render_ops_to_ofs_buf.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/at_most_one_instance_assert.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/direct_to_ansi_output_integration_tests/color_operations.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/direct_to_ansi_output_integration_tests/cursor_movement.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/direct_to_ansi_output_integration_tests/state_optimization.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/direct_to_ansi_output_integration_tests/text_operations_rendered.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/direct_to_ansi_output_integration_tests/text_operations.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/pixel_char_renderer.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/tests.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/char_ops.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/core.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/diff_chunks.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/line_level_ops.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/paint_impl.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/pixel_char_line.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/pixel_char.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/impls/growable_buffer.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/render_op/render_op_ir.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/render_op/render_op_output.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/render_pipeline.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/render_tui_styled_texts.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/test_render_pipeline.rs`
    - [ ] `tui/src/tui/terminal_window/main_event_loop.rs`
    - [ ] `tui/src/tui/terminal_window/manage_focus/has_focus.rs`
    - [ ] `tui/src/tui/terminal_window/shared_global_data.rs`
    - [ ] `tui/src/tui/terminal_window/terminal_window_integration_tests/pty_main_event_loop_test.rs`

### Phase 7: Remove Allow and Update Docs

Remove the crate-level lint suppression in `tui/src/lib.rs`, align agent guidelines so
future refactors do not reintroduce `use crate::<macro>;`, and verify zero `r3bl_tui`
future-incompatibility warnings across all targets.

- [x] Remove `#![allow(macro_expanded_macro_exports_accessed_by_absolute_paths)]` and its
      comment block in `tui/src/lib.rs` (lines 3561-3565).
- [x] Update `AGENTS.md` (`### Macro Imports`, lines 608-613),
      `.agents/skills/organize-modules/SKILL.md` (`### Step 7: Macro Module Organization`,
      lines 374-410), and `.agents/skills/organize-modules/examples.md` (`## Example 7`,
      lines 758-794) to document `#[macro_use]` textual scoping for intra-crate
      `#[macro_export]` macros and forbid `use crate::<macro>;` and `$crate::<macro>!()`.
- [x] Run `./check.fish --fmt`, `./check.fish --check`, and
      `cargo check -p r3bl_tui --all-targets -q --message-format=short`; verify `r3bl_tui`
      no longer appears in `cargo report future-incompatibilities` (only upstream
      `nix v0.28.0` via `portable-pty v0.9.0` remains).
- [x] Run final pre-commit verification via background subagent (`./check.fish --clippy`,
      `./check.fish --quick-doc`, and `./check.fish --test`).
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/src/lib.rs`
    - [ ] `AGENTS.md`
    - [ ] `.agents/skills/organize-modules/SKILL.md`
    - [ ] `.agents/skills/organize-modules/examples.md`
    - [ ] `lychee.toml`

### Phase 8: Minimal #[macro_use] Scope and Uniform #[macro_export] Policy

Keep `#[macro_export]` uniformly on all shared and documented macros (so `///` macro docs
render, `no_run` doctests compile, and `[my_macro!]` links resolve cleanly without
`private_intra_doc_links`), keep `<domain>_macros` modules private, and trim over-lifted
`#[macro_use]` attributes so every macro propagates upward only to the lowest common
ancestor `mod.rs` of its internal callers.

- [x] Eliminate all generic `macros.rs` filenames: rename `constants/macros.rs` to
      `tui/src/core/ansi/constants/ansi_const_macros.rs` and `canvas_coords/macros.rs` to
      `tui/src/core/coordinates/canvas/canvas_coords/canvas_index_and_length_impl_macros.rs`,
      and flatten `core/decl_macros/` into `tui/src/core/decl_macros.rs`.
- [x] Keep `#[macro_export]` on all shared and documented macros (Option A), restore bare
      `[`define_ansi_const!`]` resolution in `tui/src/core/ansi/constants/esc.rs`, clean
      up crate-root reference links in `canvas_index_and_length_impl_macros.rs` and
      `index_and_length_impl_macros.rs`, and keep `[`queue_terminal_command!`]`:
      `crate::queue_terminal_command` in `tui/src/tui/mod.rs`.
- [x] Trim unnecessary `#[macro_use]` attributes across coordinator `mod.rs` files (and
      inline `convert_key_event` in `key_press.rs`) so each macro only lifts to the lowest
      common ancestor of its internal callers in `tui/src/`.
- [x] Fix stale 404 URL in `tui/src/tui/layout/layout_error.rs`
      (`e7.custom_error_types.html` to
      `https://learning-rust.github.io/docs/custom-error-types/`) surfaced by `lychee`.
- [x] Update `AGENTS.md`, `.agents/skills/organize-modules/SKILL.md`,
      `.agents/skills/organize-modules/examples.md`, and
      `.agents/skills/write-documentation/link-patterns.md` to document Option A (uniform
      `#[macro_export]` + minimal `#[macro_use]` lifting + bare macro intra-doc links +
      `<domain>_macros.rs` naming).
- [x] Run `./check.fish --full` and confirm zero workspace warnings and zero `r3bl_tui`
      entries in `cargo report future-incompatibilities`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase (and updated
      skill files) for correct implementation and ensure no regressions.
    - [ ] `tui/src/core/ansi/constants/ansi_const_macros.rs`
    - [ ] `tui/src/core/ansi/constants/mod.rs`
    - [ ] `tui/src/core/ansi/constants/esc.rs`
    - [ ] `tui/src/core/coordinates/canvas/canvas_coords/canvas_index_and_length_impl_macros.rs`
    - [ ] `tui/src/core/coordinates/canvas/canvas_coords/mod.rs`
    - [ ] `tui/src/core/coordinates/viewport_coords/index_and_length_impl_macros.rs`
    - [ ] `tui/src/core/coordinates/viewport_coords/mod.rs`
    - [ ] `tui/src/core/coordinates/canvas/mod.rs`
    - [ ] `tui/src/core/decl_macros.rs`
    - [ ] `tui/src/tui/editor/editor_engine/mod.rs`
    - [ ] `tui/src/tui/layout/layout_error.rs`
    - [ ] `tui/src/tui/mod.rs`
    - [ ] `tui/src/lib.rs`
    - [ ] `tui/src/core/mod.rs`
    - [ ] `tui/src/core/ansi/mod.rs`
    - [ ] `tui/src/core/ansi/generator/mod.rs`
    - [ ] `tui/src/core/coordinates/mod.rs`
    - [ ] `tui/src/core/log/mod.rs`
    - [ ] `tui/src/core/script/mod.rs`
    - [ ] `tui/src/core/stack_alloc_types/mod.rs`
    - [ ] `tui/src/core/terminal_io/key_press.rs`
    - [ ] `tui/src/readline_async/mod.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/mod.rs`
    - [ ] `tui/src/tui/rsx/mod.rs`
    - [ ] `tui/src/tui/layout/mod.rs`
    - [ ] `tui/src/tui/editor/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/crossterm_backend/mod.rs`
    - [ ] `AGENTS.md`
    - [ ] `.agents/skills/organize-modules/SKILL.md`
    - [ ] `.agents/skills/organize-modules/examples.md`
    - [ ] `.agents/skills/write-documentation/link-patterns.md`
    - [ ] `.agents/skills/remove-crate-prefix/SKILL.md`
    - [ ] `.agents/skills/run-clippy/SKILL.md`
    - [ ] `.agents/skills/organize-tests/examples.md`
    - [ ] `.agents/skills/fast-string-allocations/SKILL.md`

### Phase 9: Document Private Items Directly and Remove Conditional Visibility

Replace `#[cfg(any(test, doc))]` / `#[cfg(not(any(test, doc)))]` dual-arm conditional
visibility across the workspace by passing `--document-private-items` to `rustdoc` across
local checks, `cargo` config, and `docs.rs` metadata.

#### Phase 9.1: Enable `--document-private-items` Across Tooling and Crates

- [x] Add `--document-private-items` to `RUSTDOCFLAGS` in `check_docs.fish`
      (`run_cargo_doc`, line 75) alongside `--extend-css` so `./check.fish --quick-doc`,
      `--doc`, `--watch-doc`, and `--full` enable it (Cargo's `RUSTDOCFLAGS` env var
      overrides `.cargo/config.toml` `build.rustdocflags`).
- [x] Add `[build]` with `rustdocflags = ["--document-private-items"]` to
      `.cargo/config.toml` so direct `cargo doc` and IDE runs match `check.fish`, and add
      `[workspace.lints.rustdoc] private_intra_doc_links = "allow"` to root `Cargo.toml`.
- [x] Add `rustdoc-args = ["--document-private-items"]` under `[package.metadata.docs.rs]`
      in all 5 workspace member manifests (`tui/Cargo.toml`, `cmdr/Cargo.toml`,
      `build-infra/Cargo.toml`, `rust-analyzer-mcp-server/Cargo.toml`, and
      `analytics_schema/Cargo.toml`).
- [ ] **Mandatory manual review:** Verify every file modified in Phase 9.1.
    - [ ] `check_docs.fish`
    - [ ] `.cargo/config.toml`
    - [ ] `Cargo.toml`
    - [ ] `analytics_schema/Cargo.toml`
    - [ ] `tui/Cargo.toml`
    - [ ] `cmdr/Cargo.toml`
    - [ ] `build-infra/Cargo.toml`
    - [ ] `rust-analyzer-mcp-server/Cargo.toml`

#### Phase 9.2: Collapse Dual-Arm Conditional Visibility Across 29 Coordinator Files

- [x] Remove dual-arm conditional visibility (`#[cfg(any(test, doc))] pub mod foo;` paired
      with `#[cfg(not(any(test, doc)))] mod foo;`, including `cfg(all(unix, ...))` and
      `cfg(all(target_os = "linux", ...))` dual-arm variants), tightening 53 of 110
      declarations down to default plain `mod foo;` (for parent/sibling-only access) and
      keeping `pub(crate) mod foo;` on only the 57 declarations where cousin or ancestor
      paths require crate-internal access in code, tests, or `mod@crate::...` rustdoc
      links (preserving platform guards such as `#[cfg(unix)]` or
      `#[cfg(target_os = "linux")]`).
- [x] Remove newly exposed dead `unused_imports` in
      `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_framer/unit_tests/mod.rs` and
      `test_fixtures.rs` revealed when `chunk_framer` switched from `pub mod` under
      `cfg(test)` to `pub(crate) mod`.
- [x] Remove the `#[cfg(any(test, doc))] #[doc(inline)] pub use ...` search-fix shims in
      `tui/src/core/mod.rs` and `tui/src/core/ansi/mod.rs` that relied on private
      submodules being `pub` during doc builds, and update any intra-doc links targeting
      those shims.
- [x] Preserve single-arm `#[cfg(any(test, doc))]`, `#[cfg(any(all(unix, doc), test))]`,
      and `#[cfg_attr(not(doc), test)]` on test-only modules and helpers because
      `--document-private-items` does not enable `cfg(test)` and test modules must stay in
      the AST for `cargo doc` while excluded from release binaries (do not touch
      `build-infra/src/cargo_rustdoc_fmt/validation_tests/test_data/`).
- [ ] **Mandatory manual review:** Verify every file modified in Phase 9.2.
    - [ ] `tui/src/core/mod.rs`
    - [ ] `tui/src/core/ansi/mod.rs`
    - [ ] `tui/src/core/ansi/constants/mod.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/keyboard/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_framer/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_framer/accumulator/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_framer/unit_tests/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_framer/unit_tests/test_fixtures.rs`
    - [ ] `tui/src/core/common/fast_strings/mod.rs`
    - [ ] `tui/src/core/coordinates/bounds_check/range/mod.rs`
    - [ ] `tui/src/core/pty/pty_mux/mod.rs`
    - [ ] `tui/src/core/pty/pty_session/mod.rs`
    - [ ] `tui/src/core/pty/pty_session/events/mod.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/mod.rs`
    - [ ] `tui/src/core/script/mod.rs`
    - [ ] `tui/src/core/terminal_io/mod.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/mod.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/crossterm_backend/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/render_op/mod.rs`

#### Phase 9.3: Fix Newly Surfaced Private-Item Intra-Doc Link Warnings (63 Files, 98 Warnings)

- [x] Fix broken `[std::process::exit(0)]` intra-doc links on private `fn controlled()`
      functions across 34 PTY and isolated process integration test files (38 doc blocks)
      by adding ``[`std::process::exit(0)`]: std::process::exit`` at the bottom of each
      doc block.
- [x] Fix the remaining private-item unresolved, ambiguous (`prim@u16`, `prim@f64` in
      `ch_unit.rs`, and `fn@crate::parse_markdown` in `zcgb_adapters.rs`), and redundant
      link warnings across `r3bl_tui`, `r3bl-cmdr`, `r3bl-build-infra`, and
      `r3bl-rust-analyzer-mcp-server` (98 total warnings across 63 files surfaced by
      `--document-private-items`), verified with `./check.fish --quick-doc` (zero
      workspace warnings).
- [ ] **Mandatory manual review:** Verify every file modified in Phase 9.3.
    - [ ] `build-infra/src/cargo_rustdoc_fmt/formatters/technical_term/upgrade_term.rs`
    - [ ] `rust-analyzer-mcp-server/src/lsp/readiness_monitor.rs`
    - [ ] `cmdr/src/analytics_client/upgrade_check.rs`
    - [ ] `cmdr/src/self_upgrade/config/upgrade_policy.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/release_target.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/cargo_install_rustup_test.rs`
    - [ ] `tui/src/core/ansi/detect_color/detect_color_integration_tests/pty_test_color_detection.rs`
    - [ ] `tui/src/core/ansi/generator/sgr_code.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_core.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_unix.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_basic_enable_disable.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_flag_verification.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_input_behavior.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_multiple_cycles.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_poison_recovery.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/protocols/csi_codes/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/protocols/vte_params_ext.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/mouse.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_bracketed_paste_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_input_device_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_keyboard_modifiers_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_keypad_and_special_keys_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_mio_poller_singleton_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_mio_poller_subscribe_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_mio_poller_thread_lifecycle_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_mio_poller_thread_reuse_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_mouse_events_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_osc_color_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_sigwinch_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_terminal_events_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_utf8_text_test.rs`
    - [ ] `tui/src/core/common/std_thread_tokio_runner.rs`
    - [ ] `tui/src/core/coordinates/bounds_check/range/range_construct_ext.rs`
    - [ ] `tui/src/core/coordinates/primitives/ch_unit.rs`
    - [ ] `tui/src/core/pty/pty_mux/input_router/keyboard_command.rs`
    - [ ] `tui/src/core/pty/pty_mux/input_router/mouse_command.rs`
    - [ ] `tui/src/core/pty/pty_mux/output_renderer.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_integration_tests/double_panic_prevention_test.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_integration_tests/pty_test_production_factory_restart.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_integration_tests/pty_test_production_poll_error.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/capabilities_integration_tests/test_disclaimer.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/capabilities_integration_tests/test_piped_stdin.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/capabilities_integration_tests/test_piped_stdout.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/capabilities_integration_tests/test_pty_is_interactive.rs`
    - [ ] `tui/src/readline_async/choose_impl/choose_integration_tests/pty_shared_writer_pause_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_concurrent_input_output_deadlock_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_editor_state_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_multiline_output_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_readline_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_shared_writer_no_blank_line_test.rs`
    - [ ] `tui/src/readline_async/spinner_impl/spinner_integration_tests/pty_spinner_test.rs`
    - [ ] `tui/src/tui/dialog/dialog_engine/dialog_engine_api.rs`
    - [ ] `tui/src/tui/editor/editor_engine/engine_public_api.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_access_ops.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_adapters.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_insert_ops.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_seg_builder_ops.rs`
    - [ ] `tui/src/tui/md_parser/single_line/parse_heading_in_single_line.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/pixel_char_renderer.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/render_op/render_op_common.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/render_op/render_op_output.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/render_op/render_ops_local_data.rs`
    - [ ] `tui/src/tui/terminal_window/terminal_window_integration_tests/pty_main_event_loop_test.rs`

#### Phase 9.4: Update Agent Instructions and Skills & Run Final Verification

- [x] Replace dual-arm conditional visibility patterns (`#[cfg(any(test, doc))] pub mod` /
      `#[cfg(not(any(test, doc)))] mod`) with the `--document-private-items` + minimum
      `mod` vs `pub(crate) mod` workflow across agent instructions and skills, clarify
      preserving single-arm `#[cfg(any(test, doc))]` only for test-only AST inclusion, and
      add ``[`std::process::exit(0)`]: std::process::exit`` to the PTY test template in
      `organize-tests/examples.md`.
- [x] Fix 4 `clippy::unnecessary_to_owned` warnings surfaced by `nightly-2026-11-16`
      across `queries.rs`, `tui_apps/main.rs`, and `history.rs`.
- [x] Reinstall modified local tool binaries (`cargo install --path build-infra --force`
      and `cargo install --path rust-analyzer-mcp-server --force`) per crate `AGENTS.md`
      rules.
- [x] Run `./check.fish --full` and confirm zero warnings across all workspace crates
      (only upstream `nix v0.28.0` future-incompatibility notice remains).
- [ ] **Mandatory manual review:** Verify every file modified in Phase 9.4.
    - [ ] `AGENTS.md`
    - [ ] `.agents/AGENTS.md`
    - [ ] `.agents/skills/organize-modules/SKILL.md`
    - [ ] `.agents/skills/organize-modules/examples.md`
    - [ ] `.agents/skills/write-documentation/SKILL.md`
    - [ ] `.agents/skills/write-documentation/link-patterns.md`
    - [ ] `.agents/skills/run-clippy/SKILL.md`
    - [ ] `.agents/skills/run-clippy/patterns.md`
    - [ ] `.agents/skills/organize-tests/SKILL.md`
    - [ ] `.agents/skills/organize-tests/examples.md`
    - [ ] `rust-analyzer-mcp-server/src/lsp/queries.rs`
    - [ ] `tui/examples/tui_apps/main.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/history.rs`

### Phase 10: Tighten Barrel `pub mod` Submodules and Remove `#[doc(inline)]` Shims

Now that `--document-private-items` is enabled workspace-wide (Phase 9), internal
implementation submodules inside barrel-export coordinators (`pub use child::*;`) no
longer need `pub mod` to get `rustdoc` HTML pages, and parent coordinators no longer need
`#[doc(inline)]` re-export shims (originally added in
`task/done/barrel-export-docs-search-fix.md`) to prevent 404s in `rustdoc` HTML search
(`s`). Keeping barrel-exported implementation submodules `pub mod` leaks internal
file/folder layout into `r3bl_tui`'s public API, creates duplicate HTML pages and search
entries when paired with `#[doc(inline)]`, and risks `ambiguous_glob_reexports`
collisions.

**Module Visibility Boundary Policy for `r3bl_tui` (`v0.8.0`):**

1. **Keep `pub mod` ONLY where module hierarchy IS the intentional public API:**
    - **Level-1 crate domains** (`tui/src/lib.rs`): `core`, `network_io`,
      `readline_async`, `tui`.
    - **Level-2 documented domain hubs**:
        - In `tui/src/core/mod.rs`: `ansi`, `color_wheel`, `common`, `coordinates`,
          `glyphs`, `graphemes`, `log`, `notification`, `pty`, `resilient_reactor_thread`,
          `script`, `storage`, `terminal_io`, `tui_style`, `tui_styled_text`.
        - In `tui/src/tui/mod.rs`: `animator`, `cmd_line_args`, `dialog`, `editor`,
          `global_constants`, `layout`, `md_parser`, `syntax_highlighting`,
          `terminal_lib_backends`, `terminal_window`.
    - **Explicit non-globbed leaf namespaces** (modules whose parent intentionally omits
      `pub use <mod>::*;` because `mod_name::item` is the public API or required by public
      struct field types):
        - `tui/src/core/ansi/generator/mod.rs`: `mouse_sgr`, `mouse_x10`, `ansi_output`
          (and inline sub-namespaces `cursor_movement`, `screen_clearing`, `color_ops`,
          `cursor_visibility`, `cursor_save_restore`, `terminal_modes`,
          `estimate_capacity` in `ansi_output.rs`).
        - `tui/src/core/ansi/detect_color/detect_color_support.rs`:
          `global_color_support`, `global_hyperlink_support`.
        - `tui/src/core/tui_style/tui_style_attribs.rs`: `tui_style_attrib`.
        - `tui/src/core/color_wheel/color_wheel_config.rs`: `defaults`.
        - `tui/src/core/stack_alloc_types/into_existing.rs`: `read_from_file`,
          `write_to_file`.
        - `tui/src/core/stack_alloc_types/fast_int_fmt.rs`: `usize_impl`, `u16_impl`.
        - `tui/src/core/stack_alloc_types/list_of.rs`: `sizing_list_of`.
        - `tui/src/core/common/telemetry.rs`: `telemetry_sizing`,
          `telemetry_default_constants`, `create`.
        - `tui/src/core/log/custom_event_formatter.rs`:
          `custom_event_formatter_constants`.
        - `tui/src/core/storage/kv.rs`: `kv_error`.
        - `tui/src/core/ansi/vt_100_pty_output_parser/modes.rs`:
          `terminal_mode_state_todo` (houses `BracketedPasteMode` used on a `pub` field of
          `TerminalModeState`).
        - `tui/src/network_io/length_prefix_protocol.rs`: `protocol_constants`,
          `handshake`, `byte_io`.
2. **Tighten all barrel-exported implementation submodules (`pub use child::*;`),
   `impl`-only inner modules, macro modules, and test/fixture modules** to minimum
   internal scope (`mod` by default; lift to `pub(crate) mod` only when cross-folder
   `crate::...` code, tests, or `mod@crate::...` rustdoc links require crate-wide path
   access).
3. **Preserve golden fixtures:** Do not modify
   `build-infra/src/cargo_rustdoc_fmt/validation_tests/test_data/`.

#### Phase 10.1: Targeted Cleanup of `#[doc(inline)]` Coordinators (`coordinates`, `graphemes`, `pty`, `md_parser`)

Eliminate all 9 `#[doc(inline)]` search-link workarounds across 5 coordinator `mod.rs`
files by tightening the underlying `pub mod` subtrees to `mod` / `pub(crate) mod` and
migrating remaining qualified paths in code, examples, doctests, and rustdoc links.

- [x] **`coordinates` subtree & `core/mod.rs`:**
    - In `tui/src/core/coordinates/mod.rs`: tighten `viewport_coords`, `canvas`,
      `percent_spec`, `bounds_check`, `byte`, `primitives`, and `vt_100_ansi_coords` from
      `pub mod` to minimum internal scope (`pub(crate) mod` for `bounds_check` and
      `canvas` which are cross-folder doc link hubs; `mod` for the rest unless
      cross-folder links require `pub(crate) mod`), update the legacy qualified-paths
      comment at lines 239-242, and delete both `#[doc(inline)]` blocks at lines 277-287.
    - Tighten child `pub mod` declarations to `mod` (or `pub(crate) mod` where needed) in:
      `tui/src/core/coordinates/bounds_check/mod.rs` (8 modules; also update lines
      1219-1226, 1264 to relative `mod@range::...`),
      `tui/src/core/coordinates/viewport_coords/mod.rs` (10 modules),
      `tui/src/core/coordinates/byte/mod.rs` (3 modules),
      `tui/src/core/coordinates/percent_spec/mod.rs` (`pc`, `req_size_pc`),
      `tui/src/core/coordinates/primitives/mod.rs` (`ch_unit`),
      `tui/src/core/coordinates/primitives/ch_unit.rs` (3 inline `impl` modules), and
      `tui/src/core/coordinates/canvas/viewport.rs` (`test_fixture_viewport`).
    - Flatten intra-crate `bounds_check::...` / `vt_100_ansi_coords::...` /
      `percent_spec::...` / `viewport_coords::...` imports and rustdoc links to flat
      `crate::...` (or canonical `mod@crate::core::coordinates::bounds_check` for
      module-level doc links) across `byte_{index,length,offset}.rs`,
      `vt_100_impl_{char,cursor,line,margin,mode,scroll}_ops.rs`, `render_to_ansi.rs`,
      `zcgb_access_ops.rs`, `ofs_buf/mod.rs`, `line_state/mod.rs`, `mouse.rs`,
      `cursor_bounds_check.rs`, `numeric_value.rs`, `result_enums.rs`,
      `viewport_bounds_check.rs`, `range_{bounds_check,construct_ext,convert_ext}.rs`,
      `vt_100_ansi_coords/mod.rs`, `percent_spec/mod.rs`, `tui/src/lib.rs`, and
      `tui/README.md`.
- [x] **`graphemes` subtree & `core/mod.rs`:**
    - In `tui/src/core/graphemes/mod.rs`: tighten `gc_string`, `traits`,
      `unicode_segment`, and `word_boundaries` from `pub mod` to `mod` / `pub(crate) mod`,
      and delete all 3 `#[doc(inline)]` blocks at lines 281-289.
    - Tighten child `pub mod` declarations to `mod` in
      `tui/src/core/graphemes/gc_string/mod.rs` (`owned`),
      `tui/src/core/graphemes/gc_string/owned/mod.rs` (`gc_string_owned`,
      `gc_string_owned_non_editor_impl`, and remove now-unused
      `pub use gc_string_owned_non_editor_impl::*;`),
      `tui/src/core/graphemes/gc_string/owned/gc_string_owned.rs` (`at_display_col_index`,
      `mutate`),
      `tui/src/core/graphemes/gc_string/owned/gc_string_owned_non_editor_impl.rs`
      (`convert`, `trunc_end`, `trunc_start`), `tui/src/core/graphemes/traits/mod.rs` (3
      modules), and `tui/src/core/graphemes/unicode_segment/mod.rs` (4 modules).
    - Flatten `segment_builder::...` imports in `zcgb_basic_ops.rs` and
      `zcgb_seg_builder_ops.rs` to flat `crate::...`.
    - Migrate the 5 doctests in `tui/src/core/graphemes/word_boundaries/mod.rs` (line 16)
      and `tui/src/core/graphemes/word_boundaries/word_boundary_detection.rs` (lines 14,
      70, 157, 233) from `r3bl_tui::core::graphemes::word_boundaries::...` to flat
      `r3bl_tui::...`, and update `mod@crate::graphemes::gc_string` in `buffer_struct.rs`.
- [x] **`pty` subtree & `core/mod.rs`:**
    - In `tui/src/core/pty/mod.rs`: tighten `pty_engine`, `pty_mux`, and `pty_session`
      from `pub mod` to `pub(crate) mod`, delete
      `#[doc(inline)] pub use pty_session::threads;` (lines 503-506), and update internal
      item links (`PtyPair`, `spawn_pty_*_thread`) to flat `crate::...` or canonical
      `mod@crate::core::pty::...`.
    - Tighten child `pub mod` declarations to `mod` / `pub(crate) mod` in
      `tui/src/core/pty/pty_engine/mod.rs` (4 modules),
      `tui/src/core/pty/pty_session/mod.rs` (`threads`),
      `tui/src/core/pty/pty_session/threads/mod.rs` (3 modules),
      `tui/src/core/pty/pty_mux/output_renderer.rs` (2 inline helper modules), and
      `tui/src/core/pty/e2e_tests/mod.rs` (6 test modules), and flatten
      `threads::orchestrator::spawn_pty_orchestrator_thread` in `pty_session/builder.rs`
      to `super::spawn_pty_orchestrator_thread`.
    - Remove all 3 `#[doc(inline)]` blocks and comment banner at lines 60-71 of
      `tui/src/core/mod.rs`.
    - Migrate external/doctest/example `pty_mux` paths to flat
      `r3bl_tui::{PTYMux, ProcessManager}` in `tui/examples/pty_mux_example.rs`,
      `tui/examples/pty_2d_panning_example.rs`, `tui/src/core/pty/pty_mux/mux.rs`,
      `tui/src/core/pty/pty_mux/mod.rs`, `tui/src/lib.rs`, and `tui/README.md`, and update
      intra-crate `crate::pty_mux` / `crate::pty_session` / `crate::pty_engine` rustdoc
      links to flat `crate::...` items or canonical `mod@crate::core::pty::...` paths.
- [x] **`md_parser` subtree & `tui/mod.rs`:**
    - In `tui/src/tui/mod.rs`: delete `#[doc(inline)] pub use md_parser::{...};` at lines
      138-142.
    - In `tui/src/tui/md_parser/mod.rs` and child coordinators (`block/mod.rs`,
      `extended/mod.rs`, `fragment/mod.rs`, `single_line/mod.rs`,
      `conformance_test_data/mod.rs`): tighten all barrel `pub mod` and test `pub mod`
      declarations to `mod` / `pub(crate) mod`.
    - Flatten `md_parser::md_parser_constants::...`, `parse_null_padded_line::...`,
      `specialized_parser_delim_matchers`, and `md_parser::conformance_test_data::*`
      imports across `md_parser_syn_hi_impl.rs`, `intermediate_types.rs`,
      `take_text_in_single_line.rs`, `take_text_between_in_single_line.rs`,
      `specialized_parsers.rs`, `specialized_parser_delim_matchers.rs`,
      `plain_parser_catch_all.rs`, `parse_fenced_code_block.rs`,
      `parse_smart_list_block.rs`, `parse_metadata_kcsv.rs`, `parse_metadata_kv.rs`,
      `parse_markdown.rs`, `parse_heading_in_single_line.rs`,
      `parse_markdown_text_in_single_line.rs`, `md_parser_types.rs`,
      `parser_snapshot_tests.rs`, `parser_bench_tests.rs`, `choose_api.rs`,
      `zcgb_adapters.rs`, `editor_event.rs`, and `common_math.rs` to flat `crate::...` (or
      `super::...` / `super::super::...` inside `md_parser/`).
- [x] Verify zero `#[doc(inline)]` attributes remain anywhere in the workspace
      (`rg '#\[doc\(inline\)\]'`), and run `./check.fish --check` and
      `./check.fish --fmt`.
- [ ] **Mandatory manual review:** Verify every file modified in Phase 10.1.
    - [ ] `tui/README.md`
    - [ ] `tui/examples/pty_2d_panning_example.rs`
    - [ ] `tui/examples/pty_mux_example.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_basic_enable_disable.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_flag_verification.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_input_behavior.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_poison_recovery.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_unix.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ansi_parser_public_api.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100/core.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_char_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_cursor_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_line_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_margin_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_mode_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_scroll_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/pty_response_event.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/mouse.rs`
    - [ ] `tui/src/core/common/common_enums.rs`
    - [ ] `tui/src/core/common/common_math.rs`
    - [ ] `tui/src/core/common/flat_2d_array/core.rs`
    - [ ] `tui/src/core/coordinates/bounds_check/array_bounds_check.rs`
    - [ ] `tui/src/core/coordinates/bounds_check/cursor_bounds_check.rs`
    - [ ] `tui/src/core/coordinates/bounds_check/mod.rs`
    - [ ] `tui/src/core/coordinates/bounds_check/numeric_value.rs`
    - [ ] `tui/src/core/coordinates/bounds_check/range/range_bounds_check.rs`
    - [ ] `tui/src/core/coordinates/bounds_check/range/range_construct_ext.rs`
    - [ ] `tui/src/core/coordinates/bounds_check/range/range_convert_ext.rs`
    - [ ] `tui/src/core/coordinates/bounds_check/result_enums.rs`
    - [ ] `tui/src/core/coordinates/bounds_check/viewport_bounds_check.rs`
    - [ ] `tui/src/core/coordinates/byte/byte_index.rs`
    - [ ] `tui/src/core/coordinates/byte/byte_length.rs`
    - [ ] `tui/src/core/coordinates/byte/byte_offset.rs`
    - [ ] `tui/src/core/coordinates/byte/mod.rs`
    - [ ] `tui/src/core/coordinates/canvas/canvas_range_ext.rs`
    - [ ] `tui/src/core/coordinates/canvas/mod.rs`
    - [ ] `tui/src/core/coordinates/canvas/viewport.rs`
    - [ ] `tui/src/core/coordinates/mod.rs`
    - [ ] `tui/src/core/coordinates/percent_spec/mod.rs`
    - [ ] `tui/src/core/coordinates/primitives/ch_unit.rs`
    - [ ] `tui/src/core/coordinates/primitives/mod.rs`
    - [ ] `tui/src/core/coordinates/viewport_coords/mod.rs`
    - [ ] `tui/src/core/coordinates/vt_100_ansi_coords/mod.rs`
    - [ ] `tui/src/core/graphemes/gc_string/mod.rs`
    - [ ] `tui/src/core/graphemes/gc_string/owned/gc_string_owned.rs`
    - [ ] `tui/src/core/graphemes/gc_string/owned/gc_string_owned_non_editor_impl.rs`
    - [ ] `tui/src/core/graphemes/gc_string/owned/mod.rs`
    - [ ] `tui/src/core/graphemes/mod.rs`
    - [ ] `tui/src/core/graphemes/traits/mod.rs`
    - [ ] `tui/src/core/graphemes/unicode_segment/mod.rs`
    - [ ] `tui/src/core/graphemes/word_boundaries/mod.rs`
    - [ ] `tui/src/core/graphemes/word_boundaries/word_boundary_detection.rs`
    - [ ] `tui/src/core/mod.rs`
    - [ ] `tui/src/core/pty/e2e_tests/mod.rs`
    - [ ] `tui/src/core/pty/mod.rs`
    - [ ] `tui/src/core/pty/pty_engine/mod.rs`
    - [ ] `tui/src/core/pty/pty_engine/pty_pair.rs`
    - [ ] `tui/src/core/pty/pty_mux/input_router/mouse_command.rs`
    - [ ] `tui/src/core/pty/pty_mux/mod.rs`
    - [ ] `tui/src/core/pty/pty_mux/mux.rs`
    - [ ] `tui/src/core/pty/pty_mux/output_renderer.rs`
    - [ ] `tui/src/core/pty/pty_session/builder.rs`
    - [ ] `tui/src/core/pty/pty_session/config.rs`
    - [ ] `tui/src/core/pty/pty_session/mod.rs`
    - [ ] `tui/src/core/pty/pty_session/session.rs`
    - [ ] `tui/src/core/pty/pty_session/threads/mod.rs`
    - [ ] `tui/src/core/pty/pty_session/threads/orchestrator.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/mod.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_integration_tests/double_panic_prevention_test.rs`
    - [ ] `tui/src/core/terminal_io/backpressure_stdout/backpressure_stdout_struct.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/capabilities_public_api.rs`
    - [ ] `tui/src/core/test_fixtures/pty_test_fixtures/generate_pty_test.rs`
    - [ ] `tui/src/core/test_fixtures/pty_test_fixtures/pty_test_child/buf_read_ext.rs`
    - [ ] `tui/src/core/test_fixtures/pty_test_fixtures/pty_test_child/pty_test_child_impl.rs`
    - [ ] `tui/src/core/test_fixtures/pty_test_fixtures/pty_test_child/pty_test_child_types.rs`
    - [ ] `tui/src/core/test_fixtures/pty_test_fixtures/spawn_controlled_in_pty.rs`
    - [ ] `tui/src/lib.rs`
    - [ ] `tui/src/readline_async/choose_api.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/line_state/mod.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_editor_state_test.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/buffer_struct.rs`
    - [ ] `tui/src/tui/editor/editor_component/editor_event.rs`
    - [ ] `tui/src/tui/editor/editor_engine/validate_buffer_mut.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_access_ops.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_adapters.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_basic_ops.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_line_metadata.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_seg_builder_ops.rs`
    - [ ] `tui/src/tui/md_parser/block/mod.rs`
    - [ ] `tui/src/tui/md_parser/block/parse_fenced_code_block.rs`
    - [ ] `tui/src/tui/md_parser/block/parse_smart_list_block.rs`
    - [ ] `tui/src/tui/md_parser/conformance_test_data/mod.rs`
    - [ ] `tui/src/tui/md_parser/extended/mod.rs`
    - [ ] `tui/src/tui/md_parser/extended/parse_metadata_kcsv.rs`
    - [ ] `tui/src/tui/md_parser/extended/parse_metadata_kv.rs`
    - [ ] `tui/src/tui/md_parser/fragment/mod.rs`
    - [ ] `tui/src/tui/md_parser/fragment/parse_fragments_in_a_line.rs`
    - [ ] `tui/src/tui/md_parser/fragment/plain_parser_catch_all.rs`
    - [ ] `tui/src/tui/md_parser/fragment/specialized_parser_delim_matchers.rs`
    - [ ] `tui/src/tui/md_parser/fragment/specialized_parsers.rs`
    - [ ] `tui/src/tui/md_parser/md_parser_types.rs`
    - [ ] `tui/src/tui/md_parser/mod.rs`
    - [ ] `tui/src/tui/md_parser/parse_markdown.rs`
    - [ ] `tui/src/tui/md_parser/parser_bench_tests.rs`
    - [ ] `tui/src/tui/md_parser/parser_snapshot_tests.rs`
    - [ ] `tui/src/tui/md_parser/single_line/mod.rs`
    - [ ] `tui/src/tui/md_parser/single_line/parse_heading_in_single_line.rs`
    - [ ] `tui/src/tui/md_parser/single_line/parse_markdown_text_in_single_line.rs`
    - [ ] `tui/src/tui/md_parser/single_line/take_text_between_in_single_line.rs`
    - [ ] `tui/src/tui/md_parser/single_line/take_text_in_single_line.rs`
    - [ ] `tui/src/tui/mod.rs`
    - [ ] `tui/src/tui/syntax_highlighting/intermediate_types.rs`
    - [ ] `tui/src/tui/syntax_highlighting/md_parser_syn_hi/md_parser_syn_hi_impl.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/input_device_impl.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/render_to_ansi.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/mod.rs`

#### Phase 10.2: Crate-Wide `r3bl_tui` Barrel `pub mod` Audit and Tightening

Audit and tighten all remaining internal barrel-exported `pub mod` declarations,
`impl`-only inner modules, macro modules, and test/fixture modules across `tui/src/` so
only intentional domain hubs and explicit non-globbed namespaces remain `pub mod`.

- [x] **Update `$crate::` macro body paths first** so tightening internal barrels does not
      break external macro expansions:
    - `tui/src/core/ansi/constants/ansi_const_macros.rs` (lines 35, 50, 66, 81, 97, 113):
      replace `$crate::core::ansi::constants::{CSI_START, ESC_STR, OSC_START}` with
      `$crate::{CSI_START, ESC_STR, OSC_START}`.
    - `tui/src/core/stack_alloc_types/fast_int_fmt.rs` (lines 48-49, 91-92): replace
      `$crate::core::stack_alloc_types::{u16_impl, usize_impl}::...` with
      `$crate::{u16_impl, usize_impl}::...`, and update doctests at lines 199 and 251 to
      `r3bl_tui::u16_impl`.
    - `tui/src/core/coordinates/canvas/canvas_coords/canvas_index_and_length_impl_macros.rs`
      (lines 42, 258): replace
      `use $crate::core::common::primitive_casting::WideningCastToUsize;` with
      `use $crate::WideningCastToUsize;`.
- [x] **Tighten `core` internal barrels, inline helpers, and test modules:**
    - `tui/src/core/mod.rs`: tighten pure macro/internal barrels (`decl_macros` to `mod`;
      `stack_alloc_types`, `heap_alloc_types`, `misc`, `test_fixtures` to `mod` /
      `pub(crate) mod`) while keeping the 15 documented Level-2 domain hubs `pub mod`.
    - Tighten `pub(crate) mod generator;` to plain `mod generator;` in
      `tui/src/core/ansi/mod.rs` (lines 251-258): flatten its already re-exported test
      helpers into `use crate::{...}` in
      `tui/src/core/terminal_io/backend_compat_tests/backend_compat_input_test.rs` (lines
      105-109: `alt_key`, `csi`, `csi_modified`, `csi_tilde`, `csi_u`, `ss3`) and
      `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_ctrl_navigation_test.rs`
      (lines 30-37: `generate_keyboard_sequence`), and update the 2 descendant rustdoc
      links ``[`generator`]: crate::generator`` to
      ``[`generator`]: mod@crate::core::ansi::generator`` in
      `tui/src/core/ansi/vt_100_terminal_input_parser/validation_tests/mod.rs` (line 55)
      and `input_parser_validation_test.rs` (line 148).
    - Tighten barrel & internal `pub mod` declarations to `mod` / `pub(crate) mod` (and
      demote any `pub(crate) mod` barrels whose cousin callers only imported already
      re-exported items) across: `tui/src/core/ansi/mod.rs` (`constants`, `detect_color`,
      `osc`, `vt_100_terminal_input_parser`), `tui/src/core/ansi/detect_color/mod.rs`,
      `tui/src/core/ansi/osc/mod.rs` (and update `osc_hyperlink.rs` doctests + `cmdr`
      `r3bl_tui::osc_codes::OscSequence` doc links to `r3bl_tui::OscSequence`),
      `tui/src/core/ansi/color/convert.rs` (`ansi_constants`),
      `tui/src/core/ansi/generator/cli_text.rs` & `generator/mod.rs`
      (`impl_cli_text_inline` to `pub(crate)`),
      `tui/src/core/ansi/vt_100_pty_output_parser/mod.rs` & `ops/mod.rs` &
      `protocols/vte_params_ext.rs` & `vt_100_pty_output_conformance_tests/**`,
      `tui/src/core/ansi/vt_100_terminal_input_parser/**`
      (`chunk_decoder/terminal_events.rs`, `chunk_framer/**`, `unit_tests/**`,
      `validation_tests/**`, `vt_100_parser_integration_tests/**`),
      `tui/src/core/ansi/terminal_raw_mode/mod.rs`,
      `tui/src/core/color_wheel/gradients/mod.rs`, `tui/src/core/common/mod.rs` (23
      submodules) & `fast_strings/mod.rs` & `fast_strings/hex_slice.rs` (`codec`) &
      `flat_2d_array/mod.rs`, `tui/src/core/heap_alloc_types/mod.rs`,
      `tui/src/core/log/mod.rs` & `log_integration_tests/mod.rs`,
      `tui/src/core/misc/mod.rs`, `tui/src/core/resilient_reactor_thread/**` (test
      modules), `tui/src/core/script/command_impl/mod.rs` (and `command_runner.rs`
      doctests) & `env_source/**` & `git/**`, `tui/src/core/stack_alloc_types/mod.rs` (and
      update `cmdr` `into_existing::{read_from_file, write_to_file}` imports in
      `cmdr/src/edi/state.rs` and `cmdr/src/analytics_client/proxy_machine_id.rs` to flat
      `r3bl_tui::{read_from_file, write_to_file}`), `tui/src/core/storage/mod.rs`,
      `tui/src/core/terminal_io/**` (`key_press.rs` inline helpers, `capabilities/mod.rs`,
      `backend_compat_tests/**`), `tui/src/core/test_fixtures/**`,
      `tui/src/core/tui_style/mod.rs`, and `tui/src/core/tui_styled_text/mod.rs`.
- [x] **Tighten `network_io`, `readline_async`, and `tui` internal barrels, inline
      helpers, and test modules:**
    - `tui/src/network_io/mod.rs`: tighten `compress`, `json_serde`,
      `length_prefix_protocol`, and `protocol_types` to `mod` / `pub(crate) mod` (keeping
      inline `protocol_constants`, `handshake`, `byte_io` in `length_prefix_protocol.rs`
      as `pub mod`).
    - `tui/src/readline_async/mod.rs`, `choose_impl/mod.rs`, `spinner_impl/mod.rs`, and
      integration test coordinators: tighten all `*_impl`, `*_api`, `spinner`,
      `modal_terminal_guard`, and child implementation/test modules to `mod` /
      `pub(crate) mod`; migrate `cmdr/src/common/ui_templates.rs`
      (`r3bl_tui::choose_impl::Header`), `tui/examples/choose_interactive.rs` &
      `tui/examples/mouse_inspector.rs` (`style::StyleSheet`),
      `tui/src/readline_async/readline_async_impl/line_state/event_handlers.rs`
      (`readline_async_impl::History`), and
      `tui/src/tui/terminal_window/shared_global_data.rs`
      (`spinner_impl::spinner_render::...`) to flat re-exports.
    - `tui/src/tui/mod.rs` (`rsx` to `#[macro_use] mod rsx;`), `tui/src/tui/rsx/mod.rs`
      (`layout_macros`, `render_macros` to `mod`), `tui/src/tui/animator/mod.rs`,
      `tui/src/tui/cmd_line_args/mod.rs`, `tui/src/tui/dialog/**`, `tui/src/tui/editor/**`
      (`editor_buffer/mod.rs`, `editor_component/**`, `editor_engine/**`,
      `test_fixtures_editor.rs`), `tui/src/tui/layout/mod.rs`,
      `tui/src/tui/syntax_highlighting/**`, `tui/src/tui/terminal_lib_backends/**`
      (`input_device_impl.rs` `global_input_resource` to `pub(crate) mod`, `paint_impl.rs`
      `render_helper` to `mod`, `growable_buffer.rs` fixture mod, test/stub mods), and
      `tui/src/tui/terminal_window/**`.
- [x] **Standardize coordinator `mod.rs` headers & inline `pub(crate) mod` reasons:**
    - Restore canonical `// Attach.` and `// Re-export.` section headers across modified
      coordinator `mod.rs` files (replacing fragmented `// Public modules.` /
      `// Crate-internal modules...` / `// Private modules...` headers from Phase 9.2),
      and split `#[macro_use]` coordinators into two blocks
      (`// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their macros).`
      followed by `// Attach.`).
    - Add a specific 1-line inline comment on every `pub(crate) mod` declaration stating
      the exact reason it requires **Crate-wide (Intra-crate)** visibility instead of
      **Folder-local (Intra-module)** `mod` (e.g., cross-folder `rustdoc` module-page
      links vs. un-re-exported crate-internal items/tests).
- [x] Run `./check.fish --check` and `./check.fish --fmt` after each batch of module trees
      (pausing for milestone review per `AGENTS.md`).
- [ ] **Mandatory manual review:** Verify every file modified in Phase 10.2.
    - [ ] `cmdr/src/analytics_client/proxy_machine_id.rs`
    - [ ] `cmdr/src/analytics_client/upgrade_check.rs`
    - [ ] `cmdr/src/common/ui_templates.rs`
    - [ ] `cmdr/src/edi/state.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/build.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/perform_upgrade.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/cargo_install_rustup_test.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade_journey/integration_tests/mod.rs`
    - [ ] `tui/README.md`
    - [ ] `tui/examples/choose_interactive.rs`
    - [ ] `tui/examples/mouse_inspector.rs`
    - [ ] `tui/examples/pty_2d_panning_example.rs`
    - [ ] `tui/examples/pty_mux_example.rs`
    - [ ] `tui/examples/spawn_pty_interactive.rs`
    - [ ] `tui/examples/spawn_pty_output_capture.rs`
    - [ ] `tui/examples/spinner.rs`
    - [ ] `tui/src/core/ansi/color/convert.rs`
    - [ ] `tui/src/core/ansi/color/rgb_value.rs`
    - [ ] `tui/src/core/ansi/constants/ansi_const_macros.rs`
    - [ ] `tui/src/core/ansi/constants/csi.rs`
    - [ ] `tui/src/core/ansi/constants/da.rs`
    - [ ] `tui/src/core/ansi/constants/dsr.rs`
    - [ ] `tui/src/core/ansi/constants/esc.rs`
    - [ ] `tui/src/core/ansi/constants/generic.rs`
    - [ ] `tui/src/core/ansi/constants/input_sequences.rs`
    - [ ] `tui/src/core/ansi/constants/mod.rs`
    - [ ] `tui/src/core/ansi/constants/mouse.rs`
    - [ ] `tui/src/core/ansi/constants/osc_constants.rs`
    - [ ] `tui/src/core/ansi/constants/raw_mode_constants.rs`
    - [ ] `tui/src/core/ansi/constants/sgr.rs`
    - [ ] `tui/src/core/ansi/constants/utf8.rs`
    - [ ] `tui/src/core/ansi/detect_color/detect_color_integration_tests/mod.rs`
    - [ ] `tui/src/core/ansi/detect_color/detect_color_support.rs`
    - [ ] `tui/src/core/ansi/detect_color/mod.rs`
    - [ ] `tui/src/core/ansi/generator/ansi_input_test_fixtures.rs`
    - [ ] `tui/src/core/ansi/generator/ansi_output.rs`
    - [ ] `tui/src/core/ansi/generator/cli_text.rs`
    - [ ] `tui/src/core/ansi/generator/dsr.rs`
    - [ ] `tui/src/core/ansi/generator/mod.rs`
    - [ ] `tui/src/core/ansi/mod.rs`
    - [ ] `tui/src/core/ansi/osc/mod.rs`
    - [ ] `tui/src/core/ansi/osc/osc_codes.rs`
    - [ ] `tui/src/core/ansi/osc/osc_color.rs`
    - [ ] `tui/src/core/ansi/osc/osc_hyperlink.rs`
    - [ ] `tui/src/core/ansi/osc/osc_pty_event.rs`
    - [ ] `tui/src/core/ansi/osc/osc_sender.rs`
    - [ ] `tui/src/core/ansi/osc/pty_osc_progress_scanner.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/mod.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_core.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_basic_enable_disable.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_flag_verification.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_multiple_cycles.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_poison_recovery.rs`
    - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_unix.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ansi_parser_public_api.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100/accessors.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_char_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_control_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_cursor_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_da_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_dsr_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_line_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_margin_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_mode_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_osc_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_scroll_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_sgr_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_terminal_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_ansi_panning.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_margin_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_osc_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_scroll_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/protocols/csi_codes/sgr_color_sequences.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/protocols/vte_osc_params.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/protocols/vte_params_ext.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/pty_response_event.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/conformance_data/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/test_sequence_generators/extended_color_builders.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/test_sequence_generators/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_char_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_control_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_cursor_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_line_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_margin_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_mode_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_osc_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_scroll_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_sgr_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_system_error_handling.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_system_state_management.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_terminal_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/chunk_decoder_entry_point.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/keyboard/csi_u.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/keyboard/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/mouse.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/osc_scanner.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/terminal_events.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/utf8.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_framer/accumulator/accumulator_struct.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_framer/accumulator/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_framer/accumulator/unparsed_buffer_action.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_framer/chunk_framer_struct.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_framer/circuit_breaker/circuit_breaker_struct.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_framer/circuit_breaker/drain_reason.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_framer/circuit_breaker/drain_result.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_framer/circuit_breaker/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_framer/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_framer/unit_tests/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_framer/unit_tests/osc_and_unrecognized_tests.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_framer/unit_tests/test_fixtures.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/ir_event_types.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/maybe_more.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/unit_tests/generator_round_trip_tests.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/unit_tests/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/validation_tests/input_parser_validation_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/validation_tests/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_osc_safety_test.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_osc_sender_test.rs`
    - [ ] `tui/src/core/color_wheel/gradients/mod.rs`
    - [ ] `tui/src/core/common/fast_strings/hex_slice.rs`
    - [ ] `tui/src/core/common/fast_strings/mod.rs`
    - [ ] `tui/src/core/common/flat_2d_array/mod.rs`
    - [ ] `tui/src/core/common/memoized_value.rs`
    - [ ] `tui/src/core/common/mod.rs`
    - [ ] `tui/src/core/common/scoped_mutex/mod.rs`
    - [ ] `tui/src/core/coordinates/bounds_check/range/mod.rs`
    - [ ] `tui/src/core/coordinates/canvas/canvas_coords/canvas_index_and_length_impl_macros.rs`
    - [ ] `tui/src/core/coordinates/canvas/canvas_coords/mod.rs`
    - [ ] `tui/src/core/coordinates/canvas/viewport.rs`
    - [ ] `tui/src/core/coordinates/mod.rs`
    - [ ] `tui/src/core/coordinates/percent_spec/mod.rs`
    - [ ] `tui/src/core/coordinates/viewport_coords/mod.rs`
    - [ ] `tui/src/core/graphemes/mod.rs`
    - [ ] `tui/src/core/heap_alloc_types/mod.rs`
    - [ ] `tui/src/core/log/custom_event_formatter.rs`
    - [ ] `tui/src/core/log/log_integration_tests/mod.rs`
    - [ ] `tui/src/core/log/mod.rs`
    - [ ] `tui/src/core/log/tracing_init.rs`
    - [ ] `tui/src/core/misc/mod.rs`
    - [ ] `tui/src/core/mod.rs`
    - [ ] `tui/src/core/pty/e2e_tests/cross_platform_commands.rs`
    - [ ] `tui/src/core/pty/e2e_tests/osc_capture_test.rs`
    - [ ] `tui/src/core/pty/mod.rs`
    - [ ] `tui/src/core/pty/pty_engine/mod.rs`
    - [ ] `tui/src/core/pty/pty_mux/mod.rs`
    - [ ] `tui/src/core/pty/pty_mux/process_manager.rs`
    - [ ] `tui/src/core/pty/pty_session/builder.rs`
    - [ ] `tui/src/core/pty/pty_session/config.rs`
    - [ ] `tui/src/core/pty/pty_session/events/mod.rs`
    - [ ] `tui/src/core/pty/pty_session/mod.rs`
    - [ ] `tui/src/core/pty/pty_session/threads/reader.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/mod.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/process_isolated_tests/fixtures.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/process_isolated_tests/mod.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_integration_tests/mod.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_integration_tests/smart_sender_test.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/unit_tests/mod.rs`
    - [ ] `tui/src/core/script/command_impl/command_run_result.rs`
    - [ ] `tui/src/core/script/command_impl/command_runner.rs`
    - [ ] `tui/src/core/script/command_impl/mod.rs`
    - [ ] `tui/src/core/script/env_source/args.rs`
    - [ ] `tui/src/core/script/env_source/conformance_tests/mod.rs`
    - [ ] `tui/src/core/script/env_source/conformance_tests/test_data/mod.rs`
    - [ ] `tui/src/core/script/env_source/conformance_tests/test_fixtures.rs`
    - [ ] `tui/src/core/script/env_source/conformance_tests/tests/test_golden_formatters.rs`
    - [ ] `tui/src/core/script/env_source/conformance_tests/tests/test_subshell_unix.rs`
    - [ ] `tui/src/core/script/env_source/conformance_tests/tests/test_subshell_windows.rs`
    - [ ] `tui/src/core/script/env_source/core.rs`
    - [ ] `tui/src/core/script/env_source/formatters/dotenv.rs`
    - [ ] `tui/src/core/script/env_source/formatters/fish.rs`
    - [ ] `tui/src/core/script/env_source/formatters/json.rs`
    - [ ] `tui/src/core/script/env_source/formatters/powershell.rs`
    - [ ] `tui/src/core/script/env_source/mod.rs`
    - [ ] `tui/src/core/script/git/branch_ops.rs`
    - [ ] `tui/src/core/script/git/file_ops.rs`
    - [ ] `tui/src/core/script/git/git_test_fixtures.rs`
    - [ ] `tui/src/core/script/git/mod.rs`
    - [ ] `tui/src/core/script/git/status_ops.rs`
    - [ ] `tui/src/core/script/git/types/constants.rs`
    - [ ] `tui/src/core/script/git/types/local_branch_info.rs`
    - [ ] `tui/src/core/script/git/types/mod.rs`
    - [ ] `tui/src/core/script/mod.rs`
    - [ ] `tui/src/core/script/temp_dir.rs`
    - [ ] `tui/src/core/stack_alloc_types/fast_int_fmt.rs`
    - [ ] `tui/src/core/stack_alloc_types/into_existing.rs`
    - [ ] `tui/src/core/stack_alloc_types/mod.rs`
    - [ ] `tui/src/core/storage/mod.rs`
    - [ ] `tui/src/core/terminal_io/backend_compat_tests/backend_compat_input_test.rs`
    - [ ] `tui/src/core/terminal_io/backend_compat_tests/backend_compat_output_test.rs`
    - [ ] `tui/src/core/terminal_io/backend_compat_tests/mod.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/mod.rs`
    - [ ] `tui/src/core/terminal_io/input_event.rs`
    - [ ] `tui/src/core/terminal_io/key_press.rs`
    - [ ] `tui/src/core/terminal_io/mod.rs`
    - [ ] `tui/src/core/test_fixtures/isolated_process_fixtures/mod.rs`
    - [ ] `tui/src/core/test_fixtures/mod.rs`
    - [ ] `tui/src/core/test_fixtures/output_device_fixtures/mod.rs`
    - [ ] `tui/src/core/test_fixtures/pty_test_fixtures/generate_pty_test.rs`
    - [ ] `tui/src/core/test_fixtures/pty_test_fixtures/mod.rs`
    - [ ] `tui/src/core/test_fixtures/pty_test_fixtures/spawn_controlled_in_pty.rs`
    - [ ] `tui/src/core/test_fixtures/retry_fixtures.rs`
    - [ ] `tui/src/core/test_fixtures/tcp_stream_fixtures/mod.rs`
    - [ ] `tui/src/core/tui_style/mod.rs`
    - [ ] `tui/src/core/tui_styled_text/mod.rs`
    - [ ] `tui/src/lib.rs`
    - [ ] `tui/src/network_io/json_serde.rs`
    - [ ] `tui/src/network_io/length_prefix_protocol.rs`
    - [ ] `tui/src/network_io/mod.rs`
    - [ ] `tui/src/readline_async/choose_impl/choose_integration_tests/mod.rs`
    - [ ] `tui/src/readline_async/choose_impl/mod.rs`
    - [ ] `tui/src/readline_async/choose_impl/select_component.rs`
    - [ ] `tui/src/readline_async/mod.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/event_conversion.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/line_control_task.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/line_state/core.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/line_state/cursor.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/line_state/event_handlers.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/line_state/mod.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/line_state/output.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/line_state/render.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/mod.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/mod.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_alt_navigation_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_ctrl_d_delete_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_ctrl_d_eof_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_ctrl_navigation_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_ctrl_u_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_ctrl_w_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_multiline_output_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_shared_writer_no_blank_line_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/readline_async_pty_test_fixtures.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/test_pty_alt_word_deletion.rs`
    - [ ] `tui/src/readline_async/spinner.rs`
    - [ ] `tui/src/readline_async/spinner_impl/mod.rs`
    - [ ] `tui/src/readline_async/spinner_impl/spinner_integration_tests/mod.rs`
    - [ ] `tui/src/tui/animator/mod.rs`
    - [ ] `tui/src/tui/cmd_line_args/mod.rs`
    - [ ] `tui/src/tui/dialog/dialog_buffer/mod.rs`
    - [ ] `tui/src/tui/dialog/dialog_component/dialog_component_struct.rs`
    - [ ] `tui/src/tui/dialog/dialog_component/mod.rs`
    - [ ] `tui/src/tui/dialog/dialog_engine/dialog_engine_api.rs`
    - [ ] `tui/src/tui/dialog/dialog_engine/mod.rs`
    - [ ] `tui/src/tui/dialog/mod.rs`
    - [ ] `tui/src/tui/dialog/test_dialog.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/clipboard/clipboard_service.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/clipboard/clipboard_service_impl.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/history/editor_history.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/mod.rs`
    - [ ] `tui/src/tui/editor/editor_component/editor_component_struct.rs`
    - [ ] `tui/src/tui/editor/editor_component/mod.rs`
    - [ ] `tui/src/tui/editor/editor_engine/caret_mut.rs`
    - [ ] `tui/src/tui/editor/editor_engine/engine_internal_api.rs`
    - [ ] `tui/src/tui/editor/editor_engine/engine_public_api.rs`
    - [ ] `tui/src/tui/editor/editor_engine/mod.rs`
    - [ ] `tui/src/tui/editor/editor_engine/scroll_editor_content.rs`
    - [ ] `tui/src/tui/editor/mod.rs`
    - [ ] `tui/src/tui/editor/test_fixtures_editor.rs`
    - [ ] `tui/src/tui/global_constants.rs`
    - [ ] `tui/src/tui/layout/mod.rs`
    - [ ] `tui/src/tui/mod.rs`
    - [ ] `tui/src/tui/rsx/mod.rs`
    - [ ] `tui/src/tui/syntax_highlighting/md_parser_syn_hi/md_parser_syn_hi_impl.rs`
    - [ ] `tui/src/tui/syntax_highlighting/md_parser_syn_hi/mod.rs`
    - [ ] `tui/src/tui/syntax_highlighting/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/crossterm_backend/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/input_device_impl.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/protocol_conversion.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/direct_to_ansi_output_integration_tests/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/direct_to_ansi_output_integration_tests/test_helpers_rendered.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/core.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/paint_impl.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/pixel_char_line.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/impls/flat_2d_array_impl.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/impls/growable_buffer.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/types.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/test_fixtures_ofs_buf.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/render_op/mod.rs`
    - [ ] `tui/src/tui/terminal_window/main_event_loop.rs`
    - [ ] `tui/src/tui/terminal_window/manage_focus/mod.rs`
    - [ ] `tui/src/tui/terminal_window/mod.rs`
    - [ ] `tui/src/tui/terminal_window/shared_global_data.rs`
    - [ ] `tui/src/tui/terminal_window/terminal_window_integration_tests/mod.rs`

#### Phase 10.3: Update Module Organization Docs/Skills & Run Final Verification

- [x] Update `.agents/skills/organize-modules/SKILL.md` and
      `.agents/skills/organize-modules/examples.md`:
    - Codify the 3-tier module visibility terminology table, the two-block
      `// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their macros).` +
      `// Attach.` split for `#[macro_use]` coordinators, and the canonical `// Attach.` +
      `// Re-export.` + inline `pub(crate) mod` reason pattern:
        - `mod foo;` -> **Folder-local (Intra-module)**
        - `pub(crate) mod foo;` -> **Crate-wide (Intra-crate)**
        - `pub mod foo;` -> **Public (Inter-crate)**
    - Replace Step 6 ("Multi-Level Barrel Exports and Rustdoc Search" / `#[doc(inline)]`)
      and Example 8 with the direct root-cause rule: never use `pub mod` on
      barrel-exported implementation submodules (`pub use child::*;`); use `mod` /
      `pub(crate) mod` with `--document-private-items`, eliminating `#[doc(inline)]`
      workarounds entirely.
    - Update the stale note in `organize-modules/SKILL.md` ("The Rustdoc Exception: If you
      _must_ link to a test module... it must be `pub mod`") to specify
      `#[cfg(any(test, doc))] pub(crate) mod` under `--document-private-items`.
- [x] Update `AGENTS.md` and `.agents/AGENTS.md` to use the 3-tier visibility terminology
      (`Folder-local (Intra-module)`, `Crate-wide (Intra-crate)`, `Public (Inter-crate)`),
      canonical `// Attach.` / `// Re-export.` + inline `pub(crate) mod` reason comments,
      and remove any remaining `#[doc(inline)]` search-link workaround references.
- [x] Run `./check.fish --full` via background subagent and confirm zero warnings across
      all stages (`typecheck`, `build`, `clippy`, `formatting`, `tests`, `doctests`,
      `doc build`, `windows build`, `lychee`).
- [ ] **Mandatory manual review:** Verify every file modified in Phase 10.3.
    - [ ] `.agents/skills/organize-modules/SKILL.md`
    - [ ] `.agents/skills/organize-modules/examples.md`
    - [ ] `.agents/skills/run-clippy/SKILL.md`
    - [ ] `.agents/skills/run-clippy/patterns.md`
    - [ ] `.agents/skills/write-documentation/link-patterns.md`
    - [ ] `AGENTS.md`
    - [ ] `.agents/AGENTS.md`
