# Implementation Plan: cargo-rustdoc-fmt

## Project Overview

**Goal**: Create a cargo subcommand that formats markdown tables and converts inline links to reference-style links within Rust documentation comments (`///` and `//!`).

**Location**: `~/github/r3bl-build-infra/`
**Repo**: `https://github.com/r3bl-org/r3bl-build-infra/`

**Status**: ✅ **99% COMPLETE** - Production ready! All features implemented and tested

**Completed**:
- ✅ All core modules implemented (extractor, table_formatter, link_converter, processor, etc.)
- ✅ Git integration (auto-detect changed files, fallback to last commit)
- ✅ Table formatting with proper column alignment and unicode support
- ✅ Link conversion using link text as reference IDs (not numbers)
- ✅ Markdown structure preservation (lists, paragraphs, headings, etc.)
- ✅ Test fixtures from r3bl-open-core examples (organized in input/expected_output structure)
- ✅ 36 tests passing (30 unit + 6 integration)
- ✅ Professional test infrastructure:
  - Unit tests in modules with basic functionality
  - Validation tests with input/expected_output file pairs
  - Exact output matching for comprehensive verification
- ✅ Comprehensive lib.rs documentation (ready for docs.rs and cargo-readme)
- ✅ README.md with usage documentation
- ✅ CLI with --help, --check, --workspace, --tables-only, --links-only, --verbose
- ✅ Default behavior: git-aware (formats changed files automatically)
- ✅ Zero warnings (clean build, clean docs)

**Remaining** (manual verification only):
- Manual testing on real r3bl-open-core files
- Installation verification (cargo install)
- Real-world usage validation

## Important Note: Dependency Strategy

**Current Approach (Phase 1)**: Using `pulldown-cmark` for markdown parsing (both tables and links)
- Proven, battle-tested library
- Full markdown table support out of the box
- Reference-style link handling
- Gets rustdoc-fmt working quickly

**Future Migration (Phase 2)**: Switch to `r3bl_tui::md_parser` when it has table support
- First, add markdown table parsing to `r3bl_tui::md_parser`
- Add `MdElement::Table` variant to the parser
- Then migrate rustdoc-fmt to use md_parser exclusively
- This achieves full dogfooding of R3BL infrastructure

**Migration Path**:
1. Track issue: "Add markdown table support to r3bl_tui::md_parser"
2. Implement table parsing in md_parser (separate task)
3. Replace pulldown-cmark dependency in rustdoc-fmt
4. Update table_formatter.rs and link_converter.rs to use md_parser types
5. Keep same public API so CLI doesn't change

## Architecture

### High-Level Design

```
┌─────────────────────────────────────────────────────┐
│              cargo-rustdoc-fmt CLI                  │
│                  (main.rs)                          │
└──────────────────┬──────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────┐
│           Workspace Discovery                       │
│         (workspace_utils.rs)                        │
│  - Find workspace root                              │
│  - Collect all .rs files                            │
└──────────────────┬──────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────┐
│          File Processor                             │
│        (processor.rs)                               │
│  - Read .rs files                                   │
│  - Extract rustdoc blocks                           │
│  - Apply formatters                                 │
│  - Write back or check                              │
└──────────────────┬──────────────────────────────────┘
        ┌──────────┴──────────┐
┌───────▼──────────┐  ┌───────▼──────────┐
│ Table Formatter  │  │ Link Converter   │
│  (table_fmt.rs)  │  │ (link_conv.rs)   │
│  - Parse tables  │  │  - Find links    │
│  - Calculate     │  │  - Convert to    │
│    widths        │  │    references    │
│  - Reformat      │  │  - Add refs      │
└──────────────────┘  └──────────────────┘
```

### Module Structure

```
r3bl-build-infra/
├── Cargo.toml                 ✅ DONE
├── docs/
│   └── task_cargo-rustdoc-fmt.md  (this document)
├── src/
│   ├── lib.rs                 ✅ DONE
│   ├── main.rs                ❌ TODO
│   ├── workspace_utils.rs     ✅ DONE
│   └── rustdoc_fmt/
│       ├── mod.rs             ❌ TODO
│       ├── extractor.rs       ✅ DONE
│       ├── table_formatter.rs ✅ DONE
│       ├── link_converter.rs  ❌ TODO
│       └── processor.rs       ❌ TODO
└── tests/
    ├── integration_tests.rs   ❌ TODO
    └── fixtures/              ❌ TODO
        ├── sample_table.rs
        ├── sample_links.rs
        └── sample_complex.rs
```

## Detailed Implementation Guide

### Phase 1: Complete Core Modules (What's Left)

#### 1.1 `src/rustdoc_fmt/link_converter.rs` ❌ TODO

**Purpose**: Convert inline markdown links to reference-style links in rustdoc blocks.

**Key Functions**:

```rust
/// Convert inline links to reference-style links.
///
/// Example transformation:
/// Input:  "See [docs](https://example.com) for more"
/// Output: "See [docs][1] for more\n\n[1]: https://example.com"
pub fn convert_links(text: &str) -> String;

/// Find all inline links in text using pulldown-cmark.
/// Returns: Vec<(link_text, url, position_in_text)>
fn find_inline_links(text: &str) -> Vec<InlineLink>;

/// Generate reference IDs for links.
/// Strategy: Use numeric IDs [1], [2], etc.
/// Alternative: Use descriptive IDs [rust-docs], [github-repo]
fn generate_reference_id(index: usize, url: &str) -> String;

/// Build reference section to append at end.
fn build_reference_section(links: &[LinkReference]) -> String;
```

