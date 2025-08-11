#!/usr/bin/env nu

# From: https://github.com/r3bl-org/nu_script_template
#
# nushell docs
# - language fundamentals: https://www.nushell.sh/book/custom_commands.html#sub-commands
#     - strings: https://www.nushell.sh/book/working_with_strings.html#string-interpolation
#     - functions (aka commands, subcommands): https://www.nushell.sh/book/custom_commands.html
#     - variables: https://www.nushell.sh/book/variables_and_subexpressions.html
# - built-in command reference: https://www.nushell.sh/commands/#command-reference
#     - length: https://www.nushell.sh/commands/docs/length.html
#     - ansi: https://www.nushell.sh/commands/docs/ansi.html
#     - print: https://www.nushell.sh/commands/docs/print.html
#     - input: https://www.nushell.sh/commands/docs/input.html
#     - input list: https://www.nushell.sh/commands/docs/input_list.html
#     - match: https://www.nushell.sh/commands/docs/match.html
# - mental model
#     - coming from bash: https://www.nushell.sh/book/coming_from_bash.html
#     - thinking in nu: https://www.nushell.sh/book/thinking_in_nu.html

source script_lib.nu

# Make sure to keep this in sync with `Cargo.toml` workspace members.
let workspace_folders = (get_cargo_projects)

# Main entry point for the script.
def main [...args: string] {
    let num_args = $args | length

    if $num_args == 0 {
        print-help all
        return
    }

    let command = $args | get 0

    match $command {
        "all" => {all}
        "build" => {build}
        "build-full" => {build-full}
        "clean" => {clean}
        "install-cargo-tools" => {install-cargo-tools}
        "test" => {test}
        "watch-all-tests" => {watch-all-tests}
        "watch-one-test" => {
            if ($args | length) > 1 {
                watch-one-test ($args | get 1)
            } else {
                print $'Error: watch-one-test requires a test pattern'
                print $'Usage: nu run.nu watch-one-test <pattern>'
            }
        }
        "watch-clippy" => {watch-clippy}
        "watch-check" => {watch-check}
        "docs" => {docs}
        "check" => {check}
        "clippy" => {clippy}
        "clippy-pedantic" => {clippy-pedantic}
        "rustfmt" => {rustfmt}
        "upgrade-deps" => {upgrade-deps}
        "serve-docs" => {serve-docs}
        "audit-deps" => {audit-deps}
        "unmaintained" => {unmaintained}
        "help" => {print-help all}
        "build-server" => {build-server}
        # TUI-specific commands
        "run-examples" => {run-examples ...$args}
        "run-examples-flamegraph-svg" => {run-examples-flamegraph-svg}
        "run-examples-flamegraph-fold" => {run-examples-flamegraph-fold}
        "bench" => {bench}
        # cmdr-specific commands
        "run-binaries" => {run-binaries}
        "install-cmdr" => {install-cmdr}
        "docker-build" => {docker-build}
        # Unified commands
        "log" => {log}
        _ => {print $'Unknown command: (ansi red_bold)($command)(ansi reset)'}
    }
}


