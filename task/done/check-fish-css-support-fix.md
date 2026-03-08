# Plan: Consolidate `cargo doc` + fix `--doc` + dep-doc caching

## Results achieved

**`--watch-doc` (the always-running terminal)**: Full doc rebuilds dropped from **~4.5 minutes to
~5-14 seconds** on every file save. The dep-doc cache skips rebuilding external crate docs
(crossterm, tokio, serde, etc.) because `Cargo.lock` + `rust-toolchain.toml` haven't changed.
Only workspace crates are rebuilt.

The ~4.5 minute full rebuild now only triggers on infrequent events:

- Reboot (tmpfs wiped, hash file gone)
- Dependency added/updated (`Cargo.lock` changes)
- Toolchain update (`rust-toolchain.toml` changes)
- Manual `cargo clean` or deletion of `/tmp/roc/target/check/`

**`--doc` mode fixed**: Now builds full docs with dependencies (was incorrectly using `--no-deps`),
with the same dep-doc caching. First run ~4.5 min, subsequent runs ~0.1-14s.

**DRY**: All 4 `cargo doc` call sites consolidated into `run_cargo_doc` (single source of truth
for `RUSTDOCFLAGS` with absolute CSS path).

**Config cleanup**: Removed `rustdocflags` from `.cargo/config.toml`. Bare `cargo doc` outside
`check.fish` still works - just without custom CSS.

## Context

Three problems solved in one change:

1. **Bug**: `check.fish --doc` calls `check_docs_quick` (`cargo doc --no-deps`) but should build
   all docs with dependencies, like `--full` does.

2. **DRY**: 4 separate `cargo doc` invocations across 2 files need the same custom CSS
   `RUSTDOCFLAGS` logic. Consolidate into a single `run_cargo_doc` function.

3. **Performance**: Dep docs rarely change but take ~90s to build. Cache them and skip rebuilding
   when `Cargo.lock` + `rust-toolchain.toml` haven't changed.

4. **Config cleanup**: Remove `rustdocflags` from `.cargo/config.toml`. The absolute path approach
   (via `RUSTDOCFLAGS` env var in `run_cargo_doc`) works universally. Bare `cargo doc` outside
   check.fish won't fail — it just won't get custom CSS.

## Design

### A. Shared `run_cargo_doc` in `script_lib.fish`

Place near the existing doc functions (~line 1720 area).

```fish
# Central cargo doc runner with custom CSS support.
#
# Always sets RUSTDOCFLAGS with absolute path for monospace font CSS.
# Accepts optional --timeout=SECS for builds that need time limits.
# All other arguments pass through to `cargo doc`.
#
# ARCHITECTURE NOTE (for future Rust rewrite):
# This function is the single source of truth for invoking `cargo doc`.
# All doc build paths (--doc, --quick-doc, --full, --watch-doc) route through
# here. Custom CSS is applied via RUSTDOCFLAGS env var with an absolute path
# (not .cargo/config.toml) because rustdoc resolves --extend-css relative to
# each crate's source directory — which fails for dependency crates outside
# the workspace root.
#
# Used by: check_docs_quick, check_docs_full, build_and_sync_quick_docs,
#          build_and_sync_full_docs
function run_cargo_doc
    set -lx RUSTDOCFLAGS "--extend-css $PWD/docs/rustdoc/custom.css"

    set -l timeout_secs 0
    set -l cargo_args
    for arg in $argv
        if string match -q -- '--timeout=*' $arg
            set timeout_secs (string replace '--timeout=' '' $arg)
        else
            set -a cargo_args $arg
        end
    end

    if test "$timeout_secs" -gt 0
        ionice_wrapper timeout --foreground $timeout_secs cargo doc $cargo_args
    else
        ionice_wrapper cargo doc $cargo_args
    end
end
```

**Why `script_lib.fish`**: Sourced by both the main process (check.fish:55) and forked watch
processes (`fish -c "source script_lib.fish; ..."`).

**Why `--timeout=SECS`** parameter: Forked watch process doesn't have `CHECK_TIMEOUT_SECS` in
scope (it only sources `script_lib.fish`, not `check_constants.fish`). Passing the value explicitly
keeps the function self-contained.

