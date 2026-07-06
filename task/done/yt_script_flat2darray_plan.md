<!-- cspell:words darray memcopy Memcopy mktemp -->

## Video Title

**Build High-Performance Flat 2D Arrays in Rust (using SIMD & L1 Cache)**

## Goal Description

Create a YouTube video script and accompanying code (to be saved in
`/home/nazmul/github/rust-scratch/flat2darray/README.md`) demonstrating how to drastically
improve 2D array performance in Rust. The script will walk the viewer through:

1. The naive approach (`Vec<Vec<T>>`, aka `Vec2DArray`) and the scalar methods used to
   manipulate it.
2. The flat 1D array approach (`Box<[T]>`, aka `Flat2DArray`) and why it improves cache
   locality.
3. The iteration problem (CPU pipeline stalls from modulo math).
4. Unlocking SIMD auto-vectorization using `.chunks_exact()` and contiguous memory
   operations.
5. Micro-benchmarks proving the massive performance gains across all grid operations
   (Traversal, Diffing, Copying, Clearing).

The resulting project will be an MVP version that mimics the exact setup used in
`r3bl-open-core` (using the nightly `test` crate for benchmarks) to serve as a pedagogical
tool. We will implement all operations for both `Vec2DArray` and `Flat2DArray` so they can
be benchmarked head-to-head.

## Proposed Changes

We will create a new directory and initialize a Cargo project with the necessary script
and code.

### `rust-scratch/flat2darray/README.md`

This file will contain the actual YouTube script, formatted with clear speaking cues and
code snippets to display on screen. It will re-use ASCII diagrams from the `Flat2DArray`
source to illustrate memory layout visually.

#### [NEW] `/home/nazmul/github/rust-scratch/flat2darray/README.md`

The script will follow this outline:

**1. The Hook (0:00 - 1:00)**

- Introduce the problem: Building a grid for a Terminal UI (or Image Processing) and
  needing a 2D data structure to represent the screen.
- Show the naive approach: The `Vec2DArray` struct containing `Vec<Vec<T>>`.
- Define the standard methods required to manipulate a 2D grid, grounding each in a
  real-world Terminal UI use case:
    - **Iterate**: Traversal via nested loops. (Use Case: Rendering the screen
      cell-by-cell to the terminal).
    - **Diffing**: Comparing two grids to find changed cells. (Use Case: Comparing the old
      frame buffer with the new frame buffer to only redraw pixels that changed).
    - **Clearing**: Wiping all cells in the grid with a default value. (Use Case: Handling
      a "clear screen" command).
    - **Scrolling / Copying**: Moving rows up/down. (Use Case: Shifting terminal history
      up when a new line is printed at the bottom).
- Explain the theory of why `Vec<Vec<T>>` is bad for these methods: It requires multiple
  heap allocations scattered in memory, destroying CPU cache locality and forcing the CPU
  to fetch from RAM constantly. We note that we will prove this theory later with
  benchmarks.

**2. The 1D Solution (1:00 - 2:30)**

- Introduce the new structure: The `Flat2DArray` struct, which flattens the 2D grid into a
  single, contiguous 1D array using `Box<[T]>`.
- Use ASCII diagrams to show how `Vec<Vec<T>>` looks in memory vs `Box<[T]>`.
- Explain how this guarantees a single contiguous memory allocation, resulting in perfect
  L1/L2 cache prefetching (Cache Friendliness).
- Show the math for accessing coordinates: `index = row * width + col`.

**3. The 2D Iteration Trap (2:30 - 4:00)**

- Introduce the "Math Pipeline Stall Problem". What if we need to iterate over the whole
  grid but _still need_ to know our `(row, col)` coordinates?
- Show the naive 1D iteration implementation for `Flat2DArray`:
    ```rust
    for (index, item) in data.iter().enumerate() {
        let row = index / width;
        let col = index % width;
    }
    ```
- Explain the trap: Division (`/`) and Modulo (`%`) are computationally expensive.
- Clarify the "Power of Two" Compiler Trick: If our grid `width` was a guaranteed
  compile-time constant **and** a perfect power of two (like 128), the compiler would
  cleverly optimize the math into lightning-fast bitshifts, regardless of what the `index`
  variable is at runtime:
    ```rust
    // Compiler optimization if width is a hardcoded power of two (e.g. 128 = 2^7):
    let row = index >> 7;   // Fast bitshift (replaces index / 128)
    let col = index & 127;  // Fast bitwise AND (replaces index % 128)
    ```
- Explain why this fails for TUIs: In a Terminal UI, the width is almost never a power of
  two, and more importantly, it is a **runtime variable** (e.g., a user resizes their
  window to 113 columns). Because the compiler doesn't know this number ahead of time at
  compile time, it cannot use the bitshift trick. It is forced to emit actual, slow
  division instructions to the CPU for every single pixel, causing significant pipeline
  stalls.

**4. Unlocking SIMD & Raw Memory Operations (4:00 - 6:30)**

- Introduce the "Two Rules of Thumb" for 1D memory access that replace scalar loops:
    - **Rule 1: If you DON'T care about 2D coordinates** (e.g., clearing the screen or scrolling memory).
        - **Instant Clearing:** You don't need chunks. Just blast through the entire raw 1D slice using `.fill()`. Explain that LLVM aggressively auto-vectorizes this into SIMD instructions, loading 16, 32, or 64 bytes at once into specialized AVX/NEON hardware registers to clear them in a single clock cycle.
        - **Zero-Allocation Scrolling:** Show how moving rows up/down can be done via
          `.copy_within()`, which maps directly to highly optimized `std::ptr::copy`
          (memmove) instructions.
    - **Rule 2: If you DO care about 2D coordinates** (e.g., rendering or diffing rows).
        - **Fast Traversal:** Reveal `.chunks_exact(width)` as the silver bullet. Explain
          that under the hood, it uses pure **pointer addition** instead of division,
          completely bypassing the Math Pipeline Stall.
        - **Fast Diffing:** Show how diffing becomes simple and incredibly fast:
          `.chunks_exact(width).zip(other.chunks_exact(width))`.

