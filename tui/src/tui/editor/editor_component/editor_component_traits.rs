/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

use crate::{EditorBuffer, FlexBoxId};

/// This marker trait is meant to be implemented by whatever state struct is being used to
/// store the editor buffer for this re-usable editor component.
///
/// It is used in the `where` clause of the [`crate::EditorComponent`] to ensure that the
/// generic type `S` implements this trait, guaranteeing that it holds a hash map of
/// [`EditorBuffer`]s w/ key of [`FlexBoxId`].
pub trait HasEditorBuffers {
    fn get_mut_editor_buffer(&mut self, id: FlexBoxId) -> Option<&mut EditorBuffer>;
    fn insert_editor_buffer(&mut self, id: FlexBoxId, buffer: EditorBuffer);
    fn contains_editor_buffer(&self, id: FlexBoxId) -> bool;
}
