# Task: Fast Cross-Platform Environment Loader in Rust (`env-source`)

## Background

In our shell setup, Fish cannot natively parse POSIX/Bash environment variable exports.
Currently, `06-environment.fish` uses `bass_source ~/.profile` to load API keys, tokens,
and PATH adjustments.

### The Problem with `bass_source` (Python)

Telemetry and profiling revealed that `bass_source` is the single biggest bottleneck in
shell startup:

1. Fish spawns `mktemp` to allocate a temporary script file on disk.
2. Fish spawns a full Python 3 runtime (`python3 -sS __bass.py`).
3. Python spawns Bash sub-process 1 to run an inline Python environment snapshot script.
4. Python spawns Bash sub-process 2 to source the file and run another Python snapshot.
5. Python computes the environment diff and writes generated Fish code to `/tmp/`.
6. Fish sources the temporary file from disk.
7. Fish spawns `rm` to delete the temporary file.

**Measured Overhead:**

- **~221 ms** total execution time (7 process spawns + temporary file disk I/O).
- Represents **over 50% of the entire shell initialization time** across the fleet.

---

## The Solution: Lean Subshell Harness in Native Rust

Instead of building a complex, fragile in-process POSIX AST parser and shell interpreter
in Rust (which fails on command substitutions `$(...)`, `eval`, functions, and external
tools like Homebrew), `posix-source` acts as a high-performance native harness around the
system `/bin/sh` or `/bin/bash` subshell:

1. **Native Execution**: Pure compiled Rust binary in `cmdr` with zero Python runtime
   overhead.
2. **Single Subshell Execution**: Spawns a single `/bin/sh` child process that sources the
   target file and outputs the final environment using null-delimited records (`env -0`).
3. **100% POSIX/Bash Compatibility**: Because a real shell executes the script, all shell
   constructs (command substitutions, directory guards, `case` blocks, nested `.`,
   functions, and `eval`) work immediately without custom interpreter logic.
4. **In-Memory Diffing**: Rust compares the child environment against the baseline
   `std::env::vars_os()`, filters out internal shell variables, and emits statements
   directly to stdout.
5. **Zero Disk I/O**: Fish consumes the output directly via pipe
   (`posix-source -i ~/.profile -o fish | source`).

**Benchmarked Latency:**

| Implementation                       | Execution Time | Process Count                                | Temp Files                |
| :----------------------------------- | :------------- | :------------------------------------------- | :------------------------ |
| `bass source ~/.profile`             | ~221 ms        | 7 processes (3x Python, 2x Bash, mktemp, rm) | Yes (`/tmp/`)             |
| `posix-source -i ~/.profile -o fish` | **~1.8 ms**    | 1 subshell (`/bin/sh`)                       | **None (pipe to stdout)** |

---

## Architecture & Data Flow

```text
~/.profile (POSIX file or bash commands)
       │
       ▼
posix-source (Rust binary in cmdr, ~1.8 ms)
       │
       ├─► 1. Capture baseline environment: std::env::vars_os()
       ├─► 2. Execute single subshell: /bin/sh -c '{ . "$1"; } >/dev/null 2>&1; env -0' -- ~/.profile
       ├─► 3. Parse null-delimited (\0) byte stream into HashMap<String, String>
       ├─► 4. Compute environment diff (added, modified, removed variables)
       ├─► 5. Filter out internal/read-only variables (PWD, SHLVL, _, BASH_FUNC_*, etc.)
       └─► 6. Format and stream directly to stdout (--output-format fish | json | dotenv)
       │
       ▼ stdout pipe
Fish Shell: posix-source -i ~/.profile -o fish | source
```

---

## Module Layout

### 1. Core Engine: `r3bl_tui` (`tui/src/core/script/posix_source/`)

The subshell executor, null parser, diff engine, and formatters live in `tui`:

