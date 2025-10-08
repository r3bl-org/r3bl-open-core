// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ## `SmallVec` vs Vec Benchmark Suite
//!
//! This module benchmarks the performance of `SmallVec` vs Vec for `TuiStyledTexts`
//! to determine the optimal data structure for our use case.
//!
//! ## Running the Benchmarks
//!
//! ```bash
//! # Run all benchmarks in this file
//! cargo bench vec_vs_smallvec
//!
//! # Run specific size benchmarks
//! cargo bench bench_small_
//! cargo bench bench_medium_
//! cargo bench bench_large_
//! ```
//!
//! ## Benchmark Categories
//!
//! - **Small collections (1-5 items)**: Where `SmallVec` should excel
//! - **Medium collections (8-20 items)**: Around the threshold
//! - **Large collections (30-100 items)**: Where Vec might be better
//! - **Extend operations**: The main performance issue from flamegraph
//! - **Mixed operations**: Realistic usage patterns

#[cfg(test)]
mod benchmarks {
    extern crate test;
    use crate::{TuiStyle, TuiStyledText, tui_styled_text};
    use smallvec::SmallVec;
    use test::Bencher;

    // Type aliases for different configurations.
    type SmallVec32 = SmallVec<[TuiStyledText; 32]>;
    type SmallVec16 = SmallVec<[TuiStyledText; 16]>;
    type SmallVec8 = SmallVec<[TuiStyledText; 8]>;
    type RegularVec = Vec<TuiStyledText>;

    // Helper to create test data.
    fn create_styled_texts(count: usize) -> Vec<TuiStyledText> {
        (0..count)
            .map(|i| {
                tui_styled_text! {
                    @style: TuiStyle::default(),
                    @text: format!("Text item {}", i)
                }
            })
            .collect()
    }

    // =============================================================================
    // Small collection benchmarks (1-5 items)
    // =============================================================================

    #[bench]
    fn bench_small_smallvec32_create(b: &mut Bencher) {
        b.iter(|| {
            let mut vec: SmallVec32 = SmallVec::new();
            for item in create_styled_texts(3) {
                vec.push(item);
            }
            vec
        });
    }

    #[bench]
    fn bench_small_smallvec16_create(b: &mut Bencher) {
        b.iter(|| {
            let mut vec: SmallVec16 = SmallVec::new();
            for item in create_styled_texts(3) {
                vec.push(item);
            }
            vec
        });
    }

    #[bench]
    fn bench_small_smallvec8_create(b: &mut Bencher) {
        b.iter(|| {
            let mut vec: SmallVec8 = SmallVec::new();
            for item in create_styled_texts(3) {
                vec.push(item);
            }
            vec
        });
    }

    #[bench]
    fn bench_small_vec_create(b: &mut Bencher) {
        b.iter(|| {
            let mut vec: RegularVec = Vec::new();
            for item in create_styled_texts(3) {
                vec.push(item);
            }
            vec
        });
    }

    // Small extend operations.
    #[bench]
    fn bench_small_smallvec32_extend(b: &mut Bencher) {
        let items = create_styled_texts(3);
        b.iter(|| {
            let mut vec: SmallVec32 = SmallVec::new();
            vec.extend(items.clone());
            vec
        });
    }

    #[bench]
    fn bench_small_smallvec8_extend(b: &mut Bencher) {
        let items = create_styled_texts(3);
        b.iter(|| {
            let mut vec: SmallVec8 = SmallVec::new();
            vec.extend(items.clone());
            vec
        });
    }

    #[bench]
    fn bench_small_vec_extend(b: &mut Bencher) {
        let items = create_styled_texts(3);
        b.iter(|| {
            let mut vec: RegularVec = Vec::new();
            vec.extend(items.clone());
            vec
        });
    }

    // =============================================================================
    // Medium collection benchmarks (8-20 items)
    // =============================================================================

    #[bench]
    fn bench_medium_smallvec32_create(b: &mut Bencher) {
        b.iter(|| {
            let mut vec: SmallVec32 = SmallVec::new();
            for item in create_styled_texts(15) {
                vec.push(item);
            }
            vec
        });
    }

    #[bench]
    fn bench_medium_smallvec16_create(b: &mut Bencher) {
        b.iter(|| {
            let mut vec: SmallVec16 = SmallVec::new();
            for item in create_styled_texts(15) {
                vec.push(item);
            }
            vec
        });
    }

    #[bench]
    fn bench_medium_smallvec8_create(b: &mut Bencher) {
        b.iter(|| {
            let mut vec: SmallVec8 = SmallVec::new();
            for item in create_styled_texts(15) {
                vec.push(item);
            }
            vec
        });
    }

    #[bench]
    fn bench_medium_vec_create(b: &mut Bencher) {
        b.iter(|| {
            let mut vec: RegularVec = Vec::new();
            for item in create_styled_texts(15) {
                vec.push(item);
            }
            vec
        });
    }

    // Medium extend operations.
    #[bench]
    fn bench_medium_smallvec32_extend(b: &mut Bencher) {
        let items = create_styled_texts(15);
        b.iter(|| {
            let mut vec: SmallVec32 = SmallVec::new();
            vec.extend(items.clone());
            vec
        });
    }

