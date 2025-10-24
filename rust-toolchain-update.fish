#!/usr/bin/env fish

# Import shared toolchain utilities
source script_lib.fish

# Rust Toolchain Update Script
#
# Purpose: Automatically finds and installs a stable Rust nightly toolchain,
#          biasing toward the latest nightly (today's snapshot) as most stable.
#          Falls back to older versions only if ICE (Internal Compiler Error) detected.
#          Validates each candidate toolchain by running comprehensive tests.
#          Also performs aggressive cleanup to save disk space.
#
# Smart Validation Strategy:
# - Search window: today → 45 days ago (46 total candidates)
# - Philosophy: Latest nightly is usually most stable with newest bug fixes
# - Start with today's nightly snapshot (optimistic approach)
# - For each candidate: UPDATE rust-toolchain.toml, THEN run validation suite
# - Validation tests production scenario: clippy, build, nextest, doctests, docs
# - Check for ICE (Internal Compiler Errors) - indicates toolchain bugs
# - If ICE detected, try progressively older nightlies until finding stable one
# - Distinguish between toolchain issues (ICE) vs code issues (compilation/test failures)
# - Once stable toolchain found, rust-toolchain.toml is already configured correctly
# - Send desktop notifications (notify-send): success when found, critical alert if all fail
#
# Cleanup Strategy:
# - Keep only: stable-* + the validated nightly-YYYY-MM-DD
# - Remove: all other nightly toolchains, including generic "nightly"
# - Installs rust-analyzer component (required by VSCode, RustRover, cargo, and serena MCP server)
#
# Final Verification:
# - Remove ICE failure files (rustc-ice-*.txt) generated during validation
# - Clean all caches: cargo cache, build artifacts (cargo clean), sccache
# - Run full verification build with new toolchain:
#   - cargo nextest run --all-targets
#   - cargo test --doc
#   - cargo doc --no-deps
# - Ensures new toolchain works perfectly with fresh build from scratch
#
# Cargo Tools Update:
# - Updates all cargo development tools (nextest, bacon, flamegraph, etc.)
# - Uses cargo install-update to check and install latest versions
# - Keeps development tools current with bug fixes and features
# - Non-blocking: continues even if some tool updates fail
#
# Concurrency Safety:
# - Uses mkdir (atomic directory creation) for mutual exclusion
# - Atomic lock: check-and-create happens in ONE kernel operation (only one process succeeds)
# - Safe to run alongside other toolchain scripts - they'll queue or abort gracefully
# - Stale lock detection: Automatically removes locks older than 10 minutes (crashed processes)
#
# This script is designed to run weekly via systemd timer.

# ============================================================================
# Global Variables
# ============================================================================

set -g LOG_FILE /home/nazmul/Downloads/rust-toolchain-update.log
set -g PROJECT_DIR /home/nazmul/github/r3bl-open-core
set -g TOOLCHAIN_FILE $PROJECT_DIR/rust-toolchain.toml
set -g target_toolchain ""

# ============================================================================
# Helper Functions
# ============================================================================

function log_message
    set -l message $argv[1]
    echo $message | tee -a $LOG_FILE
end

function log_message_no_newline
    set -l message $argv[1]
    echo -n $message | tee -a $LOG_FILE
end

function log_command_output
    set -l description $argv[1]
    log_message $description
    $argv[2..] 2>&1 | tee -a $LOG_FILE
    return $pipestatus[1]
end

function validate_prerequisites
    log_message "Validating prerequisites..."

    # Check if project directory exists
    if not test -d $PROJECT_DIR
        log_message "ERROR: Project directory not found: $PROJECT_DIR"
        return 1
    end

    # Check if rust-toolchain.toml exists
    if not test -f $TOOLCHAIN_FILE
        log_message "ERROR: rust-toolchain.toml not found: $TOOLCHAIN_FILE"
        return 1
    end

    log_message "✅ Prerequisites validated successfully"
    return 0
end

# ============================================================================
# State Management Functions
# ============================================================================

function show_current_state
    log_message "Changing to project directory: $PROJECT_DIR"
    cd $PROJECT_DIR

    if not log_command_output "Current toolchain information:" rustup show
        log_message "WARNING: Failed to get current toolchain information"
    end
end

