#!/usr/bin/env fish

# fish docs
# - getting started: https://developerlife.com/2021/01/19/fish-scripting-manual/
# - language fundamentals: https://fishshell.com/docs/current/language.html
#     - strings: https://fishshell.com/docs/current/language.html#quotes
#     - functions: https://fishshell.com/docs/current/language.html#functions
#     - variables: https://fishshell.com/docs/current/language.html#variables
# - built-in command reference: https://fishshell.com/docs/current/commands.html
#     - count: https://fishshell.com/docs/current/cmds/count.html
#     - string: https://fishshell.com/docs/current/cmds/string.html
#     - test: https://fishshell.com/docs/current/cmds/test.html
# - mental model
#     - coming from bash: https://fishshell.com/docs/current/fish_for_bash_users.html
#     - thinking in fish: https://fishshell.com/docs/current/tutorial.html

source script_lib.fish

# Make sure to keep this in sync with `Cargo.toml` workspace members.
set workspace_folders (get_cargo_projects)

# Main entry point for the script.
function main
    set num_args (count $argv)

    if test $num_args -eq 0
        print-help all
        return
    end

    set command $argv[1]

    switch $command
        case all
            all
        case build
            build
        case build-full
            build-full
        case clean
            clean
        case install-cargo-tools
            install-cargo-tools
        case test
            test_workspace
        case watch-all-tests
            watch-all-tests
        case watch-one-test
            if test (count $argv) -gt 1
                watch-one-test $argv[2]
            else
                echo "Error: watch-one-test requires a test pattern"
                echo "Usage: fish run.fish watch-one-test <pattern>"
            end
        case watch-clippy
            watch-clippy
        case watch-check
            watch-check
        case docs
            docs
        case check
            check
        case clippy
            clippy
        case clippy-pedantic
            clippy-pedantic
        case rustfmt
            rustfmt
        case upgrade-deps
            upgrade-deps
        case update-cargo-tools
            update-cargo-tools
        case serve-docs
            serve-docs
        case audit-deps
            audit-deps
        case toolchain-update
            toolchain-update
        case toolchain-sync
            toolchain-sync
        case toolchain-remove
            toolchain-remove
        case toolchain-validate
            toolchain-validate
        case toolchain-validate-complete
            toolchain-validate-complete
        case check-full
            check-full
        case unmaintained-deps
            unmaintained-deps
        case help
            print-help all
        case build-server
            build-server
            # TUI-specific commands
        case run-examples
            run-examples $argv[2..-1]
        case run-examples-flamegraph-svg
            run-examples-flamegraph-svg
        case run-examples-flamegraph-fold
            # Check for --benchmark flag
            if test (count $argv) -gt 1; and test "$argv[2]" = "--benchmark"
                run-examples-flamegraph-fold --benchmark
            else
                run-examples-flamegraph-fold
            end
        case bench
            bench
            # cmdr-specific commands
        case run-binaries
            run-binaries
        case install-cmdr
            install-cmdr
        case docker-build
            docker-build
            # Unified commands
        case log
            log
        case dev-dashboard
            dev-dashboard
        case '*'
            echo "Unknown command: "(set_color red --bold)"$command"(set_color normal)
    end
end

