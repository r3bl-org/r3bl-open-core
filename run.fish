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
        case serve-docs
            serve-docs
        case audit-deps
            audit-deps
        case unmaintained
            unmaintained
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
            run-examples-flamegraph-fold
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
        echo "    "(set_color green)"audit-deps"(set_color normal)"           Security audit"
        echo "    "(set_color green)"unmaintained"(set_color normal)"         Check for unmaintained deps"
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
        echo "    "(set_color green)"run-examples-flamegraph-fold"(set_color normal)" Generate perf-folded format"
        echo "    "(set_color green)"bench"(set_color normal)"                Run benchmarks"
        echo ""
        echo (set_color cyan --bold)"cmdr-specific commands:"(set_color normal)
        echo "    "(set_color green)"run-binaries"(set_color normal)"         Run edi, giti, or rc"
        echo "    "(set_color green)"install-cmdr"(set_color normal)"         Install cmdr binaries"
        echo "    "(set_color green)"docker-build"(set_color normal)"         Build release in Docker"
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
#   bacon nextest -W
#   bacon doc -W
#   bacon clippy -W
function build-server
    # Where you source files live.
    set orig_path /home/nazmul/github/r3bl-open-core/
    # Copy files to here.
    set dest_host "nazmul-laptop.local"
    set dest_path "$dest_host:$orig_path"

    # Function to run rsync. Simple.
    function run_rsync
        echo (set_color cyan --underline)"Changes detected, running rsync"(set_color normal)

        set prefix "┤ "
        set hanging_prefix "├ "
        set tab "\t"
        set msg_1 (set_color yellow)"$orig_path"(set_color normal)
        set msg_2 (set_color blue)"$dest_path"(set_color normal)
        echo "$prefix"from:"$tab$msg_1"
        echo "$hanging_prefix"to:"$tab$msg_2"
        rsync -q -avz --delete --exclude target/ $orig_path $dest_path

        echo (set_color cyan --underline)"Rsync complete"(set_color normal)
    end

    # Main loop
    while true
        # Construct the inotifywait command with all directories.
        set inotify_command inotifywait
        set inotify_args -r -e modify -e create -e delete -e move --exclude target $orig_path

        echo (set_color green --bold)" "(set_color yellow)"$inotify_command"(set_color normal)" "(set_color blue)"$inotify_args"(set_color normal)

        echo (set_color cyan)"❪◕‿◕❫ "(set_color normal)(set_color green)"Please run bacon on build server: "(set_color normal)(set_color yellow --underline)"bacon nextest -W"(set_color normal)", "(set_color yellow --underline)"bacon doc -W"(set_color normal)", "(set_color yellow --underline)"bacon clippy -W"(set_color normal)

        # Execute the inotifywait command with proper Ctrl+C handling
        if not $inotify_command $inotify_args >/dev/null 2>&1
            # User pressed Ctrl+C, exit gracefully
            return
        end

        # Run rsync
        run_rsync
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
# 1. Cargo tools for Rust development (bacon, nextest, flamegraph, etc.)
# 2. System tools (docker, go)
# 3. Claude Code CLI with MCP servers
# 4. Language servers (rust-analyzer)
#
# Features:
# - Cross-platform support (Windows, macOS, Linux)
# - Automatic package manager detection
# - Idempotent - safe to run multiple times
# - Configures Claude MCP servers for enhanced code assistance:
#   - rust-analyzer: Language server protocol for Rust
#   - context7: Documentation lookup service
#   - serena: Semantic code analysis
#
# Prerequisites:
# - bootstrap.sh must be run first to install Rust, Cargo, and Fish
#
# Tools installed:
# - bacon: Background rust code checker
# - cargo-nextest: Next-generation test runner
# - flamegraph: Performance profiling visualization
# - sccache: Shared compilation cache for faster builds
# - cargo-deny: Supply chain security auditing
# - cargo-outdated: Dependency version checker
# - And many more...
#
# Usage:
#   fish run.fish install-cargo-tools
function install-cargo-tools
    # Install uv package manager (required for Serena semantic code MCP server.
    # https://github.com/oraios/serena
    # https://claudelog.com/addons/serena/
    if not command -v uv >/dev/null
        echo 'Installing uv...'
        if test (uname) = Windows_NT
            # Windows installation
            powershell -c "irm https://astral.sh/uv/install.ps1 | iex"
        else
            # Linux and macOS installation
            sh -c "curl -LsSf https://astral.sh/uv/install.sh | sh"
        end
    else
        echo '✓ uv installed'
    end

    # Cargo tools - some tools install binaries with different names
    set cargo_tools \
        "bacon:bacon:cargo install bacon" \
        "cargo-workspaces:cargo-workspaces:cargo install cargo-workspaces" \
        "cargo-cache:cargo-cache:cargo install cargo-cache" \
        "cargo-outdated:cargo-outdated:cargo install cargo-outdated" \
        "cargo-update:cargo-update:cargo install cargo-update" \
        "cargo-deny:cargo-deny:cargo install cargo-deny" \
        "cargo-unmaintained:cargo-unmaintained:cargo install cargo-unmaintained" \
        "cargo-expand:cargo-expand:cargo install cargo-expand" \
        "cargo-readme:cargo-readme:cargo install cargo-readme" \
        "cargo-nextest:cargo-nextest:cargo install cargo-nextest" \
        "flamegraph:cargo-flamegraph:cargo install flamegraph" \
        "inferno:inferno-flamegraph:cargo install inferno" \
        "sccache:sccache:cargo install sccache --locked"

    for tool_info in $cargo_tools
        set tool_parts (string split : $tool_info)
        set name $tool_parts[1]
        set check $tool_parts[2]
        set install $tool_parts[3]
        install_if_missing $check $install
    end

    # Rust components
    if not rustup component list --installed | grep -q rust-analyzer
        echo 'Installing rust-analyzer...'
        rustup component add rust-analyzer
    else
        echo '✓ rust-analyzer installed'
    end

    # System tools (detect package manager)
    set pkg_mgr (get_package_manager)

    # Install go and docker if not already installed.
    if test -n "$pkg_mgr"
        install_if_missing docker "$pkg_mgr docker.io docker-compose"
        install_if_missing go "$pkg_mgr golang-go"
    end

    # Install other tools.

    # Install claude using script rather than npm
    # https://docs.anthropic.com/en/docs/claude-code/setup#native-binary-installation-beta
    install_if_missing claude "curl -fsSL https://claude.ai/install.sh | sh"

    # Configure claude MCP servers.
    # 1. Configure claude w/ mcp-language-server to use rust-analyzer if available.
    # 2. Add context7 MCP server.
    # 3. Add serena MCP server.
    if command -v claude >/dev/null
        echo 'Configuring claude MCP servers...'
        set workspace $PWD
        set mcp_cmd "$HOME/go/bin/mcp-language-server"
        claude mcp add-json context7 '{"type":"http","url":"https://mcp.context7.com/mcp"}'
        claude mcp add serena -- uvx --from git+https://github.com/oraios/serena serena start-mcp-server --context ide-assistant --project /home/nazmul/github/r3bl-open-core 2>/dev/null || true
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
    unmaintained
    rustfmt
end

# https://github.com/trailofbits/cargo-unmaintained
# This is very slow to run.
function unmaintained
    cargo unmaintained --color always --fail-fast --tree --verbose
end

function build
    cargo build
end

function build-full
    install-cargo-tools
    cargo cache -r all
    cargo clean
    cargo +nightly update
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
#
# Output format:
# Each line contains: stack_trace sample_count
# Example: main;foo;bar 42
#
# Prerequisites:
# - perf installed
# - inferno tools installed (via install-cargo-tools)
# - sudo access for kernel parameters
#
# Output:
# - flamegraph.perf-folded: Collapsed stack trace file
#
# Usage:
#   fish run.fish run-examples-flamegraph-fold
function run-examples-flamegraph-fold
    set original_dir $PWD
    cd tui

    set example_binaries (get_example_binaries)
    run_example_with_flamegraph_profiling_perf_fold $example_binaries

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

# Call main function with all arguments
main $argv
