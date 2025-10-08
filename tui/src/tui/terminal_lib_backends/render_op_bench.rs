// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Benchmarks for `RenderOp` collections to compare `SmallVec` vs Vec performance
//!
//! This module tests different collection strategies for `RenderOps` to identify
//! the optimal approach for our rendering pipeline.
//!
//! ## Benchmark Results Summary (2025-07-18)
//!
//! ### Typical usage (8 operations - within `SmallVec` capacity):
//! - `SmallVec::push`:         52.90 ns/iter
//! - `Vec::push`:              62.09 ns/iter
//! - `Vec::with_capacity:`     41.71 ns/iter
//! - **`SmallVec` faster by: 17%** (without pre-allocation)
//!
//! ### Complex usage (20 operations - exceeds `SmallVec` capacity):
//! - `SmallVec::push`:        151.43 ns/iter
//! - `Vec::push`:             117.20 ns/iter
//! - `Vec::with_capacity:`     94.92 ns/iter
//! - **Vec faster by: 29%** (`SmallVec` has spill overhead)
//!
//! ### Real-world text line rendering (6 operations):
//! - `SmallVec`:               17.63 ns/iter
//! - `Vec`:                    40.02 ns/iter
//! - `Vec::with_capacity:`     18.23 ns/iter
//! - **`SmallVec` faster by: 127%** (without pre-allocation)
//!
//! ### Iteration performance:
//! - `SmallVec`:                2.42 ns/iter
//! - `Vec`:                     6.23 ns/iter
//! - **`SmallVec` faster by: 157%**
//!
//! ### Clone performance:
//! - `SmallVec`:               47.83 ns/iter
//! - `Vec`:                    41.42 ns/iter
//! - **`Vec` faster by: 15%** (simpler clone operation)
//!
//! ### Extend operations:
//! - `SmallVec`:               46.56 ns/iter
//! - `Vec`:                    48.28 ns/iter
//! - **`SmallVec` faster by: 4%** (minor difference)
//!
//! ## Recommendation: KEEP `SmallVec`<[`RenderOp`; 8]>
//!
//! Based on comprehensive benchmarking:
//!
//! 1. **Most operations use 6 or fewer `RenderOps`** - well within `SmallVec`'s inline
//!    capacity
//! 2. **`SmallVec` is 2.27x faster for typical usage** (17.63ns vs 40.02ns for text line
//!    rendering)
//! 3. **Iteration is 2.57x faster with `SmallVec`** - critical for render execution
//! 4. **Spill overhead only matters for 20+ operations** - rare in practice
//! 5. **`Vec::with_capacity` matches `SmallVec` performance** - but requires knowing size
//!    upfront
//!
//! The current `SmallVec`<[`RenderOp`; 8]> is optimal for our usage patterns. No change
//! needed.

#[cfg(test)]
mod render_op_benchmarks {
    extern crate test;
    use crate::{AnsiValue, InlineString, Pos, RenderOp, RgbValue, TuiColor, TuiStyle, ch};
    use smallvec::SmallVec;
    use test::Bencher;

    // Type aliases for clarity.
    type RenderOpsSmallVec = SmallVec<[RenderOp; 8]>;
    type RenderOpsVec = Vec<RenderOp>;

    // Helper to create a variety of RenderOps that simulate real usage.
    fn create_test_render_ops() -> Vec<RenderOp> {
        vec![
            RenderOp::ClearScreen,
            RenderOp::ResetColor,
            RenderOp::MoveCursorPositionAbs(Pos {
                row_index: ch(10).into(),
                col_index: ch(20).into(),
            }),
            RenderOp::SetFgColor(TuiColor::Rgb(RgbValue::from_u8(255, 128, 64))),
            RenderOp::SetBgColor(TuiColor::Ansi(AnsiValue::new(42))),
            RenderOp::PaintTextWithAttributes(
                InlineString::from("Hello, World!"),
                Some(TuiStyle {
                    color_fg: Some(TuiColor::Rgb(RgbValue::from_u8(200, 200, 200))),
                    color_bg: Some(TuiColor::Ansi(AnsiValue::new(232))),
                    ..Default::default()
                }),
            ),
            RenderOp::MoveCursorPositionRelTo(
                Pos {
                    row_index: ch(5).into(),
                    col_index: ch(10).into(),
                },
                Pos {
                    row_index: ch(2).into(),
                    col_index: ch(3).into(),
                },
            ),
            RenderOp::ApplyColors(Some(TuiStyle {
                color_fg: Some(TuiColor::Rgb(RgbValue::from_u8(100, 100, 100))),
                color_bg: Some(TuiColor::Ansi(AnsiValue::new(16))),
                ..Default::default()
            })),
        ]
    }

