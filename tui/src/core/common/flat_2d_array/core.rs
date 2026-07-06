// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words LINESIZE getconf DCACHE VPCMPEQB

use crate::{ColWidth, GetMemSize, RowHeight};

/// A generic, highly optimized 1D array backing store that provides a safe 2D grid API.
///
/// # Architecture: Why this array is strictly non-growable
///
/// This array uses a [`Box`]`<[T]>` instead of a [`Vec`]`<T>`. This is a deliberate
/// design choice to make illegal states unrepresentable. A [`Vec`]`<T>` contains a
/// capacity field and allows methods like [`Vec::push`] or [`Vec::pop`] which could
/// silently change the length of the 1D buffer so it no longer perfectly matches `width *
/// height`, scrambling the 2D grid mapping.
///
/// By converting the [`Vec`]`<T>` into a [`Box`]`<[T]>` (a wide pointer) upon
/// initialization, we eliminate its ability to grow or shrink. [`Box`]`<[T]>` only
/// contains the wide pointer:
/// - the start address of the memory allocation, and the length,
/// - but not the capacity (which [`Vec`]`<T>` stores).
///
/// # Why we don't resize in-place
///
/// When a grid resize occurs, this data structure is not designed to be shifted or
/// re-mapped in-place.
///
/// Here is how this non-growable design fits into various use cases in the codebase:
///
/// ## 1. TUI Rendering Engines
///
/// The TUI rendering pipeline ([`paint`]) doesn't typically "mutate" a persistent canvas.
/// Instead, it executes the entire component tree onto a fresh buffer (like
/// [`OfsBuf`]), overwriting the entire screen from scratch every single frame.
/// It then diffs this new frame against the old frame to only paint the differences to
/// the terminal. If the user resizes the window, the engine doesn't need to
/// mathematically shift or preserve the old pixels in the offscreen buffer, it simply
/// asks the component tree to re-render everything into the new dimensions anyway.
///
/// ## 2. The Alternate Screen (Interactive CLI Apps)
///
/// When running an interactive CLI app in the Alternate screen (using an implementation
/// like [`OfsBufVT100`]), what happens when the window resizes? The terminal receives a
/// [`SIGWINCH`] and responds by immediately redrawing its entire UI from scratch at the
/// new dimensions. Just like in the TUI engine, there is no need to carefully preserve
/// and shift the 2D text during a resize, because the app will aggressively overwrite the
/// whole screen a millisecond later anyway.
///
/// ## 3. The Primary Screen (Standard Terminal Output)
///
/// For standard command-line output, we *do* want to preserve text scrollback. However,
/// in this architecture, we don't use this strictly sized 1D Array for that purpose!
/// Instead, a growable buffer of isolated rows (e.g., a [`VecDeque`] of
/// [`PixelCharLine`]s) is used (like [`ScrollbackBuffer`]). Because each row is its own
/// isolated allocation, resizing the viewport doesn't scramble any 1D-to-2D math. We
/// don't even need to resize the rows; we just pan the viewport!
///
/// # Performance: Scalar vs. [SIMD]
///
/// By default, this structure exposes 2D scalar operations such as:
/// - [`try_set()`] and
/// - `[row][col]` ([`Index`] and [`IndexMut`]).
///
/// However, for performance-critical hot loops (like diffing, clearing, or scrolling),
/// you should bypass the 2D abstraction by calling [`Self::as_simd()`] or
/// [`Self::as_simd_mut()`]. See [`Flat1DSimd`] for a detailed breakdown of the massive
/// architectural benefits this provides.
///
/// [`Box`]: std::boxed::Box
/// [`GetMemSize`]: crate::GetMemSize
/// [`Index`]: std::ops::Index
/// [`IndexMut`]: std::ops::IndexMut
/// [`OfsBuf`]: crate::OfsBuf
/// [`OfsBufVT100`]: crate::OfsBufVT100
/// [`paint`]: mod@crate::paint
/// [`PixelCharLine`]: crate::PixelCharLine
/// [`pop`]: std::vec::Vec::pop
/// [`push`]: std::vec::Vec::push
/// [`ScrollbackBuffer`]: crate::ScrollbackBuffer
/// [`SIGWINCH`]: https://en.wikipedia.org/wiki/Signal_(IPC)#SIGWINCH
/// [`try_set()`]: Self::try_set()
/// [`Vec`]: std::vec::Vec
/// [`VecDeque`]: std::collections::VecDeque
/// [SIMD]: https://en.wikipedia.org/wiki/SIMD
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flat2DArray<T> {
    /// Stored as a [`Box`] instead of [`Vec`] to guarantee the length can never change
    /// (via push/pop), ensuring it remains perfectly synced with `width * height`.
    ///
    /// We allocated this memory via [`Vec`] initially, then box it so it can no
    /// longer be resized. [`Box`] only contains the wide pointer, the start address of
    /// the memory allocation, plus the length, not the capacity (which [`Vec`] stores).
    pub data: Box<[T]>,

    /// The fixed width (columns) of the 2D grid.
    pub width: ColWidth,

    /// The fixed height (rows) of the 2D grid.
    pub height: RowHeight,
}