# Prints help for the script.
# - If "all" is passed in, prints help for all commands.
# - Otherwise, prints help for the specified command.
function print-help
    set command $argv[1]

    if test "$command" = all
        echo "Usage: "(set_color magenta --bold)"run"(set_color normal)" "(set_color green --bold)"<command>"(set_color normal)" "(set_color blue --bold)"[args]"(set_color normal)
        echo ""
        echo (set_color cyan --bold)"Workspace-wide commands:"(set_color normal)
        echo "    "(set_color green)"all"(set_color normal)"                  Run all major checks"
        echo "    "(set_color green)"build"(set_color normal)"                Build entire workspace"
        echo "    "(set_color green)"build-full"(set_color normal)"           Full build with clean and update"
        echo "    "(set_color green)"clean"(set_color normal)"                Clean entire workspace"
        echo "    "(set_color green)"test"(set_color normal)"                 Test entire workspace"
        echo "    "(set_color green)"check"(set_color normal)"                Check all workspaces"
        echo "    "(set_color green)"clippy"(set_color normal)"               Run clippy on all workspaces"
        echo "    "(set_color green)"clippy-pedantic"(set_color normal)"      Run clippy with pedantic lints"
        echo "    "(set_color green)"docs"(set_color normal)"                 Generate docs for all"
        echo "    "(set_color green)"serve-docs"(set_color normal)"           Serve documentation"
        echo "    "(set_color green)"rustfmt"(set_color normal)"              Format all code"
        echo "    "(set_color green)"install-cargo-tools"(set_color normal)"  Install development tools"
        echo "    "(set_color green)"upgrade-deps"(set_color normal)"         Upgrade dependencies"
        echo "    "(set_color green)"update-cargo-tools"(set_color normal)"   Update cargo dev tools"
        echo "    "(set_color green)"audit-deps"(set_color normal)"           Security audit"
        echo "    "(set_color green)"unmaintained-deps"(set_color normal)"    Check for unmaintained deps"
        echo "    "(set_color green)"toolchain-update"(set_color normal)"     Update Rust to month-old nightly"
        echo "    "(set_color green)"toolchain-sync"(set_color normal)"       Sync environment to rust-toolchain.toml"
        echo "    "(set_color green)"toolchain-validate"(set_color normal)"        Quick toolchain validation (components only)"
        echo "    "(set_color green)"toolchain-validate-complete"(set_color normal)"  Complete toolchain validation (full build+test)"
        echo "    "(set_color green)"toolchain-remove"(set_color normal)"     Remove ALL toolchains (testing)"
        echo "    "(set_color green)"check-full"(set_color normal)"           Run comprehensive checks (tests, docs, toolchain)"
        echo "    "(set_color green)"build-server"(set_color normal)"         Remote build server - uses rsync"
        echo ""
        echo (set_color cyan --bold)"Watch commands:"(set_color normal)
        echo "    "(set_color green)"watch-all-tests"(set_color normal)"      Watch files, run all tests"
        echo "    "(set_color green)"watch-one-test"(set_color normal)" "(set_color blue)"[pattern]"(set_color normal)"  Watch files, run specific test"
        echo "    "(set_color green)"watch-clippy"(set_color normal)"         Watch files, run clippy"
        echo "    "(set_color green)"watch-check"(set_color normal)"          Watch files, run cargo check"
        echo ""
        echo (set_color cyan --bold)"TUI-specific commands:"(set_color normal)
        echo "    "(set_color green)"run-examples"(set_color normal)" "(set_color blue)"[--release] [--no-log]"(set_color normal)"  Run TUI examples"
        echo "    "(set_color green)"run-examples-flamegraph-svg"(set_color normal)"  Generate SVG flamegraph"
        echo "    "(set_color green)"run-examples-flamegraph-fold"(set_color normal)" "(set_color blue)"[--benchmark]"(set_color normal)"  Generate perf-folded (use --benchmark for reproducible profiling)"
        echo "    "(set_color green)"bench"(set_color normal)"                Run benchmarks"
        echo ""
        echo (set_color cyan --bold)"cmdr-specific commands:"(set_color normal)
        echo "    "(set_color green)"run-binaries"(set_color normal)"         Run edi, giti, or rc"
        echo "    "(set_color green)"install-cmdr"(set_color normal)"         Install cmdr binaries"
        echo "    "(set_color green)"docker-build"(set_color normal)"         Build release in Docker"
        echo ""
        echo (set_color cyan --bold)"Development Session Commands:"(set_color normal)
        echo "    "(set_color green)"dev-dashboard"(set_color normal)"        Start 4-pane tmux development dashboard"
        echo ""
        echo (set_color cyan --bold)"Other commands:"(set_color normal)
        echo "    "(set_color green)"log"(set_color normal)"                  Monitor log.txt in cmdr or tui directory"
        echo "    "(set_color green)"help"(set_color normal)"                 Show this help"
    else
        echo "Unknown command: "(set_color red --bold)"$command"(set_color normal)
    end
end

# Synchronizes local development files to a remote build server using rsync.
#
# This function sets up a continuous file watcher that monitors changes in the local
# workspace and automatically syncs them to a remote server for distributed builds.
# Useful for leveraging more powerful remote hardware for compilation.
#
# Features:
# - Uses inotifywait to monitor file system changes
# - Automatically excludes target/ directories from sync
# - Provides real-time feedback on sync operations
# - Graceful Ctrl+C handling
#
# Prerequisites:
# - rsync installed locally
# - SSH access to the remote build server
# - inotifywait installed (part of inotify-tools)
#
# Usage:
#   fish run.fish build-server
#
# Then on the remote server, run:
#   cargo test --all-targets
#   cargo doc --no-deps
#   cargo clippy --all-targets
function build-server
    # Where you source files live.
    set orig_path $HOME/github/r3bl-open-core/
    # Copy files to here.
    set dest_host "nazmul-laptop.local"
    set dest_path "$dest_host:$orig_path"

    # Function to run rsync. Simple.
    function run_rsync --argument-names orig_path dest_path
        echo (set_color cyan --underline)"Changes detected, running rsync"(set_color normal)

        set prefix "┤ "
        set hanging_prefix "├ "
        set msg_1 (set_color yellow)"$orig_path"(set_color normal)
        set msg_2 (set_color blue)"$dest_path"(set_color normal)
        printf "$prefix%s\t%s\n" "from:" "$msg_1"
        printf "$hanging_prefix%s\t%s\n" "to:" "$msg_2"
        rsync -q -avz --delete --exclude target/ "$orig_path" "$dest_path"

        echo (set_color cyan --underline)"Rsync complete"(set_color normal)
    end

    # Main loop
    while true
        # Construct the inotifywait command with all directories.
        set inotify_command inotifywait
        set inotify_args -r -e modify -e create -e delete -e move --exclude target $orig_path

        echo (set_color green --bold)" "(set_color yellow)"$inotify_command"(set_color normal)" "(set_color blue)"$inotify_args"(set_color normal)

        echo (set_color cyan)"❪◕‿◕❫ "(set_color normal)(set_color green)"Please run on build server: "(set_color normal)(set_color yellow --underline)"cargo test --all-targets"(set_color normal)", "(set_color yellow --underline)"cargo doc --no-deps"(set_color normal)", "(set_color yellow --underline)"cargo clippy --all-targets"(set_color normal)

        # Execute the inotifywait command with proper Ctrl+C handling
        if not $inotify_command $inotify_args >/dev/null 2>&1
            # User pressed Ctrl+C, exit gracefully
            return
        end

        # Run rsync
        run_rsync "$orig_path" "$dest_path"
    end
