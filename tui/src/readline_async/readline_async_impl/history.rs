// Copyright (c) 2024-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ArrayBoundsCheck, ArrayOverflowResult, HISTORY_SIZE_MAX, NarrowingCastToU16,
            VPIndex, vp_idx, vp_len};
use std::collections::VecDeque;

/// The [`History`] struct manages the input history for the [`Readline`] line editor.
///
/// It stores lines of text entered by the user and enables backward and forward
/// navigation (when the user presses `Up` or `Down` arrow keys).
///
/// In most cases, you do not interact with this struct directly. Instead, you record
/// input history by calling [`Readline::add_history_entry()`] on
/// [`ReadlineAsyncContext::readline`].
///
/// Readline does not automatically add submitted lines to history. This gives the
/// application explicit control over which commands to record (e.g., ignoring empty
/// inputs or sensitive commands).
///
/// # Example Usage
///
/// ```no_run
/// use r3bl_tui::{readline_async::ReadlineAsyncContext, IntoErr, ReadlineEvent,
///     TuiAvailability, ok};
///
/// #[tokio::main]
/// async fn main() -> miette::Result<()> {
///     // 1. Initialize the async readline context.
///     let mut ctx = match ReadlineAsyncContext::try_new(Some("> "), None).await {
///         TuiAvailability::Available(ctx) => ctx,
///         it => return it.into_err(),
///     };
///
///     // 2. Read lines in an event loop.
///     loop {
///         match ctx.read_line().await? {
///             ReadlineEvent::Line(line) => {
///                 if line == "exit" {
///                     break;
///                 }
///
///                 // 3. Record the line in history so the user can recall it via
///                 // Up/Down arrow keys.
///                 ctx.readline.add_history_entry(line);
///             }
///             ReadlineEvent::Eof | ReadlineEvent::Interrupted => break,
///             _ => {}
///         }
///     }
///
///     ctx.request_shutdown(None).await?;
///     ctx.await_shutdown().await;
///     ok!()
/// }
/// ```
///
/// [`Readline::add_history_entry()`]: crate::Readline::add_history_entry
/// [`Readline`]: crate::Readline
/// [`ReadlineAsyncContext::readline`]: crate::ReadlineAsyncContext::readline
/// [`ReadlineAsyncContext`]: crate::ReadlineAsyncContext
#[derive(Debug)]
pub struct History {
    /// Ordered ring buffer of past input lines (newest at front, oldest at back).
    entries: VecDeque<String>,

    /// Maximum number of history entries retained before oldest entries are evicted.
    max_size: usize,

    /// Active 0-based navigation index while stepping through history (`Up`/`Down`
    /// arrows). [`None`] indicates the user is at the live/current prompt line.
    current_position: Option<VPIndex>,
}

impl Default for History {
    fn default() -> Self {
        Self {
            entries: VecDeque::default(),
            max_size: HISTORY_SIZE_MAX,
            current_position: Option::default(),
        }
    }
}

impl History {
    /// Creates a new, empty [`History`] instance with the default max size.
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Updates the maximum history capacity and truncates older entries if necessary.
    pub fn set_max_size(&mut self, max_size: usize) {
        self.max_size = max_size;
        self.entries.truncate(max_size);
        if let Some(pos) = self.current_position
            && pos.as_usize() >= self.entries.len()
        {
            self.current_position = None;
        }
    }
}

impl History {
    /// Updates the history by adding a new line to the front.
    ///
    /// If the line is empty or identical to the most recent entry, it is ignored.
    /// If adding the entry exceeds `max_size`, the oldest entry is removed.
    /// Adding a new entry also resets the current navigation position.
    pub fn update(&mut self, maybe_line: Option<String>) {
        // Receive a new line.
        if let Some(line) = maybe_line {
            // Don't add entry if last entry was same, or line was empty.
            if self.entries.front() == Some(&line) || line.is_empty() {
                return;
            }
            // Add entry to front of history.
            self.entries.push_front(line);

            // Reset offset to newest entry.
            self.current_position = None;

            // Check if already have enough entries.
            if self.entries.len() > self.max_size {
                // Remove oldest entry.
                self.entries.pop_back();
            }
        }
    }

    /// Navigates backwards in time (older entries) through the history.
    ///
    /// Returns the next older entry in the history, or the oldest entry if already at the
    /// end. This corresponds to pressing the Up arrow key in the [`Readline`] line
    /// editor.
    ///
    /// [`Readline`]: crate::Readline
    pub fn search_next(&mut self) -> Option<&str> {
        if let Some(index) = &mut self.current_position {
            let entries_length = vp_len((self.entries.len()).as_u16_narrowing());
            let next_index: VPIndex = *index + 1;
            if next_index.overflows(entries_length) == ArrayOverflowResult::Within {
                *index = next_index;
            }
            Some(&self.entries[index.as_usize()])
        } else if !self.entries.is_empty() {
            self.current_position = Some(vp_idx(0u16));
            Some(&self.entries[0])
        } else {
            None
        }
    }

