Task: Fix Readline Deadlock via Strict Lock Ordering Hierarchy (Approach A + Hybrid
Features)

## Reference

- Issue: <https://github.com/r3bl-org/r3bl-open-core/issues/488>

---

## Overview

A timing-dependent deadlock occurs in `ReadlineAsyncContext` when keyboard input handling
overlaps with background output from `SharedWriter`.

Stack traces from frozen processes show a classic lock-order inversion:

- **Input Path (`Readline::readline`):** Locks `OutputDevice` first, then locks
  `SafeLineState` second.
- **Background Output Path (`process_line_control_signal`):** Locks `SafeLineState` first,
  then locks `OutputDevice` second.

When these operations execute concurrently, each thread acquires one lock and blocks
waiting for the other lock held by the counterpart thread. Because the input loop handles
Ctrl+C interception, the process stops responding to all keyboard events and hangs
indefinitely.

---

## Architectural Analysis: Strict Lock Ordering Hierarchy & Hybrid Safety

### 1. Root Cause: Inconsistent Lock Acquisition Order

The deadlock is caused exclusively by an inversion in lock acquisition order between the
interactive input thread and the background channel monitor thread.

Deadlock requires four simultaneous
[Coffman conditions](https://en.wikipedia.org/wiki/Coffman_conditions):

1. Mutual exclusion
2. Hold and wait
3. No preemption
4. Circular wait

By establishing and enforcing a strict, universal total ordering across all mutexes:

```text
Level 1: SafeLineState (Arc<StdMutex<LineState>>)
   ↓
Level 2: OutputDevice (SafeRawTerminal: Arc<StdMutex<SendRawTerminal>>)
   ↓
Level 3: Leaf locks (SafeHistory, SafePauseBuffer, safe_spinner_is_active)
```

**Circular wait is mathematically impossible.** No thread can ever attempt to acquire a
Level 1 lock while holding a Level 2 lock.

### 2. Hybrid Safety: PauseState & TerminalLease RAII Guard

To guarantee that modals (`choose()`) and spinners never conflict with background output,
we adopt the robust `PauseState` and `TerminalLease` concepts from Approach B:

- **Explicit `PauseState` State Machine:** Refactor the `LineStateLiveness` enum into a
  `PauseState` enum with 4 explicit states: `NotPaused`, `PausedBySpinner`,
  `PausedByModal`, and `PausedByBoth`. This creates a rigorous state machine that
  flawlessly resolves overlapping suspension lifecycles (e.g., a background spinner
  stopping while a modal is open). Transition methods on `PauseState` return
  `PauseStateTransition` (`Paused`, `Resumed`, `Unchanged`), completely eliminating
  boolean blindness and `is_paused()` / `is_not_paused()` conversions.
- **RAII `TerminalLease` Guard:** A modal acquires a `TerminalLease` struct that
  transitions `LineState` to `PausedForModal` on creation, and restores it on `Drop`. This
  securely halts background flush signals during the lease.
- **Removal of Escape Hatches:** The `clone_output_device()` and `mut_input_device()`
  methods are removed from `ReadlineAsyncContext`. All background output MUST route
  through `SharedWriter` to be properly buffered in `SafePauseBuffer` while
  `TerminalLease` is active.

### 3. Clean Break: choose() API

To support `TerminalLease` elegantly, `choose()` and `DefaultIoDevices` receive a clean
break.

- `choose()` signature changes to strictly accept
  `io: (&mut OutputDevice, &mut InputDevice)`.
- `Option<SharedWriter>` is completely removed from `choose()` because suspending output
  is exclusively handled by the `TerminalLease` via the `SafeLineState` hierarchy.

### 4. Redundant History Channel Elimination (Zero Channel Sprawl)

Previously, `Readline` owned both ends of an unbounded MPSC channel (`history_sender` and
`history_receiver`) purely to send history updates from `add_history_entry` to
`readline()` on the same thread across consecutive loop iterations.

- `add_history_entry(&mut self, entry: String)` updates history directly via
  `self.safe_history.write(|h| h.update(Some(entry)))`.
- The channel, its sender/receiver fields, and the `history_receiver.recv()` branch in
  `select!` are completely removed.
- `History::new()` returns `Self` instead of `(Self, UnboundedReceiver<String>)`.

---

## Implementation Plan

### Phase 1: Lock Ordering Normalization & PauseState in readline.rs

- [x] Refactor `LineState::is_paused` to use `PauseState` Enum:
    - [x] Replace `LineStateLiveness` with
          `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum PauseState { NotPaused, PausedBySpinner, PausedByModal, PausedByBoth }`.
    - [x] Add transition methods on `PauseState` (`pause_spinner`, `resume_spinner`,
          `pause_modal`, `resume_modal`) returning `PauseStateTransition` (`Paused`,
          `Resumed`, `Unchanged`), and eliminate `is_paused` / `is_not_paused` in favor of
          direct enum comparisons and transitions.
    - [x] Update `LineState` to hold `pause_state: PauseState`.
- [x] Eliminate boolean blindness on `LineState`:
    - [x] Replace `should_print_line_on_enter: bool` with `PrintLineOnEnter` enum
          (`Print`, `DoNotPrint`).
    - [x] Replace `should_print_line_on_control_c: bool` with `PrintLineOnControlC` enum
          (`Print`, `DoNotPrint`).
    - [x] Replace `last_line_completed: bool` with `EndsWithNewline` enum (`Yes`, `No`).
    - [x] Update `Readline::should_print_line_on` API to accept
          `(PrintLineOnEnter, PrintLineOnControlC)` instead of `(bool, bool)`.
    - [x] Model `prompt` and `prompt_width` as a dedicated `Prompt` struct:
        - [x] Create `tui/src/readline_async/readline_async_impl/line_state/prompt.rs`
              with `Prompt` struct (`raw: String`, `width: VPWidth`), keeping display
              width in sync by construction.
        - [x] Provide methods: `Prompt::new`, `Prompt::set`, `Prompt::as_str`,
              `Prompt::width`, `Prompt::calculate_width`, plus `Display`, `Deref`,
              `AsRef`, and `From` trait implementations.
        - [x] Replace separate `pub prompt: String` and `pub prompt_width: VPWidth` on
              `LineState` with `pub prompt: Prompt`.
        - [x] Update call sites across `line_state` (`core.rs`, `cursor.rs`, `output.rs`,
              `render.rs`, `event_handlers.rs`).
        - [x] Re-export `Prompt` in `line_state/mod.rs` and update module docs table.
        - [x] Add unit tests for `Prompt` in `prompt.rs` and verify all tests pass.
    - [x] Update example in `tui/examples/readline_async.rs` to use new `PrintLineOnEnter`
          and `PrintLineOnControlC` enums.
    - [x] Deduplicate `CHA(1)` sequence on newline-terminated segments in
          `LineState::print_data_and_flush` to prevent visual artifacts on raw terminal
          emulators.
    - [x] Eliminate obsolete `cluster_buffer` from `LineState`:
        - [x] Remove `pub cluster_buffer: String` from `LineState` in `core.rs` and its
              initialization in `LineState::new`.
        - [x] Update `handle_char` in `event_handlers.rs` to detect grapheme cluster
              additions via `line_state.line.segment_count()`.
        - [x] Remove unused `UnicodeSegmentation` import in `event_handlers.rs`.
        - [x] Add unit test `test_handle_char_combining_characters` in `event_handlers.rs`
              verifying combining character input and middle-of-line insertion.
    - [x] Rename `line_cursor_grapheme` to `cursor_position` and clarify grapheme
          navigation:
        - [x] Rename `pub line_cursor_grapheme: SegIndex` to
              `pub cursor_position: SegIndex` on `LineState` in `core.rs`.
        - [x] Rename `current_grapheme(&self)` to `grapheme_before_cursor(&self)` and
              `next_grapheme(&self)` to `grapheme_at_cursor(&self)` in `cursor.rs`.
        - [x] Update call sites across `cursor.rs`, `event_handlers.rs`, `readline.rs`,
              `pty_editor_state_test.rs`, and `readline_async_pty_test_fixtures.rs`.
    - [x] Clarify cursor painting vs logical movement and eliminate cached current_column:
        - [x] In `cursor.rs`:
            - [x] Rename `move_cursor(&mut self, isize) -> io::Result<()>` to infallible
                  `shift_logical_cursor_by(&mut self, isize)`.
            - [x] Add `move_logical_cursor_to_start(&mut self)`.
            - [x] Add `move_logical_cursor_to_end(&mut self)`.
            - [x] Replace cached `current_column` field with pure
                  `calc_current_column(&self) -> VPCol`.
            - [x] Rename `rewind_cursor_to_start` to `paint_cursor_rewind_to_start`.
            - [x] Rename `position_cursor_at_current_column` to
                  `paint_cursor_at_current_column`.
            - [x] Rename `move_cursor_to_start_from` to `paint_cursor_to_start_from`.
            - [x] Rename `move_cursor_from_start_to` to `paint_cursor_from_start_to`.
        - [x] Update call sites in `core.rs`, `render.rs`, `output.rs`,
              `event_handlers.rs`, and unit tests in `cursor.rs`.
    - [x] Update link checking configuration:
        - [x] Exclude stackexchange.com wildcard domains in `lychee.toml`.
- [x] Implement `ReadlineLockManager` for Strict Hierarchy:
    - [x] Create `tui/src/readline_async/readline_async_impl/readline_lock_manager.rs` and
          define the `ReadlineLockManager` struct holding `line_state: SafeLineState` and
          `output_device: OutputDevice`.
    - [x] Provide `lock_both` and `lock_line_state` methods (`lock_output_device` removed
          as unused since background components hold `OutputDevice` directly). `lock_both`
          must strictly acquire `SafeLineState` first, then `OutputDevice`, preventing
          lock inversion.
    - [x] Add `pub(crate) fn output_device_mut(&mut self) -> &mut OutputDevice` to allow
          `TerminalLease` to yield the device.
    - [x] Replace `safe_line_state` and `output_device` fields in `Readline` with a single
          `pub(crate) lock_manager: ReadlineLockManager`.
- [x] Add Rustdocs for Deadlock Prevention:
    - [x] Consolidate all deadlock prevention documentation into the struct-level rustdocs
          for `ReadlineLockManager`. Explain the Coffman lock hierarchy, the purpose of
          `lock_both`, and explicitly document that single-lock closures (like
          `lock_line_state`) MUST be leaf operations.
    - [x] Add documentation to `Spinner` highlighting its "Structural Isolation", because
          it only holds `OutputDevice` and lacks `SafeLineState`, it is structurally
          immune to lock inversions.
- [x] Refactor existing nested locks to use `ReadlineLockManager`:
    - [x] Refactor `Readline::readline(&mut self)` to use `lock_both` and pass
          `&mut LineState` directly to `apply_event_to_line_state_and_render`.
    - [x] Refactor `Readline::update_prompt(&mut self, prompt: &str)` to use `lock_both`.
    - [x] Refactor `Readline::clear(&mut self)` to use `lock_both`.
    - [x] Refactor `Readline::try_new` initial prompt render to use `lock_both`.
- [x] Invert lock acquisition in `Drop` for `Readline`:
    - [x] Acquire `self.lock_manager.line_state.lock_raw_poison_safe` first, then
          `self.lock_manager.output_device.lock_raw_poison_safe` second (requires
          providing internal access for `Drop` if needed). (cannot use standard traffic
          cop due to poison-safe requirements).
- [x] Optimize `process_line_control_signal` on `Flush`:
    - [x] Check `line_state.pause_state != PauseState::NotPaused` before acquiring
          `output_device.write`. If true, return `Continuation::Continue` immediately
          without acquiring the device lock.
- [x] Update `process_line_control_signal` for `Pause` and `Resume`:
    - [x] On `Pause` (from Spinner), use `pause_spinner()` and trigger
          `clear_and_render_and_flush` only if it returns `PauseStateTransition::Paused`.
    - [x] On `Resume` (from Spinner), use `resume_spinner()` and trigger `flush_internal`
          only if it returns `PauseStateTransition::Resumed`.
- [x] Clean up `gc_string_owned_editor_impl.rs`:
    - [x] Merge all editor-specific `GCStringOwned` methods (`split_at_display_col`,
          `insert_chunk_at_col`, `get_string_at`, etc.) natively into
          `tui/src/core/graphemes/gc_string/owned/gc_string_owned.rs`.
    - [x] Delete `gc_string_owned_editor_impl.rs` entirely and remove it from `mod.rs` to
          flatten the file structure.
    - [x] Add `get_byte_index` to `gc_string_owned.rs` to support
          `calc_display_width_up_to_cursor` logic in `readline_async`.
    - [x] Strip legacy inner modules (`at_display_col_index` and `mutate`) and their
          deprecated migration notices.
- [x] **Mandatory manual review:**
    - [x] `tui/src/readline_async/readline_async_impl/line_state/core.rs`
    - [x] `tui/src/readline_async/readline_async_impl/line_state/cursor.rs`
    - [x] `tui/src/readline_async/readline_async_impl/line_state/event_handlers.rs`
    - [x] `tui/src/readline_async/readline_async_impl/line_state/output.rs`
    - [x] `tui/src/readline_async/readline_async_impl/line_state/render.rs`
    - [x] `tui/src/readline_async/readline_async_impl/line_state/mod.rs`
    - [x] `tui/src/readline_async/readline_async_impl/line_state/prompt.rs`
    - [x] `tui/src/readline_async/readline_async_impl/mod.rs`
    - [x] `tui/src/readline_async/spinner.rs`
    - [x] `tui/src/readline_async/readline_async_impl/readline.rs`
    - [x] `tui/src/readline_async/readline_async_api.rs`
    - [x] `tui/src/readline_async/mod.rs`
    - [x] `tui/src/core/misc/calc_str_len.rs`
    - [x] `tui/examples/readline_async.rs`
    - [x] `tui/src/core/resilient_reactor_thread/rrt_integration_tests/double_panic_prevention_test.rs`
    - [x] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_editor_state_test.rs`
    - [x] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_readline_test.rs`
    - [x] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/readline_async_pty_test_fixtures.rs`
    - [x] `lychee.toml`

---

### Phase 2: TerminalLease, Escape Hatches & choose() API Break

- [x] Remove API Escape Hatches:
    - [x] Delete `clone_output_device()` and `mut_input_device()` from
          `ReadlineAsyncContext`.
    - [x] Change `pub output_device` and `pub input_device` in `Readline` to `pub(crate)`
          to definitively close all bypass routes.
- [x] Add `line_control_sender` to `Readline`:
    - [x] Add
          `pub line_control_sender: Option<tokio::sync::mpsc::Sender<LineStateControlSignal>>`
          to `Readline` struct.
    - [x] Populate it in `Readline::try_new` (wrapping the cloned sender in `Some`).
- [x] Implement `ModalTerminalGuard` RAII Guard:
    - [x] Create `ModalTerminalGuard` in `tui/src/readline_async/modal_terminal_guard.rs`.
    - [x] Provide `fn acquire(&mut Readline)` which transitions state via match:
          `NotPaused -> PausedByModal`, `PausedBySpinner -> PausedByBoth`, clears the
          prompt from the screen (`line_state.clear_and_render_and_flush`), and yields
          `(&mut OutputDevice, &mut InputDevice)`.
    - [x] On `Drop`, transition state via match: `PausedByModal -> NotPaused`,
          `PausedByBoth -> PausedBySpinner`. If `is_paused()` becomes false, use
          `readline.line_control_sender.try_send(Flush)` to replay buffered lines and
          redraw the prompt synchronously.
- [x] Apply Clean Break to `choose()` API:
    - [x] Update `fn choose(io: (&mut OutputDevice, &mut InputDevice), ...)` in
          `choose_api.rs`, removing `Option<SharedWriter>`.
    - [x] Update `DefaultIoDevices` to return `(&mut OutputDevice, &mut InputDevice)`.
- [x] Update Downstream `cmdr` Consumers & Examples:
    - [x] Fix usages in `ui_templates.rs`, `branch_checkout_command.rs`, etc. to remove
          `Option<SharedWriter>` arguments and adapt to the strict API.
- [x] Modularize `readline_async_impl/` and Split `readline.rs`:
    - [x] Rename `tui/src/readline_async/readline_async_impl/readline_history.rs` to
          `tui/src/readline_async/readline_async_impl/history.rs`.
    - [x] Rename `tui/src/readline_async/readline_async_impl/readline_lock_manager.rs` to
          `tui/src/readline_async/readline_async_impl/lock_manager.rs`.
    - [x] Extract `tui/src/readline_async/readline_async_impl/types.rs` from `readline.rs`
          containing `ReadlineEvent`, `ReadlineControlFlow`, `ReadlineError`, and timing
          constants.
    - [x] Extract `tui/src/readline_async/readline_async_impl/channel_monitor.rs` from
          `readline.rs` containing `spawn_task_to_monitor_line_control_channel`,
          `process_line_control_signal`, `flush_internal`, and pause/resume flush tests.
    - [x] Extract `tui/src/readline_async/readline_async_impl/event_conversion.rs` from
          `readline.rs` containing `apply_event_to_line_state_and_render`, Crossterm event
          converter methods, and stream tests.
    - [x] Extract `tui/src/readline_async/readline_async_impl/readline_struct.rs` from
          `readline.rs` containing `Readline` struct definition, `try_new`, `Drop`, and
          `readline` async event loop.
    - [x] Update `tui/src/readline_async/readline_async_impl/mod.rs` to re-export new
          submodules via barrel export pattern, maintaining `manage_shared_writer_output`
          and `readline_internal` module aliases for compatibility.
    - [x] Delete `tui/src/readline_async/readline_async_impl/readline.rs`.

---

### Phase 3: History Channel Simplification

- [x] Refactor `History` in `tui/src/readline_async/readline_async_impl/history.rs`:
    - [x] Remove `sender: UnboundedSender<String>` field from `History`.
    - [x] Update `History::new()` to return `Self` without `UnboundedReceiver`. Implement
          `Default` for `History`.
    - [x] Update unit tests in `history.rs`.
- [x] Refactor `Readline` in
      `tui/src/readline_async/readline_async_impl/readline_struct.rs`:
    - [x] Remove `history_sender: UnboundedSender<String>` and
          `history_receiver: UnboundedReceiver<String>` fields from `Readline`.
    - [x] In `Readline::try_new`, construct `History::new()` directly without channel
          setup.
    - [x] In `Readline::readline`, remove the `maybe_line = self.history_receiver.recv()`
          branch from `tokio::select!`.
    - [x] In `Readline::add_history_entry`, update to mutate synchronously:
          `self.safe_history.write(|h| h.update(Some(entry)))`.

---

### Phase 4: Integration Tests & Deadlock Regression Stress Test

- [x] Update tests calling `apply_event_to_line_state_and_render` if signature changed:
    - [x] Check `pty_editor_state_test.rs` to ensure it passes `line_state` cleanly.
- [x] Add dedicated concurrent stress regression test:
    - [x] Create
          `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_concurrent_input_output_deadlock_test.rs`.
    - [x] Concurrently write multiple multiline blocks from background `SharedWriter`
          tasks while simultaneously typing keystrokes, Enter, and Ctrl+C.
    - [x] Assert process never freezes, output completes without corruption, and all input
          events are received cleanly.
- [x] Run validation checks:
    - [x] `./check.fish --check`
    - [x] `./check.fish --test`
    - [x] `./check.fish --clippy`
- [ ] **Mandatory manual review:**
    - [x] `tui/src/readline_async/readline_async_impl/types.rs`
    - [x] `tui/src/readline_async/readline_async_impl/lock_manager.rs`
    - [ ] `tui/src/readline_async/modal_terminal_guard.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/history.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/channel_monitor.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/event_conversion.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_struct.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/mod.rs`
    - [ ] `tui/src/readline_async/choose_api.rs`
    - [ ] `tui/src/readline_async/readline_async_api.rs`
    - [ ] `tui/src/readline_async/mod.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_basic_ops.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_delete_ops.rs`
    - [ ] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_insert_ops.rs`
    - [ ] `tui/src/readline_async/choose_impl/choose_integration_tests/pty_shared_writer_pause_test.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_concurrent_input_output_deadlock_test.rs`
    - [ ] `tui/examples/choose_with_and_without_readline_async.rs`
- [ ] Manual testing via `tui/examples/demo/ex_app_with_spinner.rs` to verify visuals.

---

### Phase 5: Documentation & Final Workspace Verification

- [ ] Update rustdoc comments in `readline_struct.rs` and `readline_async/mod.rs`
      documenting the strict lock hierarchy (`SafeLineState` -> `OutputDevice` -> Leaf
      locks).
- [ ] Verify rustdoc links build cleanly: `./check.fish --quick-doc`.
- [ ] Run full workspace validation: `./check.fish --full`.
- [ ] **Mandatory manual review:**
    - [ ] `tui/src/readline_async/mod.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_struct.rs`

<!-- cspell:words coffman stackexchange -->
