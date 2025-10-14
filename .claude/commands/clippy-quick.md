Fix errors and warnings in tests, doctests, build docs, clippy

1. can you fix any failing doc tests (`cargo test --doc`)
2. make sure all the tests pass (`cargo nextest run`)
3. make sure all the docs build (`cargo doc --no-deps`)
4. make sure (`cargo clippy --all-targets`) has no warnings
5. make sure to run `cargo fmt --all`
6. for all the code in the staged & unstaged area of the working tree can you use reference style
   links in rustdocs wherever possible for symbols enclosed in backticks
7. for all the code in the staged & unstaged area of the working tree if the rustdocs have markdown
   tables, make sure that they are properly column aligned using whitespaces