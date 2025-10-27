// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Stack-allocated list type for performance-critical rendering operations.

use crate::InlineVecStr;
use smallvec::SmallVec;
use std::ops::{AddAssign, Deref, DerefMut};

/// Storage type for `RenderList` - `SmallVec` with inline capacity of 16.
pub type RenderListStorage<T> = SmallVec<[T; RENDER_LIST_STORAGE_SIZE]>;
const RENDER_LIST_STORAGE_SIZE: usize = 16;

/// **Stack-allocated** performance-optimized list for hot-path rendering operations.
/// Uses `SmallVec<[T; 16]>` (16-item stack-allocated inline capacity) internally to
/// eliminate heap allocations in tight loops.
///
/// **Why Stack Allocation for Rendering?**
///
/// This type prioritizes performance over stack safety, making it ideal for rendering
/// code paths that are executed repeatedly (syntax highlighting, render operations,
/// etc.).
///
/// # Performance Benefits of Stack Allocation
///
/// ## 1. Zero Heap Allocations (for typical sizes)
///
/// `SmallVec<[T; 16]>` (16-item stack-allocated inline capacity) stores up to 16 items
/// **directly inline on the stack**, avoiding heap allocations entirely for the common
/// case:
///
/// - **Typical rendering**: Most render operations involve 8-16 styled text spans
/// - **Stack storage**: ~648 bytes allocated inline ([`SmallVec`] header + 16 × ~40-byte
///   items)
/// - **Result**: Zero allocator calls, zero heap fragmentation
///
/// ## 2. Cache Locality
///
/// Stack-allocated data benefits from excellent cache performance:
///
/// - **Stack data**: Lives in recently-used memory regions (L1/L2 cache)
/// - **Heap data**: May be scattered across memory, causing cache misses
/// - **Impact**: Better cache hit rates = faster memory access
///
/// ## 3. Measured Performance Gain
///
/// Benchmarking showed concrete improvements for rendering operations:
///
/// - **+5% performance** for syntax highlighting hot path
/// - **Benchmark**: `bench_smallvec_text_line_render` vs `bench_vec_text_line_render`
/// - **Why it matters**: Rendering happens continuously during scrolling/editing
///
/// ## 4. No Allocator Contention
///
/// Eliminating heap allocations removes allocator overhead:
///
/// - **No malloc/free calls** in hot path
/// - **No allocator locks** in multi-threaded scenarios
/// - **Predictable performance**: No allocator-induced latency spikes
///
/// # Trade-offs
///
/// ## Stack Usage
///
/// - **Cost**: ~648 bytes per `RenderList` on the stack
/// - **Safe for**: Shallow call stacks (1-10 levels deep)
/// - **Unsafe for**: Deep recursion (300+ frames) → See [`crate::ParseList`] for that use
///   case
///
/// ## When to Use
///
/// - ✅ **Hot rendering paths** (called thousands of times per second)
/// - ✅ **Shallow call stacks** (typical UI rendering: 5-10 levels)
/// - ✅ **Predictable sizes** (most renders: 8-16 items)
/// - ❌ **Deep recursion** (parsers with 300+ frames) → Use [`crate::ParseList`]
/// - ❌ **Unpredictable growth** (user-controlled list sizes) → Use `Vec` directly
///
/// ## Historical Context
///
/// Increasing [`SmallVec`] size from `[8]` to `[16]` gave measurable performance gains in
/// benchmarks, justifying the increased stack usage for rendering hot paths. For
/// safer contexts like parsing, see [`crate::ParseList`] which uses heap allocation.
///
/// # See Also
///
/// - [`crate::ParseList`]: Heap-allocated for stack safety with deep recursion
/// - [`crate::List`]: General-purpose with `SmallVec<[T; 8]>` (8-item stack-allocated
///   inline capacity) for balanced trade-offs
///
/// [`SmallVec`]: https://docs.rs/smallvec/latest/smallvec/
#[derive(Clone, Default, PartialEq, Debug)]
pub struct RenderList<T> {
    pub inner: RenderListStorage<T>,
}

impl<T> RenderList<T> {
    #[must_use]
    pub fn with_capacity(size: usize) -> Self {
        Self {
            inner: RenderListStorage::with_capacity(size),
        }
    }

    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: RenderListStorage::new(),
        }
    }
}

/// Add (other) item to render list (self).
impl<T> AddAssign<T> for RenderList<T> {
    fn add_assign(&mut self, other_item: T) { self.push(other_item); }
}

/// Add (other) render list to render list (self).
impl<T> AddAssign<RenderList<T>> for RenderList<T> {
    fn add_assign(&mut self, other_list: RenderList<T>) { self.extend(other_list.inner); }
}

/// Add (other) vec to render list (self).
impl<T> AddAssign<Vec<T>> for RenderList<T> {
    fn add_assign(&mut self, other_vec: Vec<T>) { self.extend(other_vec); }
}

impl<'a> From<InlineVecStr<'a>> for RenderList<&'a str> {
    fn from(other: InlineVecStr<'a>) -> Self {
        let mut it = RenderList::with_capacity(other.len());
        it.extend(other);
        it
    }
}

impl<T> From<Vec<T>> for RenderList<T> {
    fn from(other: Vec<T>) -> Self {
        let mut it = RenderList::with_capacity(other.len());
        it.extend(other);
        it
    }
}

impl<T> Deref for RenderList<T> {
    type Target = RenderListStorage<T>;
    fn deref(&self) -> &Self::Target { &self.inner }
}

impl<T> DerefMut for RenderList<T> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

impl<T> From<&[T]> for RenderList<T>
where
    T: Clone,
{
    fn from(slice: &[T]) -> Self {
        let mut it = Self::with_capacity(slice.len());
        for item in slice {
            it.push(item.clone());
        }
        it
    }
}

impl<T, const N: usize> From<[T; N]> for RenderList<T>
where
    T: Clone,
{
    fn from(array: [T; N]) -> Self {
        let mut it = Self::with_capacity(N);
        for item in array {
            it.push(item);
        }
        it
    }
}

/// Create a [`RenderList`] from a list of items.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{render_list, RenderList};
///
/// let list: RenderList<i32> = render_list![1, 2, 3];
/// assert_eq!(list.len(), 3);
/// ```
#[macro_export]
macro_rules! render_list {
    (
        $($item: expr),*
        $(,)* /* Optional trailing comma https://stackoverflow.com/a/43143459/2085356. */
    ) => {
        {
            #[allow(unused_mut)]
            let mut it = $crate::RenderList::new();
            $(
                it.inner.push($item);
            )*
            it
        }
    };
}
