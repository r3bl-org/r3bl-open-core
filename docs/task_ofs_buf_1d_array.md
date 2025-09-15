# Task: Optimize OffscreenBuffer with 1D Array Implementation

## Executive Summary

Convert the current 2D `Vec<Vec<PixelChar>>` structure in `OffscreenBuffer` to a flat 1D `Box<[PixelChar]>` array to improve memory efficiency, cache locality, and performance. The new implementation will leverage the existing type-safe `Pos`, `RowIndex`, and `ColIndex` types for an ergonomic and safe API.

## Background and Motivation

### Current Implementation Problems
1. **Memory Overhead**: Each row is a separate `Vec`, requiring:
   - Individual heap allocations (rows + 1 total allocations)
   - Vec metadata (24 bytes per Vec on 64-bit systems: ptr, capacity, length)
   - Potential memory fragmentation

2. **Cache Performance Issues**:
   - Rows may not be contiguous in memory
   - Pointer chasing when accessing elements (dereference row Vec, then column)
   - Poor spatial locality for operations that traverse the buffer

3. **Allocation Cost**: Creating a new buffer requires multiple allocations

### Benefits of 1D Array
1. **Single Allocation**: One contiguous memory block
2. **Better Cache Locality**: Sequential memory access patterns
3. **Reduced Memory Overhead**: Single Vec metadata instead of rows+1
4. **SIMD Optimization Potential**: Contiguous memory enables vectorized operations
5. **Predictable Performance**: Constant-time indexing with simple arithmetic
6. **Zero-Allocation Scrolling**: Use `copy_within()` for in-place shifts
7. **Efficient Row Operations**: Direct slice manipulation without cloning

### SIMD and Performance Background

#### What is SIMD?
**Single Instruction, Multiple Data (SIMD)** is a class of parallel computing that allows a single CPU instruction to operate on multiple data points simultaneously. Modern CPUs include SIMD instruction sets like:
- **x86_64**: SSE, AVX, AVX-512 (process 4, 8, or 16 elements per instruction)
- **ARM**: NEON (process 4-16 elements per instruction)
- **RISC-V**: Vector extensions

#### Why 1D Arrays Enable SIMD
The key to SIMD optimization is **contiguous memory layout**:

##### 2D Array Problem (Vec<Vec<PixelChar>>):
```
Memory Layout: [Row0_ptr] -> [pixel, pixel, pixel, pixel]
               [Row1_ptr] -> [pixel, pixel, pixel, pixel] (different allocation)
               [Row2_ptr] -> [pixel, pixel, pixel, pixel] (different allocation)
```
- **Fragmented Memory**: Each row is a separate allocation
- **Pointer Chasing**: CPU must follow pointers to access data
- **Cache Misses**: Rows may not be adjacent in memory
- **SIMD Blocked**: Cannot vectorize across row boundaries

##### 1D Array Solution (Box<[PixelChar]>):
```
Memory Layout: [pixel][pixel][pixel][pixel][pixel][pixel][pixel][pixel]...
                Row 0 ←──────────────────→ Row 1 ←──────────────────→
```
- **Contiguous Memory**: All pixels in one allocation
- **Sequential Access**: Natural memory access patterns
- **Cache Friendly**: Excellent spatial locality
- **SIMD Ready**: Operations can process multiple pixels per instruction

#### SIMD Operations Enabled by Flat Buffer

##### Automatic Vectorization:
- **Buffer Clear**: `slice::fill()` → vectorized store instructions
- **Memory Copy**: `copy_within()` → vectorized load/store pairs
- **Comparison**: `slice == slice` → vectorized comparison instructions
- **Iteration**: Compiler auto-vectorization of loops over contiguous data

##### Performance Impact Examples:
```rust
// This operation on 1920 pixels (80×24 buffer):
buffer.fill(PixelChar::Spacer);

// 2D Array: 24 separate fill operations (no vectorization)
// 1D Array: Single vectorized operation processing 4-16 pixels per instruction
// Result: 4-16x performance improvement
```

##### Why Standard Library Operations "Just Work":
Rust's standard library is designed with SIMD in mind:
- `slice::fill()` automatically uses SIMD when profitable
- `slice::copy_from_slice()` leverages optimized `memcpy` (often SIMD)
- Iterators enable compiler auto-vectorization
- **No explicit SIMD programming required!**

## Current Architecture Analysis

