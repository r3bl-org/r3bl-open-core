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

#[cfg(test)]
pub mod test_fixtures {
    use std::fmt::Debug;

    use crate::{
        ok, ColWidth, CommonResult, InputEvent, KeyPress, Key,
    };
    use crate::tui::{EventPropagation, FlexBoxId};

    use super::super::{ListItem, ListItemId, ListComponent, BatchAction};

    /// Mock application state for testing
    #[derive(Debug, Clone, Default, PartialEq)]
    pub struct TestState {
        pub counter: u32,
    }

    /// Mock application signal for testing
    #[derive(Debug, Clone, Default)]
    pub struct TestSignal;

    /// Simple test item that displays a string
    #[derive(Debug, Clone, PartialEq)]
    pub struct SimpleItem {
        pub id: u64,
        pub text: String,
        pub completed: bool,
    }

    impl SimpleItem {
        pub fn new(id: u64, text: &str) -> Self {
            Self {
                id,
                text: text.to_string(),
                completed: false,
            }
        }
    }

    impl ListItem<TestState, TestSignal> for SimpleItem {
        fn id(&self) -> ListItemId {
            ListItemId::new(self.id)
        }

        fn render_line(
            &mut self,
            _state: &TestState,
            is_focused: bool,
            is_selected: bool,
            _available_width: ColWidth,
        ) -> CommonResult<String> {
            let checkbox = if self.completed { "☑" } else { "☐" };
            let marker = if is_focused { "›" } else { " " };
            let bg = if is_selected { "[*]" } else { "   " };
            ok!(format!("{}{} {} {}", marker, bg, checkbox, self.text))
        }

        fn handle_event(
            &mut self,
            event: InputEvent,
            state: &mut TestState,
        ) -> CommonResult<EventPropagation> {
            ok!(match event {
                InputEvent::Keyboard(KeyPress::Plain {
                    key: Key::Character('t'),
                }) => {
                    self.completed = !self.completed;
                    EventPropagation::ConsumedRender
                }
                InputEvent::Keyboard(KeyPress::Plain {
                    key: Key::Character('i'),
                }) => {
                    state.counter += 1;
                    EventPropagation::ConsumedRender
                }
                _ => EventPropagation::Propagate,
            })
        }
    }

    /// Helper to create a test list component with sample items
    pub fn create_test_list() -> ListComponent<TestState, TestSignal, SimpleItem> {
        let items = vec![
            SimpleItem::new(1, "Item 1"),
            SimpleItem::new(2, "Item 2"),
            SimpleItem::new(3, "Item 3"),
            SimpleItem::new(4, "Item 4"),
            SimpleItem::new(5, "Item 5"),
        ];

        ListComponent::new(FlexBoxId::new(1), items)
    }

    /// Helper to create a batch action for testing
    pub fn create_delete_batch_action() -> BatchAction<TestState, TestSignal, SimpleItem> {
        BatchAction::new(
            KeyPress::Plain {
                key: Key::Character('d'),
            },
            "Delete selected items".to_string(),
            Box::new(|items, selected_indices, _state| {
                for &idx in selected_indices.iter().rev() {
                    items.remove(idx);
                }
                ok!(())
            }),
        )
    }

