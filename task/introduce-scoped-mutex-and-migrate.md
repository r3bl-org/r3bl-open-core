# Task: Implement ScopedMutex and Migrate Codebase

## Overview

This task involves implementing the `ScopedMutex` pattern (Scoped Access) and migrating the codebase from standard `std::sync::Mutex` (Guard-based) to this safer, closure-based alternative.

`ScopedMutex` provides structural deadlock prevention by detecting recursive locks at runtime. It supports different policies:
- `PanicOnAnyLockNesting`: Prevents any lock from being held while another is acquired.
- `PanicOnSpecificLockNesting`: Prevents the same lock instance from being re-acquired.
- `OptOut`: Disables recursion detection for performance-critical sections.

## Implementation plan

### Phase 1: Implement ScopedMutex (Scoped Access pattern) [COMPLETE]

- [x] **Implement ScopedMutex Core:** Update `tui/src/core/common/scoped_mutex.rs` with:
  - [x] `const SAFETY_POLICY: DeadlockPreventionPolicy` enum generic (default
        `DeadlockPreventionPolicy::PanicOnAnyLockNesting`). Provide links to types which
        use the `PanicOnAnyLockNesting` variant, and the one type that uses the `None`
        variant. This will make it easier for maintainers to understand where this type
        and its 2 variants are used.
  - [x] `thread_local!` recursion depth detection logic in `read()` and `write()` using
        `Cell<u8>`. Improved diagnostics by including current depth in `panic!` messages
        and adding `tracing::error!` logs.
  - [x] **Implement RAII `DeadlockPreventionGuard`:** Refactor recursion tracking to use a
        private `DeadlockPreventionGuard` (RAII) for better panic safety and cleaner code.
  - [x] **Refactor to State Machine:** Refactor `deadlock_prevention.rs` to use a
        centralized `ThreadLockState` enum (`NoLocksHeld`, `AnyLockHeld`,
        `SpecificLocksHeld`) instead of fragmented thread-locals, ensuring the hierarchy
        is explicitly enforced via a state machine.
  - [x] **Expand Documentation:** Add a section to `ScopedMutex` rustdocs explaining the
        "Aggressive Design" choice of a simple global counter vs. a lock-aware set to
        prevent circular wait deadlocks.
  - [x] `lock_raw()` as the unrestricted "escape hatch" for cleanup.
  - [x] `MutexExt` update to support passing the const generic.
  - [x] **Remove `new()`:** Removed `ScopedMutex::new()` to enforce the use of `MutexExt`
        and `LazyLock` for statics.
- [x] **Refactor into Module:** Reorganize `scoped_mutex.rs` into a dedicated module
      directory `tui/src/core/common/scoped_mutex/` for better maintainability.
  - [x] `deadlock_prevention.rs`: `DeadlockPreventionPolicy`, and RAII guards.
  - [x] `thread_lock_state_machine.rs`: `ThreadLockState` and `thread_local!` state.
  - [x] `mutex_ext.rs`: `MutexExt` trait and blanket implementation.
  - [x] `scoped_mutex_public_api.rs`: `ScopedMutex` struct, methods, and all tests.
  - [x] `mod.rs`: Barrel re-exports.
- [x] **Add Unit Tests:** Add deterministic unit tests for `ScopedMutex` covering
      recursion detection and the opt-out mechanism. Write these tests clearly so it is
      obvious what use case we are testing in any given test, making it easier for
      maintainers to use the tests to understand how the code works - don't muddle them
      together.
- [x] **Convert Global Statics to ScopedMutex:**
  - [x] **Recursion Detection = PanicOnAnyLockNesting (Default)**:
    - [x] `SAVED_TERMIOS` in `tui/src/core/ansi/terminal_raw_mode/raw_mode_unix.rs`.
    - [x] `ROLLING_LOG_FILE_WRITER_GUARD` in
          `tui/src/core/log/rolling_file_appender_impl.rs`.
    - [x] `TEST_FACTORY_STATE` in
          `tui/src/core/resilient_reactor_thread/process_isolated_tests/fixtures.rs`.
    - [x] Update
          `tui/src/core/ansi/terminal_raw_mode/integration_tests/test_poison_recovery.rs`.
  - [x] **Recursion Detection = None (Performance Opt-out)**:
    - [x] `DYNAMIC_CACHE` in `tui/src/core/common/string_repeat_cache.rs`. Make sure to
          document in rustdocs why we use the less-safe version of ScopedMutex.
- [x] **Update Documentation & Skills:**
  - [x] Update rustdocs in `scoped_mutex.rs` to explain recursion detection and opt-out.
  - [x] Update `tui/src/lib.rs` with Scoped Access vs. Chain of Custody patterns and
        updated poison-safety table.
  - [x] Update `.agent/skills/concurrency-safety/SKILL.md` and `patterns.md` to include
        these patterns.
- [x] **Sub-task 3: Unify Policy Declaration and Implementation**
  - Merge `thread_lock_state_machine.rs` logic into `deadlock_prevention.rs`.
  - Implement `try_acquire` and `release` methods on `DeadlockPreventionPolicy`.
  - Replace separate RAII guards with a single
    `DeadlockPreventionGuard<const POLICY: DeadlockPreventionPolicy>`.
  - Delete `thread_lock_state_machine.rs` and update re-exports.
- [x] **Sub-task 4: Rename to SharedLedger**
  - Rename `ThreadLockState` -> `SharedLedger`.
  - Rename `THREAD_LOCK_STATE` -> `THREAD_LOCAL_LEDGER`.
  - Rename `ThreadLockError` -> `SharedLedgerError`.
  - Update all rustdocs and internal references.