### Key Files and Structures
```
tui/src/tui/terminal_lib_backends/offscreen_buffer/
└── ofs_buf_core.rs            # Main OffscreenBuffer struct
```

### Current Type Hierarchy
```rust
OffscreenBuffer {
    buffer: PixelCharLines {
        lines: InlineVec<PixelCharLine> {  // SmallVec<[T; INLINE_VEC_SIZE]>
            pixel_chars: InlineVec<PixelChar>  // SmallVec optimization
        }
    }
}
```

### Access Patterns in Current Code
1. **Direct indexing**: `buffer[row][col]`
2. **Safe access**: `buffer.get(row)?.get(col)`
3. **Line operations**: `buffer[row].clone()`, `buffer[row].fill()`
4. **Iteration**: `buffer.iter()` then `line.iter()`

## Proposed Solution

### Core Data Structure
```rust
pub struct FlatGridBuffer<T>
where
    T: Copy + Default + PartialEq + Clone + Debug
{
    data: Box<[T]>,          // Box<[T]> since size never changes!
    width: ColWidth,         // Type-safe dimensions
    height: RowHeight,       // Type-safe dimensions
}

// For our specific use case:
// type FlatPixelCharBuffer = FlatGridBuffer<PixelChar>;
```

### Benefits of Generic Design

**Reusability**: The generic `FlatGridBuffer<T>` can be used for various grid-like data structures:
- `FlatGridBuffer<PixelChar>` - Terminal buffer (our primary use case)
- `FlatGridBuffer<Color>` - Color maps or background layers
- `FlatGridBuffer<u8>` - Masks, alpha channels, or simple grids
- `FlatGridBuffer<bool>` - Boolean grids for selections or flags
- `FlatGridBuffer<char>` - Simple text grids for testing

**Testing Benefits**: Generic design enables easier testing with simple types:
```rust
// Easy to test with simple types
let mut test_grid = FlatGridBuffer::<u8>::new_empty(height(3) + width(3));
test_grid.fill(42);

// Much simpler than setting up PixelChar instances for every test
```

**Performance Consistency**: All specialized types benefit from the same SIMD optimizations and memory layout advantages.

**Type Safety**: The trait bounds `T: Copy + Default + PartialEq + Clone + Debug` ensure that all operations work correctly and efficiently with any compatible type.

**Why Box<[T]> instead of Vec<T>?**

Both `Box<[PixelChar]>` and `Vec<PixelChar>` provide **identical SIMD performance** since they both:
- Use the same global allocator with proper alignment
- Store data as contiguous arrays in memory
- Support the same slice operations (`fill()`, `copy_within()`, etc.)

However, `Box<[PixelChar]>` is better for our use case because:

**Memory Efficiency**:
```rust
// Vec<T> overhead: 24 bytes on 64-bit systems
struct Vec<T> { ptr: *mut T, capacity: usize, len: usize }

// Box<[T]> overhead: 16 bytes on 64-bit systems
struct Box<[T]> { ptr: *mut T, len: usize }
// Saves 8 bytes per buffer by eliminating unused capacity field
```

**API Safety**:
- Buffer size never changes after creation (resizing creates new buffer)
- Prevents accidental `push()`, `resize()`, or `reserve()` operations
- Clearer API semantics - fixed-size buffer intent is explicit
- No risk of unexpected reallocations

**When to Use Vec<PixelChar> Instead**:
- Dynamic buffer resizing needed (not our case)
- Building buffers incrementally
- Temporary work buffers with varying sizes

**SIMD Alignment Guarantee**: Both containers get proper SIMD alignment from Rust's allocator based on `align_of::<PixelChar>()` and platform SIMD requirements.

### Flexible, Type-Safe API Design

The new API leverages the existing `Pos`, `RowIndex`, and `ColIndex` types with `impl Into<Pos>` for maximum flexibility:

