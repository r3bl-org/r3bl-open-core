#!/usr/bin/env fish

# cspell:words osascript nextest mktemp ionice gdbus

# Comprehensive Build and Test Verification Script
#
# Purpose: Runs a comprehensive suite of checks to ensure code quality, correctness, and builds properly.
#          This includes toolchain validation, tests, doctests, and documentation building.
#
# Workflow:
# 1. Detects config file changes and cleans stale artifacts if needed
# 2. Validates Rust toolchain installation with all required components
# 3. Automatically installs/repairs toolchain if issues detected
# 4. Runs requested checks (typecheck, build, clippy, tests, doctests, docs)
# 5. Detects and recovers from Internal Compiler Errors (ICE)
# 6. On persistent ICE, escalates to rust-toolchain-update.fish to find stable nightly
#
# Config Change Detection:
# - Active in ALL modes: one-off (--test, --doc, default) and watch modes
# - Monitors: Cargo.toml (root + all workspace crates), rust-toolchain.toml, .cargo/config.toml
# - On hash mismatch: automatically cleans target/check to avoid stale artifact issues
# - Handles: incremental compilation toggle, profile changes, toolchain updates, dependency changes
# - In watch mode: config files are also added to inotifywait, so changes trigger the loop
#
# Config Change Detection Algorithm:
# 1. Concatenate contents of all config files: cat $CONFIG_FILES 2>/dev/null
# 2. Compute SHA256 hash of concatenated content: sha256sum | cut -d' ' -f1
# 3. Compare with stored hash in target/check/.config_hash
# 4. If different (or no stored hash): clean target/check, store new hash
# 5. If same: proceed without cleaning (artifacts are valid)
#
# Toolchain Management:
# - Automatically validates toolchain before running checks
# - Checks for: toolchain installed, rust-analyzer, rust-src, rustc works
# - If invalid, calls rust-toolchain-sync-to-toml.fish to reinstall
# - Sends desktop notifications (notify-send) on success/failure
#
# Toolchain Corruption Detection and Recovery:
# - Detects corrupted toolchains that appear installed but are broken internally
# - Symptoms: "Missing manifest in toolchain", repeated "syncing channel updates" loops
# - Causes: interrupted installation, download failure, manifest loss
# - Detection: Checks for "Missing manifest" in rustc/component output
# - Recovery process:
#   1. Detects corruption BEFORE normal validation (prevents infinite loops)
#   2. Tries rustup toolchain uninstall first
#   3. Falls back to direct folder deletion (~/.rustup/toolchains/<name>)
#   4. Clears rustup caches (~/.rustup/downloads/, ~/.rustup/tmp/)
#   5. Delegates to sync script for fresh reinstall
# - On sync failure: Shows last 30 lines of output (not silently suppressed)
#
# ICE (Internal Compiler Error) Detection and Recovery:
# - Detects ICE by checking for rustc-ice-*.txt dump files
#   (rustc creates these files when it crashes)
# - Recovery process:
#   1. Detects ICE via presence of rustc-ice-*.txt files
#   2. Removes entire target/ folder (rm -rf target)
#   3. Removes rustc-ice-*.txt dump files
#   4. Retries the failed check once
# - If ICE persists after cleanup, escalates to rust-toolchain-update.fish
#   to find a stable nightly (only in --full mode)
# - All ICE events are logged to CHECK_LOG_FILE for debugging
#
# Logging:
# - Log file: /tmp/roc/check.log (shared between all modes)
# - Watch modes: Log is initialized fresh at session start, events appended
# - One-off modes: Log is appended (preserves history from watch sessions)
# - Logged events: mode start, check start/pass/fail, ICE detection/cleanup
# - Log file location is printed at startup for easy access
#
# Desktop Notifications:
# - Success: "Toolchain Installation Complete" (normal urgency)
# - Failure: "Toolchain Installation Failed" (critical urgency)
# - Only triggered when toolchain is actually installed/repaired
#
# Single Instance Enforcement (watch modes only):
# - Only ONE watch mode instance can run at a time
# - One-off commands (--test, --doc, default) can run concurrently without issues
# - Uses PID file with liveness check (fish doesn't support bash-style flock FD syntax)
# - PID file: /tmp/roc/check.fish.pid (checked via kill -0 for cross-platform support)
# - When entering watch mode, kills existing instance and orphaned file watcher processes
# - Prevents race conditions where multiple watch instances clean target/check simultaneously
#
# Toolchain Concurrency:
# - Toolchain installation is delegated to rust-toolchain-sync-to-toml.fish which has its own lock
# - If toolchain installation is needed, sync script prevents concurrent modifications
#
# Performance Optimizations:
# - tmpfs: Builds to /tmp/roc/target/check (RAM-based, eliminates disk I/O)
#   ⚠️  Trade-off: Cache lost on reboot, first post-reboot build is cold
# - CARGO_BUILD_JOBS=<nproc>: Forces max parallelism (benchmarked: 60% faster for cargo doc)
# - ionice -c2 -n0: Highest I/O priority in best-effort class (no sudo needed)
#
# Watch Mode & Sliding Window Debounce:
# Watch mode uses a single-threaded main loop with sliding window debounce.
# The main thread alternates between BLOCKED (waiting for events/timeout) and
# DISPATCHING (forking build processes). Actual builds run in parallel background processes.
#
# Key Insight: The file watcher (inotifywait on Linux, fswatch on macOS) acts as
# both an event listener AND a timer. This lets us implement sliding window
# debounce without threads:
#   - Exit code 0: File changed (event detected)
#   - Exit code 2: Timeout expired (no events for N seconds)
#   - Exit code 1: Error
#
# Sliding Window Debounce Algorithm:
# Instead of threshold-based debounce ("ignore if last run was < N seconds ago"),
# we use sliding window debounce ("wait for N seconds of quiet before running").
#
# Algorithm:
#   1. Wait for FIRST event (file watcher, no timeout - blocks forever)
#   2. Start sliding window: call file watcher with DEBOUNCE_WINDOW_SECS timeout
#   3. If event arrives (code 0): loop back to step 2 (reset window)
#   4. If timeout expires (code 2): quiet period detected, run build
#   5. After build completes, go back to step 1
#
# ┌─────────────────────────────────────────────────────────────────┐
# │                     MAIN THREAD TIMELINE                        │
# └─────────────────────────────────────────────────────────────────┘
#
#      BLOCKED           BLOCKED           BLOCKED        DISPATCHING
#     (waiting)         (waiting)         (waiting)       (fork builds)
#         │                 │                 │                │
#         ▼                 ▼                 ▼                ▼
#    ┌─────────┐       ┌─────────┐       ┌─────────┐      ┌────────┐
#    │inotify  │       │inotify  │       │inotify  │      │ fork   │──▶ quick build (background)
#    │wait 10s │──────▶│wait 10s │──────▶│wait 10s │─────▶│ builds │──▶ full build (background)
#    └─────────┘       └─────────┘       └─────────┘      └────────┘
#         │                 │                 │                │
#         │                 │                 │                │
#    File changed!    File changed!      TIMEOUT!         Returns
#    (exit code 0)    (exit code 0)     (exit code 2)    immediately
#    "Reset window"   "Reset window"    "Quiet period,
#                                        dispatch builds!"
#
# Example Timeline (Rapid Saves with 1s debounce):
#   0:00  (idle)              Thread BLOCKED, waiting forever for first event
#   0:05  User saves file A   Thread UNBLOCKS, starts 1s debounce window
#   0:05.3 User saves file B  Window RESET (now 1s again)
#   0:05.8 User saves file C  Window RESET again
#   0:06.8 (1s of quiet)      TIMEOUT! Thread forks builds, returns immediately
#   0:06.8 (same instant)     Thread BLOCKED again, waiting for next event
#   0:14  Quick build done    Notification: "Quick docs ready!" (background)
#   1:37  Full build done     Notification: "Full docs built!" (background)
#
# Why This Works:
# - File watcher does the heavy lifting (efficient kernel-level blocking)
# - No CPU burn while waiting (uses kernel's inotify/FSEvents subsystem)
# - Single mechanism handles both "coalesce rapid saves" AND "wait for quiet"
# - No separate event draining or timestamp tracking needed
#
# Events During Build:
# Since builds are forked to background, the main thread returns to the file
# watcher immediately. File events are processed normally while builds run in
# parallel. If a new change arrives, a new build cycle starts (previous builds
# continue in background until completion).
#
# Doc Build Architecture (--watch-doc):
# Uses a two-tier build system with eventual consistency for cross-crate links.
# See script_lib.fish for detailed documentation of the algorithm.
#
# ARCHITECTURE OVERVIEW:
#
#   File change detected
#       │
#       ▼
#   ┌─────────────────────────────────────────────┐
#   │ Quick build (~5-7s) [BLOCKING]              │
#   │ • cargo doc -p r3bl_tui --no-deps           │
#   │ • Fast feedback, broken cross-crate links   │
#   └─────────────────────────────────────────────┘
#       │
#       ├──► Catch-up check (if changes during quick build)
#       │         └──► Quick build → forks Full build
#       │
#       ▼
#   ┌─────────────────────────────────────────────┐
#   │ Full build (~90s) [FORKED TO BACKGROUND]    │
#   │ • cargo doc (all deps)                      │
#   │ • Fixes all cross-crate links               │
#   └─────────────────────────────────────────────┘
#       │
#       └──► Catch-up check (if changes during full build)
#                 │
#                 ▼
#            Quick build (~5-7s) → forks Full build (~90s)
#                 │                      │
#                 ▼                      ▼
#            [fast feedback]      [fixes links eventually]
#
# EVENTUAL CONSISTENCY MODEL:
# The system forms a cycle: quick → full → (if changes) → quick → full → ...
# Termination: When no files change during a build, docs are current and correct.
#
# WHY BROKEN LINKS IN QUICK BUILD?
# The --no-deps flag makes builds fast but rustdoc can't resolve cross-crate
# links (e.g., to crossterm, tokio) because it doesn't know they exist.
# Full build documents everything, so links become correct.
#
# BLIND SPOTS:
# While a build runs, inotifywait isn't watching. Changes during this window
# are detected via `find -newermt "@$epoch"` after each build completes.
#
# STAGING DIRECTORIES:
# - staging-quick/doc/ → Quick builds write here
# - staging-full/doc/  → Full builds write here
# - serving/doc/       → Browser loads from here (both sync to this)
# Separate staging prevents race conditions when builds run concurrently.
#
# Why Quick Blocks but Full Forks?
# - Cargo uses a global package cache lock (~/.cargo/.package-cache)
# - Running both simultaneously causes "Blocking waiting for file lock"
# - Quick build is fast (~5-7s), so blocking is acceptable
# - Full build is slow (~90s), so forking lets user continue editing
#
# Target Directory Auto-Recovery:
# In addition to debounce, watch mode periodically checks if target/ exists.
# If deleted externally (cargo clean, manual rm), triggers immediate rebuild.
#
# Exit Codes:
# - 0: All checks passed ✅
# - 1: Checks failed or toolchain installation failed ❌
# - Specific check results shown in output
#
# Usage:
#   ./check.fish              Run default checks (tests, doctests, docs)
#   ./check.fish --check      Run typecheck only (cargo check)
#   ./check.fish --build      Run build only (cargo build)
#   ./check.fish --clippy     Run clippy only (cargo clippy --all-targets)
#   ./check.fish --test       Run tests only (cargo test + doctests)
#   ./check.fish --doc        Build docs only (quick, --no-deps)
#   ./check.fish --full       Run ALL checks (check + build + clippy + tests + doctests + docs)
#                             Includes ICE escalation to rust-toolchain-update.fish
#   ./check.fish --watch      Watch mode: run default checks on file changes
#   ./check.fish --watch-test Watch mode: run tests/doctests only
#   ./check.fish --watch-doc  Watch mode: quick docs first, full docs forked to background
#   ./check.fish --help       Show detailed help