```text
tui/src/core/script/posix_source/
├── mod.rs                                    # Public API: load_posix_env, apply_to_process, DEBUG_POSIX_SOURCE
├── subshell.rs                               # Subprocess execution: /bin/sh invocation and env -0 capture
├── parser.rs                                 # Null-delimited (\0) environment byte stream parser
├── diff.rs                                   # Baseline vs mutated environment diffing engine
├── filter.rs                                 # Read-only and internal variable filtering rules
├── formatters/                               # Multi-format emitters
│   ├── mod.rs
│   ├── fish.rs                               # Fish format (set -gx, PATH list arguments, set -e)
│   ├── json.rs                               # JSON format
│   └── dotenv.rs                             # Dotenv format
└── conformance_tests/                        # Conformance & Golden test suite (#[cfg(any(test, doc))])
    ├── mod.rs                                # Module declaration
    ├── test_fixtures.rs                      # Hermetic test environment builders and assertion helpers
    ├── fixtures/                             # Static input scripts (embedded via include_str!)
    │   ├── sanitized_user_profile.sh         # Real ~/.profile stripped of secrets
    │   ├── cargo_env.sh                      # Real ~/.cargo/env (rustup case pattern)
    │   ├── homebrew_env.sh                   # Real Homebrew exports & parameter expansions
    │   ├── noisy_script.sh                   # Script with loud echo (verifies stdout isolation)
    │   └── edge_cases.sh                     # Multiline variables, tricky quotes, unset, semicolons
    ├── golden/                               # Expected output golden files
    │   ├── sanitized_user_profile.fish
    │   ├── cargo_env.fish
    │   ├── noisy_script.fish
    │   ├── edge_cases.fish
    │   ├── edge_cases.json
    │   └── edge_cases.env
    └── tests/
        ├── mod.rs
        ├── test_subshell_conformance.rs      # Executes fixtures through /bin/sh and verifies diff
        └── test_golden_formatters.rs         # Compares formatted outputs against golden files
```

### 2. Binary CLI: `r3bl-cmdr` (`cmdr/src/bin/env-source.rs`)

The CLI utility lives in `cmdr`:

- **Binary Target**: `[[bin]] name = "env-source"` in `cmdr/Cargo.toml` pointing to
  `src/bin/env-source.rs`.
- **CLI Interface**:
    ```bash
    env-source --input-file ~/.profile --output-format fish       # Fish syntax (set -gx)
    env-source --input-file ~/.profile --output-format json       # JSON dictionary
    env-source --input-file ~/.profile --output-format dotenv     # .env KEY=VALUE format
    env-source --input-command "source ~/.profile" --output-format fish # Inline command
    ```
- **Performance**: Lean synchronous entry point with direct stdout locking and zero
  asynchronous runtime overhead.

---

## Output Formatting Rules

### 1. Fish Format (`-o fish`)

- **Standard Variable Addition/Modification**:
    ```fish
    set -gx KEY 'escaped_value';
    ```
- **PATH Variable Handling**: Splits colon-separated paths and emits Fish list items so
  `PATH` remains an array:
    ```fish
    set -gx PATH '/home/nazmul/scripts/dev' '/home/nazmul/bin' '/home/nazmul/.local/bin';
    ```
- **Variable Removal (Unset)**:
    ```fish
    set -e REMOVED_KEY;
    ```
- **Ignored / Filtered Variables**:
    - Fish read-only variables: `PWD`, `SHLVL`, `history`, `pipestatus`, `status`,
      `version`, `FISH_VERSION`, `fish_pid`, `hostname`, `_`, `fish_private_mode`.
    - Shell-internal variables: `PS1`, `XPC_SERVICE_NAME`, and Bash exported functions
      starting with `BASH_FUNC_`.

### 2. JSON Format

Emits a JSON object containing added, modified, and removed entries:

```json
{
    "added": {
        "KEY": "VALUE"
    },
    "modified": {
        "PATH": "/new/path:/old/path"
    },
    "removed": ["OLD_KEY"]
}
```

### 3. Dotenv Format

Emits key-value assignments suitable for `.env` files:

```text
KEY="VALUE"
PATH="/new/path:/old/path"
```

### 4. PowerShell Format (`-o powershell`, Windows-only `#[cfg(windows)]`)

Emits PowerShell commands for Windows:

```powershell
$env:KEY = 'escaped_value';
$env:PATH = 'C:\new\path;C:\old\path';
Remove-Item -Path 'env:REMOVED_KEY' -ErrorAction SilentlyContinue;
```

