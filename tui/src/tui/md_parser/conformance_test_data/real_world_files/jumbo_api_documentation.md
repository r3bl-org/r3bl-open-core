# Comprehensive API Documentation 📋

@title: R3BL TUI Framework API Documentation
@tags: api, documentation, rust, tui, framework
@authors: R3BL Development Team
@date: 2025-07-10
@version: 0.7.1

## Table of Contents

1. [Overview](#overview)
2. [Installation](#installation)
3. [Core Components](#core-components)
4. [Advanced Features](#advanced-features)
5. [Performance Benchmarks](#performance-benchmarks)
6. [Migration Guide](#migration-guide)
7. [Troubleshooting](#troubleshooting)
8. [API Reference](#api-reference)

---

## Overview 🌟

The **R3BL TUI Framework** is a comprehensive terminal user interface library for Rust that provides:

- 🚀 **High Performance**: Zero-allocation parsing with O(1) operations
- 🦄 **Unicode Safety**: Full UTF-8 support with grapheme cluster handling
- 🎨 **Rich Styling**: Colors, themes, and customizable components
- 📦 **Modular Design**: Use only what you need
- 🔧 **Real-time Editing**: Optimized for interactive applications

### Key Statistics

| Metric | Value | Benchmark |
|--------|-------|-----------|
| **Parse Speed** | 15μs/KB | vs 45μs/KB (legacy) |
| **Memory Usage** | 2.1MB peak | vs 8.7MB (legacy) |
| **Unicode Support** | 100% ✅ | Full grapheme clusters |
| **Test Coverage** | 94.2% | 51 compatibility tests |

## Installation 📦

Add to your `Cargo.toml`:

```toml
[dependencies]
r3bl_tui = "0.7.1"
r3bl_core = "0.4.0"

# Optional features
r3bl_tui = { version = "0.7.1", features = ["async", "crossterm"] }
```

### Feature Flags

- `async`: Enable async/await support for non-blocking operations
- `crossterm`: Cross-platform terminal backend (default)
- `termion`: Alternative Unix-only backend
- `tracing`: Enhanced logging and debugging support

## Core Components 🏗️

### AsStrSlice: Virtual Array Technology

The cornerstone of our zero-allocation approach:

```rust
use r3bl_tui::{AsStrSlice, GCStringOwned};

// Convert editor lines to virtual array
let lines: Vec<GCStringOwned> = vec![
    "# Heading".into(),
    "Content here".into(),
    "More content".into(),
];

let slice = AsStrSlice::from(&lines);
// No memory copying - just virtual indexing!
```

#### Performance Characteristics

```rust
// O(1) character access
let char_at_pos = slice.char_at(position)?;

// O(1) substring operations
let substring = slice.substr(start, end)?;

// O(1) line boundary detection
let line_info = slice.line_at_char_index(index)?;
```

### Markdown Parser: Next Generation

The `parse_markdown_ng` function provides compatibility with legacy parsers:

```rust
use r3bl_tui::{parse_markdown_ng, parse_markdown, AsStrSlice};

// NG Parser: Zero-allocation path
let result_ng = parse_markdown_ng(slice)?;

// Legacy Parser: String materialization path
let materialized = slice.to_string();
let result_legacy = parse_markdown(&materialized)?;

// Results are identical!
assert_eq!(result_ng.1, result_legacy.1);
```

## Advanced Features 🚀

### Real-time Syntax Highlighting

```rust
use r3bl_tui::{SyntaxHighlighter, Theme, ColorScheme};

let highlighter = SyntaxHighlighter::new()
    .with_theme(Theme::Dark)
    .with_language("rust")
    .with_unicode_support(true);

// Real-time highlighting as user types
let highlighted = highlighter.highlight_incremental(&slice, cursor_pos)?;
```

### Custom Parser Extensions

Extend the parser with custom markdown elements:

```rust
use r3bl_tui::{Parser, ParserExtension, Element};

struct CustomAdmonitionParser;

impl ParserExtension for CustomAdmonitionParser {
    fn parse(&self, input: AsStrSlice) -> ParseResult<Element> {
        // Custom parsing logic for !!! admonitions
        if input.starts_with("!!!") {
            // Parse admonition block
            Ok((remaining, Element::CustomAdmonition { .. }))
        } else {
            Err(ParseError::NoMatch)
        }
    }
}

let parser = Parser::new()
    .add_extension(CustomAdmonitionParser)
    .build();
```

### Async Operations

Non-blocking parsing for large documents:

```rust
use r3bl_tui::{AsyncParser, ProgressCallback};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let large_document = load_large_markdown_file().await?;

    let progress_callback = |percent: f32| {
        println!("Parsing progress: {:.1}%", percent * 100.0);
    };

    let result = AsyncParser::new()
        .with_progress_callback(progress_callback)
        .parse(large_document)
        .await?;

    println!("Parsed {} elements", result.elements.len());
    Ok(())
}
```

## Performance Benchmarks 📊

### Parsing Speed Comparison

```
Document Size | Legacy Parser | NG Parser | Improvement
--------------|---------------|-----------|-------------
1 KB         | 45μs         | 15μs      | 3.0x faster
10 KB        | 420μs        | 95μs      | 4.4x faster
100 KB       | 4.2ms        | 890μs     | 4.7x faster
1 MB         | 52ms         | 8.9ms     | 5.8x faster
10 MB        | 890ms        | 89ms      | 10.0x faster
```

### Memory Usage Profile

```
Operation           | Legacy | NG Parser | Reduction
--------------------|--------|-----------|----------
Initial allocation  | 8.7MB  | 2.1MB     | 76%
Peak during parse   | 24.3MB | 3.8MB     | 84%
Post-parse cleanup  | 12.1MB | 2.1MB     | 83%
```

### Real-world Performance Tests

Based on parsing actual documentation files:

1. **Rust Book (1.2MB)**: 12ms vs 67ms (5.6x improvement)
2. **Linux Kernel Docs (4.8MB)**: 43ms vs 312ms (7.3x improvement)
3. **Wikipedia Article (890KB)**: 8.1ms vs 48ms (5.9x improvement)

## Migration Guide 🔄

### From Legacy Parser

**Before (Legacy):**
```rust
// Old approach - string materialization required
let content = editor_lines.join("\n");
let (remainder, document) = parse_markdown(&content)?;
```

**After (NG Parser):**
```rust
// New approach - zero allocation
let slice = AsStrSlice::from(&editor_lines);
let (remainder, document) = parse_markdown_ng(slice)?;
```

### Breaking Changes in v0.7.0

1. **AsStrSlice Constructor**:
   ```rust
   // OLD
   let slice = AsStrSlice::new(&lines, 0, 0);

   // NEW
   let slice = AsStrSlice::from(&lines);
   ```

2. **Error Types**:
   ```rust
   // OLD
   match result {
       Err(ParseError::InvalidInput(msg)) => { /* handle */ }
   }

   // NEW
   match result {
       Err(ParseError::InvalidInput { message, position }) => { /* handle */ }
   }
   ```

3. **Position Handling**:
   ```rust
   // OLD
   let pos = Position::new(line, col);

   // NEW
   let pos = Position::from_line_col(line, col);
   ```

## Troubleshooting 🔧

### Common Issues

#### Performance Degradation

**Symptom**: Parsing is slower than expected

**Solutions**:
```rust
// ✅ DO: Use AsStrSlice for zero-copy
let slice = AsStrSlice::from(&lines);

// ❌ DON'T: Materialize unless necessary
let content = lines.join("\n"); // Expensive!
```

#### Unicode Rendering Issues

**Symptom**: Emojis or international characters display incorrectly

**Solutions**:
```rust
// ✅ Enable Unicode normalization
let parser = Parser::new()
    .with_unicode_normalization(true)
    .build();

// ✅ Check terminal support
if !terminal_supports_unicode() {
    parser.set_fallback_mode(true);
}
```

#### Memory Leaks

**Symptom**: Memory usage grows over time

**Solutions**:
```rust
// ✅ Reuse AsStrSlice instances
let slice = AsStrSlice::from(&lines);
// slice can be reused for multiple operations

// ✅ Clear caches periodically
parser.clear_syntax_cache();
```

### Debug Mode

Enable enhanced debugging:

```rust
use r3bl_tui::debug;

// Set debug level
debug::set_level(debug::Level::Trace);

// Enable performance profiling
debug::enable_profiling(true);

// Parse with debug info
let result = parse_markdown_ng(slice)
    .with_debug_context("user_document.md")
    .execute()?;
```

## API Reference 📚

### Core Types

#### `AsStrSlice`

The virtual array abstraction for zero-copy operations.

```rust
impl AsStrSlice {
    /// Create from array of lines
    pub fn from(lines: &[GCStringOwned]) -> Self;

    /// Create with character limit
    pub fn with_limit(
        lines: &[GCStringOwned],
        start_line: LineIndex,
        start_col: ColIndex,
        max_chars: Option<CharsCount>
    ) -> Self;

    /// Get character at position
    pub fn char_at(&self, pos: Position) -> Option<char>;

    /// Extract substring
    pub fn substr(&self, start: Position, end: Position) -> Result<String>;

    /// Convert to inline string (for compatibility)
    pub fn to_inline_string(&self) -> String;
}
```

#### `ParseResult<T>`

Standard result type for all parsing operations.

```rust
pub type ParseResult<T> = Result<(AsStrSlice, T), ParseError>;

pub enum ParseError {
    InvalidInput { message: String, position: Position },
    UnexpectedEnd { context: String },
    UnicodeError { details: String },
    InternalError { description: String },
}
```

#### `Document`

Represents a parsed markdown document.

```rust
pub struct Document {
    pub elements: Vec<Element>,
    pub metadata: HashMap<String, String>,
    pub toc: TableOfContents,
}

pub enum Element {
    Heading { level: u8, content: String, id: Option<String> },
    Paragraph { content: String },
    CodeBlock { language: Option<String>, content: String },
    List { ordered: bool, items: Vec<ListItem> },
    Link { text: String, url: String, title: Option<String> },
    Image { alt: String, url: String, title: Option<String> },
    // ... more variants
}
```

### Parser Functions

#### `parse_markdown_ng`

Main parsing function for the next-generation parser.

```rust
pub fn parse_markdown_ng(input: AsStrSlice) -> ParseResult<Document>;
```

**Performance**: O(n) where n is the number of characters
**Memory**: O(1) additional allocation (zero-copy)
**Unicode**: Full support including grapheme clusters

#### `parse_markdown` (Legacy)

Legacy parser for compatibility testing.

```rust
pub fn parse_markdown(input: &str) -> ParseResult<Document>;
```

**Performance**: O(n) where n is the number of characters
**Memory**: O(n) additional allocation (string copying)
**Unicode**: Basic UTF-8 support

### Utility Functions

#### `get_real_world_editor_content`

Returns test content from the example editor.

```rust
pub fn get_real_world_editor_content() -> &'static [&'static str];
```

#### `as_str_slice_test_case!`

Macro for creating test cases in unit tests.

```rust
as_str_slice_test_case!(variable_name, "line1", "line2", "line3");
as_str_slice_test_case!(limited, limit: 100, "long", "content", "here");
```

---

## Changelog 📝

### Version 0.7.1 (Current)

- ✨ **New**: Compatibility test suite with 51 test cases
- 🚀 **Performance**: 10x faster parsing for large documents
- 🐛 **Fix**: Unicode emoji handling in headings
- 📚 **Docs**: Comprehensive API documentation

### Version 0.7.0

- 🎉 **Major**: Introduction of AsStrSlice virtual array
- 🔄 **Breaking**: New error types and position handling
- ⚡ **Performance**: Zero-allocation parsing architecture
- 🦄 **Unicode**: Full grapheme cluster support

### Version 0.6.x

- Legacy parser implementation
- Basic markdown support
- String-based parsing approach

---

**© 2025 R3BL LLC** • [Website](https://r3bl.com) • [GitHub](https://github.com/r3bl-org/r3bl-open-core) • [Documentation](https://docs.r3bl.com)