# Import shared toolchain utilities
source script_lib.fish

# ============================================================================
# Single Instance Enforcement (watch modes only)
# ============================================================================

# Lock/PID file for single-instance enforcement.
# Uses PID file with process liveness check - simpler and fish-compatible.
set -g CHECK_LOCK_FILE /tmp/roc/check.fish.pid

# Acquire exclusive "lock" for watch mode, killing any existing holder.
#
# Uses a PID file approach that works in fish:
# 1. Check if PID file exists and that process is still alive
# 2. If alive: kill it and wait for it to exit
# 3. Write our PID to the file
#
# Note: Unlike flock, PID files can become stale on SIGKILL. We handle this
# by checking if the process is alive (cross-platform: kill -0).
#
# Also kills orphaned file watcher processes from previous sessions.

# Cross-platform check if a process is alive.
# Uses kill -0 which works on both Linux and macOS (doesn't actually send a signal).
# Returns: 0 if alive, 1 if not alive or invalid PID
function is_process_alive
    set -l pid $argv[1]
    test -n "$pid" && kill -0 $pid 2>/dev/null
end

function acquire_watch_lock
    mkdir -p (dirname $CHECK_LOCK_FILE)

    # Check if another instance is running
    if test -f $CHECK_LOCK_FILE
        set -l old_pid (cat $CHECK_LOCK_FILE 2>/dev/null | string trim)

        if is_process_alive $old_pid
            # Process is alive - kill it
            echo ""
            set_color yellow
            echo "⚠️  Another watch instance running (PID: $old_pid)"
            echo "🔪 Killing to prevent race conditions..."
            set_color normal

            kill $old_pid 2>/dev/null

            # Wait for process to exit (up to 5 seconds)
            set -l waited 0
            while is_process_alive $old_pid && test $waited -lt 50
                sleep 0.1
                set waited (math $waited + 1)
            end

            if is_process_alive $old_pid
                echo "❌ Failed to terminate previous instance" >&2
                return 1
            end

            echo "✅ Previous instance terminated"
            echo ""
        end
        # else: stale PID file, process already gone - just overwrite
    end

    # Write our PID
    echo $fish_pid > $CHECK_LOCK_FILE

    # Clean up any orphaned file watcher processes (inotifywait/fswatch)
    kill_orphaned_watchers

    return 0
end

# Kill orphaned file watcher processes from previous watch mode sessions.
# These can be left behind if the parent was killed with SIGKILL.
# Supports both inotifywait (Linux) and fswatch (macOS).
function kill_orphaned_watchers
    # Kill orphaned inotifywait (Linux)
    set -l inotify_pids (pgrep -f "inotifywait.*cmdr/src" 2>/dev/null)
    if test (count $inotify_pids) -gt 0
        for pid in $inotify_pids
            kill $pid 2>/dev/null
        end
    end

    # Kill orphaned fswatch (macOS)
    set -l fswatch_pids (pgrep -f "fswatch.*cmdr/src" 2>/dev/null)
    if test (count $fswatch_pids) -gt 0
        for pid in $fswatch_pids
            kill $pid 2>/dev/null
        end
    end
end

# NOTE: acquire_watch_lock is called only for watch modes (see watch_mode function)
# One-off commands (--test, --doc, default) can run concurrently without conflict

# ============================================================================
# Configuration Constants
# ============================================================================

# Sliding window debounce for watch mode (in seconds).
# After detecting a file change, waits for this many seconds of "quiet" (no new changes)
# before running checks. Each new change resets the window, coalescing rapid saves.
# This handles IDE auto-save, formatters, and "oops forgot to save that file" moments.
set -g DEBOUNCE_WINDOW_SECS 1

# Use tmpfs for build artifacts - eliminates disk I/O for massive speedup.
# /tmp is already tmpfs on most Linux systems (including this one: 46GB).
# Benefits: ~2-3x faster builds, no SSD wear, isolated from IDE target directories.
# Trade-off: Build cache is lost on reboot (first build after reboot is cold).
#
# CHECK_TARGET_DIR: Primary target for tests and serving docs to browser.
# CHECK_TARGET_DIR_DOC_STAGING_QUICK: Staging for quick doc builds (--no-deps).
# CHECK_TARGET_DIR_DOC_STAGING_FULL: Staging for full doc builds (with deps, runs in background).
# CHECK_LOG_FILE: Log file for all check.fish output (created fresh each run).
#
# Two separate staging dirs prevent race conditions: quick builds can run while
# background full builds are syncing, without files "vanishing" mid-rsync.
set -g CHECK_TARGET_DIR /tmp/roc/target/check
set -g CHECK_TARGET_DIR_DOC_STAGING_QUICK /tmp/roc/target/check-doc-staging-quick
set -g CHECK_TARGET_DIR_DOC_STAGING_FULL /tmp/roc/target/check-doc-staging-full
set -g CHECK_LOG_FILE /tmp/roc/check.log

# Force maximum parallelism for cargo operations.
# Despite cargo's docs saying it defaults to nproc, benchmarks show this makes
# a huge difference: 4 min vs 10 min for cargo doc (60% speedup).
# Auto-detect core count: nproc (Linux) or sysctl (macOS).
switch (uname -s)
    case Darwin
        set -gx CARGO_BUILD_JOBS (sysctl -n hw.ncpu)
    case '*'
        set -gx CARGO_BUILD_JOBS (nproc)
end

# List of config files that affect build artifacts.
# Changes to these files should trigger a clean rebuild to avoid stale artifact issues.
# Used by check_config_changed to detect when target/check needs to be cleaned.
# Dynamically includes: root Cargo.toml, all workspace crate Cargo.toml files,
# rust-toolchain.toml, and .cargo/config.toml.
set -g CONFIG_FILES_TO_WATCH Cargo.toml rust-toolchain.toml .cargo/config.toml
# Dynamically add all workspace crate Cargo.toml files (*/Cargo.toml)
for crate_toml in */Cargo.toml
    if test -f $crate_toml
        set -a CONFIG_FILES_TO_WATCH $crate_toml
    end
end

# Minimum duration (in seconds) before showing desktop notifications for one-off modes.
# If a check completes faster than this, skip the notification since the user is likely
# still looking at the terminal. For longer runs, they've probably switched to their IDE.
set -g NOTIFICATION_THRESHOLD_SECS 1

# Interval (in seconds) for checking if target/check directory exists in watch modes.
# inotifywait will timeout after this interval, allowing us to check for missing target.
# If target/check is missing, a rebuild is triggered automatically.
set -g TARGET_CHECK_INTERVAL_SECS 10

# Auto-dismiss timeout (in milliseconds) for desktop notifications.
# Notifications auto-dismiss to avoid clutter, especially in watch mode.
# 5 seconds = 5000ms. Set to 0 or remove to use system default (persistent).
set -g NOTIFICATION_EXPIRE_MS 5000

# ============================================================================
# Argument Parsing
# ============================================================================

# Parse command line arguments and return the mode
# Returns: "help", "check", "build", "clippy", "test", "doc", "full",
#          "watch", "watch-test", "watch-doc", or "normal"
function parse_arguments
    if test (count $argv) -eq 0
        echo "normal"
        return 0
    end

    switch $argv[1]
        case --help -h
            echo "help"
            return 0
        case --check
            echo "check"
            return 0
        case --build
            echo "build"
            return 0
        case --clippy
            echo "clippy"
            return 0
        case --full
            echo "full"
            return 0
        case --watch -w
            echo "watch"
            return 0
        case --watch-test
            echo "watch-test"
            return 0
        case --watch-doc
            echo "watch-doc"
            return 0
        case --kill
            echo "kill"
            return 0
        case --doc
            echo "doc"
            return 0
        case --test
            echo "test"
            return 0
        case '*'
            echo "❌ Unknown argument: $argv[1]" >&2
            echo "Use --help for usage information" >&2
            return 1
    end
end

# ============================================================================
# Help Display
# ============================================================================

