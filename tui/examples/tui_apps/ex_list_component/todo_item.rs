// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use r3bl_tui::{
    ok, ColWidth, CommonResult, InputEvent, Key, KeyPress, ListItem,
    SimpleListItem, ListItemId,
};
use r3bl_tui::tui::EventPropagation;

use super::state::{AppSignal, AppState};

/// A TodoItem that implements the ListItem trait.
#[derive(Debug, Clone, PartialEq)]
pub struct TodoItem {
    pub id: u64,
    pub title: String,
    pub completed: bool,
    pub priority: Priority,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Priority {
    Low,
    Medium,
    High,
}

impl TodoItem {
    pub fn new(id: u64, title: &str, priority: Priority) -> Self {
        Self {
            id,
            title: title.to_string(),
            completed: false,
            priority,
        }
    }
}

impl ListItem<AppState, AppSignal> for TodoItem {
    fn id(&self) -> ListItemId {
        ListItemId::new(self.id)
    }

    fn render(
        &mut self,
        global_data: &mut r3bl_tui::tui::GlobalData<AppState, AppSignal>,
        current_box: Option<r3bl_tui::FlexBox>,
        surface_bounds: Option<r3bl_tui::SurfaceBounds>,
        is_focused: bool,
        is_selected: bool,
    ) -> CommonResult<Option<r3bl_tui::tui::list::ListItemRenderResult>> {
        SimpleListItem::render(self, global_data, current_box, surface_bounds, is_focused, is_selected)
    }

    fn handle_event_dispatch(
        &mut self,
        event: InputEvent,
        state: &mut AppState,
    ) -> CommonResult<r3bl_tui::tui::EventPropagation> {
        SimpleListItem::handle_event_dispatch(self, event, state)
    }
}

impl SimpleListItem<AppState, AppSignal> for TodoItem {
    fn render_line(
        &mut self,
        state: &AppState,
        is_focused: bool,
        is_selected: bool,
        available_width: ColWidth,
    ) -> CommonResult<String> {
        // Checkbox indicator
        let checkbox = if self.completed { "☑" } else { "☐" };

        // Focus marker
        let focus_marker = if is_focused { "›" } else { " " };

        // Selection background
        let selection = if is_selected { "[*]" } else { "   " };

        // Priority indicator
        let priority_marker = match self.priority {
            Priority::High => "!",
            Priority::Medium => "-",
            Priority::Low => "·",
        };

        // Truncate title if needed
        let max_title_len = available_width.as_usize().saturating_sub(15);
        let title = if self.title.len() > max_title_len {
            format!("{}...", &self.title[..max_title_len.saturating_sub(3)])
        } else {
            self.title.clone()
        };

        // Status from state (if any)
        let status_hint = if !state.status_message.is_empty() && is_focused {
            " ← "
        } else {
            "   "
        };

        ok!(format!(
            "{}{} {} {} {}{}",
            focus_marker, selection, checkbox, priority_marker, title, status_hint
        ))
    }

    fn handle_event(
        &mut self,
        event: InputEvent,
        state: &mut AppState,
    ) -> CommonResult<EventPropagation> {
        ok!(match event {
            // Toggle completion with 't'
            InputEvent::Keyboard(KeyPress::Plain {
                key: Key::Character('t'),
            }) => {
                self.completed = !self.completed;
                state.status_message = if self.completed {
                    "✓ Marked complete".to_string()
                } else {
                    "○ Marked incomplete".to_string()
                };
                EventPropagation::ConsumedRender
            }

            // Cycle priority with 'p'
            InputEvent::Keyboard(KeyPress::Plain {
                key: Key::Character('p'),
            }) => {
                self.priority = match self.priority {
                    Priority::Low => Priority::Medium,
                    Priority::Medium => Priority::High,
                    Priority::High => Priority::Low,
                };
                state.status_message = format!("Priority: {:?}", self.priority);
                EventPropagation::ConsumedRender
            }

            _ => EventPropagation::Propagate,
        })
    }
}
