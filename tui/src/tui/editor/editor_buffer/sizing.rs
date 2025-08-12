// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{EditorBuffer, EditorContent, cur_index::CurIndex, history::EditorHistory};
use crate::{CachedMemorySize, CaretRaw, GetMemSize, InlineString, MemoizedMemorySize,
            MemorySize, RingBufferHeap, ScrOfs, TinyInlineString, get_mem_size};

/// The version history is stored on the heap, as a ring buffer.
pub type HistoryBuffer = RingBufferHeap<EditorContent, MAX_UNDO_REDO_SIZE>;

/// This is the absolute maximum number of undo/redo steps that will ever be stored.
pub const MAX_UNDO_REDO_SIZE: usize = 16;

impl GetMemSize for EditorContent {
    fn get_mem_size(&self) -> usize {
        self.lines.get_mem_size()
            + std::mem::size_of::<CaretRaw>()
            + std::mem::size_of::<ScrOfs>()
            + std::mem::size_of::<Option<TinyInlineString>>()
            + std::mem::size_of::<Option<InlineString>>()
            + self.sel_list.get_mem_size()
    }
}

impl GetMemSize for EditorHistory {
    fn get_mem_size(&self) -> usize {
        let versions_size = get_mem_size::ring_buffer_size(&self.versions);
        let cur_index_field_size = std::mem::size_of::<CurIndex>();
        versions_size + cur_index_field_size
    }
}

/// Memory size caching for performance optimization.
impl GetMemSize for EditorBuffer {
    fn get_mem_size(&self) -> usize {
        self.content.get_mem_size() + self.history.get_mem_size()
    }
}

impl CachedMemorySize for EditorBuffer {
    fn memory_size_cache(&self) -> &MemoizedMemorySize { &self.memory_size_calc_cache }

    fn memory_size_cache_mut(&mut self) -> &mut MemoizedMemorySize {
        &mut self.memory_size_calc_cache
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
