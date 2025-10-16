#!/usr/bin/env fish

# ============================================================================
# Toolchain Validation Functions
# ============================================================================

# Helper function to read target toolchain from rust-toolchain.toml
function read_target_toolchain_from_toml
    set -l toolchain_file "./rust-toolchain.toml"

    if not test -f $toolchain_file
        echo "ERROR: rust-toolchain.toml not found" >&2
        return 1
    end

    # Extract the channel value from the TOML file
    set -l channel_line (grep '^channel = ' $toolchain_file)

    if test -z "$channel_line"
        echo "ERROR: No channel entry found in rust-toolchain.toml" >&2
        return 1
    end

    # Extract the value between quotes
    set -l toolchain (echo $channel_line | sed -n 's/.*channel = "\([^"]*\)".*/\1/p')

    if test -z "$toolchain"
        echo "ERROR: Failed to parse channel value" >&2
        return 1
    end

    echo $toolchain
    return 0
end

# Helper function to check if a toolchain is installed
function is_toolchain_installed
    set -l toolchain $argv[1]
    rustup toolchain list | grep -q "^$toolchain"
    return $status
end

# Helper function to ensure correct toolchain is installed (silent by default)
# Returns 0 if toolchain is OK, 1 if error, 2 if toolchain was reinstalled
function ensure_toolchain_installed
    set -l target_toolchain (read_target_toolchain_from_toml)
    if test $status -ne 0
        echo "❌ Failed to read toolchain from rust-toolchain.toml" >&2
        return 1
    end

    if is_toolchain_installed $target_toolchain
        # Toolchain already installed, return silently
        return 0
    else
        # Toolchain missing, reinstall it
        if fish ./rust-toolchain-sync-to-toml.fish
            echo "✅ Toolchain $target_toolchain was reinstalled"
            return 2
        else
            echo "❌ Toolchain installation failed" >&2
            return 1
        end
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

# Main execution with toolchain validation and retry logic
ensure_toolchain_installed
set -l toolchain_status $status
if test $toolchain_status -eq 1
    echo ""
    echo "❌ Cannot proceed without correct toolchain"
    exit 1
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
    exit $status
else
    exit $result
end
