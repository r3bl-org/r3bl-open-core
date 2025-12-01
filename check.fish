#!/usr/bin/env fish

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
# 6. Builds documentation
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
# Watch Mode & inotify Behavior:
# Watch mode uses a sequential processing model with kernel-buffered events:
#
# 1. Script waits for file changes (inotifywait with timeout for target check)
# 2. Every TARGET_CHECK_INTERVAL_SECS, checks if target/check directory exists
# 3. If target/check is missing, triggers a rebuild automatically
# 4. When a file change is detected, debounce timer is checked
# 5. If debounce passes, full check suite runs (30+ seconds, NOT listening for new changes)
# 6. While checks run, the Linux kernel buffers any new file change events
# 7. When checks complete, inotifywait is called again and immediately returns buffered events
# 8. Debounce check determines if another run happens immediately or is skipped
#
# Example Timeline:
#   00:00  Save file #1 → triggers check
#   00:00  ▶️ Tests start running (inotifywait NOT active)
#   00:15  Save file #2 → buffered by kernel
#   00:20  Save file #3 → buffered by kernel
#   00:35  ✅ Tests complete, return to inotifywait
#   00:35  inotifywait returns IMMEDIATELY with file #2 event
#   00:35  Debounce: 35s > 5s ✓ → triggers another check
#   01:10  ✅ Tests complete
#   01:10  inotifywait returns IMMEDIATELY with file #3 event
#   01:10  Debounce: 35s > 5s ✓ → triggers another check
#
# This ensures no changes are lost, but can cause cascading runs if multiple
# saves occur during long test executions. Adjust DEBOUNCE_SECONDS if needed.
#
# Exit Codes:
# - 0: All checks passed ✅
# - 1: Checks failed or toolchain installation failed ❌
# - Specific check results shown in output
#
# Usage:
#   ./check.fish
#   ./check.fish --watch
#   ./check.fish --help

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

# Debounce delay in seconds for watch mode
# Prevents rapid re-runs when multiple files are saved in quick succession
# If a file change occurs within this window after the last check started,
# it will be ignored. Increase this if you find checks running too frequently.
set -g DEBOUNCE_SECONDS 5

