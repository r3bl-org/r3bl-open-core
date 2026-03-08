# Task: Reorganize PTY Module

## Overview
This task involves a comprehensive reorganization and unification of the PTY (Pseudoterminal) subsystem in the `r3bl-open-core` codebase. The goal is to create a clear bifurcation between OS-level engine primitives and high-level async session handles, improving maintainability, consistency, and discoverability.

## Architectural Goal

### 1. `tui/src/core/pty/pty_engine/` (OS-Level Engine)
*Focus: Managing file descriptors, processes, and terminal geometry.*
- `pty_pair.rs`: Core `PtyPair` struct, `open_and_spawn` logic, and FD hygiene.
- `pty_size.rs`: Terminal geometry (`Size`, `width`, `height`, `size`) and `DefaultPtySize`.
- `pty_engine_types.rs`: Consolidated OS-level aliases (`Controller`, `ControlledChild`, etc.), `PtyControlledChildExitStatus`, and `READ_BUFFER_SIZE`.

### 2. `tui/src/core/pty/pty_session/` (Async Session API)
*Focus: Async orchestration, configuration DSL, and interactive handles.*
- `pty_session_builder.rs`: `PtySessionBuilder` (unified builder), `PtySessionConfig` & `PtySessionConfigOption` (with `+` operator DSL). Includes logic unit tests.
- `pty_session_core.rs`: `PtyReadOnlySession`, `PtyReadWriteSession`, and channel type aliases.
- `pty_input_event.rs`: `PtyInputEvent` enum. Includes conversion unit tests.
- `pty_output_event.rs`: Unified `PtyOutputEvent` and `CursorModeDetector`.
- `pty_session_impl_shared.rs`: Private shared async tasks (Reader, Writer, Bridge).
- `pty_session_impl_read_only.rs`: Internal read-only orchestration.
- `pty_session_impl_read_write.rs`: Internal bidirectional orchestration.

### 3. `tui/src/core/pty/pty_mux/` (Multiplexing Layer)
*Focus: Managing multiple PTY sessions, input routing, and output rendering.*
- Relocated from `tui/src/core/pty_mux/` to be a peer of `pty_engine` and `pty_session`.

### 4. Testing Structure (`tui/src/core/pty/`)
- **Unit Tests**: Co-located within source files (e.g., `pty_session_builder.rs`, `pty_input_event.rs`). [DONE]
- **E2E Tests**: Located in `tui/src/core/pty/e2e_tests/`. [DONE]

---

## Implementation Plan

### Phase 1: Structural Realignment [DONE]
- [x] Create `tui/src/core/pty/pty_engine/` and `tui/src/core/pty/pty_session/` subdirectories.
- [x] Move `tui/src/core/pty_mux/` into `tui/src/core/pty/pty_mux/`.
- [x] Update `tui/src/core/mod.rs` to export only `pub mod pty;`.
- [x] Update `tui/src/core/pty/mod.rs` to export `pty_engine`, `pty_session`, and `pty_mux`.

### Phase 2: Engine Consolidation [DONE]
- [x] Move engine files (`pty_pair.rs`, `pty_size.rs`) to `pty_engine/`.
- [x] Consolidate OS-level types from `pty_types.rs` into `pty_engine/pty_engine_types.rs`.
- [x] Move `PtyControlledChildExitStatus` to `pty_engine/pty_engine_types.rs`.

### Phase 3: Session API Unification [DONE]
- [x] Implement the unified `PtyOutputEvent` in `pty_session/pty_output_event.rs`.
- [x] Merge `pty_config.rs` and `pty_command_builder.rs` into `pty_session/pty_session_builder.rs`.
- [x] Apply renames: `PtyConfig` -> `PtySessionConfig`, `PtyConfigOption` -> `PtySessionConfigOption`, `PtyCommandBuilder` -> `PtySessionBuilder`.
- [x] Implement `+` operator DSL for `PtySessionConfigOption`.
- [x] Extract shared task logic into `pty_session_impl_shared.rs`.
- [x] Create `pty_session_impl_read_only.rs` and `pty_session_impl_read_write.rs`.
- [x] Wire `PtySessionConfig` into both `spawn_read_only` and `spawn_read_write`.

### Phase 4: Test Realignment & Deadlock Fix [DONE]
- [x] Move E2E tests to `tui/src/core/pty/e2e_tests/`.
- [x] Co-locate unit tests within source files.
- [x] Fix DEADLOCK in `pty_session_impl_read_write.rs` (Completion task must not await bridge/writer tasks).
- [x] Ensure all tests use the "await handle then drain channel" pattern.

### Phase 5: Global Integration & Validation [DONE]
- [x] Update all imports across the codebase.
- [x] Update all call sites.
- [x] Run final full checks (`./check.fish --full`).