### B. Dep-doc caching helpers in `script_lib.fish`

Place next to `run_cargo_doc`.

```fish
# Check if dependency docs are still valid (no dep changes since last full build).
#
# CACHE INVALIDATION:
# Hash is based on Cargo.lock + rust-toolchain.toml. These two files capture
# all scenarios that require rebuilding dependency docs:
#   - Cargo.lock: dependency version changes, additions, removals
#   - rust-toolchain.toml: toolchain changes (doc format can differ between nightlies)
#
# Hash file lives in the serving dir (tmpfs at /tmp/roc/target/check/), so:
#   - Reboot → tmpfs wiped → hash file gone → full rebuild (self-healing)
#   - cargo clean / manual rm → same effect
#   - check_config_changed cleans target → same effect
#
# Parameters:
#   $argv[1]: serving_dir (e.g., /tmp/roc/target/check)
#
# Returns: 0 if dep docs are current, 1 if they need rebuilding.
function dep_docs_are_current
    set -l serving_dir $argv[1]
    set -l hash_file $serving_dir/.dep-docs-hash
    if not test -f $hash_file
        return 1
    end
    set -l current_hash (cat Cargo.lock rust-toolchain.toml 2>/dev/null | md5sum | cut -d' ' -f1)
    set -l stored_hash (cat $hash_file)
    test "$current_hash" = "$stored_hash"
end

# Update dep docs hash after successful full build + sync.
#
# Call this AFTER syncing docs to the serving directory, not after the build
# itself, because the hash represents "what is currently served and valid."
#
# Parameters:
#   $argv[1]: serving_dir (e.g., /tmp/roc/target/check)
function update_dep_docs_hash
    set -l serving_dir $argv[1]
    cat Cargo.lock rust-toolchain.toml 2>/dev/null | md5sum | cut -d' ' -f1 > $serving_dir/.dep-docs-hash
end
```

### C. Caching signal via `DEP_DOCS_WERE_CACHED` global variable

`check_docs_full` sets `DEP_DOCS_WERE_CACHED` so callers know which sync mode to use.

```fish
# ARCHITECTURE NOTE (for future Rust rewrite):
# DEP_DOCS_WERE_CACHED is a global variable used as a side-channel signal
# from check_docs_full to its callers (check.fish --doc and --full cases).
#
# Why not an exit code? check_docs_full's exit code flows through
# run_check_with_recovery (which maps codes to 0=success, 1=failure,
# 2=recoverable) and run_full_checks (which aggregates 7 check results
# into a single code). Threading a "deps cached" exit code through both
# layers would require changes to aggregation logic in 3 functions.
# A global variable sidesteps this cleanly.
#
# In the Rust rewrite, replace this with a proper return type:
#   enum DocBuildResult { Success { deps_cached: bool }, Failure, Recoverable }
#
# Why not needed for build_and_sync_full_docs? That function handles
# caching internally and does its own sync with plain rsync -a (no --delete),
# so cached dep docs are never at risk.
#
# Sync mode depends on this signal:
#   DEP_DOCS_WERE_CACHED=true  → sync_docs_to_serving quick (no --delete)
#   DEP_DOCS_WERE_CACHED=false → sync_docs_to_serving full  (--delete if orphans)
#
# Why sync mode matters: sync_docs_to_serving full can use rsync --delete
# when serving > staging file count. If we built --no-deps (deps cached),
# staging has fewer files → orphan detection fires → --delete would wipe
# the cached dep docs from serving.
```

### D. Refactored callers

#### `check_cargo.fish` — used by `--quick-doc`, `--doc`, `--full`