impl<T: GetMemSize> GetMemSize for Flat2DArray<T> {
    fn get_mem_size(&self) -> usize {
        // We add the size of the Box wide-pointer and ColWidth/RowHeight.
        let mut total = std::mem::size_of::<Self>();
        // If T::get_mem_size() is a constant (like PixelChar), the compiler
        // will auto-vectorize this into an O(1) multiplication (len * size).
        for item in &self.data {
            total += item.get_mem_size();
        }
        total
    }
}

/// A zero-cost wrapper that exclusively exposes highly-optimized, 1D contiguous memory
/// operations. It purposefully bypasses the 2D abstractions of [`Flat2DArray`].
///
/// Exposing the contiguous 1D slice provides significant performance gains for consumers
/// (like [`OfsBuf`]) compared to using nested vectors (like `Vec<Vec<T>>`).
///
/// The actual speed comes from replacing nested [heap] allocations with a single
/// contiguous block of memory. This unlocks the following access patterns:
///
/// - **Fast Diffing**
///   - Diffing two buffers is done by zipping their contiguous slices chunked by row
///     width: [`.chunks_exact(width).zip(other.chunks_exact(width))`]. This explicitly
///     maintains 2D coordinates without expensive division (`/`) or modulo (`%`) math.
///     Because it operates on flat contiguous slices, [LLVM] will automatically vectorize
///     the inner loop with [SIMD] instructions, massively speeding up the terminal
///     rendering pipeline.
///   - This is done by [`OfsBuf::diff`] when comparing frames.
/// - **Instant Clearing**
///   - Clearing a buffer doesn't require iterating cell by cell; it can be done via a
///     bulk slice operation (which is exactly how [`Flat1DSimdMut::fill_all`] is
///     implemented under the hood using [`.as_raw_mut_slice().fill()`]).
///   - This is typically done by [`Canvas::clear_canvas`] when processing a clear screen
///     command.
/// - **Cache Locality**
///   - A flat 1D array keeps all elements contiguous in memory. When iterating over
///     cells, the CPU prefetcher works perfectly, pulling data into [L1]/[L2] cache. A
///     nested `Vec<Vec<T>>` scatters row allocations across the [heap], causing frequent
///     cache misses.
///   - This happens in the hot path of the render loop when [`OfsBuf::diff`] iterates
///     over the buffer to diff frames.
/// - **Zero-Allocation Scrolling**
///   - Scrolling the buffer up or down can use [`Flat1DSimdMut::copy_within_rows`] (which
///     maps to highly optimized [`std::ptr::copy`] instructions) instead of shifting
///     pointers or reallocating rows.
///   - This is done in the [`Canvas`] trait by the [`VT-100` output parser].
///
/// # The CPU Cache & Hardware Prefetching
///
/// To understand why [`Flat2DArray`] is so fast, you must understand how data travels
/// from your RAM to the CPU. Data does not travel 1 byte at a time; it is transferred in
/// exact 64-byte blocks (on modern x86 CPUs) called **[Cache Line]s** (you can verify
/// this on x86 CPUs by running `getconf LEVEL1_DCACHE_LINESIZE` on Linux).
///
/// When your code asks for a memory address, the memory controller fetches a 64-byte
/// [Cache Line] from [RAM] and places it into the CPU's **[L1 Cache]**. The CPU Registers
/// (where the actual math happens) are fed exclusively from this [L1 Cache].
///
/// **The [Hardware Prefetcher] (The Conveyor Belt):** Modern CPUs have a dedicated
/// circuit called the [Hardware Prefetcher]. When you iterate over a contiguous slice of
/// memory (like [`Flat2DArray`]), the [Hardware Prefetcher] detects that you are moving
/// forward in a straight line. Before your code even asks for the next block of memory,
/// the [Hardware Prefetcher] secretly reaches out to [RAM], grabs the *next* 64-byte
/// [Cache Line], and places it into the [L1 Cache]. This turns the [L1 Cache] into a
/// high-speed conveyor belt, ensuring the CPU never has to wait for data (zero Cache
/// Misses).
///
/// **The Fragmentation Penalty:** If we had used nested vectors (`Vec<Vec<T>>`) we would
/// incur a hardware penalty. Because vectors are allocated randomly on the [heap], Row 0
/// and Row 1 were not physically next to each other in [RAM]. The [Hardware Prefetcher]
/// could not predict where the next row would be, causing it to guess wrong. The CPU
/// would suffer a massive Cache Miss for every single row.
///
/// ```text
/// ╭─────────────────────────────────────────────────────────────╮
/// │ Vec<Vec<T>> (Scattered Heap Memory):                        │
/// │                                                             │
/// │ [Ptr] -> [Ptr, Ptr, Ptr]                                    │
/// │           ↓    ↓    ↓                                       │
/// │        [Row1] [Row2] [Row3]   <-- Cache Misses!             │
/// ╰─────────────────────────────────────────────────────────────╯
///
/// ╭─────────────────────────────────────────────────────────────╮
/// │ Box<[T]> (Contiguous Memory):                               │
/// │                                                             │
/// │ [Ptr] -> [Row1 | Row2 | Row3] <-- Perfect L1/L2 Cache Hits! │
/// ╰─────────────────────────────────────────────────────────────╯
/// ```
///
/// To quantify this penalty (using an Intel i7-14700 as an example):
/// - **L1 Cache Hit** (~32 KB size): ~1-4 clock cycles
/// - **L2 Cache Hit** (~4 MB size): ~10-15 clock cycles
/// - **L3 Cache Hit** (~33 MB size): ~40-70 clock cycles
/// - **Main RAM Fetch (Cache Miss)**: ~200-300+ clock cycles
///
/// The CPU pipeline would suffer a stall, wasting ~300 clock cycles per row while it
/// waited for the memory controller to fetch data from slow [RAM].
///
/// # How [SIMD] Vectorization Works
///
/// [SIMD] (Single Instruction, Multiple Data) is a CPU feature that allows the processor
/// to perform the same mathematical operation on multiple data points simultaneously.
/// [SIMD] acts as the engine that consumes the [L1 Cache] conveyor belt.
///
/// We do not have to write raw [Assembly] or use [`std::simd`] to trigger this. Instead,
/// we rely on **[LLVM] Auto-vectorization**. Because our 2D vector is now a flat,
/// contiguous 1D array, we can use built-in Rust [`slice`] operations. [LLVM] recognizes
/// these operations and automatically injects [AVX/NEON] [SIMD] instructions. These SIMD
/// instructions operate on specialized, ultra-wide CPU registers that are 256-bit (32
/// bytes) or 512-bit (64 bytes) wide. Because these registers are perfectly aligned with
/// the 64-byte Cache Lines sitting in the [L1 Cache], they can consume and process
/// massive blocks of data in a single clock cycle.
///
/// If the array is larger than the [SIMD] register (which it almost always is), [LLVM]
/// automatically generates a highly optimized loop. It chunks the array into 32-byte or
/// 64-byte blocks, unrolls the loop to keep the CPU pipeline saturated, and generates a
/// "scalar tail" to clean up any leftover bytes at the end that don't divide perfectly
/// into the register size.
///
/// Here are the exact triggers we use to unlock [SIMD]:
/// 1. **[`slice::fill`] ([SIMD] [`memset`])**: Used when clearing the screen (e.g.,
///    [`Flat2DArray::new_empty`]). [LLVM] translates this into ultra-wide [SIMD] Store
///    instructions (acting like a massive [`memset`]). Instead of writing one character
///    at a time, the CPU blasts 32 or 64 bytes of the default character directly into the
///    [L1 Cache] in a single clock cycle!
/// 2. **[`slice::copy_within`] ([SIMD] [`memmove`])**: Used when scrolling by the
///    [virtual terminal tab], e.g., [`Flat1DSimdMut::copy_within_rows`], or when shifting
///    characters left/right during text insertion. This maps directly to highly optimized
///    [`std::ptr::copy`] instructions (which act as a highly optimized SIMD [`memmove`]).
///    It shifts huge contiguous blocks of memory in bulk rather than moving elements one
///    by one. Note: This is actually the *one* scenario where nested vectors beat a flat
///    array, simply because swapping memory pointers (what a nested `Vec` does when you
///    rotate rows) is mathematically faster than physically copying contiguous bytes.
/// 3. **[`Iterator::zip`] ([SIMD] [`memcmp`])**: During the diffing phase, [LLVM]
///    vectorizes the equality checks of two contiguous slices.
///
/// # Deep Dive: The Magic of [SIMD] Diffing
///
/// When comparing two separate terminal frames (like the `self` and `other` buffers in
/// [`OfsBuf::diff`]), the performance wins happen at multiple layers of the CPU:
///
/// ### 1. Multi-Stream Hardware Prefetching
///
/// The CPU's hardware prefetcher isn't limited to tracking just one stream of memory.
/// Modern CPUs (like Intel, AMD, and Apple Silicon) can track multiple independent,
/// sequential memory streams simultaneously (often up to 16 or 32 streams at a time).
///
/// Because diffing iterates linearly through the `self` buffer and linearly through the
/// `other` buffer, the prefetcher quickly recognizes two distinct linear access patterns.
/// It fires off requests to RAM for both streams concurrently, pulling the next 64-byte
/// Cache Lines for both `self` and `other` into the L1 Cache ahead of time.
///
/// ### 2. Dual-Ported L1 Cache
///
/// L1 Caches on modern CPUs are usually "multi-ported." This means the CPU doesn't have
/// to wait to read `self` on cycle 1 and `other` on cycle 2. It can literally fetch data
/// from two completely different memory addresses in the L1 cache in the exact same clock
/// cycle.
///
/// ### 3. [SIMD] Registers and Superscalar Execution
///
/// Once the data is sitting in the L1 cache, the CPU executes the equality check (`if
/// self_row_chunk != other_row_chunk`):
///
/// 1. It issues two [SIMD] load instructions (e.g., pulling 32 bytes of `self` into
///    register [`YMM0`] and 32 bytes of `other` into register [`YMM1`]).
/// 2. Because CPUs are "superscalar" (meaning they can execute multiple instructions per
///    cycle), it loads both registers at nearly the exact same time.
/// 3. It then issues a single [SIMD] compare instruction (like `VPCMPEQB` in x86 [AVX2]).
///
/// While the latency of the entire pipeline (fetch, decode, load, compare) takes several
/// cycles, the CPU overlaps these operations in an assembly line (pipelining). The result
/// is a throughput of one massive 32-byte or 64-byte comparison retiring every single
/// clock cycle.
///
/// So, because the memory access is perfectly linear for both arrays, the hardware
/// prefetcher and L1 Cache perfectly spoon-feed the [SIMD] registers without ever
/// starving the CPU!
///
/// ### 4. Eliding Bounds Checks
///
/// By [`zip()`]ing [`chunks_exact`] together, we prove to the compiler at compile-time
/// that both iterators have the exact same length. [LLVM] completely removes bounds
/// checks from the inner loops, preventing branch mispredictions.
///
/// # Rule of Thumb for 1D vs 2D Memory Iteration
///
/// > 💡 **Note:** The performance assertions for these access patterns are continuously
/// > proven and tracked in the micro-benchmarks located in
/// > `tui/src/core/common/flat_2d_array/benches.rs`.
///
/// ### Rule 1: If you DON'T care about 2D coordinates
///
/// If you just need to clear the whole screen, or find the first occurrence of a
/// character, you don't even need chunks. Just blast through the entire raw 1D slice
/// ([`.iter()`], [`.fill()`], etc.). [LLVM] will aggressively vectorize this into [SIMD]
/// instructions because it's just one massive, uninterrupted block of memory.
///
/// ### Rule 2: If you DO care about 2D coordinates
///
/// If you need to diff rows, track `col_index`, or know when a line ends,
/// [`.chunks_exact(width)`] is the key.
///
/// - **The Math Pipeline Stall Problem (The Slow Way)**. The naive approach uses division
///   (`/`) and modulo (`%`) to calculate coordinates from a 1D index (e.g., `row = index
///   / width`, `col = index % width`). In the computer world, division and modulo are
///   extremely slow mathematical operations. If your width was a fixed number like 8 or
///   16 (powers of 2), the compiler could use a lightning-fast bitshift. But because
///   terminal widths vary at runtime (e.g., 113 columns), the compiler cannot optimize
///   this. The CPU is forced to stop and wait for the math to finish for every single
///   character on the screen, causing massive CPU pipeline stalls.
///
/// - **The Chunks Exact Solution (The Fast Way)**. By slicing the array into rows via
///   [`.chunks_exact(width)`], you walk down the chunks and their items (e.g., `for
///   (row_idx, chunk)` and `for (col_idx, item)`). Under the hood,
///   [`.chunks_exact(width)`] doesn't calculate 2D coordinates from a 1D index using
///   division. Instead, it uses simple **pointer addition**. For example, if the terminal
///   is 113 columns wide, the outer loop just adds 113 to the memory pointer for each
///   row. The inner loop adds 1 to the pointer for each column. Because addition takes 1
///   clock cycle (unlike division, which takes many), 113 becomes just as fast for the
///   CPU to process as a power of 2 like 128. You get logical 2D row boundaries to write
///   easy-to-understand code, while the [LLVM] compiler sees a predictable contiguous
///   memory layout. It will happily unroll that double loop and vectorize the inner slice
///   comparisons into [SIMD] instructions—all while completely bypassing the slow CPU
///   pipeline stalls.
///
/// [`.as_raw_mut_slice().fill()`]: slice::fill
/// [`.chunks_exact(width).zip(other.chunks_exact(width))`]: std::iter::Iterator::zip
/// [`.chunks_exact(width)`]: slice::chunks_exact
/// [`.fill()`]: slice::fill
/// [`.iter()`]: slice::iter
/// [`Canvas::clear_canvas`]:
///     crate::core::ansi::vt_100_pty_output_parser::Canvas::clear_canvas
/// [`Canvas`]: crate::core::ansi::vt_100_pty_output_parser::Canvas
/// [`chunks_exact`]: slice::chunks_exact
/// [`Iterator::zip`]: std::iter::Iterator::zip
/// [`memcmp`]: slice
/// [`memmove`]: std::ptr::copy
/// [`memset`]: slice::fill
/// [`OfsBuf::diff`]: crate::OfsBuf::diff
/// [`OfsBuf`]: crate::OfsBuf
/// [`slice::copy_within`]: slice::copy_within
/// [`slice::fill`]: slice::fill
/// [`slice`]: slice
/// [`std::simd`]: std::simd
/// [`VT-100` output parser]: mod@crate::core::ansi::vt_100_pty_output_parser
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [`YMM0`]:
///     https://en.wikipedia.org/wiki/Advanced_Vector_Extensions#Advanced_Vector_Extensions
/// [`YMM1`]:
///     https://en.wikipedia.org/wiki/Advanced_Vector_Extensions#Advanced_Vector_Extensions
/// [`zip()`]: std::iter::Iterator::zip
/// [`zip`]: std::iter::Iterator::zip
/// [Assembly]: https://en.wikipedia.org/wiki/Assembly_language
/// [AVX/NEON]: https://en.wikipedia.org/wiki/Advanced_Vector_Extensions
/// [AVX2]:
///     https://en.wikipedia.org/wiki/Advanced_Vector_Extensions#Advanced_Vector_Extensions_2
/// [Cache Line]: https://en.wikipedia.org/wiki/CPU_cache#Cache_lines
/// [Cache Lines]: https://en.wikipedia.org/wiki/CPU_cache#Cache_lines
/// [Hardware Prefetcher]: https://en.wikipedia.org/wiki/CPU_cache#Hardware_prefetching
/// [heap]: https://en.wikipedia.org/wiki/Heap_(data_structure)
/// [L1 cache]: https://en.wikipedia.org/wiki/CPU_cache#Levels_of_hierarchy
/// [L1]: https://en.wikipedia.org/wiki/CPU_cache#Levels_of_hierarchy
/// [L2]: https://en.wikipedia.org/wiki/CPU_cache#Levels_of_hierarchy
/// [LLVM]: https://llvm.org/
/// [RAM]: https://en.wikipedia.org/wiki/Random-access_memory
/// [SIMD]: https://en.wikipedia.org/wiki/SIMD
/// [virtual terminal tab]:
///     crate::pty_mux#virtual-terminal-architecture-the-virtual-tab-mental-model
#[derive(Debug)]
pub struct Flat1DSimd<'a, T> {
    pub data: &'a [T],
    pub width: ColWidth,
    pub height: RowHeight,
}

