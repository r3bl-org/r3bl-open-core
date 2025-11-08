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

use crate::tui::FlexBoxId;

/// Pool of FlexBoxIds for viewport-based rendering (Phase 3).
///
/// The ListComponent allocates a contiguous range of FlexBoxIds for visible items.
/// As items scroll into view, they borrow an ID from the pool. As they scroll out,
/// the ID is returned for reuse.
///
/// # Virtual Scrolling Pattern
///
/// This implements "virtual scrolling" for component identities:
/// - **Data identity** (`ListItemId`): Permanent, globally unique (u64)
/// - **Rendering slot** (`FlexBoxId`): Temporary, viewport-scoped (u8)
///
/// ```text
/// List with 1000 items, viewport shows 20:
///
/// ListItemId space:        FlexBoxId pool:
/// ┌────────────┐          ┌──────┐
/// │ Item #0    │          │ ID 10│ ← Assigned to visible item
/// │ Item #1    │          │ ID 11│ ← Assigned to visible item
/// │   ...      │          │  ...  │
/// │ Item #100  │ visible  │ ID 29│ ← Assigned to visible item
/// │ Item #101  │ visible  ├──────┤
/// │   ...      │ visible  │ ID 30│ ← Free (in pool)
/// │ Item #119  │ visible  │ ID 31│ ← Free (in pool)
/// │ Item #120  │          └──────┘
/// │   ...      │
/// │ Item #999  │
/// └────────────┘
/// ```
///
/// # Pool Size
///
/// The pool size should equal the maximum number of items that can be visible
/// simultaneously, plus a small buffer (3-5 IDs) for smooth scrolling:
///
/// ```
/// pool_size = viewport_height + buffer_size
/// ```
///
/// Typical values:
/// - Small lists (10-row viewport): pool_size = 15
/// - Medium lists (20-row viewport): pool_size = 25
/// - Large lists (50-row viewport): pool_size = 55
///
/// # Example
///
/// ```
/// use r3bl_tui::FlexBoxIdPool;
///
/// // Allocate IDs 10..30 (20 slots) for a list with viewport height 15
/// let mut pool = FlexBoxIdPool::new(10, 20);
///
/// // Borrow IDs as items scroll into view
/// let id1 = pool.borrow_id().expect("Pool has free IDs");
/// let id2 = pool.borrow_id().expect("Pool has free IDs");
///
/// // Return IDs as items scroll out of view
/// pool.return_id(id1);
/// pool.return_id(id2);
/// ```
#[derive(Debug, Clone)]
pub struct FlexBoxIdPool {
    /// Starting ID of the allocated range (inclusive)
    base_id: u8,

    /// Number of IDs in the pool
    pool_size: usize,

    /// Free IDs available for assignment (acts as a stack)
    free_ids: Vec<FlexBoxId>,
}

impl FlexBoxIdPool {
    /// Creates a new pool with a reserved range of FlexBoxIds.
    ///
    /// # Parameters
    ///
    /// - `base_id`: First ID in the range (inclusive)
    /// - `pool_size`: Number of IDs to allocate
    ///
    /// # Range Allocation
    ///
    /// The pool allocates IDs in the range `[base_id, base_id + pool_size)`.
    /// These IDs must not overlap with other components in the application.
    ///
    /// Recommended strategy:
    /// - List component ID: `n`
    /// - Pool range: `[n+1, n+1+pool_size)`
    ///
    /// # Panics
    ///
    /// Panics if `base_id + pool_size` would overflow `u8::MAX`.
    ///
    /// # Example
    ///
    /// ```
    /// use r3bl_tui::FlexBoxIdPool;
    ///
    /// // Allocate IDs 10..30
    /// let pool = FlexBoxIdPool::new(10, 20);
    /// ```
    #[must_use]
    pub fn new(base_id: u8, pool_size: usize) -> Self {
        assert!(
            (base_id as usize + pool_size) <= u8::MAX as usize + 1,
            "Pool range [{}..{}) would overflow u8::MAX",
            base_id,
            base_id as usize + pool_size
        );

        let end_id = base_id + pool_size as u8; // Safe due to assert above
        let free_ids = (base_id..end_id)
            .map(FlexBoxId::new)
            .collect();

        Self {
            base_id,
            pool_size,
            free_ids,
        }
    }

