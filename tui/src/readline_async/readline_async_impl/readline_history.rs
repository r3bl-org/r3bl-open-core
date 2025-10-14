// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ArrayBoundsCheck, ArrayOverflowResult, HISTORY_SIZE_MAX,
            core::coordinates::{Index, idx, len}};
use std::collections::VecDeque;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug)]
pub struct History {
    pub entries: VecDeque<String>,
    pub max_size: usize,
    pub sender: UnboundedSender<String>,
    current_position: Option<Index>,
}

impl History {
    #[must_use]
    pub fn new() -> (Self, UnboundedReceiver<String>) {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<String>();
        (
            Self {
                entries: VecDeque::default(),
                max_size: HISTORY_SIZE_MAX,
                sender,
                current_position: Option::default(),
            },
            receiver,
        )
    }
}

impl History {
    // Update history entries.
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

    // Find next history that matches a given string from an index.
    pub fn search_next(&mut self) -> Option<&str> {
        if let Some(index) = &mut self.current_position {
            let entries_length = len(self.entries.len());
            let next_index: Index = *index + 1;
            if next_index.overflows(entries_length) == ArrayOverflowResult::Within {
                *index = next_index;
            }
            Some(&self.entries[index.as_usize()])
        } else if !self.entries.is_empty() {
            self.current_position = Some(idx(0));
            Some(&self.entries[0])
        } else {
            None
        }
    }

    // Find previous history item that matches a given string from an index.
    pub fn search_previous(&mut self) -> Option<&str> {
        if let Some(index) = &mut self.current_position {
            if *index == idx(0) {
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

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_update() {
        let (mut history, _) = History::new();
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

    // write tests for search_next and search_previous.
    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_search_next() {
        let (mut history, _) = History::new();
        history.max_size = 2;
        history.update(Some("test1".into()));
        history.update(Some("test2".into()));
        history.update(Some("test3".into()));

        assert_eq!(history.search_next(), Some("test3"));
        assert_eq!(history.search_next(), Some("test2"));
        assert_eq!(history.search_next(), Some("test2"));
        assert_eq!(history.search_next(), Some("test2"));
    }

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_search_previous() {
        let (mut history, _) = History::new();
        history.max_size = 2;
        history.update(Some("test1".into()));
        history.update(Some("test2".into()));
        history.update(Some("test3".into()));

        assert_eq!(history.search_previous(), None);
        assert_eq!(history.search_next(), Some("test3"));
        assert_eq!(history.search_previous(), Some(""));
        assert_eq!(history.search_previous(), None);
    }
}
