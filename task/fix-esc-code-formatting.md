# Task: Fix ESC Code Formatting in Rustdocs

## 1. Overview
- **Goal**: Standardize and fix the formatting of ANSI escape sequences in rustdoc comments across the workspace.
- **Problem**: There are over 260 instances where `[`ESC`]` is used inside a literal sequence in documentation comments. This mix of bracketed intra-doc links and loose text (e.g., `/// Cursor Up (CUU) - [`ESC`] [ n A.`) makes the sequences jarring and difficult to read.
- **Solution**: Convert these instances into fully wrapped markdown code blocks so the entire sequence renders as a single, readable monospaced block (e.g., `` `ESC [ n A` ``).

## 2. Examples of the Pattern

The problematic pattern appears widely in files such as `sequence.rs`, `performer.rs`, and others.

**Current (Confusing) Format:**
- `/// Cursor Up (CUU) - [`ESC`] [ n A.`
- `/// Erase Line (EL) - [`ESC`] [ n K.`
- `/// Enable Private Mode - [`ESC`] [ ? n h`

**Desired (Clean) Format:**
- ``/// Cursor Up (CUU) - `ESC [ n A`.``
- ``/// Erase Line (EL) - `ESC [ n K`.``
- ``/// Enable Private Mode - `ESC [ ? n h` ``

## 3. Scope of Work
- Perform a workspace-wide search for `[`ESC`]` followed by literal sequence characters (like `[`, `(`, etc.).
- Carefully refactor these comments to use inline code blocks for the entire sequence.
- Ensure that standalone `[`ESC`]` links (which refer to `crate::EscSequence` or similar) are preserved if they are correctly used as structural links rather than embedded inside a literal terminal sequence string.

## 4. Checklist
- [ ] Sweep `tui/src/core/ansi/` for all `[`ESC`]` string occurrences.
- [ ] Convert mixed link/literal sequences to inline code blocks.
- [ ] Run `./check.fish --doc` to ensure no intra-doc links were broken and everything renders beautifully.
- [ ] **Mandatory manual review**:
  - [ ] Target files to be populated during implementation.
