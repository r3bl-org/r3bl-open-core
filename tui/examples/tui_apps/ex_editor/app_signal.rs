// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::fmt::Debug;

#[derive(Default, Clone, Debug)]
#[non_exhaustive]
pub enum AppSignal {
    #[default]
    Noop,
}
