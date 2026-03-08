# Add Overarching Narrative to PTY Module

This task involves refactoring the documentation for the `pty` module to provide a cohesive mental model and a "developer's journey" narrative. It centers on a three-layer stack model (Application, Session, Engine).

## Status
- **Priority**: High
- **Complexity**: Medium (Documentation & ASCII Diagrams)
- **Status**: [PENDING]

## Goals
1. Establish a clear "Anchor" narrative in `tui/src/core/pty/mod.rs`.
2. Define the **Task Trio** (Reader, Writer, Completion) early in the documentation.
3. Bridge the gap between low-level `pty_engine` and high-level `pty_session`.
4. Use ASCII diagrams and tables to illustrate the functional "Stack" model.

## Implementation Plan

### Step 1: Refactor `tui/src/core/pty/mod.rs`
- [x] **Narrative**: Add "The Developer's Journey" using a simple `tmux`-like multiplexer story.
- [x] **Functional Stack**: Insert the 3-layer ASCII diagram (Application -> Session -> Engine).
- [x] **Task Trio**: Explicitly define the Reader, Writer, and Completion tasks.
- [x] **Deep Dive**: Move existing coordination/channel diagrams into a "Technical Deep Dive" section.
- [x] **Cross-Linking**: Add intra-doc links to `pty_session` and `pty_engine`.

### Step 2: Refactor `tui/src/core/pty/pty_session/mod.rs` (New file)
- [x] **Create mod.rs**: Add high-level docs for the Session Layer.
- [x] **Lifecycle Diagram**: Show the flow from `PtySessionBuilder` -> `Spawn` -> `tokio::select!`.
- [x] **Code Example**: Provide a `tokio::select!` snippet showing the standard usage pattern.

### Step 3: Update `tui/src/core/pty/pty_engine/pty_pair.rs`
- [x] **Reference Upward**: Add a link in "Higher-level Sessions" pointing back to the Stack Model in `pty/mod.rs`.

### Step 4: Verification
- [x] Run `cargo doc --no-deps` to ensure all links and diagrams render correctly.
- [x] Verify that the "Story" aligns with the actual implementation in `chi` and other TUI tools.
