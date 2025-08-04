#!/usr/bin/env bash
# bootstrap.sh - Initial setup for r3bl-open-core development for Linux and macOS

# Install tool if missing
# Note: This is not used by install_rustup(), because it has a complex workflow
install_if_missing() {
    command -v "$1" &>/dev/null && echo "✓ $1 already installed" || { echo "Installing $1..."; eval "$2"; }
}

# Detect package manager
# cspell:disable
detect_pkg_mgr() {
    [[ "$OSTYPE" == "darwin"* ]] && echo "brew install" && return
    command -v apt-get &>/dev/null && echo "sudo apt-get update && sudo apt-get install -y" && return
    command -v dnf &>/dev/null && echo "sudo dnf install -y" && return
    command -v pacman &>/dev/null && echo "sudo pacman -S --noconfirm" && return
    command -v zypper &>/dev/null && echo "sudo zypper install -y" && return
    command -v apk &>/dev/null && echo "sudo apk add" && return
}
# cspell:enable

PKG_MGR=$(detect_pkg_mgr)
echo "Package manager: ${PKG_MGR:-manual installation required}"

# Install Rust toolchain
# Note: This does not use by install_rustup(), because it has a complex workflow
install_rustup() {
    # Check for rustup in multiple ways since it might be installed but not in PATH yet
    if [[ -f "$HOME/.cargo/bin/rustup" ]] || [[ -d "$HOME/.rustup" ]] || command -v rustup &>/dev/null; then
        echo "✓ rustup already installed"
    else
        # cspell:disable
        echo "Installing rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        # Source cargo env if the file exists
        [[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"
        # cspell:enable
    fi
}

install_rustup

# Ensure cargo is in PATH for this session
export PATH="$HOME/.cargo/bin:$PATH"

# Verify rust installation
if ! command -v cargo &>/dev/null; then
    echo "Warning: cargo not found in PATH after installation"
    echo "You may need to restart your shell or run: source $HOME/.cargo/env"
fi

# Install nushell
install_if_missing "nu" "cargo install nu"

# Install file watcher
# cspell:disable
if [[ "$OSTYPE" == "darwin"* ]]; then
    if [[ -z "$PKG_MGR" ]] && ! command -v brew &>/dev/null; then
        echo "Warning: Homebrew not found. Install it from https://brew.sh/ then re-run this script"
        echo "Skipping fswatch installation..."
    else
        install_if_missing "fswatch" "${PKG_MGR:-brew install} fswatch"
    fi
elif [[ -n "$PKG_MGR" ]]; then
    install_if_missing "inotifywait" "$PKG_MGR inotify-tools"
else
    echo "Warning: No supported package manager found. Install inotify-tools manually for file watching"
    echo "  Ubuntu/Debian: sudo apt-get install inotify-tools"
    echo "  RHEL/CentOS/Fedora: sudo dnf install inotify-tools"
    echo "  Arch: sudo pacman -S inotify-tools"
    echo "  openSUSE: sudo zypper install inotify-tools"
fi
# cspell:enable

# Setup development tools
if command -v nu &>/dev/null; then
    echo "Setting up development tools..."
    nu run.nu install-cargo-tools
else
    echo "Warning: nushell (nu) not found in PATH. Skipping cargo tools installation."
    echo "You may need to restart your shell or run: source $HOME/.cargo/env"
    echo "Then run: nu run.nu install-cargo-tools"
fi