```rust
impl<T> FlatGridBuffer<T>
where
    T: Copy + Default + PartialEq + Clone + Debug
{
    /// Get an element at any position-like input
    /// Examples:
    /// - buffer.get(row(5) + col(10))
    /// - buffer.get((row(5), col(10)))
    /// - buffer.get(my_pos)
    pub fn get(&self, arg_pos: impl Into<Pos>) -> Option<&T> {
        let pos = arg_pos.into();
        // Use proper bounds checking from the type system
        if !pos.row_index.overflows(self.height) && !pos.col_index.overflows(self.width) {
            let idx = pos.row_index.as_usize() * self.width.as_usize()
                    + pos.col_index.as_usize();
            Some(&self.data[idx])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, arg_pos: impl Into<Pos>) -> Option<&mut T> {
        let pos = arg_pos.into();
        // Similar implementation
    }

    pub fn set(&mut self, arg_pos: impl Into<Pos>, element: T) -> bool {
        let pos = arg_pos.into();
        if let Some(p) = self.get_mut_internal(pos) {
            *p = element;
            true
        } else {
            false
        }
    }
}
```

### Index Trait Implementation

Support the same flexible input patterns with the Index trait:

```rust
impl<T, P> Index<P> for FlatGridBuffer<T>
where
    T: Copy + Default + PartialEq + Clone + Debug,
    P: Into<Pos>
{
    type Output = T;

    fn index(&self, arg_pos: P) -> &Self::Output {
        let pos = arg_pos.into();
        let idx = pos.row_index.as_usize() * self.width.as_usize()
                + pos.col_index.as_usize();
        &self.data[idx]
    }
}

// All of these work:
let pixel = buffer[row(5) + col(10)];  // Addition syntax
let pixel = buffer[(row(5), col(10))]; // Tuple syntax
let pixel = buffer[my_pos];            // Direct Pos
```

## Implementation Tasks

**Important**: Throughout the implementation, use the type-safe bounds checking utilities from `tui/src/core/units/bounds_check.rs`:
- Use `IndexMarker::overflows()` instead of raw `<` comparisons
- Use `LengthMarker::is_overflowed_by()` for inverse checks
- Use `LengthMarker::clamp_to()` for clamping operations
- Leverage `convert_to_index()` and `convert_to_length()` for type conversions

### Phase 1: Core Infrastructure (Priority: High)

#### Task 1.1: Create FlatGridBuffer Module
**File**: `tui/src/core/1d_buffer/flat_grid_buffer.rs`

**Requirements**:
- [ ] Define `FlatGridBuffer<T>` struct with fields: `data: Box<[T]>`, `width: ColWidth`, `height: RowHeight`
- [ ] Add trait bounds: `T: Copy + Default + PartialEq + Clone + Debug`
- [ ] Implement `new_empty(arg_size: impl Into<Size>) -> Self` constructor
  - Create with `vec![T::default(); capacity]`
  - Convert to Box<[T]> with `.into_boxed_slice()`
- [ ] Add private helper: `fn index_of(&self, pos: Pos) -> usize`
- [ ] Add private helper: `fn get_mut_internal(&mut self, pos: Pos) -> Option<&mut T>`
- [ ] Implement `GetMemSize` trait
- [ ] Implement `Debug` trait

**Example Implementation**:
```rust
impl<T> FlatGridBuffer<T>
where
    T: Copy + Default + PartialEq + Clone + Debug
{
    pub fn new_empty(arg_size: impl Into<Size>) -> Self {
        let size = arg_size.into();
        let capacity = size.area();

        // Create as Vec, then convert to Box<[T]> for fixed-size buffer
        // Note: Vec allocations are automatically aligned for SIMD operations
        // The allocator ensures proper alignment for the element type
        let data = vec![T::default(); capacity].into_boxed_slice();

        Self {
            data,
            width: size.col_width,
            height: size.row_height,
        }
    }

    fn index_of(&self, pos: Pos) -> usize {
        pos.row_index.as_usize() * self.width.as_usize()
            + pos.col_index.as_usize()
    }

    fn get_mut_internal(&mut self, pos: Pos) -> Option<&mut T> {
        // Use IndexMarker::overflows for type-safe bounds checking
        if !pos.row_index.overflows(self.height) && !pos.col_index.overflows(self.width) {
            let idx = self.index_of(pos);
            Some(&mut self.data[idx])
        } else {
            None
        }
    }
}
```

#### Task 1.2: Implement Core Access Methods
**Requirements**:
- [ ] `pub fn get(&self, arg_pos: impl Into<Pos>) -> Option<&T>`
- [ ] `pub fn get_mut(&mut self, arg_pos: impl Into<Pos>) -> Option<&mut T>`
- [ ] `pub fn set(&mut self, arg_pos: impl Into<Pos>, element: T) -> bool`
- [ ] `pub fn get_unchecked(&self, arg_pos: impl Into<Pos>) -> &T` (for internal use)
- [ ] Consider adding `get_clamped()` methods that use `LengthMarker::clamp_to()` for safe access