# Display colorful help information
function show_help
    set_color green --bold
    echo "check.fish"
    set_color normal
    echo ""

    set_color yellow
    echo "PURPOSE:"
    set_color normal
    echo "  Comprehensive build and test verification for r3bl-open-core"
    echo "  Validates toolchain, runs tests, doctests, and builds documentation"
    echo ""

    set_color yellow
    echo "USAGE:"
    set_color normal
    echo "  ./check.fish              Run default checks (tests, doctests, docs)"
    echo "  ./check.fish --check      Run typecheck only (cargo check)"
    echo "  ./check.fish --build      Run build only (cargo build)"
    echo "  ./check.fish --clippy     Run clippy only (cargo clippy --all-targets)"
    echo "  ./check.fish --test       Run tests only (cargo test + doctests)"
    echo "  ./check.fish --doc        Build documentation only (quick, no deps)"
    echo "  ./check.fish --full       Run ALL checks (check + build + clippy + tests + doctests + docs)"
    echo "  ./check.fish --watch      Watch mode: run default checks on changes"
    echo "  ./check.fish --watch-test Watch mode: run tests/doctests only"
    echo "  ./check.fish --watch-doc  Watch mode: run doc build (full with deps)"
    echo "  ./check.fish --help       Show this help message"
    echo "  ./check.fish --kill       Kill any running watch instances and cleanup"
    echo ""

    set_color yellow
    echo "FEATURES:"
    set_color normal
    echo "  ✓ Single instance enforcement in watch modes (kills previous watch instances)"
    echo "  ✓ Config change detection (auto-cleans stale artifacts)"
    echo "  ✓ Automatic toolchain validation and repair"
    echo "  ✓ Corrupted toolchain detection and recovery (Missing manifest, etc.)"
    echo "  ✓ ICE escalation to rust-toolchain-update.fish (finds stable nightly)"
    echo "  ✓ Fast tests using cargo test"
    echo "  ✓ Documentation tests (doctests)"
    echo "  ✓ Documentation building"
    echo "  ✓ Blind spot recovery (catch-up build for changes during doc build)"
    echo "  ✓ ICE detection (auto-removes target/, retries once)"
    echo "  ✓ Desktop notifications on toolchain changes"
    echo "  ✓ Target directory auto-recovery in watch modes"
    echo "  ✓ Orphan doc file cleanup (full builds detect and remove stale files)"
    echo "  ✓ Performance optimizations (tmpfs, ionice, parallel jobs)"
    echo "  ✓ Comprehensive logging (all modes log to /tmp/roc/check.log)"
    echo "  ✓ One-off + watch-doc can run simultaneously (no lock contention)"
    echo ""

    set_color yellow
    echo "ONE-OFF MODES:"
    set_color normal
    echo "  (default)     Runs default checks: tests, doctests, docs"
    echo "  --check       Runs typecheck only: cargo check (fast compile check)"
    echo "  --build       Runs build only: cargo build (compile production code)"
    echo "  --clippy      Runs clippy only: cargo clippy --all-targets (lint warnings)"
    echo "  --test        Runs tests only: cargo test + doctests"
    echo "  --doc         Builds documentation only (--no-deps, quick check)"
    echo "  --full        Runs ALL checks: check + build + clippy + tests + doctests + docs"
    echo "                Includes ICE escalation to rust-toolchain-update.fish"
    echo ""

    set_color yellow
    echo "WATCH MODES:"
    set_color normal
    echo "  --watch       Runs all checks: tests, doctests, docs (full with deps)"
    echo "  --watch-test  Runs tests only: cargo test + doctests (faster iteration)"
    echo "  --watch-doc   Runs quick docs first, then forks full docs to background"
    echo ""
    echo "  Watch mode options:"
    echo "  Monitors: cmdr/src/, analytics_schema/src/, tui/src/, plus all config files"
    echo "  Toolchain: Validated once at startup, before watch loop begins"
    echo "  Behavior: Continues watching even if checks fail"
    echo "  Requirements: inotifywait (Linux) or fswatch (macOS) - installed via bootstrap.sh"
    echo ""
    echo "  Doc Builds (--watch-doc):"
    echo "  • Quick build (r3bl_tui only) runs first, blocking (~3-5s)"
    echo "  • Catch-up: detects files changed during build, rebuilds if needed"
    echo "  • Full build (all crates + deps) then forks to background (~90s)"
    echo "  • Quick docs available immediately, full docs notify when done"
    echo "  • Desktop notification when each build completes"
    echo "  • Separate staging directories prevent race conditions"
    echo "  • Output logged to /tmp/roc/check.log for debugging"
    echo ""
    echo "  Orphan File Cleanup (full builds only):"
    echo "  • Long-running sessions accumulate stale docs from renamed/deleted files"
    echo "  • Detection: compares file counts between staging and serving directories"
    echo "  • If serving has MORE files than staging, orphans exist"
    echo "  • Full builds use rsync --delete to clean orphaned files"
    echo "  • Quick builds never delete (would wipe dependency docs)"
    echo ""
    echo "  Sliding Window Debounce:"
    echo "  • Waits for $DEBOUNCE_WINDOW_SECS seconds of 'quiet' (no new changes) before dispatching"
    echo "  • Each new file change resets the window, coalescing rapid saves"
    echo "  • Handles: IDE auto-save, formatters, 'forgot to save that file' moments"
    echo "  • Adjust DEBOUNCE_WINDOW_SECS in script if needed"
    echo ""
    echo "  Target Directory Auto-Recovery:"
    echo "  • Monitors for missing target/ directory (every "$TARGET_CHECK_INTERVAL_SECS"s)"
    echo "  • Auto-triggers rebuild if target/ is deleted externally"
    echo "  • Recovers from: cargo clean, manual rm -rf target/, IDE cache clearing"
    echo ""

    set_color yellow
    echo "CONFIG CHANGE DETECTION (all modes):"
    set_color normal
    echo "  Automatically detects config file changes and cleans stale build artifacts."
    echo "  Works in ALL modes: one-off (--test, --doc, default) and watch modes."
    echo ""
    echo "  Monitored files:"
    echo "  • Cargo.toml (root + all workspace crates, dynamically detected)"
    echo "  • rust-toolchain.toml"
    echo "  • .cargo/config.toml"
    echo ""
    echo "  Algorithm:"
    echo "  1. Concatenate all config file contents"
    echo "  2. Compute SHA256 hash of concatenated content"
    echo "  3. Compare with stored hash in target/check/.config_hash"
    echo "  4. If different: clean target/check, store new hash, rebuild"
    echo "  5. If same: proceed without cleaning (artifacts are valid)"
    echo ""
    echo "  Handles these scenarios:"
    echo "  • Toggling incremental compilation on/off"
    echo "  • Changing optimization levels or profiles"
    echo "  • Updating Rust toolchain version"
    echo "  • Adding/removing dependencies in any crate"
    echo ""

    set_color yellow
    echo "TOOLCHAIN CORRUPTION RECOVERY:"
    set_color normal
    echo "  Detects and recovers from corrupted toolchain installations."
    echo ""
    echo "  Symptoms of corruption:"
    echo "  • 'Missing manifest in toolchain' errors"
    echo "  • Repeated 'syncing channel updates' loops that never complete"
    echo "  • Toolchain appears in 'rustup toolchain list' but doesn't work"
    echo ""
    echo "  Common causes:"
    echo "  • Interrupted installation (Ctrl+C, network failure, power loss)"
    echo "  • Corrupted download cache"
    echo "  • Manifest file loss or corruption"
    echo ""
    echo "  Recovery process:"
    echo "  1. Detects corruption BEFORE normal validation (prevents loops)"
    echo "  2. Tries 'rustup toolchain uninstall' first"
    echo "  3. Falls back to direct folder deletion (~/.rustup/toolchains/)"
    echo "  4. Clears rustup caches (~/.rustup/downloads/, ~/.rustup/tmp/)"
    echo "  5. Reinstalls via rust-toolchain-sync-to-toml.fish"
    echo ""
    echo "  Visibility improvements:"
    echo "  • On sync failure: shows last 30 lines of output (not silent)"
    echo "  • Reports specific failure reason (e.g., 'rust-analyzer missing')"
    echo "  • Points to full log: ~/Downloads/rust-toolchain-sync-to-toml.log"
    echo ""

    set_color yellow
    echo "WORKFLOW:"
    set_color normal
    echo "  1. Checks for config file changes (cleans target if needed)"
    echo "  2. Checks for corrupted toolchain (force-removes if detected)"
    echo "  3. Validates Rust toolchain (nightly + components)"
    echo "  4. Auto-installs/repairs toolchain if needed"
    echo "  5. Runs cargo test (all unit and integration tests)"
    echo "  6. Runs doctests"
    echo "  7. Builds documentation:"
    echo "     • One-off --doc:  cargo doc --no-deps (quick, your crates only)"
    echo "     • --watch-doc:    Forks both quick + full builds to background"
    echo "     • Other watch:    cargo doc (full, includes dependencies)"
    echo "  8. On ICE: removes target/, retries once"
    echo ""

    set_color yellow
    echo "NOTIFICATIONS:"
    set_color normal
    echo "  Desktop notifications alert you when checks complete."
    echo ""
    echo "  Platform support:"
    echo "  • Linux: gdbus (GNOME) with notify-send fallback"
    echo "  • macOS: osascript (native AppleScript)"
    echo ""
    echo "  When notifications are sent:"
    echo "  • One-off modes (--test, --doc): On success only if duration > $NOTIFICATION_THRESHOLD_SECS""s"
    echo "  • One-off modes: Always on failure (you need to know!)"
    echo "  • Default mode (all checks): Always on completion"
    echo "  • Watch modes: On success and failure"
    echo "  • Toolchain installation: On install success/failure"
    echo ""
    echo "  Auto-dismiss behavior:"
    echo "  • All notifications auto-dismiss after "(math $NOTIFICATION_EXPIRE_MS / 1000)" seconds"
    echo "  • Linux/GNOME: Uses gdbus + CloseNotification (GNOME ignores --expire-time)"
    echo "  • macOS: System handles auto-dismiss automatically"
    echo ""
    echo "  Rationale: Quick one-off operations (<$NOTIFICATION_THRESHOLD_SECS""s) don't need notifications"
    echo "  since you're likely still watching the terminal. Longer operations trigger"
    echo "  notifications because you've probably switched to your IDE."
    echo ""

    set_color yellow
    echo "PERFORMANCE OPTIMIZATIONS:"
    set_color normal
    echo "  This script uses several techniques to maximize build speed:"
    echo ""
    echo "  1. tmpfs Build Directory (/tmp/roc/target/check):"
    echo "     • Builds happen in RAM instead of disk - eliminates I/O bottleneck"
    echo "     • /tmp is typically a tmpfs mount (RAM-based filesystem)"
    echo "     • ⚠️  Trade-off: Build cache is lost on reboot"
    echo "     • First build after reboot will be a cold start (slower)"
    echo "     • Subsequent builds use cached artifacts (fast)"
    echo ""
    echo "  2. Parallel Jobs (CARGO_BUILD_JOBS=$CARGO_BUILD_JOBS):"
    echo "     • Auto-detected core count: nproc (Linux) or sysctl (macOS)"
    echo "     • Benchmarked: 4 min vs 10 min for cargo doc (60% speedup)"
    echo "     • Despite cargo docs, this doesn't always default to nproc"
    echo ""
    echo "  3. I/O Priority (ionice -c2 -n0):"
    echo "     • Gives cargo highest I/O priority in best-effort class"
    echo "     • Helps when other processes compete for disk/tmpfs access"
    echo "     • No sudo required (unlike realtime I/O class)"
    echo ""
    echo "  4. Two-Stage Doc Build:"
    echo "     • Docs are built to staging directory first"
    echo "     • Only synced to serving directory on success"
    echo "     • Browser never sees incomplete/missing docs during rebuilds"
    echo ""
    echo "  5. Incremental Compilation:"
    echo "     • Rust only recompiles what changed"
    echo "     • Pre-compiled .rlib files stay cached between runs"
    echo "     • Test executables are cached - re-running just executes the binary"
    echo ""

    set_color yellow
    echo "WHY IS IT SO FAST?"
    set_color normal
    echo "  Example output (2,700+ tests in ~9 seconds with warm cache):"
    echo ""
    echo "    ./check.fish"
    echo ""
    echo "    🚀 Running checks..."
    echo ""
    echo "    [03:41:57 PM] ▶️  Running tests..."
    echo "    [03:42:01 PM] ✅ tests passed (3s)"
    echo ""
    echo "    [03:42:01 PM] ▶️  Running doctests..."
    echo "    [03:42:06 PM] ✅ doctests passed (5s)"
    echo ""
    echo "    [03:42:06 PM] ▶️  Running docs..."
    echo "    [03:42:08 PM] ✅ docs passed (1s)"
    echo ""
    echo "    [03:42:08 PM] ✅ All checks passed!"
    echo ""
    echo "  The speed comes from a combination of techniques:"
    echo "  • tmpfs (RAM disk): All I/O happens in RAM, no SSD/HDD seeks"
    echo "  • Incremental compilation: Only changed modules rebuild"
    echo "  • Cached test binaries: Re-running tests just executes the binary"
    echo "  • 28 parallel jobs: Maximizes CPU utilization during compilation"
    echo ""
    echo "  Trade-off: First build after reboot is cold (~2-4 min),"
    echo "  but subsequent runs stay blazing fast."
    echo ""

    set_color yellow
    echo "EXIT CODES:"
    set_color normal
    echo "  0  All checks passed ✅"
    echo "  1  Checks failed or toolchain installation failed ❌"
    echo ""

    set_color yellow
    echo "EXAMPLES:"
    set_color normal
    echo "  # Run checks once"
    echo "  ./check.fish"
    echo ""
    echo "  # Watch for changes and auto-run all checks"
    echo "  ./check.fish --watch"
    echo ""
    echo "  # Watch for changes and auto-run tests/doctests only (faster iteration)"
    echo "  ./check.fish --watch-test"
    echo ""
    echo "  # Watch for changes and auto-run doc build only"
    echo "  ./check.fish --watch-doc"
    echo ""
    echo "  # Show this help"
    echo "  ./check.fish --help"
    echo ""
