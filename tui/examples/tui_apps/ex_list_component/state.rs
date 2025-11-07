// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::fmt::{Display, Formatter};

/// Display mode for the list component example.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayMode {
    /// Simple list items with single-line rendering (Phase 1)
    Simple,
    /// Complex list items with multi-line FlexBox layouts (Phase 3)
    Complex,
}

impl Default for DisplayMode {
    fn default() -> Self {
        Self::Simple
    }
}

/// Application state for the TodoList demo.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AppState {
    pub status_message: String,
    pub display_mode: DisplayMode,
}

impl Display for AppState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AppState[mode={:?}, status_len={}]",
            self.display_mode,
            self.status_message.len()
        )
    }
}

/// Application signals for the TodoList demo.
#[derive(Debug, Clone, Default)]
pub enum AppSignal {
    #[default]
    Noop,
}
