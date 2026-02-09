# Check Composition, Result Aggregation & Recovery
#
# Layered architecture for running cargo checks with automatic error recovery:
#
# Level 2: run_check_with_recovery — Generic wrapper for any check function.
#   Handles output formatting, timing, ICE detection, and stale artifact detection.
#
# Level 3: Orchestrator functions — Compose multiple Level 2 wrappers.
#   Aggregate results: Recoverable (2) > Failure (1) > Success (0).
#   - run_watch_checks: Full docs (for watch "full" mode)
#   - run_oneoff_checks: Quick docs to CHECK_TARGET_DIR (for one-off mode)
#   - run_full_checks: All checks including check, build, clippy (for --full mode)
#
# Level 4: Recovery functions — Wrap orchestrators with retry logic.
#   - run_oneoff_checks_with_recovery: Retry once on recoverable error
#   - run_watch_checks_with_recovery: Retry once on recoverable error
#   - run_full_checks_with_recovery: Retry + toolchain escalation on persistent ICE

# ============================================================================
# Level 2: Recovery-Aware Wrapper Function
# ============================================================================
# Generic wrapper that handles output, formatting, error detection, and timing
# Can wrap ANY check function and apply consistent error handling
#
# Strategy: Uses temp file to preserve ANSI codes and terminal formatting
# while still allowing output to be parsed for ICE and stale artifact detection
#
# Parameters:
#   $argv[1]: Function name to wrap (e.g., "check_nextest")
#   $argv[2]: Display label (e.g., "nextest")
#
# Returns:
#   0 = Success
#   1 = Failure (not ICE)
#   2 = Recoverable error detected (ICE or stale build artifacts)
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

    # Create temp file for error detection (output is suppressed)
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

    # Check for stale build artifacts (e.g., serde_core's private.rs lost from /tmp)
    if detect_stale_build_artifacts $temp_output
        set_color yellow
        echo "🔧 Stale build artifacts detected — will clean cache and retry ($duration_str)"
        set_color normal

        # Log stale artifacts to file with details
        if set -q CHECK_LOG_FILE; and test -n "$CHECK_LOG_FILE"
            echo "["(timestamp)"] 🔧 Stale build artifacts detected during $check_name ($duration_str)" >> $CHECK_LOG_FILE
        end

        command rm -f $temp_output
        return 2
    end

    # Regular failure (not ICE, not stale artifacts) - show the error output to user
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
# Aggregates results: Recoverable (2) > Failure (1) > Success (0)
#
# Two variants:
#   - run_watch_checks: Full docs (for watch "full" mode)
#   - run_oneoff_checks: Quick docs to CHECK_TARGET_DIR (for one-off mode)
#
# Returns:
#   0 = All checks passed
#   1 = At least one check failed (not ICE)
#   2 = At least one check had a recoverable error (caller handles retry)

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

    # Aggregate: return 2 if ANY recoverable error, then 1 if ANY failure, else 0
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

    # Aggregate: return 2 if ANY recoverable error, then 1 if ANY failure, else 0
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
# Handles recovery from ICE and stale build artifacts with automatic cleanup and retry.
#
# Two variants:
#   - run_oneoff_checks_with_recovery: For one-off normal mode (uses quick docs)
#   - run_watch_checks_with_recovery: For watch "full" mode (uses full docs)
#
# Returns:
#   0 = All checks passed
#   1 = Checks failed

# Recovery function for one-off normal mode.
# Uses run_oneoff_checks (quick docs to CHECK_TARGET_DIR).
function run_oneoff_checks_with_recovery
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

        # Recoverable error - cleanup and retry
        cleanup_for_recovery $CHECK_TARGET_DIR
        set retry_count (math $retry_count + 1)
    end

    echo ""
    set_color red --bold
    echo "["(timestamp)"] ❌ Failed even after cache recovery"
    set_color normal
    return 1
end

# Recovery function for watch "full" mode (--watch).
# Uses run_watch_checks (full docs with dependencies).
function run_watch_checks_with_recovery
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

        # Recoverable error - cleanup and retry
        cleanup_for_recovery $CHECK_TARGET_DIR $CHECK_TARGET_DIR_DOC_STAGING_FULL
        set retry_count (math $retry_count + 1)
    end

    echo ""
    set_color red --bold
    echo "["(timestamp)"] ❌ Failed even after cache recovery"
    set_color normal
    return 1
end

# ============================================================================
# Level 3b: Full Orchestrator Function (includes clippy)
# ============================================================================
# Composes all checks including check, build, and clippy
# Aggregates results: Recoverable (2) > Failure (1) > Success (0)
#
# Returns:
#   0 = All checks passed
#   1 = At least one check failed (not ICE)
#   2 = At least one check had a recoverable error
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

    # Aggregate: return 2 if ANY recoverable error, then 1 if ANY failure, else 0
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
# Handles recovery (ICE, stale artifacts) with automatic cleanup, retry, and toolchain escalation.
#
# Escalation flow:
#   1. Recoverable error detected → cleanup target/ → retry
#   2. Still failing? → escalate to rust-toolchain-update.fish (finds working nightly)
#   3. Retry once more with new toolchain
#
# Returns:
#   0 = All checks passed
#   1 = Checks failed
function run_full_checks_with_recovery
    # First attempt
    run_full_checks
    set -l result $status

    # If not ICE, we're done
    if test $result -ne 2
        return $result
    end

    # Recoverable error - cleanup and retry
    echo ""
    set_color yellow
    echo "🔄 Recoverable error detected, cleaning target/ and retrying..."
    set_color normal
    cleanup_for_recovery $CHECK_TARGET_DIR $CHECK_TARGET_DIR_DOC_STAGING_FULL

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
