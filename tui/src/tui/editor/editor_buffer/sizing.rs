// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{EditorBuffer, EditorContent};
use crate::{CCaret, CPos, CachedMemorySize, GetMemSize, InlineString,
            MemoizedMemorySize, MemorySize, TinyInlineString};
use std::mem::size_of;

impl GetMemSize for EditorContent {
    fn get_mem_size(&self) -> usize {
        self.lines.get_mem_size()
            + size_of::<CCaret>()
            + size_of::<CPos>()
            + size_of::<Option<TinyInlineString>>()
            + size_of::<Option<InlineString>>()
            + self.selection.get_mem_size()
    }
}

/// Memory size caching for performance optimization.
impl GetMemSize for EditorBuffer {
    fn get_mem_size(&self) -> usize {
        self.get_content().get_mem_size() + self.get_history().get_mem_size()
    }
}

impl CachedMemorySize for EditorBuffer {
    fn memory_size_cache(&self) -> &MemoizedMemorySize {
        self.get_memory_size_calc_cache()
    }

    fn memory_size_cache_mut(&mut self) -> &mut MemoizedMemorySize {
        self.get_memory_size_calc_cache_mut()
    }
}

impl EditorBuffer {
    /// Invalidates and immediately recalculates the memory size cache.
    /// Call this when buffer content changes to ensure the cache is always valid.
    pub fn invalidate_memory_size_calc_cache(&mut self) {
        self.invalidate_memory_size_cache();
        self.update_memory_size_cache(); // Immediately recalculate
    }

    /// Updates cache if dirty or not present.
    /// The closure is only called if recalculation is needed.
    pub fn upsert_memory_size_calc_cache(&mut self) { self.update_memory_size_cache(); }

    /// Gets the cached memory size value, recalculating if necessary.
    /// This is used by external code to access buffer memory size efficiently.
    /// The expensive memory calculation is only performed if the cache is invalid or
    /// empty. Returns a `MemorySize` that displays "?" if the cache is not
    /// available.
    #[must_use]
    pub fn get_memory_size_calc_cached(&mut self) -> MemorySize {
        self.get_cached_memory_size()
    }
}