end

# ============================================================================
# Watch Mode
# ============================================================================

# Watch source directories and run checks on file changes
# Parameters: check_type - "full", "test", or "doc"
function watch_mode
    set -l check_type $argv[1]

    # Acquire exclusive lock, killing any existing watch instance
    # One-off commands can run concurrently, but watch modes conflict
    if not acquire_watch_lock
        return 1
    end

    # Check for file watcher (inotifywait on Linux, fswatch on macOS)
    if not command -v inotifywait >/dev/null 2>&1
        and not command -v fswatch >/dev/null 2>&1
        echo "❌ Error: No file watcher found" >&2
        echo "Install with: ./bootstrap.sh" >&2
        echo "Or manually:" >&2
        switch (uname -s)
            case Darwin
                echo "  macOS: brew install fswatch" >&2
            case '*'
                echo "  Ubuntu/Debian: sudo apt install inotify-tools" >&2
                echo "  Fedora/RHEL:   sudo dnf install inotify-tools" >&2
                echo "  Arch:          sudo pacman -S inotify-tools" >&2
        end
        return 1
    end

    # Use shared SRC_DIRS constant from script_lib.fish
    # Make a local copy so we can modify it (add config files)
    set -l watch_dirs $SRC_DIRS

    # Verify directories exist
    for dir in $watch_dirs
        if not test -d $dir
            echo "⚠️  Warning: Directory $dir not found, skipping" >&2
            set -e watch_dirs[(contains -i $dir $watch_dirs)]
        end
    end

    # Add config files to watch list (for detecting config changes mid-session)
    # These are files, not directories, but inotifywait handles both
    for config_file in $CONFIG_FILES_TO_WATCH
        if test -f $config_file
            set watch_dirs $watch_dirs $config_file
        end
    end

    if test (count $watch_dirs) -eq 0
        echo "❌ Error: No valid directories to watch" >&2
        return 1
    end

    # Initialize log file (fresh for each watch session)
    mkdir -p (dirname $CHECK_LOG_FILE)
    echo "["(timestamp)"] Watch mode started" > $CHECK_LOG_FILE

    echo ""
    set_color cyan --bold
    echo "👀 Watch mode activated"
    set_color normal
    echo "Monitoring: "(string join ", " $watch_dirs)
    echo "Log file:   $CHECK_LOG_FILE"
    echo "Press Ctrl+C to stop"
    echo ""

    # Check if config files changed (cleans target if needed)
    check_config_changed $CHECK_TARGET_DIR $CONFIG_FILES_TO_WATCH

    # Validate toolchain BEFORE entering watch loop
    echo "🔧 Validating toolchain..."
    ensure_toolchain_installed
    set -l toolchain_status $status
    if test $toolchain_status -eq 1
        echo ""
        echo "❌ Toolchain validation failed"
        return 1
    end

    # Run initial check
    echo ""
    echo "🚀 Running initial checks..."
    echo ""
    run_checks_for_type $check_type
    set -l initial_result $status

    echo ""
    set_color cyan
    log_and_print $CHECK_LOG_FILE "["(timestamp)"] 👀 Watching for changes..."
    set_color normal
    echo ""

    # ══════════════════════════════════════════════════════════════════════════
    # Main Watch Loop with Sliding Window Debounce
    # ══════════════════════════════════════════════════════════════════════════
    # See header comments for detailed algorithm explanation and ASCII diagram.
    #
    # Summary:
    #   1. Wait for FIRST event (no timeout - blocks forever)
    #   2. Start sliding window: wait DEBOUNCE_WINDOW_SECS for quiet
    #   3. If new event arrives: reset window (loop back to step 2)
    #   4. If timeout expires: quiet period detected, run checks
    #   5. Go back to step 1
    # ══════════════════════════════════════════════════════════════════════════

    while true
        # ──────────────────────────────────────────────────────────────────────
        # PHASE 1: Wait for first event (blocks forever until file changes)
        # ──────────────────────────────────────────────────────────────────────
        # Use TARGET_CHECK_INTERVAL_SECS timeout to periodically check if
        # target/ directory was deleted externally (cargo clean, rm -rf, etc.)

        wait_for_file_changes $TARGET_CHECK_INTERVAL_SECS $watch_dirs
        set -l wait_status $status

        # Check if target/ directory is missing (regardless of event or timeout)
        # This handles external deletions (cargo clean, manual rm -rf target/, etc.)
        if not test -d "$CHECK_TARGET_DIR"
            echo ""
            set_color yellow
            echo "["(timestamp)"] 📁 target/ missing, triggering rebuild..."
            set_color normal
            echo ""

            run_checks_for_type $check_type
            set -l result $status

            # Handle ICE detected (status 2)
            if test $result -eq 2
                cleanup_after_ice
            end

            echo ""
            set_color cyan
            log_and_print $CHECK_LOG_FILE "["(timestamp)"] 👀 Watching for changes..."
            set_color normal
            echo ""
            continue
        end

        # If timeout (status 2) with target/ present, just loop back to keep watching
        if test $wait_status -eq 2
            continue
        end

        # ──────────────────────────────────────────────────────────────────────
        # PHASE 2: Sliding window - wait for "quiet period" before running
        # ──────────────────────────────────────────────────────────────────────
        # Each new event resets the window. Only when DEBOUNCE_WINDOW_SECS pass
        # with NO new events do we proceed to run checks.

        echo ""
        set_color brblack
        log_and_print $CHECK_LOG_FILE "["(timestamp)"] 📝 Change detected, waiting for quiet ("$DEBOUNCE_WINDOW_SECS"s window)..."
        set_color normal

        while true
            # Record window start time for remaining time calculation
            set -l window_start (date +%s.%N)

            wait_for_file_changes $DEBOUNCE_WINDOW_SECS $watch_dirs
            set -l debounce_status $status

            if test $debounce_status -eq 2
                # Timeout! No events for DEBOUNCE_WINDOW_SECS seconds = quiet period
                break
            else if test $debounce_status -eq 0
                # Another event arrived - calculate how much time was remaining
                set -l now (date +%s.%N)
                set -l elapsed (math "$now - $window_start")
                set -l remaining (math "$DEBOUNCE_WINDOW_SECS - $elapsed")
                # Format remaining time (round to 1 decimal)
                set -l remaining_str (math --scale=1 "$remaining")

                set_color brblack
                log_and_print $CHECK_LOG_FILE "["(timestamp)"] 📝 Another change, resetting window... (was "$remaining_str"s remaining)"
                set_color normal
                continue
            else
                # Error (status 1) - break out and proceed
                break
            end
        end

        # ──────────────────────────────────────────────────────────────────────
        # PHASE 3: Run checks (quiet period detected)
        # ──────────────────────────────────────────────────────────────────────

        # Check if config files changed (cleans target if needed)
        check_config_changed $CHECK_TARGET_DIR $CONFIG_FILES_TO_WATCH

        echo ""
        set_color yellow
        log_and_print $CHECK_LOG_FILE "["(timestamp)"] 🔄 Quiet period reached, running checks..."
        set_color normal
        echo ""

        run_checks_for_type $check_type
        set -l result $status

        # Handle ICE detected (status 2)
        if test $result -eq 2
            cleanup_after_ice
            # Note: cleanup_after_ice will trigger another check, continuing naturally
        end

        echo ""
        set_color cyan
        log_and_print $CHECK_LOG_FILE "["(timestamp)"] 👀 Watching for changes..."
        set_color normal
        echo ""
    end
end