end

# Watch commands - using watch-files from script_lib.fish
function watch-all-tests
    watch-files "cargo test --workspace --quiet -- --test-threads 4"
end

function watch-one-test
    set pattern $argv[1]
    watch-files "cargo test $pattern -- --nocapture --test-threads=1"
end

function watch-clippy
    watch-files "cargo clippy --workspace --all-targets --all-features"
end

function watch-check
    watch-files "cargo check --workspace"
end

# Installs and configures all development tools needed for the workspace.
#
# This comprehensive installer sets up:
# 1. cargo-binstall for fast binary installation
# 2. uv package manager (required for Serena semantic code MCP server)
# 3. Cargo tools for Rust development (bacon, flamegraph, etc.)
# 4. Wild linker with .cargo/config.toml generation
# 5. Language servers (rust-analyzer)
#
# Features:
# - Cross-platform support (macOS, Linux)
# - Uses cargo-binstall for faster installations with fallback to cargo install --locked
# - Idempotent - safe to run multiple times
# - Generates optimized .cargo/config.toml with Wild linker support
# - Uses shared utility functions from script_lib.fish
#
# Prerequisites:
# - bootstrap.sh must be run first to install Rust, Cargo, Fish, and OS dependencies
#
# Tools installed:
# - cargo-binstall: Fast binary installer
# - uv: Modern Python package manager (for Serena MCP server)
# - bacon: Background rust code checker
# - cargo-workspaces: Multi-crate workspace management
# - cargo-cache: Cargo cache management
# - cargo-outdated: Dependency version checker
# - cargo-update: Update installed binaries
# - cargo-deny: Supply chain security auditing
# - cargo-unmaintained: Check for unmaintained dependencies
# - cargo-expand: Show macro expansions
# - cargo-readme: Generate README from doc comments
# - flamegraph: Performance profiling visualization
# - inferno: Fast stack trace visualizer
# - wild: Fast linker (wild-linker package)
# - rust-analyzer: Language server
#
# Usage:
#   fish run.fish install-cargo-tools
function install-cargo-tools
    # Install cargo-binstall first for fast binary installation
    install_if_missing "cargo-binstall" "curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash"

    # Install uv package manager (required for Serena semantic code MCP server).
    # https://github.com/oraios/serena
    # https://claudelog.com/addons/serena/
    install_if_missing "uv" "curl -LsSf https://astral.sh/uv/install.sh | sh"

    # Cargo tools - using cargo binstall with fallback to cargo install
    set cargo_tools \
        "bacon" \
        "cargo-workspaces" \
        "cargo-cache" \
        "cargo-outdated" \
        "cargo-update" \
        "cargo-deny" \
        "cargo-unmaintained" \
        "cargo-expand" \
        "cargo-readme" \
        "flamegraph" \
        "inferno"

    for tool in $cargo_tools
        install_cargo_tool $tool
    end

    # Install Wild linker via cargo-binstall
    if command -v cargo-binstall >/dev/null
        install_if_missing "wild" "cargo binstall -y wild-linker"
    else
        echo "Warning: cargo-binstall not found. Cannot install Wild linker efficiently"
        install_if_missing "wild" "cargo install wild-linker"
    end

    # Install r3bl-build-infra tools (cargo-rustdoc-fmt)
    echo 'Installing r3bl-build-infra tools...'
    set original_dir $PWD
    cd build-infra
    if cargo install --path . --force
        echo '✓ cargo-rustdoc-fmt installed'
    else
        echo '⚠️  Failed to install r3bl-build-infra tools'
    end
    cd $original_dir

    # Generate appropriate cargo configuration after installation
    generate_cargo_config

    # Rust components
    if not rustup component list --installed | grep -q rust-analyzer
        echo 'Installing rust-analyzer...'
        rustup component add rust-analyzer
    else
        echo '✓ rust-analyzer installed'
    end
