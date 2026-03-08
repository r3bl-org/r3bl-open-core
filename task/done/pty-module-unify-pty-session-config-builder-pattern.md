# [PTY] Unify PtySessionConfig builder pattern across tasks

This task involves updating the `spawn_orchestrator_task` and `spawn_blocking_reader_task`
functions to accept `impl Into<PtySessionConfig>` instead of `PtySessionConfig` directly.
This ensures consistency with `PtySessionBuilder::with_config` and allows callers to use
the established "builder pattern" (using the `+` operator with
`DefaultPtySessionConfig`) directly at the call site.

## Context

`PtySessionConfig` is the central configuration struct for the Session Layer. It supports
a flexible configuration syntax:
```rust
let config = DefaultPtySessionConfig + PtySessionConfigOption::NoCaptureOutput;
```

Currently, `PtySessionBuilder` uses `impl Into<PtySessionConfig>`, but the underlying
task spawning functions take the struct directly. Unifying this improves ergonomics for
manual task orchestration and specialized testing scenarios.

## Subtasks

- [x] Update `spawn_orchestrator_task` in `tui/src/core/pty/pty_session/tasks/orchestrator.rs`
    - Change `config: PtySessionConfig` to `arg_config: impl Into<PtySessionConfig>`
    - Call `.into()` on `arg_config` at the start of the function
- [x] Update `spawn_blocking_reader_task` in `tui/src/core/pty/pty_session/tasks/reader_task.rs`
    - Change `config: PtySessionConfig` to `arg_config: impl Into<PtySessionConfig>`
    - Call `.into()` on `arg_config` at the start of the function
- [x] Verify changes by running existing PTY tests:
    - `cargo test pty`
    - `cargo test vt_100_pty_output_conformance_tests`
- [x] Run `check.fish` to ensure no regressions or lint issues


## Technical Notes

- `PtySessionConfig` and its `From` implementations are defined in
  `tui/src/core/pty/pty_session/pty_session_builder.rs`.
- `DefaultPtySessionConfig` implements `Into<PtySessionConfig>`.
- The `+` operator between `DefaultPtySessionConfig` and `PtySessionConfigOption` returns
  a `PtySessionConfig`, which itself implements `Into<PtySessionConfig>` (identity).
- Since these functions are `pub` and re-exported, this is a breaking API change for the
  Session Layer, but aligns with the project's ergonomic standards.
