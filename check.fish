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
# 4. Runs tests using cargo test
# 5. Runs documentation tests
# 6. Builds documentation (quick --no-deps for one-off, full with deps for watch)
# 7. Detects and recovers from Internal Compiler Errors (ICE)
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
# - Calls rust-toolchain-install-validate.fish to verify installation
# - If invalid, calls rust-toolchain-sync-to-toml.fish to reinstall
# - Sends desktop notifications (notify-send) on success/failure
#
# ICE and Stale Cache Detection and Recovery:
# - Detects two types of compiler corruption that require cache cleanup:
#   1. ICE (Internal Compiler Error): rustc crashes/panics, exit code 101,
#      "internal compiler error" in output, or rustc-ice-*.txt dump files
#   2. Stale cache: corrupted incremental compilation artifacts that cause
#      bizarre parser errors like "expected item, found `/`" where the `/`
#      came from corrupted cache data, not actual source code
# - Recovery process:
#   1. Detects corruption via pattern matching in cargo output
#   2. Removes entire target/ folder (rm -rf target)
#   3. Removes any rustc-ice-*.txt dump files
#   4. Retries the failed check once
# - Distinguishes corruption from real code errors by requiring both:
#   - Parser error with single punctuation token (e.g., `/`, `}`, `]`)
#   - "could not compile" message (confirms it's a build failure)
#
# Desktop Notifications:
# - Success: "Toolchain Installation Complete" (normal urgency)
# - Failure: "Toolchain Installation Failed" (critical urgency)
# - Only triggered when toolchain is actually installed/repaired
#
# Single Instance Enforcement (watch modes only):
# - Only ONE watch mode instance can run at a time
# - One-off commands (--test, --doc, default) can run concurrently without issues
# - When entering watch mode, kills any existing check.fish and orphaned inotifywait processes
# - Prevents race conditions where multiple watch instances clean target/check simultaneously
# - This avoids "couldn't create a temp dir" errors during concurrent watch builds
#
# Toolchain Concurrency:
# - Toolchain installation is delegated to rust-toolchain-sync-to-toml.fish which has its own lock
# - If toolchain installation is needed, sync script prevents concurrent modifications
#
# Performance Optimizations:
# - tmpfs: Builds to /tmp/roc/target/check (RAM-based, eliminates disk I/O)
#   ⚠️  Trade-off: Cache lost on reboot, first post-reboot build is cold
# - CARGO_BUILD_JOBS=28: Forces max parallelism (benchmarked: 60% faster for cargo doc)
# - ionice -c2 -n0: Highest I/O priority in best-effort class (no sudo needed)
#
# Watch Mode & Sliding Window Debounce:
# Watch mode uses a single-threaded main loop with sliding window debounce.
# The main thread alternates between BLOCKED (waiting for events/timeout) and
# DISPATCHING (forking build processes). Actual builds run in parallel background processes.
#
# Key Insight: inotifywait with timeout (-t) acts as both an event listener AND
# a timer. This lets us implement sliding window debounce without threads:
#   - Exit code 0: File changed (event detected)
#   - Exit code 2: Timeout expired (no events for N seconds)
#   - Exit code 1: Error
#
# Sliding Window Debounce Algorithm:
# Instead of threshold-based debounce ("ignore if last run was < N seconds ago"),
# we use sliding window debounce ("wait for N seconds of quiet before running").
#
# Algorithm:
#   1. Wait for FIRST event (inotifywait, no timeout - blocks forever)
#   2. Start sliding window: call inotifywait with DEBOUNCE_WINDOW_SECS timeout
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
# Example Timeline (Rapid Saves):
#   0:00  (idle)              Thread BLOCKED, waiting forever for first event
#   0:05  User saves file A   Thread UNBLOCKS, starts 10s debounce window
#   0:08  User saves file B   Window RESET (was 7s remaining, now 10s again)
#   0:12  User saves file C   Window RESET again
#   0:22  (10s of quiet)      TIMEOUT! Thread forks builds, returns immediately
#   0:22  (same instant)      Thread BLOCKED again, waiting for next event
#   0:44  Quick build done    Notification: "Quick docs ready!" (background)
#   1:52  Full build done     Notification: "Full docs built!" (background)
#
# Why This Works:
# - inotifywait does the heavy lifting (efficient kernel-level blocking)
# - No CPU burn while waiting (uses kernel's inotify subsystem)
# - Single mechanism handles both "coalesce rapid saves" AND "wait for quiet"
# - No separate event draining or timestamp tracking needed
#
# Events During Build:
# Since builds are forked to background, the main thread returns to inotifywait
# immediately. File events are processed normally while builds run in parallel.
# If a new change arrives, a new build cycle starts (previous builds continue
# in background until completion).
#
# Doc Build Architecture (--watch-doc):
# Uses a staging → serving pattern with separate directories to prevent race conditions:
#
#   BLOCKING (main thread)              BACKGROUND (forked process)
#   ┌─────────────────┐                 ┌─────────────────┐
#   │  Quick Build    │  completes      │  Full Build     │
#   │  (--no-deps)    │ ───────────────▶│  (with deps)    │
#   └────────┬────────┘   then forks    └────────┬────────┘
#            │                                   │
#            ▼                                   ▼
#   ┌─────────────────────────┐       ┌─────────────────────────┐
#   │ staging-quick/doc/      │       │ staging-full/doc/       │
#   └────────┬────────────────┘       └────────┬────────────────┘
#            │ rsync -a                        │ rsync -a --delete (if orphans)
#            ▼                                 ▼
#   ┌─────────────────────────────────────────────────────────┐
#   │                    serving/doc/                         │
#   │              (browser loads from here)                  │
#   └─────────────────────────────────────────────────────────┘
#
# Why Quick Blocks but Full Forks?
# - Cargo uses a global package cache lock (~/.cargo/.package-cache)
# - Running both simultaneously causes "Blocking waiting for file lock"
# - Quick build is fast (~20s), so blocking is acceptable
# - Full build is slow (~90s), so forking lets user continue editing
#
# Why Two Staging Directories?
# - Full build runs in background while quick build for NEXT change runs
# - If they shared a staging dir, rsync could fail with "directory has vanished"
# - Each build writes to its own staging dir, then rsyncs to shared serving dir
# - Quick builds: rsync -a (no --delete) - preserves dependency docs
# - Full builds: rsync -a --delete (conditional) - cleans orphans when detected
#
# Orphan File Cleanup (full builds only):
# - Long-running watch sessions accumulate stale docs from renamed/deleted files
# - Detection: compare file counts - if serving > staging, orphans exist
# - Cleanup: full builds use --delete flag to remove orphaned files
# - Quick builds never delete (would wipe dependency docs)
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
#   ./check.fish              Run all checks once (tests, doctests, docs with deps)
#   ./check.fish --test       Run tests only (cargo test + doctests)
#   ./check.fish --doc        Build docs only (quick, --no-deps)
#   ./check.fish --watch      Watch mode: run all checks on file changes
#   ./check.fish --watch-test Watch mode: run tests/doctests only
#   ./check.fish --watch-doc  Watch mode: quick docs first, full docs forked to background
#   ./check.fish --help       Show detailed help