#### Task 1.3: Implement Row-Level Access Methods
**Requirements**:
- [ ] `pub fn row(&self, arg_row: impl Into<RowIndex>) -> Option<&[T]>`
- [ ] `pub fn row_mut(&mut self, arg_row: impl Into<RowIndex>) -> Option<&mut [T]>`
- [ ] `pub fn copy_row(&mut self, arg_from: impl Into<RowIndex>, arg_to: impl Into<RowIndex>) -> bool`
- [ ] `pub fn swap_rows(&mut self, arg_row1: impl Into<RowIndex>, arg_row2: impl Into<RowIndex>) -> bool`
- [ ] `pub fn shift_rows_up(&mut self, range: Range<RowIndex>, shift_by: Length) -> bool`
- [ ] `pub fn shift_rows_down(&mut self, range: Range<RowIndex>, shift_by: Length) -> bool`

**Example**:
```rust
impl<T> FlatGridBuffer<T>
where
    T: Copy + Default + PartialEq + Clone + Debug
{
    pub fn row(&self, arg_row: impl Into<RowIndex>) -> Option<&[T]> {
        let row = arg_row.into();
        // Use proper bounds checking
        if !row.overflows(self.height) {
            let start = row.as_usize() * self.width.as_usize();
            let end = start + self.width.as_usize();
            Some(&self.data[start..end])
        } else {
            None
        }
    }

    pub fn swap_rows(&mut self, arg_row1: impl Into<RowIndex>, arg_row2: impl Into<RowIndex>) -> bool {
    let row1 = arg_row1.into();
    let row2 = arg_row2.into();
    // Use proper bounds checking
    if !row1.overflows(self.height) && !row2.overflows(self.height) && row1 != row2 {
        let width = self.width.as_usize();
        let start1 = row1.as_usize() * width;
        let start2 = row2.as_usize() * width;

        // Most efficient: use swap_with_slice for bulk swap
        let (first, second) = if start1 < start2 {
            let (left, right) = self.data.split_at_mut(start2);
            (&mut left[start1..start1 + width], &mut right[0..width])
        } else {
            let (left, right) = self.data.split_at_mut(start1);
            (&mut right[0..width], &mut left[start2..start2 + width])
        };
        first.swap_with_slice(second);
        true
    } else {
        false
    }
    }
}
```

### Phase 2: Standard Trait Implementations (Priority: High)

#### Task 2.1: Implement Index Traits
**Requirements**:
- [ ] `impl<T, P> Index<P> for FlatGridBuffer<T> where T: Copy + Default + PartialEq + Clone + Debug, P: Into<Pos>`
- [ ] `impl<T, P> IndexMut<P> for FlatGridBuffer<T> where T: Copy + Default + PartialEq + Clone + Debug, P: Into<Pos>`
- [ ] `impl<T> Index<Range<RowIndex>> for FlatGridBuffer<T>` (for row ranges)
- [ ] `impl<T> IndexMut<Range<RowIndex>> for FlatGridBuffer<T>`

#### Task 2.2: Implement Iterator Support
**Requirements**:
- [ ] `pub fn rows(&self) -> impl Iterator<Item = &[T]>`
- [ ] `pub fn rows_mut(&mut self) -> impl Iterator<Item = &mut [T]>`
- [ ] `pub fn iter(&self) -> impl Iterator<Item = &T>`
- [ ] `pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T>`
- [ ] `pub fn iter_positions(&self) -> impl Iterator<Item = (Pos, &T)>`

**Example**:
```rust
impl<T> FlatGridBuffer<T>
where
    T: Copy + Default + PartialEq + Clone + Debug
{
    pub fn rows(&self) -> impl Iterator<Item = &[T]> {
        self.data.chunks_exact(self.width.as_usize())
    }

    pub fn iter_positions(&self) -> impl Iterator<Item = (Pos, &T)> {
        self.data.iter().enumerate().map(move |(idx, element)| {
            let row = row(idx / self.width.as_usize());
            let col = col(idx % self.width.as_usize());
            (row + col, element)
        })
    }
}
```

### Phase 3: Advanced Operations (Priority: Medium)