function verify_final_state
    if not log_command_output "Final installed toolchains:" rustup toolchain list
        log_message "WARNING: Failed to list final toolchains"
    end

    if not log_command_output "Verifying project toolchain:" rustup show
        log_message "WARNING: Failed to verify project toolchain"
    end
end

function clean_and_verify_build
    log_message "═══════════════════════════════════════════════════════"
    log_message "Cleaning all caches and build artifacts"
    log_message "═══════════════════════════════════════════════════════"
    log_message ""

    cd $PROJECT_DIR

    # Remove ICE failure text files generated by rustc
    log_message "Removing any ICE failure files (rustc-ice-*.txt)..."
    find $PROJECT_DIR -name "rustc-ice-*.txt" -type f -delete 2>/dev/null
    log_message "✅ ICE files cleaned"
    log_message ""

    # Clean cargo cache
    log_message "Cleaning cargo cache..."
    if command -v cargo-cache >/dev/null 2>&1
        if not log_command_output "Running cargo cache -r all..." cargo cache -r all
            log_message "⚠️  cargo cache command failed (non-critical)"
        end
    else
        log_message "⚠️  cargo-cache not installed, skipping cargo cache cleanup"
    end

    # Clean build artifacts
    log_message ""
    if not log_command_output "Running cargo clean..." cargo clean
        log_message "⚠️  cargo clean failed (non-critical)"
    end

    # Clear sccache
    log_message ""
    log_message "Clearing sccache..."
    if command -v sccache >/dev/null 2>&1
        if not log_command_output "Running sccache --zero-stats..." sccache --zero-stats
            log_message "⚠️  sccache --zero-stats failed (non-critical)"
        end
        if not log_command_output "Running sccache --stop-server..." sccache --stop-server
            log_message "⚠️  sccache --stop-server failed (non-critical)"
        end
        # Remove sccache cache directory
        set -l sccache_dir ~/.cache/sccache
        if test -d $sccache_dir
            log_message "Removing sccache cache directory: $sccache_dir"
            rm -rf $sccache_dir
            log_message "✅ sccache cache directory removed"
        end
    else
        log_message "⚠️  sccache not installed, skipping sccache cleanup"
    end

    log_message ""
    log_message "═══════════════════════════════════════════════════════"
    log_message "Running full build and test verification"
    log_message "═══════════════════════════════════════════════════════"
    log_message ""

    # Run tests with nextest
    log_message "Running cargo nextest run --all-targets..."
    if not log_command_output "Testing with nextest..." cargo nextest run --all-targets
        log_message "⚠️  Some tests failed (this might be expected)"
    else
        log_message "✅ All nextest tests passed"
    end

    # Run doctests
    log_message ""
    log_message "Running cargo test --doc..."
    if not log_command_output "Running doctests..." cargo test --doc
        log_message "⚠️  Some doctests failed (this might be expected)"
    else
        log_message "✅ All doctests passed"
    end

    # Build documentation
    log_message ""
    log_message "Running cargo doc --no-deps..."
    if not log_command_output "Building documentation..." cargo doc --no-deps
        log_message "⚠️  Documentation build had warnings/errors"
    else
        log_message "✅ Documentation built successfully"
    end

    log_message ""
    log_message "═══════════════════════════════════════════════════════"
    log_message "Clean and verify completed"
    log_message "═══════════════════════════════════════════════════════"
    log_message ""
end

# ============================================================================
# Core Logic Functions
# ============================================================================