# Helper function to run checks based on type
# Parameters: check_type - "full", "test", or "doc"
# Returns: 0 = success, 1 = failure, 2 = ICE detected (triggers cleanup in watch loop)
function run_checks_for_type
    set -l check_type $argv[1]

    switch $check_type
        case "full"
            # Full checks: all three
            run_watch_checks
            set -l result $status

            if test $result -eq 2
                # ICE detected - let caller (watch loop) handle cleanup
                return 2
            end

            if test $result -eq 0
                echo ""
                set_color green --bold
                echo "["(timestamp)"] ✅ All checks passed!"
                set_color normal
                send_system_notification "Watch: All Passed ✅" "Tests, doctests, and docs passed" "success" $NOTIFICATION_EXPIRE_MS
            else
                # Notify on failure in watch mode
                send_system_notification "Watch: Checks Failed ❌" "One or more checks failed" "critical" $NOTIFICATION_EXPIRE_MS
            end
            return $result

        case "test"
            # Test checks: cargo test + doctests only
            run_check_with_recovery check_cargo_test "tests"
            if test $status -eq 2
                return 2  # ICE detected
            end
            if test $status -ne 0
                send_system_notification "Watch: Tests Failed ❌" "cargo test failed" "critical" $NOTIFICATION_EXPIRE_MS
                return 1
            end

            run_check_with_recovery check_doctests "doctests"
            if test $status -eq 2
                return 2  # ICE detected
            end
            if test $status -ne 0
                send_system_notification "Watch: Doctests Failed ❌" "doctests failed" "critical" $NOTIFICATION_EXPIRE_MS
                return 1
            end

            echo ""
            set_color green --bold
            echo "["(timestamp)"] ✅ All test checks passed!"
            set_color normal
            send_system_notification "Watch: Tests Passed ✅" "All tests and doctests passed" "success" $NOTIFICATION_EXPIRE_MS
            return 0

        case "doc"
            # Doc checks: quick build BLOCKING, full build FORKED to background
            # Architecture: Each build has its own staging directory, both sync to shared serving dir.
            #
            # Why quick blocks but full forks?
            # - Cargo uses a global package cache lock (~/.cargo/.package-cache)
            # - Running both simultaneously causes "Blocking waiting for file lock" messages
            # - Quick build targets only r3bl_tui (~3-5s), so blocking is acceptable
            # - Full build is slow (~90s), so forking lets user continue editing
            #
            # Build flow:
            # 1. Run quick build (blocking) → staging-quick → sync to serving → notify
            # 2. Check for changes during build (catch-up if needed)
            # 3. Fork full build → staging-full → sync to serving → patch if needed → notify
            #
            # Catch-up mechanism (quick build):
            # While the quick build runs (~3-5s), inotifywait isn't watching for changes.
            # If the user saves a file during this "blind spot", the change would be lost.
            # After the quick build, we use has_source_changes_since to detect changes.
            #
            # Catch-up mechanism (full build):
            # The full build (~90s) also has a blind spot. After syncing dep docs,
            # run_full_doc_build_task checks for changes and patches with quick docs if needed.

            # Step 1: Quick build (BLOCKING - targets only r3bl_tui for fast feedback)
            log_and_print $CHECK_LOG_FILE "["(timestamp)"] 🔨 Quick build starting (r3bl_tui only)..."

            # Capture build start time for catch-up detection
            set -l build_start_epoch (date +%s)

            # Use extracted function for quick build + sync
            if build_and_sync_quick_docs $CHECK_TARGET_DIR_DOC_STAGING_QUICK $CHECK_TARGET_DIR
                log_and_print $CHECK_LOG_FILE "["(timestamp)"] 📄 Quick build done!"
                log_and_print $CHECK_LOG_FILE "    📖 Read the docs at: file://$CHECK_TARGET_DIR/doc/r3bl_tui/index.html"
                log_and_print $CHECK_LOG_FILE ""
                send_system_notification "Watch: Quick Docs Ready ⚡" "r3bl_tui done w/ broken dep links - full build starting" "success" $NOTIFICATION_EXPIRE_MS

                # Step 1.5: Catch-up check - did any source files change during the build?
                # Uses has_source_changes_since from script_lib.fish (checks SRC_DIRS)
                if has_source_changes_since $build_start_epoch
                    log_and_print $CHECK_LOG_FILE "["(timestamp)"] ⚡ Files changed during build, catching up..."
                    if build_and_sync_quick_docs $CHECK_TARGET_DIR_DOC_STAGING_QUICK $CHECK_TARGET_DIR
                        log_and_print $CHECK_LOG_FILE "["(timestamp)"] 📄 Catch-up build done!"
                    else
                        log_and_print $CHECK_LOG_FILE "["(timestamp)"] ⚠️ Catch-up build failed (non-fatal)"
                    end
                end
            else
                log_and_print $CHECK_LOG_FILE "["(timestamp)"] ❌ Quick build failed!"
                send_system_notification "Watch: Quick Doc Build Failed ❌" "cargo doc -p r3bl_tui failed" "critical" $NOTIFICATION_EXPIRE_MS
                return 1
            end

            # Step 2: Full build (FORKED - runs in background while user continues editing)
            # Uses run_full_doc_build_task from script_lib.fish which handles:
            # - Building full docs
            # - Syncing to serving (deps are always valid)
            # - Catch-up check for changes during full build
            # - Patching with quick docs if changes detected
            # - Desktop notifications
            log_and_print $CHECK_LOG_FILE "["(timestamp)"] 🔀 Forking full build to background..."

            fish -c "
                cd $PWD
                source script_lib.fish
                run_full_doc_build_task \
                    $CHECK_TARGET_DIR_DOC_STAGING_FULL \
                    $CHECK_TARGET_DIR_DOC_STAGING_QUICK \
                    $CHECK_TARGET_DIR \
                    $CHECK_LOG_FILE \
                    $NOTIFICATION_EXPIRE_MS
            " &

            # Return immediately - quick build done, full build running in background
            return 0

        case '*'
            echo "❌ Unknown check type: $check_type" >&2
            return 1
    end
end

# ============================================================================
# Level 1: Pure Check Functions
# ============================================================================
# These functions just run the command and return status code
# They do NOT handle output formatting or ICE detection
#
# All cargo commands are wrapped with ionice for higher I/O scheduling priority:
#   -c2 = best-effort class (no sudo required, unlike realtime class 1)
#   -n0 = highest priority within the class (range: 0-7, lower = higher priority)
# This helps when other processes compete for disk I/O during builds.

function check_cargo_check
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR
    ionice_wrapper cargo check
end

function check_cargo_build
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR
    ionice_wrapper cargo build
end

function check_clippy
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR
    ionice_wrapper cargo clippy --all-targets
end

function check_cargo_test
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR
    ionice_wrapper cargo test --all-targets -q
end

function check_doctests
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR
    ionice_wrapper cargo test --doc -q
end

# Quick doc check without dependencies (for one-off --doc mode).
# Builds to QUICK staging directory to avoid race conditions with background full builds.
function check_docs_quick
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR_DOC_STAGING_QUICK
    ionice_wrapper cargo doc --no-deps
end

# One-off doc check for normal mode (./check.fish without flags).
# Builds directly to CHECK_TARGET_DIR to avoid conflicts with --watch-doc's staging dirs.
# Uses --no-deps for speed since this is just a verification step.
#
# Key insight: Normal one-off mode and --watch-doc can run simultaneously because they
# use different target directories:
#   - One-off: CHECK_TARGET_DIR (/tmp/roc/target/check)
#   - Watch-doc: staging dirs (/tmp/roc/target/check-doc-staging-*)
#
# Trade-off: During build, the doc folder is temporarily empty (cargo clears it first).
# This is acceptable for one-off mode since users typically wait for completion before
# refreshing the browser.
function check_docs_oneoff
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR
    ionice_wrapper cargo doc --no-deps
end

# Full doc build including dependencies (for watch modes).
# Builds to FULL staging directory to avoid race conditions with quick builds.
function check_docs_full
    set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR_DOC_STAGING_FULL
    ionice_wrapper cargo doc
end

# Atomically sync generated docs from staging to serving directory.
#
# Delete behavior (--delete flag):
# - Quick builds: NEVER use --delete (would wipe dependency docs from full builds)
# - Full builds: Use --delete ONLY when orphan files detected (serving > staging count)
#
# Why orphan detection matters for long-running watch sessions:
# - Renamed/deleted source files leave behind stale .html files in serving dir
# - Without cleanup, these accumulate over days of development
# - File count comparison detects this: if serving has MORE files than staging,
#   those extra files are orphans that should be removed
#
# Parameters:
#   $argv[1]: "quick" or "full" - which staging directory to sync from
function sync_docs_to_serving
    set -l build_type $argv[1]
    set -l serving_doc_dir $CHECK_TARGET_DIR/doc

    # Select staging directory based on build type
    # NOTE: Must declare staging_doc_dir OUTSIDE the if block, otherwise
    # "set -l" creates a variable scoped to the if block that vanishes.
    set -l staging_doc_dir
    if test "$build_type" = "full"
        set staging_doc_dir $CHECK_TARGET_DIR_DOC_STAGING_FULL/doc
    else
        set staging_doc_dir $CHECK_TARGET_DIR_DOC_STAGING_QUICK/doc
    end

    # Ensure serving doc directory exists
    mkdir -p $serving_doc_dir

    # Determine if we should use --delete (only for full builds with orphans)
    # NOTE: Must NOT initialize to "" - Fish treats that as a 1-element list containing
    # an empty string, which rsync receives as an empty argument causing errors.
    # Leaving uninitialized creates a 0-element list that expands to nothing.
    set -l delete_flag
    if test "$build_type" = "full"
        if has_orphan_files $staging_doc_dir $serving_doc_dir
            set delete_flag "--delete"
            set_color yellow
            echo "    🧹 Cleaning orphaned doc files (serving > staging)"
            set_color normal
        end
    end

    # -a = archive mode (preserves permissions, timestamps)
    # --delete (conditional): removes orphaned files when serving has more than staging
    rsync -a $delete_flag $staging_doc_dir/ $serving_doc_dir/
end

# Check if serving directory has orphan files (more files than staging).
# This indicates stale docs from renamed/deleted source files.
#
# Parameters:
#   $argv[1]: staging doc directory (source of truth)
#   $argv[2]: serving doc directory (may have orphans)
#
# Returns: 0 if orphans detected (serving > staging), 1 otherwise
function has_orphan_files
    set -l staging_dir $argv[1]
    set -l serving_dir $argv[2]

    # If serving dir doesn't exist yet, no orphans possible
    if not test -d $serving_dir
        return 1
    end

    # If staging dir doesn't exist, something is wrong - don't delete
    if not test -d $staging_dir
        return 1
    end

    # Count files in each directory (fast - just readdir operations)
    set -l staging_count (find $staging_dir -type f 2>/dev/null | wc -l)
    set -l serving_count (find $serving_dir -type f 2>/dev/null | wc -l)

    # Orphans exist if serving has MORE files than staging
    test $serving_count -gt $staging_count
end

# Temp file path for passing duration from run_check_with_recovery to callers.
# Using a well-known path avoids global variable side effects while still
# allowing callers to explicitly opt-in to reading the duration.
set -g CHECK_DURATION_FILE /tmp/check_fish_duration.txt

