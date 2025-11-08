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

use std::{fmt::Debug, marker::PhantomData, collections::{HashSet, HashMap}};

use crate::{RowHeight, KeyPress, CommonResult, height};
use crate::tui::FlexBoxId;

use super::{ListItem, ListItemId, FlexBoxIdPool};

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
/// # Phase 3: Complex Items with FlexBox Rendering
///
/// For items that need nested layouts, the component manages a pool of FlexBoxIds
/// that are dynamically assigned to visible items. See [`ComplexListItem`] for details.
///
/// # Example (Simple Items)
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
///     fn id(&self) -> ListItemId {
///         ListItemId::new(self.id)
///     }
/// }
///
/// impl SimpleListItem<AppState, AppSignal> for TodoItem {
///     // ... render_line and handle_event implementation
/// }
///
/// // Create the list
/// let items = vec![
///     TodoItem { id: 1, title: "Buy milk".into(), completed: false },
///     TodoItem { id: 2, title: "Write code".into(), completed: false },
/// ];
///
/// let mut list = ListComponent::new_simple(my_flexbox_id, items);
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

    /// Pool of FlexBoxIds for complex items (Phase 3)
    ///
    /// Only used when items implement [`ComplexListItem`]. The pool manages
    /// temporary rendering slots that are assigned to items as they scroll into
    /// the viewport and recycled when they scroll out.
    ///
    /// For simple items, this field is `None`.
    pub flexbox_id_pool: Option<FlexBoxIdPool>,

    /// Maps ListItemId → FlexBoxId for currently visible complex items
    ///
    /// Tracks which items currently have an assigned rendering slot. Entries are
    /// added when items scroll into view and removed when they scroll out.
    ///
    /// For simple items, this map remains empty.
    pub visible_item_flexbox_mapping: HashMap<ListItemId, FlexBoxId>,

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
    /// Creates a new list component with simple items (Phase 1).
    ///
    /// Use this constructor for items implementing [`SimpleListItem`] that render
    /// as single lines of text.
    ///
    /// # Parameters
    ///
    /// - `id`: Unique identifier for this component (used for focus management)
    /// - `items`: Initial list of items (must implement [`SimpleListItem`])
    ///
    /// # Initial State
    ///
    /// - Cursor at first item (if any items exist)
    /// - No items selected
    /// - Viewport at top of list
    /// - No batch actions registered
    /// - No FlexBox pool (simple items don't need it)
    ///
    /// # Panics
    ///
    /// Does not panic, but will have undefined behavior if `items` is empty.
    /// Consider checking `items.is_empty()` and handling appropriately.
    #[must_use]
    pub fn new_simple(id: FlexBoxId, items: Vec<I>) -> Self {
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
            flexbox_id_pool: None,
            visible_item_flexbox_mapping: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    /// Creates a new list component with complex items (Phase 3).
    ///
    /// Use this constructor for items implementing [`ComplexListItem`] that need
    /// nested FlexBox layouts.
    ///
    /// # Parameters
    ///
    /// - `id`: Unique identifier for this component (used for focus management)
    /// - `items`: Initial list of items (must implement [`ComplexListItem`])
    /// - `viewport_height_estimate`: Expected viewport height in rows (for pool sizing)
    ///
    /// # Pool Sizing
    ///
    /// The FlexBoxId pool is sized as `viewport_height_estimate + 5` to provide
    /// a buffer for smooth scrolling. If the actual viewport is larger, some items
    /// may fail to render (logged as warnings).
    ///
    /// # ID Allocation
    ///
    /// The pool allocates IDs in the range `[id+1, id+1+pool_size)`. Ensure this
    /// range doesn't overlap with other components in your application.
    ///
    /// # Panics
    ///
    /// Panics if the pool range would overflow `u8::MAX`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use r3bl_tui::*;
    ///
    /// let list = ListComponent::new_complex(
    ///     FlexBoxId::new(5),  // List component ID
    ///     items,
    ///     20,  // Expect ~20 rows viewport
    /// );
    /// // Pool will use IDs 6..31 (25 slots)
    /// ```
    #[must_use]
    pub fn new_complex(id: FlexBoxId, items: Vec<I>, viewport_height_estimate: usize) -> Self {
        let cursor_id = items
            .first()
            .map_or(ListItemId::new(0), ListItem::id);

        let pool_size = viewport_height_estimate + 5;
        let pool_base_id = id.inner + 1;
        let pool = FlexBoxIdPool::new(pool_base_id, pool_size);

        Self {
            id,
            items,
            cursor_id,
            selected_ids: HashSet::new(),
            scroll_offset_index: 0,
            viewport_height: height(10), // Default, will be updated on render
            batch_actions: Vec::new(),
            flexbox_id_pool: Some(pool),
            visible_item_flexbox_mapping: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    /// Creates a new list component (legacy constructor).
    ///
    /// This constructor is kept for backward compatibility. New code should use
    /// [`Self::new_simple`] or [`Self::new_complex`] instead.
    ///
    /// Equivalent to [`Self::new_simple`].
    #[must_use]
    pub fn new(id: FlexBoxId, items: Vec<I>) -> Self {
        Self::new_simple(id, items)
    }

    /// Creates a boxed instance with simple items for storing in component registry.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use r3bl_tui::*;
    ///
    /// let list = ListComponent::new_simple_boxed(my_id, items);
    /// ComponentRegistry::put(component_registry, my_id, list);
    /// ```
    #[must_use]
    pub fn new_simple_boxed(id: FlexBoxId, items: Vec<I>) -> Box<Self> {
        Box::new(Self::new_simple(id, items))
    }

    /// Creates a boxed instance with complex items for storing in component registry.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use r3bl_tui::*;
    ///
    /// let list = ListComponent::new_complex_boxed(my_id, items, 20);
    /// ComponentRegistry::put(component_registry, my_id, list);
    /// ```
    #[must_use]
    pub fn new_complex_boxed(id: FlexBoxId, items: Vec<I>, viewport_height_estimate: usize) -> Box<Self> {
        Box::new(Self::new_complex(id, items, viewport_height_estimate))
    }

    /// Creates a boxed instance for storing in component registry (legacy).
    ///
    /// This is kept for backward compatibility. New code should use
    /// [`Self::new_simple_boxed`] or [`Self::new_complex_boxed`] instead.
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
