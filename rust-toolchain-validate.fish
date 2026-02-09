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

# Import shared utilities (resolve relative to this script, not cwd)
source (dirname (status --current-filename))/script_lib.fish

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
# Returns: 0 if valid, 1 if not installed, 2 if components missing, 3 if corrupted, 4 if can't read TOML, 5 if wrong profile
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

    # Step 3: Check rustup profile
    echo "🔍 Checking rustup profile..."
    set -l current_profile (rustup show profile 2>/dev/null)
    if test $status -ne 0
        echo "❌ Failed to get rustup profile"
        return 5
    end
    if not string match -q -r "^(default|complete)\$" $current_profile
        echo "❌ Unexpected rustup profile: $current_profile"
        echo "   Expected: 'default' or 'complete'"
        return 5
    end
    echo "   ✅ Rustup profile: $current_profile"
    echo ""

    # Step 4: Check rust-analyzer
    echo "🔍 Checking rust-analyzer component..."
    if not is_component_installed $toolchain "rust-analyzer"
        echo "❌ rust-analyzer component is MISSING"
        return 2
    end
    echo "   ✅ rust-analyzer installed"
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
# Returns: quick validation exit code if prerequisites fail, 1 if ICE detected, 0 if stable
function validate_complete
    echo "🔍 Validating Rust toolchain (comprehensive mode - this may take several minutes)..."
    echo ""

    # Step 1: Run quick validation first (prerequisites check)
    echo "═══════════════════════════════════════════════════════"
    echo "Phase 1: Quick Validation (Prerequisites)"
    echo "═══════════════════════════════════════════════════════"
    echo ""

    validate_quick
    set -l quick_status $status

    if test $quick_status -ne 0
        echo ""
        echo "❌ Quick validation failed with exit code $quick_status"
        echo "   Cannot proceed with comprehensive validation"
        return $quick_status
    end

    echo ""
    echo "✅ Prerequisites validated - proceeding with ICE detection"
    echo ""

    # Step 2: Run comprehensive validation tests (ICE detection)
    echo "═══════════════════════════════════════════════════════"
    echo "Phase 2: ICE Detection (Build & Test Suite)"
    echo "═══════════════════════════════════════════════════════"
    echo ""

    set -l temp_output /tmp/rust-toolchain-validation-(date +%s).log
    set -l validation_steps \
        "clippy:cargo clippy --all-targets" \
        "build-prod-code:cargo build" \
        "build-test-code:cargo test --no-run" \
        "tests:cargo test --all-targets" \
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
            command rm -f $temp_output
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

    command rm -f $temp_output
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
    echo "    • Toolchain installed, profile check, rust-analyzer, rustc functional"
    echo ""
    echo "  "(set_color green)"complete"(set_color normal)" (~5-10 minutes)"
    echo "    • Runs quick mode first, then ICE detection via:"
    echo "    • clippy, build-prod, build-test, tests, doctests, docs"
    echo ""
    echo (set_color yellow)"EXIT CODES:"(set_color normal)
    echo "  0 = Success"
    echo "  1 = Not installed or ICE detected"
    echo "  2 = Missing components (quick only)"
    echo "  3 = Toolchain corrupted (quick only)"
    echo "  4 = Failed to read rust-toolchain.toml"
    echo "  5 = Wrong rustup profile (quick only)"
    echo ""
end

# ============================================================================
# Main Entry Point
# ============================================================================

function main
    set -l mode $argv[1]

    # No lock needed - validation is read-only and doesn't modify the toolchain

    # Ensure build dependencies (clang, wild) are available
    if not ensure_build_dependencies
        echo "❌ Failed to install required build dependencies"
        return 1
    end

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
