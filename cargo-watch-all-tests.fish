#!/usr/bin/env fish

# 1. Make sure to install cargo-watch via `cargo install cargo-watch`.
# More info about cargo-watch: https://crates.io/crates/cargo-watch

# 2. Make sure to install cargo-limit via `cargo install cargo-limit`.
# More info about carg-limit: https://crates.io/crates/cargo-limit

# https://doc.rust-lang.org/book/ch11-02-running-tests.html
# cargo watch -x check -x 'test --package rust_book --bin rust_book --all-features -- intermediate::smart_pointers::test_weak_refs --exact --nocapture' -c -q
# cargo watch -x check -x 'test -q --color always' -c -q

# rm -rf target
RUST_BACKTRACE=0 cargo watch -x check -x 'test -q --color always' -c -q

# cargo test -q --color always
# cargo test --package rust_book --bin rust_book --all-features -- data_structures::tree::test_node --exact --nocapture
