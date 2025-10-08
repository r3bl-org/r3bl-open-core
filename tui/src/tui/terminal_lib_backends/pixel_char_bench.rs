// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Benchmarks for `PixelChar` collections to compare `SmallVec` vs Vec performance
//!
//! The flamegraph shows 45M+ samples in `SmallVec` extend operations for `PixelChar`
//! collections. This module tests whether Vec would perform better.
//!
//! `PixelChar` is already optimized as a Copy type (not Clone), which should
//! make collection operations more efficient.
//!
//! ## Benchmark Results Summary (2025-07-18)
//!
//! ### Building typical line (80 chars):
//! - `SmallVec`:                257.74 ns/iter
//! - `Vec`:                     112.68 ns/iter
//! - `Vec::with_capacity`:       62.16 ns/iter
//! - **`Vec` is 2.3x faster, `Vec::with_capacity` is 4.1x faster**
//!
//! ### Random access pattern (simulating `print_text_with_attributes)`:
//! - `SmallVec`:                351.90 ns/iter
//! - `Vec`:                     117.50 ns/iter
//! - `Vec::with_capacity`:       72.62 ns/iter
//! - **`Vec` is 3.0x faster, `Vec::with_capacity` is 4.8x faster**
//!
//! ### Small line (8 chars - within `SmallVec` capacity):
//! - `SmallVec`:                  6.63 ns/iter
//! - `Vec`:                      30.17 ns/iter
//! - **`SmallVec` is 4.5x faster for small lines**
//!
//! ### Extend operations:
//! - `SmallVec`:                102.33 ns/iter
//! - `Vec`:                      46.26 ns/iter
//! - `Vec::with_capacity`:       37.34 ns/iter
//! - **`Vec` is 2.2x faster, `Vec::with_capacity` is 2.7x faster**
//!
//! ### Clone operations:
//! - `SmallVec`:                 89.46 ns/iter
//! - `Vec`:                      30.70 ns/iter
//! - **`Vec` is 2.9x faster**
//!
//! ### Iteration:
//! - `SmallVec`:                 30.59 ns/iter
//! - `Vec`:                      24.36 ns/iter
//! - **`Vec` is 25% faster**
//!
//! ## Recommendation: SWITCH to Vec for `PixelChar`
//!
//! Unlike `RenderOp` (which typically has 3-6 elements), `PixelChar` collections
//! typically have 80+ elements for a standard terminal line. This exceeds
//! `SmallVec`'s inline capacity of 8, causing:
//!
//! 1. **Frequent spills**: Lines longer than 8 chars spill to heap
//! 2. **Random access overhead**: The flamegraph shows this is a major issue
//! 3. **Clone overhead**: `SmallVec` cloning is slower when spilled
//!
//! The data strongly suggests replacing `SmallVec<[PixelChar; 8]>` with `Vec<PixelChar>`:
//! - Terminal lines are typically 80-200 chars (way beyond `SmallVec` capacity)
//! - Vec is 2-4x faster for typical operations
//! - Random access pattern (the main use case) shows 3-5x improvement
//! - Only small lines (<8 chars) benefit from `SmallVec`, but these are rare
//!
//! ### Optimization Strategy: `Vec::with_capacity`
//!
//! Pre-allocating with terminal width provides the best performance:
//! - 4.8x faster than `SmallVec` for random access patterns
//! - 4.1x faster for building typical lines
//! - Eliminates reallocation overhead
//! - Perfect for `OffscreenBuffer` which knows terminal dimensions

#[cfg(test)]
mod pixel_char_benchmarks {
    extern crate test;
    use crate::{PixelChar, RgbValue, TuiColor, TuiStyle};
    use smallvec::SmallVec;
    use test::Bencher;

    // Type aliases for clarity.
    type PixelCharSmallVec = SmallVec<[PixelChar; 8]>;
    type PixelCharVec = Vec<PixelChar>;

    // Helper to create test pixel chars.
    #[allow(clippy::cast_possible_truncation)]
    fn create_test_pixel_chars(count: usize) -> Vec<PixelChar> {
        let mut chars = Vec::with_capacity(count);
        for i in 0..count {
            let ch = match i % 3 {
                0 => PixelChar::PlainText {
                    display_char: ((i % 26) as u8 + b'a') as char,
                    style: TuiStyle {
                        color_fg: Some(TuiColor::Rgb(RgbValue::from_u8(255, 255, 255))),
                        color_bg: Some(TuiColor::Rgb(RgbValue::from_u8(0, 0, 0))),
                        ..Default::default()
                    },
                },
                1 => PixelChar::PlainText {
                    display_char: ' ',
                    style: TuiStyle::default(),
                },
                _ => PixelChar::Spacer,
            };
            chars.push(ch);
        }
        chars
    }

    // Benchmark 1: Building a typical text line (80 chars)
    #[allow(clippy::cast_sign_loss)]
    #[bench]
    fn bench_smallvec_build_line_typical(b: &mut Bencher) {
        b.iter(|| {
            let mut line = PixelCharSmallVec::new();
            for i in 0..80 {
                let ch = PixelChar::PlainText {
                    display_char: ((i % 26) as u8 + b'a') as char,
                    style: TuiStyle::default(),
                };
                line.push(ch);
            }
            test::black_box(line)
        });
    }

    #[allow(clippy::cast_sign_loss)]
    #[bench]
    fn bench_vec_build_line_typical(b: &mut Bencher) {
        b.iter(|| {
            let mut line = PixelCharVec::new();
            for i in 0..80 {
                let ch = PixelChar::PlainText {
                    display_char: ((i % 26) as u8 + b'a') as char,
                    style: TuiStyle::default(),
                };
                line.push(ch);
            }
            test::black_box(line)
        });
    }

