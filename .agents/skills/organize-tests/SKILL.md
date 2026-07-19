---
name: organize-tests
description: Organize tests by isolation requirements, adhering to PTY conventions and subprocess isolation patterns.
---
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

### 5. Memory Size "Tripwire" Tests
When writing tests that assert the byte size of a struct (`std::mem::size_of`), you MUST gate the test or the assertion block with `#[cfg(target_pointer_width = "64")]` to ensure it only runs on 64-bit architectures. Struct sizes vary between 32-bit and 64-bit platforms due to pointer sizes.

```rust
#[test]
fn test_my_struct_size() {
    // TRIPWIRE: If you add or remove a field, this test will fail.
    // This reminds you to update the `GetMemSize` implementation.
    #[cfg(target_pointer_width = "64")]
    {
        assert_eq!(std::mem::size_of::<MyStruct>(), 184);
    }
}
```

## Related Skills
- `organize-modules`: Use for general module structure and re-exports.
- `write-documentation`: Use for formatting "Run with:" blocks and intra-doc links.