end

# Runs all major checks and tasks for the entire workspace.
#
# This is the main CI/CD command that ensures code quality and correctness.
# It performs a comprehensive suite of checks in the following order:
# 1. Installs/updates development tools
# 2. Builds all workspace projects
# 3. Runs all tests
# 4. Runs clippy linting with auto-fixes
# 5. Generates documentation
# 6. Audits dependencies for security vulnerabilities
# 7. Checks for unmaintained dependencies
# 8. Formats all code with rustfmt
#
# This command is ideal for:
# - Pre-commit checks
# - CI/CD pipelines
# - Ensuring code quality before PR submission
#
# Usage:
#   fish run.fish all
function all
    # Installs and runs all major checks and tools for the workspace.
    install-cargo-tools
    build
    test_workspace
    clippy
    docs
    audit-deps
    unmaintained-deps
    rustfmt
end

# https://github.com/trailofbits/cargo-unmaintained
# This is very slow to run.
function unmaintained-deps
    cargo unmaintained --color always --fail-fast --tree --verbose
end

# Updates Rust toolchain to a month-old nightly version and cleans up old toolchains.
#
# This function runs the rust-toolchain-update.fish script which automatically:
# - Updates rust-toolchain.toml to use a nightly version from 1 month ago
# - Installs the target toolchain if not already present
# - Installs rust-analyzer component (required by VSCode, RustRover, cargo, and serena MCP server)
# - Removes all other nightly toolchains (keeping only stable + target nightly)
# - Performs aggressive cleanup to save disk space
#
# The strategy avoids instability issues with bleeding-edge nightly builds
# while still providing access to nightly features. The script is designed
# to run weekly via systemd timer for automated maintenance.
#
# Features:
# - Uses date from 1 month ago for stability
# - Validates rust-toolchain.toml before updating
# - Comprehensive logging to ~/Downloads/rust-toolchain-update.log
# - Disk usage reporting before/after cleanup
#
# Usage:
#   fish run.fish toolchain-update
function toolchain-update
    fish rust-toolchain-update.fish
end

# Syncs Rust environment to match the rust-toolchain.toml file.
#
# This function runs the rust-toolchain-sync-to-toml.fish script which:
# - Reads the channel value from rust-toolchain.toml (doesn't modify it)
# - Installs the exact toolchain specified in the TOML
# - Installs rust-analyzer and rust-src components automatically (required by IDEs and serena MCP server)
# - Removes all other nightly toolchains (keeping only stable + target from TOML)
# - Performs aggressive cleanup to save disk space
#
# Use this when rust-toolchain.toml changes via git operations or manual edits
# and you need to install the specified toolchain with all required components.
# Also fixes serena MCP server crashes caused by missing rust-analyzer.
#
# Features:
# - Respects existing rust-toolchain.toml value
# - Automatic component installation
# - Comprehensive logging to ~/Downloads/rust-toolchain-sync-to-toml.log
# - Disk usage reporting before/after cleanup
#
# Usage:
#   fish run.fish toolchain-sync
function toolchain-sync
    fish rust-toolchain-sync-to-toml.fish
end

# Removes ALL Rust toolchains for testing upgrade progress display.
#
# ⚠️ WARNING: This is a DESTRUCTIVE testing utility that removes ALL toolchains!
#
# This function runs the remove_toolchains.sh script which is used for testing
# the upgrade progress display in cmdr apps (edi, giti). It creates a clean slate
# by removing all toolchains so you can observe the full rustup installation process.
#
# Use case:
# - Testing cmdr/src/analytics_client/upgrade_check.rs functionality
# - Observing full rustup download and installation progress
# - Debugging upgrade UI components
#
# Recovery after testing:
#   rustup toolchain install stable && rustup default stable
#   # Or: fish run.fish toolchain-update
#
# Usage:
#   fish run.fish toolchain-remove
function toolchain-remove
    bash remove_toolchains.sh
end

function toolchain-validate
    fish rust-toolchain-validate.fish quick
end

function toolchain-validate-complete
    fish rust-toolchain-validate.fish complete
end

function check-full
    fish check.fish
end

function build
    cargo build
end

function build-full
    install-cargo-tools
    cargo clean
    toolchain-update
    cargo build
end

function clean
    cargo clean
end

function test_workspace
    for folder in $workspace_folders
        cd $folder
        echo (set_color magenta)"≡ Running tests in $folder .. ≡"(set_color normal)
        cargo test -q -- --test-threads=20
        cd ..
    end
