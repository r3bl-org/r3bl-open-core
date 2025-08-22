# Create a PTYMux example in `r3bl_tui` examples

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Complete Data Flow Architecture (Per-Process Buffers)](#complete-data-flow-architecture-per-process-buffers)
  - [**System Architecture Overview**](#system-architecture-overview)
  - [**Core Architectural Principles**](#core-architectural-principles)
  - [**Initial Setup Flow**](#initial-setup-flow)
  - [**Process Startup Flow**](#process-startup-flow)
  - [**Main Event Loop Data Flow**](#main-event-loop-data-flow)
    - [**Independent Virtual Terminal Updates (The Key Innovation)**](#independent-virtual-terminal-updates-the-key-innovation)
    - [**Input Flow**](#input-flow)
    - [**Terminal Resize Handling**](#terminal-resize-handling)
  - [**Why This Architecture Works**](#why-this-architecture-works)
    - [**Problems with Previous Approach**:](#problems-with-previous-approach)
    - [**The Per-Process Buffer Solution**:](#the-per-process-buffer-solution)
  - [**Data Flow Summary**](#data-flow-summary)
- [Objective](#objective)
  - [Core Features](#core-features)
- [Implementation Approach](#implementation-approach)
  - [Why the Incremental Approach](#why-the-incremental-approach)
- [Architecture Overview](#architecture-overview)
  - [Module Structure](#module-structure)
  - [Key Design Principles](#key-design-principles)
  - [TUI-Focused Design](#tui-focused-design)
  - [PTY Infrastructure Improvements](#pty-infrastructure-improvements)
- [Phase 0: Simple PTY Example (COMPLETED)](#phase-0-simple-pty-example-completed)
  - [Purpose](#purpose)
  - [Implementation](#implementation)
  - [Key Learnings](#key-learnings)
- [Phase 1: OSC Module Enhancements (PARTIALLY COMPLETED)](#phase-1-osc-module-enhancements-partially-completed)
  - [Completed Items](#completed-items)
  - [Remaining Work](#remaining-work)
- [Phase 2: PTYMux Module Implementation (COMPLETED)](#phase-2-ptymux-module-implementation-completed)
  - [Completed Components](#completed-components)
  - [Implementation Details](#implementation-details)
    - [1. `pty_mux/mod.rs` - Public API (COMPLETED)](#1-pty_muxmodrs---public-api-completed)
    - [2. `pty_mux/multiplexer.rs` - Main Orchestrator (COMPLETED)](#2-pty_muxmultiplexerrs---main-orchestrator-completed)
    - [3. `pty_mux/process_manager.rs` - Process Lifecycle Management (COMPLETED)](#3-pty_muxprocess_managerrs---process-lifecycle-management-completed)
    - [4. `pty_mux/input_router.rs` - Dynamic Input Event Routing (COMPLETED)](#4-pty_muxinput_routerrs---dynamic-input-event-routing-completed)
    - [5. `pty_mux/output_renderer.rs` - Dynamic Display Management (COMPLETED)](#5-pty_muxoutput_rendererrs---dynamic-display-management-completed)
- [Phase 3: Example Implementation (COMPLETED)](#phase-3-example-implementation-completed)
  - [Implementation Status](#implementation-status)
  - [Key Features Implemented](#key-features-implemented)
  - [Architecture Components](#architecture-components)
- [Implementation Checklist](#implementation-checklist)
  - [Phase 0: Simple PTY Example](#phase-0-simple-pty-example)
  - [Phase 1: OSC Module Enhancements](#phase-1-osc-module-enhancements)
  - [Phase 2: PTYMux Module Creation](#phase-2-ptymux-module-creation)
  - [Phase 3: Example Implementation](#phase-3-example-implementation)
  - [Phase 4: Testing & Documentation](#phase-4-testing--documentation)
  - [Phase 5: OffscreenBuffer Compositor Implementation](#phase-5-offscreenbuffer-compositor-implementation)
  - [Phase 6: Testing and Validation (CURRENT)](#phase-6-testing-and-validation-current)
- [Testing Strategy](#testing-strategy)
  - [Debugging Approach](#debugging-approach)
  - [Unit Tests](#unit-tests)
  - [Integration Tests](#integration-tests)
  - [Manual Testing](#manual-testing)
- [Known Issues and Solutions](#known-issues-and-solutions)
  - [Issue 1: Process Switching Display Problems](#issue-1-process-switching-display-problems)
  - [Issue 2: Input Routing Complexity](#issue-2-input-routing-complexity)
- [Future Enhancements](#future-enhancements)
  - [Short Term](#short-term)
  - [Long Term](#long-term)
- [Phase 4: Display Issues Fix (COMPLETED)](#phase-4-display-issues-fix-completed)
  - [Problems Identified](#problems-identified)
    - [Problem 1: Status Bar Timing Issue](#problem-1-status-bar-timing-issue)
    - [Problem 2: Incomplete Process Repaints](#problem-2-incomplete-process-repaints)
    - [Problem 3: Escape Sequences Sent as Input (CRITICAL BUG FOUND)](#problem-3-escape-sequences-sent-as-input-critical-bug-found)
  - [Solution Overview](#solution-overview)
  - [Implementation Steps](#implementation-steps)
    - [Step 1: Fix Process Manager (CORRECTED APPROACH)](#step-1-fix-process-manager-corrected-approach)
    - [Step 2: Remove Unused Escape Sequence Functions](#step-2-remove-unused-escape-sequence-functions)
    - [Step 3: Keep Output Renderer As Is](#step-3-keep-output-renderer-as-is)
    - [Step 4: No Event Loop Changes Needed](#step-4-no-event-loop-changes-needed)
  - [Files to Modify](#files-to-modify)
  - [Expected Benefits](#expected-benefits)
  - [Architecture Benefits](#architecture-benefits)
- [Phase 5: OffscreenBuffer Compositor Implementation (COMPLETED)](#phase-5-offscreenbuffer-compositor-implementation-completed)
  - [Problem Statement](#problem-statement)
  - [Solution: OffscreenBuffer Compositor Pattern ✅ COMPLETED](#solution-offscreenbuffer-compositor-pattern--completed)
  - [Implementation Completed ✅](#implementation-completed-)
  - [Architecture Overview](#architecture-overview-1)
  - [Implementation Components](#implementation-components)
    - [1. Add vte Dependency](#1-add-vte-dependency)
    - [2. Create ANSI Parser Module](#2-create-ansi-parser-module)
    - [3. Update OutputRenderer with Compositor](#3-update-outputrenderer-with-compositor)
    - [4. Add Parser Module to pty_mux](#4-add-parser-module-to-pty_mux)
  - [Benefits of This Approach](#benefits-of-this-approach)
  - [Implementation Checklist](#implementation-checklist-1)
  - [Testing Strategy](#testing-strategy-1)
  - [Known Considerations](#known-considerations)
- [Phase 6: Single Buffer Compositor Implementation (COMPLETED)](#phase-6-single-buffer-compositor-implementation-completed)
  - [Summary](#summary)
  - [What Was Accomplished](#what-was-accomplished)
  - [Limitations Discovered](#limitations-discovered)
  - [Conclusion](#conclusion)
- [Phase 7: Per-Process Buffer Architecture (COMPLETED)](#phase-7-per-process-buffer-architecture-completed)
  - [Overview](#overview)
  - [What Was Accomplished](#what-was-accomplished-1)
  - [Implementation Summary](#implementation-summary)
    - [Key Architectural Changes](#key-architectural-changes)
- [Phase 8: ANSI Parser Enhancements (COMPLETED)](#phase-8-ansi-parser-enhancements-completed)
  - [Overview](#overview-1)
  - [What Was Accomplished](#what-was-accomplished-2)
  - [Key Technical Improvements](#key-technical-improvements)
  - [Files Modified](#files-modified)
  - [Benefits Achieved](#benefits-achieved)
  - [Testing Results](#testing-results)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

## Complete Data Flow Architecture (Per-Process Buffers)

This section describes the target architecture for the PTY multiplexer using per-process
OffscreenBuffers - a true terminal multiplexer that works with ANY program, not just TUI
applications.

### **System Architecture Overview**

The PTY multiplexer (`pty_mux_example.rs`) creates a terminal multiplexer similar to tmux, but with
enhanced support for truecolor and TUI apps that frequently re-render their UI. It can run ANY
program (bash, ls, cat, htop, vim, etc.) in separate PTY sessions with each process maintaining its
own virtual terminal (OffscreenBuffer). Users switch between them using F1-F9 keys to instantly see
the exact state of each terminal.

**Key Innovation**: Each process has its own persistent OffscreenBuffer that acts as a virtual
terminal, continuously capturing ALL output. This eliminates the need for fake resize hacks and
supports every type of program.

### **Core Architectural Principles**

1. **Virtual Terminal per Process**: Each process writes to its own OffscreenBuffer, which acts as a
   complete virtual terminal
2. **Continuous Capture**: ALL processes are continuously polled and their output captured, not just
   the active one
3. **No Fake Resize**: Process switching is instant - just display a different buffer
4. **Universal Compatibility**: Works with bash, CLI tools, TUI apps - anything that can run in a
   terminal
5. **Clean Separation**: Each PTY gets `height - 1` rows; status bar never collides with process
   output

### **Initial Setup Flow**

1. **Example starts** (`pty_mux_example.rs:main`):
   - Creates a list of `Process` structs (can be ANY programs: bash, htop, vim, ls, etc.)
   - Builds `PTYMux` using the builder pattern
   - Calls `multiplexer.run().await`

2. **PTYMux initialization** (`mux.rs:build`):
   - Creates `ProcessManager` to manage PTY sessions
   - Creates `InputRouter` to handle keyboard input
   - Creates `OutputRenderer` (coordinates buffer display)
   - Gets terminal size and creates `InputDevice` and `OutputDevice`

3. **Process initialization** (NEW - per-process buffers):
   - Each `Process` struct gets:
     - Its own `OffscreenBuffer` sized at `(height - 1, width)`
     - Its own `vte::Parser` for ANSI parsing
     - A `PtyReadWriteSession` for I/O
   - PTY is told it has `height - 1` rows (status bar reserved)

### **Process Startup Flow**

1. **Start all processes** (`process_manager.rs:start_all_processes`):
   - Spawns each program in its own PTY
   - Each PTY is configured with size `(height - 1, width)`
   - Each process immediately starts writing to its virtual terminal
   - ALL processes run independently from the start

### **Main Event Loop Data Flow**

The event loop (`mux.rs:run_event_loop`) continuously maintains ALL virtual terminals:

- **10ms timer**: Polls output from ALL processes (not just active)
- **Render on change**: Only repaints when active process has new output or on switch

#### **Independent Virtual Terminal Updates (The Key Innovation)**

ALL processes update their buffers independently when they produce output:

1. **Continuous Output Capture** (every 10ms):

   ```rust
   for (index, process) in process_manager.processes_mut().enumerate() {
       if let Some(output) = process.try_get_output() {
           // Update THIS process's buffer (not the active one)
           process.process_pty_output_and_update_buffer(output);  // Parse ANSI → OffscreenBuffer
       }
   }
   ```

2. **Per-Process Buffer Updates** (`process.process_pty_output_and_update_buffer`):
   - Each process has its own `vte::Parser` and `OffscreenBuffer`
   - Raw PTY bytes → ANSI Parser → That process's OffscreenBuffer
   - Buffer continuously reflects the current state of that terminal
   - Works for ANY program: bash prompts, ls output, vim screens, etc.

3. **ANSI Parsing per Buffer** (`ansi_parser.rs`):
   - Same parsing as before, but into process-specific buffer
   - Each buffer maintains its own cursor position
   - Each buffer is sized at `(height - 1, width)` - status bar reserved
   - No interference between processes

4. **Display the Active Process**:

   ```
   Active Process's OffscreenBuffer → Composite Status Bar → Paint to Terminal
   ```

5. **Rendering Active Buffer** (`output_renderer.rs:render`):
   - Get the active process's OffscreenBuffer
   - Clone it (or reference for compositing)
   - Add status bar to the last row
   - Paint entire buffer atomically to terminal

6. **Status Bar Compositing**:
   - Status bar is added to row `height - 1` of display buffer
   - Shows all processes with activity indicators
   - Active process highlighted: `[F1] 🟢bash | F2 🟢vim | F3 🟢htop`

#### **Input Flow**

1. **User presses F2** to switch to process 2:
   - `InputDevice` captures the key event
   - `InputRouter` detects F2
   - Calls `process_manager.switch_to(1)`

2. **Instant Process Switch** (`process_manager.rs:switch_to`):

   ```rust
   pub fn switch_to(&mut self, index: usize) {
       self.active_index = index;
       // That's it! No fake resize, no delays, no hacks
   }
   ```

3. **Display Update**:
   - Next render cycle shows process 2's buffer
   - Buffer already contains the complete current state
   - User sees instant switch with no delay or flicker
   - Works even for bash (shows command history and prompt)

#### **Terminal Resize Handling**

When the terminal is resized:

1. **Create New Buffers**:

   ```rust
   pub fn handle_terminal_resize(&mut self, new_size: Size) {
       let pty_size = Size {
           height: new_size.height - 1,  // Reserve status bar
           width: new_size.width,
       };

       for process in &mut self.processes {
           // Fresh buffer at new size
           process.offscreen_buffer = OffscreenBuffer::new(pty_size);
           process.parser = Parser::new();

           // Tell PTY about resize
           if let Some(session) = &process.session {
               session.resize(pty_size);  // Sends SIGWINCH
           }
       }
   }
   ```

2. **Natural Reflow**:
   - Each PTY process receives SIGWINCH
   - TUI apps (vim, htop) repaint at new size
   - Bash redraws its prompt
   - Simple programs unaffected (already completed)
   - New buffers capture the resized output

### **Why This Architecture Works**

#### **Problems with Previous Approach**:

- **TUI-only**: Fake resize hack only worked with programs that repaint on SIGWINCH
- **Bash broken**: Shell sessions showed nothing because bash doesn't repaint
- **Timing issues**: 50ms delays, race conditions, escape sequence bugs
- **Lost state**: Switching away lost terminal content

#### **The Per-Process Buffer Solution**:

1. **Universal Compatibility**:
   - Works with EVERYTHING: bash, ls, cat, vim, htop, anything
   - Each program writes to its virtual terminal naturally
   - No program-specific hacks or assumptions

2. **True Persistence**:
   - Each buffer is a complete snapshot of that terminal
   - Switch back to bash → see your command history
   - Switch to vim → see your file exactly as you left it
   - Run `ls` in one terminal → output remains visible

3. **Clean Architecture**:
   - PTY processes think they own a `(height-1) × width` terminal
   - Status bar exists in a separate layer
   - No collision, no escape sequence injection, no hacks

4. **Instant Switching**:
   - No fake resize delays
   - No waiting for repaints
   - Just display a different buffer
   - Zero latency switching

### **Data Flow Summary**

```
9 Parallel Virtual Terminals:
┌─────────────────────────────────────────┐
│ Process 1: bash                         │ → OffscreenBuffer 1
│   $ ls -la                              │   (continuously updated)
│   total 48K...                          │
├─────────────────────────────────────────┤
│ Process 2: vim file.rs                  │ → OffscreenBuffer 2
│   fn main() {                           │   (continuously updated)
│       println!("Hello");                │
├─────────────────────────────────────────┤
│ Process 3: htop                         │ → OffscreenBuffer 3
│   CPU [||||    ] 42%                    │   (continuously updated)
│   Mem [||||||||] 78%                    │
└─────────────────────────────────────────┘
                    ↓
         User presses F2 (select buffer 2)
                    ↓
    ┌──────────────────────────────┐
    │ Display Buffer = Buffer 2    │
    │ + Status bar at bottom       │
    └──────────────────────────────┘
                    ↓
            Paint to Terminal
```

**The key insight**: We're not switching between processes and asking them to repaint. We're
maintaining 9 independent virtual terminals and choosing which one to display. This is how real
terminal multiplexers work.

## Objective

Use `pty` module (in `r3bl_tui`) to create an example in the `r3bl_tui` crate that can multiplex
terminal sessions like `tmux`.

This implementation creates a reusable `pty_mux` module in `tui/src/core/pty_mux/` that provides
terminal multiplexing functionality, leveraging existing r3bl_tui components. The example then
demonstrates this module's capabilities.

The example is located at `/home/nazmul/github/r3bl-open-core/tui/examples/pty_mux_example.rs`.

### Core Features

The example should be able to:

- Spawn multiple TUI processes (`claude`, `htop`, `gitui`, `less /etc/adduser.conf`) using the
  enhanced PTY infrastructure
- Allow the user to switch between them using `Ctrl+<number>` keys (dynamically supports 1-9
  processes)
- Show a single-line status bar with live process status indicators and keyboard shortcuts
- Use OSC codes to change terminal title dynamically based on the current process
- Use "fake resize" technique to trigger repaints when switching between TUI processes
- Leverage existing r3bl_tui infrastructure (RawMode, InputDevice, OutputDevice, PTY module)

> For context, look at [`task_prd_chi`](docs/task_prd_chi.md) for where this will be used in the
> future.

## Implementation Approach

### Why the Incremental Approach

Due to the complexity of PTY integration and terminal multiplexing, the implementation was broken
down into incremental steps:

1. **Simple PTY Example First**: Created `pty_simple_example.rs` to validate basic PTY functionality
2. **Infrastructure Improvements**: Enhanced the PTY module based on learnings from the simple
   example
3. **PTYMux Module**: Built the multiplexer module with the improved infrastructure
4. **Full Example**: Integrated everything into the final `pty_mux_example.rs`

This approach allows for:

- Easier debugging of PTY-specific issues
- Validation of core functionality before adding complexity
- Better understanding of terminal behavior and requirements
- Incremental testing and validation

## Architecture Overview

### Module Structure

```
tui/src/core/
├── pty_mux/                   # PTY multiplexer module (NEW)
│   ├── mod.rs                 # Public API exports
│   ├── multiplexer.rs         # Main PTYMux struct
│   ├── process_manager.rs     # Process lifecycle management
│   ├── input_router.rs        # Dynamic input event routing
│   └── output_renderer.rs     # Display management and status bar
│
├── ansi/                      # Terminal output helpers
│   ├── mod.rs                 # Module exports
│   └── terminal_output.rs     # NEW: High-level terminal operations
│
├── osc/                       # Enhanced OSC module
│   ├── mod.rs                 # Existing + new exports
│   ├── osc_buffer.rs          # Existing
│   ├── osc_codes.rs           # Enhanced with new codes
│   ├── osc_event.rs           # Enhanced with new events
│   ├── osc_hyperlink.rs       # Existing
│   └── osc_controller.rs      # NEW: OSC sequence controller
│
├── pty/                       # Enhanced PTY module
│   ├── pty_core/
│   │   ├── pty_input_events.rs  # NEW: Comprehensive input event handling
│   │   └── pty_output_events.rs # NEW: Enhanced output event handling
│   ├── pty_read_write.rs        # Enhanced with cursor mode support
│   └── ...

tui/examples/
├── pty_simple_example.rs      # Simple PTY example (precursor)
├── pty_mux_example.rs         # Full multiplexer example
└── pty_rw_echo_example.rs     # Echo test example
```

### Key Design Principles

1. **Maximum Code Reuse**: Leverage existing r3bl_tui components
2. **TUI-focused Design**: Optimized for TUI applications that respond to `SIGWINCH`
3. **Fake Resize Strategy**: Use resize events to trigger proper repaints
4. **Minimal Buffering**: TUI apps maintain their own state, minimal output buffering needed
5. **Simple Example**: Example file is just a thin wrapper around the pty_mux module
6. **Extensibility**: Easy to add more features like split panes, additional OSC codes
7. **Robust Testing**: Each phase includes comprehensive testing

### TUI-Focused Design

This implementation assumes all spawned processes are TUI applications that:

- Respond to `SIGWINCH` by repainting their entire display
- Maintain their own internal state and screen buffers
- Use cursor positioning and ANSI escape sequences for display
- Support application and normal cursor key modes

This design choice enables:

- **Accurate restoration**: TUI apps repaint themselves correctly
- **Lower memory usage**: No need to buffer complex output streams
- **Better compatibility**: Works with any well-behaved TUI application

### PTY Infrastructure Improvements

Based on the simple example implementation, the following improvements were made:

1. **Enhanced Control Sequences**: Full support for control characters with cursor mode awareness
2. **Improved Input Mapping**: Better conversion from terminal input events to PTY input
3. **Cursor Mode Support**: Proper handling of application vs normal cursor key modes
4. **Robust Testing**: Added htop as a test prerequisite in `bootstrap.sh`
5. **Debug Logging**: Comprehensive logging to `log.txt` for troubleshooting

## Phase 0: Simple PTY Example (COMPLETED)

### Purpose

The `pty_simple_example.rs` was created as an intermediate step to:

- Validate basic PTY functionality with a single process (htop)
- Debug input/output handling in isolation
- Test raw mode integration
- Ensure proper cleanup and resource management

### Implementation

Located at `tui/examples/pty_simple_example.rs`, this example:

- Spawns a single htop process in a PTY
- Maps terminal input to PTY input events
- Displays PTY output directly to the terminal
- Handles Ctrl+Q for graceful shutdown
- Provides extensive debug logging to `log.txt`

### Key Learnings

1. **Cursor Mode Handling**: Arrow keys and other special keys require proper cursor mode support
2. **Input Event Conversion**: Need comprehensive mapping from `KeyPress` to `PtyInputEvent`
3. **Output Buffering**: Direct pass-through works well for TUI applications
4. **Process Cleanup**: Proper shutdown sequence is critical (Ctrl+C → wait → Close)
5. **Debug Logging**: Essential for understanding PTY communication issues

## Phase 1: OSC Module Enhancements (PARTIALLY COMPLETED)

### Completed Items

✅ **OSC Codes** (`osc/osc_codes.rs`):

- Added `OSC0_SET_TITLE_AND_TAB` constant for setting terminal titles
- Added `BELL_TERMINATOR` constant for OSC sequence termination

✅ **OSC Events** (`osc/osc_event.rs`):

- Added `SetTitleAndTab(String)` event type

✅ **OSC Controller** (`osc/osc_controller.rs`):

- Created controller with `set_title_and_tab()` method
- Integrated with OutputDevice for writing sequences

✅ **Module Exports** (`osc/mod.rs`):

- Updated to export the new OscController

### Remaining Work

- [ ] Additional OSC sequences if needed (notifications, etc.)
- [ ] Testing with various terminal emulators

## Phase 2: PTYMux Module Implementation (COMPLETED)

### Completed Components

All core components of the PTYMux module have been fully implemented and are located in
`tui/src/core/pty_mux/`:

✅ **Module structure created** - Complete pty_mux module with 5 core components ✅ **Public API
defined** - Clean exports and comprehensive documentation ✅ **Core components implemented** - All 5
components fully functional ✅ **Integration completed** - Full integration with enhanced PTY
infrastructure ✅ **Terminal output helpers** - New `ansi/terminal_output.rs` module for crossterm
integration

### Implementation Details

#### 1. `pty_mux/mod.rs` - Public API (COMPLETED)

Provides clean module exports and comprehensive documentation for the PTYMux functionality.

#### 2. `pty_mux/multiplexer.rs` - Main Orchestrator (COMPLETED)

Key features implemented:

- `PTYMux` struct that coordinates all components
- `PTYMuxBuilder` for configuration
- Event loop with tokio::select! for concurrent handling
- Raw mode management
- OSC integration for terminal titles

Current implementation uses:

- `InputDevice::new_event_stream()` for input
- `OutputDevice::new_stdout()` for output
- Proper cleanup in destructor

#### 3. `pty_mux/process_manager.rs` - Process Lifecycle Management (COMPLETED)

Key features implemented:

- `ProcessManager` for managing multiple PTY sessions
- `Process` struct for process definitions
- `start_all_processes()` for immediate spawning (fast switching)
- Fake resize technique for TUI app repainting
- Status bar height reservation (1 line)

Uses enhanced PTY infrastructure:

- `PtyCommandBuilder` for spawning
- `PtySize` with proper dimensions
- `PtyReadWriteSession` for bidirectional communication

#### 4. `pty_mux/input_router.rs` - Dynamic Input Event Routing (COMPLETED)

Key features implemented:

- Dynamic Ctrl+1 through Ctrl+9 routing based on process count
- Ctrl+Q for exit
- Input forwarding to active PTY
- Terminal resize handling
- OSC title updates on process switch

Leverages improved input mapping:

- Conversion from `KeyPress` to `PtyInputEvent`
- Proper handling of control sequences
- Cursor mode awareness

#### 5. `pty_mux/output_renderer.rs` - Dynamic Display Management (COMPLETED)

Key features implemented:

- Direct output rendering for active process
- Status bar with process indicators (🟢 running, 🔴 stopped)
- Dynamic keyboard shortcut display
- Screen clearing on process switch
- ANSI escape sequence handling

## Phase 3: Example Implementation (COMPLETED)

### Implementation Status

The `pty_mux_example.rs` has been fully implemented with:

- Complete structure using PTYMux builder pattern
- Configuration for multiple TUI processes (less, htop, claude, gitui)
- Interactive prompts and comprehensive user guidance
- Full integration with all pty_mux module components
- Debug logging to `log.txt` for troubleshooting

### Key Features Implemented

✅ **Process switching with fake resize** - Implemented and functional for proper TUI app repainting
✅ **Status bar rendering** - Live status indicators with process states and shortcuts ✅ **Multiple
TUI processes** - Supports less, htop, claude, and gitui ✅ **Error handling** - Graceful handling
of missing commands and process failures ✅ **Terminal title updates** - OSC sequence integration
for dynamic titles ✅ **Resource management** - Proper cleanup of PTY sessions and raw mode

### Architecture Components

1. **`multiplexer.rs`** - Main PTYMux orchestrator with event loop
2. **`process_manager.rs`** - PTY lifecycle management and fake resize
3. **`input_router.rs`** - Dynamic keyboard input routing (Ctrl+1-9)
4. **`output_renderer.rs`** - Display management and status bar
5. **`terminal_output.rs`** - High-level terminal operations helper module

## Implementation Checklist

### Phase 0: Simple PTY Example

- [x] Create `pty_simple_example.rs`
- [x] Implement single process (htop) handling
- [x] Add debug logging infrastructure
- [x] Test input/output mapping
- [x] Validate cleanup sequence

### Phase 1: OSC Module Enhancements

- [x] Add new OSC codes to `osc/osc_codes.rs`
- [x] Extend `osc/osc_event.rs` with new event types
- [x] Create `osc/osc_controller.rs` with OSC sequence methods
- [x] Update `osc/mod.rs` to export new controller
- [ ] Test OSC sequence generation with various terminals

### Phase 2: PTYMux Module Creation

- [x] Create `pty_mux/mod.rs` with public API exports
- [x] Implement `pty_mux/multiplexer.rs` with main PTYMux orchestrator
- [x] Build `pty_mux/process_manager.rs` for PTY lifecycle management
- [x] Create `pty_mux/input_router.rs` for dynamic keyboard input handling
- [x] Implement `pty_mux/output_renderer.rs` for display management
- [x] Add pty_mux module to `tui/src/core/mod.rs`

### Phase 3: Example Implementation

- [x] Create `tui/examples/pty_mux_example.rs` using PTYMux
- [x] Test with different numbers of processes (2-9)
- [x] Verify dynamic keyboard shortcuts work correctly (Ctrl+1-9, Ctrl+Q)
- [x] Test terminal title updates via OSC
- [x] Validate fake resize repainting works
- [x] Ensure clean shutdown and resource cleanup
- [x] Add comprehensive documentation and usage examples
- [x] Implement all 5 core pty_mux module components

### Phase 4: Testing & Documentation

- [x] Fix critical escape sequence bug (Phase 4 completed)
- [ ] Unit tests for each pty_mux module component
- [ ] Integration tests for full PTYMux functionality
- [ ] Test with various terminal emulators
- [ ] Document keyboard shortcuts and features
- [ ] Add example to CI build if appropriate

### Phase 5: OffscreenBuffer Compositor Implementation

- [x] Add vte dependency to `tui/Cargo.toml`
- [x] Create `tui/src/core/pty_mux/ansi_parser.rs` module
- [x] Implement `AnsiToBufferProcessor` with `Perform` trait
- [x] Update `OutputRenderer` to use `OffscreenBuffer` as compositor
- [x] Add ANSI color mapping functions
- [x] Integrate with existing paint infrastructure
- [x] Simplify API to use OutputDevice instead of GlobalData
- [x] Fix all compilation errors including tests

### Phase 6: Testing and Validation (CURRENT)

- [ ] Test with all TUI processes (claude, less, htop, gitui)
- [ ] Verify no visual artifacts when switching processes
- [ ] Ensure status bar doesn't interfere with TUI apps
- [ ] Check ANSI color mapping and cursor positioning
- [ ] Add alternate screen buffer support using crossterm
- [ ] Validate process lifecycle management
- [ ] Test terminal resize handling with OffscreenBuffer

## Testing Strategy

### Debugging Approach

1. **Logging**: Use `try_initialize_logging_global()` to enable debug output to `log.txt`
2. **Incremental Testing**: Start with simple example, then add complexity
3. **Process Isolation**: Test each TUI process individually first
4. **Terminal Variety**: Test with different terminal emulators (iTerm2, Terminal.app, Alacritty)

### Unit Tests

- **OSC Controller**: Test OSC sequence generation
- **Process Manager**: Test PTY spawning and lifecycle
- **Input Router**: Test keyboard event routing
- **Output Renderer**: Test status bar formatting

### Integration Tests

- **Full Multiplexer**: Test complete process switching workflow
- **Fake Resize**: Verify TUI app repainting works correctly
- **Error Handling**: Test missing commands, PTY failures
- **Resource Cleanup**: Verify RawMode and PTY cleanup

### Manual Testing

1. Run example with all target TUI processes available
2. Test with missing processes (fallback behavior)
3. Verify process switching triggers proper repaints
4. Test terminal resize handling
5. Validate OSC title updates in terminal
6. Test process exit and restart scenarios

## Known Issues and Solutions

### Issue 1: Process Switching Display Problems

**Problem**: When switching between processes, display artifacts may occur.

**Solution**: The fake resize technique sends a resize event to trigger TUI app repainting. May need
tuning of the delay after resize.

### Issue 2: Input Routing Complexity

**Problem**: Complex key combinations may not route correctly.

**Solution**: Enhanced PTY input event system with comprehensive control sequence support and cursor
mode handling.

## Future Enhancements

### Short Term

- [ ] Add more TUI process options (`nvtop`, `lazygit`)
- [ ] Implement process restart functionality
- [ ] Add configurable keybindings
- [ ] Support for custom process arguments
- [ ] Process health monitoring and auto-restart

### Long Term

- [ ] Split pane functionality (horizontal/vertical)
- [ ] Session persistence and restoration
- [ ] Mouse support for process selection
- [ ] Themeable status bar and UI elements
- [ ] Plugin architecture for custom TUI processes
- [ ] Network transparency (like tmux)

## Phase 4: Display Issues Fix (COMPLETED)

### Problems Identified

#### Problem 1: Status Bar Timing Issue

Claude takes 3-5 seconds to start, status bar renders immediately, creates visual artifacts when
Claude finally outputs content below the existing status bar.

#### Problem 2: Incomplete Process Repaints

TUI processes only paint differential updates when switching, causing missing/partial displays
because they don't know we cleared the screen between switches.

#### Problem 3: Escape Sequences Sent as Input (CRITICAL BUG FOUND)

**Root Cause Discovered:** The escape sequences (`\x1b[2J\x1b[H` and `\x1b[?1049l\x1b[?1049h`) were
being sent **as input to PTY processes** instead of being interpreted as terminal control sequences.

**Symptoms:**

- Escape sequences appear as text in Claude's input field
- Less interprets the trailing 'h' as help command, showing help screen
- Htop shows help screen for the same reason
- Gitui becomes unresponsive due to unexpected terminal state
- All TUI apps receive these sequences as if the user typed them

**The Fatal Flaw:** Sending escape sequences via `PtyInputEvent::Write()` sends them as user input,
not as terminal control commands.

### Solution Overview

**CORRECTED Approach:** Remove escape sequence sending entirely. Use only fake resize (SIGWINCH) for
TUI app repaints, which is the correct and sufficient approach.

### Implementation Steps

#### Step 1: Fix Process Manager (CORRECTED APPROACH)

**Remove escape sequence sending entirely** from the `switch_to` method in `ProcessManager`.

The corrected `switch_to` method should only use fake resize:

```rust
pub async fn switch_to(&mut self, index: usize) -> miette::Result<()> {
    if index >= self.processes.len() { return Ok(()); }

    let old_index = self.active_index;
    self.active_index = index;
    tracing::debug!("Switching from process {} to {}", old_index, index);

    if let Some(session) = &mut self.processes[index].session {
        // Only use fake resize - this is the correct and sufficient approach
        // The fake resize sends SIGWINCH, causing TUI apps to repaint themselves

        // 1. Fake resize sequence (tiny -> actual size)
        let tiny_size = PtySize::new(10, 10);
        session.resize(tiny_size)?;
        tokio::time::sleep(Duration::from_millis(50)).await;

        // 2. Resize to actual size - this triggers SIGWINCH and full repaint
        let real_size = self.calculate_pty_size();
        session.resize(real_size)?;
    }

    Ok(())
}
```

#### Step 2: Remove Unused Escape Sequence Functions

Remove the incorrectly designed functions from `ansi_escape_codes.rs`:

- `clear_screen_and_home_bytes()`
- `alt_screen_reset_bytes()`

These were fundamentally flawed - they were designed to send escape sequences as PTY input, which is
incorrect.

#### Step 3: Keep Output Renderer As Is

The `OutputRenderer` implementation is correct:

- First output tracking works properly (already implemented with `Vec<bool>`)
- Screen clearing on our terminal (not PTY) is correct
- Status bar rendering is working well

#### Step 4: No Event Loop Changes Needed

Since we're not changing the `switch_to` signature (it remains async), no changes needed to the
event loop callers.

### Files to Modify

1. **`tui/src/core/ansi/ansi_escape_codes.rs`** - Remove incorrect escape sequence functions
2. **`tui/src/core/pty_mux/process_manager.rs`** - Remove escape sequence sending from switch_to
   method

### Expected Benefits

- **Fixed escape sequence bug**: No more escape sequences appearing as text in TUI apps
- **Proper TUI app behavior**: Apps will respond normally to keyboard input
- **No unintended help screens**: Less and htop won't trigger help on process switch
- **Responsive gitui**: Gitui will remain responsive to keyboard input
- **Clean process switching**: Only fake resize (SIGWINCH) which is the correct approach
- **Simpler and correct implementation**: Removed the fundamentally flawed escape sequence injection

### Architecture Benefits

1. **TUI-Optimized Design**: Leverages `SIGWINCH` for proper display restoration
2. **Low Memory Footprint**: Minimal buffering since TUI apps maintain their own state
3. **Simple Implementation**: Much simpler than full virtual terminal emulation
4. **Maximum Code Reuse**: Leverages existing r3bl_tui infrastructure
5. **Clean Module Organization**: OSC controller in OSC module, PTYMux logic in pty_mux module
6. **Incremental Development**: Simple example validated core functionality first
7. **Extensible Design**: Easy to add features like split panes, more OSC codes
8. **Testable Components**: Each module can be unit tested independently
9. **Resource Management**: Proper cleanup using existing RawMode and PTY infrastructure
10. **Better Compatibility**: Works with any TUI app that responds to `SIGWINCH` correctly
11. **Dynamic Process Support**: Automatically adapts UI and input handling to any number of
    processes (1-9)
12. **Robust Infrastructure**: Enhanced PTY module with comprehensive input/output handling

## Phase 5: OffscreenBuffer Compositor Implementation (COMPLETED)

### Problem Statement

Current implementation has visual artifacts when switching between processes because:

1. **Status bar interference**: Rendering status bar after every PTY output disrupts TUI cursor
   position
2. **Screen space conflicts**: TUI apps and status bar fight for cursor control
3. **Timing issues**: Status bar updates can interrupt TUI app output mid-frame

### Solution: OffscreenBuffer Compositor Pattern ✅ COMPLETED

**IMPLEMENTATION COMPLETED**: Implemented a compositor using the existing `OffscreenBuffer` (kept
generic) to provide complete isolation between PTY output and status bar rendering. All PTY-specific
logic remains in the pty_mux module.

### Implementation Completed ✅

**Key Components Implemented:**

1. **vte Dependency Added**: Added `vte = "0.13"` to `tui/Cargo.toml` for robust ANSI parsing
2. **ANSI Parser Module**: Created `tui/src/core/pty_mux/ansi_parser.rs` with:
   - `AnsiToBufferProcessor` implementing `vte::Perform` trait
   - Full SGR (Select Graphic Rendition) support for colors and text styles
   - Cursor movement and positioning commands
   - Text output with proper line wrapping and bounds checking
   - Status bar row reservation (last row protected)

3. **OutputRenderer Compositor**: Updated `output_renderer.rs` with:
   - OffscreenBuffer-based compositor pattern
   - ANSI parsing → OffscreenBuffer → status bar compositing → atomic paint
   - Crossterm integration for terminal operations
   - Simplified API using OutputDevice instead of GlobalData

4. **Integration Complete**: Full integration with PTYMux using new OutputRenderer API

**Architecture Benefits Achieved:**

- ✅ Complete isolation between PTY output and status bar
- ✅ Atomic rendering using existing paint infrastructure
- ✅ No cursor position conflicts
- ✅ OffscreenBuffer remains generic (no PTY-specific code added)
- ✅ Clean module separation with PTY logic in pty_mux

### Architecture Overview

```
Terminal (Real)
├── Alternate Screen Buffer (for clean entry/exit)
└── OffscreenBuffer (Generic Compositor)
    ├── PTY Output Area (rows 0 to height-2)
    ├── Status Bar Area (last row)
    └── Cursor State Management

PTY Child Process
├── Writes ANSI sequences to PTY
├── Thinks it owns full terminal
└── Unaware of status bar

Data Flow:
PTY Output (bytes with ANSI)
    → vte Parser (interprets ANSI)
    → AnsiToBufferProcessor (updates OffscreenBuffer)
    → Composite with Status Bar
    → Convert to RenderOps
    → Paint Once to Terminal (using existing paint.rs)
```

### Implementation Components

#### 1. Add vte Dependency

Add to `tui/Cargo.toml`:

```toml
[dependencies]
vte = "0.13"  # Same version Alacritty uses
```

#### 2. Create ANSI Parser Module

New file: `tui/src/core/pty_mux/ansi_parser.rs`

```rust
use vte::{Parser, Perform};
use crate::{OffscreenBuffer, PixelChar, Pos, TuiStyle, TuiColor, Size, ColIndex, RowIndex};

/// Processes ANSI sequences from PTY output and updates OffscreenBuffer
pub struct AnsiToBufferProcessor<'a> {
    buffer: &'a mut OffscreenBuffer,
    cursor_pos: Pos,
    current_style: Option<TuiStyle>,
    // SGR state tracking
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
    blink: bool,
    reverse: bool,
    hidden: bool,
    strikethrough: bool,
    fg_color: Option<TuiColor>,
    bg_color: Option<TuiColor>,
}

impl<'a> AnsiToBufferProcessor<'a> {
    pub fn new(buffer: &'a mut OffscreenBuffer) -> Self {
        Self {
            buffer,
            cursor_pos: Pos::default(),
            current_style: None,
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            blink: false,
            reverse: false,
            hidden: false,
            strikethrough: false,
            fg_color: None,
            bg_color: None,
        }
    }

    fn update_style(&mut self) {
        self.current_style = Some(TuiStyle {
            fg: self.fg_color,
            bg: self.bg_color,
            bold: self.bold,
            italic: self.italic,
            underline: self.underline,
            dim: self.dim,
            reverse: self.reverse,
            blink: self.blink,
            hidden: self.hidden,
            strikethrough: self.strikethrough,
        });
    }

    fn cursor_up(&mut self, n: i64) {
        let n = n.max(1) as usize;
        self.cursor_pos.row_index = self.cursor_pos.row_index.saturating_sub(n);
    }

    fn cursor_down(&mut self, n: i64) {
        let n = n.max(1) as usize;
        let max_row = self.buffer.window_size.row_height.as_usize().saturating_sub(2); // Reserve status bar row
        self.cursor_pos.row_index = (self.cursor_pos.row_index + n).min(max_row);
    }

    fn cursor_forward(&mut self, n: i64) {
        let n = n.max(1) as usize;
        let max_col = self.buffer.window_size.col_width.as_usize().saturating_sub(1);
        self.cursor_pos.col_index = (self.cursor_pos.col_index + n).min(max_col);
    }

    fn cursor_backward(&mut self, n: i64) {
        let n = n.max(1) as usize;
        self.cursor_pos.col_index = self.cursor_pos.col_index.saturating_sub(n);
    }

    fn cursor_position(&mut self, params: &[i64]) {
        let row = params.get(0).copied().unwrap_or(1).max(1) as usize - 1;
        let col = params.get(1).copied().unwrap_or(1).max(1) as usize - 1;
        let max_row = self.buffer.window_size.row_height.as_usize().saturating_sub(2);
        let max_col = self.buffer.window_size.col_width.as_usize().saturating_sub(1);

        self.cursor_pos = Pos {
            col_index: col.min(max_col),
            row_index: row.min(max_row),
        };
    }

    fn sgr(&mut self, params: &[i64]) {
        for &param in params {
            match param {
                0 => { // Reset
                    self.bold = false;
                    self.dim = false;
                    self.italic = false;
                    self.underline = false;
                    self.blink = false;
                    self.reverse = false;
                    self.hidden = false;
                    self.strikethrough = false;
                    self.fg_color = None;
                    self.bg_color = None;
                }
                1 => self.bold = true,
                2 => self.dim = true,
                3 => self.italic = true,
                4 => self.underline = true,
                5 => self.blink = true,
                7 => self.reverse = true,
                8 => self.hidden = true,
                9 => self.strikethrough = true,
                30..=37 => self.fg_color = Some(ansi_to_tui_color(param - 30)),
                40..=47 => self.bg_color = Some(ansi_to_tui_color(param - 40)),
                _ => {} // Ignore unsupported SGR parameters
            }
        }
        self.update_style();
    }
}

impl Perform for AnsiToBufferProcessor<'_> {
    fn print(&mut self, c: char) {
        let row_max = self.buffer.window_size.row_height.as_usize().saturating_sub(1);
        let col_max = self.buffer.window_size.col_width.as_usize();

        if self.cursor_pos.row_index < row_max && self.cursor_pos.col_index < col_max {
            // Write character to buffer (OffscreenBuffer has public fields)
            self.buffer.buffer[self.cursor_pos.row_index][self.cursor_pos.col_index] = PixelChar::PlainText {
                display_char: c,
                maybe_style: self.current_style,
            };

            self.cursor_pos.col_index += 1;

            // Handle line wrap
            if self.cursor_pos.col_index >= col_max {
                self.cursor_pos.col_index = 0;
                if self.cursor_pos.row_index < row_max - 1 {
                    self.cursor_pos.row_index += 1;
                }
            }
        }
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x08 => { // Backspace
                if self.cursor_pos.col_index > 0 {
                    self.cursor_pos.col_index -= 1;
                }
            }
            0x09 => { // Tab - move to next 8-column boundary
                let next_tab = ((self.cursor_pos.col_index / 8) + 1) * 8;
                let max_col = self.buffer.window_size.col_width.as_usize();
                self.cursor_pos.col_index = next_tab.min(max_col - 1);
            }
            0x0A => { // Line feed
                let max_row = self.buffer.window_size.row_height.as_usize().saturating_sub(2);
                if self.cursor_pos.row_index < max_row {
                    self.cursor_pos.row_index += 1;
                }
            }
            0x0D => { // Carriage return
                self.cursor_pos.col_index = 0;
            }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &[i64], _intermediates: &[u8], _ignore: bool, c: char) {
        match c {
            'A' => self.cursor_up(params.get(0).copied().unwrap_or(1)),
            'B' => self.cursor_down(params.get(0).copied().unwrap_or(1)),
            'C' => self.cursor_forward(params.get(0).copied().unwrap_or(1)),
            'D' => self.cursor_backward(params.get(0).copied().unwrap_or(1)),
            'H' | 'f' => self.cursor_position(params),
            'J' => {}, // Clear screen - ignore, TUI apps will repaint
            'K' => {}, // Clear line - ignore, TUI apps will repaint
            'm' => self.sgr(params), // Select Graphic Rendition
            _ => {} // Ignore other CSI sequences
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        // Ignore OSC sequences - PTYMux controls terminal title
        // TUI apps often try to set titles, but we override them
    }
}

fn ansi_to_tui_color(ansi_code: i64) -> TuiColor {
    match ansi_code {
        0 => TuiColor::Black,
        1 => TuiColor::Red,
        2 => TuiColor::Green,
        3 => TuiColor::Yellow,
        4 => TuiColor::Blue,
        5 => TuiColor::Magenta,
        6 => TuiColor::Cyan,
        7 => TuiColor::White,
        _ => TuiColor::Reset,
    }
}
```

#### 3. Update OutputRenderer with Compositor

Modify `tui/src/core/pty_mux/output_renderer.rs` to use OffscreenBuffer as compositor:

```rust
use crate::{
    GlobalData, LockedOutputDevice, OffscreenBuffer, Size, FlushKind,
    terminal_lib_backends::paint::{paint, sanitize_and_save_abs_pos},
};
use super::ansi_parser::{AnsiToBufferProcessor, Parser, Perform};

pub struct OutputRenderer {
    terminal_size: Size,
    offscreen_buffer: OffscreenBuffer,
    previous_buffer: Option<OffscreenBuffer>,
    first_output_flags: Vec<bool>,
}

impl OutputRenderer {
    pub fn new(terminal_size: Size, process_count: usize) -> Self {
        Self {
            terminal_size,
            offscreen_buffer: OffscreenBuffer::new_with_capacity_initialized(terminal_size),
            previous_buffer: None,
            first_output_flags: vec![true; process_count],
        }
    }

    pub fn render(
        &mut self,
        output: ProcessOutput,
        global_data: &mut GlobalData<(), ()>,
        locked_output_device: LockedOutputDevice<'_>,
        process_manager: &ProcessManager,
    ) -> miette::Result<()> {
        match output {
            ProcessOutput::Active(data) => {
                // Process PTY output through ANSI parser into OffscreenBuffer
                self.process_pty_output(&data);

                // Composite status bar into buffer (last row)
                self.composite_status_bar(process_manager);

                // Paint buffer to terminal using existing paint infrastructure
                self.paint_buffer(global_data, locked_output_device)?;
            }
            ProcessOutput::ProcessSwitch { to_index } => {
                // Clear buffer for new process
                self.offscreen_buffer.clear_screen();

                // Mark as first output for new process
                if let Some(flag) = self.first_output_flags.get_mut(to_index) {
                    *flag = true;
                }

                // Clear terminal screen for process switch
                let locked_output_device = global_data.output_device.lock().unwrap();
                crossterm::execute!(
                    locked_output_device.deref_mut(),
                    crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
                    crossterm::cursor::MoveTo(0, 0)
                )?;
            }
        }
        Ok(())
    }

    fn process_pty_output(&mut self, data: &[u8]) {
        let mut parser = Parser::new();
        let mut processor = AnsiToBufferProcessor::new(&mut self.offscreen_buffer);

        for &byte in data {
            parser.advance(&mut processor, byte);
        }

        // Update buffer cursor position from processor
        self.offscreen_buffer.my_pos = processor.cursor_pos;
    }

    fn composite_status_bar(&mut self, process_manager: &ProcessManager) {
        let status_text = self.generate_status_text(process_manager);
        let last_row_idx = self.terminal_size.row_height.as_usize().saturating_sub(1);

        // Clear status bar row
        for col_idx in 0..self.terminal_size.col_width.as_usize() {
            self.offscreen_buffer.buffer[last_row_idx][col_idx] = PixelChar::Spacer;
        }

        // Write status text with appropriate style
        let status_style = Some(TuiStyle {
            fg: Some(TuiColor::White),
            bg: Some(TuiColor::Blue),
            bold: true,
            ..Default::default()
        });

        for (col_idx, ch) in status_text.chars().enumerate() {
            if col_idx >= self.terminal_size.col_width.as_usize() {
                break;
            }
            self.offscreen_buffer.buffer[last_row_idx][col_idx] = PixelChar::PlainText {
                display_char: ch,
                maybe_style: status_style,
            };
        }
    }

    fn paint_buffer(
        &mut self,
        global_data: &mut GlobalData<(), ()>,
        locked_output_device: LockedOutputDevice<'_>,
    ) -> miette::Result<()> {
        // Use existing paint infrastructure from paint.rs
        // Create RenderPipeline from OffscreenBuffer and paint it
        let pipeline = self.offscreen_buffer.to_render_pipeline();

        paint(
            &pipeline,
            FlushKind::JustFlush,
            global_data,
            locked_output_device,
            false, // is_mock = false
        );

        // Save current buffer for next diff
        self.previous_buffer = Some(self.offscreen_buffer.clone());
        Ok(())
    }

    fn generate_status_text(&self, process_manager: &ProcessManager) -> String {
        let mut status_parts = Vec::new();

        // Process status indicators
        for (i, process) in process_manager.processes.iter().enumerate() {
            let status_icon = if process.session.is_some() { "🟢" } else { "🔴" };
            let key_hint = format!("F{}", i + 1);
            let is_active = i == process_manager.active_index;

            let process_part = if is_active {
                format!("[{}] {}{}", key_hint, status_icon, process.name)
            } else {
                format!("{} {}{}", key_hint, status_icon, process.name)
            };

            status_parts.push(process_part);
        }

        // Add quit instruction
        status_parts.push("Ctrl+Q=quit".to_string());

        format!(" {} ", status_parts.join(" | "))
    }
}
```

#### 4. Add Parser Module to pty_mux

Add to `tui/src/core/pty_mux/mod.rs`:

```rust
mod ansi_parser; // Add this line

pub use ansi_parser::{AnsiToBufferProcessor, Parser, Perform}; // Export if needed
```

### Benefits of This Approach

1. **OffscreenBuffer Stays Generic**: No PTY-specific code added to OffscreenBuffer
2. **Complete Isolation**: PTY output and status bar never interfere
3. **Atomic Updates**: Entire screen painted in one operation using existing paint.rs
4. **No Cursor Conflicts**: Cursor position managed in buffer, not terminal
5. **Efficient Rendering**: Leverages existing diff-based paint infrastructure
6. **Clean Architecture**: PTY logic in pty_mux, generic buffer stays generic
7. **Reuses Infrastructure**: Uses existing OffscreenBuffer, paint.rs, and RenderPipeline

### Implementation Checklist

- [x] Add vte dependency to `tui/Cargo.toml` ✅
- [x] Create `tui/src/core/pty_mux/ansi_parser.rs` module ✅
- [x] Implement `AnsiToBufferProcessor` with `Perform` trait ✅
- [x] Update `OutputRenderer` to use `OffscreenBuffer` as compositor ✅
- [x] Add ANSI color mapping functions ✅
- [x] Integrate with existing paint infrastructure ✅
- [x] Simplify API to use OutputDevice instead of GlobalData ✅
- [x] Fix all compilation errors including tests ✅
- [ ] Test with all TUI processes (claude, less, htop, gitui)
- [ ] Verify no visual artifacts when switching
- [ ] Ensure status bar doesn't interfere with TUI apps
- [ ] Add alternate screen buffer support using crossterm

### Testing Strategy

1. **Unit Tests**:
   - Test ANSI parser with various escape sequences
   - Test cursor positioning and text styling
   - Test buffer compositing with status bar

2. **Integration Tests**:
   - Test full PTY output → ANSI Parser → OffscreenBuffer → paint.rs pipeline
   - Test process switching with buffer clearing
   - Test status bar rendering without interference

3. **Visual Testing**:
   - Verify no artifacts with rapid process switching
   - Test with TUI apps that use full screen (htop)
   - Test with TUI apps that scroll (less)
   - Test with TUI apps that have complex UI (gitui)
   - Verify cursor positioning is preserved correctly

### Known Considerations

1. **Architecture**: OffscreenBuffer remains completely generic and unmodified
2. **Performance**: Uses existing diff-based rendering from paint.rs for efficiency
3. **Memory**: Single buffer per OutputRenderer, reasonable memory usage
4. **vte Integration**: Battle-tested ANSI parser used by Alacritty
5. **Size Conversions**: Uses `.as_usize()` methods for easy Size/ColWidth/RowHeight access

## Phase 6: Single Buffer Compositor Implementation (COMPLETED)

### Summary

Phase 6 implemented a single OffscreenBuffer compositor with ANSI parsing to eliminate visual
artifacts. This approach works well for TUI applications that repaint on SIGWINCH but has
limitations with programs like bash that don't automatically repaint.

### What Was Accomplished

✅ **vte Integration**: Added robust ANSI parsing using the vte crate ✅ **ANSI Parser Module**:
Created `AnsiToBufferProcessor` with full SGR support ✅ **OutputRenderer Compositor**: Implemented
OffscreenBuffer-based compositing ✅ **Status Bar Isolation**: Reserved last row prevents collision
with PTY output ✅ **Atomic Rendering**: Entire screen painted in one operation ✅ **Clean
Architecture**: OffscreenBuffer remains generic, PTY logic isolated

### Limitations Discovered

- **TUI-Only**: Works only with programs that repaint on SIGWINCH
- **Bash Incompatible**: Shell sessions don't display properly
- **Fake Resize Required**: Still needs the 50ms delay hack for switching
- **No State Persistence**: Switching away loses terminal content

### Conclusion

While Phase 6 successfully eliminated visual artifacts for TUI applications, it revealed the need
for a more comprehensive solution that works with ALL terminal programs. This led to the design of
Phase 7's per-process buffer architecture.

## Phase 7: Per-Process Buffer Architecture (COMPLETED)

### Overview

Phase 7 has implemented the complete solution: each process maintains its own persistent
OffscreenBuffer that acts as a virtual terminal. This enables true terminal multiplexing that works
with ANY program - achieving universal compatibility with interactive shells, TUI applications, and
CLI tools.

### What Was Accomplished

✅ **Per-Process Virtual Terminals**: Each process now maintains its own complete terminal state
through an OffscreenBuffer and ANSI parser

✅ **Universal Compatibility**: Successfully tested with:

- **bash**: Interactive shell with persistent command history and prompt state
- **TUI applications**: htop, less, gitui with complete screen state preservation
- **AI assistant**: claude with interactive chat capabilities

✅ **Instant Switching**: Process switching is truly instant with no delays, no fake resize tricks,
no screen clearing

✅ **Parallel Processing**: All processes update their virtual terminals continuously in the
background, ready for instant switching

✅ **Status Bar Integration**: Clean compositing that doesn't interfere with process virtual
terminals

✅ **Terminal Resize Support**: All processes automatically adapt to terminal size changes with
fresh virtual terminals

### Implementation Summary

The implementation successfully transformed the PTY multiplexer from a TUI-only system with fake
resize tricks into a universal terminal multiplexer supporting all program types.

#### Key Architectural Changes

**Process Structure Enhancement**:

```rust
pub struct Process {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    session: Option<PtyReadWriteSession>,
    // NEW: Per-process virtual terminal components
    offscreen_buffer: OffscreenBuffer,     // Complete terminal state
    ansi_parser: Parser,                   // ANSI sequence processor
    is_running: bool,
    has_unrendered_output: bool,           // Rendering optimization
}
```

**Event Loop Transformation**:

- **Before**: Only polled active process, used fake resize for switching
- **After**: Polls ALL processes continuously, instant switching via buffer selection

**Universal Compatibility Achieved**:

- **Interactive shells**: bash maintains command history and prompt state
- **TUI applications**: Complete screen state preserved (htop, less, gitui)
- **AI tools**: Interactive applications work seamlessly (claude)

## Phase 8: ANSI Parser Enhancements (COMPLETED)

### Overview

Comprehensive improvements to the ANSI parser module to fix architectural bugs, implement missing
CSI sequences, improve test coverage, and reorganize test structure for better maintainability.

### What Was Accomplished

✅ **Fixed Fundamental Architecture Bug**: Removed incorrect status bar reservation from ANSI parser

- **Problem**: Parser was incorrectly using `saturating_sub(1)` and `saturating_sub(2)` to reserve
  status bar rows
- **Solution**: Status bar is a UI concept that doesn't belong in PTY output parsing - removed all
  reservations
- **Impact**: Fixed 8 methods across `cursor_down()`, `cursor_position()`, `index_down()`,
  `print()`, `execute()`, `scroll_buffer_up()`, `scroll_buffer_down()`, `reset_terminal()`

✅ **Implemented Missing CSI Sequences**: Added 6 new CSI sequence handlers in `csi_dispatch()`

- **SCP/RCP (s/u)**: Save and restore cursor position with state tracking
- **CNL/CPL (E/F)**: Cursor next/previous line with column reset
- **CHA (G)**: Cursor horizontal absolute positioning
- **SU/SD (S/T)**: Scroll up/down with buffer management
- **DSR (n)**: Device Status Report with debug logging for bidirectional communication

✅ **Fixed All Test Failures**: All 55 tests now pass (up from 42 initially failing)

- Fixed boundary conditions after removing status bar reservation
- Corrected newline handling in htop test (changed `\n` to `\r\n`)
- Updated assertions to match corrected row boundaries
- Fixed compilation errors from missing imports and variable naming conflicts

✅ **Comprehensive Test Coverage Enhancement**: Reorganized 49+ tests into focused modules

- **`tests_cursor_movement`**: Cursor positioning, movement, and boundary testing
- **`tests_sgr_styling`**: Text styling, colors, and SGR sequence handling
- **`tests_esc_sequences`**: ESC sequence processing and special character handling
- **`tests_full_ansi_sequences`**: Complex multi-sequence scenarios and real-world patterns

✅ **Replaced Hardcoded ANSI Strings**: Converted all test code to use type-safe builders

- **Before**: Hardcoded escape sequences like `"\x1b[31m"`
- **After**: Type-safe builders like `CsiSequence::Sgr(vec![SgrCode::ForegroundRed])`
- **Benefits**: Type safety, better maintainability, IDE support, compile-time validation

✅ **Enhanced CSI Sequence Support**: Extended `CsiSequence` enum and implementations

```rust
// Added to csi_codes.rs
CursorNextLine(u16),
CursorPrevLine(u16),
CursorHorizontalAbsolute(u16),
ScrollUp(u16),
ScrollDown(u16),
DeviceStatusReport(u16),
```

### Key Technical Improvements

**1. Status Bar Architecture Fix**:

- **Issue**: ANSI parser incorrectly handled UI concepts, breaking row boundary logic
- **Fix**: Removed all `saturating_sub(1)` and `saturating_sub(2)` calls
- **Result**: Parser now correctly uses full buffer height, UI layer handles status bar separately

**2. CSI Sequence Implementation**:

```rust
// New implementations in csi_dispatch()
csi_codes::SCP_SAVE_CURSOR => {
    self.buffer.saved_cursor_pos = Some(self.cursor_pos);
    tracing::trace!("CSI s (SCP): Saved cursor position {:?}", self.cursor_pos);
}
csi_codes::CNL_CURSOR_NEXT_LINE => {
    let n = i64::from(params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1));
    self.cursor_down(n);
    self.cursor_pos.col_index = col(0);
}
// ... additional sequences
```

**3. Test Structure Reorganization**:

```rust
#[cfg(test)]
mod tests {
    mod tests_cursor_movement { /* 15+ cursor tests */ }
    mod tests_sgr_styling { /* 12+ styling tests */ }
    mod tests_esc_sequences { /* 8+ ESC tests */ }
    mod tests_full_ansi_sequences { /* 20+ integration tests */ }
}
```

**4. Type-Safe Test Code**:

```rust
// Old approach (hardcoded, error-prone)
let input = "\x1b[31mHello\x1b[0m";

// New approach (type-safe, maintainable)
let input = format!("{}Hello{}",
    CsiSequence::Sgr(vec![SgrCode::ForegroundRed]),
    CsiSequence::Sgr(vec![SgrCode::Reset])
);
```

### Files Modified

**Core Implementation**:

- `tui/src/core/pty_mux/ansi_parser/ansi_parser_internal_api.rs` - Fixed architectural bug,
  implemented CSI sequences
- `tui/src/core/pty_mux/ansi_parser/csi_codes.rs` - Added new CsiSequence variants and Display
  implementations

**Test Enhancements**:

- Reorganized 55 tests into 4 focused modules
- Replaced all hardcoded ANSI strings with type builders
- Fixed compilation errors and test assertions
- Added comprehensive tests for new CSI sequences

### Benefits Achieved

1. **Architectural Correctness**: ANSI parser now correctly handles PTY output without UI
   assumptions
2. **Enhanced Compatibility**: Supports more terminal applications with additional CSI sequences
3. **Improved Maintainability**: Type-safe tests with clear modular organization
4. **Better Debugging**: Comprehensive logging for CSI sequence processing
5. **Robust Testing**: All edge cases covered with passing test suite
6. **Future-Proof**: Clean architecture ready for additional ANSI sequence support

### Testing Results

- **Total Tests**: 55 tests across 4 modules
- **Pass Rate**: 100% (all tests passing)
- **Coverage**: Cursor movement, text styling, ESC sequences, full integration scenarios
- **Quality**: Type-safe test code using builders instead of hardcoded strings

This phase successfully completed the ANSI parser enhancements, providing a solid foundation for the
PTY multiplexer's terminal emulation capabilities.
