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

//! Global syntax highlighting resources cache.
//!
//! This module provides global caching for expensive syntax highlighting resources
//! that are loaded once and reused throughout the application lifetime.
//!
//! # Performance
//!
//! Loading syntax definitions and themes is expensive (~1.82% CPU per instance).
//! By caching these resources globally, we eliminate repeated deserialization overhead
//! in dialog-heavy applications where multiple editors are created.
//!
//! ## Benchmark Results
//!
//! The performance improvements from caching are dramatic:
//!
//! ### Individual Resource Loading
//! | Resource    | Uncached       | Cached   | Improvement              |
//! |-------------|----------------|----------|--------------------------|
//! | `SyntaxSet` | 654,835.90 ns  | 0.19 ns  | **3,446,504x faster**    |
//! | Theme       | 106,754.70 ns  | 0.19 ns  | **561,866x faster**      |
//!
//! ### Multiple Editor Creation (10 editors)
//! | Scenario   | Uncached         | Cached  | Improvement              |
//! |------------|------------------|---------|--------------------------|
//! | Total time | 3,920,191.40 ns  | 1.99 ns | **1,969,945x faster**    |
//!
//! In practical terms:
//! - Creating a `SyntaxSet` takes ~0.65ms (expensive deserialization)
//! - Creating a Theme takes ~0.11ms (file I/O or default theme creation)
//! - With caching, access is essentially free (0.19 ns)
//! - For dialog-heavy apps creating 10 editors, we save ~3.92ms per dialog
//!
//! See `docs/task_tui_perf_optimize.md` for more details.

use std::sync::LazyLock;

use syntect::{highlighting::Theme, parsing::SyntaxSet};

use crate::{load_default_theme, try_load_r3bl_theme};

/// Global storage for syntax highlighting resources. [`LazyLock`] is an idiomatic way of
/// creating a lazily initialized static variable in Rust.
static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);

/// Global storage for syntax highlighting resources. [`LazyLock`] is an idiomatic way of
/// creating a lazily initialized static variable in Rust.
static THEME: LazyLock<Theme> =
    LazyLock::new(|| try_load_r3bl_theme().unwrap_or_else(|_| load_default_theme()));

/// Get the cached syntax set, loading it on first access.
///
/// This function is thread-safe and the syntax set is loaded once and cached
/// for the lifetime of the program.
#[must_use]
pub fn get_cached_syntax_set() -> &'static SyntaxSet { &SYNTAX_SET }

/// Get the cached theme, loading it on first access.
///
/// This function is thread-safe and the theme is loaded once and cached
/// for the lifetime of the program.
#[must_use]
pub fn get_cached_theme() -> &'static Theme { &THEME }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cached_syntax_set_returns_same_instance() {
        let set1 = get_cached_syntax_set();
        let set2 = get_cached_syntax_set();

        // Verify we get the same instance (same memory address)
        assert_eq!(std::ptr::addr_of!(*set1), std::ptr::addr_of!(*set2));
    }

    #[test]
    fn test_cached_theme_returns_same_instance() {
        let theme1 = get_cached_theme();
        let theme2 = get_cached_theme();

        // Verify we get the same instance (same memory address)
        assert_eq!(std::ptr::addr_of!(*theme1), std::ptr::addr_of!(*theme2));
    }

    #[test]
    fn test_syntax_set_contains_expected_syntaxes() {
        let syntax_set = get_cached_syntax_set();

        // Verify some common syntaxes are loaded
        assert!(syntax_set.find_syntax_by_extension("rs").is_some());
        assert!(syntax_set.find_syntax_by_extension("md").is_some());
        // Note: syntect's load_defaults_newlines() might not include all extensions
        // Just verify we have some syntaxes loaded
        assert!(!syntax_set.syntaxes().is_empty());
    }
}

#[cfg(test)]
mod bench {
    extern crate test;
    use test::Bencher;

    use super::*;

    /// Benchmark: Creating new `SyntaxSet` instances repeatedly
    /// This simulates the old behavior where each `EditorEngine` created its own
    /// `SyntaxSet`
    #[bench]
    fn bench_create_syntax_set_uncached(b: &mut Bencher) {
        b.iter(|| {
            // This is expensive - loads and deserializes all syntax definitions
            let _syntax_set = SyntaxSet::load_defaults_newlines();
        });
    }

    /// Benchmark: Using cached `SyntaxSet` via `get_cached_syntax_set()`
    /// This simulates the new optimized behavior
    #[bench]
    fn bench_get_cached_syntax_set(b: &mut Bencher) {
        // Ensure the cache is initialized before benchmarking
        let _ = get_cached_syntax_set();

        b.iter(|| {
            // This should be nearly free - just returns a reference
            let _syntax_set = get_cached_syntax_set();
        });
    }

    /// Benchmark: Creating new Theme instances repeatedly
    /// This simulates the old behavior where each `EditorEngine` created its own Theme
    #[bench]
    fn bench_create_theme_uncached(b: &mut Bencher) {
        b.iter(|| {
            // This involves file I/O or fallback to default theme
            let _theme = try_load_r3bl_theme().unwrap_or_else(|_| load_default_theme());
        });
    }

    /// Benchmark: Using cached Theme via `get_cached_theme()`
    /// This simulates the new optimized behavior
    #[bench]
    fn bench_get_cached_theme(b: &mut Bencher) {
        // Ensure the cache is initialized before benchmarking
        let _ = get_cached_theme();

        b.iter(|| {
            // This should be nearly free - just returns a reference
            let _theme = get_cached_theme();
        });
    }

    /// Benchmark: Simulating dialog creation with uncached resources
    /// This shows the cumulative cost of creating multiple editors (e.g., in dialogs)
    #[bench]
    fn bench_multiple_editor_creation_uncached(b: &mut Bencher) {
        b.iter(|| {
            // Simulate creating 10 editors (common in dialog-heavy apps)
            for _ in 0..10 {
                let _syntax_set = SyntaxSet::load_defaults_newlines();
                let _theme =
                    try_load_r3bl_theme().unwrap_or_else(|_| load_default_theme());
            }
        });
    }

    /// Benchmark: Simulating dialog creation with cached resources
    /// This shows the performance benefit of caching for dialog-heavy applications
    #[bench]
    fn bench_multiple_editor_creation_cached(b: &mut Bencher) {
        // Ensure the cache is initialized before benchmarking
        let _ = get_cached_syntax_set();
        let _ = get_cached_theme();

        b.iter(|| {
            // Simulate creating 10 editors with cached resources
            for _ in 0..10 {
                let _syntax_set = get_cached_syntax_set();
                let _theme = get_cached_theme();
            }
        });
    }

    /// Benchmark: Access pattern comparison - first access vs subsequent accesses
    /// This demonstrates the one-time cost amortized over many accesses
    #[bench]
    fn bench_cache_warmup_and_access_pattern(b: &mut Bencher) {
        // Clear the cache for this test (would need a test-only clear method)
        // For now, this benchmark assumes a fresh start

        b.iter(|| {
            // First access pays the initialization cost
            let _set1 = get_cached_syntax_set();
            let _theme1 = get_cached_theme();

            // Subsequent accesses are essentially free
            for _ in 0..100 {
                let _set = get_cached_syntax_set();
                let _theme = get_cached_theme();
            }
        });
    }
}