    /// Helper to create a batch action that marks items as complete
    pub fn create_complete_batch_action() -> BatchAction<TestState, TestSignal, SimpleItem> {
        BatchAction::new(
            KeyPress::Plain {
                key: Key::Character('c'),
            },
            "Complete selected items".to_string(),
            Box::new(|items, selected_indices, _state| {
                for &idx in selected_indices {
                    items[idx].completed = true;
                }
                ok!(())
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::test_fixtures::*;
    use crate::{InputEvent, KeyPress, Key};
    use crate::tui::list::{ListItem, ListComponent};

    #[test]
    fn test_list_component_creation() {
        let list = create_test_list();

        assert_eq!(list.items.len(), 5);
        assert_eq!(list.scroll_offset_index, 0);
        assert_eq!(list.selected_ids.len(), 0);

        let cursor_index = list.get_cursor_index().unwrap();
        assert_eq!(cursor_index, 0);
    }

    #[test]
    fn test_cursor_navigation_down() {
        let mut list = create_test_list();

        assert_eq!(list.get_cursor_index().unwrap(), 0);

        list.move_cursor_down();
        assert_eq!(list.get_cursor_index().unwrap(), 1);

        list.move_cursor_down();
        assert_eq!(list.get_cursor_index().unwrap(), 2);
    }

    #[test]
    fn test_cursor_navigation_up() {
        let mut list = create_test_list();

        list.move_cursor_down();
        list.move_cursor_down();
        assert_eq!(list.get_cursor_index().unwrap(), 2);

        list.move_cursor_up();
        assert_eq!(list.get_cursor_index().unwrap(), 1);

        list.move_cursor_up();
        assert_eq!(list.get_cursor_index().unwrap(), 0);

        list.move_cursor_up();
        assert_eq!(list.get_cursor_index().unwrap(), 0);
    }

    #[test]
    fn test_cursor_navigation_home_end() {
        let mut list = create_test_list();

        list.move_cursor_to_end();
        assert_eq!(list.get_cursor_index().unwrap(), 4);

        list.move_cursor_to_start();
        assert_eq!(list.get_cursor_index().unwrap(), 0);
    }

    #[test]
    fn test_cursor_at_boundaries() {
        let mut list = create_test_list();

        list.move_cursor_up();
        assert_eq!(list.get_cursor_index().unwrap(), 0);

        list.move_cursor_to_end();
        list.move_cursor_down();
        assert_eq!(list.get_cursor_index().unwrap(), 4);
    }

    #[test]
    fn test_selection_toggle() {
        let mut list = create_test_list();

        let item_id = list.items[0].id();
        assert!(!list.is_selected(item_id));

        list.toggle_selection_at_cursor();
        assert!(list.is_selected(item_id));

        list.toggle_selection_at_cursor();
        assert!(!list.is_selected(item_id));
    }

    #[test]
    fn test_multi_selection() {
        let mut list = create_test_list();

        list.toggle_selection_at_cursor();
        assert_eq!(list.selected_ids.len(), 1);

        list.move_cursor_down();
        list.toggle_selection_at_cursor();
        assert_eq!(list.selected_ids.len(), 2);

        list.move_cursor_down();
        list.toggle_selection_at_cursor();
        assert_eq!(list.selected_ids.len(), 3);

        let item_id_0 = list.items[0].id();
        let item_id_1 = list.items[1].id();
        let item_id_2 = list.items[2].id();

        assert!(list.is_selected(item_id_0));
        assert!(list.is_selected(item_id_1));
        assert!(list.is_selected(item_id_2));
    }

    #[test]
    fn test_clear_selections() {
        let mut list = create_test_list();

        list.toggle_selection_at_cursor();
        list.move_cursor_down();
        list.toggle_selection_at_cursor();
        assert_eq!(list.selected_ids.len(), 2);

        list.clear_selections();
        assert_eq!(list.selected_ids.len(), 0);
    }

    #[test]
    fn test_batch_action_delete() {
        let mut list = create_test_list();
        list.add_batch_action(create_delete_batch_action());

        list.toggle_selection_at_cursor();
        list.move_cursor_down();
        list.move_cursor_down();
        list.toggle_selection_at_cursor();

        assert_eq!(list.items.len(), 5);
        assert_eq!(list.selected_ids.len(), 2);

        let mut state = TestState::default();
        let action = &list.batch_actions[0];

        let selected_indices: Vec<usize> = list
            .selected_ids
            .iter()
            .filter_map(|id| list.items.iter().position(|item| item.id() == *id))
            .collect();

        (action.handler)(&mut list.items, &selected_indices, &mut state).unwrap();

        assert_eq!(list.items.len(), 3);
    }

    #[test]
    fn test_batch_action_complete() {
        let mut list = create_test_list();
        list.add_batch_action(create_complete_batch_action());

        list.toggle_selection_at_cursor();
        list.move_cursor_down();
        list.toggle_selection_at_cursor();

        assert!(!list.items[0].completed);
        assert!(!list.items[1].completed);

        let mut state = TestState::default();
        let action = &list.batch_actions[0];

        let mut selected_indices: Vec<usize> = list
            .selected_ids
            .iter()
            .filter_map(|id| list.items.iter().position(|item| item.id() == *id))
            .collect();
        selected_indices.sort_unstable();

        (action.handler)(&mut list.items, &selected_indices, &mut state).unwrap();

        assert!(list.items[0].completed);
        assert!(list.items[1].completed);
        assert!(!list.items[2].completed);
    }

    #[test]
    fn test_viewport_scrolling() {
        use crate::height;

        let mut list = create_test_list();
        list.viewport_height = height(3);

        assert_eq!(list.scroll_offset_index, 0);

        for _ in 0..3 {
            list.move_cursor_down();
        }

        assert_eq!(list.get_cursor_index().unwrap(), 3);
        assert!(list.scroll_offset_index > 0);
    }

    // Note: is_navigation_key is private, so we test it indirectly through event handling

    #[test]
    fn test_item_event_handling() {
        let mut list = create_test_list();
        let mut state = TestState::default();

        let toggle_event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::Character('t'),
        });

        assert!(!list.items[0].completed);

        let result = list.items[0].handle_event(toggle_event, &mut state).unwrap();
        assert!(matches!(result, crate::tui::EventPropagation::ConsumedRender));
        assert!(list.items[0].completed);
    }

    #[test]
    fn test_item_state_modification() {
        let mut list = create_test_list();
        let mut state = TestState::default();

        assert_eq!(state.counter, 0);

        let increment_event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::Character('i'),
        });

        list.items[0].handle_event(increment_event.clone(), &mut state).unwrap();
        assert_eq!(state.counter, 1);

        list.items[0].handle_event(increment_event, &mut state).unwrap();
        assert_eq!(state.counter, 2);
    }

    #[test]
    fn test_empty_list() {
        let empty_list: ListComponent<TestState, TestSignal, SimpleItem> =
            ListComponent::new(crate::tui::FlexBoxId::new(1), vec![]);

        assert_eq!(empty_list.items.len(), 0);
        assert!(empty_list.get_cursor_index().is_none());
    }

    #[test]
    fn test_single_item_list() {
        let items = vec![SimpleItem::new(1, "Only Item")];
        let mut list = ListComponent::new(crate::tui::FlexBoxId::new(1), items);

        assert_eq!(list.get_cursor_index().unwrap(), 0);

        list.move_cursor_up();
        assert_eq!(list.get_cursor_index().unwrap(), 0);

        list.move_cursor_down();
        assert_eq!(list.get_cursor_index().unwrap(), 0);
    }

    #[test]
    fn test_list_item_id_equality() {
        use super::super::ListItemId;

        let id1 = ListItemId::new(42);
        let id2 = ListItemId::new(42);
        let id3 = ListItemId::new(99);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }
}
