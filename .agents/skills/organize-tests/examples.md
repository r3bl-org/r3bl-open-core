# Test Organization Examples

## 1. Process Isolation (Mock Pollution)
Use this template in `process_isolated_tests/mod.rs` to run multiple tests sequentially in a single child process.

```rust
use crate::generate_isolated_process_test;
use fixtures::controller_fn;

generate_isolated_process_test!(
    test_feature_isolated,
    controller_fn,
    run_all_tests_sequentially,
    std::process::Stdio::null(),
    std::process::Stdio::piped(),
    std::process::Stdio::piped()
);

fn run_all_tests_sequentially() {
    group_b::test_1();
    group_b::test_2();
    group_c::test_3();
}
```

## 2. PTY Integration Test
Use this template for tests requiring real terminal FDs.

```rust
//! [`PTY`] integration test for [feature name].
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui --lib [test_function_name] -- --nocapture
//! ```

use crate::{generate_pty_test, PtyTestContext, PtyTestMode};

generate_pty_test! {
    test_fn: test_terminal_interaction,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
}

fn controller(context: PtyTestContext) {
    let PtyTestContext { child, mut buf_reader, pty_pair, .. } = context;
    // ... verification logic ...

    // The only exit path is drain_and_wait(), preventing PTY buffer deadlocks.
    child.drain_and_wait(buf_reader, pty_pair);
}

/// [Description of the controlled process].
///
/// The harness performs [`std::process::exit(0)`] after this function returns.
fn controlled() {
    // ... terminal logic ...
}
```

## 3. High-Quality PTY Test Examples
For inspiration and best practices, refer to these well-organized PTY test modules:
- **Resilient Reactor Thread**: `tui/src/core/resilient_reactor_thread/integration_tests/`
- **Raw Mode Input**: `tui/src/core/ansi/terminal_raw_mode/raw_mode_integration_tests/`
- **Direct to ANSI Input**: `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/integration_tests/`
