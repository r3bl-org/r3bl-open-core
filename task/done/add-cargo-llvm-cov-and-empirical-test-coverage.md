# Task: Add cargo-llvm-cov and Empirical Test Coverage Verification

## Overview

Enhance our "Inside-Out" test coverage methodology with empirical runtime verification
using `cargo-llvm-cov`. Rather than adopting blind line-coverage quotas (which violate the
"Zero Test Bloat" directive by incentivizing tests for compiler derives and standard
library forwards), we use AST-level custom logic classification as a semantic filter over
raw LLVM coverage data.

Key objectives:

1. Add `cargo-llvm-cov` and `llvm-tools` to `run.fish install-cargo-tools`,
   `rust-toolchain.toml`, and `script_lib.fish`.
2. Update the `check-test-coverage` skill (`SKILL.md` and `examples.md`) with instructions
   for surgical empirical validation.
3. Update agent instructions (`AGENTS.md` and `.agents/AGENTS.md`).
4. Validate installation and run an empirical test on a targeted test binary.

## Implementation plan

### Phase 1: Tooling and toolchain configuration

- [x] Add `cargo-llvm-cov` to `cargo_tools` array in `run.fish` (`install-cargo-tools`)
- [x] Add `llvm-tools` component check and install block in `run.fish`
      (`install-cargo-tools`)
- [x] Add `llvm-tools` to `components` in `rust-toolchain.toml`
- [x] Add `llvm-tools` to `toolchain_install_additional_components` in `script_lib.fish`
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `run.fish`
    - [x] `rust-toolchain.toml`
    - [x] `script_lib.fish`

### Phase 2: Skill documentation and agent rules

- [x] Add "Empirical Verification via cargo-llvm-cov" section to
      `.agents/skills/check-test-coverage/SKILL.md`
- [x] Add walkthrough example of AST filtering over `cargo llvm-cov` output to
      `.agents/skills/check-test-coverage/examples.md`
- [x] Update `check-test-coverage` descriptions in `.agents/AGENTS.md` and `AGENTS.md`
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `.agents/skills/check-test-coverage/SKILL.md`
    - [x] `.agents/skills/check-test-coverage/examples.md`
    - [x] `.agents/AGENTS.md`
    - [x] `AGENTS.md`

### Phase 3: Tooling installation and empirical verification

- [x] Run `fish run.fish install-cargo-tools` to verify idempotent installation of
      `cargo-llvm-cov` and `llvm-tools`
- [x] Run targeted empirical test coverage command to verify execution tracking
- [x] Run `./check.fish --check` to ensure no build or script regressions
- [x] **Mandatory manual review:** Verify all toolchain and check outputs.
    - [x] Verification log and results

### Phase 4: Targeted `cov` command and CLI documentation

- [x] Add `cov` / `coverage` command to `run.fish` with automatic output filtering and
      `--html` support
- [x] Update `print-help` in `run.fish` to document `cov` and ensure robust rendering
      across all terminals
- [x] Document `fish run.fish cov` in `README.md` and `tui/README.md`
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `run.fish`
    - [x] `README.md`
    - [x] `tui/src/lib.rs`
    - [x] `tui/README.md`

### Phase 5: Empirical gap fixes

- [x] Fix uncovered parsing edge cases in `tui/src/core/ansi/osc/osc_color.rs` (short spec
      `< 4` bytes, 5+ hex digits)
- [x] Re-verify coverage via `fish run.fish cov r3bl_tui osc_color` (achieved 100.00% line
      coverage)
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `tui/src/core/ansi/osc/osc_color.rs`
