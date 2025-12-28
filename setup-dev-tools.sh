#!/bin/bash
# setup-dev-tools.sh - Install system-level development tools for r3bl-open-core
# This script installs rustup, perf, and nushell which are required for development
# Supports: Debian/Ubuntu, Fedora/RHEL, Arch, openSUSE

set -e

echo "=== r3bl-open-core Development Tools Setup ==="
echo

# Detect package manager and distro type
detect_distro() {
    if command -v apt-get &> /dev/null; then
        echo "debian"
    elif command -v dnf &> /dev/null; then
        echo "fedora"
    elif command -v pacman &> /dev/null; then
        echo "arch"
    elif command -v zypper &> /dev/null; then
        echo "suse"
    else
        echo "unknown"
    fi
}

DISTRO_TYPE=$(detect_distro)

if [ "$DISTRO_TYPE" = "unknown" ]; then
    echo "Warning: Unsupported system. No recognized package manager found."
    echo "You may need to manually install: rustup, perf, and nushell"
    exit 1
fi

echo "Detected distro type: $DISTRO_TYPE"
echo

# Install rustup if not already installed
if ! command -v rustup &> /dev/null; then
    echo "Installing rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo "rustup installed successfully!"
else
    echo "rustup is already installed."
fi

# Install perf (package differs by distro)
echo "Installing perf..."
case "$DISTRO_TYPE" in
    debian)
        sudo apt update
        sudo apt install -y linux-tools-common linux-tools-generic linux-tools-$(uname -r) 2>/dev/null || \
            sudo apt install -y linux-tools-common linux-tools-generic
        ;;
    fedora)
        sudo dnf install -y perf
        ;;
    arch)
        sudo pacman -S --noconfirm perf
        ;;
    suse)
        sudo zypper install -y perf
        ;;
esac
echo "perf installed successfully!"

# Install nushell
echo "Installing nushell..."
case "$DISTRO_TYPE" in
    debian)
        if command -v snap &> /dev/null; then
            echo "Snap is available, installing nushell via snap..."
            sudo snap install nushell
            echo "nushell installed successfully via snap!"
        else
            echo "Snap is not available, installing nushell via cargo..."
            cargo install --locked nu
            echo "nushell installed successfully via cargo!"
        fi
        ;;
    fedora)
        # Try COPR first, fall back to cargo
        if sudo dnf copr enable -y atim/nushell 2>/dev/null; then
            sudo dnf install -y nushell
            echo "nushell installed successfully via COPR!"
        else
            echo "COPR not available, installing nushell via cargo..."
            cargo install --locked nu
            echo "nushell installed successfully via cargo!"
        fi
        ;;
    arch)
        sudo pacman -S --noconfirm nushell
        echo "nushell installed successfully via pacman!"
        ;;
    suse)
        echo "Installing nushell via cargo (recommended for openSUSE)..."
        cargo install --locked nu
        echo "nushell installed successfully via cargo!"
        ;;
esac

echo
echo "=== Installation Complete ==="
echo
echo "System tools installed:"
echo "  - rustup (Rust toolchain manager)"
echo "  - perf (Linux profiling tool)"
echo "  - nushell (nu command)"
echo
echo "Next steps:"
echo "  1. Run: nu run.nu install-cargo-tools"
echo "     This will install Rust development tools like:"
echo "     - cargo-flamegraph (for profiling)"
echo "     - inferno (for collapsed stack analysis)"
echo "     - And other useful development tools"
echo
echo "  2. Then you can use the TUI profiling commands:"
echo "     - cd tui"
echo "     - nu run.nu examples-with-flamegraph-profiling"
echo "     - nu run.nu examples-with-flamegraph-profiling-detailed-perf-fold"
echo
