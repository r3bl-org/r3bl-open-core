// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

# Skill: organize-tests

Organize tests by isolation requirements, adhering to PTY conventions and subprocess isolation patterns.

## When to Use
- Adding new tests to the codebase.
- Refactoring existing tests.
- Organizing test modules and directories.
- Ensuring PTY tests follow the "Run with:" and deadlock prevention conventions.

## Instructions

### 1. Identify Isolation Requirements
Choose the correct directory based on **why** the test needs isolation. This maintains low cognitive load for future developers.

See [Taxonomy](taxonomy.md) for directory details.

### 2. Follow PTY Conventions
PTY tests are complex and prone to deadlocks (especially on macOS). Strict adherence to naming, documentation, and resource management is mandatory.

See [PTY Conventions](pty-conventions.md) for details.

### 3. Orchestrate Process Isolation
Tests that pollute global mock state (e.g., static Mutexes) must be isolated into a single subprocess and run sequentially.

See [Examples](examples.md) for macro usage.

### 4. Wire Up Modules
Always ensure test modules are visible for both tests and documentation using `#[cfg(any(test, doc))]`.

```rust
#[cfg(any(test, doc))]
pub mod unit_tests;
#[cfg(any(test, doc))]
pub mod process_isolated_tests;
#[cfg(any(test, doc))]
pub mod my_module_integration_tests;
```

## Related Skills
- `organize-modules`: Use for general module structure and re-exports.
- `write-documentation`: Use for formatting "Run with:" blocks and intra-doc links.
