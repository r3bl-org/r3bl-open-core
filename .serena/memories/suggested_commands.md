# Suggested Commands for r3bl-open-core Development

## Code Quality & Validation
```bash
cargo check                           # Fast type check
cargo build                          # Compile production code
cargo clippy --all-targets           # Linting
cargo clippy --fix --allow-dirty     # Auto-fix linting issues
cargo test --no-run                  # Compile tests without running
cargo test --all-targets             # Run all tests
cargo test --doc                     # Run doctests
```

## Documentation & Analysis
```bash
cargo doc --no-deps                  # Generate documentation
cargo bench                          # Run benchmarks (mark with #[bench])
```

## Project Structure Navigation
```bash
find tui/src -name "*.rs" -type f    # Find Rust files
grep -r "pattern" tui/src            # Search in source code
```

## Git Operations
```bash
git status                           # Check working tree status
git log --oneline -10                # View recent commits
git diff                             # See unstaged changes
```
