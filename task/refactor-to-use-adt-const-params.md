# Task: Refactor to use ADT Const Params

## Overview

This task aims to leverage Algebraic Data Type Const Parameters (ADT Const Params),
<https://doc.rust-lang.org/nightly/unstable-book/language-features/adt-const-params.html>,
to replace runtime dispatch (enums/matching) with static dispatch (const generics)
everywhere that compile-time known/set choices are made. While this provides significant
performance benefits in hot paths, it is also the architecturally correct pattern for this
codebase, as these choices are determined at compile time.

The inspiration for this refactor comes from `ScopedMutex`, which uses a `const` enum
variant to choose a deadlock prevention policy at compile time (see
`tui/src/core/common/scoped_mutex/scoped_mutex_public_api.rs`). For compile-time known
invariants, this eliminates the need for:
1. Runtime checks.
2. Complicated boilerplate, where we define a trait, then implement it for all the structs
   that use this trait as a trait constraint.

### Benefits

1. **Zero Runtime Overhead**: Removes `match` or `if` branches in hot loops. The compiler
   prunes unused branches and dead code.
2. **Type-Level Identity**: Different configurations become different types, preventing
   accidental mixing of configurations (e.g., mixing mock and production devices).
3. **Dead Code Elimination**: Production binaries will not contain logic for unused
   backends (like Mock or Crossterm on Linux).
4. **Improved Memory Layout**: Eliminates the need for
   `#[allow(clippy::large_enum_variant)]` since the struct only contains the data for the
   active backend.

## Inventory of Target Types & Modules

The following areas are candidates for refactoring to use ADT Const Params.

### 1. Backend Selection (Core Devices)
Choices are fixed at compile time via `TERMINAL_LIB_BACKEND`.

- **`InputDevice`**: Refactor to `InputDevice<const BACKEND: TerminalLibBackend>`.
- **`OutputDevice`**: Refactor to `OutputDevice<const BACKEND: TerminalLibBackend>`.
- **`RenderOpOutputVec`**: Refactor `execute_all` and routing logic.
- **`OffscreenBufferPaintImpl`**: Make generic over `BACKEND`.
- **`RawMode` & `TermApi`**: Transition low-level APIs to static dispatch.

### 2. Debug & Diagnostic Toggles
Many `DEBUG_` flags are currently `pub const bool`. Using this pattern allows the compiler to prune debug code (logging, assertions, expensive checks) entirely when disabled.

#### How Runtime Logging and Static Flags Interact
It is critical to understand that these two systems act as **sequential gates**. A log message is only emitted if **both** gates are open:

1.  **Gate 1: Global Logger (Runtime)**: Controlled by CLI args (e.g., `edi -l`). If this is `OFF`, no logs are emitted globally.
2.  **Gate 2: Static Filter (Compile-time)**: Controlled by `const` flags (e.g., `DEBUG_TUI_MOD`). If this is `false`, the code block containing the log call is skipped entirely.

| Global Logger (`-l`) | `DEBUG_` Const | Result | Implementation Note |
| :--- | :--- | :--- | :--- |
| **OFF** | `false` | **Silence** | Code is pruned; no logger exists. |
| **OFF** | `true` | **Silence** | Code runs but finds no active logger. |
| **ON** | `false` | **Silence** | **Static flag vetoes the global logger.** Code is pruned. |
| **ON** | `true` | **Log Emitted** | Both gates are open. |

**The Goal**: By using ADT Const Params, we ensure that when a static flag is `false`, the compiler **prunes** the code and **strips** the associated strings/logic from the binary, even if the global logger is enabled at runtime.

- **Flags**: `DEBUG_TUI_COMPOSITOR`, `DEBUG_TUI_SHOW_PIPELINE`, `DEBUG_MD_PARSER`, etc.
- **Pattern**: `Logger<const ENABLED: bool>` or passing these to generic components.

### 3. Engine Policies
Components that have fixed behavior for a given application context.

- **`SyntaxHighlightMode`**: If an editor instance has a fixed highlighting mode, make it static.
- **`UseRenderCache`**: Toggle caching behavior at compile time for specific editor instances.

> **Note**: `CheckboxParsePolicy` was removed from this list because it is determined automatically at runtime based on the content being parsed.

## Implementation plan

### Phase 1: Preparation

- [ ] Add `#[derive(ConstParamTy)]` to `TerminalLibBackend` in `backend_selection.rs`.
- [ ] Ensure `TerminalLibBackend` implements `PartialEq` and `Eq`.
- [ ] **Mandatory manual review:**
  - [ ] `tui/src/tui/terminal_lib_backends/backend_selection.rs`

### Phase 2: Refactor Input & Output Devices

- [ ] Refactor `InputDevice` to `InputDevice<const BACKEND: TerminalLibBackend>`.
- [ ] Refactor `OutputDevice` to `OutputDevice<const BACKEND: TerminalLibBackend>`.
- [ ] Update all call sites (including tests and examples).
- [ ] **Mandatory manual review:**
  - [ ] `tui/src/core/terminal_io/input_device.rs`
  - [ ] `tui/src/core/terminal_io/output_device.rs`

### Phase 3: Refactor Rendering Pipeline

- [ ] Refactor `RenderOpOutputVec::execute_all` and routing logic to use static dispatch.
- [ ] Refactor `OffscreenBufferPaintImpl` to be generic over `BACKEND`.
- [ ] Update `paint.rs` to use these generic implementations.
- [ ] **Mandatory manual review:**
  - [ ] `tui/src/tui/terminal_lib_backends/render_op/render_op_output.rs`
  - [ ] `tui/src/tui/terminal_lib_backends/offscreen_buffer/paint_impl.rs`
  - [ ] `tui/src/tui/terminal_lib_backends/paint.rs`

### Phase 4: Refactor Low-Level APIs

- [ ] Refactor `RawMode` (mid-level) to use static dispatch.
- [ ] Refactor `TermApi` (low-level) functions to use static dispatch.
- [ ] **Mandatory manual review:**
  - [ ] `tui/src/core/ansi/terminal_raw_mode/raw_mode_core.rs`
  - [ ] `tui/src/core/term/term_api.rs`
  - [ ] `tui/src/core/term/term_api_impl.rs`

### Phase 5: Refactor Debug Flags & Policies

- [ ] Refactor `DEBUG_` flag usage to use const generics where applicable.
- [ ] Refactor Editor Engine modes (`SyntaxHighlightMode`, `UseRenderCache`).
- [ ] **Mandatory manual review:**
  - [ ] `tui/src/tui/editor/editor_buffer/render_cache.rs`
  - [ ] `tui/src/tui/editor/editor_engine/engine_struct.rs`

### Phase 6: Final Validation

- [ ] Run `./check.fish --full` to ensure no regressions across all platforms.
- [ ] Verify binary size and performance (optional but recommended).
