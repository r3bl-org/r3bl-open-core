#!/usr/bin/env fish

# Comprehensive Build and Test Verification Script
#
# Purpose: Runs a comprehensive suite of checks to ensure code quality, correctness, and builds properly.
#          This includes toolchain validation, tests, doctests, and documentation building.
#
# Workflow:
# 1. Validates Rust toolchain installation with all required components
# 2. Automatically installs/repairs toolchain if issues detected
# 3. Runs tests using cargo-nextest (faster than cargo test)
# 4. Runs documentation tests
# 5. Builds documentation
# 6. Detects and recovers from Internal Compiler Errors (ICE)
#
# Toolchain Management:
# - Automatically validates toolchain before running checks
# - Calls rust-toolchain-install-validate.fish to verify installation
# - If invalid, calls rust-toolchain-sync-to-toml.fish to reinstall
# - Sends desktop notifications (notify-send) on success/failure
#
# ICE Detection and Recovery:
# - Monitors for Internal Compiler Error indicators (exit code 101 or "ICE" in output)
# - On ICE detection: cleans all caches, removes ICE dump files, and retries once
# - Distinguishes between toolchain issues (ICE) vs code issues (compilation/test failures)
#
# Incremental Compilation Management:
# - Incremental compilation is disabled globally in .cargo/config.toml
# - This script also explicitly sets CARGO_INCREMENTAL=0 as a redundant safeguard
# - Rationale: Nightly rustc has occasional dep graph bugs in incremental mode
# - Disabling globally prevents ICE across all cargo invocations
# - If ICE occurs anyway (shouldn't), cleanup_after_ice removes corrupted artifacts
#
# Desktop Notifications:
# - Success: "Toolchain Installation Complete" (normal urgency)
# - Failure: "Toolchain Installation Failed" (critical urgency)
# - Only triggered when toolchain is actually installed/repaired
#
# Concurrency Safety:
# - check.fish itself doesn't use locks (it only reads toolchain state and runs cargo)
# - Toolchain installation is delegated to rust-toolchain-sync-to-toml.fish which has its own lock
# - Multiple check.fish instances can run simultaneously without conflict
# - If toolchain installation is needed, sync script prevents concurrent modifications
#
# Watch Mode & inotify Behavior:
# Watch mode uses a sequential processing model with kernel-buffered events:
#
# 1. Script blocks waiting for file changes (inotifywait)
# 2. When a change is detected, debounce timer is checked
# 3. If debounce passes, full check suite runs (30+ seconds, NOT listening for new changes)
# 4. While checks run, the Linux kernel buffers any new file change events
# 5. When checks complete, inotifywait is called again and immediately returns buffered events
# 6. Debounce check determines if another run happens immediately or is skipped
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
# Configuration Constants
# ============================================================================

# Debounce delay in seconds for watch mode
# Prevents rapid re-runs when multiple files are saved in quick succession
# If a file change occurs within this window after the last check started,
# it will be ignored. Increase this if you find checks running too frequently.
set -g DEBOUNCE_SECONDS 5

# Make sure that cargo has it's own folder to work on, so it does not wait for
# lock on Claude Code, VSCode, or RustRoever (each of whom have their own
# subfolder).
set -gx CARGO_TARGET_DIR target/check

# Explicitly disable incremental compilation for check.fish (redundant safeguard).
# Incremental is already disabled globally in .cargo/config.toml, but we set it here
# for explicit clarity. The nightly compiler has a bug where the dep graph gets
# corrupted in incremental mode, causing "mir_drops_elaborated_and_const_checked" panics.
set -gx CARGO_INCREMENTAL 0

# ============================================================================
# Argument Parsing
# ============================================================================

# Parse command line arguments and return the mode
# Returns: "help", "watch", "watch-test", "watch-doc", or "normal"
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
    echo "  ./check.fish              Run checks once (default)"
    echo "  ./check.fish --watch      Watch source files and run all checks on changes"
    echo "  ./check.fish --watch-test Watch source files and run tests/doctests only"
    echo "  ./check.fish --watch-doc  Watch source files and run doc build only"
    echo "  ./check.fish --help       Show this help message"
    echo ""

    set_color yellow
    echo "FEATURES:"
    set_color normal
    echo "  ✓ Automatic toolchain validation and repair"
    echo "  ✓ Fast tests using cargo-nextest"
    echo "  ✓ Documentation tests (doctests)"
    echo "  ✓ Documentation building"
    echo "  ✓ Internal Compiler Error (ICE) detection and recovery"
    echo "  ✓ Desktop notifications on toolchain changes"
    echo ""

    set_color yellow
    echo "WATCH MODES:"
    set_color normal
    echo "  --watch       Runs all checks: nextest, doctests, docs"
    echo "  --watch-test  Runs tests only: nextest + doctests (faster, for test iteration)"
    echo "  --watch-doc   Runs doc build only (for doc iteration)"
    echo ""
    echo "  Common options for all watch modes:"
    echo "  Monitors: cmdr/src/, analytics_schema/src/, tui/src/"
    echo "  Debouncing: $DEBOUNCE_SECONDS seconds (prevents rapid re-runs)"
    echo "  Toolchain: Validated once at startup, before watch loop begins"
    echo "  Behavior: Continues watching even if checks fail"
    echo "  Requirements: inotifywait (installed via bootstrap.sh)"
    echo ""
    echo "  Event Handling:"
    echo "  • While checks run (30+ sec), new file changes are buffered by the kernel"
    echo "  • When checks complete, buffered events trigger immediately (if debounce allows)"
    echo "  • Multiple saves during test runs may cause cascading re-runs"
    echo "  • Increase DEBOUNCE_SECONDS in script if this becomes disruptive"
    echo ""

    set_color yellow
    echo "WORKFLOW:"
    set_color normal
    echo "  1. Validates Rust toolchain (nightly + components)"
    echo "  2. Auto-installs/repairs if needed"
    echo "  3. Runs nextest (faster than cargo test)"
    echo "  4. Runs doctests"
    echo "  5. Builds documentation"
    echo "  6. Detects and recovers from ICE"
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
    echo "👀 Watching for changes..."
    set_color normal
    echo ""

    # Watch loop with inotifywait
    while true
        # Wait for file changes
        set -l changed_file (inotifywait -q -r -e modify,create,delete,move \
            --format '%w%f' $watch_dirs 2>/dev/null)

        # Get current time
        set -l current_time (date +%s)

        # Check debounce
        set -l time_diff (math $current_time - $last_run)
        if test $time_diff -lt $DEBOUNCE_SECONDS
            continue
        end

        # Update last run time
        set last_run $current_time

        # Run checks
        echo ""
        set_color yellow
        echo "🔄 Changes detected, running checks..."
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
        echo "👀 Watching for changes..."
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
                echo "✅ All checks passed!"
                set_color normal
            end
            return $result

        case "test"
            # Test checks: nextest + doctests only
            run_check_with_recovery check_nextest "nextest"
            if test $status -eq 2
                return 2  # ICE detected
            end
            if test $status -ne 0
                return 1
            end

            run_check_with_recovery check_doctests "doctests"
            if test $status -eq 2
                return 2  # ICE detected
            end
            if test $status -ne 0
                return 1
            end

            echo ""
            set_color green --bold
            echo "✅ All test checks passed!"
            set_color normal
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
                echo "✅ Doc checks passed!"
                set_color normal
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