# Prints help for the script.
# - If "all" is passed in, prints help for all commands.
# - Otherwise, prints help for the specified command.
def print-help [command: string] {
    if $command == "all" {
        print $'Usage: (ansi magenta_bold)run(ansi reset) (ansi green_bold)<command>(ansi reset) (ansi blue_bold)[args](ansi reset)'
        print $''
        print $'(ansi cyan_bold)Workspace-wide commands:(ansi reset)'
        print $'    (ansi green)all(ansi reset)                  Run all major checks'
        print $'    (ansi green)build(ansi reset)                Build entire workspace'
        print $'    (ansi green)build-full(ansi reset)           Full build with clean and update'
        print $'    (ansi green)clean(ansi reset)                Clean entire workspace'
        print $'    (ansi green)test(ansi reset)                 Test entire workspace'
        print $'    (ansi green)check(ansi reset)                Check all workspaces'
        print $'    (ansi green)clippy(ansi reset)               Run clippy on all workspaces'
        print $'    (ansi green)clippy-pedantic(ansi reset)      Run clippy with pedantic lints'
        print $'    (ansi green)docs(ansi reset)                 Generate docs for all'
        print $'    (ansi green)serve-docs(ansi reset)           Serve documentation'
        print $'    (ansi green)rustfmt(ansi reset)              Format all code'
        print $'    (ansi green)install-cargo-tools(ansi reset)  Install development tools'
        print $'    (ansi green)upgrade-deps(ansi reset)         Upgrade dependencies'
        print $'    (ansi green)audit-deps(ansi reset)           Security audit'
        print $'    (ansi green)unmaintained(ansi reset)         Check for unmaintained deps'
        print $'    (ansi green)build-server(ansi reset)         Remote build server - uses rsync'
        print $''
        print $'(ansi cyan_bold)Watch commands:(ansi reset)'
        print $'    (ansi green)watch-all-tests(ansi reset)      Watch files, run all tests'
        print $'    (ansi green)watch-one-test(ansi reset) (ansi blue)[pattern](ansi reset)  Watch files, run specific test'
        print $'    (ansi green)watch-clippy(ansi reset)         Watch files, run clippy'
        print $'    (ansi green)watch-check(ansi reset)          Watch files, run cargo check'
        print $''
        print $'(ansi cyan_bold)TUI-specific commands:(ansi reset)'
        print $'    (ansi green)run-examples(ansi reset) (ansi blue)[--release] [--no-log](ansi reset)  Run TUI examples'
        print $'    (ansi green)run-examples-flamegraph-svg(ansi reset)  Generate SVG flamegraph'
        print $'    (ansi green)run-examples-flamegraph-fold(ansi reset) Generate perf-folded format'
        print $'    (ansi green)bench(ansi reset)                Run benchmarks'
        print $''
        print $'(ansi cyan_bold)cmdr-specific commands:(ansi reset)'
        print $'    (ansi green)run-binaries(ansi reset)         Run edi, giti, or rc'
        print $'    (ansi green)install-cmdr(ansi reset)         Install cmdr binaries'
        print $'    (ansi green)docker-build(ansi reset)         Build release in Docker'
        print $''
        print $'(ansi cyan_bold)Other commands:(ansi reset)'
        print $'    (ansi green)log(ansi reset)                  Monitor log.txt in cmdr or tui directory'
        print $'    (ansi green)help(ansi reset)                 Show this help'
    } else {
        print $'Unknown command: (ansi red_bold)($command)(ansi reset)'
    }
}

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
#   nu run.nu build-server
#
# Then on the remote server, run:
#   bacon nextest -W
#   bacon doc -W
#   bacon clippy -W
def build-server [] {
    # Where you source files live.
    let orig_path = "/home/nazmul/github/r3bl-open-core/"
    # Copy files to here.
    let dest_host = "nazmul-laptop.local"
    let dest_path = $"($dest_host):($orig_path)"

    # Function to run rsync. Simple.
    def run_rsync [] {
        print $'(ansi cyan_underline)Changes detected, running rsync(ansi reset)'

        let crlf = (char newline)
        let prefix = "┤ "
        let hanging_prefix = "├ "
        let tab = (char tab)
        let msg_1 = $'(ansi yellow)($orig_path)(ansi reset)'
        let msg_2 = $'(ansi blue)($dest_path)(ansi reset)'
        print $"($prefix)from:($tab)($msg_1)($crlf)($hanging_prefix)to:($tab)($msg_2)"
        ^rsync -q -avz --delete --exclude 'target/' $orig_path $dest_path

        print $'(ansi cyan_underline)Rsync complete(ansi reset)'
    }

    # Main loop
    loop {
        # Construct the inotifywait command with all directories.
        let inotify_command = "inotifywait"
        let inotify_args = [
            "-r",
            "-e", "modify",
            "-e", "create",
            "-e", "delete",
            "-e", "move" ,
            "--exclude", "target" ,
            $orig_path,
        ]

        print $'(ansi green_bold) (ansi yellow)($inotify_command)(ansi reset) (ansi blue)($inotify_args)(ansi reset)'

        print $'(ansi cyan)❪◕‿◕❫ (ansi reset)(ansi green)Please run bacon on build server: (ansi reset)(ansi yellow_underline)bacon nextest -W(ansi reset), (ansi yellow_underline)bacon doc -W(ansi reset), (ansi yellow_underline)bacon clippy -W(ansi reset)'

        # Execute the inotifywait command with proper Ctrl+C handling
        let watch_result = try {
            ^$inotify_command ...$inotify_args
        } catch {
            # User pressed Ctrl+C, exit gracefully
            return
        }

        # Run rsync
        run_rsync
    }
}

