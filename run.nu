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

# Main entry point for the script.
def main [...args: string] {
    let num_args = $args | length

    if $num_args == 0 {
        print-help all
        return
    }

    let command = $args | get 0

    match $command {
        "build" => {build}
        "clean" => {clean}
        "run" => {run}
        "test" => {test}
        "docs" => {docs}
        "clippy" => {clippy}
        "rustfmt" => {rustfmt}
        "watch-one-test" => {watch-one-test $args}
        "run-with-flamegraph-profiling" => {run-with-flamegraph-profiling}
        "upgrade-deps" => {upgrade-deps}
        "serve-docs" => {serve-docs}
        "help" => {print-help all}
        "run-release" => {run-release}
        "log" => {log}
        "run-with-crash-reporting" => {run-with-crash-reporting}
        "check-licenses" => {check-licenses}
        _ => {print $'Unknown command: (ansi red_bold)($command)(ansi reset)'}
    }
}

# Watch a single test. This expects the test name to be passed in as an argument.
# If it isn't passed in, it will prompt the user for it.
def watch-one-test [args: list<string>] {
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
        let command_string = "test -- --show-output --test-threads 4" + $test_name
        cargo watch -x check -x $command_string -c -q --delay 10
    } else {
        print $'Can not run this command without (ansi red_bold)test_name(ansi reset)'
    }
}

# Prints help for the script.
# - If "all" is passed in, prints help for all commands.
# - Otherwise, prints help for the specified command.
def print-help [command: string] {
    if $command == "all" {
        print $'Usage: (ansi blue_bold)run.nu(ansi reset) (ansi green_bold)<command>(ansi reset) (ansi yellow)[args](ansi reset)'
        print $'(ansi green_bold)<command>(ansi reset) can be:'
        print $'    (ansi green)build(ansi reset)'
        print $'    (ansi green)clean(ansi reset)'
        print $'    (ansi green)run(ansi reset)'
        print $'    (ansi green)run-with-flamegraph-profiling(ansi reset)'
        print $'    (ansi green)test(ansi reset)'
        print $'    (ansi green)watch-one-test(ansi reset) (ansi blue_bold)<test-name>(ansi reset)'
        print $'    (ansi green)clippy(ansi reset)'
        print $'    (ansi green)docs(ansi reset)'
        print $'    (ansi green)serve-docs(ansi reset)'
        print $'    (ansi green)upgrade-deps(ansi reset)'
        print $'    (ansi green)rustfmt(ansi reset)'
        print $'    (ansi green)help(ansi reset)'
    } else if $command == "watch-one-test" {
        print $'Usage: (ansi blue_bold)run.nu(ansi reset) (ansi green_bold)watch-one-test(ansi reset) (ansi yellow)<test-name>(ansi reset)'
        print $'    (ansi green)<test-name>(ansi reset) is the name of the test to watch.'
    } else {
        print $'Unknown command: (ansi red_bold)($command)(ansi reset)'
    }
}

def build [] {
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

def run-with-flamegraph [] {
    cd tui
    cargo flamegraph --example demo
    cd ..
}

def test [] {
    let folders = ["core", "macro", "redux", "tui"]
    for folder in ($folders) {
        cd $folder
        print $'(ansi magenta)≡ Running tests in ($folder) .. ≡(ansi reset)'
        cargo test -q -- --test-threads=4
        cd ..
    }
}

def clippy [] {
    cargo fix --allow-dirty --allow-staged
    cargo fmt --all
    cargo watch -x 'clippy --fix --allow-dirty --allow-staged' -c -q
}

def docs [] {
    let folders = ["core", "macro", "redux", "tui"]
    for folder in $folders {
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
    let folders = ["core", "macro", "redux", "tui"]
    for folder in $folders {
        cd $folder
        print $'(ansi magenta)≡ Upgrading ($folder) .. ≡(ansi reset)'
        cargo outdated --workspace --verbose
        cargo upgrade --to-lockfile --verbose
        cargo update
        cd ..
    }
}

def rustfmt [] {
    let folders = ["core", "macro", "redux", "tui"]
    for folder in $folders {
        cd $folder
        print $'(ansi magenta)≡ Running cargo fmt --all in ($folder) .. ≡(ansi reset)'
        cargo fmt --all
        cd ..
    }
}

def run-release [] {
    cd tui
    cargo run --release --example demo
    cd ..
}

def run-with-crash-reporting [] {
    cd tui
    cargo run --example demo out+err> | tee crash_log.txt
    cd ..
}

def log [] {
    clear
    cd tui
    tail -f -s 5 log.txt
    rm log.txt
    touch log.txt
    cd ..
}

def check-licenses [] {
    cargo install cargo-deny
    cargo deny check licenses
}