    #[bench]
    fn bench_medium_smallvec8_extend(b: &mut Bencher) {
        let items = create_styled_texts(15);
        b.iter(|| {
            let mut vec: SmallVec8 = SmallVec::new();
            vec.extend(items.clone());
            vec
        });
    }

    #[bench]
    fn bench_medium_vec_extend(b: &mut Bencher) {
        let items = create_styled_texts(15);
        b.iter(|| {
            let mut vec: RegularVec = Vec::new();
            vec.extend(items.clone());
            vec
        });
    }

    // =============================================================================
    // Large collection benchmarks (30-100 items)
    // =============================================================================

    #[bench]
    fn bench_large_smallvec32_create(b: &mut Bencher) {
        b.iter(|| {
            let mut vec: SmallVec32 = SmallVec::new();
            for item in create_styled_texts(50) {
                vec.push(item);
            }
            vec
        });
    }

    #[bench]
    fn bench_large_smallvec16_create(b: &mut Bencher) {
        b.iter(|| {
            let mut vec: SmallVec16 = SmallVec::new();
            for item in create_styled_texts(50) {
                vec.push(item);
            }
            vec
        });
    }

    #[bench]
    fn bench_large_smallvec8_create(b: &mut Bencher) {
        b.iter(|| {
            let mut vec: SmallVec8 = SmallVec::new();
            for item in create_styled_texts(50) {
                vec.push(item);
            }
            vec
        });
    }

    #[bench]
    fn bench_large_vec_create(b: &mut Bencher) {
        b.iter(|| {
            let mut vec: RegularVec = Vec::new();
            for item in create_styled_texts(50) {
                vec.push(item);
            }
            vec
        });
    }

    // Large extend operations.
    #[bench]
    fn bench_large_smallvec32_extend(b: &mut Bencher) {
        let items = create_styled_texts(50);
        b.iter(|| {
            let mut vec: SmallVec32 = SmallVec::new();
            vec.extend(items.clone());
            vec
        });
    }

    #[bench]
    fn bench_large_smallvec8_extend(b: &mut Bencher) {
        let items = create_styled_texts(50);
        b.iter(|| {
            let mut vec: SmallVec8 = SmallVec::new();
            vec.extend(items.clone());
            vec
        });
    }

    #[bench]
    fn bench_large_vec_extend(b: &mut Bencher) {
        let items = create_styled_texts(50);
        b.iter(|| {
            let mut vec: RegularVec = Vec::new();
            vec.extend(items.clone());
            vec
        });
    }

    // =============================================================================
    // Realistic usage pattern benchmarks.
    // =============================================================================

    #[bench]
    fn bench_realistic_smallvec32_multiple_extends(b: &mut Bencher) {
        b.iter(|| {
            let mut vec: SmallVec32 = SmallVec::new();
            // Simulate building up a collection with multiple extends.
            vec.extend(create_styled_texts(5));
            vec.extend(create_styled_texts(8));
            vec.extend(create_styled_texts(12));
            vec
        });
    }

    #[bench]
    fn bench_realistic_smallvec8_multiple_extends(b: &mut Bencher) {
        b.iter(|| {
            let mut vec: SmallVec8 = SmallVec::new();
            // Simulate building up a collection with multiple extends.
            vec.extend(create_styled_texts(5));
            vec.extend(create_styled_texts(8));
            vec.extend(create_styled_texts(12));
            vec
        });
    }

    #[bench]
    fn bench_realistic_vec_multiple_extends(b: &mut Bencher) {
        b.iter(|| {
            let mut vec: RegularVec = Vec::new();
            // Simulate building up a collection with multiple extends.
            vec.extend(create_styled_texts(5));
            vec.extend(create_styled_texts(8));
            vec.extend(create_styled_texts(12));
            vec
        });
    }

    // Pre-allocation benchmarks.
    #[bench]
    fn bench_realistic_vec_with_capacity(b: &mut Bencher) {
        b.iter(|| {
            let mut vec: RegularVec = Vec::with_capacity(25);
            vec.extend(create_styled_texts(5));
            vec.extend(create_styled_texts(8));
            vec.extend(create_styled_texts(12));
            vec
        });
    }

    // =============================================================================
    // Drop/cleanup benchmarks
    // =============================================================================

    #[bench]
    fn bench_drop_smallvec32_large(b: &mut Bencher) {
        b.iter(|| {
            let mut vec: SmallVec32 = SmallVec::new();
            vec.extend(create_styled_texts(50));
            // Implicit drop at end of scope.
        });
    }

    #[bench]
    fn bench_drop_smallvec8_large(b: &mut Bencher) {
        b.iter(|| {
            let mut vec: SmallVec8 = SmallVec::new();
            vec.extend(create_styled_texts(50));
            // Implicit drop at end of scope.
        });
    }

    #[bench]
    fn bench_drop_vec_large(b: &mut Bencher) {
        b.iter(|| {
            let mut vec: RegularVec = Vec::new();
            vec.extend(create_styled_texts(50));
            // Implicit drop at end of scope.
        });
    }
}
