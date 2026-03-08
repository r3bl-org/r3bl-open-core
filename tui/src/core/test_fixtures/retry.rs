// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Test Retry Mechanism
//!
//! This module provides a simple, robust retry mechanism for handling flaky tests,
//! especially integration tests that rely on external resources, kernel [`PTY`]
//! buffers, or complex process synchronization.
//!
//! ## Short-Circuit Behavior
//!
//! The retry logic implements a **"Short-circuit on success"** pattern. This means:
//! - If the test passes on the **first attempt**, it finishes immediately. No extra time
//!   is spent.
//! - Subsequent attempts only run if the previous attempt failed with an `Err`.
//! - The test only fails (panics) if **all** attempts fail.
//!
//! For example, if `max_attempts` is set to 3:
//! 1. **Attempt 1**: Runs. If `Ok`, return immediately. Total runs: 1.
//! 2. **Attempt 2**: Runs only if #1 failed. If `Ok`, return. Total runs: 2.
//! 3. **Attempt 3**: Runs only if #2 failed. If `Err`, panic. Total runs: 3.
//!
//! ## Recommended Retry Counts
//!
//! | Count | Use Case                                                                |
//! | :---- | :---------------------------------------------------------------------- |
//! | **1** | Default (no retry). Use for deterministic unit tests.                   |
//! | **3** | **Standard**. Good for local [`PTY`] tests and complex synchronization. |
//! | **5** | **Fragile**. Use for network/API tests or legacy backend comparisons.   |
//!
//! ## Entry Points
//!
//! - Use **[`retry_until_success_test!`]** for synchronous tests.
//! - Use **[`retry_until_success_test_async!`]** for asynchronous tests (Tokio).
//!
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`retry_until_success_test!`]: crate::retry_until_success_test
//! [`retry_until_success_test_async!`]: crate::retry_until_success_test_async

/// Macro version of [`retry_until_success`] for cleaner syntax in tests.
///
/// See [module-level documentation] for retry strategy recommendations and behavior
/// details.
///
/// # Examples
///
/// With explicit retry count:
///
/// ```rust
/// use r3bl_tui::retry_until_success_test;
///
/// retry_until_success_test!(5, {
///     // Your test logic here
///     Ok::<(), String>(())
/// });
/// ```
///
/// With default retry count (3):
///
/// ```rust
/// use r3bl_tui::retry_until_success_test;
///
/// retry_until_success_test!({
///     // Your test logic here
///     Ok::<(), String>(())
/// });
/// ```
///
/// [module-level documentation]: crate::core::test_fixtures::retry
#[macro_export]
macro_rules! retry_until_success_test {
    ($max_attempts:expr, $body:block) => {
        $crate::retry_until_success($max_attempts, || $body).unwrap_or_else(|e| {
            panic!(
                "Test failed after {} attempts. Last error:\n{}",
                $max_attempts, e
            )
        })
    };
    ($body:block) => {
        $crate::retry_until_success_test!(2, $body)
    };
}

/// Macro version of [`retry_until_success_async`] for cleaner syntax in async tests.
///
/// See [module-level documentation] for retry strategy recommendations and behavior
/// details.
///
/// # Examples
///
/// With explicit retry count:
///
/// ```rust
/// use r3bl_tui::retry_until_success_test_async;
///
/// # async fn test_async() {
/// retry_until_success_test_async!(5, {
///     // Your async test logic here
///     Ok::<(), String>(())
/// });
/// # }
/// ```
///
/// With default retry count (3):
///
/// ```rust
/// use r3bl_tui::retry_until_success_test_async;
///
/// # async fn test_async() {
/// retry_until_success_test_async!({
///     // Your async test logic here
///     Ok::<(), String>(())
/// });
/// # }
/// ```
///
/// [module-level documentation]: crate::core::test_fixtures::retry
#[macro_export]
macro_rules! retry_until_success_test_async {
    ($max_attempts:expr, $body:block) => {
        $crate::retry_until_success_async($max_attempts, async || $body)
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "Async test failed after {} attempts. Last error:\n{}",
                    $max_attempts, e
                )
            })
    };
    ($body:block) => {
        $crate::retry_until_success_test_async!(2, $body)
    };
}

/// A simple retry mechanism for flaky tests.
///
/// Runs the provided closure up to `max_attempts` times. If the closure returns `Ok`,
/// the function returns immediately. If it returns `Err`, it retries until
/// `max_attempts` is reached.
///
/// # Examples
///
/// ```rust
/// use r3bl_tui::retry_until_success;
///
/// let result = retry_until_success(3, || {
///     // Some flaky operation
///     if true { Ok(()) } else { Err("failed".to_string()) }
/// });
/// assert!(result.is_ok());
/// ```
///
/// # Errors
///
/// Returns the last error produced by the closure if all `max_attempts` fail.
///
/// # Panics
///
/// Panics if `max_attempts` is 0.
pub fn retry_until_success<F, T, E>(max_attempts: u8, mut f: F) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
    E: std::fmt::Display,
{
    let mut last_error: Option<E> = None;

    for attempt in 1..=max_attempts {
        match f() {
            Ok(val) => {
                if attempt > 1 {
                    eprintln!("✅ Passed on attempt {attempt}/{max_attempts}");
                }
                return Ok(val);
            }
            Err(e) => {
                eprintln!("⚠️ Attempt {attempt}/{max_attempts} failed: {e}");
                last_error = Some(e);
                if attempt < max_attempts {
                    eprintln!("🔄 Retrying...");
                }
            }
        }
    }

    Err(last_error.expect("Max attempts must be at least 1"))
}

/// Async version of [`retry_until_success`].
///
/// # Errors
///
/// Returns the last error produced by the closure if all `max_attempts` fail.
///
/// # Panics
///
/// Panics if `max_attempts` is 0.
pub async fn retry_until_success_async<F, Fut, T, E>(
    max_attempts: u8,
    mut f: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut last_error: Option<E> = None;

    for attempt in 1..=max_attempts {
        match f().await {
            Ok(val) => {
                if attempt > 1 {
                    eprintln!("✅ Passed on attempt {attempt}/{max_attempts}");
                }
                return Ok(val);
            }
            Err(e) => {
                eprintln!("⚠️ Attempt {attempt}/{max_attempts} failed: {e}");
                last_error = Some(e);
                if attempt < max_attempts {
                    eprintln!("🔄 Retrying...");
                }
            }
        }
    }

    Err(last_error.expect("Max attempts must be at least 1"))
}
