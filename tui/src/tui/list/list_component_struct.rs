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

use std::{fmt::Debug, marker::PhantomData, collections::HashSet};

use crate::{RowHeight, KeyPress, CommonResult, height};
use crate::tui::FlexBoxId;

use super::{ListItem, ListItemId};

/// Type alias for batch action handler functions.
///
/// The handler receives:
/// - `&mut Vec<I>`: Mutable access to all items
/// - `&[usize]`: Indices of selected items (sorted)
/// - `&mut S`: Mutable access to application state
pub type BatchActionHandler<S, I> = Box<dyn Fn(&mut Vec<I>, &[usize], &mut S) -> CommonResult<()> + Send + Sync>;

/// A reusable list component with custom-defined items, multi-select, and batch operations.
///
/// # Overview
///
/// `ListComponent` provides a flexible list UI that:
/// - Displays custom items implementing the [`ListItem`] trait
/// - Handles arrow key navigation automatically
/// - Supports single and multi-selection with Space key
/// - Delegates events to focused items
/// - Executes batch actions on multiple selected items
///
/// # Type Parameters
///
/// - `S`: Application state type
/// - `AS`: Application signal type
/// - `I`: Item type implementing [`ListItem<S, AS>`]
///
/// # Example
///
/// ```ignore
/// use r3bl_tui::*;
///
/// // Define your item type
/// #[derive(Debug)]
/// struct TodoItem {
///     id: u64,
///     title: String,
///     completed: bool,
/// }
///
/// impl ListItem<AppState, AppSignal> for TodoItem {
///     // ... trait implementation
/// }
///
/// // Create the list
/// let items = vec![
///     TodoItem { id: 1, title: "Buy milk".into(), completed: false },
///     TodoItem { id: 2, title: "Write code".into(), completed: false },
/// ];
///
/// let mut list = ListComponent::new(my_flexbox_id, items);
///
/// // Add a batch action for deleting selected items
/// list.add_batch_action(BatchAction {
///     key_binding: key_press! { @char 'd' },
///     description: "Delete selected items".into(),
///     handler: Box::new(|items, selected_indices, _state| {
///         for &idx in selected_indices.iter().rev() {
///             items.remove(idx);
///         }
///         Ok(())
///     }),
/// });
///
/// // Register with component registry
/// ComponentRegistry::put(
///     component_registry,
///     my_flexbox_id,
///     Box::new(list),
/// );
/// ```
#[derive(Debug)]
pub struct ListComponent<S, AS, I>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
    I: ListItem<S, AS>,
{
    /// Component identifier (must be unique within the app)
    pub id: FlexBoxId,

    /// All items in the list
    pub items: Vec<I>,

    /// Cursor position (uses stable ID, not index)
    pub cursor_id: ListItemId,

    /// Set of selected item IDs (for multi-select)
    pub selected_ids: HashSet<ListItemId>,

    /// Index of first visible item in viewport
    pub scroll_offset_index: usize,

    /// Height of viewport in rows
    pub viewport_height: RowHeight,

    /// Batch actions available when 2+ items selected
    pub batch_actions: Vec<BatchAction<S, AS, I>>,

    _phantom: PhantomData<(S, AS)>,
}

/// A batch operation that can be executed on one or more items.
///
/// Batch actions are triggered by key bindings and work on:
/// - Selected items (if any items are explicitly selected via Space key)
/// - The cursor position (if no items are selected)
///
/// They receive mutable access to the entire items vector and indices of target items.
///
/// # Example
///
/// ```ignore
/// use r3bl_tui::*;
///
/// // Delete selected items
/// let delete_action = BatchAction {
///     key_binding: key_press! { @char 'd' },
///     description: "Delete selected items".into(),
///     handler: Box::new(|items, selected_indices, _state| {
///         // Remove in reverse order to preserve indices
///         for &idx in selected_indices.iter().rev() {
///             items.remove(idx);
///         }
///         Ok(())
///     }),
/// };
///
/// // Mark all as complete
/// let complete_action = BatchAction {
///     key_binding: key_press! { @char 'c' },
///     description: "Complete all selected".into(),
///     handler: Box::new(|items, selected_indices, _state| {
///         for &idx in selected_indices {
///             items[idx].completed = true;
///         }
///         Ok(())
///     }),
/// };
/// ```
pub struct BatchAction<S, AS, I>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
    I: ListItem<S, AS>,
{
    /// Key binding that triggers this action
    pub key_binding: KeyPress,

    /// Human-readable description (for help text)
    pub description: String,

    /// Handler function
    ///
    /// Receives:
    /// - `&mut Vec<I>`: Mutable access to all items
    /// - `&[usize]`: Indices of selected items (sorted)
    /// - `&mut S`: Mutable access to application state
    ///
    /// The handler can modify items, remove them, or update state. It should
    /// return `Ok(())` on success or an error if the operation fails.
    pub handler: BatchActionHandler<S, I>,

    _phantom: PhantomData<AS>,
}

