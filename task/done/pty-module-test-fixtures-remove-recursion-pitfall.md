# Task: Pty Module Test Fixtures Remove Recursion Pitfall

## Status: PLANNED

## Problem
The `generate_pty_test!` macro currently requires the `controlled` function to manually call `std::process::exit(0)`. If a developer forgets this, the spawned child process continues to execute the rest of the test suite, leading to recursion, fork-bombs, and high resource consumption.

## Proposed Solution
Refactor the PTY test infrastructure to make it "fail-safe" and more ergonomic.

### 1. Macro Infrastructure Refinement
- **Automatic Termination**: Modify `generate_pty_test!` to call `std::process::exit(0)` automatically after the `controlled` closure/function returns.
- **Recursion Protection**: Use an internal safety net (`#[allow(unreachable_code)]`) to handle both cases: where the user calls `exit()` and where they don't.
- **Resource Efficiency**: Investigate and fix the high resource consumption during test runs. Ensure that spawning the child process doesn't trigger multiple full test runner instances.

### 2. Batch Refactoring
- **Surgical Cleanup**: Remove redundant `std::process::exit(0);` calls from all PTY integration tests.
- **Signature Updates**: Change `controlled` entry point signatures from `fn() -> !` to `fn()`.
- **Import Optimization**: Clean up now-redundant imports in integration test files.

### 3. Documentation & Narrative
- **Inverted Pyramid Style**: Update `generate_pty_test.rs` documentation to follow the project's standard structure.
- **Inclusive Terminology**: Ensure "controller/controlled" is used consistently instead of "master/slave".
- **Intra-doc Links**: Use reference-style links for all struct fields and usage examples to prevent link rot.

## Verification Plan
- **Typecheck**: `cargo check --tests` must pass.
- **Documentation**: `./check.fish --doc` must pass with zero unresolved link warnings.
- **Stability**: Run full test suite back-to-back 20 times:
  `for i in {1..20}; do cargo test --all-targets -- --nocapture || exit 1; done`
- **Resource Usage**: Monitor process tree during execution to ensure shallow, efficient process spawning.
