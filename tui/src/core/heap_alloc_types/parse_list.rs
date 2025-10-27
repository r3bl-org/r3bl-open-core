// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Heap-allocated list type for stack-safe parsing with recursive parsers.

use std::ops::{AddAssign, Deref, DerefMut};

/// Storage type for ParseList - standard Vec for heap allocation.
pub type ParseListStorage<T> = Vec<T>;

/// **Heap-allocated** list for parsing user input (markdown, config files, etc.).
/// Uses [Vec] internally for absolute stack safety with recursive parsers.
///
/// **Important**: This type is **heap-allocated**, not stack-allocated!
/// The [Vec] descriptor (24 bytes) lives on the stack, but all contents are stored
/// on the heap. This is intentional for safety with deeply recursive parsers.
///
/// # Why Heap Allocation Instead of Stack (SmallVec)?
///
/// This type deliberately uses heap allocation via [Vec] rather than stack optimization
/// with [SmallVec], due to stack overflow issues with deeply recursive parsers.
///
/// ## The Problem: Stack Overflow in Recursive Parsers
///
/// When parsing complex nested markdown documents, the nom parser combinator library
/// creates deep call stacks through recursion:
///
/// - Complex markdown documents → 300+ recursive parser frames
/// - Each frame allocates parser state on the stack
/// - If list types use `[SmallVec][16]`, each adds 648 bytes per frame
/// - **Total stack usage**: 648 bytes × 300 frames = **194,400 bytes (~189 KB)**
/// - **Result**: Stack overflow! (Default stack is typically 8 MB, but recursive parsers
///   combined with large inline allocations quickly exhaust it)
///
/// ## The Solution: Heap Allocation via Vec
///
/// By using [Vec] instead of [SmallVec], we keep stack footprint minimal:
///
/// - [Vec] descriptor: **24 bytes** (ptr + len + capacity) on the stack
/// - **Contents stored on heap**, not stack (this is the key difference!)
/// - **Total stack usage**: 24 bytes × 300 frames = **7,200 bytes (~7 KB)**
/// - **Result**: No stack overflow! Stack usage well within limits
///
/// ## Performance Trade-off
///
/// - **Cost**: Heap allocations during parsing (acceptable for parsing phase)
/// - **Benefit**: Can parse arbitrarily deep documents without stack overflow
/// - **Why acceptable**: Parsing happens once; rendering (which uses [`RenderList`] with
///   [SmallVec] optimization) happens repeatedly and benefits from inline storage
///
/// ## Historical Context
///
/// Originally, this used [`List`] (backed by `[SmallVec][8]` = 328 bytes per frame).
/// An optimization attempt increased it to `[SmallVec][16]` for 5% performance gain,
/// which caused stack overflow in `test_parse_markdown_valid` with complex documents.
/// The fix: separate domain-specific types where [ParseList] uses [Vec] for safety,
/// while [`RenderList`] keeps `[SmallVec][16]` optimization for the hot rendering path.
///
/// ## See Also
///
/// - [`RenderList`]: Performance-optimized with `[SmallVec][16]` for rendering
///   (stack-allocated)
/// - [`List`]: General-purpose with `[SmallVec][8]` for balanced performance/safety
///   (stack-allocated)
///
/// [SmallVec]: crate::smallvec
/// [`crate::RenderList`]: crate::RenderList
/// [`crate::List`]: crate::List
#[derive(Clone, Default, PartialEq, Debug)]
pub struct ParseList<T> {
    pub inner: ParseListStorage<T>,
}

impl<T> From<&[T]> for ParseList<T>
where
    T: Clone,
{
    fn from(slice: &[T]) -> Self {
        Self {
            inner: slice.to_vec(),
        }
    }
}

impl<T, const N: usize> From<[T; N]> for ParseList<T>
where
    T: Clone,
{
    fn from(array: [T; N]) -> Self {
        Self {
            inner: array.to_vec(),
        }
    }
}

impl<T> ParseList<T> {
    #[must_use]
    pub fn with_capacity(size: usize) -> Self {
        Self {
            inner: ParseListStorage::with_capacity(size),
        }
    }

    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: ParseListStorage::new(),
        }
    }
}

/// Add (other) item to parse list (self).
impl<T> AddAssign<T> for ParseList<T> {
    fn add_assign(&mut self, other_item: T) { self.push(other_item); }
}

/// Add (other) parse list to parse list (self).
impl<T> AddAssign<ParseList<T>> for ParseList<T> {
    fn add_assign(&mut self, other_list: ParseList<T>) { self.extend(other_list.inner); }
}

/// Add (other) vec to parse list (self).
impl<T> AddAssign<Vec<T>> for ParseList<T> {
    fn add_assign(&mut self, other_vec: Vec<T>) { self.extend(other_vec); }
}

impl<T> From<Vec<T>> for ParseList<T> {
    fn from(other: Vec<T>) -> Self { Self { inner: other } }
}

impl<T> Deref for ParseList<T> {
    type Target = ParseListStorage<T>;
    fn deref(&self) -> &Self::Target { &self.inner }
}

impl<T> DerefMut for ParseList<T> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

/// Create a [`ParseList`] from a list of items.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{parse_list, ParseList};
///
/// let list: ParseList<i32> = parse_list![1, 2, 3];
/// assert_eq!(list.len(), 3);
/// ```
#[macro_export]
macro_rules! parse_list {
    (
        $($item: expr),*
        $(,)* /* Optional trailing comma https://stackoverflow.com/a/43143459/2085356. */
    ) => {
        {
            #[allow(unused_mut)]
            let mut it = $crate::ParseList::new();
            $(
                it.inner.push($item);
            )*
            it
        }
    };
}