#### Task 3.1: Region Operations
**Requirements**:
- [ ] `pub fn get_region(&self, arg_start: impl Into<Pos>, arg_size: impl Into<Size>) -> Option<Vec<T>>`
- [ ] `pub fn fill_region(&mut self, arg_start: impl Into<Pos>, arg_size: impl Into<Size>, element: T) -> bool`
- [ ] `pub fn copy_region(&self, arg_start: impl Into<Pos>, arg_size: impl Into<Size>) -> Option<FlatGridBuffer<T>>`

**Example**:
```rust
impl<T> FlatGridBuffer<T>
where
    T: Copy + Default + PartialEq + Clone + Debug
{
    pub fn fill_region(&mut self, arg_start: impl Into<Pos>, arg_size: impl Into<Size>, element: T) -> bool {
        let start_pos = arg_start.into();
        let size = arg_size.into();

        // Validate bounds using type-safe overflow checking
        let end_row = start_pos.row_index + size.row_height.convert_to_index();
        let end_col = start_pos.col_index + size.col_width.convert_to_index();

        if end_row.overflows(self.height) || end_col.overflows(self.width) {
            return false;
        }

        // Fill each row in the region
        for row_offset in 0..size.row_height.as_usize() {
            let row_idx = start_pos.row_index + row(row_offset);
            let start_idx = self.index_of(row_idx + start_pos.col_index);
            let end_idx = start_idx + size.col_width.as_usize();
            self.data[start_idx..end_idx].fill(element);
        }
        true
    }
}
```

#### Task 3.2: Clear and Fill Operations
**Requirements**:
- [ ] `pub fn clear(&mut self)` - fill entire buffer with `T::default()`
- [ ] `pub fn clear_row(&mut self, arg_row: impl Into<RowIndex>)`
- [ ] `pub fn clear_rows(&mut self, arg_range: Range<RowIndex>)`
- [ ] `pub fn fill(&mut self, element: T)`

**SIMD-Optimized Implementation Examples**:
```rust
impl<T> FlatGridBuffer<T>
where
    T: Copy + Default + PartialEq + Clone + Debug
{
    pub fn clear(&mut self) {
        // slice::fill automatically uses SIMD when available
        self.data.fill(T::default());
    }

    pub fn clear_row(&mut self, arg_row: impl Into<RowIndex>) -> bool {
        let row = arg_row.into();
        if let Some(row_slice) = self.row_mut(row) {
            // SIMD-optimized fill for single row
            row_slice.fill(T::default());
            true
        } else {
            false
        }
    }

    pub fn clear_rows(&mut self, arg_range: Range<RowIndex>) -> bool {
        // Validate range
        if arg_range.start.overflows(self.height) || arg_range.end.overflows(self.height)
           || arg_range.start >= arg_range.end {
            return false;
        }

        // Calculate slice range and use SIMD-optimized fill
        let start_idx = arg_range.start.as_usize() * self.width.as_usize();
        let end_idx = arg_range.end.as_usize() * self.width.as_usize();
        self.data[start_idx..end_idx].fill(T::default());
        true
    }

    pub fn fill(&mut self, element: T) {
        // Most efficient: entire buffer fill with SIMD
        self.data.fill(element);
    }

    /// Efficient comparison using SIMD-optimized slice comparison
    pub fn equals(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.data == other.data  // slice comparison can use SIMD
    }
}
```

### Phase 4: Migration Operations (Priority: High)

#### Task 4.1: Line-Level Operations Migration
**File**: `tui/src/tui/terminal_lib_backends/offscreen_buffer/ofs_buf_line_level_ops.rs`

