# Task: Clean Command Result Handling via `CommandOutputResult` Enum

## Background & Rationale

Across the workspace, multiple modules execute external processes using
[`std::process::Command`] or [`tokio::process::Command`]. The handling of process outcomes
suffers from common pain points:

1. **Conflating Process Spawn Failures with Process Exit Codes**: Spawning a child process
   yields `std::io::Result<std::process::Output>`. If spawning fails (such as executable
   not found, permission denied), Rust returns `Err(std::io::Error)`. If spawning
   succeeds, the process runs and returns `Ok(Output)`, which carries an exit status code
   (`output.status`).
2. **Deeply Nested or Confusing Control Flow**: Modules often use deep `match` expressions
   or multiple checks that mix error logging, diagnostics wrapping, and exit status
   warnings into a single construct.
3. **Inconsistent Tri-State Representations**: In some places (`command_runner.rs`),
   non-zero exits bail immediately. In others (`env_source/run_shell.rs`), non-zero exits
   log warnings while stdout is harvested. In still others (`package_manager.rs`), only
   clean zero exits indicate success.

### Relationship to Existing `CommandRunResult<T>`

The codebase currently contains [`CommandRunResult<T>`] in
`tui/src/core/script/command_impl/command_run_result.rs`. That type serves a different,
high-level purpose: formatting terminal UI messages and command summaries specifically for
the `giti` interactive CLI in `cmdr` (using variants `Noop`, `Run`, and `Fail`). Its
`Fail` variant bundles exit codes and spawn failures together into a `miette::Report`,
losing direct access to the underlying process streams.

In contrast, [`CommandOutputResult`] operates at the foundational OS process layer. It is
a pure, non-generic tri-state enum that preserves raw streams and exit codes without UI
formatting concerns.

### The Solution: `CommandOutputResult` Tri-State Enum

Introduce a clean, reusable enum in
`tui/src/core/script/command_impl/command_output_result.rs`:

```rust
#[derive(Debug)]
pub enum CommandOutputResult {
    /// Process was successfully spawned and exited with zero status (success).
    Success(std::process::Output),
    /// Process was successfully spawned, but exited with non-zero status.
    NonZeroExit(std::process::Output),
    /// OS failed to spawn or execute the process (e.g. executable not found, EACCES).
    SpawnFailed(std::io::Error),
}
```

Implement `From<std::io::Result<std::process::Output>>` for `CommandOutputResult` to
convert both synchronous std and asynchronous tokio process execution outputs directly
into these three explicit states.

This aligns with our design philosophy:

- **Low cognitive load**: Consumers immediately see and handle the 3 discrete outcomes via
  exhaustive pattern matching.
- **Eliminating boolean blindness**: Eliminates boolean soup and lossy predicate methods
  (`status.success()`, `is_ok()`). Call sites match on the enum variants directly,
  guaranteeing compile-time exhaustiveness and binding raw payloads without unwraps.
- **Making illegal states unrepresentable**: Eliminates awkward unwraps and impossible
  states.

---

## Work Breakdown & Implementation Plan

### Phase 1: Core Type Implementation (`command_output_result.rs`)

- [x] Create `tui/src/core/script/command_impl/command_output_result.rs`:
    - [x] Define `pub enum CommandOutputResult`.
    - [x] Implement `From<std::io::Result<std::process::Output>> for CommandOutputResult`.
    - [x] Write unit tests verifying all three variants and conversion from
          `std::io::Result<std::process::Output>`.
- [x] Export `CommandOutputResult` in `tui/src/core/script/command_impl/mod.rs`
      (`pub mod command_output_result; pub use command_output_result::*;`).
- [x] Run `./check.fish --check` to verify compilation.
- [x] Mandatory manual review:
    - [x] `tui/src/core/script/command_impl/command_output_result.rs`
    - [x] `tui/src/core/script/command_impl/mod.rs`

### Phase 2: Migrate `env_source/run_shell.rs`

- [x] Refactor `try_source_and_export_env_unix` in
      `tui/src/core/script/env_source/run_shell.rs`:
    - [x] Convert `cmd.output()` to `CommandOutputResult`.
    - [x] Match on `CommandOutputResult::SpawnFailed(err)`: log error and return wrapped
          miette error early.
    - [x] Match on `CommandOutputResult::NonZeroExit(out)`: log non-zero exit warning and
          continue parsing stdout.
    - [x] Match on `CommandOutputResult::Success(out)`: parse stdout and return
          environment map.
- [x] Refactor `try_source_and_export_env_windows` in
      `tui/src/core/script/env_source/run_shell.rs` with the identical pattern.
- [x] Run `cargo test -p r3bl_tui run_shell` to verify tests pass.
- [x] Mandatory manual review:
    - [x] `tui/src/core/script/env_source/run_shell.rs`

### Phase 3: Migrate `command_runner.rs`