- [x] **Sub-task 5: Use specificity and simplify state machine**
  - Implement CSS specificity mental model for deadlock prevention policies.
  - Refactor `SharedLedger` from enum to struct.
  - Simplify transition logic using specificity scores.
- [x] **Sub-task 6: Make SharedLedger::addresses optional**
  - Change `addresses` type from `SmallVec` to `Option<SmallVec>`.
  - Update `try_acquire` to initialize the option on first acquisition.
  - Update `release` to reset the option to `None` when last lock is released.
  - Update tests to handle the `Option`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
  - [x] `tui/src/core/common/scoped_mutex/mod.rs`
  - [x] `tui/src/core/common/scoped_mutex/deadlock_prevention/`
    - [x] `tui/src/core/common/scoped_mutex/deadlock_prevention/mod.rs`
    - [x] `tui/src/core/common/scoped_mutex/deadlock_prevention/constants.rs`
    - [x] `tui/src/core/common/scoped_mutex/deadlock_prevention/policy_impl.rs`
      - [x] scrutinize tests for accurate state machine transitions
  - [x] `tui/src/core/common/scoped_mutex/mutex_ext.rs`
  - [x] `tui/src/core/common/scoped_mutex/scoped_mutex_public_api.rs`
    - [x] rustdocs, use cases, examples
    - [x] code organization
    - [x] code quality
    - [x] unit tests
  - [x] `tui/src/core/common/scoped_mutex/deadlock_prevention/policy.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/raw_mode_unix.rs`
  - [x] `tui/src/core/common/string_repeat_cache.rs`
  - [x] `tui/src/core/log/rolling_file_appender_impl.rs`
  - [x] `tui/src/core/resilient_reactor_thread/process_isolated_tests/fixtures.rs`
  - [x] `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/test_poison_recovery.rs`
  - [x] `tui/src/lib.rs`
  - [x] `.agent/skills/concurrency-safety/SKILL.md`
  - [x] `.agent/skills/concurrency-safety/patterns.md`

### Phase 2: Codebase-wide rename of integration_tests module to avoid collisions [COMPLETE]

- [x] Update all integration_tests modules / folders to following
  `<my_module_name>_integration_tests` naming pattern. And
  `#[allow(ambiguous_glob_reexports)]` and needless comments.
- [x] Update skills to reflect this

### Phase 3: Codebase-wide ScopedMutex Migration [REVIEW-PLAN]

This phase involves migrating all remaining `StdMutex` (guard-based) call sites to
`ScopedMutex` (closure-based). This is a significant architectural shift that leverages
the new `PanicOnSpecificLockNesting` policy to maintain safety while allowing legitimate
nesting.

- [ ] **Sub-task 1: Centralize Type Aliases & Policy Selection**
  - Update `tui/src/core/terminal_io/terminal_io_type_aliases.rs` to replace `StdMutex`
    with `ScopedMutex`.
  - Update `tui/src/readline_async/mod.rs` to migrate all `Safe*` type aliases.
  - **Policy Strategy:**
    - Favor `PanicOnSpecificLockNesting` for struct members (e.g., in `Readline`) to allow
      nesting different lock instances.
    - Preserve `PanicOnAnyLockNesting` for global singletons/statics (e.g.,
      `SAVED_TERMIOS`).
    - Use `None` only for high-frequency performance-critical caches (e.g.,
      `DYNAMIC_CACHE`).
- [ ] **Sub-task 2: Refactor OutputDevice & Macros**
  - Refactor `tui/src/core/terminal_io/output_device.rs`:
    - Remove `lock()` method returning a guard.
    - Add `read()` and `write()` closure-based methods, utilizing
      `PanicOnSpecificLockNesting`.
    - Ensure `lock_raw()` remains available for poison-safe cleanup.
  - Refactor or Replace `lock_output_device_as_mut!` macro:
    - Replace 70+ call sites with a closure-based
      `use_output_device!(device, |term| { ... })` or direct `device.write(|term| ...)`
      calls.
  - Update dependent macros like `queue_commands!` and `execute_commands!`.
- [ ] **Sub-task 3: Migrate Readline Async Implementation**
  - Refactor `tui/src/readline_async/readline_async_impl/readline.rs`:
    - Refactor methods (like `flush_internal`) to take mutable references instead of
      guards.
    - Audit legitimate nested locks: `line_state` + `history`, `line_state` +
      `pause_buffer`, and any lock held while writing to `OutputDevice`. Apply
      `PanicOnSpecificLockNesting`.
  - Refactor `tui/src/readline_async/spinner.rs` and `spinner_print.rs`.
- [ ] **Sub-task 4: Migrate Test Fixtures & Integration Tests**
  - Refactor `tui/src/core/test_fixtures/output_device_fixtures/stdout_mock.rs` and
    `output_device_ext.rs`.
  - Update all integration tests in `tui/src/readline_async/` and `tui/src/core/`.
- [ ] **Sub-task 5: Audit, Documentation & Cleanup**
  - Search for and remove any remaining `std::sync::Mutex` or `StdMutex` imports.
  - Update the "Poison Safety Architecture" table in `tui/src/lib.rs` to reflect the
    closure-based paradigm.
  - Ensure all new `ScopedMutex` usages are correctly documented with their chosen policy.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
  - [ ] `tui/src/core/terminal_io/terminal_io_type_aliases.rs`
  - [ ] `tui/src/core/terminal_io/output_device.rs`
  - [ ] `tui/src/readline_async/mod.rs`
  - [ ] `tui/src/readline_async/readline_async_impl/readline.rs`
  - [ ] `tui/src/readline_async/spinner.rs`
  - [ ] `tui/src/core/test_fixtures/output_device_fixtures/stdout_mock.rs`