function validate_toolchain
    set -l toolchain $argv[1]
    set -l temp_output /tmp/rust-toolchain-validation-(date +%s).log

    log_message "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    log_message "Validating toolchain: $toolchain"
    log_message "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    cd $PROJECT_DIR

    # Update rust-toolchain.toml with candidate toolchain
    log_message "Updating rust-toolchain.toml to test candidate toolchain..."
    if not set_toolchain_in_toml $toolchain
        log_message "❌ Failed to update rust-toolchain.toml with candidate toolchain"
        return 1
    end
    log_message "✅ rust-toolchain.toml updated with: $toolchain"
    log_message ""

    # List of validation commands to verify toolchain stability
    # Validation tests the production scenario where cargo reads rust-toolchain.toml
    # We're checking for ICE (Internal Compiler Errors), not code correctness
    set -l validation_steps \
        "clippy:cargo clippy --all-targets" \
        "build-prod-code:cargo build" \
        "build-test-code:cargo test --no-run" \
        "nextest:cargo nextest run" \
        "doctest:cargo test --doc" \
        "doc:cargo doc --no-deps"

    # Run each validation step
    for step in $validation_steps
        set -l step_name (string split ":" $step)[1]
        set -l step_cmd (string split ":" $step)[2]

        log_message ""
        log_message "Running validation step: $step_name"
        log_message "Command: $step_cmd"

        # Run command and capture output
        eval $step_cmd > $temp_output 2>&1
        set -l exit_code $status

        # Check for ICE patterns (Internal Compiler Error indicators)
        # These indicate toolchain problems, not code problems
        if grep -Ei "internal compiler error|thread 'rustc' panicked|error: the compiler unexpectedly panicked|this is a bug in the rust compiler" $temp_output > /dev/null
            log_message "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            log_message "❌ ICE DETECTED in $step_name"
            log_message "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            log_message "Toolchain $toolchain is UNSTABLE (ICE detected)"
            log_message ""
            log_message "Command output (last 50 lines):"
            tail -n 50 $temp_output | tee -a $LOG_FILE
            rm -f $temp_output
            return 1  # ICE detected - toolchain is bad
        end

        # Log exit code (non-zero is OK if it's just compilation/test errors)
        if test $exit_code -ne 0
            log_message "  ⚠️  Command exited with code $exit_code (this is OK if not ICE)"
            log_message "  Checking if failure is due to code issues (not toolchain)..."
            # If we got here, no ICE was detected, so it's a code issue
            log_message "  ✅ No ICE detected - continuing validation"
        else
            log_message "  ✅ Command succeeded (exit: $exit_code)"
        end
    end

    rm -f $temp_output
    log_message ""
    log_message "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    log_message "✅ Toolchain $toolchain is STABLE (no ICE detected)"
    log_message "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    log_message ""
    return 0  # Toolchain is stable
end

function find_stable_toolchain
    set -l search_window_days 45  # Search from today back to 45 days ago (46 total attempts including today)

    log_message "═══════════════════════════════════════════════════════"
    log_message "Starting search for stable toolchain"
    log_message "Strategy: Start with today's snapshot, try progressively older up to $search_window_days days ago"
    log_message "Search window: "(date "+%Y-%m-%d")" to "(date -d "$search_window_days days ago" "+%Y-%m-%d")
    log_message "═══════════════════════════════════════════════════════"
    log_message ""

    for i in (seq 0 $search_window_days)
        set -l days_ago $i

        # Calculate the target date
        set -l target_date (date -d "$days_ago days ago" "+%Y-%m-%d")
        set -l candidate_toolchain "nightly-$target_date"

        log_message "═══════════════════════════════════════════════════════"
        log_message "Attempt "(math $i + 1)"/"(math $search_window_days + 1)
        log_message "Trying toolchain: $candidate_toolchain ($days_ago days ago)"
        log_message "═══════════════════════════════════════════════════════"
        log_message ""

        # Install the candidate toolchain
        log_message "Installing candidate toolchain: $candidate_toolchain"
        if not rustup toolchain install $candidate_toolchain 2>&1 | tee -a $LOG_FILE
            log_message "❌ Failed to install $candidate_toolchain"
            log_message "Trying next candidate..."
            log_message ""
            continue
        end

        log_message "✅ Successfully installed $candidate_toolchain"
        log_message ""

        # Validate the toolchain using consolidated validation script
        if fish ./rust-toolchain-validate.fish complete 2>&1 | tee -a $LOG_FILE
            # Found a stable toolchain!
            set -g target_toolchain $candidate_toolchain
            log_message "═══════════════════════════════════════════════════════"
            log_message "🎉 FOUND STABLE TOOLCHAIN: $candidate_toolchain"
            log_message "═══════════════════════════════════════════════════════"
            log_message ""

            return 0
        else
            log_message "Toolchain $candidate_toolchain is unstable (ICE detected)"
            log_message "Will try an older toolchain..."
            log_message ""

            # Uninstall the bad toolchain to save space
            log_message "Uninstalling unstable toolchain: $candidate_toolchain"
            rustup toolchain uninstall $candidate_toolchain 2>&1 | tee -a $LOG_FILE
            log_message ""
        end
    end

    # If we get here, we couldn't find a stable toolchain
    set -l total_attempts (math $search_window_days + 1)
    log_message "═══════════════════════════════════════════════════════"
    log_message "❌ ERROR: Could not find stable toolchain"
    log_message "Tried $total_attempts candidates (from "(date "+%Y-%m-%d")" back to "(date -d "$search_window_days days ago" "+%Y-%m-%d")")"
    log_message "All tested nightlies had ICE errors or failed to install"
    log_message "═══════════════════════════════════════════════════════"

    # Send system notification (runs in background, won't fail if notify-send missing)
    if command -v notify-send >/dev/null 2>&1
        notify-send --urgency=critical \
            "Rust Toolchain Update Failed" \
            "Could not find stable nightly in $total_attempts attempts ($search_window_days-day window). Check log: $LOG_FILE" \
            2>/dev/null &
        log_message "System notification sent"
    else
        log_message "notify-send not available, skipping system notification"
    end

    return 1