# ============================================================================
# Level 2: ICE-Aware Wrapper Function
# ============================================================================
# Generic wrapper that handles output, formatting, ICE detection, and timing
# Can wrap ANY check function and apply consistent error handling
#
# Strategy: Uses temp file to preserve ANSI codes and terminal formatting
# while still allowing output to be parsed for ICE detection
#
# Parameters:
#   $argv[1]: Function name to wrap (e.g., "check_nextest")
#   $argv[2]: Display label (e.g., "nextest")
#
# Returns:
#   0 = Success
#   1 = Failure (not ICE)
#   2 = ICE detected
#
# Duration: Written to $CHECK_DURATION_FILE (seconds as decimal).
#           Callers can read with: set duration (cat $CHECK_DURATION_FILE)
function run_check_with_recovery
    set -l check_func $argv[1]
    set -l check_name $argv[2]

    echo ""
    set_color cyan
    echo "["(timestamp)"] ▶️  Running $check_name..."
    set_color normal

    # Log to file (without duplicate timestamp since log_message adds one)
    if set -q CHECK_LOG_FILE; and test -n "$CHECK_LOG_FILE"
        echo "["(timestamp)"] ▶️  Running $check_name..." >> $CHECK_LOG_FILE
    end

    # Create temp file for ICE detection (output is suppressed)
    set -l temp_output (mktemp)

    # Record start time (epoch seconds with nanosecond precision)
    set -l start_time (date +%s.%N)

    # Run command with output suppressed to temp file
    # Redirecting both stdout and stderr to file hides all output
    $check_func >$temp_output 2>&1
    set -l exit_code $status

    # Calculate duration and write to file for callers to read
    set -l end_time (date +%s.%N)
    set -l duration_secs (math "$end_time - $start_time")
    echo $duration_secs >$CHECK_DURATION_FILE
    set -l duration_str (format_duration $duration_secs)

    if test $exit_code -eq 0
        set_color green
        echo "["(timestamp)"] ✅ $check_name passed ($duration_str)"
        set_color normal

        # Log success to file
        if set -q CHECK_LOG_FILE; and test -n "$CHECK_LOG_FILE"
            echo "["(timestamp)"] ✅ $check_name passed ($duration_str)" >> $CHECK_LOG_FILE
        end

        command rm -f $temp_output
        return 0
    end

    # Check for ICE (Internal Compiler Error)
    if detect_ice_from_file $exit_code $temp_output
        set_color red
        echo "🧊 ICE detected (Internal Compiler Error) ($duration_str)"
        set_color normal

        # Log ICE to file with details
        if set -q CHECK_LOG_FILE; and test -n "$CHECK_LOG_FILE"
            echo "["(timestamp)"] 🧊 ICE detected during $check_name ($duration_str)" >> $CHECK_LOG_FILE
        end

        command rm -f $temp_output
        return 2
    end

    # Regular failure (not ICE) - show the error output to user
    set_color red
    echo "["(timestamp)"] ❌ $check_name failed ($duration_str)"
    set_color normal

    # Log failure to file
    if set -q CHECK_LOG_FILE; and test -n "$CHECK_LOG_FILE"
        echo "["(timestamp)"] ❌ $check_name failed ($duration_str)" >> $CHECK_LOG_FILE
    end

    echo ""
    # Use grep to extract just the relevant error lines (much cleaner than cat)
    # Look for actual error patterns (case-insensitive for FAILED, error:, panicked, etc.)
    # Also strip carriage returns to avoid overlapping text from cargo's progress indicators
    grep -iE "^error:|^(.*---\s+)?FAILED|panicked at|assertion|test result: FAILED" $temp_output | tr -d '\r'
    command rm -f $temp_output
    return 1
end

# ============================================================================
# Level 3: Orchestrator Functions
# ============================================================================
# Composes multiple Level 2 wrappers
# Aggregates results: ICE > Failure > Success
#
# Two variants:
#   - run_watch_checks: Full docs (for watch "full" mode)
#   - run_oneoff_checks: Quick docs to CHECK_TARGET_DIR (for one-off mode)
#
# Returns:
#   0 = All checks passed
#   1 = At least one check failed (not ICE)
#   2 = At least one check had ICE (caller handles retry)

# Orchestrator for watch "full" mode (--watch).
# Uses full doc build with dependencies.
function run_watch_checks
    set -l result_cargo_test 0
    set -l result_doctest 0
    set -l result_docs 0

    run_check_with_recovery check_cargo_test "tests"
    set result_cargo_test $status

    run_check_with_recovery check_doctests "doctests"
    set result_doctest $status

    # Full doc build with deps (used in watch "full" mode)
    run_check_with_recovery check_docs_full "docs"
    set result_docs $status

    # Aggregate: return 2 if ANY ICE, then 1 if ANY failure, else 0
    if test $result_cargo_test -eq 2 || test $result_doctest -eq 2 || test $result_docs -eq 2
        return 2
    end

    if test $result_cargo_test -ne 0 || test $result_doctest -ne 0 || test $result_docs -ne 0
        return 1
    end

    return 0
end

# Orchestrator for one-off normal mode (./check.fish without flags).
# Uses quick docs (--no-deps) built directly to CHECK_TARGET_DIR.
# This avoids conflicts with --watch-doc which uses staging directories.
function run_oneoff_checks
    set -l result_cargo_test 0
    set -l result_doctest 0
    set -l result_docs 0

    run_check_with_recovery check_cargo_test "tests"
    set result_cargo_test $status

    run_check_with_recovery check_doctests "doctests"
    set result_doctest $status

    # Quick doc build to CHECK_TARGET_DIR (no conflict with watch-doc)
    run_check_with_recovery check_docs_oneoff "docs"
    set result_docs $status

    # Aggregate: return 2 if ANY ICE, then 1 if ANY failure, else 0
    if test $result_cargo_test -eq 2 || test $result_doctest -eq 2 || test $result_docs -eq 2
        return 2
    end

    if test $result_cargo_test -ne 0 || test $result_doctest -ne 0 || test $result_docs -ne 0
        return 1
    end

    return 0
end

# ============================================================================
# Level 4: Top-Level Recovery Functions
# ============================================================================
# Handles ICE recovery with automatic cleanup and retry.
#
# Two variants:
#   - run_oneoff_checks_with_ice_recovery: For one-off normal mode (uses quick docs)
#   - run_watch_checks_with_ice_recovery: For watch "full" mode (uses full docs)
#
# Returns:
#   0 = All checks passed
#   1 = Checks failed

# Recovery function for one-off normal mode.
# Uses run_oneoff_checks (quick docs to CHECK_TARGET_DIR).
function run_oneoff_checks_with_ice_recovery
    set -l max_retries 1
    set -l retry_count 0

    while test $retry_count -le $max_retries
        run_oneoff_checks
        set -l result $status

        # If not ICE, we're done
        if test $result -ne 2
            echo ""
            if test $result -eq 0
                set_color green --bold
                echo "["(timestamp)"] ✅ All checks passed!"
                set_color normal
            else
                set_color red --bold
                echo "["(timestamp)"] ❌ Checks failed"
                set_color normal
            end
            return $result
        end

        # ICE detected - cleanup and retry
        cleanup_after_ice
        set retry_count (math $retry_count + 1)
    end

    echo ""
    set_color red --bold
    echo "["(timestamp)"] ❌ Failed even after ICE recovery"
    set_color normal
    return 1
end

# Recovery function for watch "full" mode (--watch).
# Uses run_watch_checks (full docs with dependencies).
function run_watch_checks_with_ice_recovery
    set -l max_retries 1
    set -l retry_count 0

    while test $retry_count -le $max_retries
        run_watch_checks
        set -l result $status

        # If not ICE, we're done
        if test $result -ne 2
            echo ""
            if test $result -eq 0
                set_color green --bold
                echo "["(timestamp)"] ✅ All checks passed!"
                set_color normal
            else
                set_color red --bold
                echo "["(timestamp)"] ❌ Checks failed"
                set_color normal
            end
            return $result
        end

        # ICE detected - cleanup and retry
        cleanup_after_ice
        set retry_count (math $retry_count + 1)
    end

    echo ""
    set_color red --bold
    echo "["(timestamp)"] ❌ Failed even after ICE recovery"
    set_color normal
    return 1
end

# ============================================================================
# Level 3b: Full Orchestrator Function (includes clippy)
# ============================================================================
# Composes all checks including check, build, and clippy
# Aggregates results: ICE > Failure > Success
#
# Returns:
#   0 = All checks passed
#   1 = At least one check failed (not ICE)
#   2 = At least one check had ICE
function run_full_checks
    set -l result_check 0
    set -l result_build 0
    set -l result_clippy 0
    set -l result_cargo_test 0
    set -l result_doctest 0
    set -l result_docs 0

    run_check_with_recovery check_cargo_check "typecheck"
    set result_check $status

    run_check_with_recovery check_cargo_build "build"
    set result_build $status

    run_check_with_recovery check_clippy "clippy"
    set result_clippy $status

    run_check_with_recovery check_cargo_test "tests"
    set result_cargo_test $status

    run_check_with_recovery check_doctests "doctests"
    set result_doctest $status

    run_check_with_recovery check_docs_full "docs"
    set result_docs $status

    # Aggregate: return 2 if ANY ICE, then 1 if ANY failure, else 0
    if test $result_check -eq 2 || test $result_build -eq 2 || test $result_clippy -eq 2 || \
       test $result_cargo_test -eq 2 || test $result_doctest -eq 2 || test $result_docs -eq 2
        return 2
    end

    if test $result_check -ne 0 || test $result_build -ne 0 || test $result_clippy -ne 0 || \
       test $result_cargo_test -ne 0 || test $result_doctest -ne 0 || test $result_docs -ne 0
        return 1
    end

    return 0
end

# ============================================================================
# Level 4b: Full Recovery Function with Toolchain Escalation
# ============================================================================
# Handles ICE recovery with automatic cleanup, retry, and toolchain update escalation.
#
# Escalation flow:
#   1. ICE detected → cleanup target/ → retry
#   2. Still ICE? → escalate to rust-toolchain-update.fish (finds working nightly)
#   3. Retry once more with new toolchain
#
# Returns:
#   0 = All checks passed
#   1 = Checks failed
function run_full_checks_with_ice_recovery
    # First attempt
    run_full_checks
    set -l result $status

    # If not ICE, we're done
    if test $result -ne 2
        return $result
    end

    # ICE detected - cleanup and retry
    echo ""
    set_color yellow
    echo "🧊 ICE detected, cleaning target/ and retrying..."
    set_color normal
    cleanup_after_ice

    run_full_checks
    set result $status

    # If not ICE after cleanup, we're done
    if test $result -ne 2
        return $result
    end

    # Still ICE - escalate to toolchain update
    echo ""
    set_color yellow --bold
    echo "═══════════════════════════════════════════════════════"
    echo "🔧 ICE persists after cache cleanup."
    echo "   The pinned nightly toolchain may have bugs."
    echo "   Escalating to rust-toolchain-update.fish to find a stable nightly..."
    echo "═══════════════════════════════════════════════════════"
    set_color normal
    echo ""

    # Run toolchain update to find a working nightly
    if fish ./rust-toolchain-update.fish
        echo ""
        set_color green
        echo "✅ Toolchain updated successfully. Retrying checks..."
        set_color normal
        echo ""

        # Final retry with new toolchain
        run_full_checks
        set result $status
        return $result
    else
        echo ""
        set_color red --bold
        echo "❌ Toolchain update failed. Cannot recover from ICE."
        echo "   Check ~/Downloads/rust-toolchain-update.log for details."
        set_color normal
        return 1
    end