```fish
# Quick doc check without dependencies (for --quick-doc).
# Builds to QUICK staging directory; callers rsync to serving dir after success.
function check_docs_quick
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR_DOC_STAGING_QUICK
    run_cargo_doc --timeout=$CHECK_TIMEOUT_SECS --no-deps
end

# Full doc build with dep-doc caching (for --doc, --full, and watch modes).
# Builds to FULL staging directory to avoid race conditions with quick builds.
#
# Dep-doc caching: If Cargo.lock + rust-toolchain.toml haven't changed since
# the last full build, skips dependency docs (--no-deps) for ~10x speedup.
# Sets DEP_DOCS_WERE_CACHED global so callers choose the correct sync mode.
function check_docs_full
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR_DOC_STAGING_FULL
    if dep_docs_are_current $CHECK_TARGET_DIR
        set -g DEP_DOCS_WERE_CACHED true
        run_cargo_doc --timeout=$CHECK_TIMEOUT_SECS --no-deps
    else
        set -g DEP_DOCS_WERE_CACHED false
        run_cargo_doc --timeout=$CHECK_TIMEOUT_SECS
    end
end
```

#### `script_lib.fish` `build_and_sync_quick_docs` (~line 1724)

```fish
# Builds quick docs (workspace crate only) and syncs to serving directory.
# Used by --watch-doc for fast feedback (~5-7s). Cross-crate links will be
# broken until the full build completes.
function build_and_sync_quick_docs
    set -l staging_dir $argv[1]
    set -l serving_dir $argv[2]
    set -lx CARGO_TARGET_DIR $staging_dir
    run_cargo_doc -p r3bl_tui --no-deps > /dev/null 2>&1
    set -l result $status
    if test $result -eq 0
        mkdir -p "$serving_dir/doc"
        rsync -a "$staging_dir/doc/" "$serving_dir/doc/"
    end
    return $result
end
```

#### `script_lib.fish` `build_and_sync_full_docs` (~line 1778)

```fish
# Builds full docs (with dep-doc caching) and syncs to serving directory.
# Used by --watch-doc's forked background process for correct cross-crate links.
#
# Dep-doc caching: Checks hash of Cargo.lock + rust-toolchain.toml. If unchanged,
# builds only workspace crates (--no-deps). Existing dep docs in serving dir are
# preserved because rsync -a without --delete never removes destination files.
#
# Does NOT use the DEP_DOCS_WERE_CACHED global (unlike check_docs_full) because
# this function handles its own sync internally — rsync -a without --delete is
# safe regardless of whether deps were rebuilt or cached.
function build_and_sync_full_docs
    set -l staging_dir $argv[1]
    set -l serving_dir $argv[2]
    set -lx CARGO_TARGET_DIR $staging_dir
    if dep_docs_are_current $serving_dir
        run_cargo_doc --no-deps > /dev/null 2>&1
    else
        run_cargo_doc > /dev/null 2>&1
    end
    set -l result $status
    if test $result -eq 0
        mkdir -p "$serving_dir/doc"
        rsync -a "$staging_dir/doc/" "$serving_dir/doc/"
        # Update hash only when deps were actually rebuilt
        if not dep_docs_are_current $serving_dir
            update_dep_docs_hash $serving_dir
        end
    end
    return $result
end
```

#### `check.fish` `case doc` (~line 295)

```fish
case doc
    # ... toolchain, rustdoc-fmt (unchanged) ...
    echo "📚 Building documentation (full, with deps)..."
    run_check_with_recovery check_docs_full "docs"
    set -l doc_status $status
    if test $doc_status -eq 2
        cleanup_for_recovery $CHECK_TARGET_DIR_DOC_STAGING_FULL
        run_check_with_recovery check_docs_full "docs"
        set doc_status $status
        hint_toolchain_update_on_persistent_failure $doc_status
    end
    if test $doc_status -eq 0
        # Sync mode based on dep-doc cache (see DEP_DOCS_WERE_CACHED architecture note)
        if test "$DEP_DOCS_WERE_CACHED" = true
            sync_docs_to_serving quick
        else
            sync_docs_to_serving full
            update_dep_docs_hash $CHECK_TARGET_DIR
        end
        # ... success output (unchanged) ...
    end
```

#### `check.fish` `case full` (~line 206)

```fish
# After run_full_checks_with_recovery:
if test $full_status -eq 0
    # Sync mode based on dep-doc cache (see DEP_DOCS_WERE_CACHED architecture note)
    if test "$DEP_DOCS_WERE_CACHED" = true
        sync_docs_to_serving quick
    else
        sync_docs_to_serving full
        update_dep_docs_hash $CHECK_TARGET_DIR
    end
end
```

