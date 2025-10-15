#!/usr/bin/env fish

# Helper function to check for ICE in output
function detect_ice
    set -l output $argv[1]
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
    set -l warnings (echo "$output" | grep -c '^warning:')
    set -l errors (echo "$output" | grep -c '^error:')
    echo "$warnings warnings, $errors errors"
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
        if detect_ice $nextest_output
            return 2  # ICE detected
        end
        set -l failed_count (parse_nextest_failures $nextest_output)
        set -a failures "tests: $failed_count failed 😢"
    end

    # Run doctests
    set -l doctest_output (cargo test --doc 2>&1)
    set -l doctest_status $status
    if test $doctest_status -ne 0
        if detect_ice $doctest_output
            return 2  # ICE detected
        end
        set -l failed_count (parse_doctest_failures $doctest_output)
        set -a failures "doctests: $failed_count failed 😢"
    end

    # Run doc build
    set -l doc_output (cargo doc --no-deps 2>&1)
    set -l doc_status $status
    if test $doc_status -ne 0
        if detect_ice $doc_output
            return 2  # ICE detected
        end
        set -l warning_error_counts (parse_doc_warnings_errors $doc_output)
        set -a failures "build: $warning_error_counts 😢"
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

# Main execution with retry logic
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
