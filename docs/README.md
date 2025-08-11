<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Documentation for r3bl-open-core](#documentation-for-r3bl-open-core)
  - [Contents](#contents)
    - [Release Guide](#release-guide)
    - [Contributing Guides](#contributing-guides)
    - [Technical Documentation and Design Docs](#technical-documentation-and-design-docs)
    - [Plans](#plans)
    - [Completed Plans](#completed-plans)
    - [Video Documentation](#video-documentation)
  - [Creating and Managing Documentation](#creating-and-managing-documentation)
    - [Documentation Updates](#documentation-updates)
    - [Video Documentation](#video-documentation-1)
  - [Archived Content](#archived-content)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

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

### Technical Documentation and Design Docs

- [`CLAUDE.md`](CLAUDE.md) - Claude AI integration documentation
- [`dd_parser_conformance.md`](dd_parser_conformance.md) - Parser conformance testing documentation

### Plans

- [`task_dual_channel_pty.md`](task_dual_channel_pty.md) - Task documentation for dual channel PTY
  implementation
- [`task_remove_crossterm.md`](task_remove_crossterm.md) - Task documentation for removing crossterm
  dependency
- [`task_syntect_improve.md`](task_syntect_improve.md) - Syntect improvement plan for adding missing
  language support
- [`task_textwrap_rewrite.md`](task_textwrap_rewrite.md) - Task to rewrite textwrap in TUI codebase
- [`task_tui_perf_optimize.md`](task_tui_perf_optimize.md) - TUI performance optimization guidelines
- [`task_unify_rendering.md`](task_unify_rendering.md) - Task to unify ASText and TuiStyledText
  rendering paths

### Completed Plans

The [`done`](done) folder contains:

- [`task_ng_parser_archive.md`](done/task_ng_parser_archive.md) - Complete documentation of the NG
  Parser and Simple Parser archival process, including performance analysis, migration status, and
  lessons learned from experimental parser development
- [`task_parser_strategy_analysis.md`](done/task_parser_strategy_analysis.md) - Analysis of parser
  implementation strategies that led to archiving experimental parsers (NG and Simple) and keeping
  the legacy parser as the only production implementation
- [`task_refactor_nu.md`](done/task_refactor_nu.md) - Comprehensive consolidation of three separate
  run.nu files into a unified development script, including bootstrap.sh creation, cross-platform
  file watching implementation, and audience-specific documentation refactoring
- [`task_test_pty.md`](done/task_test_pty.md) - Testing strategies and implementation approaches for
  PTY-based OSC sequence capture, including unit tests, integration tests, and platform
  considerations
- [`task_unified_grapheme_trait.md`](done/task_unified_grapheme_trait.md) - Universal grapheme-aware
  trait design documentation for both single-line and multi-line text structures
- [`task_zero_copy_gap_buffer.md`](done/task_zero_copy_gap_buffer.md) - Zero-copy gap buffer
  implementation for editor content storage, successfully eliminating string materialization in the
  markdown parser path with proven performance improvements

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
