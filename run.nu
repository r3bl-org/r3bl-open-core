#!/usr/bin/env nu

# From: https://github.com/r3bl-org/nu_script_template
#
# nu shell docs
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

let workspace_folders = [
    "core", "macro", "redux", "tui", "tuify", "ansi_color", "simple_logger", "utils", "cmdr",
]

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
        "docs" => {docs}
        "check" => {check}
        "check-watch" => {check-watch}
        "clippy" => {clippy}
        "clippy-watch" => {clippy-watch}
        "rustfmt" => {rustfmt}
        "upgrade-deps" => {upgrade-deps}
        "serve-docs" => {serve-docs}
        "help" => {print-help all}
        "audit-deps" => {audit-deps}
        _ => {print $'Unknown command: (ansi red_bold)($command)(ansi reset)'}
    }
}


# Prints help for the script.
# - If "all" is passed in, prints help for all commands.
# - Otherwise, prints help for the specified command.
def print-help [command: string] {
    if $command == "all" {
        print $'Usage: (ansi magenta_bold)run.nu(ansi reset) (ansi green_bold)<command>(ansi reset) (ansi blue_bold)[args](ansi reset)'
        print $'(ansi green_bold)<command>(ansi reset) can be:'
        print $'    (ansi green)all(ansi reset)'
        print $'    (ansi green)build(ansi reset)'
        print $'    (ansi green)build-full(ansi reset)'
        print $'    (ansi green)clean(ansi reset)'
        print $'    (ansi green)install-cargo-tools(ansi reset)'
        print $'    (ansi green)test(ansi reset)'
        print $'    (ansi green)docs(ansi reset)'
        print $'    (ansi green)check(ansi reset)'
        print $'    (ansi green)check-watch(ansi reset)'
        print $'    (ansi green)clippy(ansi reset)'
        print $'    (ansi green)clippy-watch(ansi reset)'
        print $'    (ansi green)serve-docs(ansi reset)'
        print $'    (ansi green)upgrade-deps(ansi reset)'
        print $'    (ansi green)rustfmt(ansi reset)'
        print $'    (ansi green)help(ansi reset)'
    } else {
        print $'Unknown command: (ansi red_bold)($command)(ansi reset)'
    }
}

def install-cargo-tools [] {
    cargo install bacon
    cargo install cargo-workspaces
    cargo install cargo-cache
    cargo install cargo-watch
    cargo install flamegraph
    cargo install cargo-outdated
    cargo install cargo-update
    cargo install cargo-deny
}

def all [] {
    cargo install cargo-deny
    cargo build
    test
    clippy
    docs
    audit-deps
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

def check-watch [] {
    cargo watch -x 'check --workspace'
}

def clippy [] {
    for folder in ($workspace_folders) {
        cd $folder
        print $'(ansi magenta)≡ Running cargo clippy in ($folder) .. ≡(ansi reset)'
        cargo fix --allow-dirty --allow-staged
        cargo fmt --all
        cd ..
    }
}

def clippy-watch [] {
    cargo fix --allow-dirty --allow-staged
    cargo fmt --all
    cargo watch -x 'clippy --fix --allow-dirty --allow-staged' -c -q
}

def docs [] {
    for folder in $workspace_folders {
        cd $folder
        print $'(ansi magenta)≡ Running cargo doc in ($folder) .. ≡(ansi reset)'
        cargo doc --no-deps --all-features
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
    for folder in $workspace_folders {
        cd $folder
        print $'(ansi magenta)≡ Running cargo fmt --all in ($folder) .. ≡(ansi reset)'
        cargo fmt --all
        cd ..
    }
}

# More info: https://github.com/EmbarkStudios/cargo-deny
def audit-deps [] {
    cargo deny check licenses advisories
}
