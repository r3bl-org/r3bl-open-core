// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::time::{Duration, Instant};

/// Simple timeout utility for PTY tests.
///
/// Provides a clean API for timeout handling in PTY-based integration tests.
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use r3bl_tui::Deadline;
///
/// let deadline = Deadline::default();
///
/// loop {
///     if deadline.is_expired() {
///         panic!("Timeout: operation did not complete");
///     }
///     // ... do work ...
/// #   break; // For doctest
/// }
/// ```
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