## Implementation order

1. **Add new functions** to `script_lib.fish`: `run_cargo_doc`, `dep_docs_are_current`,
   `update_dep_docs_hash`
2. **Refactor callers** in `check_cargo.fish` and `script_lib.fish` to use `run_cargo_doc`
3. **Remove `rustdocflags`** from `.cargo/config.toml`
4. **Fix `--doc` mode** in `check.fish`: switch to `check_docs_full` + cache-aware sync
5. **Fix `--full` mode** in `check.fish`: cache-aware sync
6. **Verify** all modes work (see verification section)
7. **Documentation pass**: update all comments, help text, and CLAUDE.md (see next section)

## Documentation pass (step 7)

After all code changes are verified, update comments and docs across these files. This pass is
critical for the future Rust rewrite — the fish scripts' comments serve as the architectural
specification.

### Comment/doc updates

| File | Line | Change |
|:-----|:-----|:-------|
| `check.fish` | 34 | `"quick, --no-deps"` → `"full, with deps (dep-doc caching)"` |
| `check.fish` | 296 | Update `case doc` comment to describe full build + caching |
| `check_cargo.fish` | 41 | Remove `--doc` from quick function's comment |
| `check_cargo.fish` | 48 | Add `--doc` to full function's comment, document caching |
| `check_cli.fish` | 81 | `"quick, no deps"` → `"full, with deps (cached)"` |
| `check_cli.fish` | 121 | `"--no-deps, quick check"` → `"full, with deps (dep-doc caching)"` |
| `check_cli.fish` | 232 | `"cargo doc --no-deps"` → `"cargo doc (full, deps cached when unchanged)"` |
| `check_constants.fish` | 31 | Remove `--doc` from quick staging dir comment |
| `check_constants.fish` | 32 | Update full staging dir comment to mention `--doc` |
| `CLAUDE.md` | 94 | `"cargo doc --no-deps (quick docs)"` → `"cargo doc (full, with dep-doc caching)"` |

### Architecture docs in `script_lib.fish` header

Update the existing doc build architecture section (~line 1600-1630 area) to document:

1. **`run_cargo_doc` as single source of truth** for all cargo doc invocations
2. **Why RUSTDOCFLAGS env var** instead of `.cargo/config.toml` (relative path fails for deps)
3. **Dep-doc caching algorithm**: hash of `Cargo.lock` + `rust-toolchain.toml`, stored in tmpfs
   serving dir, self-heals on reboot/clean
4. **`DEP_DOCS_WERE_CACHED` global variable pattern**: why exit codes don't work here (aggregation
   layers), when Rust rewrite should use a proper return type instead

### Architecture docs in `check_watch.fish` header

Update the doc build architecture ASCII diagram (~line 67-98) to reflect:
- `--doc` now uses full build (not quick)
- Dep-doc caching optimization in full builds

## What stays the same

- `--quick-doc` → still calls `check_docs_quick` (`--no-deps`, no caching)
- `--watch-doc` → still uses two-tier quick+full architecture with forked background build
- `sync_docs_to_serving` function body unchanged
- `run_check_with_recovery` unchanged
- `run_full_checks` aggregation unchanged
- `docs/rustdoc/custom.css` file unchanged
- All recovery logic unchanged

## Cache invalidation

Hash file: `$serving_dir/.dep-docs-hash` (e.g., `/tmp/roc/target/check/.dep-docs-hash`)

| Trigger | How |
|:--------|:----|
| Dependency version change | `Cargo.lock` changes → hash mismatch |
| Dependency added/removed | `Cargo.lock` changes → hash mismatch |
| Rust toolchain update | `rust-toolchain.toml` changes → hash mismatch |
| Reboot | tmpfs wiped → hash file gone |
| `cargo clean` / manual delete | Serving dir gone → hash file gone |
| `check_config_changed` cleans target | Serving dir gone → hash file gone |

## Verification

### End-to-end test plan

Each step must be run sequentially - each depends on state left by the previous.

**Verification commands** (run after each step to check state):

