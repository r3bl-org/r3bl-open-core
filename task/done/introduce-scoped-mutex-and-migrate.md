# Task: Implement ScopedMutex and Migrate Codebase

## Overview

This task involves implementing the `ScopedMutex` pattern (Scoped Access) and migrating
the codebase from standard `std::sync::Mutex` (Guard-based) to this safer,
closure-based implementation with built-in deadlock prevention.

## Implementation plan

### Phase 1: Core Implementation [DONE]

- [x] Create `ScopedMutex` struct with `DeadlockPreventionPolicy`.
- [x] Implement `SharedLedger` for thread-local lock tracking.
- [x] Implement `PanicOnAnyLockNesting` and `PanicOnSpecificLockNesting` policies.
- [x] Create `scoped_mutex!` macro for easy instantiation.
- [x] Add comprehensive tests for recursion detection and policy enforcement.

### Phase 2: Pilot Migration & Skills [DONE]

- [x] Migrate `SAVED_TERMIOS` to `ScopedMutex<ANY>`.
- [x] Migrate `ROLLING_LOG_FILE_WRITER_GUARD` to `ScopedMutex<ANY>`.
- [x] Migrate `DYNAMIC_CACHE` to `ScopedMutex<OPT_OUT>`.
- [x] Update `concurrency-safety` skill with `ScopedMutex` patterns.

### Phase 3: Codebase-wide ScopedMutex Migration [DONE]

This phase involves migrating all remaining `StdMutex` (guard-based) call sites to
`ScopedMutex` (closure-based). This is a significant architectural shift that uses a
deliberate **breaking API change** (removing `.lock()`) to force a codebase-wide
migration to the safer Scoped Access pattern.

- [x] **Sub-task 1: Centralize Type Aliases & Policy Selection**
  - [x] Update `tui/src/core/terminal_io/terminal_io_type_aliases.rs` to replace `StdMutex`
    with `ScopedMutex`.
  - [x] Update `tui/src/readline_async/mod.rs` to migrate all `Safe*` type aliases.
  - [x] Add `: ?Sized` support to `ScopedMutex` for trait objects.

- [x] **Sub-task 2: Refactor OutputDevice & Macros**
  - [x] Refactor `tui/src/core/terminal_io/output_device.rs` to use direct closure-based
    API.
  - [x] Remove redundant `use_output_device!` and `lock_output_device_as_mut!` macros.
  - [x] Refactor 70+ call sites to use direct `.write()` or `.read()` calls.

- [x] **Sub-task 3: Migrate Readline & Core Components**
  - [x] Refactor `Readline` loop and event handlers to use closures.
  - [x] Migrate `LRU Cache`, `ColorWheel`, and `Spinner` implementations.
  - [x] Refactor `drop()` to use `lock_raw()` for poison-safe cleanup.

- [x] **Sub-task 4: Migrate Test Fixtures & Integration Tests**
  - [x] Refactor all mock fixtures and integration tests to use the closure-based API.
  - [x] Ensure 100% clean build with zero errors and zero warnings.

- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
  - [x] `tui/src/core/terminal_io/terminal_io_type_aliases.rs`
  - [x] `tui/src/core/common/scoped_mutex/mutex_ext.rs`
  - [x] `tui/src/core/terminal_io/output_device.rs`
  - [x] `tui/src/readline_async/readline_async_impl/readline.rs`
  - [x] `tui/src/readline_async/spinner.rs`
  - [x] `tui/src/core/test_fixtures/output_device_fixtures/stdout_mock.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/raw_mode_unix.rs`
  - [x] `tui/src/core/log/rolling_file_appender_impl.rs`
  - [x] `tui/src/core/common/string_repeat_cache.rs`

### Phase 4: Formalize lock_raw_poison_safe in ScopedMutex [DONE]

- [x] Add `lock_raw_poison_safe` to `ScopedMutex` in `scoped_mutex_public_api.rs`.
- [x] Migrate `tui/src/readline_async/readline_async_impl/readline.rs` to use `lock_raw_poison_safe`.
- [x] Migrate `tui/src/core/ansi/terminal_raw_mode/raw_mode_unix.rs` to use `lock_raw_poison_safe`.
- [x] Migrate `tui/src/core/log/rolling_file_appender_impl.rs` to use `lock_raw_poison_safe`.
- [x] Migrate `tui/src/core/terminal_io/output_device.rs` to use `lock_raw_poison_safe`.
- [x] Run clippy and check all tests.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correctness.
  - [x] `tui/src/core/common/scoped_mutex/scoped_mutex_public_api.rs`
  - [x] `tui/src/readline_async/readline_async_impl/readline.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/raw_mode_unix.rs`
  - [x] `tui/src/core/log/rolling_file_appender_impl.rs`
  - [x] `tui/src/core/terminal_io/output_device.rs`

