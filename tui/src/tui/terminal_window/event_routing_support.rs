// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// This works w/ the input event routing system to provide the caller w/ information
/// about whether an even has been consumed or not. If it has been consumed, is a render
/// necessary.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EventPropagation {
    ConsumedRender,
    Consumed,
    Propagate,
    ExitMainEventLoop,
}