- Single quotes `'` in values are escaped by doubling them (`'it''s fine'`).

---

## Conformance Testing Strategy & Hermetic Mock Baseline

Following the two-tier testing pattern established in `vt_100_pty_output_parser` and
`cargo-rustdoc-fmt`, the test suite is structured into two tiers:

1. **Tier 1 (Fast In-Memory Unit Tests)**: Embedded directly inside module files
   (`parser.rs`, `filter.rs`, `diff.rs`, `formatters/*.rs`) to test parsing edge cases,
   null splitting, and output formatting with zero subprocess spawns.
2. **Tier 2 (Hermetic Subshell Conformance & Golden Tests)**: Located in a dedicated
   `conformance_tests/` sub-module inside `tui/src/core/script/posix_source/`, executing
   real-world shell scripts through `/bin/sh` and asserting exact golden outputs.

### Hermetic Baseline Mocking (`base_env: Option<&HashMap<String, String>>`)

To prevent tests from depending on ambient host environment variables (such as `$HOME`,
`$USER`, or local `$PATH`), `load_posix_env` accepts an optional baseline environment:

- **Production Mode (`None`)**: The subshell naturally inherits the calling process
  environment (`std::env::vars_os()`).
- **Test Mode (`Some(&mock_env)`)**: The subshell clears ambient variables via
  `Command::env_clear()` and injects a deterministic mock baseline (`HOME=/home/testuser`,
  `USER=testuser`, `PATH=/usr/bin:/bin`).

This guarantees that golden file assertions (`assert_eq2!`) succeed deterministically on
any machine and across Linux and macOS runners in CI/CD.

---

## Phases

### Phase 1: Core Subshell Runner, Diff Engine & Formatters in `tui`

- [x] Create module directory `tui/src/core/script/posix_source/`.
- [x] Define `DEBUG_POSIX_SOURCE` constant in `tui/src/core/script/posix_source/mod.rs`
      and wire structured tracing behind `.then(|| { ... })` with
      `// % is Display, ? is Debug.`.
- [x] Implement `subshell.rs`:
    - [x] Spawn `/bin/sh` with `{ . "$1"; } >/dev/null 2>&1; env -0` for complete stdout
          isolation.
    - [x] Support hermetic baseline injection (`Option<&HashMap<String, String>>`) via
          `Command::env_clear()`.
    - [x] Support inline shell command evaluation (`-c`).
    - [x] Capture raw stdout bytes containing null-delimited environment records.
- [x] Implement `parser.rs`:
    - [x] Parse null-delimited (`\0`) byte stream into `HashMap<String, String>` using
          safe `String::from_utf8_lossy`.
- [x] Implement `filter.rs`:
    - [x] Filter out `FISH_READONLY` variables, `PS1`, `XPC_SERVICE_NAME`, and
          `BASH_FUNC_*` functions.
- [x] Implement `diff.rs`:
    - [x] Compare mutated environment against baseline (`std::env::vars_os()` or mock).
    - [x] Classify entries into added, modified, and removed sets.
- [x] Implement `formatters/`:
    - [x] `fish.rs`: Fish list escaping, `set -gx`, `PATH` splitting, and `set -e`.
    - [x] `json.rs`: JSON serialization.
    - [x] `dotenv.rs`: Key-value `.env` serialization.
- [x] Implement `apply_to_process()` in `tui/src/core/script/posix_source/mod.rs` to allow
      in-process loading for Rust binaries.
- [x] Implement `conformance_tests/` test suite with embedded fixtures and golden files:
    - [x] `sanitized_user_profile.sh` -> `sanitized_user_profile.fish`
    - [x] `cargo_env.sh` -> `cargo_env.fish`
    - [x] `homebrew_env.sh`
    - [x] `noisy_script.sh` -> `noisy_script.fish` (verifies stdout isolation)
    - [x] `edge_cases.sh` -> `edge_cases.fish`, `edge_cases.json`, `edge_cases.env`
