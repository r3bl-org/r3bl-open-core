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

use crate::{ColWidth, CommonResult, InputEvent};
use crate::tui::EventPropagation;

/// Stable identifier for list items that survives add/remove operations.
///
/// Similar to React's `key` prop, this ID allows the list component to track items
/// even when they're reordered, added, or removed. Using stable IDs instead of array
/// indices prevents selection/focus from jumping to wrong items after mutations.
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

/// Trait for items that can be displayed and interacted with in a [`ListComponent`].
///
/// Items must provide a stable ID, know how to render themselves, and handle events
/// when they have focus. The list component manages navigation and selection, while
/// items handle their own domain-specific behavior.
///
/// # Type Parameters
///
/// - `S`: Application state type (must be `Debug + Default + Clone + Sync + Send`)
/// - `AS`: Application signal type (must be `Debug + Default + Clone + Sync + Send`)
///
/// # Phase 1: Fixed-Height Rendering
///
/// In this initial implementation, all items must render exactly 1 row. The `render()`
/// method should produce a single line of output. Variable height support will be added
/// in Phase 2.
///
/// # Event Handling
///
/// Items receive events only when:
/// 1. Exactly 1 item is selected (single-selection mode)
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
///
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

    /// Renders this item as a single line of text.
    ///
    /// # Phase 1: Fixed Height
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

    /// Handles an input event when this item has sole focus.
    ///
    /// This method is called only when exactly 1 item is selected. When multiple
    /// items are selected, batch actions handle events instead.
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
