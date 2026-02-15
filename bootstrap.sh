#!/usr/bin/env bash

# cspell:words noconfirm

# bootstrap.sh - Initial OS-level setup for r3bl-open-core development
#
# PURPOSE:
#   Sets up a fresh machine for r3bl-open-core development. This script handles
#   OS-level dependencies (system packages, Rust toolchain) and then delegates
#   Rust-specific tooling to run.fish.
#
# USAGE:
#   ./bootstrap.sh              # Run from repository root
#
# WHAT IT INSTALLS:
#   Core:
#     - rustup/cargo            Rust toolchain manager
#     - clang                   Required by Wild linker for faster linking
#
#   Cross-Compilation:
#     - mingw-w64 GCC           Windows cross-compiler (gcc, dlltool, etc.)
#
#   Shell & Development:
#     - fish                    Shell used by run.fish build scripts
#     - fzf                     Fuzzy finder (used by run.fish commands)
#     - coreutils (macOS)       GNU timeout needed by check.fish test timeouts
#     - htop                    Used in PTY integration tests
#     - screen, tmux            Terminal multiplexers for testing
#     - expect                  Scripted terminal automation for benchmarks
#     - fswatch/inotify-tools   File watcher (macOS/Linux respectively)
#     - ansifilter              Strips ANSI escape sequences from log files
#
#   Optional:
#     - Node.js & npm           For Claude Code CLI installation
#     - Claude Code             AI coding assistant (has built-in LSP)
#
# SUPPORTED PLATFORMS:
#   - macOS (via Homebrew)
#   - Linux: apt (Debian/Ubuntu), dnf (Fedora/RHEL), pacman (Arch),
#            zypper (openSUSE), apk (Alpine)
#
# POST-BOOTSTRAP:
#   After this script completes, use 'fish run.fish <command>' for development.
#   See 'fish run.fish help' for available commands.
#
# SEE ALSO:
#   - run.fish: Rust-specific development commands (build, test, lint, etc.)
#   - CLAUDE.md: Project conventions and Claude Code instructions

# Install tool if missing
install_if_missing() {
    command -v "$1" &>/dev/null && echo "✓ $1 already installed" || { echo "Installing $1..."; eval "$2"; }
}

# Detect package manager
detect_pkg_mgr() {
    [[ "$OSTYPE" == "darwin"* ]] && echo "brew install" && return
    command -v apt-get &>/dev/null && echo "sudo apt-get update && sudo apt-get install -y" && return
    command -v dnf &>/dev/null && echo "sudo dnf install -y" && return
    command -v pacman &>/dev/null && echo "sudo pacman -S --noconfirm" && return
    command -v zypper &>/dev/null && echo "sudo zypper install -y" && return
    command -v apk &>/dev/null && echo "sudo apk add" && return
}

