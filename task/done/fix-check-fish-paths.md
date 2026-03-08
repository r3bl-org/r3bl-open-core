<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Task: Fix `check.fish` Hardcoded Paths and Safety Hazards](#task-fix-checkfish-hardcoded-paths-and-safety-hazards)
  - [Status](#status)
  - [Overview](#overview)
  - [Problem](#problem)
  - [Key Constraint: Artifact Sharing](#key-constraint-artifact-sharing)
  - [Strategy](#strategy)
  - [Architecture: Staging and Serving](#architecture-staging-and-serving)
  - [User Environment and IDE Configuration](#user-environment-and-ide-configuration)
    - [Shell Environment](#shell-environment)
    - [IDE Configuration (VS Code / VSCodium)](#ide-configuration-vs-code--vscodium)
  - [Final directory layout](#final-directory-layout)
    - [Survival matrix](#survival-matrix)
- [Implementation plan](#implementation-plan)
  - [Step 0: Update `check_constants.fish` - two-tree path derivation](#step-0-update-check_constantsfish---two-tree-path-derivation)
    - [Step 0.0: Add `$CARGO_TARGET_DIR` detection with isolated default](#step-00-add-cargo_target_dir-detection-with-isolated-default)
    - [Step 0.1: Derive all other paths](#step-01-derive-all-other-paths)
    - [Step 0.2: Update comments in `check_constants.fish`](#step-02-update-comments-in-check_constantsfish)
  - [Step 1: Update `script_lib.fish` - relocate and rename `.config_hash`](#step-1-update-script_libfish---relocate-and-rename-config_hash)
    - [Step 1.0: Update `check_config_changed` to use `$CHECK_BUILD_CONFIG_HASH_FILE`](#step-10-update-check_config_changed-to-use-check_build_config_hash_file)
    - [Step 1.1: Update `check_config_changed` logic](#step-11-update-check_config_changed-logic)
    - [Step 1.2: Update doc comments referencing `.config_hash`](#step-12-update-doc-comments-referencing-config_hash)
  - [Step 2: Update `check_cli.fish` - fix help text](#step-2-update-check_clifish---fix-help-text)
    - [Step 2.0: Update hardcoded path references in help output](#step-20-update-hardcoded-path-references-in-help-output)
  - [Step 3: Update `check_lock.fish` - fix hardcoded PID path in comments](#step-3-update-check_lockfish---fix-hardcoded-pid-path-in-comments)
    - [Step 3.0: Update comment on line 6](#step-30-update-comment-on-line-6)
  - [Step 4: Update `check_recovery.fish` - surgical cleanup](#step-4-update-check_recoveryfish---surgical-cleanup)
    - [Step 4.0: Update `cleanup_oversized_target` to be surgical](#step-40-update-cleanup_oversized_target-to-be-surgical)
    - [Step 4.1: Update `cleanup_target_folder` for two-tree awareness](#step-41-update-cleanup_target_folder-for-two-tree-awareness)
    - [Step 4.2: Update `dirs_for_check_type` for two-tree awareness](#step-42-update-dirs_for_check_type-for-two-tree-awareness)
    - [Step 4.3: Update comment on line 25](#step-43-update-comment-on-line-25)
  - [Step 5: Update `check_docs.fish` - serving dir reference](#step-5-update-check_docsfish---serving-dir-reference)
    - [Step 5.0: Update `sync_docs_to_serving`](#step-50-update-sync_docs_to_serving)
  - [Step 6: Update hardcoded `/tmp/roc` references in `script_lib.fish` comments](#step-6-update-hardcoded-tmproc-references-in-script_libfish-comments)
    - [Step 6.0: Update doc comments and architecture notes](#step-60-update-doc-comments-and-architecture-notes)
  - [Step 7: Update `check_cargo.fish` - verify per-command overrides](#step-7-update-check_cargofish---verify-per-command-overrides)
    - [Step 7.0: Verify non-doc commands use `$CHECK_TARGET_DIR`](#step-70-verify-non-doc-commands-use-check_target_dir)
    - [Step 7.1: Verify doc commands use staging dirs](#step-71-verify-doc-commands-use-staging-dirs)
  - [Step 8: Verify no remaining hardcoded `/tmp/roc` paths in executable code](#step-8-verify-no-remaining-hardcoded-tmproc-paths-in-executable-code)
    - [Step 8.0: Grep for `/tmp/roc` across all `.fish` files](#step-80-grep-for-tmproc-across-all-fish-files)
  - [Step 9: Update user environment](#step-9-update-user-environment)
    - [Step 9.0: Document env var change](#step-90-document-env-var-change)
  - [Step 10: Update task documentation](#step-10-update-task-documentation)
    - [Step 10.0: Add retroactive section to `task/check-fish-fix-full-ext-doc-build-cache-invalidation.md`](#step-100-add-retroactive-section-to-taskcheck-fish-fix-full-ext-doc-build-cache-invalidationmd)
  - [Step 11: Test](#step-11-test)
    - [Step 11.0: Verify with `$CARGO_TARGET_DIR` set (current setup, after env var change)](#step-110-verify-with-cargo_target_dir-set-current-setup-after-env-var-change)
    - [Step 11.1: Verify without `$CARGO_TARGET_DIR` set](#step-111-verify-without-cargo_target_dir-set)
    - [Step 11.2: Verify cleanup survival](#step-112-verify-cleanup-survival)
    - [Step 11.3: Verify artifact sharing](#step-113-verify-artifact-sharing)
    - [Step 11.4: Verify doc staging and serving](#step-114-verify-doc-staging-and-serving)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Task: Fix `check.fish` Hardcoded Paths and Safety Hazards

This task eliminates hardcoded `/tmp/roc/` paths in favor of a dynamic, project-isolated "Two-Tree
Architecture" that respects `$CARGO_TARGET_DIR`.

## Status

**Completed** (2026-03-17)

## Overview

## Problem

The `check.fish` script hardcodes all paths to `/tmp/roc/...`, ignoring the user's
`$CARGO_TARGET_DIR` environment variable. This creates several issues:

1. **Duplication & Friction**: The user sets `CARGO_TARGET_DIR=/tmp/roc/target/check` in their shell
   config, and `check_constants.fish` hardcodes the same value independently. If the user changes
   their env var, `check.fish` silently ignores it.

2. **`.config_hash` stored inside the directory it guards**: The build config hash file lives at
   `$CHECK_TARGET_DIR/.config_hash`. Multiple cleanup paths (`cleanup_target_folder`,
   `cleanup_for_recovery`, `cleanup_oversized_target`) delete this directory, destroying the hash as
   collateral damage. This is the same anti-pattern that was fixed for `.dep-docs-hash` in
   `task/check-fish-fix-full-ext-doc-build-cache-invalidation.md`.

3. **Safety Hazard**: `cleanup_oversized_target` uses `dirname` on the target directory to find a
   parent to `rm -rf`. If a user sets `CARGO_TARGET_DIR` to a shallow path (like `/tmp/build`), it
   could run `rm -rf /tmp`.

## Key Constraint: Artifact Sharing

`CHECK_TARGET_DIR` **must** equal `$CARGO_TARGET_DIR` so that build artifacts are shared between
`check.fish` (especially `--watch-doc`) and the IDE (VS Code / rust-analyzer). Both tools point
cargo at the same directory, so incremental compilation artifacts are reused. This means:

- We **cannot** nest metadata or staging dirs inside `CHECK_TARGET_DIR` — `check_config_changed`
  does `rm -rf` on it, which would destroy nested items.
- We **cannot** change `CARGO_TARGET_DIR` to point at a parent — cargo would write artifacts to a
  different path than the IDE expects, breaking sharing.

## Strategy

1. **Two clearly separated trees**: A **shared** tree (`$CARGO_TARGET_DIR`) for cargo build
   artifacts and doc serving, and a **private** tree (`CHECK_PROJECT_ROOT`) for check.fish-owned
   metadata and doc staging.
2. **Derive shared tree from `$CARGO_TARGET_DIR`** (respecting the user's env var), with a
   project-isolated `/tmp` default when unset.
3. **Private tree at `/tmp/check-fish-<project>/`**: Holds staging dirs, config hash, duration, and
   log. Completely independent of `$CARGO_TARGET_DIR` — no `dirname` gymnastics, no hierarchy
   assumptions.
4. **Surgical cleanup**: `cleanup_oversized_target` explicitly manages known directories instead of
   `rm -rf`-ing a `dirname`-derived parent.
5. **Lock file at `/tmp/`**: Fully independent of both trees.

## Architecture: Staging and Serving

Doc builds use a two-tier "stage then promote" pattern:

- **Staging dirs** (private tree): `cargo doc` builds here via per-command
  `set -lx CARGO_TARGET_DIR $staging_dir` overrides. These artifacts are NOT shared with the IDE
  (doc builds produce different intermediate artifacts than `cargo check/build`).
- **Serving dir** (shared tree): `$CARGO_TARGET_DIR/doc/` — rsync copies finished docs here from
  staging. The browser loads from this path.

| Command               | Writes to                  | Shared with IDE? |
| :-------------------- | :------------------------- | :--------------- |
| `cargo check/build`   | `$CARGO_TARGET_DIR`        | yes              |
| `cargo clippy`        | `$CARGO_TARGET_DIR`        | yes              |
| `cargo test/doctest`  | `$CARGO_TARGET_DIR`        | yes              |
| `cargo doc` (quick)   | `staging-quick/` (private) | no               |
| `cargo doc` (full)    | `staging-full/` (private)  | no               |
| rsync after doc build | `$CARGO_TARGET_DIR/doc/`   | yes (serving)    |

## User Environment and IDE Configuration

### Shell Environment

The user's env var must change to drop the `/check` suffix (e.g., in
`~/scripts/fish/core/05-environment.fish`):

```fish
set -gx CARGO_TARGET_DIR /tmp/roc/target
```

### IDE Configuration (VS Code / VSCodium)

Since desktop applications often do not inherit shell environment variables on Linux, the following
settings must be updated in the **project-local** VS Code settings file (`.vscode/settings.json` in
the workspace root, e.g., `~/github/roc/.vscode/settings.json`). This applies to all IDE variants
(`code-insiders`, `code`, `vscodium`, `vscodium-insiders`):

1.  **`rust-analyzer.cargo.targetDir`**: Set to `true` or explicitly to `/tmp/roc/target` to ensure
    the IDE uses the same shared tree.
2.  **`terminal.integrated.env.linux`**: Add `CARGO_TARGET_DIR: "/tmp/roc/target"` so that
    integrated terminals also pick up the correct path.

Example `.vscode/settings.json` snippet:

```json
{
  "rust-analyzer.cargo.targetDir": "/tmp/roc/target",
  "terminal.integrated.env.linux": {
    "CARGO_TARGET_DIR": "/tmp/roc/target"
  }
}
```

## Final directory layout

```
SHARED TREE (cargo + IDE + check.fish)
$CARGO_TARGET_DIR/                       <- e.g. /tmp/roc/target/  (set by user or default)
+-- debug/                               <- shared build artifacts (cargo check/build)
+-- doc/                                 <- serving dir (rsync target, browser loads from here)
+-- ...                                  <- other cargo-managed dirs

PRIVATE TREE (check.fish only)
$CHECK_PROJECT_ROOT/                     <- /tmp/check-fish-<project>/
+-- .build_config_toml_hash              <- config change detection hash
+-- check_duration.txt                   <- CHECK_DURATION_FILE
+-- check.log                            <- CHECK_LOG_FILE
+-- staging-quick/                       <- CHECK_TARGET_DIR_DOC_STAGING_QUICK
|   +-- doc/                             <- cargo doc output (quick, --no-deps)
|   +-- debug/                           <- cargo's intermediate artifacts
+-- staging-full/                        <- CHECK_TARGET_DIR_DOC_STAGING_FULL
    +-- doc/                             <- cargo doc output (full, with deps)
    +-- debug/                           <- cargo's intermediate artifacts
    +-- .dep-docs-hash                   <- dep doc cache hash (already lives here)

LOCK FILE (fully independent)
/tmp/check-fish-<project>.pid            <- CHECK_LOCK_FILE
```

### Survival matrix

| Constant                        | Location                           | Survives config-change cleanup? | Survives oversized cleanup? |
| :------------------------------ | :--------------------------------- | :------------------------------ | :-------------------------- |
| `CARGO_TARGET_DIR`              | `/tmp/roc/target/`                 | **no** (intentional)            | **no** (intentional)        |
| `CHECK_PROJECT_ROOT`            | `/tmp/check-fish-roc/`             | yes                             | yes                         |
| `CHECK_TARGET_DIR_DOC_STAGING_` | `$CHECK_PROJECT_ROOT/staging-.../` | yes                             | **no** (managed dir)        |
| `.build_config_toml_hash`       | `$CHECK_PROJECT_ROOT/...`          | yes                             | yes                         |
| `.dep-docs-hash`                | `staging-full/...`                 | yes                             | **no** (rebuilt on demand)  |
| `CHECK_DURATION_FILE`           | `$CHECK_PROJECT_ROOT/...`          | yes                             | yes                         |
| `CHECK_LOG_FILE`                | `$CHECK_PROJECT_ROOT/...`          | yes                             | yes                         |
| `CHECK_LOCK_FILE`               | `/tmp/check-fish-roc.pid`          | yes                             | yes                         |

# Implementation plan

## Step 0: Update `check_constants.fish` - two-tree path derivation

Replace all hardcoded `/tmp/roc/...` paths with the two-tree architecture.

### Step 0.0: Add `$CARGO_TARGET_DIR` detection with isolated default

```fish
# Project name for isolation
set -l project_name (basename $PWD)

# PRIVATE TREE: check.fish-owned metadata and doc staging
# Always under /tmp for tmpfs performance. Independent of CARGO_TARGET_DIR.
set -g CHECK_PROJECT_ROOT /tmp/check-fish-$project_name

# SHARED TREE: cargo build artifacts (shared between check.fish and IDE)
# Respect user's CARGO_TARGET_DIR if set, otherwise default to isolated tmpfs path.
if set -q CARGO_TARGET_DIR; and test -n "$CARGO_TARGET_DIR"
    set -g CHECK_TARGET_DIR $CARGO_TARGET_DIR
else
    set -g CHECK_TARGET_DIR $CHECK_PROJECT_ROOT/target
    # Export so cargo picks it up (scoped to this process)
    set -gx CARGO_TARGET_DIR $CHECK_TARGET_DIR
end
```

**Note on default**: When `$CARGO_TARGET_DIR` is not set, the default
(`/tmp/check-fish-<project>/target`) places the shared tree inside the private tree. This is fine
because `check_config_changed` only wipes `$CHECK_TARGET_DIR` (the `target/` subdir), and the
private metadata files are siblings at the `CHECK_PROJECT_ROOT` level. When `$CARGO_TARGET_DIR` IS
set, the two trees are completely separate — no nesting, no overlap.

### Step 0.1: Derive all other paths

```fish
# Doc staging dirs (private tree - not shared with IDE)
set -g CHECK_TARGET_DIR_DOC_STAGING_QUICK $CHECK_PROJECT_ROOT/staging-quick
set -g CHECK_TARGET_DIR_DOC_STAGING_FULL  $CHECK_PROJECT_ROOT/staging-full

# Metadata files (private tree)
set -g CHECK_BUILD_CONFIG_HASH_FILE $CHECK_PROJECT_ROOT/.build_config_toml_hash
set -g CHECK_DURATION_FILE          $CHECK_PROJECT_ROOT/check_duration.txt
set -g CHECK_LOG_FILE               $CHECK_PROJECT_ROOT/check.log

# Lock file (fully independent, at /tmp root)
set -g CHECK_LOCK_FILE /tmp/check-fish-$project_name.pid
```

### Step 0.2: Update comments in `check_constants.fish`

- Document the two-tree architecture and the `$CARGO_TARGET_DIR` contract.
- Document the artifact sharing rationale (IDE reuse).
- Update the `MAX_TARGET_SIZE_GB` comment to reference surgical cleanup of managed directories.
- Remove all hardcoded `/tmp/roc/...` references.

## Step 1: Update `script_lib.fish` - relocate and rename `.config_hash`

### Step 1.0: Update `check_config_changed` to use `$CHECK_BUILD_CONFIG_HASH_FILE`

The function currently derives the hash file path from its `$target_dir` parameter:

```fish
# Before:
set -l hash_file "$target_dir/.config_hash"

# After:
set -l hash_file $CHECK_BUILD_CONFIG_HASH_FILE
```

Since the hash file now lives in the private tree (`$CHECK_PROJECT_ROOT`), it is completely
independent of the target directory it guards.

### Step 1.1: Update `check_config_changed` logic

The hash file now lives outside the target dir, so the logic needs adjustment:

```fish
function check_config_changed
    set -l target_dir $argv[1]
    set -l config_files $argv[2..-1]

    # Compute hash of all config files that affect builds
    set -l config_hash (cat $config_files 2>/dev/null | sha256sum | cut -d' ' -f1)
    set -l hash_file $CHECK_BUILD_CONFIG_HASH_FILE

    # Ensure hash file directory exists
    mkdir -p (dirname $hash_file)

    # Check if hash file exists and compare
    if test -f $hash_file
        set -l stored_hash (cat $hash_file)
        if test "$config_hash" != "$stored_hash"
            echo ""
            set_color yellow
            echo "⚠️  Build config changed (Cargo.toml, rust-toolchain.toml, or .cargo/config.toml)"
            echo "🧹 Cleaning $target_dir to avoid stale artifacts..."
            set_color normal
            if test -d "$target_dir"
                command rm -rf "$target_dir"
            end
            echo $config_hash > $hash_file
            echo ""
        end
    else
        # No hash file yet - create it (first run or after reboot)
        echo $config_hash > $hash_file
    end

    return 0
end
```

Key changes from current:

- Uses `$CHECK_BUILD_CONFIG_HASH_FILE` (global) instead of deriving from `$target_dir`.
- Removed the "target dir doesn't exist, nothing to clean" early return — hash should always be
  checked/written regardless of whether the target dir exists yet.
- Added `mkdir -p (dirname $hash_file)` to ensure `$CHECK_PROJECT_ROOT` exists.
- Wrapped `rm -rf` in `test -d` guard since target dir may not exist.

### Step 1.2: Update doc comments referencing `.config_hash`

Update the usage example in the function's docstring, the architecture notes in `script_lib.fish`
(lines ~1564-1566, ~1640-1642, ~1690-1692), and any other references to the old `.config_hash` name
or location.

## Step 2: Update `check_cli.fish` - fix help text

### Step 2.0: Update hardcoded path references in help output

The help text contains several hardcoded `/tmp/roc/...` references:

- Line 109: `/tmp/roc/check.log` -> describe as `$CHECK_PROJECT_ROOT/check.log`
- Line 148: `/tmp/roc/check.log` -> same
- Line 183: `target/check/.config_hash` -> `$CHECK_PROJECT_ROOT/.build_config_toml_hash`
- Line 269: `/tmp/roc/target/check` -> describe as derived from `$CARGO_TARGET_DIR`

Update all to describe the dynamic path derivation.

## Step 3: Update `check_lock.fish` - fix hardcoded PID path in comments

### Step 3.0: Update comment on line 6

Change `PID file: /tmp/roc/check.fish.pid` to reflect new `/tmp/check-fish-<project>.pid` path.

## Step 4: Update `check_recovery.fish` - surgical cleanup

### Step 4.0: Update `cleanup_oversized_target` to be surgical

Replace the dangerous `rm -rf (dirname $CHECK_TARGET_DIR)` with explicit cleanup of managed
directories only:

```fish
function cleanup_oversized_target
    # Sum the sizes of all managed directories
    set -l managed_dirs $CHECK_TARGET_DIR \
                        $CHECK_TARGET_DIR_DOC_STAGING_QUICK \
                        $CHECK_TARGET_DIR_DOC_STAGING_FULL

    set -l total_kb 0
    for dir in $managed_dirs
        if test -d "$dir"
            set -l dir_kb (command du -sk "$dir" 2>/dev/null | string split \t)[1]
            set total_kb (math "$total_kb + $dir_kb")
        end
    end

    set -l max_kb (math "$MAX_TARGET_SIZE_GB * 1048576")
    if test "$total_kb" -ge "$max_kb"
        set -l size_gb (math --scale=1 "$total_kb / 1048576")
        log_and_print $CHECK_LOG_FILE "["(timestamp)"] 🧹 Managed dirs are "$size_gb"GB (limit: "$MAX_TARGET_SIZE_GB"GB), cleaning..."
        for dir in $managed_dirs
            if test -d "$dir"
                command rm -rf "$dir"
            end
        end
    end
end
```

Key changes:

- **No `dirname`**: Only cleans explicitly managed directories. Eliminates the safety hazard where a
  shallow `$CARGO_TARGET_DIR` could cause `rm -rf /tmp`.
- **Sum of managed dirs**: Sizes each managed dir individually and sums them, rather than sizing a
  parent directory (which could include unmanaged content).
- **Private tree metadata survives**: `$CHECK_PROJECT_ROOT/.build_config_toml_hash`, `check.log`,
  `check_duration.txt` all survive because they are not in any managed dir.

### Step 4.1: Update `cleanup_target_folder` for two-tree awareness

The current `cleanup_target_folder` cleans all three target dirs by default. Update to include all
managed dirs from both trees:

```fish
function cleanup_target_folder
    echo "🧹 Cleaning target folders..."
    set -l dirs_to_clean
    if test (count $argv) -gt 0
        set dirs_to_clean $argv
    else
        set dirs_to_clean $CHECK_TARGET_DIR \
                          $CHECK_TARGET_DIR_DOC_STAGING_QUICK \
                          $CHECK_TARGET_DIR_DOC_STAGING_FULL
    end
    for dir in $dirs_to_clean
        if test -d "$dir"
            command rm -rf "$dir"
        end
    end
end
```

This is functionally the same as current, but the constants now point to the correct two-tree
locations.

### Step 4.2: Update `dirs_for_check_type` for two-tree awareness

The function maps check types to directories for targeted cleanup. Update to use the new constants
(no logic change needed — the constants already point to the right places).

### Step 4.3: Update comment on line 25

Change `Prevents /tmp/roc/target from filling` to reference managed directories.

## Step 5: Update `check_docs.fish` - serving dir reference

### Step 5.0: Update `sync_docs_to_serving`

The serving dir is currently `$CHECK_TARGET_DIR/doc`. With the new design, `$CHECK_TARGET_DIR` is
`$CARGO_TARGET_DIR` (e.g., `/tmp/roc/target/`), so the serving dir becomes `$CARGO_TARGET_DIR/doc/`.
The existing code at `check_docs.fish:22` already uses `$CHECK_TARGET_DIR/doc`, which is correct —
just verify it still resolves correctly.

## Step 6: Update hardcoded `/tmp/roc` references in `script_lib.fish` comments

### Step 6.0: Update doc comments and architecture notes

Update all comment references to `/tmp/roc/...` paths (lines ~929, ~1454, ~1456, ~1564-1566,
~1640-1642, ~1690-1695, ~1712, ~1742-1743) to use constant names and describe the two-tree
architecture.

## Step 7: Update `check_cargo.fish` - verify per-command overrides

### Step 7.0: Verify non-doc commands use `$CHECK_TARGET_DIR`

Lines 12-37 all do `set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR`. Since `CHECK_TARGET_DIR` now equals
`$CARGO_TARGET_DIR`, these overrides are technically redundant but harmless — they make the intent
explicit. No change needed, just verify.

### Step 7.1: Verify doc commands use staging dirs

Lines 44 and 58 override to `$CHECK_TARGET_DIR_DOC_STAGING_QUICK` and
`$CHECK_TARGET_DIR_DOC_STAGING_FULL`. These now point to the private tree. Verify the paths resolve
correctly.

## Step 8: Verify no remaining hardcoded `/tmp/roc` paths in executable code

### Step 8.0: Grep for `/tmp/roc` across all `.fish` files

Ensure all remaining hits are in comments only (not in `set` statements or executable code). The
only hardcoded `/tmp/` path in executable code should be:

- `CHECK_PROJECT_ROOT` definition: `/tmp/check-fish-$project_name`
- `CHECK_LOCK_FILE` definition: `/tmp/check-fish-$project_name.pid`

## Step 9: Update user environment

### Step 9.0: Document env var change

The user must update their shell config (`~/scripts/fish/core/05-environment.fish` line 29):

```fish
# Before:
set -gx CARGO_TARGET_DIR /tmp/roc/target/check

# After:
set -gx CARGO_TARGET_DIR /tmp/roc/target
```

This is a one-time change. The first build after will be cold.

## Step 10: Update task documentation

### Step 10.0: Add retroactive section to `task/check-fish-fix-full-ext-doc-build-cache-invalidation.md`

Document this follow-up fix as a related change, noting:

- The same anti-pattern (hash stored inside volatile directory) applied to `.config_hash`.
- The fix introduced the two-tree architecture to cleanly separate shared (cargo) and private
  (check.fish) concerns.
- `cleanup_oversized_target` was made surgical to eliminate the `dirname` safety hazard.

## Step 11: Test

### Step 11.0: Verify with `$CARGO_TARGET_DIR` set (current setup, after env var change)

Run `./check.fish --check` and confirm:

- `CHECK_TARGET_DIR` = `$CARGO_TARGET_DIR` = `/tmp/roc/target/`
- Build artifacts at `/tmp/roc/target/debug/`
- `.build_config_toml_hash` at `/tmp/check-fish-roc/.build_config_toml_hash`
- Log at `/tmp/check-fish-roc/check.log`
- Lock file at `/tmp/check-fish-roc.pid`
- Staging dirs at `/tmp/check-fish-roc/staging-quick/` and `staging-full/`

### Step 11.1: Verify without `$CARGO_TARGET_DIR` set

Temporarily unset the env var, run `./check.fish --check`, and confirm:

- Falls back to `/tmp/check-fish-roc/target` as `CHECK_TARGET_DIR`
- Private metadata at `/tmp/check-fish-roc/` (sibling to `target/`)
- All paths resolve correctly

### Step 11.2: Verify cleanup survival

1. Run a build to populate the hash file and staging dirs
2. Trigger `check_config_changed` (modify a `Cargo.toml` comment, revert after)
3. Confirm `.build_config_toml_hash` survives (in private tree)
4. Confirm staging dirs survive (in private tree)
5. Confirm `$CHECK_TARGET_DIR` was wiped and rebuilt

### Step 11.3: Verify artifact sharing

1. Run `./check.fish --check` to warm the shared build cache
2. Run `cargo check` manually (or trigger via IDE) — should be a near-instant cache hit
3. Confirm both use the same `$CARGO_TARGET_DIR/debug/` directory

### Step 11.4: Verify doc staging and serving

1. Run `./check.fish --doc` or `--watch-doc`
2. Confirm docs build to `$CHECK_PROJECT_ROOT/staging-full/doc/`
3. Confirm rsync copies to `$CARGO_TARGET_DIR/doc/` (serving dir)
4. Open `file://$CARGO_TARGET_DIR/doc/` in browser — docs should load
