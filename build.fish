#!/usr/bin/env fish

# More info on cargo sparse-registry
# https://internals.rust-lang.org/t/call-for-testing-cargo-sparse-registry/16862

# cargo update
# cargo build --release
# RUST_BACKTRACE=1 cargo run

cargo clean
cargo +nightly -Z sparse-registry update
cargo build