function check_nextest
    cargo nextest run --all-targets
end

function check_doctests
    cargo test --doc
end

function check_docs
    cargo doc --no-deps
end

# ============================================================================
# Level 2: ICE-Aware Wrapper Function
# ============================================================================
# Generic wrapper that handles output, formatting, and ICE detection
# Can wrap ANY check function and apply consistent error handling
#
# Parameters:
#   $argv[1]: Function name to wrap (e.g., "check_nextest")
#   $argv[2]: Display label (e.g., "nextest")
#
# Returns:
#   0 = Success
#   1 = Failure (not ICE)
#   2 = ICE detected
function run_check_with_recovery
    set -l check_func $argv[1]
    set -l check_name $argv[2]

    echo ""
    set_color cyan
    echo "▶️  Running $check_name..."
    set_color normal

    # Run and capture output for ICE detection
    set -l output ($check_func 2>&1)
    set -l exit_code $status

    if test $exit_code -eq 0
        set_color green
        echo "✅ $check_name passed"
        set_color normal
        return 0
    end

    # Check for ICE - the KEY centralized detection point
    if detect_ice $exit_code $output
        set_color red
        echo "🧊 ICE detected"
        set_color normal
        return 2
    end

    # Regular failure (not ICE)
    set_color red
    echo "❌ $check_name failed"
    set_color normal
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
    set -l result_nextest 0
    set -l result_doctest 0
    set -l result_docs 0

    run_check_with_recovery check_nextest "nextest"
    set result_nextest $status

    run_check_with_recovery check_doctests "doctests"
    set result_doctest $status

    run_check_with_recovery check_docs "docs"
    set result_docs $status

    # Aggregate: return 2 if ANY ICE, then 1 if ANY failure, else 0
    if test $result_nextest -eq 2 || test $result_doctest -eq 2 || test $result_docs -eq 2
        return 2
    end

    if test $result_nextest -ne 0 || test $result_doctest -ne 0 || test $result_docs -ne 0
        return 1
    end

    return 0
end

# ============================================================================
# Level 4: Top-Level Recovery Function
# ============================================================================
# Handles ICE recovery with automatic cleanup and retry
# Single entry point for both normal and watch modes
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
                echo "✅ All checks passed!"
                set_color normal
            else
                set_color red --bold
                echo "❌ Checks failed"
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
    echo "❌ Failed even after ICE recovery"
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
# ICE Detection and Recovery Functions
# ============================================================================

# Helper function to check for ICE in output or exit code
# Usage: detect_ice EXIT_CODE OUTPUT
function detect_ice
    set -l exit_code $argv[1]
    set -l output $argv[2]

    # Check for exit code 101 (rustc ICE indicator)
    if test $exit_code -eq 101
        return 0
    end

    # Check for ICE text in output
    if string match -qi "*internal compiler error*" -- $output
        or string match -qi "*ICE*" -- $output
        return 0
    end
    return 1
end

# Helper function to extract failed test count from nextest output
function parse_nextest_failures
    set -l output $argv[1]
    # Extract the number before "failed" in nextest summary
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

# Helper function to run cleanup after ICE
function cleanup_after_ice
    echo "🧊 Internal Compiler Error detected! Running cleanup..."

    # Remove ICE dump files
    set -l ice_files (find . -name "rustc-ice-*.txt" 2>/dev/null)
    if test (count $ice_files) -gt 0
        echo "🗑️  Removing ICE dump files..."
        rm -f rustc-ice-*.txt
    end

    # Remove all target folders (build artifacts and caches can become corrupted)
    cleanup_target_folder

    # Clean cargo caches
    cargo cache -r all

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
        case normal
            # Normal mode: run checks once
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
            return $status
    end
end

# ============================================================================
# Script Execution
# ============================================================================

main $argv
exit $status