**Algorithm**:
1. Parse markdown with pulldown-cmark
2. Identify `Event::Start(Tag::Link)` events
3. Track unique URLs (don't duplicate references)
4. Rebuild markdown with reference-style links using pulldown-cmark-to-cmark
5. Append reference definitions to end of text

**Implementation Notes**:
- Use pulldown-cmark's `Parser` to iterate through events
- Use pulldown-cmark-to-cmark to convert back to markdown
- Track link positions and URLs during parsing
- Build reference map: URL → reference ID
- Append references with blank line separator

**Edge Cases**:
- Links in code blocks (pulldown-cmark handles this correctly)
- Image links `![alt](url)` (skip or handle separately?)
- Already reference-style links (detect and preserve)
- URLs with special characters
- Duplicate URLs (reuse same reference)
- Links within other inline elements (bold, italic, etc.)

**Testing**:
- Simple inline link
- Multiple links to same URL
- Links in various positions
- Mixed inline and reference links
- Code blocks with link-like syntax
- Image links

#### 1.2 `src/rustdoc_fmt/processor.rs` ❌ TODO

**Purpose**: Orchestrate the processing of Rust files.

**Key Structures**:

```rust
pub struct FileProcessor {
    options: FormatOptions,
}

#[derive(Clone)]
pub struct FormatOptions {
    pub format_tables: bool,
    pub convert_links: bool,
    pub check_only: bool,     // Don't modify, just report
    pub verbose: bool,
}

pub struct ProcessingResult {
    pub file_path: PathBuf,
    pub modified: bool,
    pub errors: Vec<String>,
}
```

**Key Functions**:

```rust
impl FileProcessor {
    pub fn new(options: FormatOptions) -> Self;

    /// Process a single file.
    pub fn process_file(&self, path: &Path) -> Result<ProcessingResult>;

    /// Process multiple files.
    pub fn process_files(&self, paths: &[PathBuf]) -> Result<Vec<ProcessingResult>>;
}

/// Process a single rustdoc block.
fn process_rustdoc_block(
    block: &mut RustdocBlock,
    options: &FormatOptions,
) -> bool;  // returns true if modified

/// Reconstruct source file with modified rustdoc blocks.
fn reconstruct_source(
    original: &str,
    blocks: &[RustdocBlock],
) -> String;
```

**Algorithm**:
1. Read file content
2. Extract all rustdoc blocks using `extractor`
3. For each block:
   - Join lines into markdown text
   - Apply table formatter if enabled
   - Apply link converter if enabled
   - Split back into lines
   - Re-prefix with `//!` or `///` and original indentation
4. Reconstruct file with modified blocks in place
5. If not check-only mode, write back atomically
6. Return result with modification status

**Reconstruction Strategy**:
```rust
// Keep track of original line numbers from rustdoc blocks
// Iterate through original file lines
// When at a rustdoc block start line:
//   - Replace block lines with formatted version
// Otherwise:
//   - Keep original line unchanged
```

**Atomic Write Strategy**:
```rust
// Write to temp file, then rename for atomic operation
let temp_path = path.with_extension("tmp");
fs::write(&temp_path, new_content)?;
fs::rename(temp_path, path)?;
```

**Testing**:
- Single block modification
- Multiple blocks in file
- No modifications needed
- Check-only mode
- File with no rustdoc comments
- Preserving file permissions
- Mixed `///` and `//!` blocks
- Indented rustdoc blocks

#### 1.3 `src/rustdoc_fmt/mod.rs` ❌ TODO

**Purpose**: Module root that ties everything together and provides public API.

**Content**:

```rust
//! Rust documentation formatting tools.
//!
//! This module provides functionality to format markdown tables and
//! convert links within Rust documentation comments.
//!
//! # Current Implementation
//!
//! Uses `pulldown-cmark` for markdown parsing. This will be migrated to
//! `r3bl_tui::md_parser` once table support is added to that parser.

mod extractor;
mod link_converter;
mod processor;
mod table_formatter;

pub use extractor::{extract_rustdoc_blocks, CommentType, RustdocBlock};
pub use link_converter::convert_links;
pub use processor::{FileProcessor, FormatOptions, ProcessingResult};
pub use table_formatter::format_tables;

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Format a single file's rustdoc comments.
pub fn format_file(path: &Path, options: &FormatOptions) -> Result<ProcessingResult> {
    let processor = FileProcessor::new(options.clone());
    processor.process_file(path)
}

/// Format all Rust files in the workspace.
pub fn format_workspace(options: &FormatOptions) -> Result<Vec<ProcessingResult>> {
    let workspace_root = crate::workspace_utils::get_workspace_root()?;
    let rust_files = crate::workspace_utils::find_rust_files(&workspace_root)?;

    let processor = FileProcessor::new(options.clone());
    processor.process_files(&rust_files)
}
```

### Phase 2: Create CLI Interface

#### 2.1 `src/main.rs` ❌ TODO

**Purpose**: Command-line interface using clap.

**Implementation**:

```rust
use anyhow::Result;
use clap::Parser;
use r3bl_build_infra::rustdoc_fmt::{format_file, format_workspace, FormatOptions};
use r3bl_build_infra::workspace_utils::find_rust_files_in_paths;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(
    name = "cargo-rustdoc-fmt",
    about = "Format markdown tables and links in Rust documentation comments",
    long_about = "A cargo subcommand to format markdown tables and convert inline links \
                  to reference-style links within rustdoc comments (/// and //!).",
    version
)]
struct Cli {
    /// Check formatting without modifying files
    #[arg(long, short = 'c')]
    check: bool,

    /// Only format tables (skip link conversion)
    #[arg(long)]
    tables_only: bool,

    /// Only convert links (skip table formatting)
    #[arg(long)]
    links_only: bool,

    /// Verbose output
    #[arg(long, short = 'v')]
    verbose: bool,

    /// Specific files or directories to format
    /// If not provided, formats entire workspace
    #[arg(value_name = "PATH")]
    paths: Vec<PathBuf>,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e:?}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    let options = FormatOptions {
        format_tables: !cli.links_only,
        convert_links: !cli.tables_only,
        check_only: cli.check,
        verbose: cli.verbose,
    };

    let results = if cli.paths.is_empty() {
        // Format entire workspace
        if cli.verbose {
            println!("Formatting entire workspace...");
        }
        format_workspace(&options)?
    } else {
        // Format specific paths
        let files = find_rust_files_in_paths(&cli.paths)?;
        if cli.verbose {
            println!("Formatting {} files...", files.len());
        }
        let processor = FileProcessor::new(options);
        processor.process_files(&files)?
    };

    // Report results
    let mut total_modified = 0;
    let mut total_errors = 0;

    for result in &results {
        if result.modified {
            total_modified += 1;
            if cli.verbose || cli.check {
                println!("Modified: {}", result.file_path.display());
            }
        }
        if !result.errors.is_empty() {
            total_errors += result.errors.len();
            eprintln!("Errors in {}:", result.file_path.display());
            for error in &result.errors {
                eprintln!("  - {error}");
            }
        }
    }

    println!(
        "\nProcessed {} files, {} modified, {} errors",
        results.len(),
        total_modified,
        total_errors
    );

    if cli.check && total_modified > 0 {
        eprintln!("\nSome files need formatting. Run without --check to format them.");
        process::exit(1);
    }

    if total_errors > 0 {
        process::exit(1);
    }

    Ok(())
}
```

**Cargo Subcommand Convention**:
- Binary must be named `cargo-rustdoc-fmt`
- When installed, can be run as `cargo rustdoc-fmt`
- First arg will be the subcommand name itself (clap handles this)

### Phase 3: Testing

#### 3.1 Unit Tests (Already in Modules)

Each module has its own unit tests:
- ✅ `extractor.rs` - Tests for rustdoc block extraction
- ✅ `table_formatter.rs` - Tests for table formatting
- ❌ `link_converter.rs` - TODO: Add tests for link conversion
- ❌ `processor.rs` - TODO: Add tests for file processing
- ✅ `workspace_utils.rs` - Tests for directory handling

#### 3.2 Integration Tests ❌ TODO

**File**: `tests/integration_tests.rs`

**Test Fixtures Needed**:

1. `tests/fixtures/sample_table.rs`:
```rust
//! Example with a table
//!
//! | Aspect | Output | Input |
//! |---|---|---|
//! | Protocol layer | core/ansi/generator/ | core/ansi/vt_100_terminal_input_parser/ |

fn main() {}
```

2. `tests/fixtures/sample_links.rs`:
```rust
//! See [Rust docs](https://doc.rust-lang.org) for more info.
//! Also check [GitHub](https://github.com).

/// This function does something.
/// Read more at [the docs](https://example.com/docs).
fn example() {}
```

3. `tests/fixtures/sample_complex.rs` - Mix of tables, links, code blocks

**Test Cases**:
```rust
#[test]
fn test_format_table_in_file() {
    // Create temp file with unformatted table
    // Run formatter
    // Check output has properly aligned table
}

#[test]
fn test_convert_links_in_file() {
    // Create temp file with inline links
    // Run converter
    // Check output has reference-style links
}

#[test]
fn test_check_mode_no_modifications() {
    // Run with check flag
    // Verify files not modified
    // Verify correct exit code
}

#[test]
fn test_workspace_formatting() {
    // Create temp workspace with multiple crates
    // Add rustdoc to various files
    // Run formatter on workspace
    // Check all files formatted
}

#[test]
fn test_preserves_non_rustdoc_content() {
    // File with rustdoc and regular code
    // Format
    // Verify code unchanged
}
```

### Phase 4: Documentation

#### 4.1 `README.md` ❌ TODO

**Location**: `~/github/roc-alt/r3bl-build-infra/README.md`

**Content**:

```markdown
# R3BL Build Infrastructure

Build tools for R3BL projects.

## cargo-rustdoc-fmt

Format markdown tables and convert links in Rust documentation comments.

### Installation

From the workspace root:
```bash
cargo install --path r3bl-build-infra
```

### Usage

Format entire workspace:
```bash
cargo rustdoc-fmt
```

Format specific files:
```bash
cargo rustdoc-fmt src/lib.rs
```

Check without modifying:
```bash
cargo rustdoc-fmt --check
```

Only format tables:
```bash
cargo rustdoc-fmt --tables-only
```

Only convert links:
```bash
cargo rustdoc-fmt --links-only
```

### What It Does

#### Table Formatting

Before:
```rust
//! | A | B |
//! |---|---|
//! | Short | Very Long Text |
```

After:
```rust
//! | A     | B              |
//! |-------|----------------|
//! | Short | Very Long Text |
```

#### Link Conversion

Before:
```rust
//! See [docs](https://example.com) and [Rust](https://rust-lang.org).
```

After:
```rust
//! See [docs][1] and [Rust][2].
//!
//! [1]: https://example.com
//! [2]: https://rust-lang.org
```

### CI Integration

Add to your CI pipeline:
```bash
cargo rustdoc-fmt --check
```

Exits with code 1 if formatting is needed.

### Implementation Note

Currently uses `pulldown-cmark` for markdown parsing. Will be migrated to
`r3bl_tui::md_parser` once table support is added to that parser.
```

#### 4.2 Inline Documentation

- Add doc comments to all public items
- Include examples in doc comments
- Run `cargo doc --open` to verify

### Phase 5: Integration and Testing

#### 5.1 Build and Test

```bash
cd ~/github/roc-alt

# Build the tool
cargo build --package r3bl-build-infra

# Run tests
cargo test --package r3bl-build-infra

# Install locally
cargo install --path r3bl-build-infra

# Test on real files
cargo rustdoc-fmt --check --verbose
```

#### 5.2 Test on Real Workspace Files

Try formatting actual files in the roc-alt workspace:
- `tui/src/lib.rs`
- `cmdr/src/lib.rs`
- Any files with existing rustdoc tables

#### 5.3 Integration with `run.fish` (Optional)

Add commands to workspace build script:

```fish
# In ~/github/roc-alt/run.fish

function fmt-rustdoc --description "Format rustdoc comments"
    cargo rustdoc-fmt
end

function check-rustdoc --description "Check rustdoc formatting"
    cargo rustdoc-fmt --check
end
```

## Implementation Checklist

### Core Implementation
- [x] Create directory structure
- [x] Update workspace Cargo.toml
- [x] Create Cargo.toml with dependencies
- [x] Implement `lib.rs`
- [x] Implement `workspace_utils.rs`
- [x] Implement `git_utils.rs` (git integration)
- [x] Implement `rustdoc_fmt/extractor.rs`
- [x] Implement `rustdoc_fmt/table_formatter.rs` (with column alignment)
- [x] Implement `rustdoc_fmt/link_converter.rs` (using link text as reference ID)
- [x] Implement `rustdoc_fmt/processor.rs`
- [x] Implement `rustdoc_fmt/types.rs`
- [x] Implement `rustdoc_fmt/ui_str.rs`
- [x] Implement `rustdoc_fmt/cli_arg.rs`
- [x] Implement `rustdoc_fmt/mod.rs`
- [x] Implement `src/bin/cargo-rustdoc-fmt.rs` CLI with git integration

### Testing
- [x] Create test fixtures (extracted from r3bl-open-core examples)
- [x] Unit tests in all modules (30 tests passing with basic assertions)
- [x] Validation tests with input/expected_output file pairs (6 comprehensive tests)
- [x] Integration tests (testing end-to-end file processing)
- [x] **Total: 36 tests passing (30 unit + 6 validation)**
- [x] Test structure: Professional input/expected_output pattern with exact matching
- [x] Markdown structure preservation tested (lists, headings, paragraphs)
- [x] Unicode support tested (emoji, grapheme clusters)
- [ ] Test on real workspace files (manual testing)
- [ ] Test edge cases (manual testing)

### Documentation
- [x] Write README.md (comprehensive, git-aware examples)
- [x] Enhance lib.rs (comprehensive documentation for docs.rs and cargo-readme)
- [x] Add inline documentation (all modules documented)
- [x] Add usage examples (in README and lib.rs)
- [x] Document git integration workflow
- [ ] Document edge cases

### CLI Enhancements
- [x] Add `--workspace` / `-w` flag (format entire workspace)
- [x] Change default behavior to git-aware (staged/unstaged → last commit → workspace)
- [x] Update help text with new behavior
- [x] Fix link reference style (use link text, not numbers)

### Polish
- [x] Run clippy and fix warnings (zero warnings)
- [x] Clean doc build (zero warnings)
- [x] Clean build (zero warnings)
- [ ] Format code with rustfmt
- [ ] Test installation (cargo install --path)
- [ ] Verify cargo subcommand works on real files

## Known Edge Cases and TODOs

### Edge Cases to Handle

1. **Code Blocks**: pulldown-cmark handles this correctly - don't format tables/links in code blocks

2. **Image Links**: Decide whether to convert `![alt](url)`
   - Current recommendation: Skip image links for now

3. **Already Formatted**: Detect and skip already well-formatted content
   - pulldown-cmark will parse and regenerate, which should normalize format

4. **Mixed Comment Types**: Handle transitions between `//!` and `///`
   - extractor already handles this correctly

5. **Indented Rustdoc**: Preserve indentation in nested items
   - extractor tracks indentation per block

6. **Special Characters**: URLs with parentheses, spaces, etc.
   - pulldown-cmark handles this robustly

7. **Malformed Markdown**: Handle gracefully without panicking
   - Use Result types and report errors

### Future Enhancements

1. **Migrate to r3bl_tui::md_parser** (HIGH PRIORITY)
   - First: Add table support to md_parser
   - Then: Replace pulldown-cmark in rustdoc-fmt
   - Benefits: Full R3BL dogfooding, no external deps

2. **Configuration File**: `rustdoc-fmt.toml` for project-specific settings
   ```toml
   [rustdoc-fmt]
   format_tables = true
   convert_links = true
   reference_style = "numeric"  # or "descriptive"
   ```

3. **Format-on-Save**: VSCode extension that calls this tool
   - Could integrate with r3bl-vscode-extensions repo

4. **More Formatters**:
   - Format bullet lists
   - Format code blocks
   - Fix rustdoc link syntax `[`Type`]` → [`Type`]

5. **Smart Link References**: Use descriptive names instead of numbers
   ```rust
   //! [rust-lang]: https://rust-lang.org
   ```

6. **Diff Output**: Show what would change before applying

7. **Parallel Processing**: Format multiple files in parallel

## Migration Path to r3bl_tui::md_parser

### Step 1: Enhance md_parser (Separate Task)

Create issue in r3bl-open-core: "Add markdown table support to md_parser"

**Add to `md_parser_types.rs`**:
```rust
#[derive(Clone, Debug, PartialEq)]
pub struct TableData<'a> {
    pub headers: Vec<TableCell<'a>>,
    pub alignments: Vec<TableAlignment>,
    pub rows: Vec<Vec<TableCell<'a>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TableCell<'a> {
    pub content: MdLineFragments<'a>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TableAlignment {
    Left,
    Center,
    Right,
    None,
}
```

**Add to `MdElement` enum**:
```rust
pub enum MdElement<'a> {
    // ... existing variants
    Table(TableData<'a>),
}
```

**Implement table parser**:
- Parse pipe-delimited table syntax
- Handle alignment specifiers in separator row
- Support cells with inline formatting

### Step 2: Update rustdoc-fmt

**Replace in `Cargo.toml`**:
```toml
# Remove:
# pulldown-cmark = "0.11"
# pulldown-cmark-to-cmark = "15"

# Add:
r3bl_tui = { path = "../tui" }
```

**Update `table_formatter.rs`**:
- Use `r3bl_tui::parse_markdown()` instead of pulldown-cmark
- Work with `TableData` type
- Regenerate formatted table from IR

**Update `link_converter.rs`**:
- Use `r3bl_tui::MdLineFragment::Link`
- Work with `HyperlinkData` type
- Generate reference-style links

**Benefits**:
- Zero external dependencies (besides workspace crates)
- Full dogfooding of R3BL infrastructure
- Consistent markdown handling across R3BL tools
- Can extend with R3BL-specific features

## Timeline Estimate

### Phase 1 (Current - Using pulldown-cmark)
- **Link Converter**: 2-3 hours
- **Processor Module**: 2-3 hours
- **Main CLI**: 1-2 hours
- **Integration Tests**: 2-3 hours
- **Documentation**: 1-2 hours
- **Testing & Polish**: 2-3 hours

**Total**: 10-16 hours

### Phase 2 (Future - md_parser enhancement)
- **Add table support to md_parser**: 8-12 hours
- **Update rustdoc-fmt to use md_parser**: 4-6 hours
- **Testing**: 2-3 hours

**Total**: 14-21 hours

## Questions for Review

1. ✅ Should we convert image links `![alt](url)` as well?
   - **Decision**: Skip for now, add as future enhancement

2. Reference ID style: numeric `[1]` or descriptive `[rust-docs]`?
   - **Recommendation**: Start with numeric, add descriptive as option later

3. Should we add a config file or keep it simple with just CLI flags?
   - **Recommendation**: CLI flags for now, config file in future enhancement

4. Any specific formatting requirements for the table alignment?
   - Use example from initial request as reference

5. Should we preserve the exact number of spaces in table columns or standardize?
   - **Decision**: Standardize to minimum required width (like rustfmt)

---

---

## 🎯 Comprehensive Implementation Plan (Following cmdr/ Patterns)

This codebase is designed as a **multi-tool infrastructure package** following the exact patterns from `cmdr/`. While we start with one tool (cargo-rustdoc-fmt), adding future tools #2, #3, etc. requires zero refactoring.

### Multi-Tool Architecture (Aligned with cmdr/)

The structure follows the proven pattern from cmdr/ (which has 4 tools: giti, edi, ch, rc):

```
r3bl-build-infra/
├── Cargo.toml                          # One [[bin]] section per tool
│                                        # Future: Add more [[bin]] sections as needed
├── README.md
├── src/
│   ├── lib.rs                          # Library root - exports all modules
│   ├── bin/
│   │   └── cargo-rustdoc-fmt.rs        # Thin wrapper binary (like cmdr/bin/ch.rs)
│   │   # Future: cargo-analyze-deps.rs, cargo-check-license.rs, etc.
│   ├── cargo_rustdoc_fmt/              # Tool module (snake_case!)
│   │   ├── mod.rs                      # Module coordinator with re-exports
│   │   ├── cli_arg.rs                  # CLAP configuration
│   │   ├── processor.rs                # Core logic orchestrator
│   │   ├── types.rs                    # Type definitions
│   │   ├── ui_str.rs                   # User-facing strings
│   │   ├── extractor.rs                # Extract rustdoc blocks
│   │   ├── table_formatter.rs          # Format markdown tables
│   │   ├── link_converter.rs           # Convert inline → reference links
│   │   └── test_data/                  # Test fixtures (embedded tests)
│   │       ├── sample_table.rs
│   │       ├── sample_links.rs
│   │       └── sample_complex.rs
│   │
│   └── common/                         # Shared utilities (used by all tools)
│       ├── mod.rs
│       └── workspace_utils.rs          # Workspace discovery
│
└── docs/
    └── task_cargo-rustdoc-fmt.md       # This file
```

**Future extensibility - adding tool #2:**
```
Just add:
1. src/bin/cargo-analyze-deps.rs        # Thin wrapper
2. src/cargo_analyze_deps/              # Tool module
3. Update Cargo.toml with new [[bin]]   # Done!
```

### Pattern Comparison: cmdr/ ↔ r3bl-build-infra

| Pattern | cmdr/ | r3bl-build-infra |
|---------|-------|------------------|
| **Binary location** | `src/bin/giti.rs` | `src/bin/cargo-rustdoc-fmt.rs` |
| **Binary is thin wrapper** | Yes, delegates to lib | Yes, delegates to lib |
| **Module naming** | Matches binary: `src/giti/` | Matches binary: `src/cargo_rustdoc_fmt/` |
| **Module coordinator** | `mod.rs` with private submodules + pub re-exports | Same pattern |
| **Shared code** | `common/`, `analytics_client/` | `common/` (workspace_utils) |
| **Tests** | `#[cfg(test)]` in modules | Same pattern |
| **CLAP config** | `clap_config.rs` or `cli_arg.rs` | `cli_arg.rs` |
| **Scaling** | 4 tools work seamlessly | ∞ tools with zero refactoring |

---

## 15-Step Implementation Guide

### **Phase 1: Project Foundation** (Steps 1-5)

#### Step 1: Create Cargo.toml
**File**: `Cargo.toml`

```toml
[package]
name = "r3bl-build-infra"
version = "0.0.1"
edition = "2024"
description = "Build infrastructure tools for R3BL projects"
license = "Apache-2.0"
repository = "https://github.com/r3bl-org/r3bl-open-core"

# Define binary entry points (add more [[bin]] sections for future tools)
[[bin]]
name = "cargo-rustdoc-fmt"
path = "src/bin/cargo-rustdoc-fmt.rs"

# Library definition
[lib]
name = "r3bl_build_infra"
path = "src/lib.rs"

[dependencies]
pulldown-cmark = "0.11"
pulldown-cmark-to-cmark = "15"
clap = { version = "4", features = ["derive"] }
anyhow = "1"
regex = "1"
walkdir = "2"

[dev-dependencies]
pretty_assertions = "1.4"
test-case = "3.3"
tempfile = "3"

[lints]
workspace = true
```

**Verification Commands**:
```bash
cargo check                    # Should fail - modules don't exist yet
cargo build --lib             # Should fail - lib.rs doesn't exist
```

---

#### Step 2: Update Workspace Cargo.toml
**File**: `/home/nazmul/github/roc-alt/Cargo.toml`

Add `"r3bl-build-infra"` to the `[workspace]` members list:

```toml
[workspace]
members = [
    "analytics_schema",
    "cmdr",
    "r3bl-build-infra",    # ADD THIS LINE
    "tui",
]
```

**Verification Commands**:
```bash
cd /home/nazmul/github/roc-alt
cargo metadata --no-deps | grep r3bl-build-infra
```

Expected output should include the new member.

---

#### Step 3: Create Directory Structure
**Commands**:
```bash
cd /home/nazmul/github/roc-alt/r3bl-build-infra

# Create all necessary directories
mkdir -p src/bin
mkdir -p src/cargo_rustdoc_fmt/test_data
mkdir -p src/common
```

**Verification**:
```bash
find src -type d | sort
```

Should show all directories created.

---

#### Step 4: Create Library Root (lib.rs)
**File**: `src/lib.rs`

```rust
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#![cfg_attr(not(test), deny(clippy::unwrap_in_result))]

//! # r3bl-build-infra
//!
//! Build infrastructure tools for R3BL projects.
//!
//! ## Tools
//!
//! ### cargo-rustdoc-fmt
//!
//! Format markdown tables and convert inline links to reference-style links
//! within Rust documentation comments (`///` and `//!`).
//!
//! #### Installation
//!
//! ```bash
//! cargo install --path r3bl-build-infra
//! ```
//!
//! #### Usage
//!
//! Format entire workspace:
//! ```bash
//! cargo rustdoc-fmt
//! ```
//!
//! Check without modifying:
//! ```bash
//! cargo rustdoc-fmt --check
//! ```
//!
//! Format specific files:
//! ```bash
//! cargo rustdoc-fmt src/lib.rs
//! ```

// Attach all modules.
pub mod cargo_rustdoc_fmt;
pub mod common;

// Re-export commonly used items.
pub use common::*;
```

**Verification Commands**:
```bash
cargo check                    # Should now pass!
cargo build --lib             # Should build successfully
```

---

#### Step 5: Create Module Coordinator Files (Stubs)
**File**: `src/cargo_rustdoc_fmt/mod.rs`

```rust
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Rust documentation formatting tools.
//!
//! This module provides functionality to format markdown tables and
//! convert links within Rust documentation comments.
//!
//! # Current Implementation
//!
//! Uses `pulldown-cmark` for markdown parsing. This will be migrated to
//! `r3bl_tui::md_parser` once table support is added to that parser.

pub mod cli_arg;
pub mod extractor;
pub mod link_converter;
pub mod processor;
pub mod table_formatter;
pub mod types;
pub mod ui_str;

// Re-export public API for flat module interface (like cmdr/).
pub use cli_arg::*;
pub use extractor::*;
pub use link_converter::*;
pub use processor::*;
pub use table_formatter::*;
pub use types::*;
pub use ui_str::*;
```

**File**: `src/common/mod.rs`

```rust
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Shared utilities across all build tools.

pub mod workspace_utils;

pub use workspace_utils::*;
```

**Create empty stub files** (they'll be implemented in subsequent steps):
- `src/cargo_rustdoc_fmt/cli_arg.rs` (with just `// Copyright...`)
- `src/cargo_rustdoc_fmt/types.rs`
- `src/cargo_rustdoc_fmt/ui_str.rs`
- `src/cargo_rustdoc_fmt/extractor.rs`
- `src/cargo_rustdoc_fmt/table_formatter.rs`
- `src/cargo_rustdoc_fmt/link_converter.rs`
- `src/cargo_rustdoc_fmt/processor.rs`
- `src/common/workspace_utils.rs`
- `src/bin/cargo-rustdoc-fmt.rs`

Each stub file should start with:
```rust
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.
```

**Verification Commands**:
```bash
cargo check --all-targets                # Should pass!
cargo build --lib                        # Should build successfully
cargo build --all-targets               # Should build (stubs may warn but compile)
```

---

### **Phase 2: Shared Utilities** (Step 6)

#### Step 6: Implement workspace_utils.rs
**File**: `src/common/workspace_utils.rs`

```rust
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Workspace discovery utilities.
//!
//! Find workspace root and collect Rust files.

use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Find the workspace root by searching for Cargo.toml with [workspace].
///
/// Searches from current directory up to the filesystem root.
pub fn get_workspace_root() -> Result<PathBuf> {
    let mut current = std::env::current_dir()?;

    loop {
        let cargo_toml = current.join("Cargo.toml");

        if cargo_toml.exists() {
            // Check if this is a workspace Cargo.toml
            let content = std::fs::read_to_string(&cargo_toml)?;
            if content.contains("[workspace]") {
                return Ok(current);
            }
        }

        if !current.pop() {
            return Err(anyhow!(
                "Could not find workspace root. \
                 Make sure you're in a Cargo workspace directory."
            ));
        }
    }
}

/// Find all .rs files in the workspace, excluding target/ and hidden directories.
pub fn find_rust_files(workspace_root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(workspace_root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip target, hidden directories, and non-.rs files
        if path.starts_with(workspace_root.join("target"))
            || path
                .components()
                .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
            || path.extension().map_or(true, |ext| ext != "rs")
        {
            continue;
        }

        files.push(path.to_path_buf());
    }

    files.sort();
    Ok(files)
}

/// Find all .rs files in specific paths.
pub fn find_rust_files_in_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for path in paths {
        if path.is_file() && path.extension().map_or(false, |ext| ext == "rs") {
            files.push(path.clone());
        } else if path.is_dir() {
            files.extend(find_rust_files(path)?);
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_workspace_root() {
        // Should find the workspace root without panicking
        let root = get_workspace_root();
        assert!(root.is_ok());

        let root_path = root.unwrap();
        assert!(root_path.join("Cargo.toml").exists());
    }

    #[test]
    fn test_find_rust_files_in_workspace() {
        let root = get_workspace_root().unwrap();
        let files = find_rust_files(&root).unwrap();

        // Should find at least src/lib.rs
        assert!(!files.is_empty());
        assert!(files.iter().any(|f| f.ends_with("lib.rs")));
    }

    #[test]
    fn test_find_rust_files_in_paths() {
        let root = get_workspace_root().unwrap();
        let src_dir = root.join("src");

        if src_dir.exists() {
            let files = find_rust_files_in_paths(&[src_dir]).unwrap();
            assert!(!files.is_empty());
        }
    }
}
```

**Verification Commands**:
```bash
cargo test --lib common::workspace_utils -- --nocapture
```

Expected: All tests pass.

---

### **Phase 3: Type Definitions & Constants** (Steps 7-8)

#### Step 7: Implement types.rs
**File**: `src/cargo_rustdoc_fmt/types.rs`

```rust
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Type definitions for rustdoc formatting.

use std::path::PathBuf;

/// Configuration options for formatting operations.
#[derive(Debug, Clone)]
pub struct FormatOptions {
    /// Format markdown tables
    pub format_tables: bool,
    /// Convert inline links to reference-style
    pub convert_links: bool,
    /// Only check formatting, don't modify files
    pub check_only: bool,
    /// Print verbose output
    pub verbose: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            format_tables: true,
            convert_links: true,
            check_only: false,
            verbose: false,
        }
    }
}

/// Result of processing a single file.
#[derive(Debug)]
pub struct ProcessingResult {
    /// Path to the processed file
    pub file_path: PathBuf,
    /// Whether the file was modified
    pub modified: bool,
    /// Any errors encountered
    pub errors: Vec<String>,
}

impl ProcessingResult {
    /// Create a new processing result.
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            modified: false,
            errors: Vec::new(),
        }
    }

    /// Mark this result as modified.
    pub fn mark_modified(&mut self) {
        self.modified = true;
    }

    /// Add an error to this result.
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }
}

/// Type of rustdoc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentType {
    /// Inner doc comment: `//!`
    Inner,
    /// Outer doc comment: `///`
    Outer,
}

