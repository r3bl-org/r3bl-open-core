1. can you fix any failing doc tests (`cargo test --doc`)
2. make sure all the tests pass (`cargo nextest run`)
3. make sure all the docs build (`cargo doc --no-deps`)
4. make sure cargo clippy --all-targets has no warnings (`cargo clippy --all-targets`)
5. make sure to run `cargo fmt --all`