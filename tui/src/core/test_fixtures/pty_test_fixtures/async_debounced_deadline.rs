// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Async debounced deadline for "do X after Y ms of no activity" pattern.

use std::time::Duration;

/// Async debounced deadline for "do X after Y ms of no activity" pattern.
///
/// This is useful for batching rapid input events in PTY tests. When events arrive
/// rapidly (e.g., "hello world" as 11 individual character events), you want to
/// process all of them before taking action (e.g., printing line state).
///
/// # Use Case
///
/// **Good for:** "Do X after Y ms of no activity"
/// - Batch rapid input events before printing output
/// - Debounce user input in interactive tests
/// - Coalescescing multiple rapid events into single response
///
/// **Not good for:** "Exit if this operation takes too long"
/// - Use [`Deadline`] for timeout enforcement instead
///
/// # Integration with `tokio::select`!
///
/// ```rust,no_run
/// use std::time::Duration;
/// use r3bl_tui::AsyncDebouncedDeadline;
///
/// # async fn example() {
/// let mut debounce = AsyncDebouncedDeadline::new(Duration::from_millis(10));
/// let mut pending_state: Option<String> = None;
///
/// loop {
///     tokio::select! {
///         event = read_event() => {
///             // Process event and reset debounce timer
///             pending_state = Some(format!("State: {event:?}"));
///             debounce.reset();
///         }
///         () = debounce.sleep_until(), if debounce.is_pending() => {
///             // No events for 10ms, print buffered state
///             if let Some(state) = pending_state.take() {
///                 println!("{state}");
///             }
///             debounce.clear();
///         }
///     }
/// }
/// # async fn read_event() -> String { String::new() }
/// # }
/// ```
///
/// [`Deadline`]: crate::Deadline
#[derive(Debug, Clone)]
pub struct AsyncDebouncedDeadline {
    /// The current deadline, if set. None means no deadline pending.
    deadline: Option<tokio::time::Instant>,
    /// The debounce duration (how long to wait after last activity).
    duration: Duration,
}

impl AsyncDebouncedDeadline {
    /// Creates a new debounced deadline with the given duration.
    ///
    /// The deadline starts as `None` (not pending). Call [`reset()`](Self::reset)
    /// when an event occurs to start the debounce timer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use r3bl_tui::AsyncDebouncedDeadline;
    ///
    /// let debounce = AsyncDebouncedDeadline::new(Duration::from_millis(10));
    /// assert!(!debounce.is_pending()); // No deadline set initially
    /// ```
    #[must_use]
    pub fn new(duration: Duration) -> Self {
        Self {
            deadline: None,
            duration,
        }
    }

    /// Resets the deadline to `now + duration`.
    ///
    /// Call this when an event occurs to restart the debounce timer.
    /// If called repeatedly before the timer expires, the action is delayed
    /// until `duration` elapses with no calls to `reset()`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use r3bl_tui::AsyncDebouncedDeadline;
    ///
    /// let mut debounce = AsyncDebouncedDeadline::new(Duration::from_millis(10));
    /// assert!(!debounce.is_pending());
    ///
    /// debounce.reset(); // Start timer
    /// assert!(debounce.is_pending());
    /// ```
    pub fn reset(&mut self) {
        self.deadline = Some(tokio::time::Instant::now() + self.duration);
    }

    /// Clears the deadline (sets to `None`).
    ///
    /// Call this after the debounced action completes to stop waiting.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use r3bl_tui::AsyncDebouncedDeadline;
    ///
    /// let mut debounce = AsyncDebouncedDeadline::new(Duration::from_millis(10));
    /// debounce.reset();
    /// assert!(debounce.is_pending());
    ///
    /// debounce.clear();
    /// assert!(!debounce.is_pending());
    /// ```
    pub fn clear(&mut self) { self.deadline = None; }

    /// Returns `true` if there is a deadline pending.
    ///
    /// Use this as the condition in `tokio::select!` branches:
    /// ```rust,ignore
    /// () = debounce.sleep_until(), if debounce.is_pending() => { ... }
    /// ```
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use r3bl_tui::AsyncDebouncedDeadline;
    ///
    /// let mut debounce = AsyncDebouncedDeadline::new(Duration::from_millis(10));
    /// assert!(!debounce.is_pending());
    ///
    /// debounce.reset();
    /// assert!(debounce.is_pending());
    /// ```
    #[must_use]
    pub fn is_pending(&self) -> bool { self.deadline.is_some() }

