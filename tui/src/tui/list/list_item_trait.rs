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

use crate::{ok, ColWidth, CommonResult, InputEvent, FlexBox, RenderPipeline, SurfaceBounds, FlexBoxId};
use crate::tui::{EventPropagation, GlobalData};

/// Result of rendering a list item - either simple text or complex pipeline.
#[derive(Debug)]
pub enum ListItemRenderResult {
    /// Simple items return a single line of text
    SimpleLine(String),
    /// Complex items return a full render pipeline
    ComplexPipeline(RenderPipeline),
}

/// Stable identifier for list items that survives add/remove operations.
///
/// Similar to React's `key` prop, this ID allows the list component to track items
/// even when they're reordered, added, or removed. Using stable IDs instead of array
/// indices prevents selection/focus from jumping to wrong items after mutations.
///
/// # Identity vs Rendering Slot
///
/// `ListItemId` represents the **business logic identity** of an item and remains
/// constant throughout the item's lifetime. In contrast, `FlexBoxId` (used by
/// complex items) represents a **temporary rendering slot** that may change as
/// items scroll in and out of the viewport.
///
/// # Example
///
/// ```
/// use r3bl_tui::ListItemId;
///
/// let id1 = ListItemId::new(1);
/// let id2 = ListItemId::new(2);
/// assert_ne!(id1, id2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListItemId(pub u64);

impl ListItemId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Base trait for all list items - provides stable identity.
///
/// All list items must have a stable identifier that survives add/remove operations.
/// This base trait is implemented by both simple (string-based) and complex
/// (FlexBox-based) list items.
///
/// # Type Parameters
///
/// - `S`: Application state type (must be `Debug + Default + Clone + Sync + Send`)
/// - `AS`: Application signal type (must be `Debug + Default + Clone + Sync + Send`)
///
/// # Trait Hierarchy
///
/// ```text
/// ListItem (base)
///   ├─ SimpleListItem (Phase 1: string rendering)
///   └─ ComplexListItem (Phase 3: FlexBox rendering)
/// ```
pub trait ListItem<S, AS>: Debug + Send + Sync
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    /// Returns a stable identifier for this item.
    ///
    /// The ID must remain constant for the lifetime of the item, even if the item's
    /// content changes. This allows the list component to track focus and selection
    /// correctly across mutations.
    fn id(&self) -> ListItemId;

    /// Renders this item - must be implemented via [`SimpleListItem`] or [`ComplexListItem`].
    ///
    /// This method provides the unified rendering interface for all list items.
    /// Types should implement either [`SimpleListItem`] or [`ComplexListItem`],
    /// which provide default implementations of this method.
    ///
    /// # Errors
    ///
    /// Returns an error if rendering fails.
    fn render(
        &mut self,
        _global_data: &mut GlobalData<S, AS>,
        _current_box: Option<FlexBox>,
        _surface_bounds: Option<SurfaceBounds>,
        _is_focused: bool,
        _is_selected: bool,
    ) -> CommonResult<Option<ListItemRenderResult>> {
        crate::CommonError::new_error_result_with_only_type(
            crate::CommonErrorType::InvalidState,
        )
    }

    /// Handles an input event - must be implemented via [`SimpleListItem`] or [`ComplexListItem`].
    ///
    /// Types should implement either [`SimpleListItem`] or [`ComplexListItem`],
    /// which provide default implementations of this method.
    ///
    /// # Errors
    ///
    /// Returns an error if event handling fails.
    fn handle_event_dispatch(
        &mut self,
        _event: InputEvent,
        _state: &mut S,
    ) -> CommonResult<EventPropagation> {
        crate::CommonError::new_error_result_with_only_type(
            crate::CommonErrorType::InvalidState,
        )
    }

    /// Returns the FlexBoxId assigned to this item (for complex items).
    ///
    /// Simple items return `None`. Complex items override this to return their assigned ID.
    fn get_flexbox_id(&self) -> Option<FlexBoxId> {
        None
    }

    /// Sets the FlexBoxId for this item (for complex items).
    ///
    /// Simple items ignore this. Complex items override this to store the assigned ID.
    fn set_flexbox_id(&mut self, _id: FlexBoxId) {
        // Default: no-op for simple items
    }
}