    #[allow(clippy::cast_sign_loss)]
    #[bench]
    fn bench_vec_with_capacity_build_line_typical(b: &mut Bencher) {
        b.iter(|| {
            let mut line = PixelCharVec::with_capacity(80);
            for i in 0..80 {
                let ch = PixelChar::PlainText {
                    display_char: ((i % 26) as u8 + b'a') as char,
                    style: TuiStyle::default(),
                };
                line.push(ch);
            }
            test::black_box(line)
        });
    }

    // Benchmark 2: Random access pattern (simulating print_text_with_attributes)
    #[bench]
    fn bench_smallvec_random_access(b: &mut Bencher) {
        let chars = create_test_pixel_chars(80);
        b.iter(|| {
            let mut line = PixelCharSmallVec::new();
            // Initialize with spacers.
            for _ in 0..80 {
                line.push(PixelChar::Spacer);
            }
            // Random access pattern - update various positions.
            for (i, ch) in chars.iter().enumerate() {
                if i < line.len() {
                    line[i] = *ch;
                }
            }
            test::black_box(line)
        });
    }

    #[bench]
    fn bench_vec_random_access(b: &mut Bencher) {
        let chars = create_test_pixel_chars(80);
        b.iter(|| {
            let mut line = PixelCharVec::new();
            // Initialize with spacers.
            for _ in 0..80 {
                line.push(PixelChar::Spacer);
            }
            // Random access pattern - update various positions.
            for (i, ch) in chars.iter().enumerate() {
                if i < line.len() {
                    line[i] = *ch;
                }
            }
            test::black_box(line)
        });
    }

    #[bench]
    fn bench_vec_with_capacity_random_access(b: &mut Bencher) {
        let chars = create_test_pixel_chars(80);
        b.iter(|| {
            let mut line = PixelCharVec::with_capacity(80);
            // Initialize with spacers.
            for _ in 0..80 {
                line.push(PixelChar::Spacer);
            }
            // Random access pattern - update various positions.
            for (i, ch) in chars.iter().enumerate() {
                if i < line.len() {
                    line[i] = *ch;
                }
            }
            test::black_box(line)
        });
    }

    // Benchmark 3: Extend operations (used in rendering)
    #[bench]
    fn bench_smallvec_extend(b: &mut Bencher) {
        let chars = create_test_pixel_chars(40);
        b.iter(|| {
            let mut line = PixelCharSmallVec::new();
            line.extend(chars.iter().copied());
            line.extend(chars.iter().copied()); // Extend twice to simulate real usage
            test::black_box(line)
        });
    }

    #[bench]
    fn bench_vec_extend(b: &mut Bencher) {
        let chars = create_test_pixel_chars(40);
        b.iter(|| {
            let mut line = PixelCharVec::new();
            line.extend(chars.iter().copied());
            line.extend(chars.iter().copied());
            test::black_box(line)
        });
    }

    #[bench]
    fn bench_vec_with_capacity_extend(b: &mut Bencher) {
        let chars = create_test_pixel_chars(40);
        b.iter(|| {
            let mut line = PixelCharVec::with_capacity(80);
            line.extend(chars.iter().copied());
            line.extend(chars.iter().copied());
            test::black_box(line)
        });
    }

    // Benchmark 4: Clone operations (for buffer manipulation)
    #[bench]
    fn bench_smallvec_clone(b: &mut Bencher) {
        let mut line = PixelCharSmallVec::new();
        for ch in create_test_pixel_chars(80) {
            line.push(ch);
        }
        b.iter(|| test::black_box(line.clone()));
    }

    #[bench]
    fn bench_vec_clone(b: &mut Bencher) {
        let mut line = PixelCharVec::new();
        for ch in create_test_pixel_chars(80) {
            line.push(ch);
        }
        b.iter(|| test::black_box(line.clone()));
    }

    // Benchmark 5: Small line (8 chars - within SmallVec capacity)
    #[allow(clippy::cast_sign_loss)]
    #[bench]
    fn bench_smallvec_small_line(b: &mut Bencher) {
        b.iter(|| {
            let mut line = PixelCharSmallVec::new();
            for i in 0..8 {
                line.push(PixelChar::PlainText {
                    display_char: ((i % 26) as u8 + b'a') as char,
                    style: TuiStyle::default(),
                });
            }
            test::black_box(line)
        });
    }

    #[allow(clippy::cast_sign_loss)]
    #[bench]
    fn bench_vec_small_line(b: &mut Bencher) {
        b.iter(|| {
            let mut line = PixelCharVec::new();
            for i in 0..8 {
                line.push(PixelChar::PlainText {
                    display_char: ((i % 26) as u8 + b'a') as char,
                    style: TuiStyle::default(),
                });
            }
            test::black_box(line)
        });
    }

    // Benchmark 6: Iteration (for rendering)
    #[bench]
    fn bench_smallvec_iterate(b: &mut Bencher) {
        let mut line = PixelCharSmallVec::new();
        for ch in create_test_pixel_chars(80) {
            line.push(ch);
        }

        b.iter(|| {
            let mut count = 0;
            for ch in &line {
                match ch {
                    PixelChar::PlainText { display_char, .. } => {
                        count += *display_char as usize;
                    }
                    _ => count += 1,
                }
            }
            test::black_box(count)
        });
    }

    #[bench]
    fn bench_vec_iterate(b: &mut Bencher) {
        let mut line = PixelCharVec::new();
        for ch in create_test_pixel_chars(80) {
            line.push(ch);
        }

        b.iter(|| {
            let mut count = 0;
            for ch in &line {
                match ch {
                    PixelChar::PlainText { display_char, .. } => {
                        count += *display_char as usize;
                    }
                    _ => count += 1,
                }
            }
            test::black_box(count)
        });
    }
}