**5. Proving it with Benchmarks (6:30 - 8:00)**

- Note that we have now fully implemented both `Vec2DArray` and `Flat2DArray` with all 4
  methods.
- Show the benchmark code running on screen.
- **The Variance Problem (Crucial Point):** Before diving into average execution times,
  point out the "Margin of Error" column in the benchmark results. Highlight how
  `Vec2DArray` suffers from massive variance (e.g., ± 98% swings) because its performance
  relies entirely on lucky CPU cache placement for scattered heap allocations. Contrast
  this with `Flat2DArray`, which guarantees perfectly consistent, flatline frame times.
  Explain that in a UI, eliminating these micro-stutters is just as important as raw
  speed!
- Walk through the results of the 5 benchmark groups to validate our theories:
    1. **Clear Screen (Group 1)**: Proving that a pure 1D `.fill()` is consistently faster
       than a scalar row-by-row clear loop (1.3x speedup).
    2. **Scroll Screen (Group 2)**: Proving `.copy_within()` is highly optimized for
       moving memory without allocations (1.0x speedup, as both methods are bottlenecked
       by raw RAM bandwidth, but the flat array completely avoids heap allocations).
    3. **Read Screen / Compositing (Group 3)**: Explain **"The Rendering Problem"**. Groups 1 and 2 test *writing* memory, but Group 3 tests *reading* it. Proving that linearly streaming flat memory into the L1 cache for the compositor completely destroys nested vector heap-chasing, which constantly causes CPU prefetcher cache misses (1.6x speedup).
    4. **Memory Overhead (Group 4)**: Proving that calculating the size of `Vec<Vec<T>>`
       has massive pointer-chasing overhead, while `Box<[T]>` is near-instantaneous (a
       whopping 60.0x speedup!).
    5. **The 3-Step Performance Staircase (Group 5 - 2D Traversal)**: This is the ultimate
       proof of the Math Pipeline Stall theory. We benchmark 3 traversal methods
       head-to-head (proving a 1.4x speedup):
        1. `Vec2DArray` (Scalar): Slowest (suffers from Cache Misses + Modulo Math).
        2. `Flat2DArray` (Scalar): Fast (fixes Cache Misses, but still suffers from Modulo
           Math pipeline stalls).
        3. `Flat2DArray` (SIMD `.chunks_exact`): Fastest (fixes Cache Misses AND bypasses
           Modulo Math completely via pure pointer addition).
- Conclude the video with the final takeaway on performance design, displaying this exact
  diagram on screen to summarize the core thesis of the video:
    ```text
      1.  Vec2DArray  (Scalar) - Slowest (suffers from Cache Misses and Modulo Math)
      2.  Flat2DArray  (Scalar) - Fast (Cache Hits! But still suffers from Modulo Math pipeline stalls)
      3.  Flat2DArray  (SIMD / chunks_exact) - Fastest (Cache Hits and pure pointer addition!)
    ```

### `rust-scratch/flat2darray/` Code

We will generate the project structure using standard Cargo commands in a temporary
directory to avoid manually writing boilerplate:

```fish
cd (mktemp -d)
cargo new --lib flat2darray
cd flat2darray
# Set nightly toolchain for benchmarking.
rustup override set nightly
```

_Note: Because we aren't adding external dependencies, `Cargo.toml` is handled
automatically._

#### [MODIFY] `flat2darray/src/lib.rs`

Since `cargo new` generates a default `src/lib.rs`, we will simply modify it to enable the
nightly benchmarking feature and expose our new modules:

```rust
// Enable unstable nightly features for benchmarking.
#![feature(test)]

// Attach the modules.
pub mod vec_2d_array;
pub mod flat_2d_array;

// Expose the benchmarking module (only compiled when `cargo bench` is run).
#[cfg(test)]
mod benches;
```

#### [NEW] `flat2darray/src/vec_2d_array.rs`

Will contain the `Vec2DArray` data structure, the implementations of its 4 scalar methods
**(plus a `.get_mem_size()` method to calculate heap allocation size)**, and a
`#[cfg(test)]` inner module with inline unit tests proving the logic works correctly
before we benchmark it.

#### [NEW] `flat2darray/src/flat_2d_array.rs`

Will contain the `Flat2DArray` data structure, the implementations of its 4 SIMD-optimized
methods (as well as the scalar methods so we can benchmark them against each other!), **a
`.get_mem_size()` method**, and a `#[cfg(test)]` module with inline unit tests proving the
logic works correctly before we benchmark it.

#### [NEW] `flat2darray/src/benches.rs`

Will import both modules and contain the `#[bench]` tests for all 5 benchmarking groups
exactly as they appear in `r3bl-open-core`.

## Verification Plan

### Manual Verification

1. Review the generated script for flow, tone, and clarity, ensuring the ASCII diagrams
   are well-placed.
2. Run `cargo +nightly bench` in the newly created `flat2darray` project directory to
   ensure the benchmarks compile and successfully prove the performance gains across all 5
   groups before recording.

### Mandatory manual review

- [x] `/home/nazmul/github/roc/task/yt_script_flat2darray_plan.md`
- [ ] `/home/nazmul/github/rust-scratch/flat2darray/README.md`
