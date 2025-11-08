// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use r3bl_tui::{
    ok, render_pipeline, CommonResult, InputEvent, Key, KeyPress, ListItem, ComplexListItem,
    ListItemId, ListItemRenderResult, FlexBoxId, FlexBox, RenderPipeline, SurfaceBounds,
    RenderOpIR, RenderOpCommon, ZOrder, Pos, RowIndex,
};
use r3bl_tui::tui::{EventPropagation, GlobalData};

use super::state::{AppSignal, AppState};

/// A complex ProjectItem that demonstrates FlexBox-based nested layouts (Phase 3).
///
/// This item uses the full power of the FlexBox layout engine to render:
/// - A header bar with project name and status badge
/// - An indented body with description
/// - Progress bars and indicators
/// - Hierarchical information display
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectItem {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub status: ProjectStatus,
    pub progress: u8,  // 0-100
    pub subtask_count: usize,
    pub completed_subtasks: usize,

    /// Temporary FlexBoxId assigned by the list component
    flexbox_id: Option<FlexBoxId>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProjectStatus {
    Planning,
    InProgress,
    Review,
    Completed,
}

impl ProjectStatus {
    fn badge(&self) -> &'static str {
        match self {
            Self::Planning => "[PLAN]",
            Self::InProgress => "[WORK]",
            Self::Review => "[REVIEW]",
            Self::Completed => "[DONE]",
        }
    }
}

impl ProjectItem {
    pub fn new(
        id: u64,
        name: &str,
        description: &str,
        status: ProjectStatus,
        progress: u8,
        subtask_count: usize,
        completed_subtasks: usize,
    ) -> Self {
        Self {
            id,
            name: name.to_string(),
            description: description.to_string(),
            status,
            progress,
            subtask_count,
            completed_subtasks,
            flexbox_id: None,
        }
    }
}

impl ListItem<AppState, AppSignal> for ProjectItem {
    fn id(&self) -> ListItemId {
        ListItemId::new(self.id)
    }

    fn render(
        &mut self,
        global_data: &mut GlobalData<AppState, AppSignal>,
        current_box: Option<FlexBox>,
        surface_bounds: Option<SurfaceBounds>,
        is_focused: bool,
        is_selected: bool,
    ) -> CommonResult<Option<ListItemRenderResult>> {
        ComplexListItem::render(self, global_data, current_box, surface_bounds, is_focused, is_selected)
    }

    fn handle_event_dispatch(
        &mut self,
        event: InputEvent,
        state: &mut AppState,
    ) -> CommonResult<EventPropagation> {
        ComplexListItem::handle_event_dispatch(self, event, state)
    }
}

impl ComplexListItem<AppState, AppSignal> for ProjectItem {
    fn get_flexbox_id(&self) -> Option<FlexBoxId> {
        self.flexbox_id
    }

    fn set_flexbox_id(&mut self, id: FlexBoxId) {
        self.flexbox_id = Some(id);
    }

    fn render_as_component(
        &mut self,
        _global_data: &mut GlobalData<AppState, AppSignal>,
        current_box: FlexBox,
        _surface_bounds: SurfaceBounds,
        is_focused: bool,
        is_selected: bool,
    ) -> CommonResult<RenderPipeline> {
        let mut pipeline = render_pipeline!();

        let origin = current_box.style_adjusted_origin_pos;
        let width = current_box.style_adjusted_bounds_size.col_width.as_usize();

        // Line 1: Focus marker + Selection + Project name + Status badge
        {
            let focus_marker = if is_focused { "▶" } else { " " };
            let selection_marker = if is_selected { "✓" } else { " " };
            let badge = self.status.badge();

            // Truncate name if needed to fit badge
            let max_name_len = width.saturating_sub(20);
            let name = if self.name.len() > max_name_len {
                format!("{}...", &self.name[..max_name_len.saturating_sub(3)])
            } else {
                self.name.clone()
            };

            let line1 = format!("{} {} {} {}", focus_marker, selection_marker, name, badge);

            let row1 = origin.row_index;
            let pos1 = Pos::new((origin.col_index, row1));

            render_pipeline! {
                @push_into pipeline at ZOrder::Normal =>
                    RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(pos1)),
                    RenderOpIR::PaintTextWithAttributes(line1.into(), None)
            }
        }

        // Line 2: Indented description
        {
            let indent = "    ";
            let max_desc_len = width.saturating_sub(4);
            let desc = if self.description.len() > max_desc_len {
                format!("{}...", &self.description[..max_desc_len.saturating_sub(3)])
            } else {
                self.description.clone()
            };

            let line2 = format!("{}{}", indent, desc);

            let row2 = origin.row_index + RowIndex::new(1);
            let pos2 = Pos::new((origin.col_index, row2));

            render_pipeline! {
                @push_into pipeline at ZOrder::Normal =>
                    RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(pos2)),
                    RenderOpIR::PaintTextWithAttributes(line2.into(), None)
            }
        }

        // Line 3: Progress bar and task counter
        {
            let indent = "    ";
            let bar_width = 20;
            let filled = (bar_width * self.progress as usize) / 100;
            let empty = bar_width - filled;

            let progress_bar = format!(
                "[{}{}] {}% | Tasks: {}/{}",
                "█".repeat(filled),
                "░".repeat(empty),
                self.progress,
                self.completed_subtasks,
                self.subtask_count
            );

            let line3 = format!("{}{}", indent, progress_bar);

            let row3 = origin.row_index + RowIndex::new(2);
            let pos3 = Pos::new((origin.col_index, row3));

            render_pipeline! {
                @push_into pipeline at ZOrder::Normal =>
                    RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(pos3)),
                    RenderOpIR::PaintTextWithAttributes(line3.into(), None)
            }
        }

        ok!(pipeline)
    }

    fn handle_event(
        &mut self,
        event: InputEvent,
        state: &mut AppState,
    ) -> CommonResult<EventPropagation> {
        ok!(match event {
            // Cycle status with 's'
            InputEvent::Keyboard(KeyPress::Plain {
                key: Key::Character('s'),
            }) => {
                self.status = match self.status {
                    ProjectStatus::Planning => ProjectStatus::InProgress,
                    ProjectStatus::InProgress => ProjectStatus::Review,
                    ProjectStatus::Review => ProjectStatus::Completed,
                    ProjectStatus::Completed => ProjectStatus::Planning,
                };
                state.status_message = format!("Status: {:?}", self.status);
                EventPropagation::ConsumedRender
            }

            // Increment progress with '+'
            InputEvent::Keyboard(KeyPress::Plain {
                key: Key::Character('+'),
            }) => {
                self.progress = (self.progress + 10).min(100);
                state.status_message = format!("Progress: {}%", self.progress);
                EventPropagation::ConsumedRender
            }

            // Decrement progress with '-'
            InputEvent::Keyboard(KeyPress::Plain {
                key: Key::Character('-'),
            }) => {
                self.progress = self.progress.saturating_sub(10);
                state.status_message = format!("Progress: {}%", self.progress);
                EventPropagation::ConsumedRender
            }

            _ => EventPropagation::Propagate,
        })
    }
}