# Use a separate target/check directory to avoid lock contention with IDEs
# (VSCode, RustRover, Claude Code use their own target directories)
# This allows check.fish to run concurrently without waiting for IDE locks.
set -gx CARGO_TARGET_DIR target/check

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
    echo "  ./check.fish --doc        Build documentation only (once)"
    echo "  ./check.fish --watch      Watch source files and run all checks on changes"
    echo "  ./check.fish --watch-test Watch source files and run tests/doctests only"
    echo "  ./check.fish --watch-doc  Watch source files and run doc build only"
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
    echo ""

    set_color yellow
    echo "ONE-OFF MODES:"
    set_color normal
    echo "  (default)     Runs all checks once: tests, doctests, docs"
    echo "  --test        Runs tests only: cargo test + doctests (once)"
    echo "  --doc         Builds documentation only (once)"
    echo ""

    set_color yellow
    echo "WATCH MODES:"
    set_color normal
    echo "  --watch       Runs all checks: tests, doctests, docs"
    echo "  --watch-test  Runs tests only: cargo test + doctests (faster iteration)"
    echo "  --watch-doc   Runs doc build only (faster iteration)"
    echo ""
    echo "  Watch mode options:"
    echo "  Monitors: cmdr/src/, analytics_schema/src/, tui/src/, plus all config files"
    echo "  Debouncing: $DEBOUNCE_SECONDS seconds (prevents rapid re-runs)"
    echo "  Coalescing: Drains buffered events after each check (one run per batch)"
    echo "  Toolchain: Validated once at startup, before watch loop begins"
    echo "  Behavior: Continues watching even if checks fail"
    echo "  Requirements: inotifywait (installed via bootstrap.sh)"
    echo ""
    echo "  Target Directory Auto-Recovery (watch mode only):"
    echo "  • Monitors for missing target/check directory (every "$TARGET_CHECK_INTERVAL_SECS"s)"
    echo "  • Auto-triggers rebuild if target/check is deleted externally"
    echo "  • Recovers from: cargo clean, manual rm -rf, IDE cache clearing"
    echo ""
    echo "  Event Handling:"
    echo "  • While checks run (30+ sec), new file changes are buffered by the kernel"
    echo "  • When checks complete, buffered events trigger immediately (if debounce allows)"
    echo "  • Multiple saves during test runs may cause cascading re-runs"
    echo "  • Increase DEBOUNCE_SECONDS in script if this becomes disruptive"
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
    echo "  6. Builds documentation"
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

    echo ""
    set_color cyan --bold
    echo "👀 Watch mode activated"
    set_color normal
    echo "Monitoring: "(string join ", " $watch_dirs)
    echo "Press Ctrl+C to stop"
    echo ""

    # Check if config files changed (cleans target if needed)
    check_config_changed $CARGO_TARGET_DIR $CONFIG_FILES_TO_WATCH

    # Validate toolchain BEFORE entering watch loop
    echo "🔧 Validating toolchain..."
    ensure_toolchain_installed
    set -l toolchain_status $status
    if test $toolchain_status -eq 1
        echo ""
        echo "❌ Toolchain validation failed"
        return 1
    end

    # Track last run time for debouncing (epoch seconds)
    set -l last_run 0

    # Run initial check
    echo ""
    echo "🚀 Running initial checks..."
    echo ""
    run_checks_for_type $check_type
    set -l initial_result $status

    echo ""
    set_color cyan
    echo "["(timestamp)"] 👀 Watching for changes..."
    set_color normal
    echo ""

    # Watch loop with inotifywait (uses timeout to allow periodic target/check existence check)
    while true
        # Wait for file changes with timeout
        # Returns: 0 = event detected, 1 = error, 2 = timeout
        # Redirect stdout to /dev/null - we only need the exit status, not the filename
        inotifywait -q -r -t $TARGET_CHECK_INTERVAL_SECS -e modify,create,delete,move \
            --format '%w%f' $watch_dirs >/dev/null 2>&1
        set -l wait_status $status

        # Check if target/check directory is missing (regardless of event or timeout)
        # This handles external deletions (cargo clean, manual rm, other scripts)
        if not test -d "$CARGO_TARGET_DIR"
            echo ""
            set_color yellow
            echo "["(timestamp)"] 📁 $CARGO_TARGET_DIR missing, triggering rebuild..."
            set_color normal
            echo ""

            run_checks_for_type $check_type
            set -l result $status

            # Handle ICE detected (status 2)
            if test $result -eq 2
                cleanup_after_ice
            end

            set last_run (date +%s)

            echo ""
            set_color cyan
            echo "["(timestamp)"] 👀 Watching for changes..."
            set_color normal
            echo ""
            continue
        end

        # If timeout (status 2) with no missing target, just loop back
        if test $wait_status -eq 2
            continue
        end

        # Drain any additional buffered events (coalesce rapid saves)
        # Uses 100ms timeout to catch events that arrived during the check run
        while inotifywait -q -r -t 0.1 -e modify,create,delete,move \
                --format '%w%f' $watch_dirs >/dev/null 2>&1
            # Discard additional events - we only need to know "something changed"
        end

        # Get current time
        set -l current_time (date +%s)

        # Check debounce
        set -l time_diff (math $current_time - $last_run)
        if test $time_diff -lt $DEBOUNCE_SECONDS
            continue
        end

        # Update last run time
        set last_run $current_time

        # Check if config files changed (cleans target if needed)
        check_config_changed $CARGO_TARGET_DIR $CONFIG_FILES_TO_WATCH

        # Run checks
        echo ""
        set_color yellow
        echo "["(timestamp)"] 🔄 Changes detected, running checks..."
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
        echo "["(timestamp)"] 👀 Watching for changes..."
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
            # Doc checks only
            run_check_with_recovery check_docs "docs"
            set -l result $status

            if test $result -eq 2
                # ICE detected - let caller (watch loop) handle cleanup
                return 2
            end

            if test $result -eq 0
                echo ""
                set_color green --bold
                echo "["(timestamp)"] ✅ Doc checks passed!"
                set_color normal
                send_system_notification "Watch: Docs Built ✅" "Documentation built successfully" "success" $NOTIFICATION_EXPIRE_MS
            else
                # Notify on failure in watch mode
                send_system_notification "Watch: Doc Build Failed ❌" "cargo doc failed" "critical" $NOTIFICATION_EXPIRE_MS
            end
            return $result

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

function check_cargo_test
    cargo test --all-targets -q
end

function check_doctests
    cargo test --doc -q
end

function check_docs
    cargo doc --no-deps
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

    run_check_with_recovery check_docs "docs"
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

    if test -d target
        rm -rf target
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
            check_config_changed $CARGO_TARGET_DIR $CONFIG_FILES_TO_WATCH

            ensure_toolchain_installed
            set -l toolchain_status $status
            if test $toolchain_status -eq 1
                echo ""
                echo "❌ Cannot proceed without correct toolchain"
                return 1
            end

            echo ""
            echo "📚 Building documentation..."
            run_check_with_recovery check_docs "docs"
            set -l doc_status $status

            if test $doc_status -eq 2
                # ICE detected - cleanup and retry once
                cleanup_after_ice
                run_check_with_recovery check_docs "docs"
                set doc_status $status
            end

            # Read duration from file (written by run_check_with_recovery)
            set -l duration (cat $CHECK_DURATION_FILE)
            set -l duration_str (format_duration $duration)

            if test $doc_status -eq 0
                echo ""
                set_color green --bold
                echo "["(timestamp)"] ✅ Documentation built successfully! ($duration_str)"
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
            check_config_changed $CARGO_TARGET_DIR $CONFIG_FILES_TO_WATCH

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
            check_config_changed $CARGO_TARGET_DIR $CONFIG_FILES_TO_WATCH

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
