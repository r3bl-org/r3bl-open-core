// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Gate ensuring only one [`DirectToAnsiInputDevice`] exists at a time.
//!
//! - [`claim_and_assert()`] marks a device as existing (panics if already set)
//! - [`release()`] resets, allowing a new device
//!
//! [`DirectToAnsiInputDevice`]: super::DirectToAnsiInputDevice

use std::sync::atomic::{AtomicBool, Ordering};

/// Tracks whether a [`DirectToAnsiInputDevice`] currently exists.
///
/// # Why [`AtomicBool`] instead of [`Mutex<bool>`]?
///
/// **Do not use [`Mutex<bool>`] here.** The singleton test uses [`catch_unwind`] to
/// verify that creating a second device panics. If [`claim_and_assert()`] panics while
/// holding a [`Mutex`] lock, Rust marks the mutex as **poisoned**. Subsequent
/// `lock().unwrap()` calls (e.g., in [`release()`] during drop) will panic, causing the
/// test to hang or fail.
///
/// `AtomicBool::swap()` is **panic-safe**---there's no lock to poison.
///
/// [`catch_unwind`]: std::panic::catch_unwind
/// [`DirectToAnsiInputDevice`]: super::DirectToAnsiInputDevice
/// [`Mutex<bool>`]: std::sync::Mutex
/// [`Mutex`]: std::sync::Mutex
static DEVICE_EXISTS: AtomicBool = AtomicBool::new(false);

/// Marks that a device now exists. It can only be called once. Must call [`release()`]
/// before you can call it again.
///
/// # Panics
///
/// If you call this more than once it will panic.
#[allow(clippy::bool_assert_comparison)]
pub fn claim_and_assert() {
    // swap() returns the OLD value - false means there is no a preexisting device.
    let device_already_exists = DEVICE_EXISTS.swap(true, Ordering::SeqCst);
    assert_eq!(
        device_already_exists, false,
        "DirectToAnsiInputDevice::new() called while another device exists. \
         Use device.subscribe() for additional receivers."
    );
}

/// Clears it, so that you can call [`claim_and_assert()`] again.
pub fn release() { DEVICE_EXISTS.store(false, Ordering::SeqCst); }

// XMARK: Process isolated test.

/// Process-isolated tests for [`at_most_one_instance_assert`].
///
/// These tests touch global state (`DEVICE_EXISTS` static), so they must run in an
/// isolated process to avoid interference with other tests.
///
/// [`at_most_one_instance_assert`]: super::at_most_one_instance_assert
#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate_isolated_process_test;

    generate_isolated_process_test!(
        test_at_most_one_instance_in_isolated_process,
        controller_fn,
        run_tests_impl,
        std::process::Stdio::null(),
        std::process::Stdio::piped(),
        std::process::Stdio::piped()
    );

    /// Validates that the child process succeeded without unexpected panics.
    ///
    /// Test 3 deliberately double-claims to verify the panic guard.
    /// [`catch_unwind`] catches the panic, but Rust's default panic hook still prints
    /// the message to stderr *before* the catch. Since [`spawn_isolated_process()`],
    /// from [`generate_isolated_process_test!`], passes `--nocapture`, these deliberate
    /// panic messages appear in the piped stderr. We must tolerate them.
    ///
    /// [`catch_unwind`]: std::panic::catch_unwind
    /// [`generate_isolated_process_test!`]: crate::generate_isolated_process_test
    /// [`spawn_isolated_process()`]: crate::spawn_isolated_process
    fn controller_fn(output: std::process::Output) {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let has_unexpected_error = stderr.contains("Test failed with error")
            || (!stderr.contains("another device exists")
                && stderr.contains("panicked at"));

        if !output.status.success() || has_unexpected_error {
            eprintln!("Exit status: {:?}", output.status);
            eprintln!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
            eprintln!("Stderr: {stderr}");

            panic!(
                "Isolated test failed with status code {:?}: {}",
                output.status.code(),
                stderr
            );
        }
    }

    /// Runs all tests sequentially in an isolated process.
    fn run_tests_impl() {
        // Test 1: claim_and_assert() works once.
        release();
        claim_and_assert(); // Should not panic.
        release();

        // Test 2: release() allows another claim_and_assert().
        release();
        claim_and_assert();
        release();
        claim_and_assert(); // Should not panic.
        release();

        // Test 3: claim_and_assert() panics when called twice.
        release();
        claim_and_assert();
        let result = std::panic::catch_unwind(|| {
            claim_and_assert(); // Should panic.
        });
        assert!(
            result.is_err(),
            "Expected claim_and_assert() to panic when called twice"
        );
        release();
    }
}
