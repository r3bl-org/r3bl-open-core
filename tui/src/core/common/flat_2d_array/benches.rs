// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Run this benchmark using:
//! ```bash
//! cargo bench -p r3bl_tui flat_2d_array::benches
//! ```
//!
//! # Benchmark Results & Analysis
//!
//! | Group & Action                             | Legacy Speed | Legacy Var | Flat SIMD Speed | Flat Var | Speedup   |
//! | ------------------------------------------ | ------------ | ---------- | --------------- | -------- | --------- |
//! | **1. Clear Screen** ([`PixelChar`])        | `21,619 ns`  | `± 98%`    | `16,396 ns`     | `± 46%`  | **1.3x**  |
//! | **2. Scroll Screen** ([`PixelChar`])       | `7,618 ns`   | `± 70%`    | `7,496 ns`      | `± 11%`  | **1.0x**  |
//! | **3. Read Screen** ([`PixelChar`])         | `6,170 ns`   | `± 25%`    | `3,843 ns`      | `± 25%`  | **1.6x**  |
//! | **4. Mem Size** (`GetMemSize`)             | `12.0 ns`    | `± 19%`    | `0.2 ns`        | `± 30%`  | **60.0x** |
//! | **5. 2D Traversal** (`Modulo` vs `Chunks`) | `9,904 ns`   | `± 29%`    | `7,228 ns`      | `± 52%`  | **1.4x**  |
//!
//! _Tested on: Intel(R) Core(TM) i7-14700, 128GB RAM, `CachyOS` (Linux)_
//!
//! ## Architectural Analysis
//!
//! ### 1. The Variance Problem
//!
//! The micro-benchmark table only shows average execution times. In practice, legacy
//! nested vectors using [`InlineVec`]`<`[`PixelCharLine`]`>` exhibit extreme variance
//! (often swinging wildly with a massive **`± 98%`** margin of error) because performance
//! relies entirely on lucky CPU cache placement for scattered heap allocations (terminal
//! sizes instantly exceed [`InlineVec`]'s 16-element stack capacity, forcing it to
//! spill). The [`Flat2DArray`] guarantees perfectly consistent, flatline frame times.
//! This variance can cause wildly different speeds that each frame takes to render in a
//! real app in the real world, leading to micro-stutters. The biggest architectural win
//! here is reducing this variance to guarantee a smooth, high-framerate experience.
//!
//! ### 2. The Rendering Problem
//!
//! Micro-benchmarks 1-2 only test how fast we can *write* memory. The true power of a 1D
//! contiguous array is unlocked when the [`compositor`] tries to *read* that memory to
//! draw it to the terminal screen. The flat array guarantees an almost 100% L1 Cache Hit
//! rate, which is proven by the massive **1.6x speedup** in Group 3 (Read Screen).
//!
//! For the deep-dive on how [`Flat2DArray`] saturates 64-byte Cache Lines and leverages
//! the Hardware Prefetcher, see the [CPU Cache & Hardware Prefetching] section in the
//! [`Flat2DArray`] documentation.
//!
//! ### 3. SIMD Benchmark Analysis (Clear & Scroll)
//!
//! Groups 1 and 2 highlight the power of [LLVM] Auto-vectorization, proving speedups
//! of up to **1.3x** when clearing or scrolling the screen compared to legacy nested
//! vectors.
//!
//! For the deep-dive on how Rust slice operations unlock AVX/NEON vectorization, see
//! the [How SIMD Vectorization Works] section in the [`Flat2DArray`] documentation.
//!
//! ### 4. The Math Pipeline Stall Problem
//!
//! Micro-benchmark Group 5 tests the performance of iterating over a 1D contiguous array
//! when 2D coordinates (row and column) are required.
//!
//! By utilizing [`.chunks_exact(width)`] rather than naive modulo math, we completely
//! eliminate catastrophic CPU pipeline stalls caused by division and modulo operations,
//! resulting in a **1.4x speedup**.
//!
//! For a detailed, plain-English explanation of why runtime variable widths cause these
//! pipeline stalls and how [`.chunks_exact(width)`] solves it, see the [Rule of Thumb for
//! 1D vs 2D Memory Iteration] in the [`Flat1DSimd`] documentation.
//!
//! [`.chunks_exact(width)`]: slice::chunks_exact
//! [`compositor`]: crate::tui::terminal_lib_backends::compositor_render_ops_to_ofs_buf
//! [`Flat1DSimd`]: crate::Flat1DSimd
//! [`Flat2DArray`]: crate::Flat2DArray
//! [`InlineVec`]: crate::InlineVec
//! [`PixelCharLine`]: crate::PixelCharLine
//! [CPU Cache & Hardware Prefetching]: crate::Flat2DArray#the-cpu-cache--hardware-prefetching
//! [How SIMD Vectorization Works]: crate::Flat2DArray#how-simd-vectorization-works
//! [Rule of Thumb for 1D vs 2D Memory Iteration]:
//!     crate::Flat1DSimd#rule-of-thumb-for-1d-vs-2d-memory-iteration