- [x] Add unit tests for null parsing, diffing, and all output formatters.
- [x] Mandatory manual review:
    - [x] `tui/src/core/script/posix_source/mod.rs`
    - [x] `tui/src/core/script/posix_source/core.rs`
    - [x] `tui/src/core/script/posix_source/subshell.rs`
    - [x] `tui/src/core/script/posix_source/parser.rs`
    - [x] `tui/src/core/script/posix_source/filter.rs`
    - [x] `tui/src/core/script/posix_source/diff.rs`
    - [x] `tui/src/core/script/posix_source/formatters/mod.rs`
    - [x] `tui/src/core/script/posix_source/formatters/fish.rs`
    - [x] `tui/src/core/script/posix_source/formatters/json.rs`
    - [x] `tui/src/core/script/posix_source/formatters/dotenv.rs`
    - [x] `tui/src/core/script/posix_source/conformance_tests/mod.rs`
    - [x] `tui/src/core/script/posix_source/conformance_tests/test_fixtures.rs`
    - [x] `tui/src/core/script/posix_source/conformance_tests/tests/test_subshell_conformance.rs`
    - [x] `tui/src/core/script/posix_source/conformance_tests/tests/test_golden_formatters.rs`

### Phase 2: Binary CLI in `cmdr` & Performance Benchmarking

- [x] Add `[[bin]] name = "posix-source"` in `cmdr/Cargo.toml`.
- [x] Implement `cmdr/src/bin/posix-source.rs`:
    - [x] CLI argument parsing via `clap` (`-i`/`--input`, `-c`/`--command`,
          `-o`/`--output-format`).
    - [x] Write formatted output directly to stdout.
- [x] Add integration tests verifying end-to-end execution against real environment
      scripts (`~/.profile`, `.cargo/env`).
- [x] Benchmark execution time and verify latency is under 3 ms.
- [x] Mandatory manual review:
    - [x] `cmdr/Cargo.toml`
    - [x] `cmdr/src/bin/posix-source.rs`

### Phase 3: Fish Integration & Startup Optimization

- [x] Update `~/scripts/fish/core/06-environment.fish` to use:
    ```fish
    if command -v posix-source >/dev/null 2>&1
        posix-source -i ~/.profile -o fish | source
    end
    ```
- [x] Profile shell startup with `fish --profile-startup` to verify latency drop from ~221
      ms to ~2 ms.
- [x] Mandatory manual review:
    - [x] `~/scripts/fish/core/06-environment.fish`

### Phase 4: Fleet-Wide Migration & Deprecation of `fish-bass` (Linux & macOS)

- [x] Build and install release binary `posix-source` across all fleet platforms (Linux
      x86_64, Linux aarch64, macOS).
- [x] Update all environment loaders and fish scripts in dotfiles to use `posix-source`
      unconditionally.
- [x] Remove `bass` references and installation steps from
      `~/scripts/local-backup-restore/fresh-install/` (such as
      `02-restore-data-from-backup.fish` and
      `fresh-install-steps/01-fix-dot-profile-env-vars.fish`).
- [x] Remove `fish-bass` plugin files (`00-bass.fish`, `__bass.py`) and Python runtime
      dependencies from dotfiles manifests.
- [x] Sync configurations across the fleet via `env-save -w`.
- [x] Measure interactive SSH startup time across all hosts (`nazmul-desktop`,
      `nazmul-laptop`, `nazmul-mobile`, macOS machines).
- [x] Mandatory manual review:
    - [x] `~/scripts/local-backup-restore/fresh-install/02-restore-data-from-backup.fish`
    - [x] `~/scripts/local-backup-restore/fresh-install/fresh-install-steps/01-fix-dot-profile-env-vars.fish`
    - [x] Fleet sync status and verification logs

### Phase 5: Cross-Platform Extension (`env-source`) & Windows PowerShell Support

- [x] Refactor & Rename `posix-source` to `env-source`:
    - [x] Update `cmdr/Cargo.toml` (`[[bin]] name = "env-source"`).
    - [x] Rename `cmdr/src/bin/posix-source.rs` to `cmdr/src/bin/env-source.rs`.
    - [x] Rename `cmdr/src/posix_source/` to `cmdr/src/env_source/`.
    - [x] Rename `tui/src/core/script/posix_source/` to `tui/src/core/script/env_source/`
          and implement `pub fn env_source`, `InputKind`, and `BaseEnv`.
    - [x] Update barrel re-exports in `tui/src/core/script/mod.rs` and `tui/src/lib.rs`.