# Import shared toolchain utilities
source script_lib.fish

# ============================================================================
# Single Instance Enforcement (watch modes only)
# ============================================================================

# Kill any existing check.fish watch mode instances to prevent race conditions.
# Multiple watch instances sharing target/check can cause build failures when one
# cleans the directory while another is writing to it.
#
# NOTE: Only called when entering watch mode. One-off commands (--test, --doc)
# can safely run concurrently since they don't loop and don't conflict.
#
# Uses pgrep to find processes, excludes current process ($$), then kills them.
# Also kills orphaned inotifywait processes from previous watch mode sessions.
function kill_existing_instances
    set -l current_pid $fish_pid

    # Find other check.fish processes (exclude current process)
    set -l other_pids (pgrep -f "check.fish" | grep -v "^$current_pid\$")

    # Also find any orphaned inotifywait processes from watch mode
    set -l inotify_pids (pgrep -f "inotifywait.*cmdr/src")

    # Combine all PIDs to kill
    set -l all_pids $other_pids $inotify_pids

    if test (count $all_pids) -gt 0
        echo ""
        set_color yellow
        echo "⚠️  Found "(count $all_pids)" existing check.fish instance(s) running"
        echo "🔪 Killing to prevent race conditions..."
        set_color normal

        for pid in $all_pids
            kill $pid 2>/dev/null
        end

        # Give processes time to terminate
        sleep 0.5
        echo "✅ Previous instances terminated"
        echo ""
    end
end

# NOTE: kill_existing_instances is called only for watch modes (see watch_mode function)
# One-off commands (--test, --doc, default) can run concurrently without conflict

# ============================================================================
# Configuration Constants
# ============================================================================

