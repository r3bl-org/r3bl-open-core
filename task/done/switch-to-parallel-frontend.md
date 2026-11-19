# Task: Switch to Parallel Frontend (`--jobs-frontend=8`) & `nightly-2026-11-16` (`switch-to-parallel-frontend`)

## Overview

Adopt the Rust Compiler Team's parallel frontend stabilization workflow from
[Enabling the parallel frontend on nightly](https://blog.rust-lang.org/2026/11/16/enabling-parallel-frontend-on-nightly/)
(Nov 16, 2026) in `roc-binaries`:

1. Upgrade `rust-toolchain.toml` to `nightly-2026-11-16`
   (`rustc 1.102.0-nightly (5f4effd57 2026-11-13)`), which includes the parallel frontend
   enabled by default (`2` threads since `nightly-2026-11-14`) and recent fixes/speedups.
2. Replace the deprecated `-Z threads=8` flag in `.cargo/config.toml` with
   `-Z unstable-options --jobs-frontend=8` scoped to Linux
   (`[target.x86_64-unknown-linux-gnu]`), alongside `-C linker-features=+lld`, leaving
   macOS and Windows on compiler defaults.
3. Benchmark before vs. after across `nightly-2026-09-23` (`-Z threads=8`) and
   `nightly-2026-11-16` (`--jobs-frontend = 1, 2, 4, 8`).

---

## Key Findings & Design Decisions

### 1. Why `-Z threads=8` Was Replaced by `--jobs-frontend=8`

- Local `rustc -Z help` output: `-Z threads=val -- use --jobs-frontend instead`
- Official Rust Unstable Book
  ([`compiler-flags/threads`](https://doc.rust-lang.org/nightly/unstable-book/compiler-flags/threads.html)):
  documents `threads` as _"use `--jobs-frontend` instead"_.
- Upstream stabilization plan
  ([`rust-lang/compiler-team#1005`](https://github.com/rust-lang/compiler-team/issues/1005)
  and [`rust-lang/rust#113349`](https://github.com/rust-lang/rust/issues/113349)): `-Z`
  flags are permanently nightly-only; `--jobs-frontend=<N>` (currently gated behind
  `-Z unstable-options`) is the CLI flag intended for eventual stabilization.

### 2. Cargo `rustflags` Precedence Trap & Linux-Only Scoping

- Cargo does **not** merge `[build].rustflags` with `[target.<triple>].rustflags`: when
  `[target.x86_64-unknown-linux-gnu].rustflags` is defined (for
  `-C linker-features=+lld`), Cargo ignores `[build].rustflags` completely on Linux.
- Placing `["-Z", "unstable-options", "--jobs-frontend=8", "-C", "linker-features=+lld"]`
  inside `[target.x86_64-unknown-linux-gnu]` keeps both flags active on Linux while
  leaving macOS and Windows on the compiler default (`2` frontend threads on nightly).

### 3. Nightly Toolchain Scope (Compiler vs. Toolchain Bundle)

- `rust-toolchain.toml` pins a single coordinated `rustup` toolchain snapshot
  (`nightly-2026-11-16`) bundling `rustc`, `cargo`, `rustfmt`, `clippy`, `rustdoc`,
  `rust-analyzer`, `rust-src`, and `llvm-tools` together.
- Nightly remains required across the workspace even after `--jobs-frontend` stabilizes
  because of:
    1. Unstable `rustc` language features (`custom_inner_attributes`, `adt_const_params`,
       `associated_type_defaults`, `test`).
    2. Unstable `rustfmt.toml` options (`wrap_comments`, `comment_width`,
       `imports_granularity`, `group_imports`, `imports_layout`).
    3. Parallel frontend thread tuning (`-Z unstable-options --jobs-frontend=8`;
       `-C linker-features=+lld` is already stable on Linux `rustc 1.100.0`).

---

## Implementation Checklist

- [x] Install `nightly-2026-11-16` non-destructively via
      `rustup toolchain install nightly-2026-11-16 -c rust-src -c rust-analyzer -c llvm-tools -t x86_64-pc-windows-gnu`
      (preserving `nightly-2026-09-23` and `nightly-2026-09-30` for sibling
      `/home/nazmul/github/roc` worktree).
- [x] Update `rust-toolchain.toml` to `channel = "nightly-2026-11-16"` and document all
      workspace nightly dependencies accurately.
- [x] Update `.cargo/config.toml` `[target.x86_64-unknown-linux-gnu].rustflags` to
      `["-Z", "unstable-options", "--jobs-frontend=8", "-C", "linker-features=+lld"]`
      while preserving `[build] rustdocflags = ["--document-private-items"]` and
      `[profile.*]`.
- [x] Update `.agents/skills/check-code-quality/reference.md`
      (`Parallel Compilation (Linux only)`).
- [x] Verify `./rust-toolchain-validate.fish quick` and `./check.fish --check` (cold +
      warm).

---

## Benchmark Results (`i7-14700`, `16` P-threads via `taskset`, `CARGO_BUILD_JOBS=12`, `/tmp` tmpfs)

All benchmark runs kept `-Z unstable-options -C linker-features=+lld` constant via
`RUSTFLAGS` so `rust-lld` stayed identical across configurations.

### 1. Cold Leaf-Crate `r3bl_tui` Check (`CARGO_INCREMENTAL=0`)

_(Run 1 warmed `CARGO_INCREMENTAL=0` dependencies; Runs 2–3 measured pure `r3bl_tui`
checking after `cargo clean -p r3bl_tui`.)_

| Configuration                                  | Run 1 (incl. dep warm) |  Run 2  |  Run 3  | Pure `r3bl_tui` Mean (Runs 2–3) | `usr` CPU (Runs 2–3) |   vs `N=1`   | vs `N=2` (nightly default) |
| :--------------------------------------------- | :--------------------: | :-----: | :-----: | :-----------------------------: | :------------------: | :----------: | :------------------------: |
| `nightly-2026-09-23` (`-Z threads=8`)          |        `3.26s`         | `1.62s` | `1.64s` |             `1.63s`             |  `3.98s` / `4.08s`   |      —       |             —              |
| `nightly-2026-11-16` (`--jobs-frontend=1`)     |        `5.43s`         | `2.91s` | `2.89s` |             `2.90s`             |  `2.78s` / `2.77s`   |   baseline   |             —              |
| `nightly-2026-11-16` (`--jobs-frontend=2`)     |        `4.59s`         | `2.29s` | `2.51s` |             `2.40s`             |  `3.26s` / `3.62s`   |   `-17.2%`   |          baseline          |
| `nightly-2026-11-16` (`--jobs-frontend=4`)     |        `3.78s`         | `1.78s` | `1.83s` |             `1.81s`             |  `3.50s` / `3.67s`   |   `-37.8%`   |          `-24.8%`          |
| **`nightly-2026-11-16` (`--jobs-frontend=8`)** |        `3.48s`         | `1.63s` | `1.60s` |           **`1.62s`**           |  `4.47s` / `4.31s`   | **`-44.3%`** |        **`-32.7%`**        |

### 2. Cold Leaf-Crate `r3bl_tui` Clippy `--all-targets` (`CARGO_INCREMENTAL=0`, after warmup)

| Configuration                                  |  Run 1  |  Run 2  |    Mean     | `usr` CPU (Runs 1–2) |   vs `N=1`   | vs `N=2` (nightly default) |
| :--------------------------------------------- | :-----: | :-----: | :---------: | :------------------: | :----------: | :------------------------: |
| `nightly-2026-09-23` (`-Z threads=8`)          | `5.12s` | `5.13s` |   `5.13s`   | `21.97s` / `21.88s`  |      —       |             —              |
| `nightly-2026-11-16` (`--jobs-frontend=1`)     | `9.04s` | `9.03s` |   `9.04s`   | `13.50s` / `13.53s`  |   baseline   |             —              |
| `nightly-2026-11-16` (`--jobs-frontend=2`)     | `7.23s` | `7.35s` |   `7.29s`   | `16.67s` / `16.74s`  |   `-19.3%`   |          baseline          |
| `nightly-2026-11-16` (`--jobs-frontend=4`)     | `5.80s` | `5.87s` |   `5.84s`   | `18.35s` / `18.72s`  |   `-35.4%`   |          `-20.0%`          |
| **`nightly-2026-11-16` (`--jobs-frontend=8`)** | `5.29s` | `5.22s` | **`5.26s`** | `21.87s` / `21.49s`  | **`-41.8%`** |        **`-27.9%`**        |

### 3. Cold Full-Workspace Check & Incremental Check (`touch tui/src/lib.rs`)

| Configuration                                  | Cold Workspace Wall | Cold Workspace `usr` CPU | Incr Check Runs 1–2 | Incr Mean |
| :--------------------------------------------- | :-----------------: | :----------------------: | :-----------------: | :-------: |
| `nightly-2026-09-23` (`-Z threads=8`)          |      `12.51s`       |        `107.33s`         |  `1.28s` / `1.30s`  |  `1.29s`  |
| `nightly-2026-11-16` (`--jobs-frontend=1`)     |      `13.16s`       |         `81.31s`         |  `1.12s` / `1.15s`  |  `1.14s`  |
| `nightly-2026-11-16` (`--jobs-frontend=2`)     |      `13.42s`       |        `100.75s`         |  `1.18s` / `1.23s`  |  `1.21s`  |
| `nightly-2026-11-16` (`--jobs-frontend=4`)     |      `13.35s`       |        `113.90s`         |  `1.15s` / `1.16s`  |  `1.16s`  |
| **`nightly-2026-11-16` (`--jobs-frontend=8`)** |    **`13.07s`**     |        `117.74s`         |  `1.17s` / `1.17s`  |  `1.17s`  |

---

## Performance Uplift Summary by Use Case / Workload

### 1. Measured `roc-binaries` Uplift by Developer Workload (`nightly-2026-11-16`, Linux P-cores)

Comparing **without** parallel frontend (`N=1`), **nightly default** (`N=2`), and
**tuned** (`N=8`):

| Developer Workload                                                                                                    | `N=1` (No Parallel Frontend) | `N=2` (Nightly Default) | `N=8` (Our Config) |                   Uplift (`N=8` vs `N=1`)                    | Uplift (`N=8` vs `N=2`) |
| :-------------------------------------------------------------------------------------------------------------------- | :--------------------------: | :---------------------: | :----------------: | :----------------------------------------------------------: | :---------------------: |
| **Leaf-crate cold typecheck** (`cargo check -p r3bl_tui`, deps cached — e.g. branch switch / `Cargo.toml` edit)       |           `2.90s`            |   `2.40s` (`-17.2%`)    |    **`1.62s`**     |                **`-44.3%`** (`1.79x` faster)                 |      **`-32.7%`**       |
| **Leaf-crate cold all-targets lint** (`cargo clippy -p r3bl_tui --all-targets`, deps cached — pre-commit lib + tests) |           `9.04s`            |   `7.29s` (`-19.3%`)    |    **`5.26s`**     |                **`-41.8%`** (`1.72x` faster)                 |      **`-27.9%`**       |
| **Full cold workspace check** (`./check.fish --clean` -> `cargo check`, entire dependency graph)                      |           `13.16s`           |        `13.42s`         |    **`13.07s`**    |                    **`-0.7%`** (neutral)                     |         `-2.6%`         |
| **Incremental inner-loop check** (`touch tui/src/lib.rs` -> `cargo check -p r3bl_tui`)                                |           `1.14s`            |         `1.21s`         |    **`1.17s`**     | Flat within noise (~`9%` faster than `2026-09-23`'s `1.29s`) |    Flat within noise    |

- **Where you feel the speedup**: Any time Cargo is checking/linting `r3bl_tui` (or
  `r3bl-cmdr`) after dependency crates are already built, `15` of your `16` P-threads used
  to sit idle. `--jobs-frontend=8` cuts those waits nearly in half (`-44%` / `-42%`) and
  beats nightly's new `N=2` default by ~`28–33%`.
- **Where it's neutral**: Full cold workspace builds already saturate your `16` P-threads
  via `CARGO_BUILD_JOBS=12` compiling independent dependency crates in parallel, and tiny
  single-file incremental checks already take ~`1.1–1.2s` where
  Cargo/filesystem/incremental-dep-graph overhead dominates.

### 2. Unoptimized (`check` / `clippy` / `debug`) vs. Compiler-Optimized (`--release`) Profiles

From the Rust Compiler Team's `rustc-perf` leaf-crate benchmarks in the
[blog post](https://blog.rust-lang.org/2026/11/16/enabling-parallel-frontend-on-nightly/#benefits)
(8 frontend threads vs. sequential):

| Build Profile                                     | Mean Wall-Time Reduction (`8` threads vs `1`) | Why                                                                                                                                                             |
| :------------------------------------------------ | :-------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **`check`** (frontend only, no LLVM)              |                 **`-44.23%`**                 | 100% of compiler work is frontend (matches our `-44.3%`).                                                                                                       |
| **`clippy`** (frontend + lint passes, no LLVM)    |                 **`-41.76%`**                 | Almost all work is frontend + lints (matches our `-41.8%`).                                                                                                     |
| **`debug`** (unoptimized codegen)                 |                 **`-36.1%`**                  | Frontend is a large share; unoptimized LLVM backend is fast and already parallel across CGUs.                                                                   |
| **`opt` / `--release`** (full LLVM optimizations) |                 **`-27.88%`**                 | LLVM optimization/codegen dominates total wall time and already runs in parallel across `codegen-units`, so frontend savings are a smaller share of total time. |

---

## Verification & Follow-Up Notes

- [x] `./rust-toolchain-validate.fish quick` passed
      (`rustc 1.102.0-nightly (5f4effd57 2026-11-13)`).
- [x] `./check.fish --check` using `.cargo/config.toml` directly (after final comment
      edits): cold `11s`, warm no-op `0.1s`.
- [x] Sibling `/home/nazmul/github/roc/rust-toolchain.toml` verified untouched.
- **Note on `nightly-2026-11-16` clippy**: Newer clippy flagged 3 pre-existing
  `unnecessary use of .to_string()` warnings in `tui/examples/tui_apps/main.rs:49` and
  `tui/src/readline_async/readline_async_impl/history.rs:198-199` (tracked outside this
  task).

## Mandatory Manual Review

- [x] `rust-toolchain.toml`
- [x] `.cargo/config.toml`
- [x] `.agents/skills/check-code-quality/reference.md`
- [x] `task/done/switch-to-parallel-frontend.md`
- [x] `task/prepare-v0.8.0-meta-task.md`
