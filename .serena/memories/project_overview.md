# r3bl-open-core Project Overview

## Project Purpose
r3bl-open-core is a Rust workspace providing terminal UI (TUI) capabilities, command-line tools, and analytics infrastructure.

## Main Components
- **tui**: Terminal User Interface library with ANSI support, input handling, and raw mode control
- **cmdr**: Command-line tool functionality
- **analytics_schema**: Analytics data structures

## Tech Stack
- **Language**: Rust (stable + nightly for some features)
- **Key Libraries**: rustix (safe termios API), portable-pty (PTY handling), crossterm (terminal)
- **Code Quality**: Strict clippy lints, rustfmt, comprehensive error handling

## Development Workflow
- Use `cargo check`, `cargo build`, `cargo test` for validation
- Run `cargo clippy --all-targets` for linting
- Use `cargo doc --no-deps` for documentation
- PTY-based integration tests for terminal functionality
- Fish shell scripts for complex tasks (run.fish, check.fish, etc.)
