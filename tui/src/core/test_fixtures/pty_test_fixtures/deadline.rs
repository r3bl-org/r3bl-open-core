// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::time::{Duration, Instant};

/// Timeout enforcement for PTY tests.
///
/// Enforces maximum duration for operations with a clean polling API. Use this when
/// you need to ensure an operation completes within a time limit, such as waiting
/// for a PTY slave process to start or for test output to appear.
///
/// # Use Case
///
/// **Good for:** "Exit if this operation takes too long"
/// - Timeout enforcement for test operations
/// - Watchdog timers for subprocess startup
/// - Maximum duration guards for I/O operations
/// - Preventing tests from hanging indefinitely
///
/// **Not good for:** "Do X after Y ms of no activity"
/// - Use [`AsyncDebouncedDeadline`] for debouncing events instead
///
/// # Comparison with AsyncDebouncedDeadline
///
/// | Pattern | `Deadline` | `AsyncDebouncedDeadline` |
/// |---------|-----------|-------------------------|
/// | **Purpose** | Timeout enforcement | Event debouncing |
/// | **Resets?** | No (fixed duration) | Yes (on each event) |
/// | **Runtime** | Sync (`std::time`) | Async (`tokio::time`) |
/// | **Use with** | Polling loops | `tokio::select!` |
/// | **Example** | "Slave must start in 5s" | "Print after 10ms of silence" |
///
/// # Examples
///
/// ## Basic Timeout Enforcement
///
/// ```rust
/// use std::time::Duration;
/// use r3bl_tui::Deadline;
///
/// let deadline = Deadline::default(); // 5 second timeout
///
/// loop {
///     if deadline.is_expired() {
///         panic!("Timeout: operation did not complete within 5 seconds");
///     }
///
///     // ... do work ...
/// #   break; // For doctest
/// }
/// ```
///
/// ## Custom Timeout Duration
///
/// ```rust
/// use std::time::Duration;
/// use r3bl_tui::Deadline;
///
/// // Wait up to 10 seconds for subprocess to start
/// let deadline = Deadline::new(Duration::from_secs(10));
///
/// loop {
///     if deadline.is_expired() {
///         panic!("Subprocess did not start in time");
///     }
///
///     // Check if subprocess is ready...
/// #   break;
/// }
/// ```
///
/// ## Readable Assertions
///
/// ```rust
/// use std::time::Duration;
/// use r3bl_tui::Deadline;
///
/// let deadline = Deadline::new(Duration::from_secs(5));
///
/// loop {
///     assert!(
///         deadline.has_time_remaining(),
///         "Timeout: slave did not start within 5 seconds"
///     );
///
///     // ... check for completion ...
/// #   break;
/// }
/// ```
///
/// [`AsyncDebouncedDeadline`]: crate::AsyncDebouncedDeadline
#[derive(Debug, Clone, Copy)]
pub struct Deadline {
    expires_at: Instant,
}

impl Deadline {
    /// Creates a new deadline that expires after the given duration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use r3bl_tui::Deadline;
    ///
    /// let deadline = Deadline::default();
    /// assert!(deadline.has_time_remaining());
    /// ```
    #[must_use]
    pub fn new(timeout: Duration) -> Self {
        Self {
            expires_at: Instant::now() + timeout,
        }
    }

    /// Returns `true` if the deadline has expired.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use r3bl_tui::Deadline;
    ///
    /// let deadline = Deadline::new(Duration::from_millis(1));
    /// std::thread::sleep(Duration::from_millis(2));
    /// assert!(deadline.is_expired());
    /// ```
    #[must_use]
    pub fn is_expired(&self) -> bool { Instant::now() >= self.expires_at }

    /// Returns `true` if there is still time remaining before the deadline expires.
    ///
    /// This is the inverse of [`is_expired()`](Self::is_expired) and provides
    /// more readable assertions in tests, clearly expressing "we still have time to
    /// complete the operation."
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use r3bl_tui::Deadline;
    ///
    /// let deadline = Deadline::new(Duration::from_secs(10));
    /// assert!(deadline.has_time_remaining(), "We should still have time");
    /// ```
    #[must_use]
    pub fn has_time_remaining(&self) -> bool { !self.is_expired() }
}

impl Default for Deadline {
    /// Creates a deadline with a default timeout of 5 seconds.
    ///
    /// This is a sensible default for PTY test slave process startup timeouts.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use r3bl_tui::Deadline;
    ///
    /// let deadline = Deadline::default();
    /// assert!(deadline.has_time_remaining());
    /// ```
    fn default() -> Self { Self::new(Duration::from_secs(5)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deadline_not_expired_immediately() {
        let deadline = Deadline::new(Duration::from_secs(10));
        assert!(
            deadline.has_time_remaining(),
            "Deadline should not expire immediately"
        );
    }

    #[test]
    fn test_deadline_expires_after_duration() {
        let deadline = Deadline::new(Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(5));
        assert!(
            deadline.is_expired(),
            "Deadline should be expired after waiting"
        );
    }

    #[test]
    fn test_deadline_is_copy() {
        let deadline1 = Deadline::default();
        let deadline2 = deadline1; // Copy

        // Both should work independently
        assert!(!deadline1.is_expired());
        assert!(!deadline2.is_expired());
    }

    #[test]
    fn test_deadline_can_be_cloned() {
        let deadline1 = Deadline::default();
        let deadline2 = deadline1;

        assert!(!deadline1.is_expired());
        assert!(!deadline2.is_expired());
    }

    #[test]
    fn test_deadline_zero_duration() {
        let deadline = Deadline::new(Duration::from_secs(0));
        // Should be expired immediately (or very close to it)
        std::thread::sleep(Duration::from_millis(1));
        assert!(
            deadline.is_expired(),
            "Zero duration deadline should expire immediately"
        );
    }

    #[test]
    fn test_multiple_checks_on_same_deadline() {
        let deadline = Deadline::new(Duration::from_millis(50));

        // First check - not expired
        assert!(deadline.has_time_remaining());

        // Wait and check again
        std::thread::sleep(Duration::from_millis(60));

        // Multiple checks after expiration
        assert!(deadline.is_expired());
        assert!(deadline.is_expired()); // Still expired
        assert!(deadline.is_expired()); // Still expired
    }
}