- [x] Refactor `run` in `tui/src/core/script/command_impl/command_runner.rs`:
    - [x] Convert `.output().await` to `CommandOutputResult`.
    - [x] Handle `CommandOutputResult::Success(out)`: return `ok!(out.stdout)`.
    - [x] Handle `CommandOutputResult::NonZeroExit(out)`: invoke
          `bail_command_ran_and_failed!`.
    - [x] Handle `CommandOutputResult::SpawnFailed(err)`: return wrapped miette error.
- [x] Refactor `run_interactive` in `tui/src/core/script/command_impl/command_runner.rs`
      with the identical pattern.
- [x] Refactor `pipe` in `tui/src/core/script/command_impl/command_runner.rs`:
    - [x] Convert `command_one.output().await` to `CommandOutputResult`.
    - [x] Convert `child_handle.wait_with_output().await` to `CommandOutputResult`.
- [x] Run `cargo test -p r3bl_tui command_runner` to verify tests pass.
- [x] Mandatory manual review:
    - [x] `tui/src/core/script/command_impl/command_runner.rs`

### Phase 4: Migrate `package_manager.rs`

- [x] Refactor `PackageManager::detect` in `tui/src/core/script/package_manager.rs`:
    - [x] Use
          `matches!(CommandOutputResult::from(cmd.output()), CommandOutputResult::Success(_))`.
- [x] Refactor `is_command_available` in `tui/src/core/script/package_manager.rs`:
    - [x] Use
          `matches!(CommandOutputResult::from(cmd.output()), CommandOutputResult::Success(_))`.
- [x] Refactor `check_if_package_is_installed` in
      `tui/src/core/script/package_manager.rs`:
    - [x] Match on `CommandOutputResult::from(cmd.output().await)`.
    - [x] `Success` returns `ok!(true)`, `NonZeroExit` returns `ok!(false)`, `SpawnFailed`
          returns `Err(err).into_diagnostic()`.
- [x] Refactor `install_package` in `tui/src/core/script/package_manager.rs`:
    - [x] Match on `CommandOutputResult::from(cmd.output().await)`.
    - [x] `Success` returns `ok!()`, `NonZeroExit` returns miette error with stderr,
          `SpawnFailed` returns wrapped miette error.
- [x] Run `cargo test -p r3bl_tui package_manager` to verify tests pass.
- [x] Mandatory manual review:
    - [x] `tui/src/core/script/package_manager.rs`

### Phase 5: Migrate Monorepo Consumer Crates (`build-infra` & `rust-analyzer-mcp-server`)

- [x] Refactor `build-infra/src/common/cargo_fmt_runner.rs`:
    - [x] Convert `cmd.output()` to `CommandOutputResult`.
    - [x] Handle `Success`, `NonZeroExit`, and `SpawnFailed`.
- [x] Reinstall `build-infra` binary per `build-infra/AGENTS.md`:
    - [x] Run `cargo install --path build-infra --force`.
- [x] Refactor `rust-analyzer-mcp-server/src/lsp/subprocess.rs`:
    - [x] In `locate_rust_analyzer_binary`, match on
          `CommandOutputResult::from(cmd.output())`.
- [x] Reinstall `rust-analyzer-mcp-server` binary per
      `rust-analyzer-mcp-server/AGENTS.md`:
    - [x] Run `cargo install --path rust-analyzer-mcp-server --force`.
- [x] Run `cargo test -p r3bl-build-infra` and
      `cargo test -p r3bl-rust-analyzer-mcp-server`.
- [x] Mandatory manual review:
    - [x] `build-infra/src/common/cargo_fmt_runner.rs`
    - [x] `rust-analyzer-mcp-server/src/lsp/subprocess.rs`

### Phase 6: Verification & Quality Gates

- [x] Run `./check.fish --check` (verify typecheck passes across workspace).
- [x] Run `./check.fish --test` (verify all unit tests and doctests pass).
- [x] Run Windows metadata cross-compilation:
      `cargo rustc -p r3bl_tui --target x86_64-pc-windows-gnu -- --emit=metadata`.
- [x] Run `./check.fish --fmt` (format code and rustdoc).
- [x] Run `./check.fish --clippy` (verify zero warnings).
- [x] Run `./check.fish --quick-doc` (verify documentation and intra-doc links build
      cleanly).
- [x] Mandatory manual review:
    - [x] `tui/src/core/script/command_impl/command_output_result.rs`
    - [x] `tui/src/core/script/command_impl/mod.rs`
    - [x] `tui/src/core/script/command_impl/command_runner.rs`
    - [x] `tui/src/core/script/package_manager.rs`
    - [x] `tui/src/core/script/env_source/run_shell.rs`
    - [x] `build-infra/src/common/cargo_fmt_runner.rs`
    - [x] `rust-analyzer-mcp-server/src/lsp/subprocess.rs`