    /// Returns the current deadline as `Option<Instant>`.
    ///
    /// Useful for direct access to the deadline in advanced use cases.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use r3bl_tui::AsyncDebouncedDeadline;
    ///
    /// let mut debounce = AsyncDebouncedDeadline::new(Duration::from_millis(10));
    /// assert!(debounce.get().is_none());
    ///
    /// debounce.reset();
    /// assert!(debounce.get().is_some());
    /// ```
    #[must_use]
    pub fn get(&self) -> Option<tokio::time::Instant> { self.deadline }

    /// Sleeps until the deadline expires.
    ///
    /// - If deadline is `Some(instant)`, sleeps until that instant
    /// - If deadline is `None`, returns a pending future (never completes)
    ///
    /// This is designed to be used in `tokio::select!` branches with
    /// an `if` condition to prevent spurious wakeups:
    ///
    /// ```rust,ignore
    /// () = debounce.sleep_until(), if debounce.is_pending() => {
    ///     // Deadline fired!
    ///     debounce.clear();
    /// }
    /// ```
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use std::time::Duration;
    /// use r3bl_tui::AsyncDebouncedDeadline;
    ///
    /// # tokio_test::block_on(async {
    /// let mut debounce = AsyncDebouncedDeadline::new(Duration::from_millis(10));
    /// debounce.reset();
    ///
    /// // This will complete after ~10ms
    /// debounce.sleep_until().await;
    /// # });
    /// ```
    pub async fn sleep_until(&self) {
        match self.deadline {
            Some(deadline) => tokio::time::sleep_until(deadline).await,
            None => std::future::pending().await, // Never completes
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_starts_not_pending() {
        let debounce = AsyncDebouncedDeadline::new(Duration::from_millis(10));
        assert!(!debounce.is_pending());
        assert!(debounce.get().is_none());
    }

    #[test]
    fn test_reset_sets_pending() {
        let mut debounce = AsyncDebouncedDeadline::new(Duration::from_millis(10));
        debounce.reset();
        assert!(debounce.is_pending());
        assert!(debounce.get().is_some());
    }

    #[test]
    fn test_clear_removes_pending() {
        let mut debounce = AsyncDebouncedDeadline::new(Duration::from_millis(10));
        debounce.reset();
        assert!(debounce.is_pending());

        debounce.clear();
        assert!(!debounce.is_pending());
        assert!(debounce.get().is_none());
    }

    #[test]
    fn test_multiple_resets() {
        let mut debounce = AsyncDebouncedDeadline::new(Duration::from_millis(10));

        debounce.reset();
        let first = debounce.get().unwrap();

        std::thread::sleep(Duration::from_millis(5));

        debounce.reset();
        let second = debounce.get().unwrap();

        // Second deadline should be later than first
        assert!(second > first);
    }

    #[tokio::test]
    async fn test_sleep_until_completes_after_duration() {
        let mut debounce = AsyncDebouncedDeadline::new(Duration::from_millis(10));
        debounce.reset();

        let start = tokio::time::Instant::now();
        debounce.sleep_until().await;
        let elapsed = start.elapsed();

        // Should take approximately 10ms (allow some tolerance)
        assert!(
            elapsed >= Duration::from_millis(8) && elapsed <= Duration::from_millis(20),
            "Expected ~10ms, got {:?}",
            elapsed
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_reset_extends_deadline() {
        let mut debounce = AsyncDebouncedDeadline::new(Duration::from_millis(20));
        debounce.reset();

        // Wait 10ms, then reset (should extend deadline)
        tokio::time::sleep(Duration::from_millis(10)).await;
        debounce.reset();

        let start = tokio::time::Instant::now();
        debounce.sleep_until().await;
        let elapsed = start.elapsed();

        // Should take ~20ms from second reset (not 10ms from first)
        assert!(
            elapsed >= Duration::from_millis(15),
            "Expected >= 15ms, got {:?}",
            elapsed
        );
    }
}
