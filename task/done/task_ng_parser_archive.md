<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Archiving MD Parser NG and Simple Parser](#archiving-md-parser-ng-and-simple-parser)
  - [Archive Location](#archive-location)
  - [Overview](#overview)
  - [Implementation plan](#implementation-plan)
    - [Step 1: Analysis and Design [COMPLETE]](#step-1-analysis-and-design-complete)
    - [Step 2: Implementation [COMPLETE]](#step-2-implementation-complete)
    - [Step 3: Testing and Validation [COMPLETE]](#step-3-testing-and-validation-complete)
  - [Development Timeline](#development-timeline)
    - [Phase 1: NG Parser Development](#phase-1-ng-parser-development)
    - [Phase 2: Simple Parser Development](#phase-2-simple-parser-development)
  - [Performance Analysis](#performance-analysis)
    - [Benchmark Results](#benchmark-results)
    - [Key Findings](#key-findings)
  - [Implementation Status](#implementation-status)
    - [NG Parser (nom-based)](#ng-parser-nom-based)
    - [Simple Parser](#simple-parser)
    - [Known Issues](#known-issues)
  - [Strategic Decision: Archive Both Parsers](#strategic-decision-archive-both-parsers)
    - [Rationale](#rationale)
  - [Archival Plan](#archival-plan)
    - [Repository Structure](#repository-structure)
    - [Dependency Management](#dependency-management)
    - [Migration Steps](#migration-steps)
  - [Migration Status](#migration-status)
    - [Files to Archive](#files-to-archive)
    - [Files to Keep in r3bl-open-core](#files-to-keep-in-r3bl-open-core)
    - [Detailed Migration Execution](#detailed-migration-execution)
  - [Lessons Learned](#lessons-learned)
  - [Future Recommendations](#future-recommendations)
  - [Final Migration Status (July 15, 2025)](#final-migration-status-july-15-2025)
    - [[COMPLETE] All Tasks Completed](#complete-all-tasks-completed)
    - [[COMPLETE] Post-Migration Fixes Completed (July 15, 2025)](#complete-post-migration-fixes-completed-july-15-2025)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Archiving MD Parser NG and Simple Parser

## Archive Location

- Repository: [r3bl-open-core-archive](https://github.com/r3bl-org/r3bl-open-core-archive)
- Crate: `md_parser_ng`
- Commit SHA for r3bl_tui dependency: `fe1182a0f6c40f38852f2204b9895bef546aeed7` (2025-05-24)
- Edition: 2024
- nom version: 8.0.0

## Overview

This document summarizes the development, performance analysis, and archival of two experimental
markdown parsers:

- **NG Parser**: nom-based parser with virtual array abstraction
- **Simple Parser**: Direct string manipulation without nom

Both parsers were developed as potential replacements for the legacy parser but ultimately archived
in favor of retaining the mature, battle-tested legacy implementation.

## Implementation plan

This task has been completed successfully. This section documents the completed work.

### Step 1: Analysis and Design [COMPLETE]

Analyzed requirements and designed the solution.

- [x] Conduct research and analysis
- [x] Design implementation approach
- [x] Document findings and recommendations

### Step 2: Implementation [COMPLETE]

Implemented the solution as designed.

- [x] Write core implementation
- [x] Add supporting utilities
- [x] Integrate with existing code

### Step 3: Testing and Validation [COMPLETE]

Tested and validated the implementation.

- [x] Write unit tests
- [x] Perform integration testing
- [x] Document results

## Development Timeline

### Phase 1: NG Parser Development

- Implemented using nom parser combinators
- Created `AsStrSlice` virtual array abstraction for `&[GCString]`
- Complete markdown parsing implementation
- **Result**: 600-5,000x slower than legacy parser due to abstraction overhead

### Phase 2: Simple Parser Development

- Dropped nom dependency entirely
- Direct string manipulation approach
- ~1,000 lines of straightforward code
- **Result**: Performance comparable to legacy parser (within 25%)

## Performance Analysis

### Benchmark Results

| Parser | Small Content | Medium Content | Large Content |
| ------ | ------------- | -------------- | ------------- |
| Legacy | ~2.6-24K ns   | ~72-196K ns    | ~196K ns      |
| Simple | ~2-25K ns     | ~76-243K ns    | ~243K ns      |
| NG/nom | ~19K-1.4M ns  | ~44M-685M ns   | ~686M ns      |

### Key Findings

- Simple parser achieved 600-5,000x performance improvement over NG parser
- Simple parser performance was comparable to legacy parser (within 25%)
- Virtual array abstraction in NG parser caused massive overhead
- Direct string operations proved most efficient

## Implementation Status

### NG Parser (nom-based)

- [COMPLETE] Complete implementation with all markdown features
- [BLOCKED] Severe performance issues due to AsStrSlice overhead
- [BLOCKED] Complex code with nom combinators

### Simple Parser

- [COMPLETE] Complete implementation (~1,000 lines)
- [COMPLETE] 46/52 compatibility tests passing
- [COMPLETE] Performance comparable to legacy parser
- [BLOCKED] 6 edge cases with non-standard markdown handling

### Known Issues

1. **Markdown Dialect Conflicts**: R3BL uses non-standard conventions:
   - `*text*` for bold (standard: italic)
   - `_text_` for italic (standard: also italic)
   - Standard markdown uses `**text**` for bold

2. **Edge Cases**: Special character handling and complex nested structures

## Strategic Decision: Archive Both Parsers

### Rationale

1. **Legacy Parser Wins**:
   - Already mature and battle-tested
   - Performance is excellent for real-world use
   - Zero migration risk
   - All edge cases understood

2. **Simple Parser Learnings**:
   - Proved nom overhead was unnecessary
   - Validated direct string manipulation approach
   - ~25% performance difference doesn't justify migration risk

3. **NG Parser Learnings**:
   - Virtual array abstractions can be extremely costly
   - Parser combinators add significant overhead for this use case
   - Complexity doesn't always equal better performance

## Archival Plan

### Repository Structure

```
r3bl-open-core-archive/
├── md_parser_ng/
│   ├── Cargo.toml (with pinned dependencies)
│   ├── Cargo.lock
│   ├── README.md
│   ├── rustfmt.toml
│   ├── rust-toolchain.toml
│   ├── .gitignore
│   ├── src/
│   │   ├── mod.rs (main lib entry point)
│   │   ├── parse_markdown_ng.rs
│   │   ├── parse_markdown_simple.rs
│   │   ├── local_constants.rs
│   │   ├── local_types.rs
│   │   ├── as_str_slice/
│   │   │   ├── mod.rs
│   │   │   ├── as_str_slice_core.rs
│   │   │   ├── cache.rs
│   │   │   ├── lazy_cache.rs
│   │   │   ├── compat.rs
│   │   │   ├── iterators.rs
│   │   │   ├── traits/
│   │   │   │   ├── compare.rs
│   │   │   │   ├── conversions.rs
│   │   │   │   ├── display_and_debug.rs
│   │   │   │   ├── find_substring.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── nom_input.rs
│   │   │   │   └── offset.rs
│   │   │   ├── operations/
│   │   │   │   ├── character_ops.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── search_and_split.rs
│   │   │   │   ├── text_extraction.rs
│   │   │   │   └── trim_whitespace.rs
│   │   │   └── position/
│   │   │       ├── advance_with_synthetic_new_line.rs
│   │   │       ├── line_advancement.rs
│   │   │       └── mod.rs
│   │   ├── block_ng/
│   │   │   ├── mod.rs
│   │   │   ├── parse_block_code_ng.rs
│   │   │   └── parse_block_smart_list_ng.rs
│   │   ├── extended_ng/
│   │   │   ├── mod.rs
│   │   │   ├── parse_metadata_k_csv_ng.rs
│   │   │   ├── parse_metadata_k_v_ng.rs
│   │   │   └── parser_take_text_until_eol_or_eoi_ng.rs
│   │   ├── fragment_ng/
│   │   │   ├── mod.rs
│   │   │   ├── parse_fragments_in_a_line_ng.rs
│   │   │   ├── plain_parser_catch_all_ng.rs
│   │   │   ├── specialized_parsers_ng.rs
│   │   │   └── take_text_between_ng.rs
│   │   ├── standard_ng/
│   │   │   ├── mod.rs
│   │   │   ├── parse_heading_ng.rs
│   │   │   └── parse_markdown_text_including_eol_or_eoi_ng.rs
│   │   └── compat_test_data/
│   │       ├── mod.rs
│   │       ├── invalid_inputs.rs
│   │       ├── valid_small_inputs.rs
│   │       ├── valid_medium_inputs.rs
│   │       ├── valid_large_inputs.rs
│   │       ├── valid_jumbo_inputs.rs
│   │       └── real_world_files/
│   │           ├── ex_editor.md
│   │           ├── jumbo_api_documentation.md
│   │           ├── large_complex_document.md
│   │           ├── medium_blog_post.md
│   │           └── small_quick_start.md
│   ├── benches/
│   │   └── benchmark_parsers.rs
│   ├── tests/
│   │   └── compat_test_suite.rs
│   └── docs/
│       ├── ng_parser_simple_drop_nom.md
│       ├── ng_parser_virtual_array.md
│       └── parser_strategy_analysis.md
```

### Dependency Management

Use git SHA to pin exact r3bl_tui version:

```toml
[dependencies]
r3bl_tui = { git = "https://github.com/r3bl-org/r3bl-open-core.git", rev = "fe1182a0f6c40f38852f2204b9895bef546aeed7" } # 2025-05-24
```

### Migration Steps

1. [COMPLETE] Get current commit SHA before removing code:
   `fe1182a0f6c40f38852f2204b9895bef546aeed7` (2025-05-24)
2. [COMPLETE] Create the `md_parser_ng` crate directory at
   `/home/nazmul/github/r3bl-open-core-archive/md_parser_ng`
3. [COMPLETE] Add `md_parser_ng` to the workspace in
   `/home/nazmul/github/r3bl-open-core-archive/Cargo.toml`
4. [COMPLETE] Copy `tui/src/tui/md_parser_ng/` folder contents to archive repo `src/` directory
5. [COMPLETE] Copy all the `compat_test_data` files to archive repo (maintain structure)
6. [COMPLETE] Extract NG/Simple parser-specific tests and benchmarks:
   - [COMPLETE] Move `bench_test_suite.rs` to `benches/benchmark_parsers.rs`
   - [COMPLETE] Move `compat_test_suite.rs` and `debug_blog_post_test.rs` to `tests/`
   - [COMPLETE] Copy `debug_parser_outputs.rs` to archive repo
7. [COMPLETE] Create Cargo.toml for md_parser_ng crate with pinned dependencies (edition 2024, nom
   8.0.0)
8. [COMPLETE] Add comprehensive README documenting rationale
9. [COMPLETE] Copy/Move documentation files:
   - [COMPLETE] Move `docs/ng_parser_simple_drop_nom.md` →
     `/home/nazmul/github/r3bl-open-core-archive/md_parser_ng/docs/`
   - [COMPLETE] Move `docs/ng_parser_virtual_array.md` →
     `/home/nazmul/github/r3bl-open-core-archive/md_parser_ng/docs/`
   - [COMPLETE] Copy `docs/parser_strategy_analysis.md` →
     `/home/nazmul/github/r3bl-open-core-archive/md_parser_ng/docs/` (kept in both repos)
10. [COMPLETE] Remove NG parser code, tests, and benchmarks from r3bl-open-core
11. [COMPLETE] Update legacy parser documentation in r3bl-open-core
12. [COMPLETE] Fix all compiler errors in r3bl-open-core (801 tests passing)

## Migration Status

[COMPLETE] **Migration Complete** (Date: 2025-07-15)

The NG parser module has been successfully:

1. Copied to the r3bl-open-core-archive repository as the `md_parser_ng` crate
2. Removed from r3bl-open-core along with all references
3. Legacy parser is now the only parser used in r3bl-open-core
4. All compiler errors in r3bl-open-core have been resolved (801 tests passing)

Note: The archived md_parser_ng crate has compilation issues due to API changes and missing type
exports from r3bl_tui. This is intentional - the code is preserved as-is for historical accuracy
rather than being modified to compile against current APIs.

### Files to Archive

#### Core NG Parser Module

- `tui/src/tui/md_parser_ng/` (entire directory)

#### Documentation Files

- `docs/ng_parser_virtual_array.md`
- `docs/ng_parser_simple_drop_nom.md`
- `docs/parser_strategy_analysis.md`

#### Test Files

- `tui/tests/debug_parser_outputs.rs` (if it contains NG parser tests)
- Extract NG/Simple parser tests from `tui/src/tui/md_parser/snapshot_tests.rs`

### Files to Keep in r3bl-open-core

- `compat_test_data/` folder (needed by legacy parser tests)
- Legacy parser tests and benchmarks
- Legacy parser implementation
- `docs/parser_strategy_analysis.md` (useful for future parser development)

### Detailed Migration Execution

#### Phase 1: Setup Archive Repository Structure

1. Create directory: `/home/nazmul/github/r3bl-open-core-archive/md_parser_ng`
2. Update workspace Cargo.toml to include new crate:
   ```toml
   [workspace]
   members = [
       # ... existing members ...
       "md_parser_ng",
   ]
   ```

#### Phase 2: Create Crate Structure

Create the following structure in archive repo:

```
/home/nazmul/github/r3bl-open-core-archive/md_parser_ng/
├── Cargo.toml
├── README.md
├── src/
├── tests/
├── benches/
└── docs/
```

#### Phase 3: Copy Source Files

- Copy all contents from `tui/src/tui/md_parser_ng/` to `md_parser_ng/src/`
- Maintain the module structure (as_str_slice/, block_ng/, etc.)

#### Phase 4: Handle Dependencies

The md_parser_ng crate will need dependencies from r3bl_tui, pinned to the specific commit SHA to
ensure compatibility.

## Lessons Learned

1. **Premature Optimization**: The legacy parser was already fast enough
2. **Abstraction Cost**: Virtual arrays and parser combinators can add massive overhead
3. **Simplicity Wins**: Direct string manipulation is often the best approach
4. **Migration Risk**: Rewriting working code needs exceptional benefits to justify

## Future Recommendations

Instead of rewriting the parser, invest in:

1. **Legacy Parser Optimizations**:
   - Profile the `&[GCString] → String` conversion
   - Look for algorithmic improvements
   - Add SIMD optimizations if beneficial

2. **Higher-Impact Features**:
   - Better error reporting
   - Streaming parser for very large documents
   - Parser caching/memoization
   - Additional markdown features

3. **Large Document Handling** (>10MB):
   - Incremental parsing
   - Viewport-based rendering
   - Background parsing threads
   - Chunked processing

## Final Migration Status (July 15, 2025)

### [COMPLETE] All Tasks Completed

- Created `md_parser_ng` crate in `r3bl-open-core-archive` repository
- Migrated all NG and Simple parser code with proper structure
- Moved all tests and benchmarks specific to NG/Simple parsers
- Created comprehensive documentation
- Updated `parser_strategy_analysis.md` with prominent archival notice
- Set up proper dependencies with pinned r3bl_tui version
- Removed all NG parser code from r3bl-open-core
- Removed NG/Simple parser-specific tests from r3bl-open-core
- Updated legacy parser documentation
- Fixed all compiler errors and warnings (801 tests passing)

### [COMPLETE] Post-Migration Fixes Completed (July 15, 2025)

After the initial migration, the following issues were identified and resolved:

1. **Restored Legacy Parser Testing Infrastructure**:
   - Copied back `compat_test_data/` folder (now renamed to `conformance_test_data/`)
   - Restored comprehensive benchmark tests (`parser_bench_tests.rs`)
   - Restored snapshot tests (`parser_snapshot_tests.rs`)
   - All 155 tests passing, including 48 snapshot tests

2. **File Renaming for Clarity**:
   - `compat_test_data/` → `conformance_test_data/`
   - `snapshot_tests.rs` → `parser_snapshot_tests.rs`
   - `benchmark_legacy_parser.rs` → `parser_bench_tests.rs`

3. **Fixed Archive Repository Configuration**:
   - Updated nom version to 8.0.0 as requested
   - Ensured correct SHA pinning: `fe1182a0f6c40f38852f2204b9895bef546aeed7` (2025-05-24). This
     commit is before `AsStrSlice` and lots of other files are added.
   - Edition set to 2024

4. **Benchmark Infrastructure**:
   - Confirmed `cargo bench` works correctly with nightly toolchain
   - Added comprehensive documentation on running benchmarks
   - All benchmarks use the same conformance test data as snapshot tests
   - Fixed all warnings by using `let _unused = ...` pattern instead of `let _ = ...`

5. **Test Coverage Verification**:
   - Verified ALL 48 test data constants from `conformance_test_data` are used
   - Both `parser_snapshot_tests.rs` and `parser_bench_tests.rs` use identical test data
   - 100% coverage of available test constants with no unused data
   - Test categories include:
     - 23 small input tests (basic markdown elements)
     - 21 medium input tests (complex structures)
     - 2 large input tests (real-world documents)
     - 2 invalid input tests (malformed syntax)
     - Multiple real-world content tests

6. **Final Build Verification**:
   - `cargo test --package r3bl_tui --lib`: All 801 library tests pass
   - `cargo bench --package r3bl_tui`: All 49 benchmarks run without warnings
   - No compilation warnings or errors

Both experimental parsers have been successfully archived in `r3bl-open-core-archive` for historical
reference and learning purposes.