# Watch commands - using watch-files from script_lib.nu
def watch-all-tests [] { watch-files "cargo test --workspace --quiet -- --test-threads 4" }
def watch-one-test [pattern: string] { watch-files $"cargo test ($pattern) -- --nocapture --test-threads=1" }
def watch-clippy [] { watch-files "cargo clippy --workspace --all-targets --all-features" }
def watch-check [] { watch-files "cargo check --workspace" }


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
# - bootstrap.sh must be run first to install Rust, Cargo, and Nushell
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
#   nu run.nu install-cargo-tools
def install-cargo-tools [] {
    # Install uv package manager (required for Serena semantic code MCP server.
    # https://github.com/oraios/serena
    if (which uv | is-empty) {
        print 'Installing uv...'
        if ($nu.os-info.name == "windows") {
            # Windows installation
            ^powershell -c "irm https://astral.sh/uv/install.ps1 | iex"
        } else {
            # Linux and macOS installation
            ^sh -c "curl -LsSf https://astral.sh/uv/install.sh | sh"
        }
    } else {
        print '✓ uv installed'
    }

    # Cargo tools - some tools install binaries with different names
    let cargo_tools = [
        {name: "bacon", check: "bacon", install: "cargo install bacon"},
        {name: "cargo-workspaces", check: "cargo-workspaces", install: "cargo install cargo-workspaces"},
        {name: "cargo-cache", check: "cargo-cache", install: "cargo install cargo-cache"},
        {name: "cargo-outdated", check: "cargo-outdated", install: "cargo install cargo-outdated"},
        {name: "cargo-update", check: "cargo-update", install: "cargo install cargo-update"},
        {name: "cargo-deny", check: "cargo-deny", install: "cargo install cargo-deny"},
        {name: "cargo-unmaintained", check: "cargo-unmaintained", install: "cargo install cargo-unmaintained"},
        {name: "cargo-expand", check: "cargo-expand", install: "cargo install cargo-expand"},
        {name: "cargo-readme", check: "cargo-readme", install: "cargo install cargo-readme"},
        {name: "cargo-nextest", check: "cargo-nextest", install: "cargo install cargo-nextest"},
        {name: "flamegraph", check: "cargo-flamegraph", install: "cargo install flamegraph"},
        {name: "inferno", check: "inferno-flamegraph", install: "cargo install inferno"},
        {name: "sccache", check: "sccache", install: "cargo install sccache --locked"}
    ]

    $cargo_tools | each {|tool| install_if_missing $tool.check $tool.install}

    # Rust components
    if (rustup component list --installed | str contains "rust-analyzer" | not $in) {
        print 'Installing rust-analyzer...'; rustup component add rust-analyzer
    } else { print '✓ rust-analyzer installed' }

    # System tools (detect package manager)
    # cspell:disable
    let pkg_mgr = get_package_manager
    # cspell:enable

    # Install go and docker if not already installed.
    if ($pkg_mgr != null) {
        install_if_missing "docker" $"($pkg_mgr) docker.io docker-compose"
        install_if_missing "go" $"($pkg_mgr) golang-go"
    }

    # Install other tools.

    # Install claude using script rather than npm
    # https://docs.anthropic.com/en/docs/claude-code/setup#native-binary-installation-beta
    install_if_missing "claude" "curl -fsSL https://claude.ai/install.sh | sh"

    # Install go and mcp-language-server if not already installed
    # https://github.com/isaacphi/mcp-language-server
    if (which go | is-not-empty) {
        let mcp_path = $"($env.HOME)/go/bin/mcp-language-server"
        # cspell:disable
        if not ($mcp_path | path exists) {
            print 'Installing mcp-language-server...'
            go install github.com/isaacphi/mcp-language-server@latest
        } else {
            print '✓ mcp-language-server installed'
        }
        # cspell:enable
    }

    # Configure claude MCP servers.
    # 1. Configure claude w/ mcp-language-server to use rust-analyzer if available.
    # 2. Add context7 MCP server.
    # 3. Add serena MCP server.
    if (which claude | is-not-empty) {
        try {
            print 'Configuring claude MCP servers...'
            let workspace = $env.PWD
            let mcp_cmd = $"($env.HOME)/go/bin/mcp-language-server"
            claude mcp add-json "rust-analyzer" $'{"type":"stdio","command":"($mcp_cmd)","args":["--workspace","($workspace)","--lsp","rust-analyzer"],"cwd":"($workspace)"}'
            claude mcp add-json "context7" '{"type":"http","url":"https://mcp.context7.com/mcp"}'
            claude mcp add serena -- uvx --from git+https://github.com/oraios/serena serena start-mcp-server --context ide-assistant --project /home/nazmul/github/r3bl-open-core
        }
    }
}

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
#   nu run.nu all
def all [] {
    # Installs and runs all major checks and tools for the workspace.
    install-cargo-tools
    build
    test
    clippy
    docs
    audit-deps
    unmaintained
    rustfmt
}

