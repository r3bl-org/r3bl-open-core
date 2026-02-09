# Toolchain Validation, Corruption Detection & Auto-Repair
#
# Validates Rust toolchain installation with all required components.
# Detects corrupted toolchains that appear installed but are broken internally.
#
# Corruption Detection:
# - Symptoms: "Missing manifest in toolchain", repeated "syncing channel updates" loops
# - Causes: interrupted installation, download failure, manifest loss
# - Detection: Checks for "Missing manifest" in rustc/component output
# - Recovery process:
#   1. Detects corruption BEFORE normal validation (prevents infinite loops)
#   2. Tries rustup toolchain uninstall first
#   3. Falls back to direct folder deletion (~/.rustup/toolchains/<name>)
#   4. Clears rustup caches (~/.rustup/downloads/, ~/.rustup/tmp/)
#   5. Delegates to sync script for fresh reinstall
# - On sync failure: Shows last 30 lines of output (not silently suppressed)

# Detects if a toolchain has a corrupted installation (e.g., "Missing manifest").
#
# A toolchain can appear in `rustup toolchain list` but be corrupted internally.
# This happens when installation is interrupted, download fails, or manifest is lost.
# Symptoms: "Missing manifest in toolchain", repeated "syncing channel updates" loops.
#
# Parameters:
#   $argv[1]: toolchain name (e.g., "nightly-2025-12-24")
#
# Returns: 0 if corrupted, 1 if OK or not installed
function is_toolchain_corrupted
    set -l toolchain $argv[1]

    # If not installed at all, it's not "corrupted" (just missing)
    if not is_toolchain_installed $toolchain
        return 1
    end

    # Try to run rustc and capture stderr for corruption patterns
    set -l rustc_output (rustup run $toolchain rustc --version 2>&1)
    set -l rustc_status $status

    # Check for known corruption patterns
    if echo "$rustc_output" | grep -qi "Missing manifest"
        return 0  # Corrupted
    end

    # Also check component listing (another way corruption manifests)
    set -l component_output (rustup component list --toolchain $toolchain 2>&1)
    if echo "$component_output" | grep -qi "Missing manifest"
        return 0  # Corrupted
    end

    # If rustc failed but no specific corruption pattern, check if it's truly broken
    if test $rustc_status -ne 0
        # Could be corruption or just missing components - check for other patterns
        if echo "$rustc_output" | grep -qi "error: toolchain .* is not installed"
            return 0  # Corrupted (claims not installed but is in list)
        end
    end

    return 1  # Not corrupted
end

# Force-removes a corrupted toolchain, including direct folder deletion if needed.
#
# When a toolchain has "Missing manifest", `rustup toolchain uninstall` may fail.
# This function tries rustup first, then falls back to direct folder deletion.
#
# Parameters:
#   $argv[1]: toolchain name (e.g., "nightly-2025-12-24")
#
# Returns: 0 if removed (or wasn't installed), 1 if removal failed
function force_remove_corrupted_toolchain
    set -l toolchain $argv[1]
    set -l toolchain_dir "$HOME/.rustup/toolchains/$toolchain-x86_64-unknown-linux-gnu"

    echo "🔧 Force-removing corrupted toolchain: $toolchain"

    # Try rustup uninstall first (might work even if corrupted)
    if rustup toolchain uninstall $toolchain 2>/dev/null
        echo "   ✅ Removed via rustup uninstall"
        return 0
    end

    # Rustup failed - try direct folder deletion
    if test -d "$toolchain_dir"
        echo "   ⚠️  rustup uninstall failed, removing folder directly..."
        if command rm -rf "$toolchain_dir"
            echo "   ✅ Removed folder: $toolchain_dir"
            # Also clear rustup caches to prevent stale state
            command rm -rf ~/.rustup/downloads/ 2>/dev/null
            command rm -rf ~/.rustup/tmp/ 2>/dev/null
            return 0
        else
            echo "   ❌ Failed to remove folder" >&2
            return 1
        end
    end

    # Folder doesn't exist - toolchain is effectively removed
    echo "   ✅ Toolchain folder already gone"
    return 0
end

# Helper function to ensure correct toolchain is installed with validation.
#
# Performs validation checks and handles recovery from various failure modes:
# - Missing toolchain: installs via sync script
# - Missing components: installs via sync script
# - Corrupted toolchain: force-removes first, then reinstalls
#
# Returns: 0 if toolchain is OK, 1 if error, 2 if toolchain was reinstalled
#
# Uses library functions for validation, delegates to sync script for installation.
# No lock needed for validation - sync script manages its own lock.
function ensure_toolchain_installed
    set -l target_toolchain (read_toolchain_from_toml)
    if test $status -ne 0
        echo "❌ Failed to read toolchain from rust-toolchain.toml" >&2
        return 1
    end

    # First, check for corruption (toolchain exists but is broken)
    # This must be handled BEFORE normal validation, otherwise we get stuck in loops
    if is_toolchain_corrupted $target_toolchain
        echo "🧊 Detected corrupted toolchain installation: $target_toolchain"
        echo "   Symptoms: Missing manifest, incomplete installation"
        echo ""
        force_remove_corrupted_toolchain $target_toolchain
        # Continue to reinstall below
    end

    # Perform quick validation using library functions (read-only, no lock needed)
    set -l validation_failed 0
    set -l failure_reason ""

    # Check if toolchain is installed
    if not is_toolchain_installed $target_toolchain
        set validation_failed 1
        set failure_reason "toolchain not installed"
    end

    # Check rust-analyzer component
    if test $validation_failed -eq 0
        if not is_component_installed $target_toolchain "rust-analyzer"
            set validation_failed 1
            set failure_reason "rust-analyzer component missing"
        end
    end

    # Check rust-src component
    if test $validation_failed -eq 0
        if not is_component_installed $target_toolchain "rust-src"
            set validation_failed 1
            set failure_reason "rust-src component missing"
        end
    end

    # Verify rustc works
    if test $validation_failed -eq 0
        if not rustup run $target_toolchain rustc --version >/dev/null 2>&1
            set validation_failed 1
            set failure_reason "rustc failed to run"
        end
    end

    # If validation passed, we're done
    if test $validation_failed -eq 0
        return 0
    end

    # Validation failed - delegate to sync script for installation
    # The sync script will acquire its own lock to prevent concurrent modifications
    echo "⚠️  Toolchain validation failed ($failure_reason), installing..."
    echo ""

    # Run sync script - capture output to temp file so we can show it on failure
    set -l sync_log (mktemp)
    if fish ./rust-toolchain-sync-to-toml.fish > $sync_log 2>&1
        # Success - clean up and notify
        command rm -f $sync_log
        if command -v notify-send >/dev/null 2>&1
            notify-send --urgency=normal \
                "Toolchain Installation Complete" \
                "✅ Successfully installed: $target_toolchain with all components" \
                2>/dev/null &
        end
        echo "✅ Toolchain $target_toolchain was installed/repaired"
        return 2
    else
        # Failure - show what went wrong
        echo ""
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "❌ Sync script failed. Output (last 30 lines):"
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        tail -n 30 $sync_log
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo ""
        echo "📋 Full log: ~/Downloads/rust-toolchain-sync-to-toml.log"
        command rm -f $sync_log

        if command -v notify-send >/dev/null 2>&1
            notify-send --urgency=critical \
                "Toolchain Installation Failed" \
                "❌ Failed to install $target_toolchain - check terminal" \
                2>/dev/null &
        end
        echo "❌ Toolchain installation failed" >&2
        return 1
    end
end
