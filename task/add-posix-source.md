# Task: Fast POSIX Environment Loader in Rust (`posix-source`)

## Background

In our shell setup, Fish cannot natively parse POSIX/Bash environment variable exports.
Currently, `06-environment.fish` uses `bass_source ~/.profile` to load API keys, tokens,
and PATH adjustments.

### The Problem with `bass_source` (Python)

Telemetry and profiling revealed that `bass_source` is the single biggest bottleneck in
shell startup:

1. Spawns a full Python 3 runtime (`python3 -sS __bass.py`).
2. Spawns a child Bash sub-process (`bash -c "source ~/.profile; env"`).
3. Python computes an environment diff and writes a temp file to `/tmp/`.
4. Fish sources the temporary file.
5. Fish spawns `rm` to delete the temp file.

**Measured Overhead:**

- **~127 ms** on mobile CPUs (`nazmul-mobile`).
- **~60 ms** on desktop workstations (`nazmul-desktop`, `nazmul-laptop`).
- Represents **over 50% of the entire shell initialization time** across the fleet.

---

## Goals

Create a lightweight, ultra-fast native Rust utility (`posix-source`) in `cmdr` to parse
POSIX shell environment files and emit native Fish commands in **< 1 ms**, mimicking
`source ~/.profile` natively for Fish shell.

1. **Sub-Millisecond Execution:**
    - Pure compiled native Rust machine code.
    - Zero Python VM startup overhead.
    - Zero temporary disk files.
2. **Robust POSIX Parsing:**
    - Parse `export KEY="VALUE"`, `export KEY='VALUE'`, and `export KEY=VALUE`.
    - Handle variable substitutions (e.g., `PATH="$HOME/.local/bin:$PATH"`).
    - Handle `unset VAR`.
3. **Multi-Format Code Output:**
    - Emit valid Fish statements directly to stdout (e.g., `set -gx KEY 'VALUE';`).
    - Emit valid Bash, JSON, and Dotenv outputs via `--format <fish|bash|json|dotenv>`.
    - Allow direct evaluation in Fish via `posix-source ~/.profile | source`.

---

### Crate Placement & Architecture

### 1. Core Engine: `r3bl_tui` (`tui/src/core/script/posix_env/`)

The parser, evaluator, and formatters live in **`tui`** under
`r3bl_tui::core::script::posix_env`:

- **Reusability**: Any Rust code or script in `r3bl-open-core` can parse POSIX environment
  files and apply them to the current process via `apply_to_process()` without shelling
  out.
- **Output Formatter**: Pluggable formatters for `fish` (default), `bash`, `json`, and
  `dotenv`.
- **Zero Heavy Dependencies**: Uses `nom 8` (already in `tui`) with zero async/Tokio
  overhead.

### 2. Binary CLI: `r3bl-cmdr` (`cmdr/src/bin/posix-source.rs`)

The CLI utility is located in **`cmdr`**:

- **Binary Target**: `[[bin]] name = "posix-source"` in `cmdr/Cargo.toml` pointing to
  `src/bin/posix-source.rs`.
- **CLI Options**:
    ```bash
    posix-source ~/.profile                  # Default: fish syntax
    posix-source --format fish ~/.profile    # Explicit fish syntax
    posix-source --format bash ~/.profile    # POSIX / Bash export syntax
    posix-source --format json ~/.profile    # JSON dictionary
    posix-source --format dotenv ~/.profile  # .env KEY=VALUE format
    ```
- **Sub-Millisecond Strategy**: Lean synchronous `fn main()` with zero Tokio runtime
  startup overhead, writing directly to locked stdout.

---

## Proposed Architecture & `bass` Compatibility

### Emulating `bass` (`__bass.py`) Semantics in Native Rust

In `fish-bass`, `__bass.py` runs a sub-shell, captures environment diffs, and emits Fish
commands. `posix-source` replaces the Python sub-process pipeline with a native Rust
evaluation engine:

1. **In-Memory Environment State**:
    - Initialize an in-memory `HashMap<String, String>` from the current process
      environment (`std::env::vars()`).
    - Process statements sequentially so subsequent lines observe earlier assignments and
      expansions (e.g., `PATH="$HOME/bin:$PATH"`).

2. **`nom 8` Zero-Copy Parser**:
    - High performance, zero-allocation parser combinators operating directly on `&str`
      slices.
    - Parses `export KEY=VALUE`, `export KEY="VALUE"`, `export KEY='VALUE'`, `KEY=VALUE`,
      and `unset KEY`.
    - Recognizes quotes, escapes (`\"`, `\\`), inline comments (`#`), and variable
      references (`$VAR`, `${VAR}`).
    - Recognizes directory/file guard checks (`if [ -d "$DIR" ]; then ... fi`,
      `[ -f "$FILE" ] && . "$FILE"`) and evaluates them via fast `std::path::Path` checks
      (< 1 µs).
    - Handles nested file sourcing (`. "$HOME/.cargo/env"` or `source <path>`).

3. **Formatters (`fish`, `bash`, `json`, `dotenv`)**:
    - Compares modified/added variables against the initial environment state.
    - **`fish` (Default)**:
        - Splits `PATH` on colons and formats as Fish list arguments:
          `set -gx PATH '/home/nazmul/.local/bin' '/home/nazmul/bin' ...;`
        - For other variables: Emits `set -gx KEY 'value';`
        - For unsets: Emits `set -e KEY;`
        - Filters out `FISH_READONLY` and `IGNORED` variables matching `__bass.py`.
    - **`bash`**: Emits `export KEY='value';` and `unset KEY;`.
    - **`json`**: Emits JSON object with keys, values, and PATH list.
    - **`dotenv`**: Emits `KEY="value"` pairs.

