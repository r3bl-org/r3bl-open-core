# Task: Reorganize `resilient_reactor_thread` test modules

## Overview

`rrt_restart_tests.rs` (1,290 lines) mixes three fundamentally different test categories:
pure unit tests (Group A), mock-based tests requiring subprocess isolation (Groups B/C),
and a PTY integration test.

The goal is a 3-directory structure where the directory name answers **why** each test is
isolated:
- **unit_tests/** - Stateless, parallel (Group A backoff logic)
- **process_isolated_tests/** - Global mock state pollution (Groups B/C via
  `generate_isolated_process_test!`)
- **integration_tests/** - Needs real PTY fd for epoll (1 PTY test per file convention)

Target layout:

```
tui/src/core/resilient_reactor_thread/
├── mod.rs                                     # replaces `pub mod tests` with 3 new modules
├── unit_tests/
│   ├── mod.rs
│   └── group_a_backoff_logic.rs               # Group A tests
├── process_isolated_tests/
│   ├── mod.rs                                 # generate_isolated_process_test! + dispatcher
│   ├── fixtures.rs                            # Shared mock infrastructure (TestWorker, etc)
│   ├── group_b_run_worker_loop.rs             # Group B tests
│   └── group_c_rrt_integration.rs             # Group C tests
└── integration_tests/
    ├── mod.rs                                 # only PTY test module declarations
    ├── pty_test_production_factory_restart.rs  # Real PTY restart cycle test
    └── pty_test_production_poll_error.rs       # Real PTY epoll error test
```

## Implementation plan

### Phase 1: Move files into place

- [x] Extract Group A to `unit_tests/group_a_backoff_logic.rs`
- [x] Extract Group B to `process_isolated_tests/group_b_run_worker_loop.rs`
- [x] Extract Group C to `process_isolated_tests/group_c_rrt_integration.rs`
- [x] Extract mock infrastructure to `process_isolated_tests/fixtures.rs`
- [x] Migrate PTY tests to `integration_tests/` (1 test per file)

### Phase 2: Write and simplify `mod.rs` files

- [x] Create `process_isolated_tests/mod.rs` with `generate_isolated_process_test!` +
      `run_all_restart_tests_sequentially` dispatcher
- [x] Simplify `integration_tests/mod.rs` to only declare PTY test modules

### Phase 3: Delete stale files

- [x] Delete duplicate or old test files
- [x] `rm -r tests/` (old monolithic directory)

### Phase 4: Wire up parent module

- [x] In `resilient_reactor_thread/mod.rs`, replace `pub mod tests;` with:
      `pub mod unit_tests;`, `pub mod process_isolated_tests;`, `pub mod integration_tests;`
      (Used `#[cfg(any(test, doc))]` to allow subprocess macros and docs while keeping 
      production builds clean).

### Phase 5: Verify

- [x] `./check.fish --check`
- [x] `cargo test -p r3bl_tui resilient_reactor_thread -- --nocapture`
- [x] `cargo test -p r3bl_tui group_a_backoff_logic`
- [x] `cargo test -p r3bl_tui test_production_poll_error -- --nocapture`
- [x] `cargo test -p r3bl_tui test_rrt_restart_in_isolated_process -- --nocapture`
- [x] `./check.fish --clippy`
