// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use r3bl_tui::InlineVec;
use smallvec::smallvec;
use std::fmt::{Debug, Display, Formatter};

/// State.
#[derive(Clone, PartialEq, Eq)]
pub struct State {
    pub stack: InlineVec<i32>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            stack: smallvec![0],
        }
    }
}

impl Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "State {{ stack: {:?} }}", self.stack)
    }
}

impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "State[stack_size={}]", self.stack.len())
    }
}

/// Action.
#[derive(Default, Clone, Debug)]
#[non_exhaustive]
#[allow(dead_code)]
pub enum AppSignal {
    Startup,
    AddPop(i32),
    SubPop(i32),
    Clear,
    #[default]
    Noop,
}
