#!/usr/bin/env fish
# cargo update
# cargo build --release
pushd tui
RUST_BACKTRACE=1 cargo run --release --example demo
popd
