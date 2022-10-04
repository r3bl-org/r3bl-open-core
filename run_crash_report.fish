#!/usr/bin/env fish
# cargo update
# cargo build --release
pushd tui
RUST_BACKTRACE=FULL cargo run --example demo 2>&1 | tee crash_log.txt
popd