    /// Navigates forwards in time (newer entries) through the history.
    ///
    /// Returns the next newer entry in the history. If navigating past the newest entry,
    /// it returns an empty string (`""`) to signify exiting the history and returning to
    /// the current line.
    /// This corresponds to pressing the Down arrow key in the [`Readline`] line editor.
    ///
    /// [`Readline`]: crate::Readline
    pub fn search_previous(&mut self) -> Option<&str> {
        if let Some(index) = &mut self.current_position {
            if *index == vp_idx(0u16) {
                self.current_position = None;
                return Some("");
            }
            *index -= 1;
            Some(&self.entries[index.as_usize()])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::needless_return)]
    fn test_update() {
        let mut history = History::new();
        history.max_size = 2;
        history.update(Some("test1".into()));
        assert_eq!(history.entries.front(), Some(&"test1".to_string()));

        history.update(None);
        assert_eq!(history.entries.front(), Some(&"test1".to_string()));

        history.update(Some("test1".into()));
        assert_eq!(history.entries.front(), Some(&"test1".to_string()));

        history.update(Some("test2".into()));
        assert_eq!(history.entries.front(), Some(&"test2".to_string()));

        assert_eq!(history.entries.len(), 2);

        history.update(Some("test3".into()));
        assert_eq!(history.entries.len(), 2);
        assert!(history.entries.contains(&"test2".to_string()));
        assert!(history.entries.contains(&"test3".to_string()));
    }

    // Write tests for search_next and search_previous.
    #[test]
    #[allow(clippy::needless_return)]
    fn test_search_next() {
        let mut history = History::new();
        history.max_size = 2;
        history.update(Some("test1".into()));
        history.update(Some("test2".into()));
        history.update(Some("test3".into()));

        assert_eq!(history.search_next(), Some("test3"));
        assert_eq!(history.search_next(), Some("test2"));
        assert_eq!(history.search_next(), Some("test2"));
        assert_eq!(history.search_next(), Some("test2"));
    }

    #[test]
    #[allow(clippy::needless_return)]
    fn test_search_previous() {
        let mut history = History::new();
        history.max_size = 2;
        history.update(Some("test1".into()));
        history.update(Some("test2".into()));
        history.update(Some("test3".into()));

        assert_eq!(history.search_previous(), None);
        assert_eq!(history.search_next(), Some("test3"));
        assert_eq!(history.search_previous(), Some(""));
        assert_eq!(history.search_previous(), None);
    }

    #[test]
    #[allow(clippy::needless_return)]
    fn test_set_max_size() {
        let mut history = History::new();
        history.update(Some("cmd1".into()));
        history.update(Some("cmd2".into()));
        history.update(Some("cmd3".into()));
        assert_eq!(history.entries.len(), 3);

        // Truncate by reducing max size.
        history.set_max_size(2);
        assert_eq!(history.entries.len(), 2);
        assert_eq!(history.entries.front(), Some(&"cmd3".to_string()));
        assert_eq!(history.entries.back(), Some(&"cmd2".to_string()));
    }

    #[test]
    #[allow(clippy::needless_return)]
    fn test_set_max_size_resets_out_of_bounds_position() {
        let mut history = History::new();
        history.update(Some("cmd1".into()));
        history.update(Some("cmd2".into()));
        history.update(Some("cmd3".into()));

        // Navigate to oldest entry (index 2).
        assert_eq!(history.search_next(), Some("cmd3"));
        assert_eq!(history.search_next(), Some("cmd2"));
        assert_eq!(history.search_next(), Some("cmd1"));

        // Shrink capacity to 1, truncating out index 2 and 1.
        history.set_max_size(1);
        assert_eq!(history.entries.len(), 1);

        // Position was reset to None, so search_next starts fresh at index 0.
        assert_eq!(history.search_next(), Some("cmd3"));
    }

    #[test]
    #[allow(clippy::needless_return)]
    fn test_empty_history_search_next() {
        let mut history = History::new();
        // search_next on an empty history returns None.
        assert_eq!(history.search_next(), None);
    }

    #[test]
    #[allow(clippy::needless_return)]
    fn test_ignore_empty_string_update() {
        let mut history = History::new();
        history.update(Some(String::new()));
        assert_eq!(history.entries.len(), 0);
    }

    #[test]
    #[allow(clippy::needless_return)]
    fn test_search_previous_multi_step() {
        let mut history = History::new();
        history.update(Some("first".into()));
        history.update(Some("second".into()));

        // Navigate back twice: "second" (index 0), then "first" (index 1).
        assert_eq!(history.search_next(), Some("second"));
        assert_eq!(history.search_next(), Some("first"));

        // Step forward from index 1 -> index 0 (returns "second").
        assert_eq!(history.search_previous(), Some("second"));

        // Step forward from index 0 -> None (exits history, returns "").
        assert_eq!(history.search_previous(), Some(""));
        assert_eq!(history.search_previous(), None);
    }
}
