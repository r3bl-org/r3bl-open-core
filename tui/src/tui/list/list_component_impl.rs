/*
 *   Copyright (c) 2025 R3BL LLC
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

use crate::{ok, render_pipeline, InputEvent, KeyPress, Key, SpecialKey, CommonResult, RowIndex, Pos};
use crate::tui::{EventPropagation, Component, FlexBoxId, GlobalData, FlexBox, SurfaceBounds, HasFocus, RenderPipeline, RenderOpIR, RenderOpCommon, ZOrder};

use super::{ListComponent, ListItem, ListItemId};

impl<S, AS, I> ListComponent<S, AS, I>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
    I: ListItem<S, AS>,
{
    // ┌─────────────────────────────────────────────────────────────────────────┐
    // │ Navigation Methods (Phase 1: Fixed Height)                              │
    // └─────────────────────────────────────────────────────────────────────────┘

    /// Moves cursor up by one item, scrolling viewport if necessary.
    pub fn move_cursor_up(&mut self) {
        let Some(cursor_index) = self.get_cursor_index() else {
            return;
        };

        if cursor_index == 0 {
            return;
        }

        let new_cursor_index = cursor_index - 1;
        self.cursor_id = self.items[new_cursor_index].id();

        self.ensure_cursor_visible(new_cursor_index);
    }

    /// Moves cursor down by one item, scrolling viewport if necessary.
    pub fn move_cursor_down(&mut self) {
        let Some(cursor_index) = self.get_cursor_index() else {
            return;
        };

        if cursor_index >= self.items.len() - 1 {
            return;
        }

        let new_cursor_index = cursor_index + 1;
        self.cursor_id = self.items[new_cursor_index].id();

        self.ensure_cursor_visible(new_cursor_index);
    }

    /// Moves cursor to the first item.
    pub fn move_cursor_to_start(&mut self) {
        if self.items.is_empty() {
            return;
        }

        self.cursor_id = self.items[0].id();
        self.scroll_offset_index = 0;
    }

    /// Moves cursor to the last item.
    pub fn move_cursor_to_end(&mut self) {
        if self.items.is_empty() {
            return;
        }

        let last_index = self.items.len() - 1;
        self.cursor_id = self.items[last_index].id();
        self.ensure_cursor_visible(last_index);
    }

    /// Moves cursor up by one page (viewport height).
    pub fn move_cursor_page_up(&mut self) {
        let Some(cursor_index) = self.get_cursor_index() else {
            return;
        };

        let page_size = self.viewport_height.as_usize();
        let new_index = cursor_index.saturating_sub(page_size);

        self.cursor_id = self.items[new_index].id();
        self.scroll_offset_index = new_index.saturating_sub(page_size / 2);
    }

    /// Moves cursor down by one page (viewport height).
    pub fn move_cursor_page_down(&mut self) {
        let Some(cursor_index) = self.get_cursor_index() else {
            return;
        };

        let page_size = self.viewport_height.as_usize();
        let new_index = (cursor_index + page_size).min(self.items.len() - 1);

        self.cursor_id = self.items[new_index].id();
        self.ensure_cursor_visible(new_index);
    }

    /// Ensures the cursor item is visible in the viewport.
    ///
    /// Adjusts `scroll_offset_index` if the cursor is above or below the viewport.
    /// In Phase 1 with fixed-height items, this is straightforward arithmetic.
    fn ensure_cursor_visible(&mut self, cursor_index: usize) {
        let viewport_size = self.viewport_height.as_usize();

        if cursor_index < self.scroll_offset_index {
            self.scroll_offset_index = cursor_index;
        } else if cursor_index >= self.scroll_offset_index + viewport_size {
            self.scroll_offset_index = cursor_index - viewport_size + 1;
        }
    }

    // ┌─────────────────────────────────────────────────────────────────────────┐
    // │ Event Handling                                                           │
    // └─────────────────────────────────────────────────────────────────────────┘

    /// Checks if an event is a navigation key that the list always handles.
    fn is_navigation_key(event: &InputEvent) -> bool {
        matches!(
            event,
            InputEvent::Keyboard(KeyPress::Plain {
                key: Key::SpecialKey(
                    SpecialKey::Up
                        | SpecialKey::Down
                        | SpecialKey::PageUp
                        | SpecialKey::PageDown
                        | SpecialKey::Home
                        | SpecialKey::End
                )
            })
        )
    }

    /// Handles navigation key events.
    fn handle_navigation(&mut self, event: InputEvent) -> CommonResult<EventPropagation> {
        let InputEvent::Keyboard(KeyPress::Plain { key }) = event else {
            return ok!(EventPropagation::Propagate);
        };

        match key {
            Key::SpecialKey(SpecialKey::Up) => self.move_cursor_up(),
            Key::SpecialKey(SpecialKey::Down) => self.move_cursor_down(),
            Key::SpecialKey(SpecialKey::Home) => self.move_cursor_to_start(),
            Key::SpecialKey(SpecialKey::End) => self.move_cursor_to_end(),
            Key::SpecialKey(SpecialKey::PageUp) => self.move_cursor_page_up(),
            Key::SpecialKey(SpecialKey::PageDown) => self.move_cursor_page_down(),
            _ => return ok!(EventPropagation::Propagate),
        }

        ok!(EventPropagation::ConsumedRender)
    }

    /// Finds the batch action that matches the given key press.
    fn find_batch_action_for_key(&self, event: &InputEvent) -> Option<usize> {
        let InputEvent::Keyboard(key_press) = event else {
            return None;
        };

        self.batch_actions
            .iter()
            .position(|action| &action.key_binding == key_press)
    }

    /// Executes a batch action on selected items.
    fn execute_batch_action(
        &mut self,
        action_index: usize,
        state: &mut S,
    ) -> CommonResult<EventPropagation> {
        let action = &self.batch_actions[action_index];

        let mut selected_indices: Vec<usize> = self
            .selected_ids
            .iter()
            .filter_map(|id| self.items.iter().position(|item| item.id() == *id))
            .collect();

        // If nothing is selected, operate on the cursor position
        if selected_indices.is_empty() {
            if let Some(cursor_index) = self.get_cursor_index() {
                selected_indices.push(cursor_index);
            }
        }

        selected_indices.sort_unstable();

        (action.handler)(&mut self.items, &selected_indices, state)?;

        self.fix_cursor_after_mutation();

        ok!(EventPropagation::ConsumedRender)
    }

    /// Fixes cursor position after items have been added/removed.
    ///
    /// Ensures `cursor_id` points to a valid item. If the cursor item was deleted,
    /// moves to the nearest valid item.
    fn fix_cursor_after_mutation(&mut self) {
        if self.items.is_empty() {
            self.cursor_id = ListItemId::new(0);
            self.scroll_offset_index = 0;
            return;
        }

        let cursor_still_exists = self
            .items
            .iter()
            .any(|item| item.id() == self.cursor_id);

        if !cursor_still_exists {
            self.cursor_id = self.items[0].id();
            self.scroll_offset_index = 0;
        }

        self.selected_ids
            .retain(|id| self.items.iter().any(|item| item.id() == *id));
    }

}

impl<S, AS, I> Component<S, AS> for ListComponent<S, AS, I>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
    I: ListItem<S, AS>,
{
    fn reset(&mut self) {
        self.clear_selections();
        self.move_cursor_to_start();
    }

    fn get_id(&self) -> FlexBoxId {
        self.id
    }

    fn render(
        &mut self,
        global_data: &mut GlobalData<S, AS>,
        current_box: FlexBox,
        surface_bounds: SurfaceBounds,
        has_focus: &mut HasFocus,
    ) -> CommonResult<RenderPipeline> {
        let mut pipeline = render_pipeline!();

        let available_height = current_box.style_adjusted_bounds_size.row_height;
        self.viewport_height = available_height;

        let is_focused = has_focus.does_id_have_focus(self.id);
        let viewport_end = (self.scroll_offset_index + available_height.as_usize())
            .min(self.items.len());

        let origin_pos = current_box.style_adjusted_origin_pos;

        let Some(cursor_index) = self.get_cursor_index() else {
            return ok!(pipeline);
        };

        // Phase 3: Recycle FlexBoxIds for items that scrolled out of viewport
        if self.flexbox_id_pool.is_some() {
            let visible_item_ids: std::collections::HashSet<ListItemId> = (self.scroll_offset_index..viewport_end)
                .filter_map(|idx| self.items.get(idx).map(|item| item.id()))
                .collect();

            // Return FlexBoxIds for items that scrolled out
            if let Some(ref mut pool) = self.flexbox_id_pool {
                self.visible_item_flexbox_mapping.retain(|item_id, flexbox_id| {
                    if visible_item_ids.contains(item_id) {
                        true // Keep mapping
                    } else {
                        pool.return_id(*flexbox_id); // Recycle ID
                        false // Remove mapping
                    }
                });
            }
        }

        // Render each visible item
        // Track cumulative row offset for complex items that span multiple rows
        let mut cumulative_row_offset = 0_usize;

        for item_index in self.scroll_offset_index..viewport_end
        {
            let item_id = self.items[item_index].id();
            let is_item_focused = is_focused && (item_index == cursor_index);
            let is_item_selected = self.is_selected(item_id);

            // Phase 3: For complex items, assign FlexBoxId if needed
            if let Some(ref mut pool) = self.flexbox_id_pool {
                if !self.visible_item_flexbox_mapping.contains_key(&item_id) {
                    if let Some(flexbox_id) = pool.borrow_id() {
                        self.visible_item_flexbox_mapping.insert(item_id, flexbox_id);
                        self.items[item_index].set_flexbox_id(flexbox_id);
                    }
                }
            }

            // Create a FlexBox for this specific item with adjusted origin
            // For complex items that span multiple rows, this uses cumulative_row_offset
            // For simple items (1 row each), this is the same as viewport_row_index
            let item_origin = Pos::new((
                origin_pos.col_index,
                origin_pos.row_index + RowIndex::new(cumulative_row_offset),
            ));

            let mut item_box = current_box;
            item_box.style_adjusted_origin_pos = item_origin;

            // Call unified render method (works for both simple and complex items)
            let render_result = self.items[item_index].render(
                global_data,
                Some(item_box),
                Some(surface_bounds),
                is_item_focused,
                is_item_selected,
            )?;

            if let Some(result) = render_result {
                match result {
                    super::ListItemRenderResult::SimpleLine(line_text) => {
                        // Simple rendering: paint text at position
                        let row_pos_index = origin_pos.row_index + RowIndex::new(cumulative_row_offset);
                        let render_pos = Pos::new((origin_pos.col_index, row_pos_index));

                        render_pipeline! {
                            @push_into pipeline at ZOrder::Normal =>
                                RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(render_pos)),
                                RenderOpIR::PaintTextWithAttributes(line_text.into(), None)
                        }

                        // Simple items always consume 1 row
                        cumulative_row_offset += 1;
                    }
                    super::ListItemRenderResult::ComplexPipeline(item_pipeline) => {
                        // Complex rendering: merge item's pipeline into ours
                        pipeline.join_into(item_pipeline);

                        // Complex items consume 3 rows (hardcoded for now)
                        // TODO: Make this configurable per item type
                        cumulative_row_offset += 3;
                    }
                }
            }
        }

        ok!(pipeline)
    }

    fn handle_event(
        &mut self,
        global_data: &mut GlobalData<S, AS>,
        input_event: InputEvent,
        _has_focus: &mut HasFocus,
    ) -> CommonResult<EventPropagation> {
        let state = &mut global_data.state;

        if Self::is_navigation_key(&input_event) {
            return self.handle_navigation(input_event);
        }

        if let InputEvent::Keyboard(KeyPress::Plain {
            key: Key::Character(' '),
        }) = input_event
        {
            self.toggle_selection_at_cursor();
            return ok!(EventPropagation::ConsumedRender);
        }

        // Batch actions: Always check first, work on single or multiple items
        if let Some(action_index) = self.find_batch_action_for_key(&input_event) {
            return self.execute_batch_action(action_index, state);
        }

        // Single-item actions: Work on the focused item regardless of selection
        let Some(cursor_index) = self.get_cursor_index() else {
            return ok!(EventPropagation::Propagate);
        };

        let propagation = self.items[cursor_index].handle_event_dispatch(input_event, state)?;

        ok!(propagation)
    }
}
