---
name: fast-string-allocations
description: Zero-allocation string building strategies. Use when formatting strings, generating ANSI codes, or writing hot loops to avoid heap allocations and Formatter state machine overhead.
---

# Fast String Allocations & Zero-Overhead Formatting

When writing code that involves string building, formatting, or rendering (especially in the TUI engine, logging, or event loops), you MUST adhere to the codebase's strict zero-allocation architecture.

## When to Use

- When building ANSI escape sequences
- When building strings in a 60 FPS event loop or render cycle
- When using `tracing::*!` macros
- Whenever you are tempted to use `format!` or `write!` on a `Formatter`

## Core Strategy

Read the full Source of Truth at `mod@crate::core::common::fast_strings#string-allocation-performance-strategy` for detailed architectural constraints.

### Performance Hierarchy (Fastest to Slowest)

1. **Direct `push_str()`** - Zero overhead, direct memory copy.
2. **`crate::format_no_alloc!`** - Zero heap allocations, reuses existing heap.
3. **`FastStringify`** - Temporary heap allocation, completely bypasses `Formatter` state machine.
4. **`inline_string!`** - Stack allocated (16 bytes), but dynamically spills to the heap if exceeded.
5. **`format!` / `write!`** - Always heap allocates (`format!`) OR uses heavy formatter overhead (`write!`). Avoid in hot paths.

## Implementation Guidelines

### 1. The `FastStringify` Trait
Use for custom types (like `ANSI` codes) that are serialized millions of times.
- Bypasses the expensive `Formatter` state machine by pushing everything into a temporary heap `String` and making ONE `f.write_str()` call.
- Use `generate_impl_display_for_fast_stringify!` to auto-generate the `Display` block.

### 2. The `inline_string!` Macro
Use for stack-allocated strings (16 bytes).
- Drop-in replacement for `format!`.
- The primary use case is the core Editor Component and `tracing::*!` macros.
- **Warning:** If text exceeds 16 bytes, it spills to the heap and incurs the cost of `format!`.

### 3. The `format_no_alloc!` Macro
Use for hot loops where the string is guaranteed to exceed 16 bytes (meaning `inline_string!` would spill).
- Hoist a `String::with_capacity(N)` outside the loop.
- Pass the hoisted string into the macro inside the loop to clear and write into the existing capacity.
- Zero allocations per tick.