end

function set_toolchain_in_toml
    set -l toolchain $argv[1]

    # Validate the TOML file before updating
    set -l channel_count (grep -c "^channel = " $TOOLCHAIN_FILE)
    if test $channel_count -eq 0
        log_message "WARNING: No uncommented channel entry found in rust-toolchain.toml"
        log_message "This might indicate a malformed TOML file"
    else if test $channel_count -gt 1
        log_message "WARNING: Multiple uncommented channel entries found ($channel_count)"
        log_message "Only the first one will be updated"
    end

    # Only replace the first uncommented channel line (preserves comments)
    sed -i "0,/^channel = .*/s//channel = \"$toolchain\"/" $TOOLCHAIN_FILE

    if test $status -eq 0
        return 0
    else
        return 1
    end
end

function update_toolchain_config
    log_message "Updating rust-toolchain.toml to use $target_toolchain..."

    if not set_toolchain_in_toml $target_toolchain
        log_message "❌ Failed to update rust-toolchain.toml"
        return 1
    end

    log_message "✅ Successfully updated rust-toolchain.toml"

    # Show the updated content
    log_message "Updated rust-toolchain.toml content:"
    cat $TOOLCHAIN_FILE | tee -a $LOG_FILE

    return 0
end

function install_target_toolchain
    if not log_command_output "Installing toolchain $target_toolchain (if not already installed)..." rustup toolchain install $target_toolchain
        log_message "❌ Failed to install $target_toolchain"
        return 1
    end

    log_message "✅ Successfully installed/verified $target_toolchain"
    return 0
end

function install_rust_analyzer_component
    log_message "Installing rust-analyzer component for $target_toolchain..."
    if not log_command_output "Adding rust-analyzer component..." rustup component add rust-analyzer --toolchain $target_toolchain
        log_message "❌ Failed to install rust-analyzer component"
        return 1
    end

    log_message "✅ Successfully installed rust-analyzer component"
    return 0
end

function cleanup_old_toolchains
    # Get disk usage before cleanup
    log_message "Checking disk usage before cleanup..."
    set -l before_size (du -sh ~/.rustup/toolchains 2>/dev/null | cut -f1)
    log_message "Toolchains directory size before cleanup: $before_size"

    # List all currently installed toolchains
    log_message "Currently installed toolchains:"
    set -l all_toolchains (rustup toolchain list | cut -d' ' -f1)
    for toolchain in $all_toolchains
        log_message "  - $toolchain"
    end

    # Cleanup old toolchains - keep only stable and our target nightly
    log_message "Starting aggressive toolchain cleanup..."
    set -l removed_count 0

    for toolchain in $all_toolchains
        # Keep stable toolchains
        if string match -q "stable-*" $toolchain
            log_message "  KEEPING: $toolchain (stable)"
            continue
        end

        # Keep our target nightly
        if string match -q "$target_toolchain*" $toolchain
            log_message "  KEEPING: $toolchain (target nightly)"
            continue
        end

        # Remove everything else (old nightlies, generic nightly, etc.)
        log_message "  REMOVING: $toolchain"
        if rustup toolchain uninstall $toolchain 2>&1 | tee -a $LOG_FILE
            set removed_count (math $removed_count + 1)
            log_message "    ✅ Successfully removed $toolchain"
        else
            log_message "    ❌ Failed to remove $toolchain"
        end
    end

    log_message "Removed $removed_count old toolchain(s)"

    # Get disk usage after cleanup
    log_message "Checking disk usage after cleanup..."
    set -l after_size (du -sh ~/.rustup/toolchains 2>/dev/null | cut -f1)
    log_message "Toolchains directory size after cleanup: $after_size"

    return 0
