---
name: check-test-coverage
description: Audit and verify test coverage for a specific file or module, ensuring all custom logic branches, state transitions, and boundary conditions are covered while strictly eliminating dependency test bloat (never testing std, third-party crates, or macro-derived boilerplate). Use via /check-test-coverage <filename>.
---

// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

# Skill: check-test-coverage

Audit and verify test coverage for a specific file or module. Ensure all custom logic branches, state transitions, and boundary conditions are covered while strictly eliminating dependency test bloat (never testing `std`, third-party crates, or macro-derived boilerplate).

## When to Use

- When invoked via the `/check-test-coverage <file_path>` command.
- When reviewing a file before creating a commit or finalizing a task.
- When evaluating whether a newly added or modified file has sufficient test coverage.
- When auditing existing test suites to identify missing custom logic tests or remove redundant test bloat.

## Core Philosophy: Zero Test Bloat ("Test OUR Code, Not Dependencies")

Traditional "code coverage" tools blindly count lines or instructions executed, encouraging developers to write low-signal tests for boilerplate, compiler derives, and third-party crates.

In this codebase, **test quality is measured by branch-targeted verification of OUR custom logic**:

1. **Test OUR Code Exclusively**:
   - Tests must target the unique execution paths, custom algorithms, match branches, state transitions, conversion traits (`From`, `TryFrom`), and error conditions written by us.
2. **Never Test Dependencies or Compiler Derives**:
   - Never write tests that merely assert standard library behavior (`std::collections::HashMap`, `std::env`, `std::ffi::OsString`).
   - Never write unit tests for standard compiler derives (`#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]`).
   - Never write unit tests for third-party derive macros (such as `#[derive(Display, EnumString)]` with `#[strum(serialize_all = "lowercase")]` or `clap::ValueEnum`).
3. **High Signal, Low Maintenance Overhead**:
   - Redundant tests slow down `cargo test` and `check.fish` iteration cycles.
   - Redundant test failures during refactoring obscure real bugs and create maintenance friction.

---

## 5-Step Audit Workflow

When running `/check-test-coverage <path>`, follow these five steps systematically:

### Step 1: Inspect the Target File

Read the entire target file using `view_file` or AST navigation (`rust_analyzer_symbols`). Catalog all items:
- Public and private functions (`fn`)
- Structs and Enums
- Derived traits vs custom trait implementations (`impl Trait for Type`)
- Match expressions, `if let` blocks, and conditionals
- Error handling paths (`Result`, `Option`, `unwrap_or_else`, `unreachable!`)

### Step 2: Classify Code Components (Custom Logic vs Upstream/Derives)

Separate items into two categories:

| Category | What it includes | Testing Requirement |
| :--- | :--- | :--- |
| **OUR Custom Logic** | Custom functions, custom parsers/formatters, manual trait impls (`From`, `TryFrom`, `Display`), business logic, state machines, boundary checks, error branches | **Must be covered** with targeted test cases (1 test per branch/boundary) |
| **Upstream / Derives** | `#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]`, strum macro attributes, clap derive attributes, standard library forwards | **Do NOT test** (testing these is test bloat) |

### Step 3: Discover All Test Locations

Locate all test functions covering the file:
1. **Inline unit tests**: `mod tests` inside the file itself.
2. **Adjacent unit test modules**: E.g., `unit_tests.rs` or `mod.rs` test modules in the same directory.
3. **Integration and conformance tests**: E.g., `tests/`, `conformance_tests/`, or crate-level integration suites.

### Step 4: Construct the Branch & Boundary Matrix

Create a structured audit table mapping each piece of custom logic to its test coverage:

```markdown
| Target Symbol / Branch | Test Function | Classification | Status |
| :--- | :--- | :--- | :--- |
| `InputKind::from((Some, None))` | `test_tuple_to_input_kind_file` | Custom `From` logic | Covered |
| `InputKind::from((None, Some))` | `test_tuple_to_input_kind_command` | Custom `From` logic | Covered |
| `BaseEnv::default()` | — | Compiler `#[derive(Default)]` | Skipped (Dependency / Derive) |
| `OutputFormat` strum display | — | Strum macro derive | Skipped (Dependency / Derive) |
```

### Step 5: Deliver Actionable Verdict & Recommendations

Provide a concise, high-signal report:
1. **Verdict**: Explicitly state whether coverage is **Sufficient** or **Insufficient**.
2. **Branch Coverage Breakdown**: Show which custom logic branches are covered vs missing.
3. **Zero Bloat Verification**: Confirm no tests are testing standard library or third-party macros.
4. **Concrete Test Code**: If any custom branches are missing, provide exact, ready-to-use unit test snippets.

---

## Detailed Classification Guide

### What MUST Be Tested (Our Custom Logic)

- **Manual Trait Implementations**:
  ```rust
  // MUST TEST: Custom conversion logic with 2 branches
  impl From<(&Option<PathBuf>, &Option<String>)> for InputKind {
      fn from(tuple: (&Option<PathBuf>, &Option<String>)) -> InputKind {
          match tuple {
              (Some(file), None) => InputKind::File(file.clone()),
              (None, Some(cmd)) => InputKind::InlineCommand(cmd.clone()),
              _ => unreachable!(),
          }
      }
  }
  ```
- **Custom Parsing & Formatting Logic**:
  - Regex parsing, state machines, ANSI code builders, diff calculators.
- **Boundary & Overflow Checks**:
  - Off-by-one boundary cases, index-overflow conditions (see `check-bounds-safety` skill).
- **Error & Fallback Branches**:
  - `Err` returns, fallback defaults when inputs are malformed.

### What MUST NOT Be Tested (Test Bloat)

- **Standard Derives**:
  ```rust
  // DO NOT TEST: Testing BaseEnv::default() tests the Rust compiler derive macro.
  #[derive(Debug, Clone, Default, PartialEq, Eq)]
  pub enum BaseEnv {
      #[default]
      Inherit,
      Explicit(EnvMap),
  }
  ```
- **Third-Party Macro Derives**:
  ```rust
  // DO NOT TEST: Testing .to_string() tests strum's macro generator.
  #[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Display, EnumString)]
  #[strum(serialize_all = "lowercase")]
  pub enum OutputFormat {
      Fish,
      Powershell,
      Json,
      Dotenv,
  }
  ```
- **Standard Library Collections / Types**:
  - Do NOT test whether `HashMap::insert` stores a value or whether `PathBuf::from` works.

---

## Related Skills

- `organize-tests`: Test directory taxonomy, isolation patterns, and zero test-bloat directive.
- `check-bounds-safety`: Type-safe index and length bounds verification.
- `check-code-quality`: Full quality verification suite (`./check.fish --full`).