# Sliding window debounce for watch mode (in seconds).
# After detecting a file change, waits for this many seconds of "quiet" (no new changes)
# before running checks. Each new change resets the window, coalescing rapid saves.
# This handles IDE auto-save, formatters, and "oops forgot to save that file" moments.
set -g DEBOUNCE_WINDOW_SECS 5

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
set -gx CARGO_BUILD_JOBS 28

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
# Returns: "help", "test", "doc", "watch", "watch-test", "watch-doc", or "normal"
function parse_arguments
    if test (count $argv) -eq 0
        echo "normal"
        return 0
    end

    switch $argv[1]
        case --help -h
            echo "help"
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
    echo "  ./check.fish              Run all checks once (default)"
    echo "  ./check.fish --test       Run tests only (once)"
    echo "  ./check.fish --doc        Build documentation only (quick, no deps)"
    echo "  ./check.fish --watch      Watch source files and run all checks on changes"
    echo "  ./check.fish --watch-test Watch source files and run tests/doctests only"
    echo "  ./check.fish --watch-doc  Watch source files and run doc build (full with deps)"
    echo "  ./check.fish --help       Show this help message"
    echo ""

    set_color yellow
    echo "FEATURES:"
    set_color normal
    echo "  ✓ Single instance enforcement in watch modes (kills previous watch instances)"
    echo "  ✓ Config change detection (auto-cleans stale artifacts)"
    echo "  ✓ Automatic toolchain validation and repair"
    echo "  ✓ Fast tests using cargo test"
    echo "  ✓ Documentation tests (doctests)"
    echo "  ✓ Documentation building"
    echo "  ✓ ICE and stale cache detection (auto-removes target/, retries once)"
    echo "  ✓ Desktop notifications on toolchain changes"
    echo "  ✓ Target directory auto-recovery in watch modes"
    echo "  ✓ Orphan doc file cleanup (full builds detect and remove stale files)"
    echo "  ✓ Performance optimizations (tmpfs, ionice, parallel jobs)"
    echo ""

    set_color yellow
    echo "ONE-OFF MODES:"
    set_color normal
    echo "  (default)     Runs all checks once: tests, doctests, docs (full)"
    echo "  --test        Runs tests only: cargo test + doctests (once)"
    echo "  --doc         Builds documentation only (--no-deps, quick check)"
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
    echo "  Requirements: inotifywait (installed via bootstrap.sh)"
    echo ""
    echo "  Doc Builds (--watch-doc):"
    echo "  • Quick build (--no-deps) runs first, blocking (~20s)"
    echo "  • Full build (with deps) then forks to background (~90s)"
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
    echo "WORKFLOW:"
    set_color normal
    echo "  1. Checks for config file changes (cleans target if needed)"
    echo "  2. Validates Rust toolchain (nightly + components)"
    echo "  3. Auto-installs/repairs toolchain if needed"
    echo "  4. Runs cargo test (all unit and integration tests)"
    echo "  5. Runs doctests"
    echo "  6. Builds documentation:"
    echo "     • One-off --doc:  cargo doc --no-deps (quick, your crates only)"
    echo "     • --watch-doc:    Forks both quick + full builds to background"
    echo "     • Other watch:    cargo doc (full, includes dependencies)"
    echo "  7. On ICE or stale cache: removes target/, retries once"
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
    echo "  2. Parallel Jobs (CARGO_BUILD_JOBS=28):"
    echo "     • Forces cargo to use all CPU cores for parallel compilation"
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

    # Kill any existing watch mode instances to prevent race conditions
    # One-off commands can run concurrently, but watch modes conflict
    kill_existing_instances

    # Check for inotifywait
    if not command -v inotifywait >/dev/null 2>&1
        echo "❌ Error: inotifywait not found" >&2
        echo "Install with: ./bootstrap.sh" >&2
        echo "Or manually: sudo apt-get install inotify-tools" >&2
        return 1
    end

    # Define directories to watch
    set -l watch_dirs cmdr/src analytics_schema/src tui/src

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

        inotifywait -q -r -t $TARGET_CHECK_INTERVAL_SECS -e modify,create,delete,move \
            --format '%w%f' $watch_dirs >/dev/null 2>&1
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

            inotifywait -q -r -t $DEBOUNCE_WINDOW_SECS -e modify,create,delete,move \
                --format '%w%f' $watch_dirs >/dev/null 2>&1
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
            run_all_checks_with_recovery
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
            # - Quick build is fast (~20s), so blocking is acceptable
            # - Full build is slow (~90s), so forking lets user continue editing
            #
            # Build flow:
            # 1. Run quick build (blocking) → staging-quick → sync to serving → notify
            # 2. Fork full build            → staging-full  → sync to serving → notify

            # Step 1: Quick build (BLOCKING - fast, gets docs to user quickly)
            log_and_print $CHECK_LOG_FILE "["(timestamp)"] 🔨 Quick build starting (--no-deps)..."

            set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR_DOC_STAGING_QUICK
            ionice_wrapper cargo doc --no-deps > /dev/null 2>&1
            set -l quick_result $status

            if test $quick_result -eq 0
                sync_docs_to_serving quick
                log_and_print $CHECK_LOG_FILE "["(timestamp)"] 📄 Quick build done!"
                log_and_print $CHECK_LOG_FILE "    📖 Read the docs at: file://$CHECK_TARGET_DIR/doc/r3bl_tui/index.html"
                log_and_print $CHECK_LOG_FILE ""
                send_system_notification "Watch: Quick Docs Ready 📄" "Local crate docs available - full build starting" "success" $NOTIFICATION_EXPIRE_MS
            else
                log_and_print $CHECK_LOG_FILE "["(timestamp)"] ❌ Quick build failed!"
                send_system_notification "Watch: Quick Doc Build Failed ❌" "cargo doc --no-deps failed" "critical" $NOTIFICATION_EXPIRE_MS
                return $quick_result
            end

            # Step 2: Full build (FORKED - runs in background while user continues editing)
            log_and_print $CHECK_LOG_FILE "["(timestamp)"] 🔀 Forking full build to background..."

            fish -c "
                cd $PWD
                source script_lib.fish

                log_and_print '$CHECK_LOG_FILE' '['(timestamp)'] [bg] 🔨 Full build starting (with deps)...'

                set -lx CARGO_TARGET_DIR $CHECK_TARGET_DIR_DOC_STAGING_FULL
                ionice_wrapper cargo doc > /dev/null 2>&1

                if test \$status -eq 0
                    rsync -a $CHECK_TARGET_DIR_DOC_STAGING_FULL/doc/ $CHECK_TARGET_DIR/doc/
                    log_and_print '$CHECK_LOG_FILE' '['(timestamp)'] [bg] ✅ Full build done!'
                    send_system_notification 'Watch: Full Docs Built ✅' 'All documentation including dependencies built' 'success' $NOTIFICATION_EXPIRE_MS
                else
                    log_and_print '$CHECK_LOG_FILE' '['(timestamp)'] [bg] ❌ Full build failed!'
                    send_system_notification 'Watch: Full Doc Build Failed ❌' 'cargo doc failed' 'critical' $NOTIFICATION_EXPIRE_MS
                end
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
        rm -f $temp_output
        return 0
    end

    # Check for ICE or stale cache - use exit code + temp file check (no string capture needed)
    # This avoids variable corruption issues while still detecting compiler corruption
    if detect_ice_from_file $exit_code $temp_output
        set_color red
        echo "🧊 Compiler corruption detected (ICE or stale cache) ($duration_str)"
        set_color normal
        rm -f $temp_output
        return 2
    end

    # Regular failure (not ICE) - show the error output to user
    set_color red
    echo "["(timestamp)"] ❌ $check_name failed ($duration_str)"
    set_color normal
    echo ""
    # Use grep to extract just the relevant error lines (much cleaner than cat)
    # Look for actual error patterns (case-insensitive for FAILED, error:, panicked, etc.)
    # Also strip carriage returns to avoid overlapping text from cargo's progress indicators
    grep -iE "^error:|^(.*---\s+)?FAILED|panicked at|assertion|test result: FAILED" $temp_output | tr -d '\r'
    rm -f $temp_output
    return 1
