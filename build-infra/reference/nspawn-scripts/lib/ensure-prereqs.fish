#!/usr/bin/env fish

# Host Prerequisites for Test Suite
#
# Usage: source /path/to/tests/lib/ensure-prereqs.fish
#        ensure_host_prereqs
#
# Checks and installs required host dependencies for running tests.

# ============================================================================
# Individual Prerequisite Functions
# ============================================================================

function ensure_podman -d "Ensure podman is installed (needed for OCI image extraction)"
    if command -q podman
        return 0
    end

    echo "Installing podman (needed for OCI image extraction)..."

    pkg_install_translated podman

    if not command -q podman
        echo "❌ Failed to install podman"
        return 1
    end

    echo "✅ podman installed"
end

function ensure_systemd_nspawn -d "Ensure systemd-nspawn is available"
    if command -q systemd-nspawn
        return 0
    end

    echo "Installing systemd-container (provides systemd-nspawn)..."

    # On Arch, systemd-nspawn is part of base systemd package
    # On Debian/Fedora, it's in systemd-container
    if is_arch_based
        echo "❌ systemd-nspawn not found"
        echo "   On Arch, this should be part of systemd. Try: sudo pacman -S systemd"
        return 1
    else
        pkg_install_translated systemd-container
    end

    if not command -q systemd-nspawn
        echo "❌ Failed to install systemd-nspawn"
        return 1
    end

    echo "✅ systemd-nspawn installed"
end

function ensure_wget -d "Ensure wget is available"
    if command -q wget
        return 0
    end

    echo "Installing wget..."

    # On Fedora 43+, wget is replaced by wget2 (provides backwards-compatible wget command)
    pkg_install_translated wget

    if not command -q wget
        echo "❌ Failed to install wget"
        return 1
    end

    echo "✅ wget installed"
end

# ============================================================================
# Main Prerequisite Check
# ============================================================================

function ensure_host_prereqs -d "Ensure all host prerequisites are installed"
    echo "Checking host prerequisites..."

    ensure_wget; or return 1
    ensure_podman; or return 1
    ensure_systemd_nspawn; or return 1

    echo "✅ All prerequisites satisfied"
    return 0
end
