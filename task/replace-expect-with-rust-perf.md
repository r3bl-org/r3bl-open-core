# Task: Replace expect with In-Repo Rust PTY Driver for Performance Profiling

## Overview

The automated flamegraph benchmarking workflow
(`./run.fish run-examples-flamegraph-fold --benchmark`, `/analyze-performance`) currently
relies on the external Unix tool `expect` to drive the `ex_editor` example headlessly in a
pseudoterminal (PTY).

While effective, using `expect` introduces several drawbacks:

1. **External dependency**: Requires host installation of `expect` and Tcl
   (`sudo apt install expect`), which is not guaranteed across environments.
2. **Platform limitations**: `expect` is Unix-specific and does not run natively on
   Windows without MSYS2/Cygwin.
3. **Brittle shell scripts**: Keystroke sequences and ANSI escape codes in
   `script_lib.fish` are embedded as raw, escaped string literals (`send \"\x1b\[D\"`).
4. **Architectural divergence**: The repository already possesses a comprehensive,
   cross-platform PTY test infrastructure in `r3bl_tui` (`portable_pty`, `PtyPair`,
   `PtyTestChild`, `drain_and_wait`), making `expect` an unnecessary external redundancy.

This task replaces the `expect` script with a dedicated, in-repo Rust PTY driver
(`flamegraph_benchmark_driver`). The driver uses our existing PTY primitives to allocate a
virtual terminal of identical geometry (60 rows x 220 cols), launch the example binary,
inject the deterministic 25-step workload using type-safe key definitions, and cleanly
shut down without PTY buffer deadlocks.

## Architecture and Design

### 1. Dedicated Rust Driver Binary

A dedicated example binary (`tui/examples/flamegraph_benchmark_driver.rs`) will act as the
controller:

- Accepts the target binary path as an argument.
- Configures terminal geometry to 60 rows by 220 columns via `PtyPair::open_and_spawn`.
- Sends the exact sequence of keystrokes:
    - Startup delay to allow initial render.
    - Option `3\r` to enter `ex_editor`.
    - The 25 benchmark operations (text entry, modal dialog toggling, cursor navigation).
    - Graceful exit (`q`).
- Prevents PTY buffer deadlock by draining the buffer while awaiting exit
  (`drain_and_wait`).

### 2. Integration with perf and script_lib.fish

In `script_lib.fish`:

- Replace the `expect` dependency check with a check/build for the Rust driver.
- Run
  `sudo perf record -g --call-graph=fp,8 -F 999 -o perf.data -- $driver_path $binary_path`.
- The driver spends the vast majority of its time blocked in I/O or sleep while the child
  process does all the CPU-heavy rendering, ensuring `perf` captures the TUI rendering
  pipeline cleanly.

## Implementation plan

### Phase 1: Create Rust PTY Benchmark Driver

- [ ] Create `tui/examples/flamegraph_benchmark_driver.rs` using `PtyPair` and
      `PtyTestChild`.
- [ ] Implement command line argument parsing for the target binary path.
- [ ] Implement terminal dimensions configuration (60 rows x 220 cols).
- [ ] Port the 25 scripted operations from `script_lib.fish` into structured Rust
      constants and byte arrays.
- [ ] Implement graceful shutdown and buffer draining via `drain_and_wait`.
- [ ] Test the driver standalone against `tui_apps` to verify clean execution and exit.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/examples/flamegraph_benchmark_driver.rs`

### Phase 2: Integrate Driver into script_lib.fish and run.fish

- [ ] Update `run_example_with_flamegraph_profiling_perf_fold` in `script_lib.fish` to
      compile the benchmark driver before profiling.
- [ ] Update `run_benchmark_with_scripted_input` in `script_lib.fish` to execute the Rust
      driver under `perf record` instead of `expect`.
- [ ] Remove `expect` checks and system package installation instructions from
      `script_lib.fish`.
- [ ] Run `./run.fish run-examples-flamegraph-fold --benchmark` and verify
      `flamegraph-benchmark.perf-folded` is generated successfully.
- [ ] Compare sample distribution against `tui/flamegraph-benchmark-baseline.perf-folded`
      to ensure benchmark comparability.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `script_lib.fish`
    - [ ] `run.fish`

### Phase 3: Documentation and Cleanup

- [ ] Update `.agents/skills/analyze-performance/SKILL.md` to reference the Rust driver
      rather than `expect`.
- [ ] Update comments and prerequisites in `run.fish` and `setup-dev-tools.sh` /
      `bootstrap.sh` if applicable.
- [ ] Run `./check.fish --check` to ensure all targets compile cleanly.
- [ ] Format markdown files using `prettier --write`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `.agents/skills/analyze-performance/SKILL.md`
    - [ ] `run.fish`
    - [ ] `task/replace-expect-with-rust-perf.md`
