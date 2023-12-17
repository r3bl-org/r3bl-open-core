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
        "install" => {install}
        "log" => {log}
        "test" => {test}
        "docs" => {docs}
        "clippy" => {clippy}
        "rustfmt" => {rustfmt}
        "watch-one-test" => {watch-one-test $args}
        "watch-macro-expansion-one-test" => {watch-macro-expansion-one-test $args}
        "run-with-flamegraph-profiling" => {run-with-flamegraph-profiling}
        "watch-run" => {watch-run}
        "watch-all-tests" => {watch-all-tests}
        "watch-clippy" => {watch-clippy}
        "serve-docs" => {serve-docs}
        "help" => {print-help all}
        _ => {print $'Unknown command: (ansi red_bold)($command)(ansi reset)'}
    }
}

def log [] {
    clear
    if ('log.txt' | path exists) {
        rm log.txt
    }
    touch log.txt
    tail -f -s 5 log.txt
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
        let command_string = "test -- --test-threads=1 --nocapture " + $test_name
        cargo watch -x check -x $command_string -c -q
    } else {
        print $'Can not run this command without (ansi red_bold)test_name(ansi reset)'
    }
}

# Watch a single test. This expects the test name to be passed in as an argument.
# If it isn't passed in, it will prompt the user for it.
def watch-macro-expansion-one-test [args: list<string>] {
    let num_args = $args | length

    let test_name = if $num_args == 2 {
        let test_name = $args | get 1
        $test_name
    } else {
        print-help watch-macro-expansion-one-test
        let user_input = (input "Enter the test-name: " )
        $user_input
    }

    if $test_name != "" {
        let command_string = "expand --test " + $test_name
        cargo watch -x $command_string -c -q -d 1
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
        print $'    (ansi green)install(ansi reset)'
        print $'    (ansi green)log(ansi reset)'
        print $'    (ansi green)run-with-flamegraph-profiling(ansi reset), (ansi blue)For more info, watch: https://www.youtube.com/watch?v=Sy26IMkOEiM(ansi reset)'
        print $'    (ansi green)watch-run(ansi reset)'
        print $'    (ansi green)test(ansi reset)'
        print $'    (ansi green)watch-one-test(ansi reset) (ansi blue_bold)<test-name>(ansi reset)'
        print $'    (ansi green)watch-all-tests(ansi reset)'
        print $'    (ansi green)clippy(ansi reset)'
        print $'    (ansi green)watch-clippy(ansi reset)'
        print $'    (ansi green)docs(ansi reset)'
        print $'    (ansi green)watch-macro-expansion-one-test(ansi reset) (ansi blue_bold)<test-name>(ansi reset)'
        print $'    (ansi green)serve-docs(ansi reset)'
        print $'    (ansi green)rustfmt(ansi reset)'
        print $'    (ansi green)help(ansi reset)'
    } else if $command == "watch-one-test" {
        print $'Usage: (ansi blue_bold)run.nu(ansi reset) (ansi green_bold)watch-one-test(ansi reset) (ansi yellow)<test-name>(ansi reset)'
        print $'    (ansi green)<test-name>(ansi reset) is the name of the test to watch.'
    } else if $command == "watch-macro-expansion-one-test" {
        print $'Usage: (ansi blue_bold)run.nu(ansi reset) (ansi green_bold)watch-macro-expansion-one-test(ansi reset) (ansi yellow)<test-name>(ansi reset)'
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
    cargo run --example main_interactive -q
}

def install [] {
    cargo install --path .
}

def run-with-flamegraph [] {
    cargo flamegraph --example main_interactive
}

def watch-run [] {
    cargo watch -- cargo run --example main_interactive
}

def test [] {
    cargo test
}

def watch-all-tests [] {
    cargo watch --exec check --exec 'test --quiet --color always -- --test-threads 1' --clear --quiet --delay 1
}

def clippy [] {
    cargo clippy --all-targets --all-features -- -D warnings
}

def watch-clippy [] {
    cargo fix --allow-dirty --allow-staged
    cargo fmt --all
    cargo watch -x 'clippy --fix --allow-dirty --allow-staged' -c -q
}

def docs [] {
    cargo doc --no-deps --all-features
}

def serve-docs [] {
    npm i -g serve
    serve ../target/doc
}

def rustfmt [] {
    cargo fmt --all
}