    /// Borrows an ID from the pool for an item scrolling into view.
    ///
    /// Returns `None` if the pool is exhausted (all IDs are currently assigned).
    /// This indicates the pool size is too small for the viewport.
    ///
    /// # Example
    ///
    /// ```
    /// use r3bl_tui::FlexBoxIdPool;
    ///
    /// let mut pool = FlexBoxIdPool::new(10, 20);
    /// let id = pool.borrow_id().expect("Pool has free IDs");
    /// ```
    pub fn borrow_id(&mut self) -> Option<FlexBoxId> {
        self.free_ids.pop()
    }

    /// Returns an ID to the pool when an item scrolls out of view.
    ///
    /// # Panics
    ///
    /// Panics if `id` is not in the pool's allocated range. This indicates
    /// a programming error (returning an ID that doesn't belong to this pool).
    ///
    /// # Example
    ///
    /// ```
    /// use r3bl_tui::FlexBoxIdPool;
    ///
    /// let mut pool = FlexBoxIdPool::new(10, 20);
    /// let id = pool.borrow_id().unwrap();
    ///
    /// // Later, when item scrolls out
    /// pool.return_id(id);
    /// ```
    pub fn return_id(&mut self, id: FlexBoxId) {
        assert!(
            self.is_in_range(id),
            "Cannot return FlexBoxId {:?} - not in pool range [{}..{})",
            id,
            self.base_id,
            self.base_id as usize + self.pool_size
        );

        self.free_ids.push(id);
    }

    /// Checks if an ID belongs to this pool's allocated range.
    #[must_use]
    fn is_in_range(&self, id: FlexBoxId) -> bool {
        let id_value = id.inner;
        id_value >= self.base_id && id_value < self.base_id + self.pool_size as u8
    }

    /// Returns the number of IDs currently available for borrowing.
    ///
    /// Use this to detect pool exhaustion before it happens.
    ///
    /// # Example
    ///
    /// ```
    /// use r3bl_tui::FlexBoxIdPool;
    ///
    /// let mut pool = FlexBoxIdPool::new(10, 20);
    /// assert_eq!(pool.available_count(), 20);
    ///
    /// let id = pool.borrow_id().unwrap();
    /// assert_eq!(pool.available_count(), 19);
    /// ```
    #[must_use]
    pub fn available_count(&self) -> usize {
        self.free_ids.len()
    }

    /// Returns the total capacity of the pool (borrowed + available).
    #[must_use]
    pub fn total_capacity(&self) -> usize {
        self.pool_size
    }

    /// Returns the base ID of the pool's allocated range.
    #[must_use]
    pub fn base_id(&self) -> u8 {
        self.base_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_creation() {
        let pool = FlexBoxIdPool::new(10, 20);
        assert_eq!(pool.available_count(), 20);
        assert_eq!(pool.total_capacity(), 20);
        assert_eq!(pool.base_id(), 10);
    }

    #[test]
    fn test_borrow_and_return() {
        let mut pool = FlexBoxIdPool::new(10, 5);

        let id1 = pool.borrow_id().expect("Should have free ID");
        assert_eq!(pool.available_count(), 4);

        let id2 = pool.borrow_id().expect("Should have free ID");
        assert_eq!(pool.available_count(), 3);

        pool.return_id(id1);
        assert_eq!(pool.available_count(), 4);

        pool.return_id(id2);
        assert_eq!(pool.available_count(), 5);
    }

    #[test]
    fn test_pool_exhaustion() {
        let mut pool = FlexBoxIdPool::new(10, 2);

        let _id1 = pool.borrow_id().expect("Should have free ID");
        let _id2 = pool.borrow_id().expect("Should have free ID");

        assert!(pool.borrow_id().is_none(), "Pool should be exhausted");
    }

    #[test]
    #[should_panic(expected = "not in pool range")]
    fn test_return_invalid_id() {
        let mut pool = FlexBoxIdPool::new(10, 5);
        let wrong_id = FlexBoxId::new(99);
        pool.return_id(wrong_id);
    }

    #[test]
    fn test_id_range() {
        let pool = FlexBoxIdPool::new(10, 5);

        assert!(pool.is_in_range(FlexBoxId::new(10)));
        assert!(pool.is_in_range(FlexBoxId::new(14)));
        assert!(!pool.is_in_range(FlexBoxId::new(9)));
        assert!(!pool.is_in_range(FlexBoxId::new(15)));
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn test_pool_overflow_detection() {
        FlexBoxIdPool::new(250, 10);
    }
}