end

function check
    cargo check --workspace
end

# Runs comprehensive clippy linting with automatic fixes for all workspace projects.
#
# This function performs multiple passes of clippy with different lint levels,
# automatically fixing issues where possible. It applies a curated set of lints
# that improve code quality, safety, and idiomaticity.
#
# Workflow:
# 1. Runs cargo fix for basic issues
# 2. Applies specific clippy lints with auto-fix
# 3. Runs rustfmt after fixes
# 4. Final pass with pedantic lints
#
# Lints applied include:
# - Code clarity: needless_return, redundant_closure, unreadable_literal
# - Type safety: cast_sign_loss, cast_possible_truncation
# - Best practices: must_use_candidate, items_after_statements
# - Performance: manual_instant_elapsed, map_unwrap_or
# - Documentation: doc_markdown, missing_panics_doc
#
# Note: Check `tui/src/lib.rs` and `cmdr/src/lib.rs` to ensure these lints
# also generate warnings during normal clippy runs.
#
# Usage:
#   fish run.fish clippy
function clippy
    for folder in $workspace_folders
        cd $folder
        echo (set_color magenta)"≡ Running cargo clippy in $folder .. ≡"(set_color normal)
        cargo fix --allow-dirty --allow-staged

        # fix clippy lints.
        cargo clippy --fix --allow-dirty -- -W clippy::manual_is_multiple_of
        cargo clippy --fix --allow-dirty -- -W clippy::needless_return
        cargo clippy --fix --allow-dirty -- -W clippy::doc_markdown
        cargo clippy --fix --allow-dirty -- -W clippy::redundant_closure
        cargo clippy --fix --allow-dirty -- -W clippy::redundant_closure_for_method_calls
        cargo clippy --fix --allow-dirty -- -W clippy::cast_sign_loss
        cargo clippy --fix --allow-dirty -- -W clippy::cast_lossless
        cargo clippy --fix --allow-dirty -- -W clippy::cast_possible_truncation
        cargo clippy --fix --allow-dirty -- -W clippy::semicolon_if_nothing_returned
        cargo clippy --fix --allow-dirty -- -W clippy::must_use_candidate
        cargo clippy --fix --allow-dirty -- -W clippy::items_after_statements
        cargo clippy --fix --allow-dirty -- -W clippy::unreadable_literal
        cargo clippy --fix --allow-dirty -- -W clippy::redundant_closure
        cargo clippy --fix --allow-dirty -- -W clippy::redundant_else
        cargo clippy --fix --allow-dirty -- -W clippy::iter_without_into_iter
        cargo clippy --fix --allow-dirty -- -W clippy::explicit_iter_loop
        cargo clippy --fix --allow-dirty -- -W clippy::ignored_unit_patterns
        cargo clippy --fix --allow-dirty -- -W clippy::match_wildcard_for_single_variants
        cargo clippy --fix --allow-dirty -- -W clippy::default_trait_access
        cargo clippy --fix --allow-dirty -- -W clippy::manual_instant_elapsed
        cargo clippy --fix --allow-dirty -- -W clippy::map_unwrap_or
        cargo clippy --fix --allow-dirty -- -W clippy::missing_panics_doc
        cargo clippy --fix --allow-dirty -- -W clippy::unwrap_in_result
        cargo clippy --fix --allow-dirty -- -W clippy::unused_self
        cargo clippy --fix --allow-dirty -- -W clippy::single_char_pattern
        cargo clippy --fix --allow-dirty -- -W clippy::manual_let_else
        cargo clippy --fix --allow-dirty -- -W clippy::unnecessary_semicolon
        cargo clippy --fix --allow-dirty -- -W clippy::cast_precision_loss
        cargo clippy --fix --allow-dirty -- -W clippy::if_not_else
        cargo clippy --fix --allow-dirty -- -W clippy::unnecessary_wraps
        cargo clippy --fix --allow-dirty -- -W clippy::return_self_not_must_use
        cargo clippy --fix --allow-dirty -- -W clippy::match_bool
        cargo clippy --fix --allow-dirty -- -W clippy::comparison_chain
        cargo clippy --fix --allow-dirty -- -W clippy::elidable_lifetime_names
        cargo clippy --fix --allow-dirty -- -W clippy::wildcard_imports

        # fix rustc lints.
        cargo clippy --fix --allow-dirty -- -W unused_imports
        cargo clippy --fix --allow-dirty -- -W missing_debug_implementations

        # run rustfmt.
        cargo fmt --all

        # run clippy with pedantic lints.
        cargo clippy --fix --allow-dirty -- -W clippy::pedantic

        cd ..
    end
end

