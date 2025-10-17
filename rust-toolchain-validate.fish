#!/usr/bin/env fish

# Consolidated Rust Toolchain Validation Script
#
# Purpose: Provides two validation modes for Rust toolchain verification
#          - quick: Fast component check (1-2 seconds)
#          - complete: Full build+test validation with ICE detection (5-10 minutes)
#
# Concurrency Safety:
# - This script is read-only (doesn't modify toolchain) so no lock is needed
# - Multiple validation instances can run simultaneously without conflict
# - Safe to run alongside toolchain modification scripts (update, sync)
#
# Usage:
#   ./rust-toolchain-validate.fish quick     # Fast validation
#   ./rust-toolchain-validate.fish complete  # Comprehensive validation
#
# Or via run.fish:
#   fish run.fish toolchain-validate              # Quick mode
#   fish run.fish toolchain-validate-complete     # Complete mode

# Import shared utilities from script_lib.fish
source script_lib.fish

# ============================================================================
# Configuration
# ============================================================================

set -g PROJECT_DIR (pwd)
set -g TOOLCHAIN_FILE $PROJECT_DIR/rust-toolchain.toml

# ============================================================================
# Quick Validation (Mode: quick)
# ============================================================================

# Validates basic toolchain installation and components
# Time: ~1-2 seconds
# Returns: 0 if valid, 1 if not installed, 2 if components missing, 3 if corrupted, 4 if can't read TOML
function validate_quick
    echo "🔍 Validating Rust toolchain installation (quick mode)..."
    echo ""

    # Step 1: Read toolchain from TOML
    echo "📖 Reading rust-toolchain.toml..."
    set -l toolchain (read_toolchain_from_toml)
    if test $status -ne 0
        echo "❌ Failed to read rust-toolchain.toml"
        return 4
    end
    echo "   Target toolchain: $toolchain"
    echo ""

    # Step 2: Check if installed
    echo "🔍 Checking if toolchain is installed..."
    if not is_toolchain_installed $toolchain
        echo "❌ Toolchain $toolchain is NOT installed"
        return 1
    end
    echo "   ✅ Toolchain installed"
    echo ""

    # Step 3: Check rust-analyzer
    echo "🔍 Checking rust-analyzer component..."
    if not is_component_installed $toolchain "rust-analyzer"
        echo "❌ rust-analyzer component is MISSING"
        return 2
    end
    echo "   ✅ rust-analyzer installed"
    echo ""

    # Step 4: Check rust-src
    echo "🔍 Checking rust-src component..."
    if not is_component_installed $toolchain "rust-src"
        echo "❌ rust-src component is MISSING"
        return 2
    end
    echo "   ✅ rust-src installed"
    echo ""

    # Step 5: Verify not corrupted
    echo "🔍 Verifying toolchain integrity..."
    if not rustup run $toolchain rustc --version >/dev/null 2>&1
        echo "❌ Toolchain is CORRUPTED (rustc fails)"
        return 3
    end
    set -l rustc_version (rustup run $toolchain rustc --version)
    echo "   ✅ Toolchain operational: $rustc_version"
    echo ""

    echo "✅ Quick validation passed!"
    return 0
end

# ============================================================================
# Comprehensive Validation (Mode: complete)
# ============================================================================