end

# ============================================================================
# Toolchain Validation Functions
# ============================================================================

# Detects if a toolchain has a corrupted installation (e.g., "Missing manifest").
#
# A toolchain can appear in `rustup toolchain list` but be corrupted internally.
# This happens when installation is interrupted, download fails, or manifest is lost.
# Symptoms: "Missing manifest in toolchain", repeated "syncing channel updates" loops.
#
# Parameters:
#   $argv[1]: toolchain name (e.g., "nightly-2025-12-24")
#
# Returns: 0 if corrupted, 1 if OK or not installed
function is_toolchain_corrupted
    set -l toolchain $argv[1]

    # If not installed at all, it's not "corrupted" (just missing)
    if not is_toolchain_installed $toolchain
        return 1
    end

    # Try to run rustc and capture stderr for corruption patterns
    set -l rustc_output (rustup run $toolchain rustc --version 2>&1)
    set -l rustc_status $status

    # Check for known corruption patterns
    if echo "$rustc_output" | grep -qi "Missing manifest"
        return 0  # Corrupted
    end

    # Also check component listing (another way corruption manifests)
    set -l component_output (rustup component list --toolchain $toolchain 2>&1)
    if echo "$component_output" | grep -qi "Missing manifest"
        return 0  # Corrupted
    end

    # If rustc failed but no specific corruption pattern, check if it's truly broken
    if test $rustc_status -ne 0
        # Could be corruption or just missing components - check for other patterns
        if echo "$rustc_output" | grep -qi "error: toolchain .* is not installed"
            return 0  # Corrupted (claims not installed but is in list)
        end
    end

    return 1  # Not corrupted
end

# Force-removes a corrupted toolchain, including direct folder deletion if needed.
#
# When a toolchain has "Missing manifest", `rustup toolchain uninstall` may fail.
# This function tries rustup first, then falls back to direct folder deletion.
#
# Parameters:
#   $argv[1]: toolchain name (e.g., "nightly-2025-12-24")
#
# Returns: 0 if removed (or wasn't installed), 1 if removal failed
function force_remove_corrupted_toolchain
    set -l toolchain $argv[1]
    set -l toolchain_dir "$HOME/.rustup/toolchains/$toolchain-x86_64-unknown-linux-gnu"

    echo "🔧 Force-removing corrupted toolchain: $toolchain"

    # Try rustup uninstall first (might work even if corrupted)
    if rustup toolchain uninstall $toolchain 2>/dev/null
        echo "   ✅ Removed via rustup uninstall"
        return 0
    end

    # Rustup failed - try direct folder deletion
    if test -d "$toolchain_dir"
        echo "   ⚠️  rustup uninstall failed, removing folder directly..."
        if command rm -rf "$toolchain_dir"
            echo "   ✅ Removed folder: $toolchain_dir"
            # Also clear rustup caches to prevent stale state
            command rm -rf ~/.rustup/downloads/ 2>/dev/null
            command rm -rf ~/.rustup/tmp/ 2>/dev/null
            return 0
        else
            echo "   ❌ Failed to remove folder" >&2
            return 1
        end
    end

    # Folder doesn't exist - toolchain is effectively removed
    echo "   ✅ Toolchain folder already gone"
    return 0
end

# Helper function to ensure correct toolchain is installed with validation.
#
# Performs validation checks and handles recovery from various failure modes:
# - Missing toolchain: installs via sync script
# - Missing components: installs via sync script
# - Corrupted toolchain: force-removes first, then reinstalls
#
# Returns: 0 if toolchain is OK, 1 if error, 2 if toolchain was reinstalled
#
# Uses library functions for validation, delegates to sync script for installation.
# No lock needed for validation - sync script manages its own lock.
function ensure_toolchain_installed
    set -l target_toolchain (read_toolchain_from_toml)
    if test $status -ne 0
        echo "❌ Failed to read toolchain from rust-toolchain.toml" >&2
        return 1
    end

    # First, check for corruption (toolchain exists but is broken)
    # This must be handled BEFORE normal validation, otherwise we get stuck in loops
    if is_toolchain_corrupted $target_toolchain
        echo "🧊 Detected corrupted toolchain installation: $target_toolchain"
        echo "   Symptoms: Missing manifest, incomplete installation"
        echo ""
        force_remove_corrupted_toolchain $target_toolchain
        # Continue to reinstall below
    end

    # Perform quick validation using library functions (read-only, no lock needed)
    set -l validation_failed 0
    set -l failure_reason ""

    # Check if toolchain is installed
    if not is_toolchain_installed $target_toolchain
        set validation_failed 1
        set failure_reason "toolchain not installed"
    end

    # Check rust-analyzer component
    if test $validation_failed -eq 0
        if not is_component_installed $target_toolchain "rust-analyzer"
            set validation_failed 1
            set failure_reason "rust-analyzer component missing"
        end
    end

    # Check rust-src component
    if test $validation_failed -eq 0
        if not is_component_installed $target_toolchain "rust-src"
            set validation_failed 1
            set failure_reason "rust-src component missing"
        end
    end

    # Verify rustc works
    if test $validation_failed -eq 0
        if not rustup run $target_toolchain rustc --version >/dev/null 2>&1
            set validation_failed 1
            set failure_reason "rustc failed to run"
        end
    end

    # If validation passed, we're done
    if test $validation_failed -eq 0
        return 0
    end

    # Validation failed - delegate to sync script for installation
    # The sync script will acquire its own lock to prevent concurrent modifications
    echo "⚠️  Toolchain validation failed ($failure_reason), installing..."
    echo ""

    # Run sync script - capture output to temp file so we can show it on failure
    set -l sync_log (mktemp)
    if fish ./rust-toolchain-sync-to-toml.fish > $sync_log 2>&1
        # Success - clean up and notify
        command rm -f $sync_log
        if command -v notify-send >/dev/null 2>&1
            notify-send --urgency=normal \
                "Toolchain Installation Complete" \
                "✅ Successfully installed: $target_toolchain with all components" \
                2>/dev/null &
        end
        echo "✅ Toolchain $target_toolchain was installed/repaired"
        return 2
    else
        # Failure - show what went wrong
        echo ""
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "❌ Sync script failed. Output (last 30 lines):"
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        tail -n 30 $sync_log
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo ""
        echo "📋 Full log: ~/Downloads/rust-toolchain-sync-to-toml.log"
        command rm -f $sync_log

        if command -v notify-send >/dev/null 2>&1
            notify-send --urgency=critical \
                "Toolchain Installation Failed" \
                "❌ Failed to install $target_toolchain - check terminal" \
                2>/dev/null &
        end
        echo "❌ Toolchain installation failed" >&2
        return 1
    end
end

# ============================================================================
# ICE Detection Functions
# ============================================================================
# Detects Internal Compiler Errors (ICE) by checking for rustc dump files.
#
# When rustc crashes, it creates: rustc-ice-YYYY-MM-DDTHH_MM_SS-PID.txt
# This file-based detection is 100% reliable — no false positives possible.
#
# Note: Detection is only called when cargo commands fail (exit code != 0).
# If commands succeed, there's no ICE to detect.
#
# Recovery is handled by cleanup_after_ice() which removes target/ entirely.
# ============================================================================

function detect_ice_from_file
    if test (count (find . -maxdepth 1 -name "rustc-ice-*.txt" 2>/dev/null)) -gt 0
        return 0
    end
    return 1
end

# Helper function to extract failed test count from cargo test output
function parse_cargo_test_failures
    set -l output $argv[1]
    # Extract the number before "failed" in test result summary
    set -l failed (echo "$output" | grep -oE '[0-9]+\s+failed' | grep -oE '[0-9]+' | tail -1)
    if test -z "$failed"
        echo "0"
    else
        echo $failed
    end
end

# Helper function to extract failed doctest count
function parse_doctest_failures
    set -l output $argv[1]
    # Extract the number before "failed" in doctest result
    set -l failed (echo "$output" | grep -oE '[0-9]+\s+failed' | grep -oE '[0-9]+' | tail -1)
    if test -z "$failed"
        echo "0"
    else
        echo $failed
    end
end

# Helper function to count warnings and errors in doc output
function parse_doc_warnings_errors
    set -l output $argv[1]
    # Count lines containing "warning:" (cargo format: "warning: ...")
    set -l warnings (echo "$output" | grep -ic 'warning:')
    # Count lines containing "error:" (cargo format: "error: ...")
    set -l errors (echo "$output" | grep -ic 'error:')

    # Only return if there are warnings or errors
    if test $warnings -gt 0 -o $errors -gt 0
        echo "$warnings warnings, $errors errors"
        return 0
    else
        return 1
    end
end

# Helper function to clean target folder
# Removes all build artifacts and caches to ensure a clean rebuild
# This is important because various parts of the cache (incremental, metadata, etc.)
# can become corrupted and cause compiler panics or other mysterious failures
function cleanup_target_folder
    echo "🧹 Cleaning target folders..."

    # Clean the main check target directory (tmpfs location)
    if test -d "$CHECK_TARGET_DIR"
        command rm -rf "$CHECK_TARGET_DIR"
    end

    # Also clean staging directories to ensure fresh doc builds
    if test -d "$CHECK_TARGET_DIR_DOC_STAGING_QUICK"
        command rm -rf "$CHECK_TARGET_DIR_DOC_STAGING_QUICK"
    end
    if test -d "$CHECK_TARGET_DIR_DOC_STAGING_FULL"
        command rm -rf "$CHECK_TARGET_DIR_DOC_STAGING_FULL"
    end
end

# Helper function to run cleanup after ICE (Internal Compiler Error)
# Logs ICE events for debugging purposes.
function cleanup_after_ice
    log_message "🧊 ICE detected! Running cleanup..."

    # Remove ICE dump files and log their names for debugging
    set -l ice_files (find . -name "rustc-ice-*.txt" 2>/dev/null)
    if test (count $ice_files) -gt 0
        log_message "🗑️  Removing "(count $ice_files)" ICE dump file(s):"
        for ice_file in $ice_files
            log_message "    - $ice_file"
        end
        command rm -f rustc-ice-*.txt
    end

    # Remove all target folders (build artifacts and caches can become corrupted)
    cleanup_target_folder

    log_message "✨ Cleanup complete. Retrying checks..."
    echo ""
end