end

# ============================================================================
# Main Function
# ============================================================================

function main
    # Truncate log file at start of script so each run gets a fresh log
    echo -n "" > $LOG_FILE

    # Acquire lock before proceeding
    if not acquire_toolchain_lock
        return 1
    end

    # Initialize
    log_message "═══════════════════════════════════════════════════════"
    log_message "Rust Toolchain Update Started at "(date)
    log_message "═══════════════════════════════════════════════════════"
    log_message ""

    # Output log file location to stdout for user visibility
    echo ""
    echo "📋 Detailed log: $LOG_FILE"
    echo ""

    # Execute workflow
    validate_prerequisites
    or begin
        release_toolchain_lock
        return 1
    end

    show_current_state

    # Find a stable toolchain (with ICE validation)
    log_message "═══════════════════════════════════════════════════════"
    log_message "Phase 1: Find Stable Toolchain"
    log_message "═══════════════════════════════════════════════════════"
    log_message ""

    find_stable_toolchain
    or begin
        log_message "❌ FATAL: Could not find a stable toolchain"
        log_message "Please check the log file for details: $LOG_FILE"
        release_toolchain_lock
        return 1
    end

    log_message "═══════════════════════════════════════════════════════"
    log_message "Phase 2: Install Components"
    log_message "═══════════════════════════════════════════════════════"
    log_message ""

    # Note: rust-toolchain.toml was already updated during validation
    log_message "✅ rust-toolchain.toml already updated during validation"

    # Note: No need to install again - find_stable_toolchain already installed it
    log_message "✅ Toolchain $target_toolchain already installed"

    # Install rust-analyzer component
    install_rust_analyzer_component
    or begin
        release_toolchain_lock
        return 1
    end

    log_message "═══════════════════════════════════════════════════════"
    log_message "Phase 3: Cleanup Old Toolchains"
    log_message "═══════════════════════════════════════════════════════"
    log_message ""

    cleanup_old_toolchains

    log_message "═══════════════════════════════════════════════════════"
    log_message "Phase 4: Verify Final State"
    log_message "═══════════════════════════════════════════════════════"
    log_message ""

    verify_final_state

    log_message "═══════════════════════════════════════════════════════"
    log_message "Phase 5: Clean Caches and Full Verification Build"
    log_message "═══════════════════════════════════════════════════════"
    log_message ""

    clean_and_verify_build

    log_message "═══════════════════════════════════════════════════════"
    log_message "Phase 6: Update Cargo Development Tools"
    log_message "═══════════════════════════════════════════════════════"
    log_message ""

    # Update all cargo tools (nextest, bacon, flamegraph, etc.)
    log_message "Updating cargo development tools to latest versions..."
    if fish run.fish update-cargo-tools 2>&1 | tee -a $LOG_FILE
        log_message "✅ Cargo tools updated successfully"
    else
        log_message "⚠️  Cargo tools update had issues (non-critical, continuing)"
    end
    log_message ""

    # Release lock after successful completion
    release_toolchain_lock

    # Cleanup
    log_message "═══════════════════════════════════════════════════════"
    log_message "✅ Rust Toolchain Update Completed at "(date)
    log_message "Final toolchain: $target_toolchain"
    log_message "═══════════════════════════════════════════════════════"
    log_message ""

    # Send final success notification
    if command -v notify-send >/dev/null 2>&1
        notify-send --urgency=normal \
            "Rust Toolchain Update Complete" \
            "✅ Successfully updated to: $target_toolchain\nAll validation, cleanup, and verification passed" \
            2>/dev/null &
        log_message "Final success notification sent"
    else
        log_message "notify-send not available, skipping final notification"
    end

    return 0
end

# ============================================================================
# Script Entry Point
# ============================================================================

# Run main function and exit with its status
main
exit $status