end

# ============================================================================
# Level 3: Orchestrator Function
# ============================================================================
# Composes multiple Level 2 wrappers
# Aggregates results: ICE > Failure > Success
#
# Returns:
#   0 = All checks passed
#   1 = At least one check failed (not ICE)
#   2 = At least one check had ICE
function run_all_checks_with_recovery
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

# ============================================================================
# Level 4: Top-Level Recovery Function
# ============================================================================
# Handles ICE recovery with automatic cleanup and retry
# Single entry point for both one-off and watch modes
#
# Returns:
#   0 = All checks passed
#   1 = Checks failed
function run_checks_with_ice_recovery
    set -l max_retries 1
    set -l retry_count 0

    while test $retry_count -le $max_retries
        run_all_checks_with_recovery
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
# Toolchain Validation Functions
# ============================================================================

# Helper function to ensure correct toolchain is installed with validation
# Returns 0 if toolchain is OK, 1 if error, 2 if toolchain was reinstalled
#
# Uses library functions for validation, delegates to sync script for installation
# No lock needed - validation is read-only, sync script manages its own lock
function ensure_toolchain_installed
    set -l target_toolchain (read_toolchain_from_toml)
    if test $status -ne 0
        echo "❌ Failed to read toolchain from rust-toolchain.toml" >&2
        return 1
    end

    # Perform quick validation using library functions (read-only, no lock needed)
    set -l validation_failed 0

    # Check if toolchain is installed
    if not is_toolchain_installed $target_toolchain
        set validation_failed 1
    end

    # Check rust-analyzer component
    if test $validation_failed -eq 0
        if not is_component_installed $target_toolchain "rust-analyzer"
            set validation_failed 1
        end
    end

    # Check rust-src component
    if test $validation_failed -eq 0
        if not is_component_installed $target_toolchain "rust-src"
            set validation_failed 1
        end
    end

    # Verify rustc works
    if test $validation_failed -eq 0
        if not rustup run $target_toolchain rustc --version >/dev/null 2>&1
            set validation_failed 1
        end
    end

    # If validation passed, we're done
    if test $validation_failed -eq 0
        return 0
    end

    # Validation failed - delegate to sync script for installation
    # The sync script will acquire its own lock to prevent concurrent modifications
    echo "⚠️  Toolchain validation failed, installing..."

    if fish ./rust-toolchain-sync-to-toml.fish >/dev/null 2>&1
        # Send success notification
        if command -v notify-send >/dev/null 2>&1
            notify-send --urgency=normal \
                "Toolchain Installation Complete" \
                "✅ Successfully installed: $target_toolchain with all components" \
                2>/dev/null &
        end
        echo "✅ Toolchain $target_toolchain was installed/repaired"
        return 2
    else
        # Send failure notification
        if command -v notify-send >/dev/null 2>&1
            notify-send --urgency=critical \
                "Toolchain Installation Failed" \
                "❌ Failed to install $target_toolchain" \
                2>/dev/null &
        end
        echo "❌ Toolchain installation failed" >&2
        return 1
    end