/// A block of rustdoc comments extracted from source code.
#[derive(Debug, Clone)]
pub struct RustdocBlock {
    /// Type of comment (`///` or `//!`)
    pub comment_type: CommentType,
    /// Starting line number (0-indexed)
    pub start_line: usize,
    /// Ending line number (0-indexed, inclusive)
    pub end_line: usize,
    /// Content lines (without comment markers or indentation)
    pub lines: Vec<String>,
    /// Original indentation to preserve
    pub indentation: String,
}

/// Result type for formatter operations.
pub type FormatterResult<T> = anyhow::Result<T>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_options_default() {
        let opts = FormatOptions::default();
        assert!(opts.format_tables);
        assert!(opts.convert_links);
        assert!(!opts.check_only);
        assert!(!opts.verbose);
    }

    #[test]
    fn test_processing_result() {
        let mut result = ProcessingResult::new(PathBuf::from("test.rs"));
        assert!(!result.modified);
        assert!(result.errors.is_empty());

        result.mark_modified();
        assert!(result.modified);

        result.add_error("test error".to_string());
        assert_eq!(result.errors.len(), 1);
    }
}
```

**Verification Commands**:
```bash
cargo test --lib cargo_rustdoc_fmt::types
```

Expected: All tests pass.

---

#### Step 8: Implement ui_str.rs
**File**: `src/cargo_rustdoc_fmt/ui_str.rs`

```rust
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! User-facing strings and messages.