# Install Rust toolchain
install_rustup() {
    # Check for rustup in multiple ways since it might be installed but not in PATH yet
    if [[ -f "$HOME/.cargo/bin/rustup" ]] || [[ -d "$HOME/.rustup" ]] || command -v rustup &>/dev/null; then
        echo "✓ rustup already installed"
    else
        echo "Installing rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        # Source cargo env if the file exists
        [[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"
    fi
}

# Install clang (required by Wild linker)
install_clang() {
    if [[ "$OSTYPE" == "linux"* ]] && [[ -n "$PKG_MGR" ]]; then
        install_if_missing "clang" "$PKG_MGR clang"
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        # Clang is usually available on macOS through Xcode Command Line Tools
        if ! command -v clang &>/dev/null; then
            echo "Warning: clang not found. Install Xcode Command Line Tools:"
            echo "  xcode-select --install"
        else
            echo "✓ clang already available"
        fi
    fi
}

# Install mingw-w64 cross-compiler for Windows cross-compilation checks.
# Needed by cc-rs build scripts (e.g. libmimalloc-sys) that probe for
# x86_64-w64-mingw32-gcc even during metadata-only builds.
# GCC packages pull in binutils (dlltool, etc.) as dependencies.
install_mingw_tools() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        if [[ -z "$PKG_MGR" ]] && ! command -v brew &>/dev/null; then
            echo "Warning: Homebrew not found. Skipping mingw-w64 installation..."
        else
            install_if_missing "x86_64-w64-mingw32-gcc" "${PKG_MGR:-brew install} mingw-w64"
        fi
    elif [[ -n "$PKG_MGR" ]]; then
        if command -v apt-get &>/dev/null; then
            install_if_missing "x86_64-w64-mingw32-gcc" "$PKG_MGR gcc-mingw-w64-x86-64"
        elif command -v dnf &>/dev/null; then
            install_if_missing "x86_64-w64-mingw32-gcc" "$PKG_MGR mingw64-gcc"
        elif command -v pacman &>/dev/null; then
            install_if_missing "x86_64-w64-mingw32-gcc" "$PKG_MGR mingw-w64-gcc"
        elif command -v zypper &>/dev/null; then
            install_if_missing "x86_64-w64-mingw32-gcc" "$PKG_MGR cross-x86_64-w64-mingw32-gcc"
        elif command -v apk &>/dev/null; then
            install_if_missing "x86_64-w64-mingw32-gcc" "$PKG_MGR mingw-w64-gcc"
        fi
    else
        echo "Warning: No supported package manager found. Install mingw-w64 GCC manually"
        echo "  Ubuntu/Debian: sudo apt-get install gcc-mingw-w64-x86-64"
        echo "  RHEL/CentOS/Fedora: sudo dnf install mingw64-gcc"
        echo "  Arch: sudo pacman -S mingw-w64-gcc"
        echo "  openSUSE: sudo zypper install cross-x86_64-w64-mingw32-gcc"
    fi
}

# Install fish shell and fzf
install_shell_tools() {

    if [[ "$OSTYPE" == "darwin"* ]]; then
        if [[ -z "$PKG_MGR" ]] && ! command -v brew &>/dev/null; then
            echo "Warning: Homebrew not found. Install it from https://brew.sh/ then re-run this script"
            echo "Skipping fish and fzf installation..."
        else
            install_if_missing "fish" "${PKG_MGR:-brew install} fish"
            install_if_missing "fzf" "${PKG_MGR:-brew install} fzf"
            install_if_missing "gtimeout" "${PKG_MGR:-brew install} coreutils"
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
}

# Install file watcher
install_file_watcher() {
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
}

# Install development utilities
install_dev_utilities() {
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

    # Install expect for scripted terminal automation (used in benchmark mode)
    if [[ "$OSTYPE" == "darwin"* ]]; then
        if [[ -z "$PKG_MGR" ]] && ! command -v brew &>/dev/null; then
            echo "Warning: Homebrew not found. Skipping expect installation..."
        else
            install_if_missing "expect" "${PKG_MGR:-brew install} expect"
        fi
    elif [[ -n "$PKG_MGR" ]]; then
        install_if_missing "expect" "$PKG_MGR expect"
    else
        echo "Warning: No supported package manager found. Install expect manually"
        echo "  Ubuntu/Debian: sudo apt-get install expect"
        echo "  RHEL/CentOS/Fedora: sudo dnf install expect"
        echo "  Arch: sudo pacman -S expect"
        echo "  openSUSE: sudo zypper install expect"
    fi

    # Install ansifilter for stripping ANSI escape sequences from log files
    if [[ "$OSTYPE" == "darwin"* ]]; then
        if [[ -z "$PKG_MGR" ]] && ! command -v brew &>/dev/null; then
            echo "Warning: Homebrew not found. Skipping ansifilter installation..."
        else
            install_if_missing "ansifilter" "${PKG_MGR:-brew install} ansifilter"
        fi
    elif [[ -n "$PKG_MGR" ]]; then
        install_if_missing "ansifilter" "$PKG_MGR ansifilter"
    else
        echo "Warning: No supported package manager found. Install ansifilter manually"
        echo "  Ubuntu/Debian: sudo apt-get install ansifilter"
        echo "  RHEL/CentOS/Fedora: sudo dnf install ansifilter"
        echo "  Arch: sudo pacman -S ansifilter"
        echo "  openSUSE: sudo zypper install ansifilter"
    fi
}

# Install Node.js and npm
install_nodejs() {
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
}

# Install Claude Code and plugins
install_claude_code() {
    if command -v npm &>/dev/null; then
        if ! command -v claude &>/dev/null; then
            echo "Installing Claude Code..."
            npm install -g @anthropic-ai/claude-code
            # Fix npm permissions
            sudo chown -R $USER:$(id -gn) $(npm -g config get prefix)
        else
            echo "✓ claude already installed"
        fi

        # Claude Code now has built-in LSP server functionality - no plugins needed
        if command -v claude &>/dev/null; then
            echo "Claude Code installed successfully (has built-in LSP)"
        fi
    else
        echo "Warning: npm not found. Cannot install Claude Code"
    fi
}

# Setup development tools via run.fish
setup_cargo_tools() {
    if command -v fish &>/dev/null; then
        echo "Setting up development tools..."
        fish run.fish install-cargo-tools
    else
        echo "Warning: fish shell not found in PATH. Skipping cargo tools installation."
        echo "You may need to restart your shell or install fish first"
        echo "Then run: fish run.fish install-cargo-tools"
    fi
}

# Main function - orchestrates the entire bootstrap process
main() {
    echo "🚀 Starting r3bl-open-core bootstrap process..."
    echo "This script installs OS-level dependencies and then calls run.fish for Rust tooling."
    echo ""

    # Detect package manager first
    PKG_MGR=$(detect_pkg_mgr)
    echo "Package manager: ${PKG_MGR:-manual installation required}"
    echo ""

    # Core system setup
    echo "📦 Installing core system components..."
    install_rustup

    # Ensure cargo is in PATH for this session
    export PATH="$HOME/.cargo/bin:$PATH"

    # Verify rust installation
    if ! command -v cargo &>/dev/null; then
        echo "Warning: cargo not found in PATH after installation"
        echo "You may need to restart your shell or run: source $HOME/.cargo/env"
    fi

    echo ""
    echo "🔧 Installing development dependencies..."
    install_clang
    install_mingw_tools
    install_shell_tools
    install_file_watcher
    install_dev_utilities
    install_nodejs
    install_claude_code

    echo ""
    echo "⚙️  Setting up Rust development tools..."
    setup_cargo_tools

    echo ""
    echo "✅ Bootstrap complete! You can now use 'fish run.fish <command>' for development tasks."
}

# Run main function
main "$@"