/// Simple list items with fixed-height string rendering (Phase 1).
///
/// Items implementing this trait render as a single line of text and handle their
/// own events. This is the most common and performant way to implement list items.
///
/// # Performance
///
/// Simple items are lightweight and fast to render. They use basic string concatenation
/// and avoid the overhead of the FlexBox layout engine.
///
/// # Event Handling
///
/// Items receive events only when:
/// 1. The focused item (cursor position) receives the event
/// 2. The list component doesn't handle the event (navigation keys are always captured)
///
/// Items should return:
/// - `EventPropagation::ConsumedRender` if the event changed item state
/// - `EventPropagation::Consumed` if the event was handled but no re-render needed
/// - `EventPropagation::Propagate` if the event should continue up the chain
///
/// # Example Implementation
///
/// ```ignore
/// use r3bl_tui::*;
///
/// #[derive(Debug)]
/// struct TodoItem {
///     id: u64,
///     title: String,
///     completed: bool,
/// }
///
/// impl ListItem<AppState, AppSignal> for TodoItem {
///     fn id(&self) -> ListItemId {
///         ListItemId::new(self.id)
///     }
/// }
///
/// impl SimpleListItem<AppState, AppSignal> for TodoItem {
///     fn render_line(
///         &mut self,
///         _state: &AppState,
///         is_focused: bool,
///         is_selected: bool,
///         _available_width: ColWidth,
///     ) -> CommonResult<String> {
///         let checkbox = if self.completed { "☑" } else { "☐" };
///         let marker = if is_focused { "›" } else { " " };
///         ok!(format!("{} {} {}", marker, checkbox, self.title))
///     }
///
///     fn handle_event(
///         &mut self,
///         event: InputEvent,
///         _state: &mut AppState,
///     ) -> CommonResult<EventPropagation> {
///         ok!(match event {
///             InputEvent::Keyboard(KeyPress::Plain {
///                 key: Key::Character(' ')
///             }) => {
///                 self.completed = !self.completed;
///                 EventPropagation::ConsumedRender
///             }
///             _ => EventPropagation::Propagate,
///         })
///     }
/// }
/// ```
pub trait SimpleListItem<S, AS>: ListItem<S, AS>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    /// Override base trait render to call render_line
    fn render(
        &mut self,
        global_data: &mut GlobalData<S, AS>,
        _current_box: Option<FlexBox>,
        _surface_bounds: Option<SurfaceBounds>,
        is_focused: bool,
        is_selected: bool,
    ) -> CommonResult<Option<ListItemRenderResult>> {
        // Extract available_width from current_box if provided, otherwise use a default
        let available_width = _current_box
            .map(|b| b.style_adjusted_bounds_size.col_width)
            .unwrap_or_else(|| crate::ColWidth::new(80));

        let line = self.render_line(&global_data.state, is_focused, is_selected, available_width)?;
        ok!(Some(ListItemRenderResult::SimpleLine(line)))
    }

    /// Override base trait event handler to call simple handle_event
    fn handle_event_dispatch(
        &mut self,
        event: InputEvent,
        state: &mut S,
    ) -> CommonResult<EventPropagation> {
        self.handle_event(event, state)
    }

    /// Renders this item as a single line of text.
    ///
    /// # Fixed Height
    ///
    /// Must return exactly 1 line of text. The returned string should not contain
    /// newline characters. If the text is longer than `available_width`, the list
    /// component will truncate it.
    ///
    /// # Parameters
    ///
    /// - `state`: Read-only access to application state
    /// - `is_focused`: True if this item is at the cursor position
    /// - `is_selected`: True if this item is in the selection set
    /// - `available_width`: Maximum width available for rendering
    ///
    /// # Styling Recommendations
    ///
    /// Use different visual indicators for the four states:
    /// - `(focused=true, selected=true)`: Bold + underline + color
    /// - `(focused=true, selected=false)`: Bold + color
    /// - `(focused=false, selected=true)`: Background color
    /// - `(focused=false, selected=false)`: Normal text
    ///
    /// # Errors
    ///
    /// Returns an error if rendering fails (e.g., due to invalid state or formatting issues).
    fn render_line(
        &mut self,
        state: &S,
        is_focused: bool,
        is_selected: bool,
        available_width: ColWidth,
    ) -> CommonResult<String>;

    /// Handles an input event when this item has focus.
    ///
    /// This method is called when the focused item receives an event that the list
    /// component doesn't handle itself.
    ///
    /// # Navigation Keys
    ///
    /// The list component always handles these keys, so items will never see them:
    /// - Arrow keys (Up, Down)
    /// - Page Up / Page Down
    /// - Home / End
    /// - Space (used for selection toggle)
    ///
    /// # State Modification
    ///
    /// Items can freely modify `state` since it's passed as `&mut`. If the item's
    /// own state changes in a way that affects rendering, return
    /// `EventPropagation::ConsumedRender`.
    ///
    /// # Errors
    ///
    /// Returns an error if event handling fails (e.g., due to invalid state transitions).
    fn handle_event(
        &mut self,
        event: InputEvent,
        state: &mut S,
    ) -> CommonResult<EventPropagation>;
}

