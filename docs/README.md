# Documentation for r3bl-open-core

This folder contains documentation, guides, and resources for the r3bl-open-core monorepo.

## Contents

### Release Guide

[`release-guide.md`](release-guide.md) - Comprehensive guide for releasing crates in this workspace,
including:

- Step-by-step release procedures for each crate
- Version update workflows
- Publishing to crates.io
- Git tagging conventions
- Deprecated workflows for archived crates

### Contributing Guides

The [`contributing_guides`](contributing_guides) folder contains templates and guidelines for:

- [`BRANCH.md`](contributing_guides/BRANCH.md) - Branch naming and management guidelines
- [`COMMIT_MESSAGE.md`](contributing_guides/COMMIT_MESSAGE.md) - Commit message conventions
- [`ISSUE.md`](contributing_guides/ISSUE.md) - Issue reporting guidelines
- [`PULL_REQUEST.md`](contributing_guides/PULL_REQUEST.md) - Pull request submission guidelines
- [`STYLE_GUIDE.md`](contributing_guides/STYLE_GUIDE.md) - Code style and formatting guidelines

### Technical Documentation

- [`parser_strategy_analysis.md`](parser_strategy_analysis.md) - Analysis of parser implementation
  strategies

### Plans

- [`task_tui_perf_optimize.md`](task_tui_perf_optimize.md) - TUI performance optimization guidelines
- [`ng_parser_archive.md`](ng_parser_archive.md) - Archived parser documentation

### Video Documentation

The [`video`](video) folder contains:

- [`r3bl_terminal_async_clip_ffmpeg.gif`](video/r3bl_terminal_async_clip_ffmpeg.gif) - Terminal
  async demonstration

## Creating and Managing Documentation

### Documentation Updates

When updating documentation:

1. Update relevant `.lib.rs` files. The corresponding `README.md` is automatically generated from
   these files using `cargo readme`, details are in the [`release-guide`](release-guide.md).
2. For crate documentation shown on crates.io and docs.rs:
   - `README.md` files use relative links
   - `lib.rs` files use absolute links to githubusercontent.com

### Video Documentation

For recording demos and tutorials:

- Use screen recording tools like Kooha on Linux
- Keep videos under 2 minutes (10MB GitHub limit)
- Save as MP4 or GIF format
- Upload directly to GitHub by dragging into issue/PR comments

## Archived Content

As the project evolves, deprecated documentation and crates are moved to the
[r3bl-open-core-archive](https://github.com/r3bl-org/r3bl-open-core-archive) repository for
historical reference.