function clippy-pedantic
    # Don't use experimental linting options: -W clippy::nursery
    cargo clippy -- -W clippy::all -W clippy::pedantic >~/Downloads/clippy-fix-pedantic.rs 2>&1
    bat ~/Downloads/clippy-fix-pedantic.rs --paging=always --color=always
end

function docs
    for folder in $workspace_folders
        cd $folder
        echo (set_color magenta)"≡ Running cargo doc in $folder .. ≡"(set_color normal)
        cargo doc --no-deps --all-features
        cargo readme >README.md
        cd ..
    end
end

function serve-docs
    npm i -g serve
    serve target/doc
end

function upgrade-deps
    for folder in $workspace_folders
        cd $folder
        echo (set_color magenta)"≡ Upgrading $folder .. ≡"(set_color normal)
        cargo outdated --workspace --verbose
        cargo upgrade --verbose
        cd ..
    end
end

# Updates all installed cargo development tools to their latest versions.
#
# This function checks for and installs updates to all cargo-installed binaries
# using cargo-update (cargo install-update). It provides a simple way to keep
# development tools current with latest bug fixes and features.
#
# Features:
# - Automatic update process (uses --all flag)
# - Updates all installed cargo tools in one command
# - Only updates tools that have newer versions available
# - Provides clear feedback on update status
# - Safe to run regularly (idempotent)
#
# Tools updated include:
# - bacon: Background rust code checker
# - flamegraph & inferno: Performance profiling
# - bacon: Background task runner
# - cargo-deny: Security auditing
# - wild-linker: Fast linker
# - And all other cargo-installed tools
#
# Prerequisites:
# - cargo-update must be installed (installed via install-cargo-tools)
#
# Usage:
#   fish run.fish update-cargo-tools
#
# This command is also called automatically by:
# - rust-toolchain-update.fish (weekly systemd timer)
function update-cargo-tools
    echo (set_color cyan --bold)"Checking for cargo tool updates..."(set_color normal)
    echo ""

    # Check if cargo-update is installed
    if not command -v cargo-install-update >/dev/null
        echo (set_color red)"Error: cargo-update not installed"(set_color normal)
        echo "Run: "(set_color yellow)"fish run.fish install-cargo-tools"(set_color normal)
        return 1
    end

    # Show what needs updating
    echo (set_color magenta)"≡ Current status ≡"(set_color normal)
    cargo install-update --list

    echo ""
    echo (set_color cyan --bold)"Updating all cargo tools..."(set_color normal)

    # Update all tools
    # Note: --all updates all packages, no --force means only update if newer version exists
    if cargo install-update --all
        echo ""
        echo (set_color green)"✓ All cargo tools updated successfully!"(set_color normal)
        return 0
    else
        echo ""
        echo (set_color red)"⚠️  Some updates may have failed. Check output above."(set_color normal)
        return 1
    end
end

function rustfmt
    cargo fmt --all
end

# More info: https://github.com/EmbarkStudios/cargo-deny
# To allow exceptions, please edit the `deny.toml` file.
function audit-deps
    cargo deny check licenses advisories
end

# TUI-specific commands

# Runs TUI example programs with optional release mode and logging control.
#
# This function provides an interactive menu to select and run examples from
# the tui/examples directory. Examples are automatically discovered from both
# standalone .rs files and subdirectories.
#
# Arguments:
# - --release: Build and run in release mode for optimized performance
# - --no-log: Disable logging output
#
# Features:
# - Fuzzy search for example selection
# - Automatic example discovery
# - Support for both debug and release builds
# - Optional logging control
#
# Usage:
#   fish run.fish run-examples              # Debug mode with logging
#   fish run.fish run-examples --release    # Release mode with logging
#   fish run.fish run-examples --no-log     # Debug mode without logging
#   fish run.fish run-examples --release --no-log  # Release mode without logging
function run-examples
    set original_dir $PWD
    cd tui

    set example_binaries (get_example_binaries)

    set release false
    set no_log false

    for arg in $argv
        if test "$arg" = --release
            set release true
        else if test "$arg" = --no-log
            set no_log true
        end
    end

    run_example $example_binaries $release $no_log
    cd $original_dir
end

# Generates an SVG flamegraph for performance profiling of TUI examples.
#
# This function runs a selected example with detailed performance profiling
# and generates an interactive SVG flamegraph for visualization. Uses the
# 'profiling-detailed' cargo profile for optimal symbol resolution.
#
# Features:
# - Interactive example selection
# - Frame pointer-based call graphs
# - Enhanced symbol resolution with readable names
# - Automatic browser opening for visualization
# - Kernel parameter management for profiling
#
# Technical details:
# - Uses perf with 99Hz sampling rate
# - Forces frame pointers for accurate stack traces
# - Prevents inlining to preserve function boundaries
# - 8-level call graph depth limit
#
# Prerequisites:
# - perf installed (via setup-dev-tools.sh)
# - cargo-flamegraph installed (via install-cargo-tools)
# - sudo access for kernel parameters
#
# Output:
# - flamegraph.svg: Interactive visualization file
#
# Usage:
#   fish run.fish run-examples-flamegraph-svg
function run-examples-flamegraph-svg
    set original_dir $PWD
    cd tui

    set example_binaries (get_example_binaries)
    run_example_with_flamegraph_profiling_svg $example_binaries

    cd $original_dir