impl<S, AS, I> Debug for BatchAction<S, AS, I>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
    I: ListItem<S, AS>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BatchAction")
            .field("key_binding", &self.key_binding)
            .field("description", &self.description)
            .field("handler", &"<function>")
            .finish()
    }
}

impl<S, AS, I> BatchAction<S, AS, I>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
    I: ListItem<S, AS>,
{
    /// Creates a new batch action.
    #[must_use]
    pub fn new(
        key_binding: KeyPress,
        description: String,
        handler: BatchActionHandler<S, I>,
    ) -> Self {
        Self {
            key_binding,
            description,
            handler,
            _phantom: PhantomData,
        }
    }
}

impl<S, AS, I> ListComponent<S, AS, I>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
    I: ListItem<S, AS>,
{
    /// Creates a new list component with the given items.
    ///
    /// # Parameters
    ///
    /// - `id`: Unique identifier for this component (used for focus management)
    /// - `items`: Initial list of items (must implement [`ListItem`])
    ///
    /// # Initial State
    ///
    /// - Cursor at first item (if any items exist)
    /// - No items selected
    /// - Viewport at top of list
    /// - No batch actions registered
    ///
    /// # Panics
    ///
    /// Does not panic, but will have undefined behavior if `items` is empty.
    /// Consider checking `items.is_empty()` and handling appropriately.
    #[must_use]
    pub fn new(id: FlexBoxId, items: Vec<I>) -> Self {
        let cursor_id = items
            .first()
            .map_or(ListItemId::new(0), ListItem::id);

        Self {
            id,
            items,
            cursor_id,
            selected_ids: HashSet::new(),
            scroll_offset_index: 0,
            viewport_height: height(10), // Default, will be updated on render
            batch_actions: Vec::new(),
            _phantom: PhantomData,
        }
    }

    /// Creates a boxed instance for storing in component registry.
    ///
    /// This is the preferred way to create components when using the TUI framework,
    /// as the component registry requires `Box<dyn Component<S, AS>>`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use r3bl_tui::*;
    ///
    /// let list = ListComponent::new_boxed(my_id, items);
    /// ComponentRegistry::put(component_registry, my_id, list);
    /// ```
    #[must_use]
    pub fn new_boxed(id: FlexBoxId, items: Vec<I>) -> Box<Self> {
        Box::new(Self::new(id, items))
    }

    /// Adds a batch action to the list.
    ///
    /// Batch actions are only available when 2 or more items are selected. They
    /// provide a way to perform operations on multiple items at once.
    ///
    /// # Example
    ///
    /// ```ignore
    /// list.add_batch_action(BatchAction {
    ///     key_binding: key_press! { @char 'd' },
    ///     description: "Delete selected items".into(),
    ///     handler: Box::new(|items, selected_indices, _state| {
    ///         for &idx in selected_indices.iter().rev() {
    ///             items.remove(idx);
    ///         }
    ///         Ok(())
    ///     }),
    /// });
    /// ```
    pub fn add_batch_action(&mut self, action: BatchAction<S, AS, I>) {
        self.batch_actions.push(action);
    }

    /// Returns the current index of the cursor item, if it exists in the list.
    ///
    /// This performs a linear search through items to find the cursor ID.
    /// Returns `None` if the cursor ID is not found (which indicates corrupted state).
    #[must_use]
    pub fn get_cursor_index(&self) -> Option<usize> {
        self.items
            .iter()
            .position(|item| item.id() == self.cursor_id)
    }

    /// Checks if an item is currently selected.
    #[must_use]
    pub fn is_selected(&self, item_id: ListItemId) -> bool {
        self.selected_ids.contains(&item_id)
    }

    /// Toggles selection for the item at the cursor.
    pub fn toggle_selection_at_cursor(&mut self) {
        if self.selected_ids.contains(&self.cursor_id) {
            self.selected_ids.remove(&self.cursor_id);
        } else {
            self.selected_ids.insert(self.cursor_id);
        }
    }

    /// Clears all selections.
    pub fn clear_selections(&mut self) {
        self.selected_ids.clear();
    }
}