# Validates toolchain by running actual build+test suite
# Detects ICE (Internal Compiler Errors) indicating toolchain instability
# Time: ~5-10 minutes (full build + tests)
# Returns: 0 if valid and stable, 1 if ICE detected or build failed, 4 if can't read TOML
function validate_complete
    echo "🔍 Validating Rust toolchain (comprehensive mode - this may take several minutes)..."
    echo ""

    # Step 1: Read toolchain from TOML
    echo "📖 Reading rust-toolchain.toml..."
    set -l toolchain (read_toolchain_from_toml)
    if test $status -ne 0
        echo "❌ Failed to read rust-toolchain.toml"
        return 4
    end
    echo "   Target toolchain: $toolchain"
    echo ""

    # Step 2: Check basic prerequisites
    echo "🔍 Checking if toolchain is installed..."
    if not is_toolchain_installed $toolchain
        echo "❌ Toolchain $toolchain is NOT installed"
        return 1
    end
    echo "   ✅ Toolchain installed"
    echo ""

    # Step 3: Run comprehensive validation tests
    echo "🔍 Running comprehensive validation suite..."
    echo "   This includes: clippy, build, tests, doctests, and docs"
    echo ""

    set -l temp_output /tmp/rust-toolchain-validation-(date +%s).log
    set -l validation_steps \
        "clippy:cargo clippy --all-targets" \
        "build:cargo build" \
        "nextest:cargo nextest run" \
        "doctest:cargo test --doc" \
        "doc:cargo doc --no-deps"

    for step in $validation_steps
        set -l step_name (string split ":" $step)[1]
        set -l step_cmd (string split ":" $step)[2]

        echo ""
        echo "Running validation step: $step_name"

        # Run command and capture output
        eval $step_cmd > $temp_output 2>&1
        set -l exit_code $status

        # Check for ICE patterns (Internal Compiler Error indicators)
        if grep -Ei "internal compiler error|thread 'rustc' panicked|error: the compiler unexpectedly panicked|this is a bug in the rust compiler" $temp_output > /dev/null
            echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            echo "❌ ICE DETECTED in $step_name"
            echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            echo "Toolchain $toolchain is UNSTABLE (Internal Compiler Error)"
            echo ""
            echo "Last 20 lines of output:"
            tail -n 20 $temp_output
            rm -f $temp_output
            return 1
        end

        # Log exit code
        if test $exit_code -ne 0
            echo "  ⚠️  Command exited with code $exit_code"
            echo "  ✅ No ICE detected (compilation/test failures are OK)"
        else
            echo "  ✅ Passed"
        end
    end

    rm -f $temp_output
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "✅ Comprehensive validation passed!"
    echo "   Toolchain $toolchain is stable (no ICE detected)"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    return 0
end

# ============================================================================
# Help Documentation
# ============================================================================

function print_help
    echo ""
    echo (set_color cyan --bold)"Rust Toolchain Validation Script"(set_color normal)
    echo ""
    echo (set_color yellow)"USAGE:"(set_color normal)
    echo "  ./rust-toolchain-validate.fish (set_color green)quick(set_color normal)     # Fast validation"
    echo "  ./rust-toolchain-validate.fish (set_color green)complete(set_color normal)  # Comprehensive validation"
    echo ""
    echo (set_color yellow)"MODES:"(set_color normal)
    echo ""
    echo "  "(set_color green)"quick"(set_color normal)"     (~1-2 seconds)"
    echo "    • Toolchain installed, rust-analyzer, rust-src, rustc functional"
    echo ""
    echo "  "(set_color green)"complete"(set_color normal)" (~5-10 minutes)"
    echo "    • clippy, build, nextest, doctests, docs (detects ICE)"
    echo ""
    echo (set_color yellow)"EXIT CODES:"(set_color normal)
    echo "  0 = Success"
    echo "  1 = Not installed or ICE detected"
    echo "  2 = Missing components (quick only)"
    echo "  3 = Toolchain corrupted (quick only)"
    echo "  4 = Failed to read rust-toolchain.toml"
    echo ""
end

# ============================================================================
# Main Entry Point
# ============================================================================

function main
    set -l mode $argv[1]

    # No lock needed - validation is read-only and doesn't modify the toolchain

    switch $mode
        case quick
            validate_quick
            return $status

        case complete
            validate_complete
            return $status

        case ""
            print_help
            return 1

        case "*"
            echo (set_color red)"Unknown validation mode: $mode"(set_color normal)
            print_help
            return 1
    end
end

main $argv
exit $status
