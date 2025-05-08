/*
 *   Copyright (c) 2022-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use crate::Size;

/// This works w/ the main event loop to let it know whether it should request_shutdown or
/// resize after an input event has occurred (and has been passed thru the input event
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
