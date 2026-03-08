# Simplify `MainEventLoopFuture` by removing `'a` lifetime

## Context

The `MainEventLoopFuture<'a, S, AS>` type alias carries a lifetime parameter `'a` solely
because `exit_keys` is passed as a borrowed slice (`&'a [InputEvent]`). This lifetime
propagates through `main_event_loop` -> `main_event_loop_impl` -> the `Box::pin(async move
{...})` future, adding complexity to both function signatures with no real benefit.

Exit keys are a tiny, fixed-size collection (typically 1-3 items). Cloning them once at
startup eliminates the borrow, removes the `'a` lifetime from the entire API surface, and
simplifies the public types.

## Changes

### 1. `tui/src/tui/terminal_window/terminal_window_api.rs`

- **Type alias**: `MainEventLoopFuture<'a, S, AS>` -> `MainEventLoopFuture<S, AS>`
  - Drop `'a` parameter. Remove `+ 'a` on the `dyn Future` (defaults to implicit `'static`).
- **`main_event_loop` fn**:
  - Drop `'a` lifetime parameter.
  - Keep `exit_keys: &[InputEvent]` (callers unchanged), but clone inside: `let exit_keys = exit_keys.to_vec();`
  - Pass owned `Vec<InputEvent>` to `main_event_loop_impl`.
  - Change `S` bound: `+ 'a` -> `+ 'static`.

### 2. `tui/src/tui/terminal_window/main_event_loop.rs`

- **`main_event_loop_impl` fn**:
  - Drop `'a` lifetime parameter.
  - Change `exit_keys: &'a [InputEvent]` -> `exit_keys: Vec<InputEvent>`.
  - Return type: `MainEventLoopFuture<'a, S, AS>` -> `MainEventLoopFuture<S, AS>`.
  - Change `S` bound: `+ 'a` -> `+ 'static`.
  - Inside `async move`, pass `&exit_keys` to inner functions (they already take
    `&[InputEvent]` - no changes needed).
- **Test `test_main_event_loop_impl`**:
  - Change call from `&exit_keys` to `exit_keys.to_vec()` since the function now takes
    owned `Vec<InputEvent>`.

### 3. No changes needed

- All ~20 inner helper functions in `main_event_loop.rs` already take `exit_keys: &[InputEvent]`
  (reborrowed, no named lifetime). Zero changes.
- All 7 callers (6 example launchers + cmdr/edi) pass `&[InputEvent]`. Zero changes.
- `S: 'static` is not a breaking restriction - `AS` already requires `'static`, and all
  existing state types are owned structs.

## Verification

```bash
./check.fish --check       # typecheck
./check.fish --quick-doc   # doc build (the no_run example on PtySessionBuilder)
./check.fish --clippy      # linting
./check.fish --test        # run tests (main_event_loop has unit tests)
```