```
~/.profile (POSIX file)
       │
       ▼
posix-source (Rust binary in cmdr, <1ms)
       │
       ▼ (calls r3bl_tui::core::script::posix_env)
       ├─► Evaluates file & env substitutions in-memory
       ├─► Evaluates [ -d "$DIR" ] directory guards
       └─► Evaluates nested . "$HOME/.cargo/env"
       │
       ▼ stdout (--format fish | bash | json | dotenv)
set -gx GITHUB_TOKEN '...';
set -gx PATH '/home/nazmul/.local/bin' '/home/nazmul/bin' ...;
       │
       ▼
Fish Shell (`source` via pipe)
```

---

## Phases

### Phase 1: Architecture & `nom` Parser Design in `tui`

- [ ] Define AST types (`Statement`, `ValueExpr`, `ValuePart`, `Condition`) in
      `tui/src/core/script/posix_env/ast.rs`.
- [ ] Implement `nom` parsers in `tui/src/core/script/posix_env/parser.rs`:
    - [ ] Identifiers and keyword matching (`export`, `unset`, `source`, `.`).
    - [ ] Single-quoted, double-quoted, and unquoted value parsers with variable
          interpolation (`$VAR`, `${VAR}`).
    - [ ] Directory and file condition guards (`if [ -d ... ]`, `if [ -f ... ]`).
    - [ ] Inline comments and whitespace stripping.
- [ ] Implement evaluation engine in `tui/src/core/script/posix_env/evaluator.rs`:
    - [ ] In-memory environment tracking and variable expansion (`std::env::vars()`
          baseline).
    - [ ] Filesystem guard evaluation (`is_dir()`, `is_file()`).
    - [ ] Nested file inclusion / sourcing.
- [ ] Implement formatters in `tui/src/core/script/posix_env/formatters/`:
    - [ ] `fish.rs` (default): `set -gx` statements, `PATH` list, `set -e`,
          `FISH_READONLY` filtering.
    - [ ] `bash.rs`: `export KEY='value'`, `unset KEY`.
    - [ ] `json.rs`: JSON output representation.
    - [ ] `dotenv.rs`: `.env` key-value pairs.
- [ ] Implement in-process loading: `apply_to_process()` in
      `tui/src/core/script/posix_env/mod.rs`.
- [ ] Mandatory manual review:
    - [ ] `task/add-posix-source.md`

### Phase 2: Implementation & Unit Tests in `tui` & `cmdr`

- [ ] Implement `tui/src/core/script/posix_env/` modules with comprehensive unit tests
      for:
    - [ ] Simple key-value exports.
    - [ ] Double-quoted and single-quoted strings with spaces.
    - [ ] Export statements with inline comments (`# ...`).
    - [ ] Prepending/appending to existing environment variables (e.g., `$PATH`).
    - [ ] Unset commands (`unset DISPLAY`).
    - [ ] Directory guards (`if [ -d ... ]`) and sourcing (`. "$HOME/.cargo/env"`).
    - [ ] All 4 output formatters (`fish`, `bash`, `json`, `dotenv`).
- [ ] Configure `[[bin]] name = "posix-source"` in `cmdr/Cargo.toml`.
- [ ] Implement `src/bin/posix-source.rs` entry point in `cmdr` with CLI format argument.
- [ ] Benchmark execution time (target: < 1.0 ms).
- [ ] Mandatory manual review:
    - [ ] `tui/src/core/script/posix_env/mod.rs`
    - [ ] `tui/src/core/script/mod.rs`
    - [ ] `cmdr/Cargo.toml`
    - [ ] `cmdr/src/bin/posix-source.rs`

### Phase 3: Fish Integration & Local Verification

- [ ] Update `~/scripts/fish/core/06-environment.fish` to use:
    ```fish
    if command -v posix-source >/dev/null 2>&1
        posix-source ~/.profile | source
    else
        bass_source ~/.profile
    end
    ```
- [ ] Profile startup with `fish --profile-startup` to verify latency drop from ~127ms to
      <1ms.
- [ ] Mandatory manual review:
    - [ ] `~/scripts/fish/core/06-environment.fish`

### Phase 4: Fleet-Wide Migration & Deprecation of `fish-bass` (Linux & macOS)

- [ ] Audit dotfiles and shell scripts across all Linux and macOS hosts for `bass` and
      `bass_source` usages.
- [ ] Build and install release binary `posix-source` across all fleet platforms (x86_64
      and aarch64 for Linux and macOS).
- [ ] Update all environment loaders and fish scripts in dotfiles to use `posix-source`
      unconditionally.
- [ ] Remove `bass` references and installation steps from
      `~/scripts/local-backup-restore/fresh-install/` (such as
      `02-restore-data-from-backup.fish` and
      `fresh-install-steps/01-fix-dot-profile-env-vars.fish`) so newly provisioned
      machines do not install or require `fish-bass`.
- [ ] Remove `fish-bass` plugin files (`00-bass.fish`, `__bass.py`) and Python runtime
      dependencies from dotfiles package manifests.
- [ ] Sync configurations across the fleet via `env-save -w`.
- [ ] Measure interactive SSH startup time across all hosts (`nazmul-desktop`,
      `nazmul-laptop`, `nazmul-mobile`, macOS machines).
- [ ] Mandatory manual review:
    - [ ] `~/scripts/local-backup-restore/fresh-install/02-restore-data-from-backup.fish`
    - [ ] `~/scripts/local-backup-restore/fresh-install/fresh-install-steps/01-fix-dot-profile-env-vars.fish`
    - [ ] Fleet sync status and verification logs
