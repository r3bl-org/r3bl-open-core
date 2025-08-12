// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::Size;

/// This works w/ the main event loop to let it know whether it should `request_shutdown`
/// or resize after an input event has occurred (and has been passed thru the input event
/// routing system).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Continuation<T> {
    Exit,
    Continue,
    ResizeAndContinue(Size),
    Return,
    Break,
    Result(T),
}

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