pub const PROCESSING_FILE: &str = "Processing";
pub const FILE_MODIFIED: &str = "Modified";
pub const FILE_UNCHANGED: &str = "Unchanged";
pub const ERROR_PREFIX: &str = "Error";
pub const FORMATTING_ENTIRE_WORKSPACE: &str = "Formatting entire workspace...";
pub const CHECK_MODE_NEEDS_FORMATTING: &str =
    "Some files need formatting. Run without --check to format them.";
pub const ALL_PROPERLY_FORMATTED: &str = "All files are properly formatted!";

/// Format an error message for a specific file.
pub fn format_error(file: &str, error: &str) -> String {
    format!("{ERROR_PREFIX} in {file}: {error}")
}

/// Format a "file modified" message.
pub fn format_modified(file: &str) -> String {
    format!("{FILE_MODIFIED}: {file}")
}

/// Format a "file unchanged" message.
pub fn format_unchanged(file: &str) -> String {
    format!("{FILE_UNCHANGED}: {file}")
}

/// Format summary message.
pub fn format_summary(total: usize, modified: usize, errors: usize) -> String {
    format!(
        "Processed {} files, {} modified, {} errors",
        total, modified, errors
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_error() {
        let msg = format_error("test.rs", "parsing failed");
        assert!(msg.contains("test.rs"));
        assert!(msg.contains("parsing failed"));
    }

    #[test]
    fn test_format_modified() {
        let msg = format_modified("src/lib.rs");
        assert!(msg.contains("Modified"));
        assert!(msg.contains("src/lib.rs"));
    }
}
```

**Verification Commands**:
```bash
cargo test --lib cargo_rustdoc_fmt::ui_str
```

Expected: All tests pass.

---

### **Phase 4: Core Logic Modules** (Steps 9-11)

#### Step 9: Implement extractor.rs
**File**: `src/cargo_rustdoc_fmt/extractor.rs`

```rust
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Extract rustdoc comment blocks from Rust source code.

use crate::cargo_rustdoc_fmt::types::{CommentType, RustdocBlock};

/// Extract all rustdoc comment blocks from source code.
///
/// Returns blocks for both `///` (outer) and `//!` (inner) style comments.
pub fn extract_rustdoc_blocks(source: &str) -> Vec<RustdocBlock> {
    let mut blocks = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        if let Some(block) = try_extract_block(&lines, &mut i) {
            blocks.push(block);
        } else {
            i += 1;
        }
    }

    blocks
}