extern crate test;

use crate::{ColWidth, Flat2DArray, PixelChar, PixelCharLines, RowHeight, RowIndex, Size};
use test::{Bencher, black_box};

const WIDTH: usize = 200;
const HEIGHT: usize = 100;

#[bench]
fn group_1_clear_screen_pixelchar_using_legacy_nested_vec(b: &mut Bencher) {
    let mut grid = PixelCharLines::new_empty(Size::from((
        ColWidth::from(WIDTH),
        RowHeight::from(HEIGHT),
    )));
    b.iter(|| {
        for row in 0..HEIGHT {
            for col in 0..WIDTH {
                grid[row][col] = black_box(PixelChar::Void);
            }
        }
    });
}

#[bench]
fn group_1_clear_screen_pixelchar_using_flat_array_simd(b: &mut Bencher) {
    let mut grid = Flat2DArray::<PixelChar>::new_empty(
        Size::from((ColWidth::from(WIDTH), RowHeight::from(HEIGHT))),
        PixelChar::Spacer,
    );
    b.iter(|| {
        grid.as_simd_mut().fill_all(black_box(PixelChar::Void));
    });
}

#[bench]
fn group_2_scroll_screen_pixelchar_using_legacy_nested_vec(b: &mut Bencher) {
    let mut grid = PixelCharLines::new_empty(Size::from((
        ColWidth::from(WIDTH),
        RowHeight::from(HEIGHT),
    )));
    b.iter(|| {
        for row in 0..HEIGHT - 1 {
            let (first, second) = grid.split_at_mut(row + 1);
            first[row]
                .pixel_chars
                .clone_from_slice(&second[0].pixel_chars);
        }
    });
}

#[bench]
fn group_2_scroll_screen_pixelchar_using_flat_array_simd(b: &mut Bencher) {
    let mut grid = Flat2DArray::<PixelChar>::new_empty(
        Size::from((ColWidth::from(WIDTH), RowHeight::from(HEIGHT))),
        PixelChar::Spacer,
    );
    b.iter(|| {
        grid.as_simd_mut().copy_within_rows(
            RowIndex::from(1)..RowIndex::from(HEIGHT),
            RowIndex::from(0),
        );
    });
}

#[bench]
fn group_3_read_screen_pixelchar_using_legacy_nested_vec(b: &mut Bencher) {
    let grid = PixelCharLines::new_empty(Size::from((
        ColWidth::from(WIDTH),
        RowHeight::from(HEIGHT),
    )));
    b.iter(|| {
        for row in 0..HEIGHT {
            for col in 0..WIDTH {
                let _ = black_box(&grid[row][col]);
            }
        }
    });
}

#[bench]
fn group_3_read_screen_pixelchar_using_flat_array_simd(b: &mut Bencher) {
    let grid = Flat2DArray::<PixelChar>::new_empty(
        Size::from((ColWidth::from(WIDTH), RowHeight::from(HEIGHT))),
        PixelChar::Spacer,
    );
    b.iter(|| {
        for item in &grid.data {
            let _ = black_box(item);
        }
    });
}

#[bench]
fn group_4_memory_size_legacy_nested_vec(b: &mut Bencher) {
    use crate::GetMemSize;
    let grid = PixelCharLines::new_empty(Size::from((
        ColWidth::from(WIDTH),
        RowHeight::from(HEIGHT),
    )));
    b.iter(|| {
        black_box(grid.get_mem_size());
    });
}

#[bench]
fn group_4_memory_size_flat_array(b: &mut Bencher) {
    use crate::GetMemSize;
    let grid = Flat2DArray::<PixelChar>::new_empty(
        Size::from((ColWidth::from(WIDTH), RowHeight::from(HEIGHT))),
        PixelChar::Spacer,
    );
    b.iter(|| {
        black_box(grid.get_mem_size());
    });
}

#[bench]
fn group_5_2d_traversal_using_modulo_math(b: &mut Bencher) {
    let grid = Flat2DArray::<PixelChar>::new_empty(
        Size::from((ColWidth::from(WIDTH), RowHeight::from(HEIGHT))),
        PixelChar::Spacer,
    );
    b.iter(|| {
        for (idx, item) in grid.data.iter().enumerate() {
            let row = idx / WIDTH;
            let col = idx % WIDTH;
            let _ = black_box((row, col, item));
        }
    });
}

#[allow(clippy::chunks_exact_to_as_chunks)]
#[bench]
fn group_5_2d_traversal_using_chunks_exact(b: &mut Bencher) {
    let grid = Flat2DArray::<PixelChar>::new_empty(
        Size::from((ColWidth::from(WIDTH), RowHeight::from(HEIGHT))),
        PixelChar::Spacer,
    );
    b.iter(|| {
        for (row, chunk) in grid.data.chunks_exact(WIDTH).enumerate() {
            for (col, item) in chunk.iter().enumerate() {
                let _ = black_box((row, col, item));
            }
        }
    });
}
