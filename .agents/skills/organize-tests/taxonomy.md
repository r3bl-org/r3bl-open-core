# Test Directory Taxonomy

Organize test directories by **why** the test is isolated.

| Directory                   | Isolation Reason                                                                   | Runs via                                                                      |
| :-------------------------- | :--------------------------------------------------------------------------------- | :---------------------------------------------------------------------------- |
| `unit_tests/`               | **None**: Stateless, parallel.                                                    | `#[test]`                                                                     |
| `process_isolated_tests/`   | **Mock State Pollution**: Static Mutexes or global resources would leak threads.   | `generate_isolated_process_test!` (single subprocess, sequential dispatcher)  |
| `<prefix>_integration_tests/` | **OS Resources**: Needs real PTY file descriptors or `epoll` handles.            | `generate_pty_test!` (1 test per file)                                        |
| `test_data/`                | **Conformance & Golden Fixtures**: Static inputs and expected outputs on disk.     | `#[test]` / snapshot assertions via `include_str!`                            |

## 1. `unit_tests/`
Use for pure logic, state-less calculations, and modules that don't use threads or global mock state. These are the fastest tests and should run in parallel by default.

## 2. `process_isolated_tests/`
Use when multiple tests share a global mock (e.g., `TEST_FACTORY_STATE`).
- **Orchestration**: Create a `mod.rs` that uses `generate_isolated_process_test!`.
- **Dispatcher**: Define a `run_all_tests_sequentially()` function that calls each test function.
- **Benefits**: Prevents flakey tests caused by global state leakage while avoiding the overhead of multiple subprocesses.

## 3. `<prefix>_integration_tests/`
Use for tests that interact with the terminal or OS pollers.
- **Unique Naming**: The directory MUST be named `<prefix>_integration_tests/` (e.g., `rrt_integration_tests/`, `log_integration_tests/`) to avoid name collisions in the flat API caused by barrel re-exports.
- **PTY requirement**: These tests usually fail in standard `cargo test` environments because `stdin` is not a real TTY.
- **Isolation**: Each complex test gets its own file to prevent resource contention.

## 4. `test_data/` (Conformance & Golden Test Data)
Use for external fixture files, input scripts, and golden output files validated by test runners:

- **Directory Segregation**: Always separate inputs and expected outputs into distinct `input/` and `expected_output/` subdirectories:
  ```text
  test_data/
  ├── AGENTS.md            # Test data protection rules (do not modify)
  ├── mod.rs               # Rust constants exposing fixtures via include_str!
  ├── input/               # Raw input files to process or evaluate
  │   └── <platform_or_category>/
  │       └── <test_case>.<ext>
  └── expected_output/     # Exact golden outputs to assert against
      └── <platform_or_category>/
          └── <test_case>.<ext>
  ```
- **Matching Basenames**: Paired input and expected output files share identical base names without redundant `input_` or `expected_` prefixes (e.g., `input/unix/cargo_env.sh` pairs with `expected_output/unix/cargo_env.fish`).
- **Tooling Compatibility**: Monorepo tooling (such as `cargo-rustdoc-fmt`) explicitly detects and skips directories named `test_data/`, preventing automated formatters from altering raw test fixtures.
- **`AGENTS.md` Guardian**: Every `test_data/` directory must contain an `AGENTS.md` file instructing AI agents and developers not to modify, reformat, or re-encode fixtures as collateral damage during refactoring.