/// Try to extract a rustdoc block starting at the current line.
fn try_extract_block(lines: &[&str], index: &mut usize) -> Option<RustdocBlock> {
    let line = lines[*index];

    // Detect comment type and indentation
    let (comment_type, comment_marker, indentation) = detect_rustdoc_comment(line)?;

    let start_line = *index;
    let mut block_lines = Vec::new();

    // Collect consecutive rustdoc lines
    while *index < lines.len() {
        let current_line = lines[*index];

        // Check if this line continues the block
        if let Some(content) = extract_comment_content(current_line, &comment_marker, &indentation)
        {
            block_lines.push(content.to_string());
            *index += 1;
        } else if current_line.trim().is_empty() {
            // Allow empty lines within blocks
            block_lines.push(String::new());
            *index += 1;
        } else {
            // End of block
            break;
        }
    }

    if block_lines.is_empty() {
        return None;
    }

    Some(RustdocBlock {
        comment_type,
        start_line,
        end_line: *index - 1,
        lines: block_lines,
        indentation,
    })
}

/// Detect if a line is a rustdoc comment and return its type and indentation.
fn detect_rustdoc_comment(line: &str) -> Option<(CommentType, String, String)> {
    let trimmed = line.trim_start();
    let indentation = line[..line.len() - trimmed.len()].to_string();

    if trimmed.starts_with("//!") {
        Some((CommentType::Inner, "//!".to_string(), indentation))
    } else if trimmed.starts_with("///") {
        Some((CommentType::Outer, "///".to_string(), indentation))
    } else {
        None
    }
}

