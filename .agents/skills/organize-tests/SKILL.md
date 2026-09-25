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

### 5. Organize Conformance & Golden Test Data (`test_data/`)
When tests validate external files or assert outputs against golden files, place them in `test_data/` with `input/` and `expected_output/` subdirectories. Use matching basenames (e.g., `input/unix/cargo_env.sh` and `expected_output/unix/cargo_env.fish`) and protect the directory with an `AGENTS.md` file. See [Taxonomy](taxonomy.md) for details.

### 6. Memory Size "Tripwire" Tests
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

### 7. Test OUR Code, Not Dependencies (Zero Test Bloat)
Tests must exclusively target the branches, state transitions, error paths, and delegation logic of **our** codebase (the System Under Test).

**Why Noisy Tests Add Negative Value:**
- **Compilation Overhead**: Each redundant test function generates additional compiler symbols, AST nodes, and test runner harness code, slowing down incremental `cargo test` and `check.fish` iteration cycles.
- **Obfuscation & Signal Dilution**: When a real regression occurs, walls of redundant, noisy failures obscure the root cause, making triage and debugging far harder.
- **Refactoring Drag**: Tests that assert upstream crate or `std` behaviors add zero bug-catching value while creating maintenance friction during internal architecture refactors.

**Key Rules:**
1. **Never Test the Standard Library or Third-Party Crates**: Do NOT write test cases that merely assert or re-verify standard library behaviors (such as `std::ffi::OsString` UTF-8 validation permutations, `std::collections::HashMap` storage integrity, `tokio` task scheduling, `serde` serialization formats, etc.). These dependencies are already heavily tested upstream.
2. **Branch-Targeted Coverage**: Count the execution paths and branches in **our** code:
   - If our method has two branches (e.g., `Ok` fast path vs `unwrap_or_else` fallback closure), write exactly one test per branch.
   - Redundant permutations of valid inputs (such as testing multiple languages, emojis, whitespace variants, or path styles on a method that simply forwards to a standard library function) test the standard library, not our logic.
3. **High Signal, Low Cognitive Load**: Every test in the repository must serve a distinct purpose by covering a specific branch or boundary condition of our implementation. Proactively remove redundant, needless, or duplicative tests that only inflate maintenance overhead.

### 8. Three-Tiered Protocol & Parser Testing (Ground Truth vs. Test Fixtures)
When testing terminal protocols, parsers, and escape sequences, prevent circular tautology by adhering to the three-tier division of labor:

1. **Validation Tests (`validation_tests/`)**:
   - **Ground truth only**: Assertions and inputs must use hardcoded raw byte literals (`b"\x1b[..."`) captured from real terminals (`showkey -a`, `sed -n l`).
   - **No generators or constants**: Never refactor validation tests to use generator functions or Tier 1 constants. These tests validate the constants and generators; they must not depend on them.

2. **Unit & Component Tests (`unit_tests/`, `chunk_framer/`, `chunk_decoder/`)**:
   - **Use Tier 1 constants & generator helpers**: Once validated by Tier 1 tests, use named constants (`MODIFIER_PARAMETER_SHIFT`) and generator helpers (`csi_tilde`) to test internal state machines and decoding logic without scattering magic byte sequences.

3. **PTY Integration Tests (`<prefix>_integration_tests/`)**:
   - **Use generated sequences**: PTY tests validate end-to-end subsystem integration and scalability by generating sequences through our generator functions.

See [Taxonomy](taxonomy.md) for full directory and tier details.

## Related Skills
- `organize-modules`: Use for general module structure and re-exports.
- `write-documentation`: Use for formatting "Run with:" blocks and intra-doc links.
- `check-code-quality`: Use for comprehensive quality checklists and test execution.

