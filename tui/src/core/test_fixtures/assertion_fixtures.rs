// Copyright (c) 2024-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Panic payload extraction ([`CaughtPanicResult`], [`extract_panic_message`]) and
//! multi-line unified string diff assertions (`assert_str_diff_eq`).

use std::any::Any;

/// Type alias for the result of a caught panic.
///
/// The following methods have the same return type, `Result<(), dyn Any + Send>`:
/// 1. [`std::thread::JoinHandle::join`]
/// 2. [`std::panic::catch_unwind`]
///
/// The `Err(dyn Any + Send)` variant contains the payload captured during the panic.
/// Panicking generates a string message or payload.
///
/// For example, when a [`Mutex`] `lock().expect("conversion error")` panics due to lock
/// poisoning, the [`Result::Err(PoisonError)`] gets formatted to a string and the type is
/// erased (just a string is left).
///
/// # Example
///
/// ```no_run
/// use r3bl_tui::CaughtPanicResult;
///
/// // 1. Using catch_unwind.
/// let result: CaughtPanicResult = std::panic::catch_unwind(|| {
///     panic!("intentional panic");
/// });
///
/// // 2. Using thread join.
/// let result: CaughtPanicResult = std::thread::spawn(|| {
///     panic!("intentional panic");
/// }).join();
/// ```
///
/// [`Mutex`]: std::sync::Mutex
/// [`Result::Err(PoisonError)`]: std::sync::PoisonError
pub type CaughtPanicResult<T = ()> = Result<T, Box<dyn Any + Send>>;

/// Extracts the message (as a [`String`]) from a [`CaughtPanicResult`].
///
/// This function attempts to downcast the generic panic payload into a
/// human-readable [`String`]. It handles both literal string slices ([`&str`]) and
/// heap-allocated [`String`]s.
///
/// # Example
///
/// ```no_run
/// use r3bl_tui::{CaughtPanicResult, extract_panic_message};
///
/// let result: CaughtPanicResult = std::panic::catch_unwind(|| {
///     panic!("intentional panic");
/// });
/// let message = extract_panic_message(result);
/// assert_eq!(message, "intentional panic");
/// ```
///
/// # Panics
///
/// Panics if the provided `result` is [`Ok`]. This function is only intended to be
/// called on results that are known to contain a panic (e.g., in a test after a
/// suspected failure).
///
/// # Returns
///
/// - The extracted message as a [`String`].
/// - A default "Unknown panic payload" message if the payload type is neither [`&str`]
///   nor [`String`].
pub fn extract_panic_message<T>(result: CaughtPanicResult<T>) -> String {
    let panic_payload = result.err().expect("Expected a panic but found Ok");
    // Try cast to &str.
    if let Some(s) = panic_payload.downcast_ref::<&str>() {
        return s.to_string();
    }
    // Try cast to String.
    if let Some(s) = panic_payload.downcast_ref::<String>() {
        return s.clone();
    }
    // Both attempts at casting failed.
    format!("Unknown panic payload: {panic_payload:?}")
}

/// Compares `actual` against `expected` using [`similar::TextDiff`] for readable
/// unified diffs on multi-line strings in unit tests.
///
/// # Panics
///
/// Panics if `actual != expected`, printing a unified line-by-line diff with 3 lines
/// of context to [`stderr`] before panicking at the caller's source location.
///
/// [`stderr`]: std::io::stderr
#[cfg(test)]
#[track_caller] // Reports the test call site's file and line number on panic instead of this helper's.
pub fn assert_str_diff_eq(actual: &str, expected: &str) {
    if actual != expected {
        let diff = similar::TextDiff::from_lines(expected, actual);
        let unified_diff = diff
            .unified_diff()
            .context_radius(3)
            .header("expected", "actual")
            .to_string();
        eprintln!("=== DIFF (expected vs actual) ===\n{unified_diff}");
        panic!("Actual string output does not match expected string output");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_panic_message() {
        let payload_str: Box<dyn Any + Send> = Box::new("test str");
        assert_eq!(
            extract_panic_message(Result::<(), _>::Err(payload_str)),
            "test str"
        );

        let payload_string: Box<dyn Any + Send> = Box::new("test string".to_string());
        assert_eq!(
            extract_panic_message(Result::<(), _>::Err(payload_string)),
            "test string"
        );

        let payload_other: Box<dyn Any + Send> = Box::new(42);
        assert!(
            extract_panic_message(Result::<(), _>::Err(payload_other))
                .contains("Unknown panic payload")
        );
    }

    #[test]
    fn test_assert_str_diff_eq_matches() {
        assert_str_diff_eq("line 1\nline 2\n", "line 1\nline 2\n");
    }

    #[test]
    #[should_panic(
        expected = "Actual string output does not match expected string output"
    )]
    fn test_assert_str_diff_eq_panics_on_mismatch() {
        assert_str_diff_eq("line 1\nactual\n", "line 1\nexpected\n");
    }
}