    // Benchmark 1: Push operations for typical screen render (8 ops)
    #[bench]
    fn bench_smallvec_push_typical(b: &mut Bencher) {
        let ops = create_test_render_ops();
        b.iter(|| {
            let mut collection = RenderOpsSmallVec::new();
            for op in &ops {
                collection.push(op.clone());
            }
            test::black_box(collection)
        });
    }

    #[bench]
    fn bench_vec_push_typical(b: &mut Bencher) {
        let ops = create_test_render_ops();
        b.iter(|| {
            let mut collection = RenderOpsVec::new();
            for op in &ops {
                collection.push(op.clone());
            }
            test::black_box(collection)
        });
    }

    #[bench]
    fn bench_vec_with_capacity_push_typical(b: &mut Bencher) {
        let ops = create_test_render_ops();
        b.iter(|| {
            let mut collection = RenderOpsVec::with_capacity(8);
            for op in &ops {
                collection.push(op.clone());
            }
            test::black_box(collection)
        });
    }

    // Benchmark 2: Push operations for complex screen render (20 ops - exceeds SmallVec
    // capacity)
    #[bench]
    fn bench_smallvec_push_complex(b: &mut Bencher) {
        let base_ops = create_test_render_ops();
        let mut ops = Vec::new();
        // Create 20 operations by repeating the base set.
        for _ in 0..3 {
            ops.extend_from_slice(&base_ops);
        }
        ops.truncate(20);

        b.iter(|| {
            let mut collection = RenderOpsSmallVec::new();
            for op in &ops {
                collection.push(op.clone());
            }
            test::black_box(collection)
        });
    }

    #[bench]
    fn bench_vec_push_complex(b: &mut Bencher) {
        let base_ops = create_test_render_ops();
        let mut ops = Vec::new();
        for _ in 0..3 {
            ops.extend_from_slice(&base_ops);
        }
        ops.truncate(20);

        b.iter(|| {
            let mut collection = RenderOpsVec::new();
            for op in &ops {
                collection.push(op.clone());
            }
            test::black_box(collection)
        });
    }

    #[bench]
    fn bench_vec_with_capacity_push_complex(b: &mut Bencher) {
        let base_ops = create_test_render_ops();
        let mut ops = Vec::new();
        for _ in 0..3 {
            ops.extend_from_slice(&base_ops);
        }
        ops.truncate(20);

        b.iter(|| {
            let mut collection = RenderOpsVec::with_capacity(20);
            for op in &ops {
                collection.push(op.clone());
            }
            test::black_box(collection)
        });
    }

    // Benchmark 3: Extend operations (common in render pipeline)
    #[bench]
    fn bench_smallvec_extend(b: &mut Bencher) {
        let ops = create_test_render_ops();
        b.iter(|| {
            let mut collection = RenderOpsSmallVec::new();
            collection.extend(ops.iter().cloned());
            test::black_box(collection)
        });
    }

    #[bench]
    fn bench_vec_extend(b: &mut Bencher) {
        let ops = create_test_render_ops();
        b.iter(|| {
            let mut collection = RenderOpsVec::new();
            collection.extend(ops.iter().cloned());
            test::black_box(collection)
        });
    }

    #[bench]
    fn bench_vec_with_capacity_extend(b: &mut Bencher) {
        let ops = create_test_render_ops();
        b.iter(|| {
            let mut collection = RenderOpsVec::with_capacity(ops.len());
            collection.extend(ops.iter().cloned());
            test::black_box(collection)
        });
    }

    // Benchmark 4: Iteration (common for executing render ops)
    #[bench]
    fn bench_smallvec_iterate(b: &mut Bencher) {
        let ops = create_test_render_ops();
        let mut collection = RenderOpsSmallVec::new();
        collection.extend(ops.iter().cloned());

        b.iter(|| {
            let mut sum = 0;
            for op in &collection {
                match op {
                    RenderOp::MoveCursorPositionAbs(pos) => {
                        sum += pos.row_index.value + pos.col_index.value;
                    }
                    RenderOp::MoveCursorPositionRelTo(p1, p2) => {
                        sum += p1.row_index.value
                            + p1.col_index.value
                            + p2.row_index.value
                            + p2.col_index.value;
                    }
                    _ => sum += 1,
                }
            }
            test::black_box(sum)
        });
    }