```fish
test -f /tmp/roc/target/check/.dep-docs-hash && echo "HASH: exists" || echo "HASH: missing"
test -d /tmp/roc/target/check/doc/crossterm && echo "DEPS: present" || echo "DEPS: missing"
```

**Timing expectations** - the clearest signal of whether caching worked:

- Cache miss (full dep build): ~60-90s for docs step
- Cache hit (no-deps build): ~5-7s for docs step

### Happy path

#### Step 1: `./check.fish --quick-doc`

- **Expected**: Fast build (~5-7s), no deps, no hash file created
- **Verify**: No `.dep-docs-hash` in serving dir, no `crossterm/` in doc dir

#### Step 2: `./check.fish --doc`

- **Expected**: Full build with deps (~90s first run), hash file created
- **Verify**: `.dep-docs-hash` exists, `crossterm/` dir exists in serving dir

#### Step 3: `./check.fish --full`

- **Expected**: All 7 checks pass, cache hit on doc step (deps already built), hash preserved
- **Verify**: Output shows docs pass quickly, hash file unchanged

### Cache usage and invalidation

#### Step 4: `./check.fish --doc` (again)

- **Expected**: Cache hit - fast build (~5-7s), deps skipped
- **Verify**: Completes fast, `crossterm/` still present

#### Step 5: `./check.fish --full` (again)

- **Expected**: Cache hit on doc step
- **Verify**: Docs step completes fast

#### Step 6: Remove hash file, then `./check.fish --doc`

```fish
rm /tmp/roc/target/check/.dep-docs-hash
./check.fish --doc
```

- **Expected**: Cache miss - full rebuild (~90s), hash recreated
- **Verify**: Hash file recreated, `crossterm/` present

#### Step 7a: Remove all tmpfs dirs, then `./check.fish --quick-doc`

```fish
rm -rf /tmp/roc/target/check /tmp/roc/target/check-doc-staging-quick /tmp/roc/target/check-doc-staging-full
./check.fish --quick-doc
```

- **Expected**: Quick build only, no hash file created, no dep docs
- **Verify**: No `.dep-docs-hash`, no `crossterm/`

#### Step 7b: `./check.fish --doc`

- **Expected**: Cache miss (no hash file) - full rebuild, hash created
- **Verify**: Hash file created, `crossterm/` present

#### Step 8: `./check.fish --full`

- **Expected**: Cache hit on doc step
- **Verify**: Docs step completes fast

### Watch mode

#### Step 9: `./check.fish --watch-doc` with live code change

```fish
./check.fish --watch-doc &
# Wait for initial build cycle to complete
# Make a doc comment change in a .rs file
# Observe rebuild cycle in /tmp/roc/check.log
./check.fish --kill
```

- **Expected**: Initial full build uses cache (~1s, not ~90s). After code change, quick build
  (~5s) then forked full build also uses cache (~14s, not ~90s). Dep docs preserved throughout.
- **Verify**: Hash file present, `crossterm/` present, log shows fast full build times

### Test results (2026-03-13)

All 10 steps passed. Bug fix in `string replace` (needed `--` separator) found and fixed during
step 1.

| Step | Mode | Duration | Cache | Result |
|:-----|:-----|:---------|:------|:-------|
| 1 | `--quick-doc` | 0.2s | N/A (no caching) | PASS |
| 2 | `--doc` (cold) | 3m 16s | miss | PASS |
| 3 | `--full` | docs: 19s | hit | PASS |
| 4 | `--doc` (warm) | 0.1s | hit | PASS |
| 5 | `--full` (warm) | docs: 0.2s | hit | PASS |
| 6 | `--doc` (hash removed) | 18s | miss (cargo incremental warm) | PASS |
| 7a | `--quick-doc` (all wiped) | 13s | N/A | PASS |
| 7b | `--doc` (cold) | 4m 35s | miss | PASS |
| 8 | `--full` (warm) | docs: 14s | hit | PASS |
| 9 | `--watch-doc` | initial: ~1s, after change: ~14s | hit | PASS |

Key finding: dep-doc caching reduces full doc build from **~4.5 minutes to ~0.1-14 seconds**
depending on whether cargo's own incremental cache is warm.