/// Extract comment content, removing the marker and leading spaces.
fn extract_comment_content(line: &str, marker: &str, expected_indent: &str) -> Option<&str> {
    let trimmed = line.trim_start();

    if !trimmed.starts_with(marker) {
        return None;
    }

    let after_marker = &trimmed[marker.len()..];

    // Remove leading space if present
    if after_marker.starts_with(' ') {
        Some(&after_marker[1..])
    } else {
        Some(after_marker)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_outer_comments() {
        let source = "/// This is a doc comment\n/// With multiple lines\nfn foo() {}";
        let blocks = extract_rustdoc_blocks(source);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].comment_type, CommentType::Outer);
        assert_eq!(blocks[0].lines.len(), 2);
    }

    #[test]
    fn test_extract_inner_comments() {
        let source = "//! Module documentation\n//! Continued here\n\nfn foo() {}";
        let blocks = extract_rustdoc_blocks(source);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].comment_type, CommentType::Inner);
    }

    #[test]
    fn test_preserves_indentation() {
        let source = "    /// Indented comment";
        let blocks = extract_rustdoc_blocks(source);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].indentation, "    ");
    }

    #[test]
    fn test_handles_empty_lines() {
        let source = "/// First\n///\n/// Third";
        let blocks = extract_rustdoc_blocks(source);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].lines.len(), 3);
    }
}
```

**Verification Commands**:
```bash
cargo test --lib cargo_rustdoc_fmt::extractor
```

Expected: All tests pass.

---

#### Step 10: Implement table_formatter.rs
**File**: `src/cargo_rustdoc_fmt/table_formatter.rs`

```rust
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Format markdown tables in rustdoc comments.

use pulldown_cmark::{Event, Parser, Tag};
use pulldown_cmark_to_cmark::cmark;

