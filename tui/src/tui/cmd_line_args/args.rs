// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{DialogEngine, EditorBuffer, EditorEngine, FlexBoxId, GlobalData, HasFocus};
use std::fmt::Debug;

#[derive(Debug)]
pub struct RenderArgs<'a> {
    pub engine: &'a mut EditorEngine,
    pub buffer: &'a EditorBuffer,
    pub has_focus: &'a mut HasFocus,
}

#[derive(Debug)]
pub struct EditorArgsMut<'a> {
    pub engine: &'a mut EditorEngine,
    pub buffer: &'a mut EditorBuffer,
}

/// [`DialogEngine`] args struct that holds references.
///
/// ![Editor component lifecycle
/// diagram](https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/docs/memory-architecture.drawio.svg)
#[derive(Debug)]
pub struct DialogEngineArgs<'a, S, AS>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    pub self_id: FlexBoxId,
    pub global_data: &'a mut GlobalData<S, AS>,
    pub engine: &'a mut DialogEngine,
    pub has_focus: &'a mut HasFocus,
}
