// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

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
