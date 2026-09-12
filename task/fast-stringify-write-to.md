# Task: Extend FastStringify with write_to for Zero-Allocation Terminal I/O

## Overview

In `r3bl_tui`, terminal escape sequences and styled text implement the `FastStringify`
trait. While `FastStringify` provides an optimized alternative to `std::fmt` for building
strings in memory (`write_to_buf(&self, acc: &mut BufTextStorage)`), it currently offers
no direct way to write bytes to an I/O stream (`std::io::Write`).

Consequently, throughout `readline_async` (such as `cursor.rs`, `output.rs`, and
`render.rs`), code that writes sequences to `term: &mut dyn io::Write` is forced to wrap
sequences in `inline_string!("{}", seq).as_bytes()` or `seq.to_string().as_bytes()`. This
unnecessarily invokes `format_args!`, the `std::fmt::Formatter` state machine, and
temporary string allocations just to emit a few bytes (typically 4 to 8 bytes) to a
terminal.

This task extends `FastStringify` with a dedicated
`write_to(&self, writer: &mut dyn io::Write)` method, provides an optimized stack-buffer
override for `CsiSequence`, introduces a constant for `CHA(1)`, and migrates call sites
across `readline_async` to direct, zero-allocation terminal writes.

## Problem Analysis and Architecture

### 1. The Gap Between FastStringify and io::Write

`FastStringify` currently defines:

- `fn write_to_buf(&self, acc: &mut BufTextStorage) -> std::fmt::Result;`
- `fn write_buf_to_fmt(&self, acc: &BufTextStorage, f: &mut Formatter<'_>) -> std::fmt::Result;`

And generates `impl Display` via `generate_impl_display_for_fast_stringify!`:

```rust
impl std::fmt::Display for $type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buffer = $crate::BufTextStorage::new();
        self.write_to_buf(&mut buffer)?;
        self.write_buf_to_fmt(&buffer, f)
    }
}
```

When writing to `term: &mut dyn io::Write`:

1. Callers write `inline_string!("{}", CsiSequence::CursorDown(delta)).as_bytes()`.
2. `inline_string!` sets up a `SmallString` stack buffer and calls `write_fmt`.
3. `Display::fmt` allocates a heap `String` (`BufTextStorage::new()`).
4. `write_to_buf` formats into the heap `String`.
5. `write_buf_to_fmt` copies the heap `String` into the stack `SmallString`.
6. `term.write_all()` finally writes the bytes.

This pays for:

- Format string parsing and `format_args!` setup.
- Standard library `Formatter` dispatch.
- Heap allocation in `BufTextStorage::new()`.
- Intermediate copying across two string buffers.

### 2. The Solution: Add `write_to` to `FastStringify`

We add a `write_to` method to `FastStringify`:

```rust
pub trait FastStringify: Display {
    fn write_to_buf(&self, acc: &mut BufTextStorage) -> std::fmt::Result;

    fn write_buf_to_fmt(&self, acc: &BufTextStorage, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(acc)
    }

    /// Writes directly to an I/O stream ([`std::io::Write`]) such as a terminal, stdout, or
    /// PTY stream without invoking [`std::fmt`] machinery or allocating intermediate strings.
    ///
    /// # Default Implementation
    /// The default implementation buffers into [`BufTextStorage`] via [`write_to_buf`] and
    /// flushes all bytes in a single call to [`std::io::Write::write_all`]. Implementations
    /// with known small bounds can override this method to write into a stack-allocated byte
    /// buffer, achieving zero heap allocations and zero [`std::fmt`] overhead.
    ///
    /// # Errors
    /// Returns an [`std::io::Error`] if writing to the output stream fails.
    ///
    /// [`write_to_buf`]: FastStringify::write_to_buf
    fn write_to(&self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        let mut buffer = BufTextStorage::new();
        self.write_to_buf(&mut buffer).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "FastStringify formatting error in write_to_buf",
            )
        })?;
        writer.write_all(buffer.as_bytes())
    }
}
```

### 3. Specialized Zero-Allocation Override for `CsiSequence`

For `CsiSequence`:

- The maximum length of any `CsiSequence` is under 32 bytes (cursor movements are 4 to 8
  bytes; the longest chained private mode is ~25 bytes).
- By routing byte serialization into a `SmallVec<[u8; 32]>`, we ensure zero heap
  allocations.
- Dynamic numbers continue using the existing `convert_u16_to_ascii_str_slice!` stack
  array macro.
- A single atomic `writer.write_all(&buf)` emits the entire escape sequence in one write
  call, avoiding split escape codes across unbuffered streams.
- `write_to_buf` can reuse the byte serialization via UTF-8 conversion, maintaining a
  single source of truth for sequence encoding.