end

# Generates collapsed stack format (perf-folded) for detailed performance analysis.
#
# This function profiles a selected example and outputs the data in collapsed
# stack format, which is more compact than SVG and useful for further analysis
# or integration with other profiling tools.
#
# Features:
# - Same profiling capabilities as SVG version
# - Outputs text-based collapsed stack format
# - Shows total sample counts
# - Much smaller file size than SVG
# - Can be converted to various formats later
# - Benchmark mode for reproducible performance testing with scripted input
#
# Automated Benchmarking for Consistent Flamegraph Data:
# The --benchmark flag enables automated benchmarking using the same ex_editor
# input sequence that stress tests the rendering pipeline. This ensures that
# .perf-folded files generated are comparable across code changes.
#
# Command: ./run.fish run-examples-flamegraph-fold --benchmark
#
# Benchmark methodology:
# - 8-second continuous workload with 999 Hz sampling
# - Scripted input: pangrams, lorem ipsum, rapid cursor movements
# - ~85% active rendering time (vs 20% in interactive mode)
# - Reproducible results for performance regression analysis
# - Accurately captures rendering hot path, not initialization overhead
#
# Output format:
# Each line contains: stack_trace sample_count
# Example: main;foo;bar 42
#
# Prerequisites:
# - perf installed
# - inferno tools installed (via install-cargo-tools)
# - expect installed (via bootstrap.sh) - required for --benchmark mode
# - sudo access for kernel parameters
#
# Output:
# - flamegraph.perf-folded: Collapsed stack trace file (interactive mode)
# - flamegraph-benchmark.perf-folded: Collapsed stack trace file (benchmark mode)
#
# Usage:
#   fish run.fish run-examples-flamegraph-fold              # Interactive mode (manual input)
#   fish run.fish run-examples-flamegraph-fold --benchmark  # Benchmark mode (automated, reproducible)
function run-examples-flamegraph-fold
    set original_dir $PWD
    cd tui

    set example_binaries (get_example_binaries)

    # Check for --benchmark flag in arguments
    set benchmark_mode false
    for arg in $argv
        if test "$arg" = "--benchmark"
            set benchmark_mode true
        end
    end

    run_example_with_flamegraph_profiling_perf_fold $example_binaries $benchmark_mode

    cd $original_dir
end

# Runs performance benchmarks for the TUI crate.
#
# This function executes all benchmarks defined in the TUI crate and provides
# both real-time output and a summary of results. Benchmark results are saved
# for later analysis.
#
# Features:
# - Real-time benchmark output with tee
# - Filtered summary showing only benchmark results
# - Full output saved to ~/Downloads/bench.txt
# - Graceful Ctrl+C handling
#
# Output:
# - Console: Real-time progress and filtered results
# - File: Complete output saved to ~/Downloads/bench.txt
#
# Usage:
#   fish run.fish bench
#
# Note: Benchmarks must be marked with #[bench] attribute
function bench
    set original_dir $PWD
    cd tui

    echo "Running benchmarks and saving to "(set_color blue --bold)"~/Downloads/bench.txt"(set_color normal)"..."
    echo (set_color yellow)"Real-time output:"(set_color normal)

    if cargo bench 2>&1 | tee ~/Downloads/bench.txt
        echo ""
        echo (set_color green)"Benchmarks complete! Showing benchmark results:"(set_color normal)
        rg bench ~/Downloads/bench.txt | bat
        echo ""
        echo "Full output with compilation and test discovery saved to ~/Downloads/bench.txt"
    end

    cd $original_dir
end

# cmdr-specific commands

function run-binaries
    set original_dir $PWD
    cd cmdr

    set binaries edi giti rc
    set selection (printf '%s\n' $binaries | fzf --prompt 'Select a binary to run: ')
    set fzf_status $status

    if test $fzf_status -ne 0
        # User pressed Ctrl+C, exit gracefully
        cd $original_dir
        return
    end

    if test -n "$selection"
        cargo run --bin $selection
    end

    cd $original_dir
end

function install-cmdr
    set original_dir $PWD
    cd cmdr

    cargo install --path . --force

    cd $original_dir
end

