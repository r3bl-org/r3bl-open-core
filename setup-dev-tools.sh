#!/bin/bash
# setup-dev-tools.sh - Install system-level development tools for r3bl-open-core
# This script installs rustup, perf, and nushell which are required for development

set -e

echo "=== r3bl-open-core Development Tools Setup ==="
echo

# Check if running on a supported system
if ! command -v apt &> /dev/null; then
    echo "Warning: This script is designed for Debian/Ubuntu systems with apt."
    echo "You may need to manually install: rustup, perf, and nushell"
    exit 1
fi

# Install rustup if not already installed
if ! command -v rustup &> /dev/null; then
    echo "Installing rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo "rustup installed successfully!"
else
    echo "rustup is already installed."
fi

# Install perf
echo "Installing perf..."
sudo apt update
sudo apt install -y linux-tools-common linux-tools-generic linux-tools-$(uname -r)
echo "perf installed successfully!"

# Install nushell
echo "Installing nushell..."
if command -v snap &> /dev/null; then
    echo "Snap is available, installing nushell via snap..."
    sudo snap install nushell
    echo "nushell installed successfully via snap!"
else
    echo "Snap is not available, installing nushell via cargo..."
    cargo install --locked nu
    echo "nushell installed successfully via cargo!"
fi

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