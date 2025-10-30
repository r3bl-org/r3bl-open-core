# Task Completion Checklist (r3bl-open-core)

After completing a refactoring or feature task:

## Validation Commands
```bash
cargo check                          # Fast type check
cargo build                          # Compile all code
cargo clippy --all-targets           # Check for linting issues
cargo clippy --fix --allow-dirty     # Auto-fix if needed
cargo test --no-run                  # Compile tests
cargo test --all-targets             # Run all tests including integration tests
cargo test --doc                     # Run doctests
cargo doc --no-deps                  # Generate docs (check for doc errors)
```

## Code Review Checklist
- [ ] All files have proper copyright headers
- [ ] Module-level documentation is clear and examples are at trait/module level
- [ ] Method documentation includes type signatures in examples
- [ ] No clippy warnings present
- [ ] All `unwrap()` calls are justified (tests/binaries only, or with doc comment)
- [ ] Type-safe bounds checking used instead of plain usize
- [ ] Git status is clean or changes are staged appropriately

## Documentation
- [ ] Module-level docs explain the "why" and when to use the API
- [ ] Method-level docs show the "how" with syntax examples
- [ ] ASCII diagrams used for visual concepts
- [ ] Examples match the abstraction level (comprehensive at module, simple at method)