**Migration patterns**:
```rust
// SCROLLING/SHIFTING - Much more efficient with flat buffer!

// Old: Inefficient - allocates new Vec for each row copy
for row_idx in start_idx..end_idx.saturating_sub(1) {
    let next_line = self.buffer[row_idx + 1].clone();  // Allocates!
    self.buffer[row_idx] = next_line;
}

// New: Efficient - in-place memory operations, zero allocations
pub fn shift_lines_up(&mut self, range: Range<RowIndex>, shift_by: Length) -> bool {
    let width = self.width.as_usize();
    let start_row = range.start.as_usize();
    let end_row = range.end.as_usize();

    // Use copy_within for efficient in-place shifting
    let src_start = (start_row + 1) * width;
    let src_end = end_row * width;
    let dest_start = start_row * width;

    // Note: With Box<[T]>, we may need to use: (&mut *self.data).copy_within(...)
    self.data.copy_within(src_start..src_end, dest_start);

    // Clear the last row
    let last_row_start = (end_row - 1) * width;
    self.data[last_row_start..last_row_start + width].fill(PixelChar::Spacer);
}

// SWAPPING ROWS
// Old: Clone both rows
let temp = self.buffer[row1].clone();
self.buffer[row1] = self.buffer[row2].clone();
self.buffer[row2] = temp;

// New: In-place swap, no allocations
self.swap_rows(row1, row2);  // Uses slice::swap internally

// CLEARING
// Old: self.buffer[row].fill(PixelChar::Spacer)
// New: self.clear_row(row)  // Same efficiency, cleaner API

// SETTING AN ENTIRE ROW
// Old: self.buffer[row] = new_line
// New: if let Some(row_slice) = self.row_mut(row) {
//          row_slice.copy_from_slice(&new_line);  // Efficient memcpy
//      }
```

#### Task 4.2: Character Operations Migration
**File**: `tui/src/tui/terminal_lib_backends/offscreen_buffer/ofs_buf_char_ops.rs`

**Migration patterns**:
```rust
// Old: self.buffer.get(row_idx)?.get(col_idx)
// New: self.buffer.get(row(row_idx) + col(col_idx))

// Old: self.buffer[row][col] = pixel
// New: self.buffer[row(r) + col(c)] = pixel
//  Or: self.buffer.set(row(r) + col(c), pixel)
```

#### Task 4.3: Update OffscreenBuffer
**File**: `tui/src/tui/terminal_lib_backends/offscreen_buffer/ofs_buf_core.rs`

**Requirements**:
- [ ] Replace `buffer: PixelCharLines` with `buffer: FlatGridBuffer<PixelChar>`
- [ ] Update `Deref` and `DerefMut` implementations to return `&FlatGridBuffer<PixelChar>`
- [ ] Update `new_empty()` constructor
- [ ] Update `clear()` method
- [ ] Update `diff()` method to work with flat buffer

**Optimized diff implementation**:
```rust
// Old: Nested loops with poor cache locality
pub fn diff(&self, other: &Self) -> Option<PixelCharDiffChunks> {
    for (row_idx, (self_row, other_row)) in self.buffer.iter().zip(other.buffer.iter()).enumerate() {
        for (col_idx, (self_pixel, other_pixel)) in self_row.iter().zip(other_row.iter()).enumerate() {
            // Compare and collect differences...
        }
    }
}

// New: Single pass through contiguous memory - excellent cache locality!
pub fn diff(&self, other: &Self) -> Option<PixelCharDiffChunks> {
    if self.window_size != other.window_size {
        return None;
    }

    let mut diffs = Vec::new();
    let width = self.width.as_usize();

    // Single linear pass through memory with excellent cache locality
    // The zip().enumerate() pattern allows for potential auto-vectorization
    // of the comparison operation by the compiler
    for (idx, (self_pixel, other_pixel)) in self.data.iter().zip(other.data.iter()).enumerate() {
        if self_pixel != other_pixel {
            let row = row(idx / width);
            let col = col(idx % width);
            diffs.push((row + col, *other_pixel));
        }
    }

    // Alternative: For even better performance on large buffers with few differences,
    // consider chunked comparison to leverage SIMD more effectively:
    // self.data.chunks_exact(8).zip(other.data.chunks_exact(8))
    //     .enumerate()
    //     .filter(|(_, (chunk1, chunk2))| chunk1 != chunk2)
    //     .flat_map(|(chunk_idx, (chunk1, chunk2))| /* find specific differences */)

    Some(PixelCharDiffChunks::from(diffs))
}
```

### Phase 5: Testing (Priority: Critical)

#### Task 5.1: Unit Tests
**File**: `tui/src/core/1d_buffer/flat_grid_buffer.rs` (test module)

**Test Coverage**:
- [ ] Index calculation correctness
- [ ] Boundary conditions: (0,0), (max_row, max_col)
- [ ] Empty buffer handling (0x0 size)
- [ ] Large buffer stress test (1000x1000)
- [ ] All access methods with valid/invalid inputs
- [ ] Region operations with various sizes and positions
- [ ] Row operations (copy, swap, clear)

