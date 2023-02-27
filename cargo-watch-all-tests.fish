#!/usr/bin/env fish

# 1. Make sure to install cargo-watch via `cargo install cargo-watch`.
# More info about cargo-watch: https://crates.io/crates/cargo-watch

# 2. Make sure to install cargo-limit via `cargo install cargo-limit`.
# More info about carg-limit: https://crates.io/crates/cargo-limit

# https://doc.rust-lang.org/book/ch11-02-running-tests.html
# cargo watch -x check -x 'test --package rust_book --bin rust_book --all-features -- intermediate::smart_pointers::test_weak_refs --exact --nocapture' -c -q
# cargo watch -x check -x 'test -q --color always' -c -q

# rm -rf target

# https://github.com/watchexec/cargo-watch
# By default, the workspace directories of your project and all local dependencies are watched,
# in this case w/ a delay of 10 seconds.
RUST_BACKTRACE=0 cargo watch --exec check --exec 'test --quiet --color always -- --test-threads 4' --clear --quiet --delay 10

# cargo test -q --color always
# cargo test --package rust_book --bin rust_book --all-features -- data_structures::tree::test_node --exact --nocapture
