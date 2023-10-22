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
    "core", "macro", "redux", "tui", "tuify", "ansi_color", "simple_logger"
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
        "run" => {run}
        "run-release" => {run-release}
        "run-with-flamegraph-profiling" => {run-with-flamegraph-profiling}
        "run-with-crash-reporting" => {run-with-crash-reporting}
        "test" => {test}
        "watch-one-test" => {watch-one-test $args}
        "watch-all-tests" => {watch-all-tests}
        "watch-macro-expand-one-test" => {watch-macro-expand-one-test $args}
        "docs" => {docs}
        "check" => {check}
        "check-watch" => {check-watch}
        "clippy" => {clippy}
        "clippy-watch" => {clippy-watch}
        "rustfmt" => {rustfmt}
        "upgrade-deps" => {upgrade-deps}
        "serve-docs" => {serve-docs}
        "help" => {print-help all}
        "log" => {log}
        "check-licenses" => {check-licenses}
        _ => {print $'Unknown command: (ansi red_bold)($command)(ansi reset)'}
    }
}

def watch-macro-expand-one-test [args: list<string>] {
    let num_args = $args | length

    let test_name = if $num_args == 2 {
        let test_name = $args | get 1
        $test_name
    } else {
        print-help watch-one-test
        let user_input = (input "Enter the test-name: " )
        $user_input
    }

    if $test_name != "" {
        # More info on cargo test: https://doc.rust-lang.org/cargo/commands/cargo-test.html
        # More info on cargo watch: https://github.com/watchexec/cargo-watch
        let command_string = "expand --test " + $test_name
        cargo watch -x $command_string -c -q -d 10
        # cargo watch -x "expand --test $argv" -c -q -d 10
    } else {
        print $'Can not run this command without (ansi red_bold)test_name(ansi reset)'
    }
}

# Watch a single test. This expects the test name to be passed in as an argument.
# If it isn't passed in, it will prompt the user for it.
def watch-one-test [args: list<string>] {
    let num_args = $args | length

    let folder_name = ""
    let test_name = ""

    let folder_name = if $num_args > 1 {
        let folder_name = $args | get 1
        $folder_name
    } else {
        print-help watch-one-test
        let user_input = (input "Enter the folder-name: " )
        $user_input
    }

    let test_name = if $num_args > 2 {
        let test_name = $args | get 2
        $test_name
    } else {
        print-help watch-one-test
        let user_input = (input "Enter the test-name: " )
        $user_input
    }

    if $folder_name != "" {
        cd $folder_name
    }

    # Debug.
    # print $folder_name
    # print $test_name
    # pwd

    if $test_name != "" {
        # OG command:
        # set -l prefix "cargo watch -x check -x 'test -- --test-threads=1 --nocapture"
        # set -l middle "$argv'"
        # set -l postfix "-c -q -d 5"
        # set -l cmd "$prefix $middle $postfix"

        # More info on cargo test: https://doc.rust-lang.org/cargo/commands/cargo-test.html
        # More info on cargo watch: https://github.com/watchexec/cargo-watch
        let command_string = "test -- --test-threads=1 --nocapture " + $test_name
        cargo watch -x check -x $command_string -c -q -d 5
    } else {
        print $'Can not run this command without (ansi red_bold)test_name(ansi reset)'
    }
}

def watch-all-tests [] {
    RUST_BACKTRACE=0 cargo watch --exec check --exec 'test --quiet --color always -- --test-threads 4' --clear --quiet --delay 10
}

# Prints help for the script.
# - If "all" is passed in, prints help for all commands.
# - Otherwise, prints help for the specified command.
def print-help [command: string] {
    if $command == "all" {
        print $'Usage: (ansi magenta_bold)run.nu(ansi reset) (ansi green_bold)<command>(ansi reset) (ansi blue_bold)[args](ansi reset)'
        print $'(ansi green_bold)<command>(ansi reset) can be:'
        print $'    (ansi green)install-cargo-tools(ansi reset)'
        print $'    (ansi green)build-full(ansi reset)'
        print $'    (ansi green)build(ansi reset)'
        print $'    (ansi green)all(ansi reset)'
        print $'    (ansi green)clean(ansi reset)'
        print $'    (ansi green)docs(ansi reset)'
        print $'    (ansi green)run(ansi reset)'
        print $'    (ansi green)run-release(ansi reset)'
        print $'    (ansi green)run-with-crash-reporting(ansi reset)'
        print $'    (ansi green)run-with-flamegraph-profiling(ansi reset)'
        print $'    (ansi green)test(ansi reset)'
        print $'    (ansi green)watch-one-test(ansi reset) (ansi blue_bold)<folder-name> (ansi blue_bold)<test-name>(ansi reset)'
        print $'    (ansi green)watch-all-tests(ansi reset)'
        print $'    (ansi green)watch-macro-expand-one-test(ansi reset) (ansi blue_bold)<test-name>(ansi reset)'
        print $'    (ansi green)check(ansi reset)'
        print $'    (ansi green)check-watch(ansi reset)'
        print $'    (ansi green)clippy(ansi reset)'
        print $'    (ansi green)clippy-watch(ansi reset)'
        print $'    (ansi green)serve-docs(ansi reset)'
        print $'    (ansi green)upgrade-deps(ansi reset)'
        print $'    (ansi green)rustfmt(ansi reset)'
        print $'    (ansi green)help(ansi reset)'
    } else if $command == "watch-one-test" {
        print $'(ansi green)watch-one-test(ansi reset) (ansi blue_bold)<folder-name> (ansi blue_bold)<test-name>(ansi reset)'
        print $'    (ansi blue_bold)folder-name(ansi reset) (ansi yellow)eg: `tui`(ansi reset)'
        print $'    (ansi blue_bold)test-name(ansi reset) (ansi yellow)eg: `test_with_unicode`(ansi reset)'
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
    install-cargo-tools
    build-full
    test
    clippy
    docs
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

def run [] {
    cd tui
    cargo run --example demo
    cd ..
}

def run-release [] {
    cd tui
    cargo run --release --example demo
    cd ..
}

def run-with-flamegraph-profiling [] {
    cd tui
    cargo flamegraph --example demo
    cd ..
    google-chrome tui/flamegraph.svg
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
        cargo doc
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

def run-with-crash-reporting [] {
    cd tui
    cargo run --example demo out+err> | tee crash_log.txt
    cd ..
    code -n tui/crash_log.txt
}

def log [] {
    clear
    cd tui
    if ('log.txt' | path exists) {
        rm log.txt
    }
    touch log.txt
    tail -f -s 5 log.txt
    cd ..
}

def check-licenses [] {
    cargo deny check licenses
}
