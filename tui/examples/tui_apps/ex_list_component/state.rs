// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::fmt::{Display, Formatter};

/// Application state for the TodoList demo.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AppState {
    pub status_message: String,
}

impl Display for AppState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "AppState[status_len={}]", self.status_message.len())
    }
}

/// Application signals for the TodoList demo.
#[derive(Debug, Clone, Default)]
pub enum AppSignal {
    #[default]
    Noop,
}
