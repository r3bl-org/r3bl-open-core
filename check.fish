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
# Exit Codes:
# - 0: All checks passed ✅
# - 1: Checks failed or toolchain installation failed ❌
# - Specific check results shown in output
#
# Usage:
#   ./check.fish
#   fish ./check.fish

# Import shared toolchain utilities
source script_lib.fish

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

# Helper function to run cleanup after ICE
function cleanup_after_ice
    echo "🧊 Internal Compiler Error detected! Running cleanup..."

    # Remove ICE dump files
    set -l ice_files (find . -name "rustc-ice-*.txt" 2>/dev/null)
    if test (count $ice_files) -gt 0
        echo "🗑️  Removing ICE dump files..."
        rm -f rustc-ice-*.txt
    end

    # Clean cargo caches and build artifacts
    cargo cache -r all
    cargo clean
    sccache --stop-server 2>/dev/null

    echo "✨ Cleanup complete. Retrying checks..."
    echo ""
end

# Main check function
function run_checks
    set -l failures

    # Run nextest
    set -l nextest_output (cargo nextest run --all-targets 2>&1)
    set -l nextest_status $status
    if test $nextest_status -ne 0
        if detect_ice $nextest_status $nextest_output
            return 2  # ICE detected
        end
        set -l failed_count (parse_nextest_failures $nextest_output)
        set -a failures "tests: $failed_count failed 😢"
    end

    # Run doctests
    set -l doctest_output (cargo test --doc 2>&1)
    set -l doctest_status $status
    if test $doctest_status -ne 0
        if detect_ice $doctest_status $doctest_output
            return 2  # ICE detected
        end
        set -l failed_count (parse_doctest_failures $doctest_output)
        set -a failures "doctests: $failed_count failed 😢"
    end

    # Run doc build
    set -l doc_output (cargo doc --no-deps 2>&1)
    set -l doc_status $status
    if test $doc_status -ne 0
        if detect_ice $doc_status $doc_output
            return 2  # ICE detected
        end
        # Check for warnings/errors in failed build
        if parse_doc_warnings_errors $doc_output >/dev/null
            set -l warning_error_counts (parse_doc_warnings_errors $doc_output)
            set -a failures "build: $warning_error_counts 😢"
        else
            set -a failures "build: failed 😢"
        end
    else
        # Even on success, check for warnings
        if parse_doc_warnings_errors $doc_output >/dev/null
            set -l warning_error_counts (parse_doc_warnings_errors $doc_output)
            set -a failures "docs: $warning_error_counts ⚠️"
        end
    end

    # Return results
    if test (count $failures) -eq 0
        echo "✅ OK!"
        return 0
    else
        echo (string join ", " $failures)
        return 1
    end
end

# ============================================================================
# Main Entry Point
# ============================================================================

function main
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
    echo ""

    run_checks
    set -l result $status

    if test $result -eq 2
        # ICE detected, cleanup and retry once
        cleanup_after_ice
        run_checks
        set result $status
    end

    return $result
end

# ============================================================================
# Script Execution
# ============================================================================

main
exit $status