### 4. Compile-Time Constant for `CHA(1)`

`CursorHorizontalAbsolute(TermCol::ONE)` is emitted constantly throughout `readline_async`
to return the cursor to column 1. Because this sequence is invariant (`\x1b[1G`), we
introduce:

```rust
pub const CSI_CHA_1: &str = "\x1b[1G";
```

Call sites can write `CSI_CHA_1.as_bytes()` directly without invoking any dynamic
generator.

## Implementation Steps

### Step 1: Extend `FastStringify` in `tui/src/core/common/fast_strings/fast_stringify.rs`

- Add `fn write_to(&self, writer: &mut dyn std::io::Write) -> std::io::Result<()>` with
  default implementation.
- Add comprehensive doc comments and doctests explaining zero-allocation I/O semantics.

### Step 2: Implement Optimized `write_to` for `CsiSequence` in `sequence.rs`

- In `tui/src/core/ansi/vt_100_pty_output_parser/protocols/csi_codes/sequence.rs`:
    - Implement a `write_bytes(&self, acc: &mut SmallVec<[u8; 32]>)` helper (or write
      directly).
    - Override `FastStringify::write_to` to format into `SmallVec<[u8; 32]>` and call
      `writer.write_all(&buf)`.
    - Unify `write_to_buf` to delegate to `write_bytes` (or keep in sync) to avoid logic
      duplication.

### Step 3: Add `CSI_CHA_1` Constant in `csi.rs`

- In `tui/src/core/ansi/constants/csi.rs`:
    - Define `pub const CSI_CHA_1: &str = "\x1b[1G";`.
    - Export it through the module barrel export chain.

### Step 4: Migrate `cursor.rs` Call Sites

- In `tui/src/readline_async/readline_async_impl/line_state/cursor.rs`:
    - In `paint_cursor_to_start_from`:
        - Replace
          `inline_string!("{}", CsiSequence::CursorHorizontalAbsolute(TermCol::ONE))` with
          `term.write_all(CSI_CHA_1.as_bytes())?`.
        - Replace `inline_string!("{}", CsiSequence::CursorUp(delta))` with
          `CsiSequence::CursorUp(delta).write_to(term)?`.
    - In `paint_cursor_from_start_to`:
        - Replace `inline_string!("{}", CsiSequence::CursorDown(delta))` with
          `CsiSequence::CursorDown(delta).write_to(term)?`.
        - Replace `inline_string!("{}", CsiSequence::CursorForward(delta))` with
          `CsiSequence::CursorForward(delta).write_to(term)?`.
    - Remove unused `inline_string` import.

### Step 5: Migrate `output.rs` and `render.rs` Call Sites

- In `tui/src/readline_async/readline_async_impl/line_state/output.rs`:
    - Replace `inline_string!("{}", CsiSequence::CursorUp(...))` with `.write_to(term)?`.
    - Replace `inline_string!("{}", CsiSequence::CursorHorizontalAbsolute(TermCol::ONE))`
      with `CSI_CHA_1`.
    - Replace `inline_string!("{}", CsiSequence::CursorForward(...))` with
      `.write_to(term)?`.
    - In segment loop, use `CSI_CHA_1.as_bytes()` directly instead of
      `cha_1 = inline_string!(...)`.
- In `tui/src/readline_async/readline_async_impl/line_state/render.rs`:
    - Check and clean up any remaining `inline_string!` calls writing CSI sequences.

### Step 6: Unit and Integration Testing

- Add unit tests for `FastStringify::write_to`:
    - Test default implementation on a mock type.
    - Test `CsiSequence::write_to` parity with `Display` / `to_string()` for all cursor
      variants.
    - Verify zero-byte output on empty cases.
- Run all existing `cursor.rs` tests to verify zero regressions on cursor wrapping and
  row/col calculation.

### Step 7: Quality Verification

- Run `./check.fish --check`.
- Run `./check.fish --clippy`.
- Run `./check.fish --test`.
- Run `./check.fish --quick-doc`.
- Run `./check.fish --fmt`.

### Step 8: Mandatory Manual Review

- [ ] `tui/src/core/common/fast_strings/fast_stringify.rs`
- [ ] `tui/src/core/ansi/constants/csi.rs`
- [ ] `tui/src/core/ansi/vt_100_pty_output_parser/protocols/csi_codes/sequence.rs`
- [ ] `tui/src/readline_async/readline_async_impl/line_state/cursor.rs`
- [ ] `tui/src/readline_async/readline_async_impl/line_state/output.rs`
- [ ] `tui/src/readline_async/readline_async_impl/line_state/render.rs`
