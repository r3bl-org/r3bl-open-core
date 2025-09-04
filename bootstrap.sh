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

# Configure RUSTFLAGS for faster compilation on Linux
if [[ "$OSTYPE" == "linux"* ]]; then
    if ! grep -q "RUSTFLAGS.*-Z threads=" "$HOME/.profile" 2>/dev/null; then
        echo "Configuring Rust parallel compiler for faster builds..."
        echo "" >> "$HOME/.profile"
        echo "# https://corrode.dev/blog/tips-for-faster-rust-compile-times/#switch-to-the-new-parallel-compiler-frontend" >> "$HOME/.profile"
        echo "export RUSTFLAGS=\"-Z threads=8\"" >> "$HOME/.profile"
        echo "✓ Added RUSTFLAGS configuration to ~/.profile"
        echo "Note: You may need to restart your shell or run: source ~/.profile"
    else
        echo "✓ RUSTFLAGS already configured in ~/.profile"
    fi
fi

# Install fish shell and fzf
if [[ "$OSTYPE" == "darwin"* ]]; then
    if [[ -z "$PKG_MGR" ]] && ! command -v brew &>/dev/null; then
        echo "Warning: Homebrew not found. Install it from https://brew.sh/ then re-run this script"
        echo "Skipping fish and fzf installation..."
    else
        install_if_missing "fish" "${PKG_MGR:-brew install} fish"
        install_if_missing "fzf" "${PKG_MGR:-brew install} fzf"
    fi
elif [[ -n "$PKG_MGR" ]]; then
    install_if_missing "fish" "$PKG_MGR fish"
    install_if_missing "fzf" "$PKG_MGR fzf"
else
    echo "Warning: No supported package manager found. Install fish and fzf manually"
    echo "  Ubuntu/Debian: sudo apt-get install fish fzf"
    echo "  RHEL/CentOS/Fedora: sudo dnf install fish fzf"
    echo "  Arch: sudo pacman -S fish fzf"
    echo "  openSUSE: sudo zypper install fish fzf"
fi

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

# Install htop for PTY integration tests
if [[ "$OSTYPE" == "darwin"* ]]; then
    if [[ -z "$PKG_MGR" ]] && ! command -v brew &>/dev/null; then
        echo "Warning: Homebrew not found. Skipping htop installation..."
    else
        install_if_missing "htop" "${PKG_MGR:-brew install} htop"
    fi
elif [[ -n "$PKG_MGR" ]]; then
    install_if_missing "htop" "$PKG_MGR htop"
else
    echo "Warning: No supported package manager found. Install htop manually"
    echo "  Ubuntu/Debian: sudo apt-get install htop"
    echo "  RHEL/CentOS/Fedora: sudo dnf install htop"
    echo "  Arch: sudo pacman -S htop"
    echo "  openSUSE: sudo zypper install htop"
fi

# Install screen and tmux terminal multiplexers
if [[ "$OSTYPE" == "darwin"* ]]; then
    if [[ -z "$PKG_MGR" ]] && ! command -v brew &>/dev/null; then
        echo "Warning: Homebrew not found. Skipping screen and tmux installation..."
    else
        install_if_missing "screen" "${PKG_MGR:-brew install} screen"
        install_if_missing "tmux" "${PKG_MGR:-brew install} tmux"
    fi
elif [[ -n "$PKG_MGR" ]]; then
    install_if_missing "screen" "$PKG_MGR screen"
    install_if_missing "tmux" "$PKG_MGR tmux"
else
    echo "Warning: No supported package manager found. Install screen and tmux manually"
    echo "  Ubuntu/Debian: sudo apt-get install screen tmux"
    echo "  RHEL/CentOS/Fedora: sudo dnf install screen tmux"
    echo "  Arch: sudo pacman -S screen tmux"
    echo "  openSUSE: sudo zypper install screen tmux"
fi

# Install Node.js and npm
if [[ "$OSTYPE" == "darwin"* ]]; then
    if [[ -z "$PKG_MGR" ]] && ! command -v brew &>/dev/null; then
        echo "Warning: Homebrew not found. Skipping Node.js installation..."
    else
        install_if_missing "node" "${PKG_MGR:-brew install} node"
    fi
elif [[ -n "$PKG_MGR" ]]; then
    install_if_missing "node" "$PKG_MGR nodejs npm"
else
    echo "Warning: No supported package manager found. Install Node.js and npm manually"
    echo "  Ubuntu/Debian: sudo apt-get install nodejs npm"
    echo "  RHEL/CentOS/Fedora: sudo dnf install nodejs npm"
    echo "  Arch: sudo pacman -S nodejs npm"
    echo "  openSUSE: sudo zypper install nodejs npm"
fi

# Install Claude Code via npm
if command -v npm &>/dev/null; then
    if ! command -v claude &>/dev/null; then
        echo "Installing Claude Code..."
        npm install -g @anthropic-ai/claude-code
        # Fix npm permissions
        sudo chown -R $USER:$(id -gn) $(npm -g config get prefix)
        
        # Configure MCP servers for Claude
        if command -v claude &>/dev/null; then
            echo "Configuring Claude MCP servers..."
            claude mcp add-json context7 '{"type":"http","url":"https://mcp.context7.com/mcp"}'
            claude mcp add serena -- uvx --from git+https://github.com/oraios/serena serena start-mcp-server --context ide-assistant --project "$PWD" 2>/dev/null || true
        fi
    else
        echo "✓ claude already installed"
    fi
else
    echo "Warning: npm not found. Cannot install Claude Code"
fi

# Setup development tools
if command -v fish &>/dev/null; then
    echo "Setting up development tools..."
    fish run.fish install-cargo-tools
else
    echo "Warning: fish shell not found in PATH. Skipping cargo tools installation."
    echo "You may need to restart your shell or install fish first"
    echo "Then run: fish run.fish install-cargo-tools"
fi