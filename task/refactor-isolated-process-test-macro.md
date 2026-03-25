# Task: Refactor isolated process test macro

## Overview

The codebase has 10+ tests that spawn themselves in an isolated process to avoid global
state contamination (env vars, statics, CWD changes). They all manually implement the
same boilerplate: env-var check, `Command::new(current_exe())`, spawn self, check output.

A `generate_self_isolated_process_test!` macro already exists in `test_piped_stdout.rs`
and is used by 2 tests. This task moves it to a proper home, enhances it with
Windows support and async capability, and migrates all 8 remaining candidates.

This is distinct from `generate_pty_test!` which handles PTY-based process isolation
(those tests are already using their macro and are not in scope here).

### Key design decisions

- **Two macros** for readability: `generate_isolated_process_test!` (sync) and
  `generate_async_isolated_process_test!` (async). The name itself documents the
  shape of the test. Shared logic lives in an internal `__isolated_process_impl!` helper.
- **Explicit stdio** always: caller specifies `stdin`, `stdout`, `stderr` - no hidden
  defaults.
- **Rename** from `generate_self_isolated_process_test!` - "self-spawning" is an
  implementation detail. New names match `generate_pty_test!` style.

### Enhancements over current macro

The current macro is missing several things the hand-written tests include:
- `suppress_wer_dialogs()` (Windows WER dialog suppression)
- `new_isolated_test_command()` (Windows `CREATE_NO_WINDOW` flag)
- `RUST_BACKTRACE=1` env var
- `--nocapture` arg (current macro omits it)

## Implementation plan

### Phase 1: Create the new submodule [DONE]

- [x] Create `tui/src/core/test_fixtures/isolated_process_fixtures/mod.rs`
- [x] Create `generate_isolated_process_test.rs` with the sync macro and internal
      `__isolated_process_impl!` helper. Include `suppress_wer_dialogs()`,
      `new_isolated_test_command()`, `RUST_BACKTRACE=1`, explicit stdio params,
      `--test-threads 1 --nocapture` args.
- [x] Wire up in `test_fixtures/mod.rs` (add `#[macro_use] pub mod isolated_process_fixtures`)
- [x] Verify it compiles

### Phase 2: Migrate existing usage + sync candidates

- [ ] Move `test_piped_stdout.rs` and `test_piped_stdin.rs` to the new macro
      (rename invocation from `generate_self_isolated_process_test!` to
      `generate_isolated_process_test!`)
- [ ] Delete the old macro definition from `test_piped_stdout.rs`
- [ ] Delete the "OLD WAY" dead code from both files
- [ ] Migrate `detect_color_support.rs:662` (`test_all_color_support_detection_*`)
- [ ] Migrate `at_most_one_instance_assert.rs:90` (`test_at_most_one_instance_*`)
- [ ] Migrate `text_operations_rendered.rs:363` (`test_all_rendered_output_*`)
- [ ] Migrate `rrt_restart_tests.rs:289` (`test_rrt_restart_*`)
- [ ] Migrate `fs_path.rs:694` (`test_all_fs_path_functions_*`)
- [ ] Run tests to verify all sync migrations pass

### Phase 3: Add async variant + migrate async candidates

- [ ] Create `generate_async_isolated_process_test.rs` in
      `isolated_process_fixtures/`. Generates `#[tokio::test] async fn` and
      `.await`s the controlled function.
- [ ] Wire up in `isolated_process_fixtures/mod.rs`
- [ ] Migrate `git/status_ops.rs:155` (`test_status_ops_*`)
- [ ] Migrate `git/file_ops.rs:227` (`test_changed_files_*`)
- [ ] Migrate `git/branch_ops.rs:492` (`test_branch_ops_*`)
- [ ] Run tests to verify all async migrations pass

### Phase 4: Documentation + cleanup

- [ ] Add rustdoc to both macros (architecture diagram, usage examples, comparison
      with `generate_pty_test!` - mirroring the style of `generate_pty_test.rs`)
- [ ] Verify `cargo doc` builds cleanly with no broken intra-doc links
- [ ] Run full test suite (`./check.fish --test`)

## Reference: Migration targets

| # | File | Test function | Macro |
|---|------|---------------|-------|
| 1 | `term_integration_tests/test_piped_stdout.rs` | `test_piped_stdout_is_interactive` | sync (existing) |
| 2 | `term_integration_tests/test_piped_stdin.rs` | `test_piped_stdin_is_not_interactive` | sync (existing) |
| 3 | `ansi/detect_color_support.rs:662` | `test_all_color_support_detection_*` | sync |
| 4 | `direct_to_ansi/input/at_most_one_instance_assert.rs:90` | `test_at_most_one_instance_*` | sync |
| 5 | `direct_to_ansi/output/.../text_operations_rendered.rs:363` | `test_all_rendered_output_*` | sync |
| 6 | `resilient_reactor_thread/tests/rrt_restart_tests.rs:289` | `test_rrt_restart_*` | sync |
| 7 | `script/fs_path.rs:694` | `test_all_fs_path_functions_*` | sync |
| 8 | `script/git/status_ops.rs:155` | `test_status_ops_*` | async |
| 9 | `script/git/file_ops.rs:227` | `test_changed_files_*` | async |
| 10 | `script/git/branch_ops.rs:492` | `test_branch_ops_*` | async |