end

# ============================================================================
# ICE and Stale Cache Detection Functions
# ============================================================================
# This section detects compiler corruption requiring target/ cleanup.
#
# Two detection strategies:
# 1. ICE detection: Look for rustc-ice-*.txt files, exit code 101, or
#    panic messages like "internal compiler error", "thread 'rustc' panicked"
# 2. Stale cache detection: Look for parser errors with single punctuation
#    tokens (e.g., "expected item, found `/`") combined with "could not compile"
#
# Why stale cache causes these errors:
# - Incremental compilation caches parsed AST and type info
# - Corruption (e.g., interrupted build, disk issues) leaves invalid data
# - Rustc reads corrupted cache, sees garbage bytes as "tokens"
# - Results in impossible syntax errors that don't exist in source code
#
# Recovery is handled by cleanup_after_ice() which removes target/ entirely.
# ============================================================================

# Helper function to check for ICE or stale cache errors from a file
# This avoids string variable corruption issues while still detecting issues
# Usage: detect_ice_from_file EXIT_CODE TEMP_FILE_PATH
#
# Detects two categories of compiler corruption:
# 1. ICE (Internal Compiler Error) - rustc panics/crashes
# 2. Stale cache errors - corrupted incremental compilation artifacts
#
# IMPORTANT: This function must avoid false positives from words containing "ice"
# as a substring (like "device", "choice", "slice", "service", etc.)
function detect_ice_from_file
    set -l exit_code $argv[1]
    set -l temp_file $argv[2]

    # Most reliable check: look for actual ICE dump files on disk
    # These are only created by rustc when an actual ICE occurs
    if test (count (find . -maxdepth 1 -name "rustc-ice-*.txt" 2>/dev/null)) -gt 0
        return 0
    end

    # Secondary check: look for exit code 101 (rustc error indicator) with ICE patterns in file
    if test $exit_code -eq 101
        # Check for actual ICE patterns in file content
        # Be very specific to avoid false positives from words like "device", "choice", "slice"
        if grep -qi "internal compiler error" $temp_file 2>/dev/null
            or grep -qi "thread 'rustc' panicked" $temp_file 2>/dev/null
            or grep -qi "rustc ICE" $temp_file 2>/dev/null
            or grep -qi "panicked at 'mir_" $temp_file 2>/dev/null
            return 0
        end
    end

    # Stale cache detection: corrupted incremental compilation artifacts
    # These manifest as parser errors for non-existent syntax issues
    # Pattern: "expected X, found Y" where Y is a single punctuation character
    # Example: "expected item, found `/`" (the `/` came from corrupted cache, not source)
    #
    # Note: Fish shell treats backticks specially, so we use printf to create
    # a pattern file with literal backticks for grep to match against.
    set -l pattern_file (mktemp)
    printf 'expected.*found \x60[^a-zA-Z0-9]\x60' >$pattern_file
    if grep -qEf $pattern_file $temp_file 2>/dev/null
        rm -f $pattern_file
        # Verify it's likely cache corruption by checking for "could not compile"
        # This distinguishes from legitimate syntax errors in user code
        if grep -qi "could not compile" $temp_file 2>/dev/null
            return 0
        end
    else
        rm -f $pattern_file
    end

    return 1
