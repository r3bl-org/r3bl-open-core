# Test Directory Taxonomy

Organize test directories by **why** the test is isolated.

| Directory | Isolation Reason | Runs via |
| :--- | :--- | :--- |
| `unit_tests/` | **None** - Stateless, parallel. | `#[test]` |
| `process_isolated_tests/` | **Mock State Pollution** - Static Mutexes or global resources would leak between parallel threads. | `generate_isolated_process_test!` (single subprocess, sequential dispatcher) |
| `<prefix>_integration_tests/` | **OS Resources** - Needs real PTY file descriptors or `epoll` handles. | `generate_pty_test!` (1 test per file) |

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
