# ANSI Parser Testing Guide

This guide provides comprehensive information about testing ANSI escape sequence processing in the R3BL TUI library. The testing infrastructure uses VT100 conformance tests to ensure compatibility with real-world terminal applications.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Type-Safe Sequence Builders](#type-safe-sequence-builders)
- [Conformance Test Structure](#conformance-test-structure)
- [Real-World Testing Scenarios](#real-world-testing-scenarios)
- [Running Tests](#running-tests)
- [Adding New Tests](#adding-new-tests)
- [Troubleshooting](#troubleshooting)
- [VT100 Specification References](#vt100-specification-references)

## Overview

The ANSI parser testing infrastructure validates the complete sequence processing pipeline:

```
ANSI Sequences → VTE Parser → Perform Trait → OffscreenBuffer Updates
```

Instead of testing isolated sequence fragments, the conformance tests validate realistic patterns extracted from actual terminal applications (vim, emacs, tmux) to ensure real-world compatibility.

## Architecture

### Testing Pipeline

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│ Conformance     │───▶│ Type-Safe        │───▶│ ANSI Parser     │
│ Data Functions  │    │ Sequence         │    │ Integration     │
│                 │    │ Builders         │    │ Tests           │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│ vim_sequences   │    │ CsiSequence      │    │ OffscreenBuffer │
│ emacs_sequences │    │ EscSequence      │    │ State           │
│ tmux_sequences  │    │ SgrCode          │    │ Validation      │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

### Key Components

- **Conformance Data Modules**: Organized sequence patterns by application/functionality
- **Type-Safe Builders**: Compile-time validated sequence construction
- **Test Framework**: Integration tests with realistic terminal dimensions
- **Assertion Helpers**: Specialized functions for validating terminal state

## Type-Safe Sequence Builders

### Traditional vs. Modern Approach

```rust
// ❌ Traditional: Hardcoded escape sequences (error-prone)
let old_approach = b"\x1b[2J\x1b[H\x1b[31mError\x1b[0m\x1b[2;5H";

// ✅ Modern: Type-safe builders (compile-time validated)
let new_approach = format!("{}{}{}Error{}{}",
    CsiSequence::EraseDisplay(2),                    // Clear screen
    CsiSequence::CursorPosition {                    // Home cursor
        row: term_row(1), col: term_col(1)
    },
    SgrCode::ForegroundBasic(ANSIBasicColor::Red),   // Red text
    SgrCode::Reset,                                  // Reset styling
    CsiSequence::CursorPosition {                    // Position cursor
        row: term_row(2), col: term_col(5)
    }
);
```

### Builder Benefits

- **Compile-time validation**: Invalid sequences cause compilation errors
- **Self-documenting**: Clear intent through function/enum names
- **Refactoring safety**: Changes to builders automatically update all tests
- **IDE support**: Full autocomplete and error checking
- **Specification mapping**: Direct correspondence to VT100 commands

## Conformance Test Structure

### Module Organization

```
vt_100_ansi_conformance_tests/
├── conformance_data/           # Sequence builder functions
│   ├── basic_sequences.rs      # Simple operations
│   ├── cursor_sequences.rs     # Cursor control
│   ├── styling_sequences.rs    # Text formatting
│   ├── vim_sequences.rs        # Vim editor patterns
│   ├── emacs_sequences.rs      # Emacs editor patterns
│   ├── tmux_sequences.rs       # Terminal multiplexer
│   └── edge_case_sequences.rs  # Boundary conditions
├── tests/                      # Test modules
│   ├── test_real_world_scenarios.rs
│   ├── test_cursor_operations.rs
│   ├── test_sgr_and_character_sets.rs
│   └── ...
└── test_fixtures.rs           # Shared utilities
```

### Sequence Pattern Categories

| Category | Purpose | Examples |
|----------|---------|----------|
| **Basic** | Simple operations | Clear screen, move cursor |
| **Cursor** | Positioning & control | Save/restore, movement |
| **Styling** | Text formatting | Colors, bold, italic |
| **Display** | Screen manipulation | Erase, scroll regions |
| **Real-world** | Application patterns | Editor status lines |
| **Edge cases** | Boundary testing | Malformed sequences |

## Real-World Testing Scenarios

### Terminal Dimensions

Tests use realistic **80x25** terminal dimensions instead of constrained test buffers:

```rust
fn create_realistic_terminal_buffer() -> OffscreenBuffer {
    use crate::{height, width};
    OffscreenBuffer::new_empty(height(25) + width(80))
}
```

### Application Patterns

#### Vim Editor Scenarios
```rust
// Status line with mode indicator
let status_sequence = vim_sequences::vim_status_line("INSERT", 25);
ofs_buf.apply_ansi_bytes(status_sequence);

// Syntax highlighting with multiple colors
let syntax_sequence = vim_sequences::vim_syntax_highlighting();
ofs_buf.apply_ansi_bytes(syntax_sequence);

// Error message display
let error_sequence = vim_sequences::vim_error_message("E32: No such file", 25);
ofs_buf.apply_ansi_bytes(error_sequence);
```

#### Emacs Editor Scenarios
```rust
// Mode line with buffer information
let mode_line = emacs_sequences::emacs_mode_line();
ofs_buf.apply_ansi_bytes(mode_line);

// Minibuffer prompt
let prompt = emacs_sequences::emacs_minibuffer_prompt("Find file: ");
ofs_buf.apply_ansi_bytes(prompt);
```

#### Tmux Terminal Multiplexer
```rust
// Status bar with session information
let status_bar = tmux_sequences::tmux_status_bar();
ofs_buf.apply_ansi_bytes(status_bar);

// Pane splitting visualization
let pane_split = tmux_sequences::tmux_pane_split_horizontal();
ofs_buf.apply_ansi_bytes(pane_split);
```

### Complex Interaction Patterns

#### Cursor Save/Restore with Styling
```rust
// Test cursor save, move, style, restore pattern
let complex_sequence = cursor_sequences::save_do_restore(
    &format!("{}{}{}",
        CsiSequence::CursorPosition { row: term_row(10), col: term_col(10) },
        SgrCode::ForegroundBasic(ANSIBasicColor::Blue),
        "Temporary text"
    ),
    true  // Use ESC 7/8 (vs CSI s/u)
);
```

#### Multi-Color Syntax Highlighting
```rust
// Rainbow text with different colors per character
let rainbow = styling_sequences::rainbow_text("SYNTAX");
ofs_buf.apply_ansi_bytes(rainbow);

// Verify each character has correct color
assert_styled_char_at(&ofs_buf, 0, 0, 'S',
    |style| style.color_fg == Some(ANSIBasicColor::Red.into()),
    "red color on S");
```

## Running Tests

### Basic Test Execution

```bash
# All conformance tests (101+ tests)
cargo test vt_100_ansi_conformance_tests

# Specific test categories
cargo test test_real_world_scenarios     # Application patterns
cargo test test_cursor_operations         # Cursor control
cargo test test_sgr_and_character_sets    # Text styling
cargo test test_line_wrap_and_scroll_control
```

### Filtered Test Execution

```bash
# Run only vim-related tests
cargo test vt_100_ansi_conformance_tests -- vim

# Run only cursor positioning tests
cargo test test_cursor_operations::positioning

# Run with detailed output
cargo test vt_100_ansi_conformance_tests -- --nocapture
```

### Development Testing

```bash
# Watch mode for continuous testing
cargo watch -x "test vt_100_ansi_conformance_tests"

# Test with specific pattern
cargo watch -x "test test_real_world_scenarios::test_vim"
```

## Adding New Tests

### 1. Create Sequence Patterns

Add functions to appropriate conformance data module:

```rust
// In conformance_data/vim_sequences.rs
pub fn vim_visual_block_selection(start_row: u16, start_col: u16, end_row: u16, end_col: u16) -> String {
    format!("{}{}{}{}{}",
        CsiSequence::CursorPosition { row: term_row(start_row), col: term_col(start_col) },
        EscSequence::SaveCursor,
        SgrCode::Invert,
        // ... selection highlighting logic
        SgrCode::Reset
    )
}
```

### 2. Add VT100 Specification References

Document the specification compliance:

```rust
/// Visual block selection highlighting.
///
/// **VT100 Spec**: Uses ESC 7 (Save cursor) and SGR 7 (Reverse video)
/// as documented in VT100 User Guide Section 3.3.4 and 3.3.5.
pub fn vim_visual_block_selection(...) -> String {
    // Implementation
}
```

### 3. Create Integration Tests

Add test functions that validate complete behavior:

```rust
#[test]
fn test_vim_visual_block_selection() {
    let mut ofs_buf = create_realistic_terminal_buffer();

    // Apply visual selection sequence
    let selection = vim_sequences::vim_visual_block_selection(5, 10, 8, 20);
    let (osc_events, dsr_responses) = ofs_buf.apply_ansi_bytes(selection);

    // Validate behavior
    assert_eq!(osc_events.len(), 0);
    assert_eq!(dsr_responses.len(), 0);

    // Check selection highlighting
    for row in 5..=8 {
        for col in 10..=20 {
            assert_styled_char_at(&ofs_buf, row-1, col-1, // Convert to 0-based
                |style| matches!(style.attribs.invert, Some(_)),
                "visual selection highlighting");
        }
    }
}
```

### 4. Test with Realistic Scenarios

Ensure tests use realistic terminal dimensions and patterns:

```rust
// ✅ Good: Realistic dimensions
let mut ofs_buf = create_realistic_terminal_buffer(); // 80x25

// ❌ Bad: Constrained test buffer
let mut ofs_buf = OffscreenBuffer::new_empty(size!(10, 10));
```

## Troubleshooting

### Common Test Failures

#### 1. Buffer Dimension Issues
```
Error: Index out of bounds

Solution: Use realistic terminal dimensions (80x25) instead of small test buffers:
```rust
// Change this:
let mut ofs_buf = OffscreenBuffer::new_empty(size!(10, 10));

// To this:
let mut ofs_buf = create_realistic_terminal_buffer();
```

#### 2. Style Assertion Failures
```
Error: Expected color not found

Solution: Check both foreground and background colors:
```rust
// Instead of checking only foreground:
assert_styled_char_at(&ofs_buf, row, col, ch,
    |style| style.color_fg == Some(color),
    "foreground color");

// Also check background if relevant:
assert_styled_char_at(&ofs_buf, row, col, ch,
    |style| style.color_bg == Some(bg_color),
    "background color");
```

#### 3. Coordinate System Confusion
```
Error: Character not found at expected position

Solution: Remember 1-based vs 0-based indexing:
```rust
// Terminal coordinates are 1-based
CsiSequence::CursorPosition { row: term_row(5), col: term_col(10) }

// But buffer assertions are 0-based
assert_plain_char_at(&ofs_buf, 4, 9, expected_char); // row-1, col-1
```

### Debugging Techniques

#### 1. Print Buffer State
```rust
// Add this to debug buffer contents:
eprintln!("Buffer state:\n{}", ofs_buf.debug_string());
```

#### 2. Validate Sequence Generation
```rust
// Print generated sequences for inspection:
let sequence = vim_sequences::vim_status_line("INSERT", 25);
eprintln!("Generated sequence: {:?}", sequence.as_bytes());
```

#### 3. Step-by-Step Testing
```rust
// Test sequences individually:
let clear_seq = basic_sequences::clear_and_home();
ofs_buf.apply_ansi_bytes(clear_seq);
// Validate clear worked

let status_seq = vim_sequences::vim_status_line("INSERT", 25);
ofs_buf.apply_ansi_bytes(status_seq);
// Validate status line
```

## VT100 Specification References

### Primary Sources

1. **[VT100 User Guide](https://vt100.net/docs/vt100-ug/)**
   - Chapter 3: Programming the VT100
   - Section 3.3: Control Sequences

2. **[ANSI X3.64 Standard](https://www.ecma-international.org/wp-content/uploads/ECMA-48_5th_edition_june_1991.pdf)**
   - ECMA-48: Control Functions for Coded Character Sets

3. **[XTerm Control Sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html)**
   - Extended sequences and modern terminal behavior

### Sequence Categories

#### CSI (Control Sequence Introducer) - ESC[
- **Cursor positioning**: `ESC[{row};{col}H`
- **Display manipulation**: `ESC[{param}J` (Erase Display)
- **SGR styling**: `ESC[{params}m`
- **Scrolling**: `ESC[{count}S` (Scroll Up)

#### ESC (Escape) Sequences
- **Cursor save**: `ESC 7`
- **Cursor restore**: `ESC 8`
- **Character sets**: `ESC ( 0` (DEC line drawing)

#### SGR (Select Graphic Rendition) Codes
- **Colors**: 30-37 (foreground), 40-47 (background)
- **Attributes**: 1 (bold), 3 (italic), 4 (underline), 7 (reverse)
- **Reset**: 0 (reset all), 22 (reset bold), 27 (reset reverse)

### Implementation Notes

The ANSI parser aims for VT100 baseline compatibility with selected XTerm extensions. Not all modern terminal features are supported, focusing on sequences commonly used by terminal applications like vim, emacs, and tmux.

## Performance Considerations

### Test Execution Time

The conformance tests are designed to run quickly while maintaining comprehensive coverage:

- **Type-safe builders**: Compile-time validation reduces runtime overhead
- **Realistic buffers**: 80x25 dimensions balance realism with performance
- **Focused assertions**: Tests validate specific behavior without exhaustive state checking

### Memory Usage

- **Zero-copy parsing**: VTE parser processes bytes without string allocation
- **Incremental updates**: Only modified buffer regions are updated during testing
- **Bounded sequences**: Test sequences are reasonably sized to avoid memory pressure

---

For more information about the ANSI parser implementation, see the module documentation in `src/core/pty_mux/ansi_parser/mod.rs` and the conformance test documentation in `vt_100_ansi_conformance_tests/mod.rs`.