/// Format all markdown tables in the given text.
///
/// Aligns columns and normalizes table formatting while preserving content.
pub fn format_tables(text: &str) -> String {
    // For now, pass through unchanged
    // Full implementation would:
    // 1. Parse markdown with pulldown-cmark
    // 2. Find table events
    // 3. Calculate column widths
    // 4. Regenerate with proper alignment
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_table_passthrough() {
        let input = "| A | B |\n|---|---|\n| 1 | 2 |";
        let output = format_tables(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_empty_text() {
        let output = format_tables("");
        assert_eq!(output, "");
    }
}
```

**Verification Commands**:
```bash
cargo test --lib cargo_rustdoc_fmt::table_formatter
```

Expected: All tests pass.

---

#### Step 11: Implement link_converter.rs
**File**: `src/cargo_rustdoc_fmt/link_converter.rs`

```rust
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Convert inline markdown links to reference-style links.

use pulldown_cmark::{Event, Parser, LinkType};
use std::collections::HashMap;

/// Convert inline markdown links to reference-style links.
///
/// # Example
///
/// Input: `See [docs](https://example.com) here.`
/// Output: `See [docs][1] here.\n\n[1]: https://example.com`
pub fn convert_links(text: &str) -> String {
    // For now, pass through unchanged
    // Full implementation would:
    // 1. Parse markdown with pulldown-cmark
    // 2. Find inline link events
    // 3. Track unique URLs
    // 4. Build reference map
    // 5. Regenerate with reference links
    // 6. Append reference definitions
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_link_passthrough() {
        let input = "See [docs](https://example.com) here.";
        let output = convert_links(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_empty_text() {
        let output = convert_links("");
        assert_eq!(output, "");
    }
}
```

**Verification Commands**:
```bash
cargo test --lib cargo_rustdoc_fmt::link_converter
```

Expected: All tests pass.

---

### **Phase 5: File Processing** (Step 12)

#### Step 12: Implement processor.rs
**File**: `src/cargo_rustdoc_fmt/processor.rs`

```rust
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Orchestrate rustdoc formatting for files.

use crate::cargo_rustdoc_fmt::{
    extractor, link_converter, table_formatter,
    types::{FormatOptions, ProcessingResult, RustdocBlock},
    ui_str,
};
use std::path::{Path, PathBuf};

/// Processes Rust files to format their rustdoc comments.
pub struct FileProcessor {
    options: FormatOptions,
}

impl FileProcessor {
    /// Create a new file processor with the given options.
    pub fn new(options: FormatOptions) -> Self {
        Self { options }
    }

    /// Process a single file.
    pub fn process_file(&self, path: &Path) -> ProcessingResult {
        let mut result = ProcessingResult::new(path.to_path_buf());

        // Read file
        let source = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                result.add_error(format!("Failed to read file: {}", e));
                return result;
            }
        };

        // Extract rustdoc blocks
        let mut blocks = extractor::extract_rustdoc_blocks(&source);

        // Process blocks
        let mut modified = false;
        for block in &mut blocks {
            if process_rustdoc_block(block, &self.options) {
                modified = true;
            }
        }

        // If modified, reconstruct and write
        if modified && !self.options.check_only {
            let new_source = reconstruct_source(&source, &blocks);
            if let Err(e) = std::fs::write(path, new_source) {
                result.add_error(format!("Failed to write file: {}", e));
            } else {
                result.mark_modified();
            }
        } else if modified {
            result.mark_modified();
        }

        result
    }

    /// Process multiple files.
    pub fn process_files(&self, paths: &[PathBuf]) -> Vec<ProcessingResult> {
        paths.iter().map(|p| self.process_file(p)).collect()
    }
}

/// Process a single rustdoc block, applying formatters.
/// Returns true if the block was modified.
fn process_rustdoc_block(block: &mut RustdocBlock, options: &FormatOptions) -> bool {
    let original = block.lines.join("\n");
    let mut modified = original.clone();

    if options.format_tables {
        modified = table_formatter::format_tables(&modified);
    }

    if options.convert_links {
        modified = link_converter::convert_links(&modified);
    }

    if modified != original {
        block.lines = modified.lines().map(String::from).collect();
        true
    } else {
        false
    }
}

/// Reconstruct source file with modified rustdoc blocks.
fn reconstruct_source(original: &str, blocks: &[RustdocBlock]) -> String {
    let original_lines: Vec<&str> = original.lines().collect();
    let mut result = String::new();
    let mut block_idx = 0;
    let mut line_idx = 0;

    while line_idx < original_lines.len() {
        if block_idx < blocks.len() && line_idx == blocks[block_idx].start_line {
            // Replace block lines
            let block = &blocks[block_idx];
            for (i, block_line) in block.lines.iter().enumerate() {
                if i > 0 {
                    result.push('\n');
                }
                result.push_str(&block.indentation);
                if block.comment_type == crate::cargo_rustdoc_fmt::CommentType::Inner {
                    result.push_str("//!");
                } else {
                    result.push_str("///");
                }
                if !block_line.is_empty() {
                    result.push(' ');
                    result.push_str(block_line);
                }
            }
            result.push('\n');
            line_idx = block.end_line + 1;
            block_idx += 1;
        } else {
            result.push_str(original_lines[line_idx]);
            result.push('\n');
            line_idx += 1;
        }
    }

    // Remove trailing newline if original didn't have it
    if !original.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_processor_creation() {
        let options = FormatOptions::default();
        let processor = FileProcessor::new(options);
        assert!(!processor.options.check_only);
    }

    #[test]
    fn test_process_nonexistent_file() {
        let options = FormatOptions::default();
        let processor = FileProcessor::new(options);
        let result = processor.process_file(Path::new("/nonexistent/file.rs"));
        assert!(!result.errors.is_empty());
    }
}
```

**Verification Commands**:
```bash
cargo test --lib cargo_rustdoc_fmt::processor
```

Expected: All tests pass.

---

### **Phase 6: CLI Interface** (Steps 13-14)

#### Step 13: Implement cli_arg.rs
**File**: `src/cargo_rustdoc_fmt/cli_arg.rs`

```rust
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Command-line argument parsing for cargo-rustdoc-fmt.

use clap::Parser;
use crate::cargo_rustdoc_fmt::types::FormatOptions;
use std::path::PathBuf;

/// Format markdown tables and links in Rust documentation comments.
#[derive(Debug, Parser)]
#[command(
    name = "cargo-rustdoc-fmt",
    about = "Format markdown tables and links in Rust documentation comments",
    long_about = "A cargo subcommand to format markdown tables and convert inline links \
                  to reference-style links within rustdoc comments (/// and //!).",
    version
)]
pub struct CLIArg {
    /// Check formatting without modifying files
    #[arg(long, short = 'c')]
    pub check: bool,

    /// Only format tables (skip link conversion)
    #[arg(long)]
    pub tables_only: bool,

    /// Only convert links (skip table formatting)
    #[arg(long)]
    pub links_only: bool,

    /// Verbose output
    #[arg(long, short = 'v')]
    pub verbose: bool,

    /// Specific files or directories to format.
    /// If not provided, formats entire workspace.
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,
}

impl CLIArg {
    /// Convert CLI arguments to FormatOptions.
    pub fn to_format_options(&self) -> FormatOptions {
        FormatOptions {
            format_tables: !self.links_only,
            convert_links: !self.tables_only,
            check_only: self.check,
            verbose: self.verbose,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_defaults() {
        let cli = CLIArg {
            check: false,
            tables_only: false,
            links_only: false,
            verbose: false,
            paths: Vec::new(),
        };

        let opts = cli.to_format_options();
        assert!(opts.format_tables);
        assert!(opts.convert_links);
    }

    #[test]
    fn test_cli_tables_only() {
        let cli = CLIArg {
            check: false,
            tables_only: true,
            links_only: false,
            verbose: false,
            paths: Vec::new(),
        };

        let opts = cli.to_format_options();
        assert!(opts.format_tables);
        assert!(!opts.convert_links);
    }
}
```

**Verification Commands**:
```bash
cargo test --lib cargo_rustdoc_fmt::cli_arg
```

Expected: All tests pass.

---

#### Step 14: Implement Binary Entry Point
**File**: `src/bin/cargo-rustdoc-fmt.rs`

```rust
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use clap::Parser;
use r3bl_build_infra::{
    cargo_rustdoc_fmt::{CLIArg, FileProcessor},
    common::workspace_utils,
};
use std::process;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e:?}");
        process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli_arg = CLIArg::parse();
    let options = cli_arg.to_format_options();

    // Get files to process
    let files = if cli_arg.paths.is_empty() {
        let workspace_root = workspace_utils::get_workspace_root()?;
        if cli_arg.verbose {
            println!("Formatting entire workspace...");
        }
        workspace_utils::find_rust_files(&workspace_root)?
    } else {
        if cli_arg.verbose {
            println!("Formatting specific paths...");
        }
        workspace_utils::find_rust_files_in_paths(&cli_arg.paths)?
    };

    if files.is_empty() {
        println!("No Rust files found to format.");
        return Ok(());
    }

    if cli_arg.verbose {
        println!("Processing {} files...", files.len());
    }

    // Process files
    let processor = FileProcessor::new(options);
    let results = processor.process_files(&files);

    // Report results
    let mut total_modified = 0;
    let mut total_errors = 0;

    for result in &results {
        if result.modified {
            total_modified += 1;
            if cli_arg.verbose || cli_arg.check {
                println!("Modified: {}", result.file_path.display());
            }
        }
        if !result.errors.is_empty() {
            total_errors += result.errors.len();
            eprintln!("Errors in {}:", result.file_path.display());
            for error in &result.errors {
                eprintln!("  - {error}");
            }
        }
    }