- [x] Refactor `SourceKind` to `InputKind` & Update CLI Argument Scheme:
    - [x] Rename enum `SourceKind` to `InputKind` across `r3bl_tui` and `r3bl_cmdr`.
    - [x] Update CLI options in `cmdr/src/env_source/clap_config.rs` to use `--input-file`
          (aliases: `input`, `file`) and `--input-command` (alias: `command`).
    - [x] Rename clap `ArgGroup` from `"source"` to `"input"`.
- [x] Implement `#[cfg(windows)]` PowerShell Formatter (`-o powershell`):
    - [x] Add `#[cfg(windows)] Powershell` variant to `OutputFormat` in
          `tui/src/core/script/env_source/formatters/mod.rs`.
    - [x] Add `#[cfg(unix)]` to `OutputFormat::Fish` to prevent selection on Windows.
    - [x] Implement `tui/src/core/script/env_source/formatters/powershell.rs` with
          single-quote doubling (`'it''s fine'`).
    - [x] Add unit tests for PowerShell escaping and deletion
          (`Remove-Item -Path 'env:KEY' -ErrorAction SilentlyContinue`).
- [x] Implement `#[cfg(windows)]` Subshell Engine & Parser:
    - [x] Add Windows subshell runner in `subshell.rs` using
          `cmd.exe /c "(call %1) >nul 2>&1 & set"`.
    - [x] Add `parse_env_windows` in `parser.rs` for newline-delimited `set` output.
    - [x] Add case-insensitive key comparison in `diff.rs` on Windows to prevent duplicate
          entries like `PATH` and `Path`.
- [x] Add Actionable Error Handling & Diagnostics:
    - [x] Add `miette::WrapErr` context to `/bin/sh` and `cmd.exe` subprocess invocations
          in `subshell.rs`.
- [x] Verification & Cross-Compilation Checks:
    - [x] Verify Linux build: `./check.fish --check`, `./check.fish --test`,
          `./check.fish --clippy`, `./check.fish --quick-doc`.
    - [x] Verify Windows metadata cross-compilation:
          `cargo rustc -p r3bl_tui --target x86_64-pc-windows-gnu -- --emit=metadata`.
    - [x] Verify Windows binary cross-compilation:
          `cargo check -p r3bl_cmdr --bin env-source --target x86_64-pc-windows-gnu`.
    - [x] Native Windows build & test execution on `nazmul-win.local` over SSH:
        - [x] Run `cargo test` in `r3bl_tui` and `r3bl_cmdr` on `nazmul-win.local`.
        - [x] Verify end-to-end `.bat` evaluation
              (`env-source --input-file test.bat --output-format powershell`) inside
              native PowerShell on Windows.
- [x] Restructure and Align `conformance_tests/` for Unix & Windows:
    - [x] Align test directory naming with `md_parser` and `vt_100_pty_output_parser`
          conventions:
        - [x] Replace `fixtures/` and `golden/` with `test_data/` containing
              `input/` and `expected_output/` subdirectories.
        - [x] Use clean matching basenames without redundant `input_`/`expected_` prefixes.
        - [x] Create `test_data/AGENTS.md` for test data integrity documentation.
        - [x] Create `test_fixtures.rs` dedicated to test harness helpers, mock
              initial environment builders (`create_mock_initial_env_unix`,
              `create_mock_initial_env_windows`), and temp script runners
              (`run_fixture_sh`, `run_fixture_bat`).
    - [x] Un-gate `test_golden_formatters.rs` to run on all platforms, and add PowerShell
          golden tests (`test_golden_cargo_env_powershell`,
          `test_golden_noisy_script_powershell`,
          `test_golden_edge_cases_all_formats_windows`).
    - [x] Split subshell tests into `test_subshell_unix.rs` (`#[cfg(unix)]`) and
          `test_subshell_windows.rs` (`#[cfg(windows)]`).
    - [x] Update Windows filter in `filter.rs` to ignore `PROMPT` (the Windows equivalent
          of Unix `PS1`), and include `PATHEXT` in Windows mock environment.
    - [x] Update `run_shell.rs` on Windows to handle script paths with spaces using
          `raw_arg`.
