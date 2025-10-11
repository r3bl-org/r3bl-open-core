Fix errors and warnings in tests, doctests, build docs, clippy 2. can you fix any failing doc tests
(`cargo test --doc`) 3. make sure all the tests pass (`cargo nextest run`) 4. make sure all the docs
build (`cargo doc --no-deps`) 5. make sure cargo clippy --all-targets has no warnings
(`cargo clippy --all-targets`) 6. make sure to run `cargo fmt --all`
