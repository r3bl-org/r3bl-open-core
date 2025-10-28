# Readline Async: Add Missing Keyboard Shortcuts

**Status:** TODO
**Priority:** Medium
**Assignee:** TBD
**Estimated Effort:** 3-4 weeks (phased implementation)

## Overview

The `readline_async` implementation currently supports 6 keyboard shortcuts. This task is to:
1. Reorganize existing integration tests for better discoverability
2. Implement 36 missing keyboard shortcuts used in standard shells (bash/zsh)
3. Achieve feature parity with common readline implementations

## Table of Contents

- [Current State](#current-state)
- [Problem Statement](#problem-statement)
- [Proposed Solution](#proposed-solution)
  - [Part 1: Reorganize Existing Tests](#part-1-reorganize-existing-tests)
  - [Part 2: Add Master Documentation](#part-2-add-master-documentation)
  - [Part 3: Implement Missing Shortcuts](#part-3-implement-missing-shortcuts)
- [Implementation Phases](#implementation-phases)
- [Acceptance Criteria](#acceptance-criteria)

---

## Current State

### Currently Implemented Shortcuts

| Shortcut | Action | Test File | Status |
|----------|--------|-----------|--------|
| Ctrl+D | EOF (empty line) or Delete char (non-empty) | `pty_ctrl_d_test.rs` | ✅ Implemented |
| Ctrl+U | Clear from cursor to start of line | `pty_ctrl_u_test.rs` | ✅ Implemented |
| Ctrl+Left | Move cursor to start of previous word | `pty_ctrl_navigation_test.rs` | ✅ Implemented |
| Ctrl+Right | Move cursor to start of next word | `pty_ctrl_navigation_test.rs` | ✅ Implemented |
| Alt+D | Delete word forward from cursor | `pty_alt_kill_test.rs` | ✅ Implemented |
| Alt+Backspace | Delete word backward from cursor | `pty_alt_kill_test.rs` | ✅ Implemented |

**Total: 6 shortcuts**

### Current Test Organization Problems

1. **Inconsistent categorization**:
   - `pty_ctrl_u_test.rs` tests line clearing (Ctrl+U) - standalone file
   - `pty_ctrl_navigation_test.rs` tests word navigation (Ctrl+Left/Right) - but doesn't include Ctrl+U
   - `pty_alt_kill_test.rs` - vague name, actually tests word deletion

2. **Missing documentation**:
   - No central overview of all keyboard shortcuts
   - Individual test rustdocs don't always list all shortcuts tested

3. **Poor discoverability**:
   - Hard to know where to add a new shortcut test
   - Hard to find which test covers a specific shortcut

---

## Problem Statement

Users expect standard readline keyboard shortcuts to work. Missing shortcuts reduce productivity and make the readline implementation feel incomplete. The current test organization also makes it difficult to:
- Know which shortcuts are implemented
- Find where to add new shortcut tests
- Avoid duplicate/overlapping tests

---

## Proposed Solution

### Part 1: Reorganize Existing Tests

**Current structure:**
```
integration_tests/
├── pty_ctrl_d_test.rs           # Ctrl+D (EOF vs delete)
├── pty_ctrl_navigation_test.rs  # Ctrl+Left/Right (word nav)
├── pty_ctrl_u_test.rs           # Ctrl+U (clear line)
├── pty_alt_kill_test.rs         # Alt+D, Alt+Backspace
├── pty_ctrl_d_eof_test.rs       # Ctrl+D EOF variant
└── pty_ctrl_d_delete_test.rs    # Ctrl+D delete variant
```

**Proposed structure:**
```
integration_tests/
├── mod.rs                              # Master table of all shortcuts
├── pty_ctrl_d_dual_behavior_test.rs    # Ctrl+D special case (EOF/delete)
├── pty_line_editing_test.rs            # Line-level operations (Ctrl+U, Ctrl+K, etc.)
├── pty_word_navigation_test.rs         # Word movement (Ctrl+Left/Right, Alt+B/F)
├── pty_word_editing_test.rs            # Word-level editing (Alt+D, Alt+Backspace, etc.)
├── pty_char_navigation_test.rs         # Character movement (arrows, Home/End, Ctrl+A/E/B/F)
├── pty_char_editing_test.rs            # Character editing (Backspace, Delete, Ctrl+H)
└── pty_history_navigation_test.rs      # History (Up/Down, Ctrl+P/N, Ctrl+R)
```

**File renaming:**
```
pty_ctrl_u_test.rs          → pty_line_editing_test.rs
pty_ctrl_navigation_test.rs → pty_word_navigation_test.rs
pty_alt_kill_test.rs        → pty_word_editing_test.rs
pty_ctrl_d_test.rs          → pty_ctrl_d_dual_behavior_test.rs
pty_ctrl_d_eof_test.rs      → DELETE (merge into dual_behavior)
pty_ctrl_d_delete_test.rs   → DELETE (merge into dual_behavior)
```

### Part 2: Add Master Documentation

Update `tui/src/readline_async/readline_async_impl/integration_tests/mod.rs` with a comprehensive table:

```rust
//! Integration tests for readline_async keyboard shortcuts.
//!
//! ## Keyboard Shortcuts Coverage
//!
//! This table shows all implemented keyboard shortcuts and which test file covers them:
//!
//! | Shortcut | Action | Test File | Status |
//! |----------|--------|-----------|--------|
//! | **Line Editing** |
//! | Ctrl+U | Clear from cursor to start of line | `pty_line_editing_test.rs` | ✅ Tested |
//! | Ctrl+K | Clear from cursor to end of line | `pty_line_editing_test.rs` | ⚠️ TODO |
//! | Ctrl+W | Delete word backward | `pty_line_editing_test.rs` | ⚠️ TODO |
//! | **Word Navigation** |
//! | Ctrl+Left | Move cursor to start of previous word | `pty_word_navigation_test.rs` | ✅ Tested |
//! | Ctrl+Right | Move cursor to start of next word | `pty_word_navigation_test.rs` | ✅ Tested |
//! | Alt+B | Move cursor backward one word | `pty_word_navigation_test.rs` | ⚠️ TODO |
//! | Alt+F | Move cursor forward one word | `pty_word_navigation_test.rs` | ⚠️ TODO |
//! | **Word Editing** |
//! | Alt+D | Delete word forward from cursor | `pty_word_editing_test.rs` | ✅ Tested |
//! | Alt+Backspace | Delete word backward from cursor | `pty_word_editing_test.rs` | ✅ Tested |
//! | Alt+T | Transpose words | `pty_word_editing_test.rs` | ⚠️ TODO |
//! | Alt+U | Uppercase word | `pty_word_editing_test.rs` | ⚠️ TODO |
//! | Alt+L | Lowercase word | `pty_word_editing_test.rs` | ⚠️ TODO |
//! | Alt+C | Capitalize word | `pty_word_editing_test.rs` | ⚠️ TODO |
//! | **Character Navigation** |
//! | Ctrl+A | Move to start of line | `pty_char_navigation_test.rs` | ⚠️ TODO |
//! | Ctrl+E | Move to end of line | `pty_char_navigation_test.rs` | ⚠️ TODO |
//! | Ctrl+B | Move backward one character | `pty_char_navigation_test.rs` | ⚠️ TODO |
//! | Ctrl+F | Move forward one character | `pty_char_navigation_test.rs` | ⚠️ TODO |
//! | Left Arrow | Move cursor left one character | `pty_char_navigation_test.rs` | ⚠️ TODO |
//! | Right Arrow | Move cursor right one character | `pty_char_navigation_test.rs` | ⚠️ TODO |
//! | Home | Move to start of line | `pty_char_navigation_test.rs` | ⚠️ TODO |
//! | End | Move to end of line | `pty_char_navigation_test.rs` | ⚠️ TODO |
//! | **Character Editing** |
//! | Backspace | Delete character before cursor | `pty_char_editing_test.rs` | ⚠️ TODO |
//! | Delete | Delete character at cursor | `pty_char_editing_test.rs` | ⚠️ TODO |
//! | Ctrl+H | Same as Backspace | `pty_char_editing_test.rs` | ⚠️ TODO |
//! | Ctrl+T | Transpose characters | `pty_char_editing_test.rs` | ⚠️ TODO |
//! | **History Navigation** |
//! | Up Arrow | Previous history entry | `pty_history_navigation_test.rs` | ⚠️ TODO |
//! | Down Arrow | Next history entry | `pty_history_navigation_test.rs` | ⚠️ TODO |
//! | Ctrl+P | Previous history entry (Emacs-style) | `pty_history_navigation_test.rs` | ⚠️ TODO |
//! | Ctrl+N | Next history entry (Emacs-style) | `pty_history_navigation_test.rs` | ⚠️ TODO |
//! | Ctrl+R | Reverse search history | `pty_history_navigation_test.rs` | ⚠️ TODO |
//! | **Special Cases** |
//! | Ctrl+D | EOF (empty line) or Delete char (non-empty) | `pty_ctrl_d_dual_behavior_test.rs` | ✅ Tested |
//! | Ctrl+Y | Yank (paste) last killed text | `pty_line_editing_test.rs` | ⚠️ TODO (requires kill ring) |
//! | Ctrl+L | Clear screen | TBD | ⚠️ TODO |
//! | Ctrl+_ | Undo last edit | TBD | ⚠️ TODO (requires undo stack) |
//!
//! ## Adding New Tests
//!
//! When adding a new keyboard shortcut test:
//! 1. Identify the functional category (line/word/char × editing/navigation)
//! 2. Add the test to the appropriate file (or create new category if needed)
//! 3. Update this table with the shortcut and test file
//! 4. Update the test file's rustdoc to list all shortcuts it covers
```

Each test file should also have a **Shortcuts Tested** section in its rustdoc:

```rust
//! Word-level editing operations.
//!
//! ## Shortcuts Tested
//!
//! | Shortcut | Action |
//! |----------|--------|
//! | Alt+D | Delete word forward from cursor |
//! | Alt+Backspace | Delete word backward from cursor |
//! | Alt+T | Transpose words |
//! | Alt+U | Uppercase word |
//! | Alt+L | Lowercase word |
//! | Alt+C | Capitalize word |
//!
//! Run with: `cargo test -p r3bl_tui --lib pty_word_editing -- --nocapture`
```

### Part 3: Implement Missing Shortcuts

## Missing Shortcuts - Prioritized by Implementation Phase

### **Phase 1: Essential (MVP) - Priority 1**

These are fundamental shortcuts users expect in any readline implementation:

| Shortcut | Action | Category | Implementation Notes |
|----------|--------|----------|----------------------|
| **Ctrl+A** | Move to start of line | Char Navigation | Set cursor to position 0 |
| **Ctrl+E** | Move to end of line | Char Navigation | Set cursor to line length |
| **Ctrl+K** | Kill/clear from cursor to end of line | Line Editing | Store killed text in kill ring (for Ctrl+Y) |
| **Left Arrow** | Move cursor left one character | Char Navigation | Decrement cursor position by 1 grapheme |
| **Right Arrow** | Move cursor right one character | Char Navigation | Increment cursor position by 1 grapheme |
| **Home** | Move to start of line | Char Navigation | Same as Ctrl+A |
| **End** | Move to end of line | Char Navigation | Same as Ctrl+E |
| **Backspace** | Delete character before cursor | Char Editing | Delete 1 grapheme before cursor |
| **Delete** | Delete character at cursor | Char Editing | Delete 1 grapheme at cursor |
| **Up Arrow** | Previous history entry | History | Navigate history backward |
| **Down Arrow** | Next history entry | History | Navigate history forward |

**Estimated effort:** 1-2 weeks
**Test file:** `pty_char_navigation_test.rs`, `pty_char_editing_test.rs`, `pty_history_navigation_test.rs`, `pty_line_editing_test.rs`

### **Phase 2: Power User - Priority 2**

Common shortcuts that power users expect:

| Shortcut | Action | Category | Implementation Notes |
|----------|--------|----------|----------------------|
| **Ctrl+W** | Delete word backward | Word Editing | May differ from Alt+Backspace in whitespace handling |
| **Ctrl+Y** | Yank (paste) last killed text | Editing | **Requires kill ring implementation** |
| **Ctrl+B** | Move backward one character | Char Navigation | Same as Left Arrow |
| **Ctrl+F** | Move forward one character | Char Navigation | Same as Right Arrow |
| **Alt+B** | Move backward one word | Word Navigation | Emacs-style alternative to Ctrl+Left |
| **Alt+F** | Move forward one word | Word Navigation | Emacs-style alternative to Ctrl+Right |
| **Ctrl+H** | Same as Backspace | Char Editing | ASCII/terminal convention |
| **Ctrl+P** | Previous history entry | History | Same as Up Arrow |
| **Ctrl+N** | Next history entry | History | Same as Down Arrow |
| **Ctrl+R** | Reverse search history | History | **Complex feature** - interactive search mode |

**Estimated effort:** 1-2 weeks
**Dependencies:** Kill ring data structure for Ctrl+Y
**Test file:** `pty_word_navigation_test.rs`, `pty_word_editing_test.rs`, `pty_char_navigation_test.rs`, `pty_history_navigation_test.rs`

### **Phase 3: Advanced Editing - Priority 3**

Less common but useful for power users:

| Shortcut | Action | Category | Implementation Notes |
|----------|--------|----------|----------------------|
| **Ctrl+T** | Transpose characters | Char Editing | Swap char before/after cursor |
| **Alt+T** | Transpose words | Word Editing | Swap current/previous word |
| **Alt+U** | Uppercase word | Word Editing | Convert word to uppercase |
| **Alt+L** | Lowercase word | Word Editing | Convert word to lowercase |
| **Alt+C** | Capitalize word | Word Editing | Capitalize first letter of word |
| **Ctrl+L** | Clear screen | Terminal | Send clear screen ANSI sequence |
| **Ctrl+_** or **Ctrl+/** | Undo last edit | Editing | **Requires undo stack implementation** |
| **Alt+.** | Insert last argument from previous command | History | Very handy for reusing args |
| **Ctrl+X Ctrl+E** | Edit command in $EDITOR | Advanced | Open current line in external editor |

**Estimated effort:** 1 week
**Dependencies:** Undo stack for Ctrl+_
**Test file:** `pty_char_editing_test.rs`, `pty_word_editing_test.rs`

### **Phase 4: Completion & Special - Priority 4**

Advanced features requiring subsystems:

| Shortcut | Action | Category | Implementation Notes |
|----------|--------|----------|----------------------|
| **Tab** | Auto-completion | Completion | **Big feature** - requires completion engine |
| **Shift+Tab** | Completion menu navigation | Completion | Navigate backwards in completion menu |
| **Ctrl+I** | Same as Tab | Completion | ASCII convention |
| **Ctrl+S** | Forward search history | History | Often disabled by terminal flow control (stty) |
| **Ctrl+Q** | Quoted insert | Advanced | Insert next character literally |
| **Ctrl+V** | Quoted insert | Advanced | Alternative to Ctrl+Q |
| **Alt+#** | Comment line and execute | Advanced | Prefix line with # and add to history |

**Estimated effort:** 2-3 weeks (mostly Tab completion)
**Dependencies:** Completion engine, path completion, command completion

### **Shortcuts to Handle Carefully**

These may need special consideration:

| Shortcut | Action | Notes |
|----------|--------|-------|
| **Ctrl+C** | Send SIGINT | Should probably exit readline, not send signal |
| **Ctrl+Z** | Send SIGTSTP | May want to suspend application |
| **Ctrl+D** | ✅ Already implemented | Dual behavior (EOF/delete) |

---

## Implementation Phases

### Recommended Order

**Phase 1 (MVP):** Priority 1 - Essential shortcuts (1-2 weeks)
- Gives users basic cursor movement and editing
- Makes the readline actually usable for real work
- **Test files:** `pty_char_navigation_test.rs`, `pty_char_editing_test.rs`, `pty_history_navigation_test.rs`

**Phase 2 (Power User):** Priority 2 - Common shortcuts (1-2 weeks)
- Adds efficiency for experienced users
- Kill ring (Ctrl+Y) enables copy/paste workflow
- **Test files:** `pty_word_navigation_test.rs`, `pty_word_editing_test.rs`, `pty_line_editing_test.rs`

**Phase 3 (Advanced):** Priority 3 - Nice to have (1 week)
- Polish and advanced editing features
- Undo support
- **Test files:** `pty_char_editing_test.rs`, `pty_word_editing_test.rs`

**Phase 4 (Completion):** Tab completion (2-3 weeks)
- This is a major feature requiring its own subsystem
- Can be deferred or implemented separately

**Total estimated effort:** 5-8 weeks (excluding completion)

---

## Implementation Guidance

### 1. Data Structures Needed

**Kill Ring** (for Ctrl+Y):
```rust
struct KillRing {
    entries: Vec<String>,
    current: usize,
    max_size: usize,
}
```

**Undo Stack** (for Ctrl+_):
```rust
struct UndoEntry {
    line: String,
    cursor: usize,
    timestamp: Instant,
}

struct UndoStack {
    entries: Vec<UndoEntry>,
    max_size: usize,
}
```

### 2. Test Pattern

Follow the existing PTY test pattern from `pty_ctrl_navigation_test.rs`:

```rust
generate_pty_test! {
    /// PTY-based integration test for [category] operations.
    ///
    /// ## Shortcuts Tested
    ///
    /// | Shortcut | Action |
    /// |----------|--------|
    /// | ... | ... |
    ///
    /// Run with: `cargo test -p r3bl_tui --lib test_name -- --nocapture`
    test_fn: test_name,
    master: pty_master_entry_point,
    slave: pty_slave_entry_point
}
```

### 3. Key Code Mapping

Ensure VT100 input parser correctly maps these sequences:

```rust
// Character navigation
Left Arrow:     ESC[D
Right Arrow:    ESC[C
Up Arrow:       ESC[A
Down Arrow:     ESC[B
Home:           ESC[H  or  ESC[1~
End:            ESC[F  or  ESC[4~

// Emacs-style navigation
Ctrl+A: 0x01
Ctrl+E: 0x05
Ctrl+B: 0x02
Ctrl+F: 0x06
Ctrl+P: 0x10
Ctrl+N: 0x0E

// Editing
Ctrl+K: 0x0B
Ctrl+W: 0x17
Ctrl+Y: 0x19
Ctrl+T: 0x14
Ctrl+H: 0x08
Backspace: 0x7F  (or 0x08 depending on terminal)
Delete: ESC[3~

// Alt combinations (ESC prefix)
Alt+B: ESC b
Alt+F: ESC f
Alt+T: ESC t
Alt+U: ESC u
Alt+L: ESC l
Alt+C: ESC c
Alt+.: ESC .
```

### 4. Word Boundary Logic

Ensure consistent word boundary detection across all word operations:
- Use the same `is_word_boundary()` logic for all word-based shortcuts
- Handle punctuation consistently
- See existing implementation in word navigation tests

---

## Acceptance Criteria

### For Reorganization
- [ ] All test files renamed according to new structure
- [ ] Master table in `mod.rs` is complete and accurate
- [ ] Each test file has "Shortcuts Tested" table in rustdoc
- [ ] All existing tests still pass after reorganization
- [ ] No duplicate tests remain

### For Each Shortcut Implementation
- [ ] Shortcut correctly mapped in VT100 input parser
- [ ] Action implemented in `LineState::apply_event_and_render()`
- [ ] PTY integration test added to appropriate file
- [ ] Test verifies correct cursor position and line content
- [ ] Test handles edge cases (empty line, cursor at start/end, etc.)
- [ ] Master table in `mod.rs` updated with ✅ status
- [ ] Individual test file rustdoc updated

### For Each Phase Completion
- [ ] All shortcuts in phase implemented
- [ ] All tests passing
- [ ] Documentation updated
- [ ] Manual testing in real terminal confirms behavior matches bash/zsh

---

## Quick Stats

- **Currently Implemented:** 6 shortcuts
- **Priority 1 (Essential):** 11 shortcuts
- **Priority 2 (Common):** 10 shortcuts
- **Priority 3 (Nice to have):** 9 shortcuts
- **Priority 4 (Special):** 6 shortcuts
- **Total missing:** 36 shortcuts
- **Total when complete:** 42 shortcuts

---

## Related Files

**Code:**
- `tui/src/readline_async/readline_async_impl/line_state.rs` - Main line editing logic
- `tui/src/core/ansi/vt_100_terminal_input_parser/` - Input sequence parsing

**Tests:**
- `tui/src/readline_async/readline_async_impl/integration_tests/` - PTY integration tests

**Documentation:**
- `tui/src/readline_async/readline_async_impl/mod.rs` - Module-level docs

---

## References

- [Bash Readline Documentation](https://www.gnu.org/software/bash/manual/html_node/Bindable-Readline-Commands.html)
- [Zsh Line Editor (ZLE)](https://zsh.sourceforge.io/Doc/Release/Zsh-Line-Editor.html)
- [GNU Readline Library](https://tiswww.case.edu/php/chet/readline/rltop.html)
- Existing implementation: `tui/src/readline_async/readme.md`

---

## Notes

- This task can be split into multiple smaller tasks (one per phase)
- Each phase is independently valuable and can be shipped separately
- Tab completion (Phase 4) is a major undertaking and could be its own epic
- Consider using feature flags to enable shortcuts incrementally
