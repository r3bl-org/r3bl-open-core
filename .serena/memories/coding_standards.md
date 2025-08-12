# R3BL Open Core Coding Standards

## Rust Edition and Linting
- **Edition**: 2024
- **Linting**: Comprehensive workspace-level clippy and rustc lint configuration
- **Strictness**: Uses `all` and `pedantic` clippy lints with custom overrides

## Copyright Headers
The project uses short-style copyright headers in this format:
```rust
// Copyright (c) YYYY R3BL LLC. Licensed under Apache License, Version 2.0.
```

**NOT** the block-style format:
```rust
/*
 *   Copyright (c) YYYY[-YYYY] R3BL LLC
 *   All rights reserved.
 *   ...
 */
```

## Code Quality Enforcement
- **Fast checks**: `cargo check`
- **Linting**: `cargo clippy --all-targets` / `cargo clippy --fix --allow-dirty`
- **Documentation**: `cargo doc --no-deps`
- **Testing**: `cargo nextest run`

## Performance Analysis
- **Benchmarks**: `cargo bench` (mark tests with `#[bench]`)
- **Profiling**: `cargo flamegraph`
- **TUI-specific**: Use `run_example_with_flamegraph_profiling_perf_fold` in `lib_script.nu`

## Key Lint Configurations
- Warns on: missing debug implementations, unused imports, trivial casts
- Allows: `needless_pass_by_value`, `similar_names`, `single_match_else`
- Uses workspace-level lint inheritance

## Dependencies
- Tokio for async runtime with full features and tracing
- Miette for error handling with fancy features
- Clap for CLI argument parsing
- Serde for serialization
- Reqwest with rustls-tls for HTTP (no openssl)

## Development Tools
- **Build cache**: sccache for faster compilation
- **Testing**: nextest for parallel test execution
- **Documentation**: Built-in cargo doc
- **Formatting**: rustfmt