# https://github.com/trailofbits/cargo-unmaintained
# This is very slow to run.
def unmaintained [] {
    cargo unmaintained --color always --fail-fast --tree --verbose
}

def build [] {
    cargo build
}

def build-full [] {
    install-cargo-tools
    cargo cache -r all
    cargo clean
    cargo +nightly update
    cargo build
}

def clean [] {
    cargo clean
}

def test [] {
    for folder in ($workspace_folders) {
        cd $folder
        print $'(ansi magenta)≡ Running tests in ($folder) .. ≡(ansi reset)'
        cargo test -q -- --test-threads=20
        cd ..
    }
}

def check [] {
    cargo check --workspace
}

# Replaced by watch-check command above

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
#   nu run.nu clippy
def clippy [] {
    for folder in ($workspace_folders) {
        cd $folder
        print $'(ansi magenta)≡ Running cargo clippy in ($folder) .. ≡(ansi reset)'
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
    }
}

def clippy-pedantic [] {
    # Don't use experimental linting options: -W clippy::nursery
    cargo clippy -- -W clippy::all -W clippy::pedantic out+err> ~/Downloads/clippy-fix-pedantic.rs
    bat ~/Downloads/clippy-fix-pedantic.rs --paging=always --color=always
}

# Replaced by watch-clippy command above

def docs [] {
    for folder in $workspace_folders {
        cd $folder
        print $'(ansi magenta)≡ Running cargo doc in ($folder) .. ≡(ansi reset)'
        cargo doc --no-deps --all-features
        # Output redirection in Nushell:
        # https://stackoverflow.com/questions/76403457/how-to-redirect-stdout-to-a-file-in-nushell
        cargo readme out> README.md
        cd ..
    }
}

def serve-docs [] {
    npm i -g serve
    serve target/doc
}

def upgrade-deps [] {
    for folder in $workspace_folders {
        cd $folder
        print $'(ansi magenta)≡ Upgrading ($folder) .. ≡(ansi reset)'
        cargo outdated --workspace --verbose
        cargo upgrade --verbose
        cd ..
    }
}

def rustfmt [] {
    cargo fmt --all
    # for folder in $workspace_folders {
    #     cd $folder
    #     print $'(ansi magenta)≡ Running cargo fmt --all in ($folder) .. ≡(ansi reset)'
    #     cargo fmt --all
    #     cd ..
    # }
}