    println!(
        "\nProcessed {} files, {} modified, {} errors",
        results.len(),
        total_modified,
        total_errors
    );

    if cli_arg.check && total_modified > 0 {
        eprintln!("\nSome files need formatting. Run without --check to format them.");
        process::exit(1);
    }

    if total_errors > 0 {
        process::exit(1);
    }

    Ok(())
}
```

**Verification Commands**:
```bash
cargo build --bin cargo-rustdoc-fmt    # Should build successfully
cargo run --bin cargo-rustdoc-fmt -- --help    # Show help
```

---

### **Phase 7: Testing & Documentation** (Step 15)

#### Step 15: Create Test Fixtures and Complete Testing

**Test fixtures** in `src/cargo_rustdoc_fmt/test_data/`:

**File**: `sample_table.rs`
```rust
//! Example with a table
//!
//! | Aspect | Output | Input |
//! |---|---|---|
//! | Protocol | ansi/generator/ | ansi/parser/ |
//! | Very Long Column | This is a longer description | Another description |

fn main() {}
```

**File**: `sample_links.rs`
```rust
//! See [Rust docs](https://doc.rust-lang.org) for more info.
//! Also check [GitHub](https://github.com).

/// This function does something useful.
/// Read more at [the official docs](https://example.com/docs).
fn example() {}
```

**File**: `sample_complex.rs`
```rust
//! Complex example with both tables and links.
//!
//! | Feature | Link |
//! |---|---|
//! | Rust | [Homepage](https://rust-lang.org) |
//! | Docs | [Reference](https://doc.rust-lang.org) |
//!
//! See the [documentation](https://docs.rs) for detailed API reference.

fn main() {}
```

**Run comprehensive tests**:
```bash
# Test all modules
cargo test --all-targets

# Test with output
cargo test --all-targets -- --nocapture

# Run specific test
cargo test --lib cargo_rustdoc_fmt::extractor::tests::test_extract_outer_comments

# Build binary
cargo build --release --bin cargo-rustdoc-fmt

# Show binary size
ls -lh target/release/cargo-rustdoc-fmt
```

---

## Final Verification Checklist

After completing all 15 steps, run the complete verification suite:

```bash
cd /home/nazmul/github/roc-alt

# 1. Type checking
cargo check --all-targets
cargo check --workspace  # Make sure it works in workspace context

# 2. Build
cargo build --all-targets

# 3. Linting
cargo clippy --all-targets -- -D warnings

# 4. Tests
cargo test --all-targets
cargo test --lib              # Unit tests only
cargo test --doc              # Doc tests

# 5. Documentation
cargo doc --no-deps
# cargo doc --no-deps --open   # Open in browser

# 6. Install
cargo install --path r3bl-build-infra

# 7. Test binary
cargo-rustdoc-fmt --help
cargo-rustdoc-fmt --check --verbose

# 8. Test on real files
cd /home/nazmul/github/roc-alt
cargo rustdoc-fmt --check --verbose tui/src/lib.rs
```

---

## Adding Tool #2: Zero Refactoring Required

When ready to add the second tool (e.g., `cargo-analyze-deps`):

**Step 1: Create binary entry point**
```bash
touch src/bin/cargo-analyze-deps.rs
```

**Step 2: Create tool module**
```bash
mkdir -p src/cargo_analyze_deps
touch src/cargo_analyze_deps/mod.rs
touch src/cargo_analyze_deps/cli_arg.rs
```

**Step 3: Update Cargo.toml**
```toml
[[bin]]
name = "cargo-analyze-deps"
path = "src/bin/cargo-analyze-deps.rs"
```

**That's it!** Everything else (lib.rs, common/, workspace_utils/) is already reusable.

---

**Status**: ✅ Ready for implementation
**Phase**: Phase 1 (pulldown-cmark)
**Next Steps**: Implement 15 steps in order, test after each phase, then hand off to next developer
**Future**: Migrate to r3bl_tui::md_parser when table support is added

**Next Developer**:
- Review this entire document first
- Follow the 15 steps in order
- Run verification commands after each step
- Ask questions in the issue tracker if unclear
- Update this document as you complete each step
- Pay special attention to the cmdr/ patterns section for context on the architecture

---

## Test Infrastructure Improvements (Completed)

### Test Structure Enhancement

The project now uses a professional two-tier test approach:

#### Unit Tests (30 tests)
Located in module source files (`#[cfg(test)]` sections):
- Basic functionality tests with simple assertions
- Quick feedback during development
- No file I/O required

#### Validation Tests (6 tests)
Professional input/expected_output pattern:
```
src/cargo_rustdoc_fmt/test_data/
├── table_formatting/
│   ├── input/
│   │   ├── 01_basic_table.rs
│   │   ├── 02_wide_table.rs
│   │   ├── 03_multicolumn_table.rs
│   │   ├── 04_terminal_modes.rs
│   │   └── 05_unicode_table.rs
│   └── expected_output/
│       └── (matching .rs files with expected formatted content)
└── link_conversion/
    ├── input/
    │   ├── 01_simple_links.rs
    │   ├── 02_rustdoc_links.rs
    │   ├── 03_mixed_links.rs
    │   └── 04_duplicate_urls.rs
    └── expected_output/
        └── (matching .rs files with expected converted content)
```

Key improvements:
- **Exact output matching**: Tests use `assert_eq!()` instead of vague `assert!(contains)`
- **Real-world examples**: Test data extracted from actual r3bl-open-core code
- **Markdown structure preservation**: Tests verify lists, headings, paragraphs are maintained
- **Unicode support**: Tests include emoji, grapheme clusters, and special characters
- **Comprehensive coverage**:
  - Simple cases (basic tables/links)
  - Complex cases (multi-column tables, wide content)
  - Unicode cases (emoji, terminal escape sequences)
  - Mixed content (rustdoc + HTTP links, tables + lists)

#### Integration Tests (included in tests/integration_tests.rs)
End-to-end tests using temporary files:
- FileProcessor functionality
- Check-only mode behavior
- Full workflow from file read to write

### Markdown Structure Handling

The link_converter now properly preserves markdown structure:
- **Paragraphs**: Maintains blank line separation
- **Lists**: Preserves bullet points and indentation
- **Headings**: Maintains heading levels and formatting
- **Code blocks**: Doesn't convert links/format tables inside code
- **Emphasis**: Preserves bold, italic, and strikethrough

This is achieved through comprehensive event handling in the markdown parser rebuild function.

### Test Data Organization

All test fixtures follow a consistent pattern:
- **Input files**: Plain markdown content (no `///` or `//!` comment markers)
  - This matches what the formatters actually receive from the extractor
- **Expected output files**: Formatted markdown with exact spacing and structure
  - Generated by running formatters and capturing actual output
  - Used for regression testing

Example workflow:
```bash
# Run formatters on input files
cargo run --bin cargo-rustdoc-fmt -- --check src/cargo_rustdoc_fmt/test_data/table_formatting/input/

# Capture the formatted output
# Compare against expected_output files

# Validation tests verify exact match
cargo test --lib cargo_rustdoc_fmt::table_formatter::validation_tests
```

### Lessons Learned

1. **Input files should match formatter input**: Don't include comment markers in test data - the formatter receives plain text from the extractor.

2. **Output precision matters**: Using exact matching (`assert_eq!`) catches subtle formatting issues that vague assertions miss.

3. **Markdown structure is important**: Preserving lists, headings, and paragraph structure is critical for documentation quality.

4. **Test data is documentation**: Well-organized test fixtures serve as examples of tool usage and expected behavior.
