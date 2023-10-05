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
        "all" => {all}
        "build" => {build}
        "clean" => {clean}
        "run" => {run}
        "test" => {test}
        "run-one-test" => {run-one-test $args}
        "help" => {print-help all}
        _ => {print $'Unknown command: (ansi red_bold)($command)(ansi reset)'}
    }
}

# Prints help for the script.
# - If "all" is passed in, prints help for all commands.
# - Otherwise, prints help for the specified command.
def print-help [command: string] {
    if $command == "all" {
        print $'Usage: (ansi blue_bold)run.nu(ansi reset) (ansi green_bold)<command>(ansi reset) (ansi yellow)[args](ansi reset)'
        print $'(ansi green_bold)<command>(ansi reset) can be:'
        print $'    (ansi green)all(ansi reset)'
        print $'    (ansi green)build(ansi reset)'
        print $'    (ansi green)clean(ansi reset)'
        print $'    (ansi green)run(ansi reset)'
        print $'    (ansi green)test(ansi reset)'
        print $'    (ansi green)run-one-test(ansi reset)'
        print $'    (ansi green)help(ansi reset)'
    } else if $command == "run-one-test" {
        print $'Usage: (ansi blue_bold)run.nu(ansi reset) (ansi green_bold)run-one-test(ansi reset) (ansi yellow)<test-name>(ansi reset)'
        print $'    (ansi green)<test-name>(ansi reset) is the name of the test to run.'
    } else {
        print $'Unknown command: (ansi red_bold)($command)(ansi reset)'
    }
}

def all [] {
    clean
    build
    test
}

def build [] {
    cargo build
}

def clean [] {
    cargo clean
}

def run [] {
    cargo run --example demo
}

def test [] {
    cargo test
}

def run-one-test [args: list<string>] {
    let num_args = $args | length

    let test_name = if $num_args == 2 {
        let test_name = $args | get 1
        $test_name
    } else {
        print-help run-one-test
        let user_input = (input "Enter the test-name: " )
        $user_input
    }

    if $test_name != "" {
        # More info on cargo test: https://doc.rust-lang.org/cargo/commands/cargo-test.html
        # More info on cargo watch: https://github.com/watchexec/cargo-watch
        let command_string = "test -- --test-threads=1 --nocapture " + $test_name
        cargo watch -x check -x $command_string -c -q -d 5
    } else {
        print $'Can not run this command without (ansi red_bold)test_name(ansi reset)'
    }
}
