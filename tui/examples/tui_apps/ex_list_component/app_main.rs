// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use r3bl_tui::{
    box_end, box_start, inline_string, key_press, new_style, ok, render_component_in_current_box,
    req_size_pc, surface, throws_with_return, tui_color, BatchAction, ListComponent,
};
use r3bl_tui::{
    App, BoxedSafeApp, CommonResult, ComponentRegistry, ComponentRegistryMap, ContainsResult,
    EventPropagation, FlexBoxId, GlobalData, HasFocus, InputEvent, Key, KeyPress,
    LayoutDirection, LayoutManagement, PerformPositioningAndSizing, RenderOpCommon, RenderOpIR,
    RenderOpIRVec, RenderPipeline, ZOrder,
};

use super::state::{AppSignal, AppState};
use super::todo_item::{Priority, TodoItem};

// Constants for the component IDs
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Id {
    Container = 0,
    List = 1,
    Help = 2,
}

mod id_impl {
    use super::{FlexBoxId, Id};
    use r3bl_tui::TuiStyleId;

    impl From<Id> for u8 {
        fn from(id: Id) -> u8 {
            id as u8
        }
    }

    impl From<Id> for FlexBoxId {
        fn from(id: Id) -> FlexBoxId {
            FlexBoxId::new(id)
        }
    }

    impl From<Id> for TuiStyleId {
        fn from(id: Id) -> TuiStyleId {
            TuiStyleId(id as u8)
        }
    }
}

#[derive(Default)]
pub struct TodoListApp {
    _phantom: std::marker::PhantomData<(AppState, AppSignal)>,
}

mod constructor {
    use super::*;

    impl TodoListApp {
        pub fn new_boxed() -> BoxedSafeApp<AppState, AppSignal> {
            Box::new(Self::default())
        }
    }
}

mod app_impl {
    use super::*;

    impl App for TodoListApp {
        type S = AppState;
        type AS = AppSignal;

        fn app_init(
            &mut self,
            component_registry: &mut ComponentRegistryMap<Self::S, Self::AS>,
            has_focus: &mut HasFocus,
        ) {
            // Create sample todo items
            let items = vec![
                TodoItem::new(1, "Write Rust code", Priority::High),
                TodoItem::new(2, "Read documentation", Priority::Medium),
                TodoItem::new(3, "Run tests", Priority::High),
                TodoItem::new(4, "Fix bugs", Priority::Medium),
                TodoItem::new(5, "Review PRs", Priority::Low),
                TodoItem::new(6, "Update changelog", Priority::Low),
                TodoItem::new(7, "Write blog post", Priority::Medium),
                TodoItem::new(8, "Refactor code", Priority::Low),
            ];

            // Create list component
            let mut list = ListComponent::new(Id::List.into(), items);

            // Add batch action: Delete selected items
            list.add_batch_action(BatchAction::new(
                key_press! { @char 'd' },
                "Delete selected items".to_string(),
                Box::new(|items, selected_indices, state| {
                    let count = selected_indices.len();
                    for &idx in selected_indices.iter().rev() {
                        items.remove(idx);
                    }
                    state.status_message = format!("Deleted {} item(s)", count);
                    ok!(())
                }),
            ));

            // Add batch action: Mark all selected as complete
            list.add_batch_action(BatchAction::new(
                key_press! { @char 'c' },
                "Complete selected items".to_string(),
                Box::new(|items, selected_indices, state| {
                    let count = selected_indices.len();
                    for &idx in selected_indices {
                        items[idx].completed = true;
                    }
                    state.status_message = format!("Completed {} item(s)", count);
                    ok!(())
                }),
            ));

            // Add batch action: Set high priority for selected
            list.add_batch_action(BatchAction::new(
                key_press! { @char 'h' },
                "Set high priority".to_string(),
                Box::new(|items, selected_indices, state| {
                    let count = selected_indices.len();
                    for &idx in selected_indices {
                        items[idx].priority = Priority::High;
                    }
                    state.status_message = format!("Set {} item(s) to high priority", count);
                    ok!(())
                }),
            ));

            // Register list component
            let id = FlexBoxId::from(Id::List);
            if let ContainsResult::DoesNotContain = ComponentRegistry::contains(component_registry, id) {
                ComponentRegistry::put(component_registry, id, Box::new(list));
            }

            // Give focus to list
            if has_focus.get_id().is_none() {
                has_focus.set_id(id);
            }
        }

        fn app_render(
            &mut self,
            global_data: &mut GlobalData<Self::S, Self::AS>,
            component_registry: &mut ComponentRegistryMap<Self::S, Self::AS>,
            has_focus: &mut HasFocus,
        ) -> CommonResult<RenderPipeline> {
            let window_size = global_data.window_size;

            throws_with_return!({
                let mut surface = surface!(stylesheet: create_stylesheet()?);

                surface.surface_start(r3bl_tui::SurfaceProps {
                    pos: r3bl_tui::col(0) + r3bl_tui::row(0),
                    size: window_size,
                })?;

                render_layout(&mut surface, global_data, component_registry, has_focus)?;

                surface.surface_end()?;

                surface.render_pipeline
            });
        }

