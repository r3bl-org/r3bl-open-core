<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Task: Refactor and Consolidate run.nu Files](#task-refactor-and-consolidate-runnu-files)
  - [Overview](#overview)
  - [Current State](#current-state)
  - [Goals](#goals)
  - [Implementation Plan](#implementation-plan)
    - [Phase 1: Create bootstrap.sh](#phase-1-create-bootstrapsh)
    - [Phase 2: Consolidate Commands in Root run.nu](#phase-2-consolidate-commands-in-root-runnu)
      - [Workspace-wide Commands (existing, no changes)](#workspace-wide-commands-existing-no-changes)
      - [Enhanced install-cargo-tools](#enhanced-install-cargo-tools)
      - [New Watch Commands (cross-platform file watching)](#new-watch-commands-cross-platform-file-watching)
      - [TUI-specific Commands](#tui-specific-commands)
      - [cmdr-specific Commands](#cmdr-specific-commands)
      - [Unified Commands](#unified-commands)
    - [Phase 3: Command Summary](#phase-3-command-summary)
    - [Phase 4: Update Documentation](#phase-4-update-documentation)
      - [Files to update:](#files-to-update)
    - [Phase 5: Clean Up](#phase-5-clean-up)
  - [Key Benefits](#key-benefits)
  - [Success Criteria](#success-criteria)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Task: Refactor and Consolidate run.nu Files

## Overview

This document outlines the plan to consolidate three separate `run.nu` files in the r3bl-open-core
repository into a single unified script, along with creating a bootstrap script for initial setup.

## Current State

- **Root run.nu**: ~414 lines - Contains workspace-wide commands (removed ramdisk commands)
- **tui/run.nu**: 285 lines - Contains TUI-specific commands and duplicates
- **cmdr/run.nu**: 221 lines - Contains cmdr-specific commands and duplicates (removing watch
  commands)
- Total: ~920 lines across 3 files with significant duplication

## Goals

1. Consolidate all functionality into a single root `run.nu` file
2. Eliminate duplicate command implementations
3. Replace deprecated cargo-watch with inotifywait
4. Create bootstrap.sh for initial development environment setup
5. Maintain all unique functionality while improving organization
6. Add smart installation checks to avoid reinstalling existing tools

## Implementation Plan

### Phase 1: Create bootstrap.sh

Create a bootstrap script that handles initial setup:

```bash
#!/usr/bin/env bash
# bootstrap.sh - Initial setup for r3bl-open-core development

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

PKG_MGR=$(detect_pkg_mgr)
echo "Package manager: ${PKG_MGR:-manual installation required}"

# Install essentials
install_if_missing "rustup" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh && source \$HOME/.cargo/env"
install_if_missing "nu" "cargo install nu"

# Install file watcher
if [[ "$OSTYPE" == "darwin"* ]]; then
    install_if_missing "fswatch" "${PKG_MGR:-echo 'Install Homebrew first'} fswatch"
elif [[ -n "$PKG_MGR" ]]; then
    install_if_missing "inotifywait" "$PKG_MGR inotify-tools"
else
    echo "Warning: Install inotify-tools manually for file watching"
fi

# Setup development tools
nu run.nu install-cargo-tools
```

### Phase 2: Consolidate Commands in Root run.nu

#### Workspace-wide Commands (existing, no changes)

- `all` - Run all major checks (now calls install-cargo-tools)
- `build` - Build entire workspace
- `build-full` - Full build with clean and update
- `test` - Test entire workspace
- `clean` - Clean entire workspace
- `check` - Check all workspaces
- `clippy` - Run clippy on all workspaces
- `clippy-pedantic` - Pedantic mode
- `docs` - Generate docs for all
- `serve-docs` - Serve documentation
- `rustfmt` - Format all code
- `install-cargo-tools` - Install development tools (with smart checks)
- `upgrade-deps` - Upgrade dependencies
- `audit-deps` - Security audit
- `unmaintained` - Check for unmaintained deps
- `build-server` - Remote build server

#### Enhanced install-cargo-tools

Add intelligent checking before installation:

```nu
# Helper to install tools conditionally
def install_if_missing [tool: string, cmd: string] {
    if (which $tool | is-empty) { print $'Installing ($tool)...'; bash -c $cmd } else { print $'✓ ($tool) installed' }
}

def install-cargo-tools [] {
    # Cargo tools
    [bacon cargo-workspaces cargo-cache cargo-outdated cargo-update cargo-deny
     cargo-unmaintained cargo-expand cargo-readme cargo-nextest flamegraph inferno]
    | each {|tool| install_if_missing $tool $"cargo install ($tool)"}

    # Rust components
    if (rustup component list --installed | str contains "rust-analyzer" | not $in) {
        print 'Installing rust-analyzer...'; rustup component add rust-analyzer
    } else { print '✓ rust-analyzer installed' }

    # System tools (detect package manager)
    let pkg_mgr = if ($env.OS? == "Darwin") { "brew install" } else {
        ["apt-get" "dnf" "pacman"] | each {|pm|
            if (which $pm | is-not-empty) {
                match $pm { "apt-get" => "sudo apt install -y", "dnf" => "sudo dnf install -y", _ => $"sudo ($pm) -S --noconfirm" }
            }
        } | where $it != null | first?
    }

    if ($pkg_mgr != null) {
        install_if_missing "docker" $"($pkg_mgr) docker.io docker-compose"
        install_if_missing "go" $"($pkg_mgr) golang-go"
    }

    # Optional tools
    install_if_missing "claude" "curl -fsSL https://claude.ai/install.sh | sh"
    install_if_missing "mcp-language-server" "go install github.com/isaacphi/mcp-language-server@latest"

    # Configure claude if available
    if (which claude | is-not-empty) {
        try {
            print 'Configuring claude MCP servers...'
            claude mcp add-json "rust-analyzer" '{"type":"stdio","command":"/home/nazmul/go/bin/mcp-language-server","args":["--workspace","/home/nazmul/github/r3bl-open-core","--lsp","rust-analyzer"],"cwd":"/home/nazmul/github/r3bl-open-core"}'
            claude mcp add-json "context7" '{"type":"http","url":"https://mcp.context7.com/mcp"}'
        }
    }
}
```

#### New Watch Commands (cross-platform file watching)

Replace all cargo-watch implementations with cross-platform file watching:

```nu
# Cross-platform file watcher
def watch-files [command: string, dir: string = "."] {
    let watcher = if ($env.OS? == "Darwin") { "fswatch" } else { "inotifywait" }

    if (which $watcher | is-empty) {
        error make { msg: $"Install ($watcher) first" }
    }

    loop {
        print $'Watching ($dir) for changes...'

        if ($env.OS? == "Darwin") {
            ^fswatch -r --exclude "target|\.git" -1 $dir | complete
        } else {
            ^inotifywait -r -e modify,create,delete,move --exclude "target|\.git" $dir
        }

        print $'Running: ($command)'
        bash -c $command
    }
}

# Watch commands
def watch-all-tests [] { watch-files "cargo test --workspace --quiet -- --test-threads 4" }
def watch-one-test [pattern: string] { watch-files $"cargo test ($pattern) -- --nocapture --test-threads=1" }
def watch-clippy [] { watch-files "cargo clippy --workspace --all-targets --all-features" }
def watch-check [] { watch-files "cargo check --workspace" }
```

#### TUI-specific Commands

Port from tui/run.nu with consistent naming:

- `run-examples` - Interactive example runner (runs from tui/)
- `flamegraph-svg` - Generate SVG flamegraph
- `flamegraph-fold` - Generate perf-folded format
- `bench` - Run benchmarks
- `watch-test-expand` - Watch and expand macros for specific test

#### cmdr-specific Commands

Port from cmdr/run.nu:

- `run-binaries` - Interactive binary runner (edi, giti, rc)
- `install` - Install cmdr binaries
- `docker-build` - Build release in Docker

```nu
def run-binaries [] {
    cd cmdr
    let binaries = ["edi", "giti", "rc"]
    let selection = $binaries | input list --fuzzy 'Select a binary to run:'

    if $selection != null and $selection != "" {
        cargo run --bin $selection
    }
    cd ..
}
```

#### Unified Commands

- `log` - Monitor log.txt in current directory

### Phase 3: Command Summary

Final consolidated command structure:

```
# Workspace-wide commands
- all                    # Run all major checks
- build                  # Build entire workspace
- build-full             # Full build with clean
- test                   # Test entire workspace
- clean                  # Clean entire workspace
- check                  # Check all workspaces
- clippy                 # Run clippy
- clippy-pedantic        # Pedantic mode
- docs                   # Generate docs
- serve-docs             # Serve documentation
- rustfmt                # Format code
- install-cargo-tools    # Smart install with checks
- upgrade-deps           # Upgrade dependencies
- audit-deps             # Security audit
- unmaintained           # Check unmaintained deps
- build-server           # Remote build server

# Watch commands (inotifywait-based)
- watch-all-tests        # Watch and test all
- watch-one-test [pattern] # Watch and test specific
- watch-clippy           # Watch and clippy
- watch-check            # Watch and check

# TUI-specific
- run-examples           # Run TUI examples
- flamegraph-svg         # Generate flamegraph SVG
- flamegraph-fold        # Generate perf-folded
- bench                  # Run benchmarks
- watch-test-expand      # Watch and expand macros

# cmdr-specific
- run-binaries           # Run edi/giti/rc
- install                # Install binaries
- docker-build           # Docker build

# Unified
- log                    # Monitor log.txt
```

### Phase 4: Update Documentation

#### Files to update:

1. `README.md` - Add bootstrap.sh instructions
2. `tui/src/lib.rs` - Update development workflow docs; the `tui/README.md` is generated from this
   fil using `cargo readme`
3. `cmdr/src/lib.rs` - Update development workflow docs; the `cmdr/README.md` is generated from this
   fil using `cargo readme`
4. `analytics_schema/src/lib.rs` - Update if needed; ; the `analytics/README.md` is generated from
   this fil using `cargo readme`

### Phase 5: Clean Up

1. Test all functionality thoroughly
2. Remove `tui/run.nu`
3. Remove `cmdr/run.nu`
4. Verify no functionality was lost

## Key Benefits

1. **Single source of truth**: All commands in one file
2. **Smart installation**: Checks before installing, faster subsequent runs
3. **Event-driven watching**: Uses inotifywait instead of polling
4. **Simplified test watching**: Leverages cargo's workspace intelligence
5. **Clear organization**: Commands grouped by purpose
6. **Improved onboarding**: bootstrap.sh handles all setup

## Success Criteria

- [ ] All existing functionality preserved
- [ ] No duplicate implementations
- [ ] Bootstrap script works on fresh systems
- [ ] All watch commands use inotifywait
- [ ] install-cargo-tools skips already installed tools
- [ ] watch-one-test works from any directory
- [ ] Documentation updated
- [ ] Old run.nu files removed
- [ ] All tests pass
