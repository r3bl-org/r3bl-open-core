# Task: Support Git Worktrees in check.fish and Folder-Scoped Cache Isolation

<!-- prettier-ignore-start -->
<!-- BEGIN mktoc -->

- [Task: Support Git Worktrees in check.fish and Folder-Scoped Cache Isolation](#task-support-git-worktrees-in-checkfish-and-folder-scoped-cache-isolation)
- [Overview](#overview)
- [Problem Statement & Root Causes](#problem-statement--root-causes)
  - [1. Shared Target Directory Contention](#1-shared-target-directory-contention)
  - [2. Hardcoded Shell Environment Variable](#2-hardcoded-shell-environment-variable)
  - [3. Staging and Serving URL Visibility](#3-staging-and-serving-url-visibility)
- [Design & Architecture](#design--architecture)
  - [1. The Native Symlink Architecture](#1-the-native-symlink-architecture)
- [1. Directory Independence: Always switch to repository root where this script lives](#1-directory-independence-always-switch-to-repository-root-where-this-script-lives)
- [2. Complete CARGO_TARGET_DIR eradication:](#2-complete-cargo_target_dir-eradication)
- [Strip any inherited CARGO_TARGET_DIR from stale shell environments so cargo](#strip-any-inherited-cargo_target_dir-from-stale-shell-environments-so-cargo)
- [always uses the native ./target symlink without being hijacked by old exports.](#always-uses-the-native-target-symlink-without-being-hijacked-by-old-exports)
- [Dynamic RAM-aware storage selection (RAM tmpfs vs NVMe disk-backed /var/tmp):](#dynamic-ram-aware-storage-selection-ram-tmpfs-vs-nvme-disk-backed-vartmp)
- [Always ensure the tmpfs backing directory exists. This is critical for recovering](#always-ensure-the-tmpfs-backing-directory-exists-this-is-critical-for-recovering)
- [from a reboot (which clears tmpfs) or an rsync (which copies the symlink but not the tmpfs dir).](#from-a-reboot-which-clears-tmpfs-or-an-rsync-which-copies-the-symlink-but-not-the-tmpfs-dir)
- [Native Symlink Isolation (Zero env vars needed for cargo!)](#native-symlink-isolation-zero-env-vars-needed-for-cargo)
- [Check and auto-heal the symlink:](#check-and-auto-heal-the-symlink)
- [- If target is a symlink but points to the wrong target or is broken, recreate it.](#--if-target-is-a-symlink-but-points-to-the-wrong-target-or-is-broken-recreate-it)
- [- If target is a physical directory or file, delete it and replace with symlink.](#--if-target-is-a-physical-directory-or-file-delete-it-and-replace-with-symlink)
- [- If target does not exist, create symlink.](#--if-target-does-not-exist-create-symlink)
  - [2. Target Directory Isolation Matrix](#2-target-directory-isolation-matrix)
  - [3. Worktree-Scoped File Watchers & Mutual Exclusion](#3-worktree-scoped-file-watchers--mutual-exclusion)
- [Implementation Steps](#implementation-steps)
  - [Phase 1: Update Cargo Configuration](#phase-1-update-cargo-configuration)
  - [Phase 2: Update Shell Configuration in notes](#phase-2-update-shell-configuration-in-notes)
  - [Phase 3: Update check.fish Scripts for Worktree Symlink & Watcher Isolation](#phase-3-update-checkfish-scripts-for-worktree-symlink--watcher-isolation)
  - [Phase 4: Verification & Worktree Synchronization](#phase-4-verification--worktree-synchronization)
  - [Phase 5: Update Documentation (README.md)](#phase-5-update-documentation-readmemd)

<!-- END mktoc -->
<!-- prettier-ignore-end -->

---

## Overview

When developing concurrently across multiple Git worktrees (for example:
`/home/nazmul/github/roc` on `main`, `/home/nazmul/github/roc-build-spawny` on
`build-infra-spawny`, and `/home/nazmul/github/roc-fix-shift-home-lockup` on
`fix-shift-home-lockup`), each worktree operates as an independent workspace with its own
branch, working tree, and active terminal sessions.

Currently, running `check.fish` (especially `--watch-doc` or `--watch`) across worktrees
results in cache collisions, build lock contention, file watcher conflicts, and
overwritten documentation. This task establishes clean, automatic folder-scoped isolation
so that every worktree runs in its own self-contained tmpfs world without requiring manual
overrides, environment variables, or heuristics, perfectly preserving shared IDE cache.

---

## Problem Statement & Root Causes

### 1. Shared Target Directory Contention

In `.cargo/config.toml`, the project currently hardcodes a build path:

```toml
[build]
target-dir = "/tmp/roc/target"
```

This forces `cargo` and IDEs (`rust-analyzer`) across all worktrees to build into the
exact same path in RAM.

### 2. Hardcoded Shell Environment Variable

In `~/github/notes/files/scripts/fish/core/06-environment.fish:29`, the shell exported:

```fish
set -gx CARGO_TARGET_DIR /tmp/roc/target
```

This pinned all terminal sessions to the single directory `/tmp/roc/target`. When multiple
worktrees run `check.fish` simultaneously:

- **Cargo Lock Contention**: Cargo creates a file lock at `/tmp/roc/target/.lock`.
  Concurrent runs block or fail waiting on this lock.
- **Cache Invalidation**: Because different worktrees are checked out to different
  branches with divergent codebases, builds thrash and invalidate incremental cache.
- **Doc Overwrites**: All doc builds sync to `file://$CHECK_TARGET_DIR/doc/`, causing
  whichever worktree finishes last to overwrite browser-facing docs.

### 3. Staging and Serving URL Visibility

`check.fish --watch-doc` already prints the exact URL where docs are served:

```fish
log_and_print $CHECK_LOG_FILE "    📖 Read the docs at: file://$CHECK_TARGET_DIR/doc/"
```

Because stdout explicitly outputs the target URL on every build, there is no need or
expectation that docs are always pinned to a fixed `/tmp/roc/...` path. Each worktree can
serve and display its own independent documentation URL.

---

## Design & Architecture

### 1. The Native Symlink Architecture

Instead of relying on fragile environment variables that `rust-analyzer` might miss, or
hardcoded paths in `.cargo/config.toml` that break worktrees, we use a robust symlink
strategy.

By default, `cargo` and `rust-analyzer` naturally output to the local `./target/`
directory. `check.fish` dynamically provisions an isolated tmpfs directory based on the
worktree folder name, and automatically creates a symlink from `./target/` to that tmpfs
location.

```fish
# Inside check.fish (at the very top, before sourcing anything)
# 1. Directory Independence: Always switch to repository root where this script lives
set -g CHECK_REPO_ROOT (cd (dirname (status --current-filename)) && pwd)
cd $CHECK_REPO_ROOT
set -l __check_dir $CHECK_REPO_ROOT

# Inside check_constants.fish
# 2. Complete CARGO_TARGET_DIR eradication:
# Strip any inherited CARGO_TARGET_DIR from stale shell environments so cargo
# always uses the native ./target symlink without being hijacked by old exports.
set -q CARGO_TARGET_DIR; and set -e CARGO_TARGET_DIR

set -l project_name (basename "$CHECK_REPO_ROOT")
set -l repo_hash (string sub -l 8 (echo -n "$CHECK_REPO_ROOT" | sha256sum | cut -d' ' -f1))
set -l project_id "$project_name-$repo_hash"
set -g CHECK_LOCK_FILE /tmp/check-fish-$USER-$project_id.pid

# Dynamic RAM-aware storage selection (RAM tmpfs vs NVMe disk-backed /var/tmp):
set -l total_ram_gib (get_system_ram_gib)
if test $total_ram_gib -ge 48
    set -g CHECK_PROJECT_ROOT /tmp/check-fish-$USER-$project_id
else
    set -g CHECK_PROJECT_ROOT /var/tmp/check-fish-$USER-$project_id
end

set -g CHECK_TARGET_DIR $CHECK_PROJECT_ROOT/target

# Always ensure the tmpfs backing directory exists. This is critical for recovering
# from a reboot (which clears tmpfs) or an rsync (which copies the symlink but not the tmpfs dir).
mkdir -p "$CHECK_TARGET_DIR"
mkdir -p "$CHECK_TARGET_DIR_DOC_STAGING_QUICK"
mkdir -p "$CHECK_TARGET_DIR_DOC_STAGING_FULL"

# Native Symlink Isolation (Zero env vars needed for cargo!)
# Check and auto-heal the symlink:
# - If target is a symlink but points to the wrong target or is broken, recreate it.
# - If target is a physical directory or file, delete it and replace with symlink.
# - If target does not exist, create symlink.
function ensure_target_symlink
    mkdir -p "$CHECK_TARGET_DIR"
    set -l local_target "$CHECK_REPO_ROOT/target"
    if test -L "$local_target"
        set -l link_target (readlink "$local_target")
        if test "$link_target" != "$CHECK_TARGET_DIR"
            rm -f "$local_target"
            ln -s "$CHECK_TARGET_DIR" "$local_target"
        end
    else
        if test -e "$local_target"
            echo "Moving existing physical target directory to tmpfs..."
            mv "$local_target"/* "$CHECK_TARGET_DIR"/ 2>/dev/null
            rmdir "$local_target" 2>/dev/null; or rm -rf "$local_target"
        else
            # User nuked target/ to reset cache; wipe backing store too.
            find "$CHECK_TARGET_DIR" -mindepth 1 -delete 2>/dev/null
        end
        ln -s "$CHECK_TARGET_DIR" "$local_target"
    end
end

ensure_target_symlink
```

This guarantees:

1. `check.fish`, `rust-analyzer`, and manual `cargo` commands all share the exact same RAM
   cache.
2. Zero configuration is required for VS Code or `.cargo/config.toml`.
3. Worktrees are 100% isolated.
4. Complete eradication of `CARGO_TARGET_DIR`:
    - Standard checks (`cargo check`, `cargo build`, `cargo test`, `cargo clippy`) run
      with zero target dir overrides, building natively to `./target`.
    - Doc staging builds use Cargo's native CLI flag (`cargo doc --target-dir <DIR>`)
      rather than setting environment variables.
5. Broken or misdirected symlinks after reboots, folder renames, or `rsync`s automatically
   heal themselves when `check.fish` runs.
6. Directory independence: `check.fish` can be safely invoked from any subfolder (e.g.
   `tui/`) and operates correctly from the project root.

### 2. Target Directory Isolation Matrix

| Worktree Path                                   | `project_name`              | Isolated Target Path (RAM tmpfs >= 48 GiB)         | Isolated Target Path (Disk NVMe < 48 GiB)              |
| :---------------------------------------------- | :-------------------------- | :------------------------------------------------- | :----------------------------------------------------- |
| `/home/nazmul/github/roc`                       | `roc-<hash>`                       | `/tmp/check-fish-$USER-roc-<hash>/target`                       | `/var/tmp/check-fish-$USER-roc-<hash>/target`                       |
| `/home/nazmul/github/roc-build-spawny`          | `roc-build-spawny-<hash>`          | `/tmp/check-fish-$USER-roc-build-spawny-<hash>/target`          | `/var/tmp/check-fish-$USER-roc-build-spawny-<hash>/target`          |
| `/home/nazmul/github/roc-fix-shift-home-lockup` | `roc-fix-shift-home-lockup-<hash>` | `/tmp/check-fish-$USER-roc-fix-shift-home-lockup-<hash>/target` | `/var/tmp/check-fish-$USER-roc-fix-shift-home-lockup-<hash>/target` |

### 3. Worktree-Scoped File Watchers & Mutual Exclusion

To prevent cross-worktree interference during watch mode:

1. **Watcher Scoping**: `check_watch.fish` must construct `watch_dirs` using absolute
   paths rooted at `$CHECK_REPO_ROOT` (e.g. `$CHECK_REPO_ROOT/$crate/src`).
2. **Safe Orphan Cleanup**: `kill_orphaned_watchers` in `check_lock.fish` must filter
   `pgrep` patterns using `$CHECK_REPO_ROOT` with exact word boundary (`pgrep -f "inotifywait.*$CHECK_REPO_ROOT/"`
   and `pgrep -f "fswatch.*$CHECK_REPO_ROOT/"`). This prevents watch sessions in one
   worktree from terminating the watcher of a concurrent sibling worktree.
3. **Continuous Symlink Integrity**: `check_watch.fish` periodic loop must call
   `ensure_target_symlink` to auto-heal `./target` if an external command (`cargo clean`,
   `rm -rf target`, `git clean -fdx`) removed the symlink during an active session.

---

## Implementation Steps

### Phase 1: Update Cargo Configuration

- [x] Modify `.cargo/config.toml` in `roc`: Remove the `[build]` section entirely
      (specifically `target-dir = "/tmp/roc/target"`).
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `.cargo/config.toml`

### Phase 2: Update Shell Configuration in notes

- [x] In `/home/nazmul/github/notes/files/scripts/fish/core/06-environment.fish`, remove
      the hardcoded line `set -gx CARGO_TARGET_DIR /tmp/roc/target`.
- [x] Verify that new fish shells inherit a clean state where `$CARGO_TARGET_DIR` is not
      globally pinned.
- [x] Run `fish -c "env-save -w"` in your terminal to commit this configuration change to
      the `notes` repository and sync it across the entire fleet.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `/home/nazmul/github/notes/files/scripts/fish/core/06-environment.fish`

### Phase 3: Update check.fish Scripts for Worktree Symlink & Watcher Isolation

- [x] Modify [`check.fish`](file:///home/nazmul/github/roc/check.fish): Resolve `CHECK_REPO_ROOT` absolutely at line 1 and `cd` there before sourcing any module to prevent relative path crashes: `set -g CHECK_REPO_ROOT (cd (dirname (status --current-filename)) && pwd)` followed by `cd $CHECK_REPO_ROOT` and `set -l __check_dir $CHECK_REPO_ROOT`.
- [x] Modify
      [`check_constants.fish`](file:///home/nazmul/github/roc/check_constants.fish): 1.
      Eradicate inherited shell exports: add
      `set -q CARGO_TARGET_DIR; and set -e CARGO_TARGET_DIR` so Cargo strictly uses `./target`. 2. Update the project
      name derivation to use a hash and `$USER` to prevent multi-user collisions: `set -l repo_hash (string sub -l 8 (echo -n "$CHECK_REPO_ROOT" | sha256sum | cut -d' ' -f1))` and `set -l project_id "$project_name-$repo_hash"`. 3. Remove the conditional `if set -q CARGO_TARGET_DIR` block. 4. Encapsulate
      symlink provisioning and healing into `ensure_target_symlink` using
      `set -l link_target (readlink "$local_target")` checking. It must also safely `mv` existing physical directories instead of `rm -rf` to avoid crashing IDEs, and detect manual `rm -rf target` to clear tmpfs. 5. Add `mkdir -p $CHECK_TARGET_DIR_DOC_STAGING_QUICK` and `mkdir -p $CHECK_TARGET_DIR_DOC_STAGING_FULL` to guarantee tmpfs robustness.
- [x] Modify [`check_cargo.fish`](file:///home/nazmul/github/roc/check_cargo.fish): 1.
      Remove `set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR` from all standard check
      functions (`check_cargo_check`, `check_cargo_build`, `check_clippy`,
      `check_cargo_test`, `check_doctests`, `check_windows_build`), allowing cargo to
      build natively to `./target` without environment variable overrides. 2. Update `check_docs_quick` and `check_docs_full` to pass Cargo's native CLI flag `--target-dir $CHECK_TARGET_DIR_DOC_STAGING_QUICK` etc. to `run_cargo_doc` instead of setting `CARGO_TARGET_DIR`.
- [x] Modify [`check_docs.fish`](file:///home/nazmul/github/roc/check_docs.fish): 1. Update
      `build_and_sync_quick_docs` and `build_and_sync_full_docs` to pass
      `--target-dir $staging_dir` to `run_cargo_doc` instead of setting
      `CARGO_TARGET_DIR`. 2. Fix the CSS path bug: change `$PWD/docs/rustdoc/custom.css` to `$CHECK_REPO_ROOT/docs/rustdoc/custom.css`.
- [x] Modify [`check_lock.fish`](file:///home/nazmul/github/roc/check_lock.fish) and
      [`check_watch.fish`](file:///home/nazmul/github/roc/check_watch.fish): 1. In
      `check_watch.fish`, construct `watch_dirs` using absolute paths rooted at
      `$CHECK_REPO_ROOT`. 2. In `check_lock.fish`, scope `kill_orphaned_watchers` using exact directory separators: `pgrep -f "inotifywait.*$CHECK_REPO_ROOT/"` and `pgrep -f "fswatch.*$CHECK_REPO_ROOT/"`. 3. In `check_watch.fish`, ensure the watch loop first explicitly checks `if not test -L "$CHECK_REPO_ROOT/target"; or not test -d "$CHECK_TARGET_DIR"` to detect manual `cargo clean` deletions and trigger a rebuild. `ensure_target_symlink` must only be invoked *after* this detection logic executes.
- [x] Modify `check.fish`, `check_cargo.fish`, and [`check_cli.fish`](file:///home/nazmul/github/roc/check_cli.fish) to add a `--clean` command flag that explicitly empties `$CHECK_TARGET_DIR` (using `find -delete`) to give users a reliable way to clear the actual tmpfs cache.
- [x] Modify [`check_recovery.fish`](file:///home/nazmul/github/roc/check_recovery.fish)
      and [`script_lib.fish`](file:///home/nazmul/github/roc/script_lib.fish): 1. Replace `rm -rf "$dir" && mkdir -p "$dir"` with `find "$dir" -mindepth 1 -delete 2>/dev/null` in `cleanup_target_folder`, `cleanup_oversized_target`, and `check_config_changed` to prevent OS error 20. 2. In `script_lib.fish`, change the hardcoded `SRC_DIRS` to dynamically resolve all workspace crate source folders (e.g., using `for crate_dir in */src; set -a SRC_DIRS $crate_dir; end`) to prevent ignoring crates like `rust-analyzer-mcp-server`.
- [x] **Code Documentation**: Ensure that all modified `.fish` scripts (especially `check_constants.fish`, `check_watch.fish`, and `check.fish`) include copious, detailed inline comments explaining the symlink logic, tmpfs cache routing, and the `cargo clean` watcher detection algorithm.
- [x] Run `./check.fish --check` to verify the symlink is created and the build
      environment works.
- [x] Commit changes to `main` with message: ```text [all] Scope target directory to
      worktree via symlinks

      Task: support-worktree-in-check-script.md
      ```

- [x] Push `main` to `origin/main`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `check.fish`
    - [ ] `check_constants.fish`
    - [ ] `check_cli.fish`
    - [ ] `check_cargo.fish`
    - [ ] `check_docs.fish`
    - [ ] `check_lock.fish`
    - [ ] `check_watch.fish`
    - [ ] `check_recovery.fish`
    - [ ] `script_lib.fish`

### Phase 4: Verification & Worktree Synchronization

- [x] In worktree `/home/nazmul/github/roc-build-spawny`, run `git rebase main`.
- [x] In worktree `/home/nazmul/github/roc-fix-shift-home-lockup`, run `git rebase main`.
- [x] Run `./check.fish --check` in `/home/nazmul/github/roc` to initialize its symlink.
- [x] Run `./check.fish --check` in `/home/nazmul/github/roc-build-spawny` to initialize
      its symlink.
- [x] Run `./check.fish --check` in `/home/nazmul/github/roc-fix-shift-home-lockup` to
      initialize its symlink.
- [x] Verify `./target` in `/home/nazmul/github/roc` correctly symlinks to
      `/tmp/check-fish-$USER-roc-<hash>/target`.
- [x] Verify `./target` in `/home/nazmul/github/roc-build-spawny` correctly symlinks to
      `/tmp/check-fish-$USER-roc-build-spawny-<hash>/target`.
- [x] Verify `./target` in `/home/nazmul/github/roc-fix-shift-home-lockup` correctly
      symlinks to `/tmp/check-fish-$USER-roc-fix-shift-home-lockup-<hash>/target`.
- [x] **Test Watch Mode Concurrency:** Start `./check.fish --watch` in worktree A, modify a file in worktree A, and verify it does not trigger an unwanted rebuild or terminate watch mode in worktree B.
- [x] Verify multiple worktrees can run concurrently without `.lock` file conflicts in
      their respective IDEs and terminals, and without file watcher process collisions.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `task/support-worktree-in-check-script.md`

### Phase 5: Update Documentation (README.md)

- [x] Modify `README.md`: Replace the obsolete per-tool `CARGO_TARGET_DIR` configuration
      section (`CARGO_TARGET_DIR=target/vscode`, etc.) with the new unified tmpfs symlink
      architecture. Document the following key features: 1. **Git Worktree Isolation**:
      Explain that `check.fish` automatically symlinks `./target` to a worktree-scoped
      tmpfs directory, eliminating lock contention. 2. **Smart RAM-Aware Storage**:
      Document the optimization where it uses RAM (`/tmp`) if the system has `>= 48 GiB`
      RAM, but automatically routes to disk (`/var/tmp`) for lower-RAM systems to prevent
      OOMs. 3. **Directory Independence**: Mention that `check.fish` is safe to run from
      any folder (e.g., inside a subcrate); it will always properly resolve to the project
      root. 4. **Rsync / Auto-Healing**: Explain that it is completely safe to use `rsync`
      to clone/sync folders. `check.fish` auto-heals the `target` symlink; users simply
      run `./check.fish` once on the new machine to initialize the backing directory. 5. **Cache Deletion (`--clean`)**: Document the distinction between `cargo clean` (which may cleanly empty the tmpfs contents) and the shell command `rm -rf target` (which only deletes the symlink and orphans the tmpfs cache). Recommend using `./check.fish --clean` for a guaranteed, reliable cache wipe.
- [x] Commit changes to `main` with message: ```text [docs] Update README to document new
      check.fish architecture

      Task: support-worktree-in-check-script.md
      ```

- [x] Push `main` to `origin/main`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `README.md`