- [x] Mandatory manual review:
    - [x] `cmdr/Cargo.toml`
    - [x] `cmdr/src/bin/env-source.rs`
    - [x] `cmdr/src/env_source/clap_config.rs`
    - [x] `cmdr/src/env_source/mod.rs`
    - [x] `tui/src/core/script/env_source/args.rs`
    - [x] `tui/src/core/script/env_source/core.rs`
    - [x] `tui/src/core/script/env_source/filter.rs`
    - [x] `tui/src/core/script/env_source/formatters/mod.rs`
    - [x] `tui/src/core/script/env_source/formatters/powershell.rs`
    - [x] `tui/src/core/script/env_source/run_shell.rs`
    - [x] `tui/src/core/script/env_source/parser.rs`
    - [x] `tui/src/core/script/env_source/diff.rs`
    - [x] `tui/src/core/script/env_source/conformance_tests/mod.rs`
    - [x] `tui/src/core/script/env_source/conformance_tests/test_data/mod.rs`
    - [x] `tui/src/core/script/env_source/conformance_tests/test_data/AGENTS.md`
    - [x] `tui/src/core/script/env_source/conformance_tests/test_fixtures.rs`
    - [x] `tui/src/core/script/env_source/conformance_tests/tests/mod.rs`
    - [x] `tui/src/core/script/env_source/conformance_tests/tests/test_golden_formatters.rs`
    - [x] `tui/src/core/script/env_source/conformance_tests/tests/test_subshell_unix.rs`
    - [x] `tui/src/core/script/env_source/conformance_tests/tests/test_subshell_windows.rs`

### Phase 6: Dotfiles Update & Fleet Deployment for `env-source`

- [x] Update `~/scripts/fish/core/06-environment.fish` to invoke `env-source`:
    ```fish
    if command -v env-source >/dev/null 2>&1
        env-source -i ~/.profile -o fish | source
    end
    ```
- [x] Build and install release binary `env-source` locally
      (`cargo install --path cmdr --bin env-source --force`).
- [x] Update any dotfiles / fresh-install scripts referencing `posix-source` to
      `env-source`.
- [x] Deploy `env-source` binary and sync configurations across the fleet via
      `env-save -w`.
- [x] Verify interactive shell startup time across fleet hosts.
- [x] Mandatory manual review:
    - [x] `~/scripts/fish/core/06-environment.fish`
    - [x] Dotfiles references
    - [x] Fleet sync status logs

### Phase 7: Documentation & Metadata Updates for `env-source`

- [x] Update `cmdr/README.md`:
    - [x] Update table of contents to include `Run env-source binary target`.
    - [x] Update introduction and app list to include `env-source` alongside `edi` and
          `giti`.
    - [x] Add dedicated `Run env-source binary target` section with CLI options and usage
          examples (POSIX `sh`, Fish, PowerShell, Dotenv).
- [x] Update `cmdr/src/lib.rs`:
    - [x] Synchronize crate-level rustdoc comments with `cmdr/README.md` to ensure
          `cargo doc` and `docs.rs` reflect `env-source`.
- [x] Update root `README.md`:
    - [x] Add `env-source` to the main binary crate (`r3bl-cmdr`) list and version check
          examples.
    - [x] Update tooling setup and `run.fish` reference tables to include `env-source` in
          `cmdr` binary lists.
- [x] Update `cmdr/Cargo.toml`:
    - [x] Update `package.description` to mention `env-source` alongside `giti` and `edi`.
- [x] Update `run.fish`:
    - [x] Update `install-cmdr` help text and echo messages to list `env-source` (`edi`,
          `giti`, `rc`, `env-source`).
- [x] Verification:
    - [x] Run `./check.fish --quick-doc` to ensure rustdoc links and docs build without
          warnings.
    - [x] Run `./check.fish --check` to ensure workspace compilation.
- [x] Mandatory manual review:
    - [x] `cmdr/README.md`
    - [x] `cmdr/src/lib.rs`
    - [x] `README.md`
    - [x] `cmdr/Cargo.toml`
    - [x] `run.fish`

<!-- cspell:words pipestatus SHLVL testuser -->