# Builds and runs the cmdr project in a Docker container.
#
# This function provides a clean, reproducible build environment using Docker.
# It handles the complete Docker workflow including cleanup, build, and execution.
#
# Workflow:
# 1. Stops all running containers
# 2. Removes existing images
# 3. Builds fresh Docker image
# 4. Runs the container
#
# Features:
# - Complete environment isolation
# - Reproducible builds
# - Automatic cleanup of old containers/images
#
# Prerequisites:
# - Docker installed and running
# - Dockerfile present in cmdr/docker/
#
# Usage:
#   fish run.fish docker-build
function docker-build
    set original_dir $PWD
    cd cmdr/docker

    echo "Checking for running containers..."
    docker_stop_all_containers

    echo "Checking for existing images..."
    docker_remove_all_images

    echo "Building Docker image..."
    docker build -t r3bl-cmdr-install .

    echo "Running Docker container..."
    docker run r3bl-cmdr-install

    cd $original_dir
end

# Unified commands

# Monitors log files from TUI examples or cmdr binaries in real-time.
#
# This function provides intelligent log file monitoring with automatic detection
# of available log files. It uses tail -f for continuous output streaming.
#
# Features:
# - Auto-detects log files in tui/ and cmdr/ directories
# - Interactive selection when multiple logs exist
# - Real-time streaming with tail -f
# - Graceful Ctrl+C handling
# - Clear user feedback about log sources
#
# Log sources:
# - tui/log.txt: Generated by run-examples command
# - cmdr/log.txt: Generated by run-binaries command
#
# Usage:
#   fish run.fish log
#
# Prerequisites:
#   Run 'fish run.fish run-examples' or 'fish run.fish run-binaries' first to generate logs
function log
    clear

    # Check for log files in tui and cmdr directories only
    set log_locations \
        "tui/log.txt:tui directory (from run-examples)" \
        "cmdr/log.txt:cmdr directory (from run-binaries)"

    set existing_logs
    for location in $log_locations
        set location_parts (string split : $location)
        set path $location_parts[1]
        set desc $location_parts[2]

        if test -f $path
            set existing_logs $existing_logs "$path:$desc"
        end
    end

    set log_file ""
    if test (count $existing_logs) -eq 0
        # No existing logs - inform user
        echo "No log files found. Run 'fish run.fish run-examples' or 'fish run.fish run-binaries' first to generate logs."
        return
    else if test (count $existing_logs) -eq 1
        # Only one log exists - use it
        set log_parts (string split : $existing_logs[1])
        set log_file $log_parts[1]
        set log_desc $log_parts[2]
        echo (set_color magenta)"Monitoring log file: "(set_color green)"$log_file"(set_color magenta)" ($log_desc)"(set_color normal)
    else
        # Multiple logs exist - let user choose
        echo "Multiple log files found:"
        set options
        for log_info in $existing_logs
            set log_parts (string split : $log_info)
            set path $log_parts[1]
            set desc $log_parts[2]
            set options $options "$path ($desc)"
        end

        set selection (printf '%s\n' $options | fzf --prompt 'Select log file to monitor: ')
        set fzf_status $status

        if test $fzf_status -ne 0
            # User pressed Ctrl+C, exit gracefully
            return
        end

        if test -z "$selection"
            echo "No log file selected."
            return
        end

        # Extract just the path from the selection (before the first space)
        set log_file (string replace -r ' .*$' '' $selection)
        echo (set_color magenta)"Monitoring log file: "(set_color green)"$selection"(set_color normal)
    end

    if test -n "$log_file"
        tail -f -s 5 $log_file
    end
end

# Starts a 4-pane tmux development dashboard for comprehensive development monitoring.
#
# This function creates a persistent tmux session with four panes running in parallel:
#
# Layout: 2x2 grid
#   ├─ Top-left:     bacon test --headless (run all tests)
#   ├─ Top-right:    bacon doc --headless (generate documentation)
#   ├─ Bottom-left:  bacon doctests --headless (run documentation tests)
#   └─ Bottom-right: watch -n 60 ./check.fish (periodic health check every 60s)
#
# The check.fish script monitors:
# - cargo test --all-targets (all unit and integration tests)
# - cargo test --doc (documentation tests)
# - cargo doc --no-deps (documentation generation)
# - Automatic ICE (Internal Compiler Error) detection and recovery
#
# Features:
# - Session name: "r3bl-dev" (can reconnect from other terminals)
# - Headless bacon runs minimize output while providing background monitoring
# - Health check provides comprehensive status updates
# - Persistent session survives terminal disconnects
# - Interactive tmux multiplexing with customizable layouts
#
# Usage:
#   fish run.fish dev-dashboard
#
# To reconnect to an existing session:
#   tmux attach-session -t r3bl-dev
#
# To kill the session:
#   tmux kill-session -t r3bl-dev
function dev-dashboard
    fish tmux-r3bl-dev.fish
end

# Call main function with all arguments
main $argv