**Example Test**:
```rust
#[test]
fn test_flexible_api() {
    let mut buffer = FlatGridBuffer::<PixelChar>::new_empty(height(10) + width(20));
    let test_pixel = PixelChar::PlainText {
        display_char: 'X',
        style: TuiStyle::default()
    };

    // Test all input styles work
    buffer.set(row(5) + col(10), test_pixel);
    assert_eq!(buffer[row(5) + col(10)], test_pixel);
    assert_eq!(buffer[(row(5), col(10))], test_pixel);

    let pos = row(5) + col(10);
    assert_eq!(buffer[pos], test_pixel);
    assert_eq!(buffer.get(pos), Some(&test_pixel));
}

#[test]
fn test_generic_with_simple_type() {
    let mut buffer = FlatGridBuffer::<u8>::new_empty(height(5) + width(5));

    // Test with simple numeric type
    buffer.set(row(2) + col(3), 42);
    assert_eq!(buffer[row(2) + col(3)], 42);

    buffer.fill(255);
    assert_eq!(buffer[row(0) + col(0)], 255);
    assert_eq!(buffer[row(4) + col(4)], 255);
}
```

#### Task 5.2: Integration Tests
**Requirements**:
- [ ] Test with ANSI parser operations
- [ ] Test with render pipeline
- [ ] Test scrolling and shifting operations
- [ ] Test diff generation and application
- [ ] Ensure all existing OffscreenBuffer tests pass

### Phase 6: Documentation (Priority: Medium)

#### Task 6.1: API Documentation
**Requirements**:
- [ ] Document all public methods with examples
- [ ] Explain the flexible `impl Into<Pos>` pattern
- [ ] Document performance characteristics
- [ ] Add module-level documentation explaining the flat buffer design

**Example Documentation**:
```rust
/// A flat, cache-friendly buffer for grid-like data structures.
///
/// This generic implementation uses a single contiguous memory block for better cache
/// locality compared to a Vec of Vecs. All operations use type-safe
/// indices to prevent row/column confusion.
///
/// # Type Parameters
///
/// * `T` - The element type stored in the grid. Must implement:
///   `Copy + Default + PartialEq + Clone + Debug`
///
/// # Examples
///
/// ## With PixelChar for terminal applications:
/// ```
/// let mut buffer = FlatGridBuffer::<PixelChar>::new_empty(height(24) + width(80));
///
/// // Multiple ways to write to the same position:
/// buffer.set(row(10) + col(20), PixelChar::Spacer);
/// buffer.set((row(10), col(20)), PixelChar::Spacer);
/// buffer[row(10) + col(20)] = PixelChar::Spacer;  // Index syntax
///
/// // Multiple ways to read from the same position:
/// let pixel1 = buffer.get(row(10) + col(20));     // Returns Option<&PixelChar>
/// let pixel2 = buffer[(row(10), col(20))];         // Panics if out of bounds
/// let pixel3 = &buffer[pos];                       // Where pos is a Pos
///
/// // Row-level operations:
/// if let Some(row_slice) = buffer.row(row(5)) {
///     // row_slice is &[PixelChar]
///     for pixel in row_slice {
///         // Process each pixel in the row
///     }
/// }
///
/// // Efficient scrolling (zero allocations!):
/// buffer.shift_rows_up(row(0)..row(24), len(1));
///
/// // Region operations:
/// buffer.fill_region(row(5) + col(10), height(3) + width(20), PixelChar::Spacer);
/// ```
///
/// ## With simple types for other use cases:
/// ```
/// let mut color_grid = FlatGridBuffer::<u32>::new_empty(height(100) + width(100));
/// color_grid.fill(0xFF0000); // Fill with red
///
/// let mut mask = FlatGridBuffer::<bool>::new_empty(height(10) + width(10));
/// mask.clear(); // Fill with false (default for bool)
/// ```
pub struct FlatGridBuffer<T> { /* ... */ }
```

## Migration Strategy

### Step 1: Parallel Development
1. Create `FlatGridBuffer<T>` in new module at `tui/src/core/1d_buffer/`
2. Implement all core functionality with tests
3. Benchmark against current implementation using `FlatGridBuffer<PixelChar>`

### Step 2: Integration
1. Update `OffscreenBuffer` to use `FlatGridBuffer<PixelChar>`
2. Migrate all operations one file at a time
3. Run full test suite after each file migration

### Step 3: Cleanup
1. Remove old `PixelCharLines` and `PixelCharLine` types
2. Remove unnecessary intermediate types
3. Update all documentation

## Success Criteria

1. **Correctness**: All existing tests pass
2. **Performance**:
   - Memory usage reduced by at least 20%
   - Sequential access improved by at least 2x
   - No regression in random access performance
3. **Type Safety**: No usize parameters in public API (except for tests)
4. **Ergonomics**: Clean, flexible API with multiple input styles
5. **Documentation**: Comprehensive docs with examples

## Risk Mitigation

### Potential Issues and Solutions

1. **Index Calculation Errors**
   - Mitigation: Extensive unit tests
   - Use property-based testing for index calculations
   - Debug assertions in development builds

2. **Performance Regression in Specific Operations**
   - Mitigation: Benchmark each operation type
   - Profile with `cargo flamegraph`
   - Keep optimization options open (e.g., unsafe for hot paths if needed)

3. **Migration Complexity**
   - Mitigation: Migrate one module at a time
   - Keep old implementation until new one is fully tested
   - Use version control to track progress

## Implementation Notes

### Index Calculation Formula
```rust
// For position (row, col) in a buffer of width W:
index = row * W + col

