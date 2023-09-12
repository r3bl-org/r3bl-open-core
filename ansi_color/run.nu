#!/usr/bin/env nu

# nu shell docs
# - language fundamentals: https://www.nushell.sh/book/custom_commands.html#sub-commands
#     - strings: https://www.nushell.sh/book/working_with_strings.html#string-interpolation
#     - functions (aka commands, subcommands): https://www.nushell.sh/book/custom_commands.html
#     - variables: https://www.nushell.sh/book/variables_and_subexpressions.html
# - built-in command reference: https://www.nushell.sh/commands/#command-reference
#     - length: https://www.nushell.sh/commands/docs/length.html
#     - ansi: https://www.nushell.sh/commands/docs/ansi.html
#     - print: https://www.nushell.sh/commands/docs/print.html
# - mental model
#     - coming from bash: https://www.nushell.sh/book/coming_from_bash.html
#     - thinking in nu: https://www.nushell.sh/book/thinking_in_nu.html

# Main entry point for the script.
def main [...args: string] {
    let num_args = $args | length

    if $num_args == 0 {
        print-help
        return
    }

    let command = $args | get 0

    if $command == "build" {
        build
    } else if $command == "clean" {
        clean
    } else if $command == "run" {
        run
    } else if $command == "run-with-flamegraph-profiling" {
        run-with-flamegraph-profiling
    } else if $command == "watch-run" {
        watch-run
    } else if $command == "test" {
        test
    } else if $command == "watch-one-test" {
        let num_args = $args | length
        if $num_args != 2 {
            print $'🤔 Expected argument for the (ansi blue_bold)<testname>(ansi reset), but got (ansi red_bold)none(ansi reset).'
            print-help
            return
        }
        let test_name = $args | get 1
        watch-one-test $test_name
    } else if $command == "watch-all-tests" {
        watch-all-tests
    } else if $command == "clippy" {
        clippy
    } else if $command == "watch-clippy" {
        watch-clippy
    } else if $command == "docs" {
        docs
    } else if $command == "watch-macro-expansion-one-test" {
        let num_args = $args | length
        if $num_args != 2 {
            print $'🤔 Expected argument for the (ansi blue_bold)<testname>(ansi reset), but got (ansi red_bold)none(ansi reset).'
            print-help
            return
        }
        let test_name = $args | get 1
        watch-macro-expansion-one-test $test_name
    } else if $command == "serve-docs" {
        serve-docs
    } else if $command == "upgrade-deps" {
        upgrade-deps
    } else if $command == "rustfmt" {
        rustfmt
    } else if $command == "help" {
        print-help
    } else if $command == "all" {
        all
    } else {
        print $'Unknown command: (ansi red_bold)($command)(ansi reset)'
    }
}

# TODO: add argument print help for a specific command
def print-help [] {
    print $'Usage: (ansi blue_bold)./scripts.nu(ansi reset) (ansi green_bold)<command>(ansi reset) (ansi yellow)[args](ansi reset)'
    print $'(ansi green_bold)<command>(ansi reset) can be:'
    print $'    (ansi green)all(ansi reset)'
    print $'    (ansi green)build(ansi reset)'
    print $'    (ansi green)clean(ansi reset)'
    print $'    (ansi green)run(ansi reset)'
    print $'    (ansi green)run-with-flamegraph-profiling(ansi reset)'
    print $'    (ansi green)watch-run(ansi reset)'
    print $'    (ansi green)test(ansi reset)'
    print $'    (ansi green)watch-one-test(ansi reset) (ansi blue_bold)<test-name>(ansi reset)'
    print $'    (ansi green)watch-all-tests(ansi reset)'
    print $'    (ansi green)clippy(ansi reset)'
    print $'    (ansi green)watch-clippy(ansi reset)'
    print $'    (ansi green)docs(ansi reset)'
    print $'    (ansi green)watch-macro-expansion-one-test(ansi reset) (ansi blue_bold)<test-name>(ansi reset)'
    print $'    (ansi green)serve-docs(ansi reset)'
    print $'    (ansi green)upgrade-deps(ansi reset)'
    print $'    (ansi green)rustfmt(ansi reset)'
    print $'    (ansi green)help(ansi reset)'
}

def all [] {
    clean
    build
    test
    clippy
    docs
    rustfmt
}

def build [] {
    cargo build
}

def clean [] {
    cargo clean
}

def run [] {
    cargo run --example main
}

def run-with-flamegraph [] {
    cargo flamegraph --example main
}

def watch-run [] {
    cargo watch -- cargo run --example main
}

def test [] {
    cargo test
}

def watch-one-test [test_name: string] {
    # More info on cargo test: https://doc.rust-lang.org/cargo/commands/cargo-test.html
    # More info on cargo watch: https://github.com/watchexec/cargo-watch
    let command_string = "test -- --test-threads=1 --nocapture " + $test_name
    cargo watch -x check -x $command_string -c -q
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

def watch-macro-expansion-one-test [test_name: string] {
    let command_string = "expand --test " + $test_name
    cargo watch -x $command_string -c -q -d 1
}

def serve-docs [] {
    npm i -g serve
    serve target/doc
}

def upgrade-deps [] {
    cargo outdated --workspace --verbose
    cargo upgrade --to-lockfile --verbose
    cargo update
}

def rustfmt [] {
    cargo fmt --all
}