# r3bl-build-infra

## Why R3BL?

<img src="https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/r3bl-term.svg?raw=true" height="256px">

<!-- R3BL TUI library & suite of apps focused on developer productivity -->

<span style="color:#FD2F53">R</span><span style="color:#FC2C57">3</span><span
style="color:#FB295B">B</span><span style="color:#FA265F">L</span><span
style="color:#F92363"> </span><span style="color:#F82067">T</span><span
style="color:#F61D6B">U</span><span style="color:#F51A6F">I</span><span
style="color:#F31874"> </span><span style="color:#F11678">l</span><span
style="color:#EF137C">i</span><span style="color:#ED1180">b</span><span
style="color:#EB0F84">r</span><span style="color:#E90D89">a</span><span
style="color:#E60B8D">r</span><span style="color:#E40A91">y</span><span
style="color:#E10895"> </span><span style="color:#DE0799">&amp;</span><span
style="color:#DB069E"> </span><span style="color:#D804A2">s</span><span
style="color:#D503A6">u</span><span style="color:#D203AA">i</span><span
style="color:#CF02AE">t</span><span style="color:#CB01B2">e</span><span
style="color:#C801B6"> </span><span style="color:#C501B9">o</span><span
style="color:#C101BD">f</span><span style="color:#BD01C1"> </span><span
style="color:#BA01C4">a</span><span style="color:#B601C8">p</span><span
style="color:#B201CB">p</span><span style="color:#AE02CF">s</span><span
style="color:#AA03D2"> </span><span style="color:#A603D5">f</span><span
style="color:#A204D8">o</span><span style="color:#9E06DB">c</span><span
style="color:#9A07DE">u</span><span style="color:#9608E1">s</span><span
style="color:#910AE3">e</span><span style="color:#8D0BE6">d</span><span
style="color:#890DE8"> </span><span style="color:#850FEB">o</span><span
style="color:#8111ED">n</span><span style="color:#7C13EF"> </span><span
style="color:#7815F1">d</span><span style="color:#7418F3">e</span><span
style="color:#701AF5">v</span><span style="color:#6B1DF6">e</span><span
style="color:#6720F8">l</span><span style="color:#6322F9">o</span><span
style="color:#5F25FA">p</span><span style="color:#5B28FB">e</span><span
style="color:#572CFC">r</span><span style="color:#532FFD"> </span><span
style="color:#4F32FD">p</span><span style="color:#4B36FE">r</span><span
style="color:#4739FE">o</span><span style="color:#443DFE">d</span><span
style="color:#4040FE">u</span><span style="color:#3C44FE">c</span><span
style="color:#3948FE">t</span><span style="color:#354CFE">i</span><span
style="color:#324FFD">v</span><span style="color:#2E53FD">i</span><span
style="color:#2B57FC">t</span><span style="color:#285BFB">y</span>

## Table of contents

<!-- TOC -->

