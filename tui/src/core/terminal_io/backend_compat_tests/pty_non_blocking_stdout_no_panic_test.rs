// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words fcntl getfl setfl EAGAIN NONBLOCK

//! [`PTY`] integration test for [`FullBufferWaitingStdout`] /
//! [`OutputDevice::new_stdout()`] ensuring that setting [`O_NONBLOCK`] on [`stdin`] does
//! not cause [`stdout`] writes to panic the application under heavy load.
//!
//! # Related Documentation
//! - **The [`stdout`] recovery mechanism:**
//!   - In [`FullBufferWaitingStdout`]:
//!       - See [Mental Model] for why setting [`stdin`] to non-blocking affects
//!         [`stdout`] on Linux.
//!       - See [Blocking vs Busy Waiting vs Yielding] for the educational guide on thread
//!         yielding.
//! - **The [`stdin`] non-blocking requirement:**
//!   - See [`MioPollWorker`] section [How This Affects stdout as well] to see how the
//!     [`stdout`] side-effect is introduced.
//!   - See [`consume_stdin_input_with_sender`] section [Why We Need Non-Blocking Read] to
//!     understand why edge-triggered polling requires non-blocking input reads.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_non_blocking_stdout_no_panic -- --nocapture
//! ```
//!
//! [`FullBufferWaitingStdout`]: FullBufferWaitingStdout
//! [`O_NONBLOCK`]: rustix::fs::OFlags::NONBLOCK
//! [`OutputDevice::new_stdout()`]: crate::core::terminal_io::OutputDevice::new_stdout
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`stdin`]: std::io::stdin
//! [`stdout`]: std::io::stdout
//! [Blocking vs Busy Waiting vs Yielding]:
//!     FullBufferWaitingStdout#blocking-vs-busy-waiting-vs-yielding
//! [How This Affects stdout as well]: MioPollWorker#how-this-affects-stdout-as-well
//! [Mental Model]: FullBufferWaitingStdout#the-mental-model
//! [Why We Need Non-Blocking Read]:
//!     consume_stdin_input_with_sender#why-we-need-non-blocking-read

// Imported specifically for the intra-doc links in the module-level documentation.
#[allow(unused_imports)]
use crate::{
    core::terminal_io::FullBufferWaitingStdout,
    tui::terminal_lib_backends::direct_to_ansi::input::mio_poller::{
        MioPollWorker, handler_stdin::consume_stdin_input_with_sender,
    },
};

use crate::{OutputDevice, PtyTestContext, PtyTestMode, generate_pty_test};
use std::io::Write;

generate_pty_test! {
    test_fn: test_pty_non_blocking_stdout_no_panic,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
}

fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        ..
    } = context;

    eprintln!("Non-Blocking Stdout No Panic Controller: Starting...");

    child
        .wait_for_ready(&mut buf_reader, "READY")
        .expect("Failed to wait for READY");

    child
        .wait_for_ready(&mut buf_reader, "SMALL_WRITE_DONE")
        .expect("Failed to wait for SMALL_WRITE_DONE");

    child
        .wait_for_ready(&mut buf_reader, "MASSIVE_WRITE_DONE")
        .expect("Failed to wait for MASSIVE_WRITE_DONE");

    child.drain_and_wait(buf_reader, pty_pair);
    eprintln!("Non-Blocking Stdout No Panic Controller: Test passed!");
}

/// The harness performs `std::process::exit(0)` after this function returns.
fn controlled() {
    println!("READY");
    std::io::stdout().flush().expect("Failed to flush READY");

    // 1. Set stdin to non-blocking to replicate MioPollWorker side-effect on stdout
    let stdin = std::io::stdin();
    let original_stdin_flags = if let Ok(flags) = rustix::fs::fcntl_getfl(&stdin) {
        let _ = rustix::fs::fcntl_setfl(&stdin, flags | rustix::fs::OFlags::NONBLOCK);
        Some(flags)
    } else {
        None
    };

    // 2. Create the wrapped OutputDevice
    let output_device = OutputDevice::new_stdout();

    // 3. Scenario 1 (Small Write): Should succeed immediately.
    output_device.write(|writer| {
        writer
            .write_all(b"SMALL_WRITE_DONE\n")
            .expect("Small write failed");
        writer.flush().expect("Small flush failed");
    });

    // 4. Scenario 2 (Massive Write): Blast the PTY buffer to trigger EAGAIN / WouldBlock
    // A PTY buffer is typically a few KB. We write megabytes to guarantee overflow.
    let chunk = vec![b'A'; 1024]; // 1 KB chunk
    output_device.write(|writer| {
        for _ in 0..10_000 {
            // 10 MB total
            writer.write_all(&chunk).expect("Massive write failed");
        }
        writer
            .write_all(b"\nMASSIVE_WRITE_DONE\n")
            .expect("Massive done write failed");
        writer.flush().expect("Massive flush failed");
    });

    // 5. Restore stdin flags to be clean (though the harness exits anyway)
    if let Some(flags) = original_stdin_flags {
        let _ = rustix::fs::fcntl_setfl(&stdin, flags);
    }
}
