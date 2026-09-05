// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

pub use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet, FxHasher};

/// High-performance [`HashMap`] utilizing [`FxHasher`] (~1 CPU cycle lookups). Replaces
/// [`std::collections::HashMap`] across the crate to eliminate `SipHash` overhead.
///
/// [`FxHasher`]: rustc_hash::FxHasher
pub type HashMap<K, V> = FxHashMap<K, V>;

/// High-performance [`HashSet`] utilizing [`FxHasher`] (~1 CPU cycle lookups). Replaces
/// [`std::collections::HashSet`] across the crate to eliminate `SipHash` overhead.
///
/// [`FxHasher`]: rustc_hash::FxHasher
pub type HashSet<T> = FxHashSet<T>;

/// Type alias for an environment variable map with fast [`FxHasher`].
///
/// [`FxHasher`]: rustc_hash::FxHasher
pub type EnvMap = HashMap<String, String>;
