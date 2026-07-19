// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::VPHeight;

/// Storage capacity constraint for the maximum total number of lines (history + active
/// screen lines) retained in memory.
///
/// This is a storage capacity limit and is completely independent of 2D viewport
/// pan/scroll state. When new lines are appended to storage, if total lines exceed this
/// capacity constraint, the oldest lines are evicted from the top of the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageLineLimit {
    /// No limit. Lines are never evicted from storage.
    Unlimited,

    /// A fixed capacity limit on the maximum number of history lines retained in storage.
    ///
    /// When total storage capacity (history + viewport height) is exceeded, the oldest
    /// line is popped from the front of the queue and evicted to enforce the capacity
    /// limit.
    Fixed(usize),
}

impl StorageLineLimit {
    /// Calculates the maximum total line capacity (history limit + viewport row height)
    /// allowed for this storage line limit and viewport height.
    #[must_use]
    pub fn calc_max_line_count(&self, vp_height: VPHeight) -> Option<usize> {
        match self {
            StorageLineLimit::Fixed(limit) => Some(*limit + vp_height.as_usize()),
            StorageLineLimit::Unlimited => None,
        }
    }
}