end

# Deprecated: detect_ice - use detect_ice_from_file instead
# Kept for backward compatibility
function detect_ice
    set -l exit_code $argv[1]
    set -l output $argv[2]

    # Most reliable check: look for actual ICE dump files on disk
    # These are only created by rustc when an actual ICE occurs
    if test (count (find . -maxdepth 1 -name "rustc-ice-*.txt" 2>/dev/null)) -gt 0
        return 0
    end

    # Secondary check: look for exit code 101 (rustc error indicator) with ICE patterns in output
    if test $exit_code -eq 101
        # Check for actual ICE patterns in output
        # Be very specific to avoid false positives from words like "device", "choice", "slice"
        if string match -qi "*internal compiler error*" -- $output
            or string match -qi "*thread 'rustc' panicked*" -- $output
            or string match -qi "*rustc ICE*" -- $output
            or string match -qi "*panicked at 'mir_*" -- $output
            return 0
        end
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
        rm -rf "$CHECK_TARGET_DIR"
    end

    # Also clean staging directories to ensure fresh doc builds
    if test -d "$CHECK_TARGET_DIR_DOC_STAGING_QUICK"
        rm -rf "$CHECK_TARGET_DIR_DOC_STAGING_QUICK"
    end
    if test -d "$CHECK_TARGET_DIR_DOC_STAGING_FULL"
        rm -rf "$CHECK_TARGET_DIR_DOC_STAGING_FULL"
    end
end

# Helper function to run cleanup after ICE or stale cache corruption
function cleanup_after_ice
    echo "🧊 Compiler corruption detected (ICE or stale cache)! Running cleanup..."

    # Remove ICE dump files
    set -l ice_files (find . -name "rustc-ice-*.txt" 2>/dev/null)
    if test (count $ice_files) -gt 0
        echo "🗑️  Removing ICE dump files..."
        rm -f rustc-ice-*.txt
    end

    # Remove all target folders (build artifacts and caches can become corrupted)
    cleanup_target_folder

    echo "✨ Cleanup complete. Retrying checks..."
    echo ""
end

# Deprecated: run_checks
# REFACTORED into composable architecture
# Use run_checks_with_ice_recovery instead
# Kept for backward compatibility only
function run_checks
    echo "⚠️  run_checks is deprecated. Use run_checks_with_ice_recovery" >&2
    run_checks_with_ice_recovery
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
            # Check if config files changed (cleans target if needed)
            check_config_changed $CHECK_TARGET_DIR $CONFIG_FILES_TO_WATCH

            # Validate toolchain first
            # No lock needed - validation is read-only, installation delegates to sync script
            ensure_toolchain_installed
            set -l toolchain_status $status
            if test $toolchain_status -eq 1
                echo ""
                echo "❌ Cannot proceed without correct toolchain"
                return 1
            end

            # toolchain_status can be 0 (OK) or 2 (was reinstalled, already printed message)
            echo ""
            echo "🚀 Running checks..."

            # Use new composable architecture with automatic ICE recovery
            run_checks_with_ice_recovery
            set -l check_status $status

            # Send desktop notification for final result
            if test $check_status -eq 0
                send_system_notification "Build Checks Complete ✅" "All tests, doctests, and docs passed" "success" $NOTIFICATION_EXPIRE_MS
            else
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
