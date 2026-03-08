# Task: Fix `check.fish --full` External Doc Build Cache Invalidation

This task addresses an issue where `check.fish --full` (and other doc modes) unnecessarily rebuilt dependency documentation (taking ~2m 14s) even when valid artifacts existed in the staging directory.

## Status
**Completed** (2026-03-16)

## Problem Statement
The `check.fish` script uses an "eventual consistency" two-tier build system for documentation. To ensure accuracy, it caches dependency documentation using a hash of `Cargo.lock` and `rust-toolchain.toml`.

However, this cache was being invalidated too frequently because:
1. The `.dep-docs-hash` was stored in the **serving directory** (`/tmp/roc/target/check/`).
2. The `check_config_changed` function wipes this serving directory whenever *any* workspace `Cargo.toml` is modified to prevent stale test binaries.
3. Deleting the serving directory deleted the hash, forcing a full `cargo doc` rebuild of all dependencies even if the `target/check-doc-staging-full/` directory still contained valid artifacts.
4. In `--watch` (full) mode, the hash was never updated after a successful build, causing every subsequent quiet period to trigger a full rebuild.

## Strategy
1. **Relocate Hash**: Move `.dep-docs-hash` from the volatile serving directory to the more stable staging directory (`/tmp/roc/target/check-doc-staging-full/`).
2. **Resilient Sync**: Update the sync logic to detect when the serving directory has been wiped and automatically restore all dependency docs from the staging cache, even on a cache hit.
3. **Fix Watch Mode**: Add the missing `update_dep_docs_hash` call to the watch loop's full-check orchestrator.

## Implementation Details

### 1. `script_lib.fish`
- Updated `dep_docs_are_current` and `update_dep_docs_hash` to accept a `staging_dir` parameter and store the hash there.
- Updated `build_and_sync_full_docs` to use the staging directory for hashing.
- Updated the `ARCHITECTURE NOTE` to reflect the new caching strategy.

### 2. `check_cargo.fish`
- Updated `check_docs_full` to pass the staging directory to `dep_docs_are_current`.

### 3. `check.fish`
- Updated the `full` and `doc` cases to check for the existence of `target/check/doc/r3bl_tui`.
  - *Note (2026-03-17)*: While terminal messages and notifications have been generalized to use `$WORKSPACE_NAME`, this specific filesystem check remains as a "canary" for detecting serving directory wipes.
- Updated printed documentation URLs to point to the generic `doc/` directory, allowing users to manually navigate between crate-specific indices in a multi-crate workspace.
- Updated `update_dep_docs_hash` calls to use the staging directory.

### 4. `check_watch.fish`
- Added `update_dep_docs_hash $CHECK_TARGET_DIR_DOC_STAGING_FULL` to the `run_checks_for_type "full"` case.
- Generalized all desktop notifications and log headers to use the dynamic `$WORKSPACE_NAME`.

## Verification Results
- **Initial Run (Cold)**: ~2m 14s (Full rebuild of dependencies).
- **Subsequent Run (Warm)**: ~7-10s (Cache hit, dependencies restored from staging in seconds).
- **Config Change (Cargo.toml)**: ~45-50s (Typecheck/Build/Clippy/Tests run, but Docs stage hits cache and completes in seconds).

## Future Rust Port Notes
When rewriting this in Rust (for `build-infra/cargo-monitor`):
- Use `cargo metadata --no-deps --format-version 1` to determine if the current directory is a **Workspace** or a **Package**.
- Use this discovery logic to intelligently apply or omit the `--workspace` flag, avoiding redundant-flag warnings in single-package projects.
- Automatically identify the "primary" crate (matching the folder name or containing a binary) to resolve the default documentation index path (`target/doc/<primary_crate>/index.html`).
- Use the same caching logic: tie dependency doc invalidation strictly to `Cargo.lock` and `rust-toolchain.toml`.
- Ensure the "staging to serving" sync handles the "restoration of dependencies" case if the serving directory is cleaned for other reasons.

## Retroactive Follow-up: Two-Tree Architecture (2026-03-17)

This task initially addressed `.dep-docs-hash` by moving it to the staging directory. However, a similar anti-pattern existed for `.config_hash`, which was still stored in the volatile serving directory.

A comprehensive follow-up (see `task/fix-check-fish-paths.md`) introduced a **Two-Tree Architecture** to cleanly separate concerns:

1.  **Shared Tree ($CARGO_TARGET_DIR)**: For cargo build artifacts and the doc serving directory. Shared with the IDE (rust-analyzer) for performance.
2.  **Private Tree ($CHECK_PROJECT_ROOT)**: For check.fish-specific metadata (config hash, logs, duration) and doc staging directories.

This architecture ensures that:
- Metadata survives when the shared target directory is wiped.
- `cleanup_oversized_target` can surgically clean managed directories without safety hazards.
- The system remains project-isolated and environment-aware.
