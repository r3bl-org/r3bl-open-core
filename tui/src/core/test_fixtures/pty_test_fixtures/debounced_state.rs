// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Convenience wrapper combining async debounced deadline with buffered state.

use super::async_debounced_deadline::AsyncDebouncedDeadline;
use std::time::Duration;

/// Convenience wrapper combining [`AsyncDebouncedDeadline`] with buffered state.
///
/// This is a higher-level abstraction over `AsyncDebouncedDeadline` that couples
/// the debounce timer with a buffered value. Use this when you want to buffer
/// rapid events and take action after a period of inactivity.
///
/// # Use Case
///
/// **Good for:** Buffering rapid events before taking action
/// - PTY test slave: batch input events before printing line state
/// - Rate limiting: buffer API calls and flush after quiet period
/// - Coalescing UI updates: batch rapid state changes
///
/// **Not good for:** Debouncing without state buffering
/// - Use [`AsyncDebouncedDeadline`] directly if you don't need to store a value
///
/// # Integration with `tokio::select`!
///
/// ```no_run
/// use std::time::Duration;
/// use r3bl_tui::DebouncedState;
///
/// # async fn example() {
/// let mut buffered = DebouncedState::new(Duration::from_millis(10));
///
/// loop {
///     tokio::select! {
///         event = read_event() => {
///             // Buffer state and reset timer on each event
///             buffered.set(format!("Event: {event:?}"));
///         }
///         () = buffered.sleep_until(), if buffered.is_pending() => {
///             // No events for 10ms, take action
///             if let Some(state) = buffered.take() {
///                 println!("{state}");
///             }
///         }
///     }
/// }
/// # async fn read_event() -> String { String::new() }
/// # }
/// ```
#[derive(Debug)]
pub struct DebouncedState<T> {
    debounce: AsyncDebouncedDeadline,
    pending: Option<T>,
}

impl<T> DebouncedState<T> {
    /// Creates a new debounced state with the given duration.
    ///
    /// The state starts empty (no pending value). Call [`set()`]
    /// to buffer a value and start the debounce timer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use r3bl_tui::DebouncedState;
    ///
    /// let buffered: DebouncedState<String> =
    ///     DebouncedState::new(Duration::from_millis(10));
    /// assert!(!buffered.is_pending());
    /// ```
    ///
    /// [`set()`]: Self::set
    #[must_use]
    pub fn new(duration: Duration) -> Self {
        Self {
            debounce: AsyncDebouncedDeadline::new(duration),
            pending: None,
        }
    }

    /// Buffers a value and resets the debounce timer.
    ///
    /// If called repeatedly before the timer expires, the value is updated
    /// and the timer is reset, effectively delaying the action until
    /// `duration` elapses with no calls to `set()`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use r3bl_tui::DebouncedState;
    ///
    /// let mut buffered = DebouncedState::new(Duration::from_millis(10));
    /// buffered.set("first".to_string());
    /// assert!(buffered.is_pending());
    ///
    /// // Update value and reset timer
    /// buffered.set("second".to_string());
    /// assert!(buffered.is_pending());
    /// ```
    pub fn set(&mut self, value: T) {
        self.pending = Some(value);
        self.debounce.reset();
    }

    /// Takes the buffered value and clears the debounce timer.
    ///
    /// Returns `Some(value)` if a value was buffered, or `None` if no value
    /// was pending. After calling this, [`is_pending()`]
    /// will return `false`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use r3bl_tui::DebouncedState;
    ///
    /// let mut buffered = DebouncedState::new(Duration::from_millis(10));
    /// buffered.set("hello".to_string());
    ///
    /// assert_eq!(buffered.take(), Some("hello".to_string()));
    /// assert!(!buffered.is_pending());
    /// assert_eq!(buffered.take(), None); // Already taken
    /// ```
    ///
    /// [`is_pending()`]: Self::is_pending
    pub fn take(&mut self) -> Option<T> {
        self.debounce.clear();
        self.pending.take()
    }

    /// Returns `true` if there is a buffered value pending.
    ///
    /// Use this as the condition in `tokio::select!` branches:
    /// ```no_run
    /// # use std::time::Duration;
    /// # use r3bl_tui::DebouncedState;
    /// # async fn example(buffered: DebouncedState<String>) {
    /// tokio::select! {
    ///     () = buffered.sleep_until(), if buffered.is_pending() => { /* Handle timeout */ }
    /// }
    /// # }
    /// ```
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use r3bl_tui::DebouncedState;
    ///
    /// let mut buffered = DebouncedState::new(Duration::from_millis(10));
    /// assert!(!buffered.is_pending());
    ///
    /// buffered.set("hello".to_string());
    /// assert!(buffered.is_pending());
    /// ```
    #[must_use]
    pub fn is_pending(&self) -> bool { self.debounce.is_pending() }

    /// Returns `true` if the debounce is active and should be polled in `tokio::select!`.
    ///
    /// This is an alias for [`is_pending()`] with clearer semantics
    /// for use in select branches. Use this when you want your code to read naturally:
    ///
    /// ```no_run
    /// # use std::time::Duration;
    /// # use r3bl_tui::DebouncedState;
    /// # async fn example(mut buffered_state: DebouncedState<String>) {
    /// tokio::select! {
    ///     () = buffered_state.sleep_until(), if buffered_state.should_poll() => {
    ///         if let Some(state) = buffered_state.take() {
    ///             println!("{state}");
    ///         }
    ///     }
    /// }
    /// # }
    /// ```
    ///
    /// Reads in English as: "If we should poll the debounced state, then sleep until
    /// the debounce timer expires, and when it fires, execute this code."
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use r3bl_tui::DebouncedState;
    ///
    /// let mut buffered = DebouncedState::new(Duration::from_millis(10));
    /// assert!(!buffered.should_poll()); // No active debounce
    ///
    /// buffered.set("hello".to_string());
    /// assert!(buffered.should_poll()); // Debounce is active
    /// ```
    ///
    /// [`is_pending()`]: Self::is_pending
    #[must_use]
    pub fn should_poll(&self) -> bool { self.is_pending() }

    /// Sleeps until the debounce deadline expires.
    ///
    /// - If a value is buffered, sleeps until the deadline
    /// - If no value is buffered, returns a pending future (never completes)
    ///
    /// This is designed to be used in `tokio::select!` branches with
    /// an `if` condition to prevent spurious wakeups:
    ///
    /// ```no_run
    /// # use std::time::Duration;
    /// # use r3bl_tui::DebouncedState;
    /// # async fn example(mut buffered: DebouncedState<String>) {
    /// tokio::select! {
    ///     () = buffered.sleep_until(), if buffered.is_pending() => {
    ///         if let Some(state) = buffered.take() {
    ///             println!("{state}");
    ///         }
    ///     }
    /// }
    /// # }
    /// ```
    pub async fn sleep_until(&self) { self.debounce.sleep_until().await; }
}
