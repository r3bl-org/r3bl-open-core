// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration tests for [`SAVED_TERMIOS`] poison recovery.
//!
//! This test verifies that even if the internal [`SAVED_TERMIOS`] mutex is poisoned by a
//! panic, [`enable_raw_mode()`] and [`disable_raw_mode()`] can still function correctly
//! by using [`into_inner()`] to recover the data.
//!
//! # Why [`PTY`] isolation?
//!
//! Without [`PTY`] isolation, these tests would mutate the `cargo test` runner's own
//! terminal, causing the "staircase effect" (newlines moving the cursor down but not back
//! to column 0) in the test output.
//!
//! By running in a [`PTY`], terminal state mutations only affect the **[child process's
//! `TTY`]**, leaving the developer's terminal environment untouched.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_raw_mode_poison_recovery -- --nocapture
//! ```
//!
//! [`into_inner()`]: std::sync::Mutex::into_inner
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`SAVED_TERMIOS`]: crate::terminal_raw_mode::raw_mode_unix::SAVED_TERMIOS
//! [child process's `TTY`]: crate::core::pty::pty_engine::pty_pair#what-is-a-tty

use crate::{CaughtPanicResult, GLYPH_SUCCESS, MSG_CONTROLLED_READY, MSG_CONTROLLED_STARTING,
            MSG_SUCCESS, PtyTestContext, PtyTestMode, disable_raw_mode,
            enable_raw_mode, extract_panic_message, generate_pty_test,
            terminal_raw_mode::raw_mode_unix::SAVED_TERMIOS};
use rustix::termios::Termios;
use std::{io::{Write, stdout},
          sync::{LockResult, MutexGuard}};

generate_pty_test! {
    test_fn: test_pty_raw_mode_poison_recovery,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Cooked,
}

fn controller(
    PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        ..
    }: PtyTestContext,
) {
    child
        .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
        .unwrap();

    // Capture all output until MSG_SUCCESS. This implicitly verifies all previous steps
    // (poisoning, recovery, and re-enabling) because the controlled process uses
    // assert!() and expect(), which would panic and prevent printing MSG_SUCCESS.
    let result = child.read_until_marker(&mut buf_reader, MSG_SUCCESS, |_| true);

    assert!(
        result.found_marker,
        "Controlled process failed to reach SUCCESS marker.\nCaptured output:\n{}",
        result.lines.join("\n")
    );

    // Verify exit status.
    let exit_code = child.drain_and_wait(buf_reader, pty_pair);
    assert_eq!(exit_code, 0, "Controlled process exited with error");
}
/// The harness performs [`std::process::exit(0)`] after this function returns.
fn controlled() {
    println!("{MSG_CONTROLLED_STARTING}");
    println!("{MSG_CONTROLLED_READY}");
    stdout().flush().unwrap();

    // 1. Poison the mutex by panicking while holding it in a background thread.
    let result: CaughtPanicResult = std::thread::spawn(|| {
        SAVED_TERMIOS.lock_raw(|result: LockResult<MutexGuard<Option<Termios>>>| {
            let _guard = result.unwrap();
            panic!("Intentional panic to poison SAVED_TERMIOS");
        });
    })
    .join();
    assert!(result.is_err());
    let panic_msg = extract_panic_message(result);
    assert_eq!(panic_msg, "Intentional panic to poison SAVED_TERMIOS");

    // 2. Verify mutex is poisoned.
    SAVED_TERMIOS.lock_raw(|result: LockResult<MutexGuard<Option<Termios>>>| {
        assert!(result.is_err());
    });

    // 3. Call disable_raw_mode().
    // It should NOT return an error because it uses into_inner() to recover.
    disable_raw_mode().expect("disable_raw_mode() should succeed (poison-safe)");

    // 4. Verify SAVED_TERMIOS is now None and the lock is still poisoned.
    SAVED_TERMIOS.lock_raw(|result: LockResult<MutexGuard<Option<Termios>>>| {
        let guard = result.unwrap_err().into_inner();
        assert!(guard.is_none(), "SAVED_TERMIOS should be None");
    });

    // 5. Future enable_raw_mode() should succeed (it also uses into_inner()).
    enable_raw_mode().expect("enable_raw_mode() should succeed (poison-safe)");

    // 6. Cleanup.
    disable_raw_mode().expect("disable_raw_mode() should succeed (poison-safe)");

    println!("{GLYPH_SUCCESS} test_saved_termios_poisoning_recovery passed");
    println!("{MSG_SUCCESS}");
    stdout().flush().unwrap();
}
