/*
 *   Copyright (c) 2022 R3BL LLC
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

use std::{collections::HashMap, fmt::Debug};

use r3bl_rs_utils_core::*;

use crate::*;

/// There are certain fields that need to be in each state struct to represent global information
/// about keyboard focus.
///
/// 1. An `id` [FlexBoxId] is used to store which [FlexBox] id currently holds keyboard focus. This
///    is global.
/// 2. Each `id` may have a [Position] associated with it, which is used to draw the "cursor" (the
///    meaning of which depends on the specific [Component] impl). This cursor is scoped to each
///    `id` so it isn't strictly a single global value (like `id` itself). Here are examples of what
///    a "cursor" might mean for various [Component]s:
///    - for an editor, it will be the insertion point where text is added / removed
///    - for a text viewer, it will be the cursor position which can be moved around
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct HasFocus {
    /// Map of `id` to its [Position]. Each cursor ([Position]) is scoped to an `id`. The map is
    /// global.
    pub cursor_position_map: CursorPositionMap,

    /// This `id` has keyboard focus. This is global.
    pub maybe_id: Option<FlexBoxId>,

    /// This `id` is saved only when [set_modal_id](HasFocus::set_modal_id) is called. This is global.
    pub maybe_id_before_modal: Option<FlexBoxId>,
}

pub type CursorPositionMap = HashMap<FlexBoxId, Option<Position>>;

impl HasFocus {
    /// Get the `id` of the [FlexBox] that has keyboard focus.
    pub fn get_id(&self) -> Option<FlexBoxId> { self.maybe_id }

    /// Check to see whether [set_id][HasFocus::set_id] has been called.
    pub fn is_empty(&self) -> bool { self.maybe_id.is_none() }

    /// Check to see whether [set_id][HasFocus::set_id] has been called.
    pub fn is_set(&self) -> bool { !self.is_empty() }

    /// Set the `id` of the [FlexBox] that has keyboard focus.
    pub fn set_id(&mut self, id: FlexBoxId) { self.maybe_id = Some(id) }

    /// Check whether the given `id` currently has keyboard focus.
    pub fn does_id_have_focus(&self, id: FlexBoxId) -> bool { self.maybe_id == Some(id) }

    /// Check whether the `id` of the [FlexBox] currently has keyboard focus.
    pub fn does_current_box_have_focus(&self, current_box: &FlexBox) -> bool {
        self.does_id_have_focus(current_box.id)
    }
}

impl HasFocus {
    /// Saves the current `id` to `prev_id` and sets `id` to the given `id`.
    pub fn set_modal_id(&mut self, id: FlexBoxId) {
        self.maybe_id_before_modal = self.maybe_id;
        self.set_id(id);
    }

    /// Checks whether any modal `id` is set.
    pub fn is_modal_set(&self) -> bool { self.maybe_id_before_modal.is_some() }

    /// Checks whether the given `id` is the modal `id`.
    pub fn is_modal_id(&self, id: FlexBoxId) -> bool {
        self.is_modal_set() && self.does_id_have_focus(id)
    }

    /// Restores the `id` from `prev_id` and sets `prev_id` to `None`.
    pub fn reset_modal_id(&mut self) -> Option<FlexBoxId> {
        if let Some(prev_id) = self.maybe_id_before_modal {
            self.maybe_id = Some(prev_id);
            self.maybe_id_before_modal = None;
            Some(prev_id)
        } else {
            None
        }
    }
}

impl HasFocus {
    /// For a given [FlexBox] `id`, set the position of the cursor inside of it.
    pub fn set_cursor_position_for_id(&mut self, id: FlexBoxId, maybe_position: Option<Position>) {
        let map = &mut self.cursor_position_map;
        map.insert(id, maybe_position);
    }

    /// For a given [FlexBox] `id`, get the position of the cursor inside of it.
    pub fn get_cursor_position_for_id(&self, id: FlexBoxId) -> Option<Position> {
        let map = &self.cursor_position_map;
        if let Some(value) = map.get(&id) {
            *value
        } else {
            None
        }
    }
}