// Example for width=80:
// (0, 0) -> 0
// (0, 79) -> 79
// (1, 0) -> 80
// (1, 79) -> 159
```

### Memory Layout
```
Row 0: [0..width)
Row 1: [width..2*width)
Row 2: [2*width..3*width)
...
Row n: [n*width..(n+1)*width)
```

### Key Performance Improvements

#### Scrolling Operations
The flat buffer design transforms scrolling from O(n) allocations to zero allocations:

```rust
// Vec<Vec<>> approach: Each scroll allocates n row Vecs
// Flat buffer: Uses copy_within() - just memmove internally!
```

Performance characteristics:
- **Old**: O(rows × cols) memory allocation per scroll
- **New**: O(1) - no allocations, just memory movement
- **Expected improvement**: 10-50x faster for scrolling operations

#### Memory Usage
For a typical 80×24 terminal buffer:
- **Old**: 25 heap allocations (1 outer + 24 inner Vecs), ~600 bytes Vec overhead
- **New**: 1 heap allocation, 24 bytes Vec overhead
- **Savings**: ~96% reduction in allocation overhead

#### SIMD Acceleration Benefits
The flat buffer design naturally enables SIMD optimizations:

**Automatic Vectorization**:
- **Fill Operations**: `slice::fill()` uses SIMD instructions when available (up to 16x faster for large buffers)
- **Copy Operations**: `copy_within()` and `copy_from_slice()` leverage optimized memcpy with SIMD
- **Comparisons**: Slice equality (`==`) operations can use SIMD for bulk comparisons

**Real-World Performance Impact**:
- **Buffer Clear**: 80×24 buffer clear ~8-16x faster with SIMD (depends on CPU)
- **Scrolling**: Memory movement uses vectorized instructions automatically
- **Diff Calculation**: Contiguous memory enables compiler auto-vectorization

**No Code Complexity**: SIMD benefits come "for free" through standard library operations without explicit SIMD programming.

### Type Safety Benefits
Using `RowIndex`, `ColIndex`, and `Pos` prevents common bugs:
- Cannot swap row and column arguments
- Cannot pass arbitrary integers
- Self-documenting code
- Compile-time safety

### Flexible API Benefits
The `impl Into<Pos>` pattern allows:
- Natural expression syntax: `row(5) + col(10)`
- Multiple input styles for different contexts
- Backward compatibility with existing Pos-based code
- Zero-cost abstraction (conversions at compile time)

## Estimated Timeline

- Phase 1 (Core Infrastructure): 3-4 hours
- Phase 2 (Trait Implementations): 2-3 hours
- Phase 3 (Advanced Operations): 2-3 hours
- Phase 4 (Migration): 4-6 hours
- Phase 5 (Optimization): 2-3 hours (optional)
- Phase 6 (Testing): 3-4 hours
- Phase 7 (Documentation): 2 hours

**Total: 18-25 hours of focused development**

## References

- [Current OffscreenBuffer implementation](../tui/src/tui/terminal_lib_backends/offscreen_buffer/)
- [Pos and dimension types](../tui/src/core/dimens/)
- [Rust Index trait](https://doc.rust-lang.org/std/ops/trait.Index.html)
- [Cache-efficient data structures](https://en.wikipedia.org/wiki/Cache-oblivious_algorithm)