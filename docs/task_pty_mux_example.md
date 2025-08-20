# Create a PTYMux example in `r3bl_tui` examples

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

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
- [Phase 2: PTYMux Module Implementation (IN PROGRESS)](#phase-2-ptymux-module-implementation-in-progress)
  - [Completed Components](#completed-components)
  - [Implementation Details](#implementation-details)
    - [1. `pty_mux/mod.rs` - Public API (COMPLETED)](#1-pty_muxmodrs---public-api-completed)
    - [2. `pty_mux/multiplexer.rs` - Main Orchestrator (COMPLETED)](#2-pty_muxmultiplexerrs---main-orchestrator-completed)
    - [3. `pty_mux/process_manager.rs` - Process Lifecycle Management (COMPLETED)](#3-pty_muxprocess_managerrs---process-lifecycle-management-completed)
    - [4. `pty_mux/input_router.rs` - Dynamic Input Event Routing (COMPLETED)](#4-pty_muxinput_routerrs---dynamic-input-event-routing-completed)
    - [5. `pty_mux/output_renderer.rs` - Dynamic Display Management (COMPLETED)](#5-pty_muxoutput_rendererrs---dynamic-display-management-completed)
- [Phase 3: Example Implementation (IN PROGRESS)](#phase-3-example-implementation-in-progress)
  - [Current State](#current-state)
  - [Known Issues](#known-issues)
  - [Next Steps](#next-steps)
- [Implementation Checklist](#implementation-checklist)
  - [Phase 0: Simple PTY Example](#phase-0-simple-pty-example)
  - [Phase 1: OSC Module Enhancements](#phase-1-osc-module-enhancements)
  - [Phase 2: PTYMux Module Creation](#phase-2-ptymux-module-creation)
  - [Phase 3: Example Implementation](#phase-3-example-implementation)
  - [Phase 4: Testing & Documentation](#phase-4-testing--documentation)
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
- [Architecture Benefits](#architecture-benefits)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

## Objective

Use `pty` module (in `r3bl_tui`) to create an example in the `r3bl_tui` crate that can multiplex
terminal sessions like `tmux`.

This implementation creates a reusable `pty_mux` module in `tui/src/core/pty_mux/` that provides
terminal multiplexing functionality, leveraging existing r3bl_tui components. The example then
demonstrates this module's capabilities.

The example is located at `/home/nazmul/github/r3bl-open-core/tui/examples/pty_mux_example.rs`.

### Core Features

The example should be able to:

- Spawn multiple TUI processes (`bash`, `htop`, `btop`, `gitui`) using the enhanced PTY infrastructure
- Allow the user to switch between them using `Ctrl+<number>` keys (dynamically supports 1-9 processes)
- Show a single-line status bar with live process status indicators and keyboard shortcuts
- Use OSC codes to change terminal title dynamically based on the current process
- Use "fake resize" technique to trigger repaints when switching between TUI processes
- Leverage existing r3bl_tui infrastructure (RawMode, InputDevice, OutputDevice, PTY module)

> For context, look at [`task_prd_chi`](docs/task_prd_chi.md) for where this will be used in the future.

## Implementation Approach

### Why the Incremental Approach

Due to the complexity of PTY integration and terminal multiplexing, the implementation was broken down
into incremental steps:

1. **Simple PTY Example First**: Created `pty_simple_example.rs` to validate basic PTY functionality
2. **Infrastructure Improvements**: Enhanced the PTY module based on learnings from the simple example
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

## Phase 2: PTYMux Module Implementation (IN PROGRESS)

### Completed Components

All core components of the PTYMux module have been created and are located in `tui/src/core/pty_mux/`:

✅ Module structure created
✅ Public API defined
✅ Core components implemented
✅ Integration with enhanced PTY infrastructure

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

## Phase 3: Example Implementation (IN PROGRESS)

### Current State

The `pty_mux_example.rs` has been created with:
- Basic structure using PTYMux builder pattern
- Configuration for bash and htop processes
- Interactive prompts for user guidance

### Known Issues

1. **Process switching may have display artifacts** - Need to validate fake resize implementation
2. **Status bar rendering needs testing** - Ensure proper positioning and updates
3. **Error handling for missing commands** - Need graceful fallback

### Next Steps

1. Test process switching thoroughly
2. Validate status bar rendering
3. Add more TUI processes (btop, gitui)
4. Implement error recovery for failed processes
5. Add comprehensive error messages

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
- [ ] Test with different numbers of processes (2-9)
- [ ] Verify dynamic keyboard shortcuts work correctly
- [ ] Test terminal title updates via OSC
- [ ] Validate fake resize repainting works
- [ ] Ensure clean shutdown and resource cleanup

### Phase 4: Testing & Documentation
- [ ] Unit tests for each pty_mux module component
- [ ] Integration tests for full PTYMux functionality
- [ ] Test with various terminal emulators
- [ ] Document keyboard shortcuts and features
- [ ] Add example to CI build if appropriate

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

**Solution**: The fake resize technique sends a resize event to trigger TUI app repainting. May need tuning of the delay after resize.

### Issue 2: Input Routing Complexity

**Problem**: Complex key combinations may not route correctly.

**Solution**: Enhanced PTY input event system with comprehensive control sequence support and cursor mode handling.

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

## Architecture Benefits

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
11. **Dynamic Process Support**: Automatically adapts UI and input handling to any number of processes (1-9)
12. **Robust Infrastructure**: Enhanced PTY module with comprehensive input/output handling