# Helper function to log messages to both terminal and log file.
# Ensures log file directory exists and appends to CHECK_LOG_FILE.
# Falls back to echo-only if CHECK_LOG_FILE is not set.
function log_message
    set -l message $argv

    # Always echo to terminal
    echo $message

    # Also append to log file if CHECK_LOG_FILE is set
    if set -q CHECK_LOG_FILE; and test -n "$CHECK_LOG_FILE"
        mkdir -p (dirname $CHECK_LOG_FILE)
        echo "["(timestamp)"] $message" >> $CHECK_LOG_FILE
    end
end

# Deprecated: run_checks
# REFACTORED into composable architecture
# Use run_oneoff_checks_with_ice_recovery instead
# Kept for backward compatibility only
function run_checks
    echo "⚠️  run_checks is deprecated. Use run_oneoff_checks_with_ice_recovery" >&2
    run_oneoff_checks_with_ice_recovery
end

# ============================================================================
# Main Entry Point
# ============================================================================

function main
    # Parse command line arguments
    set -l mode (parse_arguments $argv)
    set -l parse_status $status
    if test $parse_status -ne 0
        return 1
    end

    # Branch based on mode
    switch $mode
        case help
            show_help
            return 0
        case watch
            watch_mode "full"
            return $status
        case watch-test
            watch_mode "test"
            return $status
        case watch-doc
            watch_mode "doc"
            return $status
        case kill
            # Kill any running watch instance and cleanup
            echo "🔪 Killing any running watch instances..."

            # Kill process holding lock file
            if test -f $CHECK_LOCK_FILE
                set -l old_pid (cat $CHECK_LOCK_FILE 2>/dev/null | string trim)
                if is_process_alive $old_pid
                    kill $old_pid 2>/dev/null
                    echo "   Killed watch process (PID: $old_pid)"
                else
                    echo "   No active watch process found"
                end
                command rm -f $CHECK_LOCK_FILE
            else
                echo "   No lock file found"
            end

            # Kill orphaned file watcher processes (inotifywait/fswatch)
            kill_orphaned_watchers

            echo "✅ Cleanup complete"
            return 0
        case check
            # Check-only mode: fast typecheck
            check_config_changed $CHECK_TARGET_DIR $CONFIG_FILES_TO_WATCH

            ensure_toolchain_installed
            set -l toolchain_status $status
            if test $toolchain_status -eq 1
                echo ""
                echo "❌ Cannot proceed without correct toolchain"
                return 1
            end

            echo ""
            echo "🔍 Running typecheck (cargo check)..."
            run_check_with_recovery check_cargo_check "typecheck"
            set -l check_status $status

            if test $check_status -eq 2
                # ICE detected - cleanup and retry once
                cleanup_after_ice
                run_check_with_recovery check_cargo_check "typecheck"
                set check_status $status
            end

            return $check_status
        case build
            # Build-only mode: compile production code
            check_config_changed $CHECK_TARGET_DIR $CONFIG_FILES_TO_WATCH

            ensure_toolchain_installed
            set -l toolchain_status $status
            if test $toolchain_status -eq 1
                echo ""
                echo "❌ Cannot proceed without correct toolchain"
                return 1
            end

            echo ""
            echo "🔨 Building production code (cargo build)..."
            run_check_with_recovery check_cargo_build "build"
            set -l build_status $status

            if test $build_status -eq 2
                # ICE detected - cleanup and retry once
                cleanup_after_ice
                run_check_with_recovery check_cargo_build "build"
                set build_status $status
            end

            return $build_status
        case clippy
            # Clippy-only mode: lint warnings
            check_config_changed $CHECK_TARGET_DIR $CONFIG_FILES_TO_WATCH

            ensure_toolchain_installed
            set -l toolchain_status $status
            if test $toolchain_status -eq 1
                echo ""
                echo "❌ Cannot proceed without correct toolchain"
                return 1
            end

            echo ""
            echo "📎 Running clippy (cargo clippy --all-targets)..."
            run_check_with_recovery check_clippy "clippy"
            set -l clippy_status $status

            if test $clippy_status -eq 2
                # ICE detected - cleanup and retry once
                cleanup_after_ice
                run_check_with_recovery check_clippy "clippy"
                set clippy_status $status
            end

            return $clippy_status
        case full
            # Full mode: comprehensive pre-commit check
            # Runs: check + build + clippy + tests + doctests + docs
            check_config_changed $CHECK_TARGET_DIR $CONFIG_FILES_TO_WATCH

            ensure_toolchain_installed
            set -l toolchain_status $status
            if test $toolchain_status -eq 1
                echo ""
                echo "❌ Cannot proceed without correct toolchain"
                return 1
            end

            echo ""
            echo "🚀 Running comprehensive checks (check + build + clippy + tests + doctests + docs)..."

            # Run all checks with ICE recovery
            run_full_checks_with_ice_recovery
            set -l full_status $status

            # Send desktop notification for final result
            if test $full_status -eq 0
                echo ""
                set_color green --bold
                echo "["(timestamp)"] ✅ All comprehensive checks passed!"
                set_color normal
                send_system_notification "Full Checks Complete ✅" "check, build, clippy, tests, doctests, docs all passed" "success" $NOTIFICATION_EXPIRE_MS
            else
                echo ""
                set_color red --bold
                echo "["(timestamp)"] ❌ Some checks failed"
                set_color normal
                send_system_notification "Full Checks Failed ❌" "One or more checks failed - see terminal" "critical" $NOTIFICATION_EXPIRE_MS
            end

            return $full_status
        case doc
            # Docs-only mode: build docs once without watching
            # Check if config files changed (cleans target if needed)
            check_config_changed $CHECK_TARGET_DIR $CONFIG_FILES_TO_WATCH

            ensure_toolchain_installed
            set -l toolchain_status $status
            if test $toolchain_status -eq 1
                echo ""
                echo "❌ Cannot proceed without correct toolchain"
                return 1
            end

            echo ""
            echo "📚 Building documentation (quick mode, no deps)..."
            run_check_with_recovery check_docs_quick "docs"
            set -l doc_status $status

            if test $doc_status -eq 2
                # ICE detected - cleanup and retry once
                cleanup_after_ice
                run_check_with_recovery check_docs_quick "docs"
                set doc_status $status
            end

            # Read duration from file (written by run_check_with_recovery)
            set -l duration (cat $CHECK_DURATION_FILE)
            set -l duration_str (format_duration $duration)

            if test $doc_status -eq 0
                # Sync docs to serving directory
                sync_docs_to_serving quick

                echo ""
                set_color green --bold
                echo "["(timestamp)"] ✅ Documentation built successfully! ($duration_str)"
                echo "    file://$CHECK_TARGET_DIR/doc/r3bl_tui/index.html"
                set_color normal
                # Only notify if duration > threshold (user likely switched away)
                if test (math "floor($duration)") -ge "$NOTIFICATION_THRESHOLD_SECS"
                    send_system_notification "Doc Build Complete ✅" "Built in $duration_str" "normal" $NOTIFICATION_EXPIRE_MS
                end
            else
                echo ""
                set_color red --bold
                echo "["(timestamp)"] ❌ Documentation build failed ($duration_str)"
                set_color normal
                # Always notify on failure (user needs to know)
                send_system_notification "Doc Build Failed ❌" "Failed after $duration_str - see terminal" "critical" $NOTIFICATION_EXPIRE_MS
            end

            return $doc_status
        case test
            # Test-only mode: run tests once without watching
            # Check if config files changed (cleans target if needed)
            check_config_changed $CHECK_TARGET_DIR $CONFIG_FILES_TO_WATCH

            ensure_toolchain_installed
            set -l toolchain_status $status
            if test $toolchain_status -eq 1
                echo ""
                echo "❌ Cannot proceed without correct toolchain"
                return 1
            end

            echo ""
            echo "🧪 Running tests..."
            run_check_with_recovery check_cargo_test "tests"
            set -l test_status $status

            if test $test_status -eq 2
                # ICE detected - cleanup and retry once
                cleanup_after_ice
                run_check_with_recovery check_cargo_test "tests"
                set test_status $status
            end

            # Read duration from file (written by run_check_with_recovery)
            set -l duration (cat $CHECK_DURATION_FILE)
            set -l duration_str (format_duration $duration)

            if test $test_status -eq 0
                echo ""
                set_color green --bold
                echo "["(timestamp)"] ✅ All tests passed! ($duration_str)"
                set_color normal
                # Only notify if duration > threshold (user likely switched away)
                if test (math "floor($duration)") -ge "$NOTIFICATION_THRESHOLD_SECS"
                    send_system_notification "Tests Complete ✅" "Passed in $duration_str" "normal" $NOTIFICATION_EXPIRE_MS
                end
            else
                echo ""
                set_color red --bold
                echo "["(timestamp)"] ❌ Tests failed ($duration_str)"
                set_color normal
                # Always notify on failure (user needs to know)
                send_system_notification "Tests Failed ❌" "Failed after $duration_str - see terminal" "critical" $NOTIFICATION_EXPIRE_MS
            end

            return $test_status
        case normal
            # Normal mode: run checks once
            # Initialize log file (append mode - preserves history from watch sessions)
            mkdir -p (dirname $CHECK_LOG_FILE)
            log_message ""
            log_message "═══════════════════════════════════════════════════"
            log_message "One-off mode started"
            log_message "═══════════════════════════════════════════════════"

            # Show log file location for debugging
            set_color brblack
            echo "Log file: $CHECK_LOG_FILE"
            set_color normal

            # Check if config files changed (cleans target if needed)
            check_config_changed $CHECK_TARGET_DIR $CONFIG_FILES_TO_WATCH

            # Validate toolchain first
            # No lock needed - validation is read-only, installation delegates to sync script
            ensure_toolchain_installed
            set -l toolchain_status $status
            if test $toolchain_status -eq 1
                echo ""
                log_message "❌ Cannot proceed without correct toolchain"
                return 1
            end

            # toolchain_status can be 0 (OK) or 2 (was reinstalled, already printed message)
            echo ""
            log_message "🚀 Running checks..."

            # Use new composable architecture with automatic ICE recovery
            run_oneoff_checks_with_ice_recovery
            set -l check_status $status

            # Log and send desktop notification for final result
            if test $check_status -eq 0
                log_message "✅ All checks passed!"
                send_system_notification "Build Checks Complete ✅" "All tests, doctests, and docs passed" "success" $NOTIFICATION_EXPIRE_MS
            else
                log_message "❌ Checks failed"
                send_system_notification "Build Checks Failed ❌" "One or more checks failed - see terminal" "critical" $NOTIFICATION_EXPIRE_MS
            end

            return $check_status
    end
end

# ============================================================================
# Script Execution
# ============================================================================

main $argv
exit $status