/// Complex list items with nested FlexBox layouts (Phase 3).
///
/// Items implementing this trait can use the full power of the FlexBox layout engine
/// to create nested, hierarchical UIs with automatic positioning, borders, padding,
/// and percentage-based sizing.
///
/// # FlexBoxId Management
///
/// Unlike `ListItemId` (which is permanent), the `FlexBoxId` is **temporary** and
/// only valid while the item is visible in the viewport. The `ListComponent` manages
/// a pool of FlexBoxIds and assigns them dynamically as items scroll into view.
///
/// # Use Cases
///
/// - Tree views with nested indentation
/// - Cards with icons, buttons, and multiple text sections
/// - Items with complex borders and styling
/// - Multi-column layouts within items
///
/// # Performance Trade-offs
///
/// Complex items are 10-100x slower to render than simple items due to the FlexBox
/// layout engine overhead. However, only visible items are rendered, so the total
/// cost depends on viewport size (typically 20-50 items) rather than list size.
///
/// # Example Implementation
///
/// ```ignore
/// use r3bl_tui::*;
///
/// #[derive(Debug)]
/// struct FileTreeItem {
///     id: u64,
///     name: String,
///     depth: usize,
///     flexbox_id: Option<FlexBoxId>,
/// }
///
/// impl ListItem<AppState, AppSignal> for FileTreeItem {
///     fn id(&self) -> ListItemId {
///         ListItemId::new(self.id)
///     }
/// }
///
/// impl ComplexListItem<AppState, AppSignal> for FileTreeItem {
///     fn get_flexbox_id(&self) -> Option<FlexBoxId> {
///         self.flexbox_id
///     }
///
///     fn set_flexbox_id(&mut self, id: FlexBoxId) {
///         self.flexbox_id = Some(id);
///     }
///
///     fn render_as_component(
///         &mut self,
///         global_data: &mut GlobalData<AppState, AppSignal>,
///         current_box: FlexBox,
///         _surface_bounds: SurfaceBounds,
///         is_focused: bool,
///         is_selected: bool,
///     ) -> CommonResult<RenderPipeline> {
///         let mut pipeline = render_pipeline!();
///
///         // Horizontal layout: [indent spacer] [icon] [text]
///         // The FlexBox engine handles all positioning automatically
///
///         ok!(pipeline)
///     }
///
///     fn handle_event(
///         &mut self,
///         event: InputEvent,
///         _state: &mut AppState,
///     ) -> CommonResult<EventPropagation> {
///         // Handle expand/collapse, etc.
///         ok!(EventPropagation::Propagate)
///     }
/// }
/// ```
pub trait ComplexListItem<S, AS>: ListItem<S, AS>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    /// Override base trait render to call render_as_component
    fn render(
        &mut self,
        global_data: &mut GlobalData<S, AS>,
        current_box: Option<FlexBox>,
        surface_bounds: Option<SurfaceBounds>,
        is_focused: bool,
        is_selected: bool,
    ) -> CommonResult<Option<ListItemRenderResult>> {
        let Some(current_box) = current_box else {
            return crate::CommonError::new_error_result_with_only_type(
                crate::CommonErrorType::InvalidArguments,
            );
        };
        let Some(surface_bounds) = surface_bounds else {
            return crate::CommonError::new_error_result_with_only_type(
                crate::CommonErrorType::InvalidArguments,
            );
        };

        let pipeline = self.render_as_component(
            global_data,
            current_box,
            surface_bounds,
            is_focused,
            is_selected,
        )?;
        ok!(Some(ListItemRenderResult::ComplexPipeline(pipeline)))
    }

    /// Override base trait event handler to call complex handle_event
    fn handle_event_dispatch(
        &mut self,
        event: InputEvent,
        state: &mut S,
    ) -> CommonResult<EventPropagation> {
        self.handle_event(event, state)
    }

    /// Returns the temporary FlexBoxId assigned to this item for rendering.
    ///
    /// Returns `None` if the item is not currently visible in the viewport.
    /// The ID is assigned by the `ListComponent` when the item scrolls into view
    /// and is returned to the pool when it scrolls out.
    fn get_flexbox_id(&self) -> Option<FlexBoxId>;

    /// Sets the temporary FlexBoxId for this item's rendering slot.
    ///
    /// Called by the `ListComponent` when the item scrolls into view and is
    /// assigned a rendering slot from the FlexBoxId pool.
    fn set_flexbox_id(&mut self, id: FlexBoxId);

    /// Renders this item using the FlexBox layout engine.
    ///
    /// The item can create nested FlexBox layouts, apply styles, and use all
    /// the capabilities of the full rendering system.
    ///
    /// # Parameters
    ///
    /// - `global_data`: Access to application state and signals
    /// - `current_box`: The FlexBox allocated for this item (contains origin, size, style)
    /// - `surface_bounds`: The surface boundaries for rendering
    /// - `is_focused`: True if this item is at the cursor position
    /// - `is_selected`: True if this item is in the selection set
    ///
    /// # Returns
    ///
    /// A `RenderPipeline` containing the rendering operations for this item.
    ///
    /// # Errors
    ///
    /// Returns an error if rendering fails (e.g., due to layout errors or invalid state).
    fn render_as_component(
        &mut self,
        global_data: &mut GlobalData<S, AS>,
        current_box: FlexBox,
        surface_bounds: SurfaceBounds,
        is_focused: bool,
        is_selected: bool,
    ) -> CommonResult<RenderPipeline>;

    /// Handles an input event when this item has focus.
    ///
    /// This method is called when the focused item receives an event that the list
    /// component doesn't handle itself.
    ///
    /// # Navigation Keys
    ///
    /// The list component always handles these keys, so items will never see them:
    /// - Arrow keys (Up, Down)
    /// - Page Up / Page Down
    /// - Home / End
    /// - Space (used for selection toggle)
    ///
    /// # State Modification
    ///
    /// Items can freely modify `state` since it's passed as `&mut`. If the item's
    /// own state changes in a way that affects rendering, return
    /// `EventPropagation::ConsumedRender`.
    ///
    /// # Errors
    ///
    /// Returns an error if event handling fails (e.g., due to invalid state transitions).
    fn handle_event(
        &mut self,
        event: InputEvent,
        state: &mut S,
    ) -> CommonResult<EventPropagation>;
}