/// A mutable zero-cost wrapper for [SIMD]-optimized bulk operations.
/// See [`Flat1DSimd`] for architectural details.
///
/// [SIMD]: https://en.wikipedia.org/wiki/SIMD
#[derive(Debug)]
pub struct Flat1DSimdMut<'a, T> {
    pub data: &'a mut [T],
    pub width: ColWidth,
    pub height: RowHeight,
}

/// Error returned when trying to access coordinates outside the 2D array bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error, miette::Diagnostic)]
pub enum Flat2DArrayError {
    #[error("2D coordinates are out of bounds")]
    #[diagnostic(
        code(r3bl_tui::core::common::flat_2d_array::out_of_bounds),
        help("Verify that the row and column indices are within the array dimensions.")
    )]
    OutOfBounds,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{col, height, row, width};

    #[test]
    fn test_new_empty() {
        let mut grid = Flat2DArray::new_empty((width(10), height(5)), 0);
        assert_eq!(grid.as_simd().as_raw_slice().len(), 50);

        // Test mutating the underlying data
        assert_eq!(grid.try_get(row(0) + col(0)), Ok(&0));
        grid.as_simd_mut().as_raw_mut_slice()[0] = 99;
        assert_eq!(grid.as_simd().as_raw_slice()[0], 99);
    }
}