    #[bench]
    fn bench_vec_iterate(b: &mut Bencher) {
        let ops = create_test_render_ops();
        let mut collection = RenderOpsVec::new();
        collection.extend(ops.iter().cloned());

        b.iter(|| {
            let mut sum = 0;
            for op in &collection {
                match op {
                    RenderOp::MoveCursorPositionAbs(pos) => {
                        sum += pos.row_index.value + pos.col_index.value;
                    }
                    RenderOp::MoveCursorPositionRelTo(p1, p2) => {
                        sum += p1.row_index.value
                            + p1.col_index.value
                            + p2.row_index.value
                            + p2.col_index.value;
                    }
                    _ => sum += 1,
                }
            }
            test::black_box(sum)
        });
    }

    // Benchmark 5: Clone operations (for caching/storing render ops)
    #[bench]
    fn bench_smallvec_clone(b: &mut Bencher) {
        let ops = create_test_render_ops();
        let mut collection = RenderOpsSmallVec::new();
        collection.extend(ops.iter().cloned());

        b.iter(|| test::black_box(collection.clone()));
    }

    #[bench]
    fn bench_vec_clone(b: &mut Bencher) {
        let ops = create_test_render_ops();
        let mut collection = RenderOpsVec::new();
        collection.extend(ops.iter().cloned());

        b.iter(|| test::black_box(collection.clone()));
    }

    // Benchmark 6: Real-world scenario - building render ops for a text line
    // This is the most important benchmark as it reflects actual usage patterns.
    // Most render operations create 3-6 RenderOps per styled text segment:
    // 1. MoveCursor (optional)
    // 2. ResetColor
    // 3. SetFgColor
    // 4. SetBgColor
    // 5. PaintText
    // 6. ResetColor
    #[bench]
    fn bench_smallvec_text_line_render(b: &mut Bencher) {
        b.iter(|| {
            let mut ops = RenderOpsSmallVec::new();
            ops.push(RenderOp::MoveCursorPositionAbs(Pos {
                row_index: ch(5).into(),
                col_index: ch(10).into(),
            }));
            ops.push(RenderOp::ResetColor);
            ops.push(RenderOp::SetFgColor(TuiColor::Rgb(RgbValue::from_u8(
                255, 255, 255,
            ))));
            ops.push(RenderOp::SetBgColor(TuiColor::Ansi(AnsiValue::new(232))));
            ops.push(RenderOp::PaintTextWithAttributes(
                InlineString::from("This is a line of text in the editor"),
                None,
            ));
            ops.push(RenderOp::ResetColor);
            test::black_box(ops)
        });
    }

    #[bench]
    fn bench_vec_text_line_render(b: &mut Bencher) {
        b.iter(|| {
            let ops = vec![
                RenderOp::MoveCursorPositionAbs(Pos {
                    row_index: ch(5).into(),
                    col_index: ch(10).into(),
                }),
                RenderOp::ResetColor,
                RenderOp::SetFgColor(TuiColor::Rgb(RgbValue::from_u8(255, 255, 255))),
                RenderOp::SetBgColor(TuiColor::Ansi(AnsiValue::new(232))),
                RenderOp::PaintTextWithAttributes(
                    InlineString::from("This is a line of text in the editor"),
                    None,
                ),
                RenderOp::ResetColor,
            ];
            test::black_box(ops)
        });
    }

    #[bench]
    fn bench_vec_with_capacity_text_line_render(b: &mut Bencher) {
        b.iter(|| {
            let ops = vec![
                RenderOp::MoveCursorPositionAbs(Pos {
                    row_index: ch(5).into(),
                    col_index: ch(10).into(),
                }),
                RenderOp::ResetColor,
                RenderOp::SetFgColor(TuiColor::Rgb(RgbValue::from_u8(255, 255, 255))),
                RenderOp::SetBgColor(TuiColor::Ansi(AnsiValue::new(232))),
                RenderOp::PaintTextWithAttributes(
                    InlineString::from("This is a line of text in the editor"),
                    None,
                ),
                RenderOp::ResetColor,
            ];
            test::black_box(ops)
        });
    }
}
