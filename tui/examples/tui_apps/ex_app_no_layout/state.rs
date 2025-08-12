// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::fmt::{Debug, Display, Formatter};

/// Action.
#[derive(Default, Clone, Debug)]
#[non_exhaustive]
#[allow(dead_code)]
pub enum AppSignal {
    Add,
    Sub,
    Clear,
    #[default]
    Noop,
}

/// State.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct State {
    pub counter: isize,
}

impl Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "State {{ counter: {:?} }}", self.counter)
    }
}

impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "State[counter={}]", self.counter)
    }
}
