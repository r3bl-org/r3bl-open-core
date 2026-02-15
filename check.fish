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
# 5. Detects and recovers from ICE and stale build artifacts (auto-cleans cache, retries)
# 6. On persistent ICE, escalates to rust-toolchain-update.fish to find stable nightly
#
# Config Change Detection:
# - Active in ALL modes: one-off (--test, --doc, default) and watch modes
# - Monitors: Cargo.toml (root + all workspace crates), rust-toolchain.toml, .cargo/config.toml
# - On hash mismatch: automatically cleans target/check to avoid stale artifact issues
# - In watch mode: config files are also added to inotifywait, so changes trigger the loop
#
# Exit Codes:
# - 0: All checks passed
# - 1: Checks failed or toolchain installation failed
#
# Usage:
#   ./check.fish              Run default checks (tests, doctests, docs)
#   ./check.fish --check      Run typecheck only (cargo check)
#   ./check.fish --build      Run build only (cargo build)
#   ./check.fish --clippy     Run clippy only (cargo clippy --all-targets)
#   ./check.fish --test       Run tests only (cargo test + doctests)
#   ./check.fish --doc        Build docs only (quick, --no-deps)
#   ./check.fish --full       Run ALL checks (check + build + clippy + tests + doctests + docs + windows)
#   ./check.fish --watch      Watch mode: run default checks on file changes
#   ./check.fish --watch-test Watch mode: run tests/doctests only
#   ./check.fish --watch-doc  Watch mode: quick docs first, full docs forked to background
#   ./check.fish --help       Show detailed help
#
# Modules (sourced in order):
#   check_constants.fish       Global configuration (paths, timeouts, parallelism)
#   check_lock.fish            Single instance enforcement (PID-based)
#   check_cli.fish             Argument parsing and help display
#   check_recovery.fish        Target cleanup, recovery helpers, logging
#   check_detection.fish       ICE and stale artifact detection
#   check_toolchain.fish       Toolchain validation, corruption detection, auto-repair
#   check_cargo.fish           Pure cargo command wrappers (check, build, clippy, test, docs)
#   check_docs.fish            Doc sync (staging → serving) and orphan detection
#   check_orchestrators.fish   Check composition, result aggregation, retry with recovery
#   check_watch.fish           Watch mode loop, sliding window debounce, check dispatch

# Import shared utilities (resolve relative to this script, not cwd)
source (dirname (status --current-filename))/script_lib.fish

# Import check.fish modules (order matters: constants first, then utilities, then consumers)
set -l __check_dir (dirname (status --current-filename))
source $__check_dir/check_constants.fish       # Globals — must be first (sets vars used by all others)
source $__check_dir/check_lock.fish            # Lock/PID — uses CHECK_LOCK_FILE
source $__check_dir/check_cli.fish             # parse_arguments, show_help — uses constants in help text
source $__check_dir/check_recovery.fish        # log_message, cleanup_*, dirs_for_check_type — uses constants
source $__check_dir/check_detection.fish       # ICE/stale detection — uses log_message from check_recovery
source $__check_dir/check_toolchain.fish       # ensure_toolchain_installed — uses script_lib functions
source $__check_dir/check_cargo.fish           # Pure check wrappers — uses constants + ionice_wrapper
source $__check_dir/check_docs.fish            # sync_docs_to_serving — uses constants
source $__check_dir/check_orchestrators.fish   # Composes checks — uses cargo, docs, recovery, detection
source $__check_dir/check_watch.fish           # Watch mode — uses lock, orchestrators, recovery, constants

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

    # Proactive cleanup: remove stale ICE dump files from interrupted previous runs.
    # Prevents false ICE detection when a non-ICE compile error occurs.
    cleanup_stale_ice_files

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
                # Recoverable error - cleanup and retry once
                cleanup_for_recovery $CHECK_TARGET_DIR
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
                # Recoverable error - cleanup and retry once
                cleanup_for_recovery $CHECK_TARGET_DIR
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
                # Recoverable error - cleanup and retry once
                cleanup_for_recovery $CHECK_TARGET_DIR
                run_check_with_recovery check_clippy "clippy"
                set clippy_status $status
            end

            return $clippy_status
        case full
            # Full mode: comprehensive pre-commit check
            # Runs: check + build + clippy + tests + doctests + docs + windows
            check_config_changed $CHECK_TARGET_DIR $CONFIG_FILES_TO_WATCH

            ensure_toolchain_installed
            set -l toolchain_status $status
            if test $toolchain_status -eq 1
                echo ""
                echo "❌ Cannot proceed without correct toolchain"
                return 1
            end

            echo ""
            echo "🚀 Running comprehensive checks (check + build + clippy + tests + doctests + docs + windows)..."

            # Run all checks with recovery (ICE, stale artifacts)
            run_full_checks_with_recovery
            set -l full_status $status

            # Sync docs from staging to serving directory (so docs are browseable)
            if test $full_status -eq 0
                sync_docs_to_serving full
            end

            # Send desktop notification for final result
            if test $full_status -eq 0
                echo ""
                set_color green --bold
                echo "["(timestamp)"] ✅ All comprehensive checks passed!"
                set_color normal
                send_system_notification "Full Checks Complete ✅" "check, build, clippy, tests, doctests, docs, windows all passed" "success" $NOTIFICATION_EXPIRE_MS
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
                # Recoverable error - cleanup and retry once
                cleanup_for_recovery $CHECK_TARGET_DIR_DOC_STAGING_QUICK
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
                # Recoverable error - cleanup and retry once
                cleanup_for_recovery $CHECK_TARGET_DIR
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
            run_oneoff_checks_with_recovery
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