# More info: https://github.com/EmbarkStudios/cargo-deny
# To allow exceptions, please edit the `deny.toml` file.
def audit-deps [] {
    cargo deny check licenses advisories
}

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
#   nu run.nu run-examples              # Debug mode with logging
#   nu run.nu run-examples --release    # Release mode with logging
#   nu run.nu run-examples --no-log     # Debug mode without logging
#   nu run.nu run-examples --release --no-log  # Release mode without logging
def run-examples [...args: string] {
    let original_dir = $env.PWD
    cd tui

    let example_binaries: list<string> = get_example_binaries

    let release = ($args | any {|arg| $arg == "--release"})
    let no_log = ($args | any {|arg| $arg == "--no-log"})

    run_example $example_binaries $release $no_log
    cd $original_dir
}

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
#   nu run.nu run-examples-flamegraph-svg
def run-examples-flamegraph-svg [] {
    let original_dir = $env.PWD
    cd tui

    let example_binaries: list<string> = get_example_binaries
    run_example_with_flamegraph_profiling_svg $example_binaries

    cd $original_dir
}

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
#   nu run.nu run-examples-flamegraph-fold
def run-examples-flamegraph-fold [] {
    let original_dir = $env.PWD
    cd tui

    let example_binaries: list<string> = get_example_binaries
    run_example_with_flamegraph_profiling_perf_fold $example_binaries

    cd $original_dir
}

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
#   nu run.nu bench
#
# Note: Benchmarks must be marked with #[bench] attribute
def bench [] {
    let original_dir = $env.PWD
    cd tui

    print $'Running benchmarks and saving to (ansi blue_bold)~/Downloads/bench.txt(ansi reset)...'
    print $'(ansi yellow)Real-time output:(ansi reset)'
    try {
        ^cargo bench out+err>| ^tee ~/Downloads/bench.txt
        print $''
        print $'(ansi green)Benchmarks complete! Showing benchmark results:(ansi reset)'
        cat ~/Downloads/bench.txt | rg bench | bat
        print $''
        print "Full output with compilation and test discovery saved to ~/Downloads/bench.txt"
    } catch {
        # Silently handle Ctrl+C interruption
        null
    }

    cd $original_dir
}


# cmdr-specific commands

def run-binaries [] {
    let original_dir = $env.PWD
    cd cmdr

    let binaries = ["edi", "giti", "rc"]
    let selection = try {
        $binaries | input list --fuzzy 'Select a binary to run:'
    } catch {
        # User pressed Ctrl+C, exit gracefully
        cd $original_dir
        return
    }

    if $selection != null and $selection != "" {
        cargo run --bin $selection
    }

    cd $original_dir
}

def install-cmdr [] {
    let original_dir = $env.PWD
    cd cmdr

    cargo install --path . --force

    cd $original_dir
}

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
#   nu run.nu docker-build
def docker-build [] {
    let original_dir = $env.PWD
    cd cmdr/docker

    print "Checking for running containers..."
    docker_stop_all_containers

    print "Checking for existing images..."
    docker_remove_all_images

    print "Building Docker image..."
    ^docker build -t r3bl-cmdr-install .

    print "Running Docker container..."
    ^docker run r3bl-cmdr-install

    cd $original_dir
}

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
#   nu run.nu log
#
# Prerequisites:
#   Run 'nu run.nu run-examples' or 'nu run.nu run-binaries' first to generate logs
def log [] {
    clear

    # Check for log files in tui and cmdr directories only
    let log_locations = [
        {path: "tui/log.txt", desc: "tui directory (from run-examples)"},
        {path: "cmdr/log.txt", desc: "cmdr directory (from run-binaries)"}
    ]

    let existing_logs = $log_locations | where ($it.path | path exists)

    let log_file = if ($existing_logs | length) == 0 {
        # No existing logs - inform user
        print "No log files found. Run 'nu run.nu run-examples' or 'nu run.nu run-binaries' first to generate logs."
        return
    } else if ($existing_logs | length) == 1 {
        # Only one log exists - use it
        let log = $existing_logs | first
        print $"(ansi magenta)Monitoring log file: (ansi green)($log.path)(ansi magenta) \(($log.desc)\)(ansi reset)"
        $log.path
    } else {
        # Multiple logs exist - let user choose
        print "Multiple log files found:"
        let options = $existing_logs | each {|log| $"($log.path) \(($log.desc)\)"}
        let selection = try {
            $options | input list --fuzzy 'Select log file to monitor:'
        } catch {
            # User pressed Ctrl+C, exit gracefully
            return
        }

        if ($selection == "") or ($selection == null) {
            print "No log file selected."
            return
        }

        # Extract just the path from the selection (before the first space)
        let log_path = $selection | split row " " | first
        print $"(ansi magenta)Monitoring log file: (ansi green)($selection)(ansi reset)"
        $log_path
    }

    try {
        ^tail -f -s 5 $log_file
        null
    } catch {
        # Silently handle Ctrl+C interruption
        null
    }
}
