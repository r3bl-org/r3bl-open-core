# PTY Test Conventions

PTY tests use real pseudoterminal pairs to simulate terminal I/O.

## 1. Documentation Requirements

### Run with: section
Every PTY test file must have module-level rustdoc (`//!`) that includes a **Run with:** section. This allows developers to quickly copy-paste the exact command needed to run the isolated test.

```rust
//! [`PTY`] integration test for [feature name].
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui --lib [test_function_name] -- --nocapture
//! ```
```

### Controlled Function exit behavior
The `controlled` function's rustdoc must include the following line to clarify that the harness handles process termination:
`/// The harness performs [`std::process::exit(0)`] after this function returns.`

## 2. Naming & Roles
Use the following terminology to maintain consistency with the `generate_pty_test!` macro:
- **Controller**: The parent process that manages the PTY and verifies results. The function is usually named `controller`.
- **Controlled**: The child process that runs the actual application/worker logic. The function is usually named `controlled`.
- **Avoid**: Do not use "master/slave" terminology.

## 3. Shared Constants
Shared constants used across multiple PTY tests (e.g., handshake signals) should be defined in:
`tui/src/core/test_fixtures/pty_test_fixtures/constants.rs`

## 4. Resource Management & Deadlocks
- **macOS Buffer Limits**: macOS has very small PTY buffers. If the child writes more than ~1024 bytes without the parent reading, the child will block forever.
- **Deadlock Prevention**: Always use `child.drain_and_wait(buf_reader, pty_pair)` in the controller to ensure all output is consumed before waiting for the process to exit.
- **Independence**: Use `try_clone_reader()` to get an owned `Box<dyn Read>`. This ensures the reader and `PtyPair` are independent, preventing lifetime and borrow checker issues.

## 5. Mode Selection
- **Raw Mode**: Use for tests that handle raw ANSI escapes and keystrokes directly.
- **Cooked Mode**: Use for tests that rely on line-buffered terminal behavior.