- [Introduction](#introduction)
- [cargo-rustdoc-fmt](#cargo-rustdoc-fmt)
  - [Features](#features)
  - [Installation](#installation)
  - [Usage Examples](#usage-examples)
  - [What It Does](#what-it-does)
  - [Git Integration](#git-integration)
  - [CI Integration](#ci-integration)
  - [Architecture](#architecture)
  - [Implementation Notes](#implementation-notes)
- [Changelog](#changelog)
- [Learn how these crates are built, provide
  feedback](#learn-how-these-crates-are-built-provide-feedback)

<!-- /TOC -->

## Introduction

Build tools and utilities designed for R3BL projects, but usable in any Rust project.

Please read the
main [README.md] of
the `r3bl-open-core` monorepo and workspace to get a better understanding of the
context in which this crate is meant to exist.

## cargo-rustdoc-fmt

A cargo subcommand that formats markdown tables and converts inline links to
reference-style links within Rust documentation comments (`///` and `//!`).

### Features

- **Table Formatting**: Aligns markdown table columns for readability
- **Link Conversion**: Converts inline markdown links to reference-style links,
  keeping documentation cleaner
- **Workspace Support**: Process entire Rust workspaces or specific files
- **Check Mode**: Verify formatting without modifying files (useful for CI)
- **Selective Formatting**: Choose to format only tables, only links, or both
- **Git Integration**: Auto-detects changed files in git working tree

### Installation

From [crates.io]:

```bash
cargo install r3bl-build-infra
```

Or from source (in a workspace containing this crate):

```bash
cargo install --path build-infra
```

### Usage Examples

**Format git-changed files** (default - auto-detects staged/unstaged changes):
```bash
cargo rustdoc-fmt
```

**Format entire workspace**:
```bash
cargo rustdoc-fmt --workspace
```

**Format specific files**:
```bash
cargo rustdoc-fmt src/lib.rs src/main.rs
```

**Format a directory**:
```bash
cargo rustdoc-fmt src/
```

**Check formatting without modifying** (useful for CI):
```bash
cargo rustdoc-fmt --check
```

**Only format tables** (skip link conversion):
```bash
cargo rustdoc-fmt --tables-only
```

**Only convert links** (skip table formatting):
```bash
cargo rustdoc-fmt --links-only
```

**Verbose output**:
```bash
cargo rustdoc-fmt --verbose
```

**Combine options**:
```bash
cargo rustdoc-fmt --check --verbose src/
```

### What It Does

#### Table Formatting

Markdown tables in rustdoc comments are reformatted with consistent column widths.

**Before:**
```rust
//! | A | B |
//! |---|---|
//! | Short | Very Long Text |
```

**After:**
```rust
//! | A     | B              |
//! |-------|----------------|
//! | Short | Very Long Text |
```

#### Link Conversion

Inline markdown links are converted to reference-style links using the link text
as the reference identifier, reducing visual clutter in documentation.

**Before:**
```rust
//! See [docs](https://example.com) and [Rust](https://rust-lang.org).
```

**After:**
```rust
//! See [docs] and [Rust].
//!
//! [docs]: https://example.com
//! [Rust]: https://rust-lang.org
```

### Git Integration

When run without arguments, `cargo-rustdoc-fmt` intelligently determines which files
to format:

1. **If there are staged/unstaged changes**: Formats only those changed files
2. **If working tree is clean**: Formats files from the most recent commit
3. **If not in a git repository**: Formats the entire workspace

This makes it perfect for pre-commit hooks and development workflows.

### CI Integration

Add to your continuous integration pipeline to enforce formatting standards:

```bash
cargo rustdoc-fmt --check
```

Exits with code 1 if formatting is needed, allowing CI to fail the build.

**Example GitHub Actions step**:

```yaml
- name: Check rustdoc formatting
  run: cargo rustdoc-fmt --check --verbose
```

### Architecture

The project follows a multi-tool design pattern (similar to the `cmdr/` crate).
Currently implements `cargo-rustdoc-fmt`, with support for adding additional
build tools in the future without refactoring.

**Module structure:**
- `src/lib.rs` - Library root
- `src/bin/cargo-rustdoc-fmt.rs` - Binary entry point
- `src/cargo_rustdoc_fmt/` - Tool implementation
  - `cli_arg.rs` - CLI argument parsing
  - `extractor.rs` - Extract rustdoc blocks from source
  - `table_formatter.rs` - Format markdown tables
  - `link_converter.rs` - Convert inline to reference-style links
  - `processor.rs` - Orchestrate file processing
  - `ir_event_types` - Type definitions
  - `ui_str.rs` - User-facing messages
- `src/common/` - Shared utilities
  - `git_utils.rs` - Git integration
  - `workspace_utils.rs` - Workspace discovery and file finding

### Implementation Notes

Currently uses `pulldown-cmark` for markdown parsing. This will be migrated to
`r3bl_tui::md_parser` once table support is added to that parser, achieving full
R3BL infrastructure dogfooding.

## Changelog

Please check out the
[changelog] to
see how the crate has evolved over time.

## Learn how these crates are built, provide feedback

To learn how we built this crate, please take a look at the following resources.
- If you like consuming video content, here's our [YT channel].
  Please consider [subscribing].
- If you like consuming written content, here's our developer [site].

[changelog]: https://github.com/r3bl-org/r3bl-open-core/blob/main/CHANGELOG.md#r3bl-build-infra
[crates.io]: https://crates.io/crates/r3bl-build-infra
[README.md]: https://github.com/r3bl-org/r3bl-open-core/blob/main/README.md
[site]: https://developerlife.com/
[subscribing]: https://www.youtube.com/channel/CHANNEL_ID?sub_confirmation=1
[YT channel]: https://www.youtube.com/@developerlifecom

License: Apache-2.0