        fn app_handle_input_event(
            &mut self,
            input_event: InputEvent,
            global_data: &mut GlobalData<Self::S, Self::AS>,
            component_registry: &mut ComponentRegistryMap<Self::S, Self::AS>,
            has_focus: &mut HasFocus,
        ) -> CommonResult<EventPropagation> {
            // Handle quit
            if let InputEvent::Keyboard(KeyPress::Plain {
                key: Key::Character('q'),
            }) = input_event
            {
                return ok!(EventPropagation::ExitMainEventLoop);
            }

            // Clear status message on any key
            if !global_data.state.status_message.is_empty() {
                global_data.state.status_message.clear();
            }

            // Route to component
            ComponentRegistry::route_event_to_focused_component(
                global_data,
                input_event,
                component_registry,
                has_focus,
            )
        }

        fn app_handle_signal(
            &mut self,
            _signal: &Self::AS,
            _global_data: &mut GlobalData<Self::S, Self::AS>,
            _component_registry: &mut ComponentRegistryMap<Self::S, Self::AS>,
            _has_focus: &mut HasFocus,
        ) -> CommonResult<EventPropagation> {
            ok!(EventPropagation::Propagate)
        }
    }
}

fn render_layout(
    surface: &mut r3bl_tui::Surface,
    global_data: &mut GlobalData<AppState, AppSignal>,
    component_registry: &mut ComponentRegistryMap<AppState, AppSignal>,
    has_focus: &mut HasFocus,
) -> CommonResult<()> {
    use r3bl_tui::throws;

    throws!({
        // Container box (vertical) that stacks list and help
        let container_id = FlexBoxId::from(Id::Container);
        box_start! (
            in:                     surface,
            id:                     container_id,
            dir:                    LayoutDirection::Vertical,
            requested_size_percent: req_size_pc!(width: 100, height: 100),
            styles:                 [container_id]
        );

        // List box (90% height)
        {
            let component_id = FlexBoxId::from(Id::List);

            box_start! (
                in:                     surface,
                id:                     component_id,
                dir:                    LayoutDirection::Vertical,
                requested_size_percent: req_size_pc!(width: 100, height: 90),
                styles:                 [component_id]
            );

            // Render list component
            render_component_in_current_box!(
                in:           surface,
                component_id: component_id,
                from:         component_registry,
                global_data:  global_data,
                has_focus:    has_focus
            );

            box_end! (in: surface);
        }

        // Help text box (10% height at bottom)
        {
            let help_id = FlexBoxId::from(Id::Help);

            box_start! (
                in:                     surface,
                id:                     help_id,
                dir:                    LayoutDirection::Horizontal,
                requested_size_percent: req_size_pc!(width: 100, height: 10),
                styles:                 [help_id]
            );

            render_help_text(surface, global_data)?;

            box_end! (in: surface);
        }

        box_end! (in: surface); // End container
    })
}

fn create_stylesheet() -> CommonResult<r3bl_tui::TuiStylesheet> {
    throws_with_return!({
        use r3bl_tui::tui_stylesheet;

        tui_stylesheet! {
            new_style!(id: {Id::List} bold color_fg: {tui_color!(cyan)}),
            new_style!(id: {Id::Help} color_fg: {tui_color!(slate_gray)})
        }
    })
}

fn render_help_text(
    surface: &mut r3bl_tui::Surface,
    global_data: &GlobalData<AppState, AppSignal>,
) -> CommonResult<()> {
    let current_box = surface.stack_of_boxes.last().unwrap();
    let origin = current_box.style_adjusted_origin_pos;
    let box_height = current_box.style_adjusted_bounds_size.row_height;

    // Get styles from stylesheet by ID
    let list_style = surface.stylesheet.find_style_by_id(Id::List);
    let help_style = surface.stylesheet.find_style_by_id(Id::Help);

    // Status message (if any) renders first (top of help area)
    if !global_data.state.status_message.is_empty() {
        let mut status_ops = RenderOpIRVec::new();
        status_ops += RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(origin));
        status_ops += RenderOpIR::PaintTextWithAttributes(
            inline_string!("{}", &global_data.state.status_message),
            list_style,
        );
        surface.render_pipeline.push(ZOrder::Normal, status_ops);
    }

    // Help text renders at the bottom (last line of help area)
    let help_text = "Navigation: ↑/↓  Select: Space  Toggle: t  Priority: p  | Batch: d=delete c=complete h=high  | Quit: q";
    let mut help_ops = RenderOpIRVec::new();
    let help_row_offset = box_height.as_usize().saturating_sub(1);
    let help_pos = r3bl_tui::col(*origin.col_index) +
                   r3bl_tui::row(origin.row_index.as_usize() + help_row_offset);
    help_ops += RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(help_pos));
    help_ops += RenderOpIR::PaintTextWithAttributes(
        inline_string!("{}", help_text),
        help_style,
    );
    surface.render_pipeline.push(ZOrder::Normal, help_ops);

    ok!(())
}
