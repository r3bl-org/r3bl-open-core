/*
 *   Copyright (c) 2024 R3BL LLC
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

use std::fmt::Debug;

use crate::{DialogEngine, EditorBuffer, EditorEngine, FlexBoxId, GlobalData, HasFocus};

pub struct RenderArgs<'a> {
    pub editor_engine: &'a mut EditorEngine,
    pub editor_buffer: &'a EditorBuffer,
    pub has_focus: &'a mut HasFocus,
}

pub struct EditorArgsMut<'a> {
    pub editor_engine: &'a mut EditorEngine,
    pub editor_buffer: &'a mut EditorBuffer,
}

pub struct EditorArgs<'a> {
    pub editor_engine: &'a EditorEngine,
    pub editor_buffer: &'a EditorBuffer,
}

/// [DialogEngine] args struct that holds references.
///
/// ![Editor component lifecycle
/// diagram](https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/docs/memory-architecture.drawio.svg)
pub struct DialogEngineArgs<'a, S, AS>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    pub self_id: FlexBoxId,
    pub global_data: &'a mut GlobalData<S, AS>,
    pub dialog_engine: &'a mut DialogEngine,
    pub has_focus: &'a mut HasFocus,
}
