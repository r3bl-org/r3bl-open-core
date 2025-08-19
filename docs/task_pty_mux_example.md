# Create a PTYMux example in `r3bl_tui` examples

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Objective](#objective)
  - [Core Features](#core-features)
- [Architecture Overview](#architecture-overview)
  - [Module Structure](#module-structure)
  - [Key Design Principles](#key-design-principles)
  - [TUI-Focused Design](#tui-focused-design)
- [Phase 1: OSC Module Enhancements](#phase-1-osc-module-enhancements)
  - [1. Enhance `osc/osc_codes.rs` - Add New OSC Sequences](#1-enhance-oscosc_codesrs---add-new-osc-sequences)
  - [2. Enhance `osc/osc_event.rs` - Extended Event Types](#2-enhance-oscosc_eventrs---extended-event-types)
  - [3. Create `osc/osc_controller.rs` - NEW OSC Sequence Controller](#3-create-oscosc_controllerrs---new-osc-sequence-controller)
  - [4. Update `osc/mod.rs` - Module Exports](#4-update-oscmodrs---module-exports)
- [Phase 2: PTYMux Module Implementation](#phase-2-ptymux-module-implementation)
  - [1. Create `pty_mux/mod.rs` - Public API](#1-create-pty_muxmodrs---public-api)
  - [2. Create `pty_mux/multiplexer.rs` - Main Orchestrator](#2-create-pty_muxmultiplexerrs---main-orchestrator)
  - [3. Create `pty_mux/process_manager.rs` - Process Lifecycle Management](#3-create-pty_muxprocess_managerrs---process-lifecycle-management)
  - [4. Create `pty_mux/input_router.rs` - Dynamic Input Event Routing](#4-create-pty_muxinput_routerrs---dynamic-input-event-routing)
  - [5. Create `pty_mux/output_renderer.rs` - Dynamic Display Management](#5-create-pty_muxoutput_rendererrs---dynamic-display-management)
- [Phase 3: Simple Example Implementation](#phase-3-simple-example-implementation)
- [Implementation Checklist](#implementation-checklist)
  - [Phase 1: OSC Module Enhancements](#phase-1-osc-module-enhancements-1)
  - [Phase 2: PTYMux Module Creation](#phase-2-ptymux-module-creation)
  - [Phase 3: Example Implementation](#phase-3-example-implementation)
  - [Phase 4: Testing & Documentation](#phase-4-testing--documentation)
- [Testing Strategy](#testing-strategy)
  - [Unit Tests](#unit-tests)
  - [Integration Tests](#integration-tests)
  - [Manual Testing](#manual-testing)
- [Future Enhancements](#future-enhancements)
  - [Short Term](#short-term)
  - [Long Term](#long-term)
- [Architecture Benefits](#architecture-benefits)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

## Objective

Use `pty` module (in `r3bl_tui`) to create an example in the `r3bl_tui` crate that can multiplex
terminal sessions like `tmux`.

This implementation will create a reusable `pty_mux` module in `tui/src/core/pty_mux/` that provides
terminal multiplexing functionality, leveraging existing r3bl_tui components. The example will then
be a simple demonstration of this module.

The example will be in this folder `/home/nazmul/github/r3bl-open-core/tui/examples/` and it should
be named `pty_mux_example.rs`.

### Core Features

The example should be able to:

- Spawn multiple TUI processes (`claude`, `btop`, `gitui`, `iotop`) using `spawn_read_write()` in a
  single "real" terminal window.
- Allow the user to switch between them using `Ctrl+<number>` keys (dynamically supports 1-9
  processes).
- Show a single-line status bar with live process status indicators and keyboard shortcuts.
- Use OSC codes to change terminal title dynamically based on the current process.
- Use "fake resize" technique to trigger repaints when switching between TUI processes.
- Leverage existing r3bl_tui infrastructure (RawMode, InputDevice, OutputDevice, PTY module).

> For context, look at [`task_prd_chi`](docs/task_prd_chi.md) for where this will be used in the
> future.

## Architecture Overview

### Module Structure

```
tui/src/core/
├── pty_mux/                   # New PTY multiplexer module
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
```

### Key Design Principles

1. **Maximum Code Reuse**: Leverage existing r3bl_tui components
2. **TUI-focused Design**: Optimized for TUI applications that respond to `SIGWINCH`
3. **Fake Resize Strategy**: Use resize events to trigger proper repaints
4. **Minimal Buffering**: TUI apps maintain their own state, minimal output buffering needed
5. **Simple Example**: Example file is just a thin wrapper around the pty_mux module
6. **Extensibility**: Easy to add more features like split panes, additional OSC codes

### TUI-Focused Design

This implementation assumes all spawned processes are TUI applications that:

- Respond to `SIGWINCH` by repainting their entire display
- Maintain their own internal state and screen buffers
- Use cursor positioning and ANSI escape sequences for display

This design choice enables:

- **Accurate restoration**: TUI apps repaint themselves correctly
- **Lower memory usage**: No need to buffer complex output streams
- **Better compatibility**: Works with any well-behaved TUI application

## Phase 1: OSC Module Enhancements

### 1. Enhance `osc/osc_codes.rs` - Add New OSC Sequences

```rust
// Existing codes...
pub const START: &str = "\x1b]9;4;";
pub const OSC8_START: &str = "\x1b]8;;";
pub const END: &str = "\x1b\\";
pub const DELIMITER: char = ';';

// NEW: Terminal title and tab control

/// We only implement OSC 0 (title + tab). OSC 1 (icon only) and OSC 2
/// (title only) are not needed for modern terminal multiplexing where
/// consistent branding is preferred.
pub const OSC0_SET_TITLE_AND_TAB: &str = "\x1b]0;";  // Set both title and tab name

// NEW: Alternative terminators
pub const BELL_TERMINATOR: &str = "\x07";         // BEL character (0x07)
```

### 2. Enhance `osc/osc_event.rs` - Extended Event Types

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OscEvent {
    // Existing events
    Progress(Progress),
    Hyperlink { url: String, text: String },

    // NEW events for pty_mux functionality
    SetTitleAndTab(String),
}
```

### 3. Create `osc/osc_controller.rs` - NEW OSC Sequence Controller

```rust
use crate::core::terminal_io::OutputDevice;
use super::{osc_codes, OscEvent};

/// Controller for sending OSC sequences to the terminal
pub struct OscController<'a> {
    output_device: &'a OutputDevice,
}

impl<'a> OscController<'a> {
    pub fn new(output_device: &'a OutputDevice) -> Self {
        Self { output_device }
    }

    /// Set terminal window title and tab name (OSC 0)
    pub fn set_title_and_tab(&mut self, text: &str) -> miette::Result<()> {
        let sequence = format!("{}{}{}",
            osc_codes::OSC0_SET_TITLE_AND_TAB,
            text,
            osc_codes::BELL_TERMINATOR
        );
        self.write_sequence(&sequence)
    }

    fn write_sequence(&mut self, sequence: &str) -> miette::Result<()> {
        write!(
            lock_output_device_as_mut!(self.output_device),
            "{}",
            sequence
        )?;
        Ok(())
    }
}
```

### 4. Update `osc/mod.rs` - Module Exports

```rust
//! OSC (Operating System Command) sequence parsing and formatting.
//!
//! This module provides support for:
//! - OSC 9;4 sequences used by Cargo and other build tools to communicate progress
//!   information. Supports four progress states: progress updates (0-100%), progress
//!   cleared, build errors, and indeterminate progress.
//! - OSC 8 sequences for creating terminal hyperlinks that can be clicked to open URLs or
//!   file paths.
//! - Terminal control sequences (OSC 0) for setting window titles and tab names.
//!
//! The [`OscBuffer`] handles partial sequences split across buffer reads and
//! gracefully ignores malformed input.

pub mod osc_buffer;
pub mod osc_codes;
pub mod osc_event;
pub mod osc_hyperlink;
pub mod osc_controller;  // NEW

// Re-export main types and functions for convenience
pub use osc_buffer::*;
pub use osc_event::*;
pub use osc_hyperlink::*;
pub use osc_controller::*;  // NEW
```

## Phase 2: PTYMux Module Implementation

### 1. Create `pty_mux/mod.rs` - Public API

```rust
//! Terminal multiplexer module for r3bl_tui.
//!
//! This module provides tmux-like functionality for multiplexing terminal sessions,
//! allowing users to run multiple TUI processes in a single terminal window and switch
//! between them using keyboard shortcuts.
//!
//! Key features:
//! - Multiple PTY session management for TUI applications
//! - Dynamic keyboard-driven process switching (Ctrl+1 through Ctrl+9, based on process count)
//! - Status bar with process information
//! - OSC sequence integration for terminal titles
//! - Fake resize technique for proper TUI app repainting

pub mod multiplexer;
pub mod process_manager;
pub mod input_router;
pub mod output_renderer;

pub use multiplexer::{PTYMux, PTYMuxBuilder};
pub use process_manager::{Process, ProcessManager, ProcessOutput};
pub use input_router::InputRouter;
pub use output_renderer::OutputRenderer;
```

### 2. Create `pty_mux/multiplexer.rs` - Main Orchestrator

```rust
use crate::{
    core::{
        terminal_io::{InputDevice, OutputDevice},
        osc::OscController,
        get_size,
    },
    tui::terminal_lib_backends::RawMode,
    Size, InputEvent,
};
use super::{ProcessManager, InputRouter, OutputRenderer, Process};

pub struct PTYMux {
    process_manager: ProcessManager,
    input_router: InputRouter,
    output_renderer: OutputRenderer,
    terminal_size: Size,
    output_device: OutputDevice,
    input_device: InputDevice,
}

pub struct PTYMuxBuilder {
    processes: Vec<Process>,
}

impl Default for PTYMuxBuilder {
    fn default() -> Self {
        Self {
            processes: Vec::new(),
        }
    }
}

impl PTYMuxBuilder {
    pub fn processes(mut self, processes: Vec<Process>) -> Self {
        self.processes = processes;
        self
    }

    pub fn build(self) -> miette::Result<PTYMux> {
        let terminal_size = get_size()?;
        let output_device = OutputDevice::new(std::io::stdout())?;
        let input_device = InputDevice::new()?;

        Ok(PTYMux {
            process_manager: ProcessManager::new(self.processes, terminal_size),
            input_router: InputRouter::new(),
            output_renderer: OutputRenderer::new(terminal_size),
            terminal_size,
            output_device,
            input_device,
        })
    }
}

impl PTYMux {
    pub fn builder() -> PTYMuxBuilder {
        PTYMuxBuilder::default()
    }

    pub async fn run(mut self) -> miette::Result<()> {
        // Start raw mode using existing RawMode
        RawMode::start(
            self.terminal_size,
            lock_output_device_as_mut!(&self.output_device),
            false
        );

        // Set initial terminal title using OSC controller
        let mut osc = OscController::new(&self.output_device);
        osc.set_title_and_tab("PTYMux Example - Starting")?;

        // Main event loop
        let result = self.run_event_loop(&mut osc).await;

        // Cleanup: End raw mode
        RawMode::end(
            self.terminal_size,
            lock_output_device_as_mut!(&self.output_device),
            false
        );

        result
    }

    async fn run_event_loop(&mut self, osc: &mut OscController<'_>) -> miette::Result<()> {
        loop {
            tokio::select! {
                // Handle user input using existing InputDevice
                Ok(event) = self.input_device.next() => {
                    if self.input_router.handle_input(
                        InputEvent::try_from(event)?,
                        &mut self.process_manager,
                        osc
                    ).await? {
                        break; // Exit requested
                    }
                }

                // Handle PTY outputs
                Some(output) = self.process_manager.next_output() => {
                    self.output_renderer.render(
                        output,
                        &self.output_device,
                        &self.process_manager
                    )?;
                }
            }
        }
        Ok(())
    }
}
```

### 3. Create `pty_mux/process_manager.rs` - Process Lifecycle Management

```rust
use std::collections::VecDeque;
use crate::{
    core::pty::{
        PtyReadWriteSession, PtyCommandBuilder,
        PtyInputEvent, PtyOutputEvent, PtyConfigOption
    },
    Size,
};
use tokio::time::{Duration, sleep};

const STATUS_BAR_HEIGHT: u16 = 1;

pub struct ProcessManager {
    processes: Vec<Process>,
    active_index: usize,
    terminal_size: Size,
}

pub struct Process {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    session: Option<PtyReadWriteSession>,
    is_running: bool,
}

#[derive(Debug)]
pub enum ProcessOutput {
    Active(Vec<u8>),
    ProcessSwitch { from: usize, to: usize },
}

impl Process {
    pub fn new(name: impl Into<String>, command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            args,
            session: None,
            is_running: false,
        }
    }
}

impl ProcessManager {
    pub fn new(processes: Vec<Process>, terminal_size: Size) -> Self {
        Self {
            processes,
            active_index: 0,
            terminal_size,
        }
    }

    pub async fn switch_to(&mut self, index: usize) -> miette::Result<()> {
        if index >= self.processes.len() {
            return Ok(());
        }

        let old_index = self.active_index;
        self.active_index = index;

        // Spawn process if not running
        if self.processes[index].session.is_none() {
            self.spawn_process(index)?;
        }

        // Send fake resize to trigger repaint of newly active TUI app
        // Use reduced height to reserve space for status bar
        if let Some(session) = &self.processes[index].session {
            let pty_size = Size {
                width: self.terminal_size.width,
                height: self.terminal_size.height - STATUS_BAR_HEIGHT,  // Reserve status bar space
            };

            session.input_event_ch_tx_half.send(
                PtyInputEvent::Resize(pty_size)
            )?;

            // Small delay to let the TUI app repaint
            sleep(Duration::from_millis(100)).await;
        }

        Ok(())
    }

    fn spawn_process(&mut self, index: usize) -> miette::Result<()> {
        let process = &mut self.processes[index];

        // Reserve bottom row for status bar - PTY gets reduced height
        let pty_size = Size {
            width: self.terminal_size.width,
            height: self.terminal_size.height - STATUS_BAR_HEIGHT,  // Reserve space for status bar
        };

        // Use existing PtyCommandBuilder with reduced size
        let session = PtyCommandBuilder::new(&process.command)
            .args(&process.args)
            .size(pty_size)  // Set the reduced size
            .spawn_read_write(PtyConfigOption::Output)?;

        process.session = Some(session);
        process.is_running = true;
        Ok(())
    }

    pub async fn next_output(&mut self) -> Option<ProcessOutput> {
        // Poll active PTY session
        if let Some(session) = &mut self.processes[self.active_index].session {
            if let Ok(event) = session.output_event_receiver_half.try_recv() {
                match event {
                    PtyOutputEvent::Output(data) => {
                        return Some(ProcessOutput::Active(data));
                    }
                    PtyOutputEvent::Exit(_status) => {
                        self.processes[self.active_index].is_running = false;
                        // Process exit is now only reflected in status bar
                    }
                    _ => {}
                }
            }
        }

        // Poll background processes for status updates only
        for (i, process) in self.processes.iter_mut().enumerate() {
            if i != self.active_index {
                if let Some(session) = &mut process.session {
                    if let Ok(event) = session.output_event_receiver_half.try_recv() {
                        match event {
                            PtyOutputEvent::Exit(_status) => {
                                process.is_running = false;
                                // Background process exit is only reflected in status bar
                            }
                            _ => {
                                // Discard output from background TUI apps
                                // They will repaint when brought to foreground
                            }
                        }
                    }
                }
            }
        }

        None
    }

    pub fn send_input(&mut self, event: PtyInputEvent) -> miette::Result<()> {
        if let Some(session) = &self.processes[self.active_index].session {
            session.input_event_ch_tx_half.send(event)?;
        }
        Ok(())
    }

    pub fn active_name(&self) -> &str {
        &self.processes[self.active_index].name
    }

    pub fn processes(&self) -> &[Process] {
        &self.processes
    }

    pub fn active_index(&self) -> usize {
        self.active_index
    }

    pub fn update_terminal_size(&mut self, new_size: Size) {
        self.terminal_size = new_size;
    }
}
```

### 4. Create `pty_mux/input_router.rs` - Dynamic Input Event Routing

```rust
use crate::{
    core::{osc::OscController, pty::PtyInputEvent},
    InputEvent, Key, KeyPress, KeyState, ModifierKeysMask,
};
use super::ProcessManager;

const STATUS_BAR_HEIGHT: u16 = 1;

pub struct InputRouter;

impl InputRouter {
    pub fn new() -> Self {
        Self
    }

    pub async fn handle_input(
        &mut self,
        event: InputEvent,
        process_manager: &mut ProcessManager,
        osc: &mut OscController<'_>,
    ) -> miette::Result<bool> {
        match event {
            InputEvent::Keyboard(key) => match key {
                // Dynamic process switching: Handle Ctrl+1 through Ctrl+9 based on available processes
                KeyPress::WithModifiers {
                    key: Key::Character(ch),
                    mask: ModifierKeysMask {
                        ctrl_key_state: KeyState::Pressed,
                        shift_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
                } if ch.is_ascii_digit() && ch >= '1' && ch <= '9' => {
                    // Convert character to process index (1-based to 0-based)
                    let process_index = (ch as u8 - b'1') as usize;

                    // Only switch if the process index is valid for current process count
                    if process_index < process_manager.processes().len() {
                        process_manager.switch_to(process_index).await?;
                        self.update_terminal_title(process_manager, osc)?;
                    }
                }
                KeyPress::WithModifiers {
                    key: Key::Character('q'),
                    mask: ModifierKeysMask {
                        ctrl_key_state: KeyState::Pressed,
                        shift_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
                } => return Ok(true), // Exit
                _ => {
                    // Forward all other input to active PTY
                    process_manager.send_input(PtyInputEvent::from(key))?;
                }
            },
            InputEvent::Resize(new_size) => {
                // Handle terminal resize - forward to all active PTYs
                self.handle_resize(process_manager, new_size)?;
            }
            _ => {}
        }
        Ok(false)
    }

    fn update_terminal_title(
        &self,
        process_manager: &ProcessManager,
        osc: &mut OscController<'_>,
    ) -> miette::Result<()> {
        let title = format!("PTYMux - {}", process_manager.active_name());
        osc.set_title_and_tab(&title)?;
        Ok(())
    }

    fn handle_resize(
        &self,
        process_manager: &mut ProcessManager,
        new_size: crate::Size,
    ) -> miette::Result<()> {
        // Update manager's size (full terminal size)
        process_manager.update_terminal_size(new_size);

        // Forward reduced size to all PTY sessions to reserve status bar space
        let pty_size = Size {
            width: new_size.width,
            height: new_size.height - STATUS_BAR_HEIGHT,  // Reserve status bar space
        };

        for process in process_manager.processes() {
            if let Some(session) = &process.session {
                session.input_event_ch_tx_half.send(
                    PtyInputEvent::Resize(pty_size)
                )?;
            }
        }
        Ok(())
    }
}
```

### 5. Create `pty_mux/output_renderer.rs` - Dynamic Display Management

```rust
use crate::{
    core::terminal_io::OutputDevice,
    Size,
};
use super::{ProcessManager, ProcessOutput};
use crossterm::{cursor, style, terminal, execute};

const STATUS_BAR_HEIGHT: u16 = 1;

pub struct OutputRenderer {
    terminal_size: Size,
}

impl OutputRenderer {
    pub fn new(terminal_size: Size) -> Self {
        Self { terminal_size }
    }

    pub fn render(
        &mut self,
        output: ProcessOutput,
        output_device: &OutputDevice,
        process_manager: &ProcessManager,
    ) -> miette::Result<()> {
        match output {
            ProcessOutput::Active(data) => {
                // Write active process output directly to terminal
                write!(
                    lock_output_device_as_mut!(output_device),
                    "{}",
                    String::from_utf8_lossy(&data)
                )?;
            }
            ProcessOutput::ProcessSwitch { from: _from, to: _to } => {
                // Clear screen - the newly active TUI app will repaint itself
                self.clear_screen(output_device)?;
            }
        }

        // Always render status bar after output
        self.render_status_bar(output_device, process_manager)?;
        Ok(())
    }

    fn clear_screen(&self, output_device: &OutputDevice) -> miette::Result<()> {
        execute!(
            lock_output_device_as_mut!(output_device),
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0)
        )?;
        Ok(())
    }

    fn render_status_bar(
        &self,
        output_device: &OutputDevice,
        process_manager: &ProcessManager,
    ) -> miette::Result<()> {
        // Move to bottom line
        execute!(
            lock_output_device_as_mut!(output_device),
            cursor::MoveTo(0, self.terminal_size.height - STATUS_BAR_HEIGHT),
            style::SetBackgroundColor(style::Color::DarkGrey),
            style::SetForegroundColor(style::Color::White),
            terminal::Clear(terminal::ClearType::CurrentLine)
        )?;

        // Show process tabs with live status indicators: 1:[🟢claude] 2:[🔴btop] etc.
        for (i, process) in process_manager.processes().iter().enumerate() {
            let is_active = i == process_manager.active_index();
            let status_indicator = if process.is_running { "🟢" } else { "🔴" };

            // Highlight active process with different background/text color
            if is_active {
                write!(
                    lock_output_device_as_mut!(output_device),
                    " {}:[{}{}] ",
                    i + 1, status_indicator, process.name
                )?;
            } else {
                write!(
                    lock_output_device_as_mut!(output_device),
                    " {}:[{}{}] ",
                    i + 1, status_indicator, process.name
                )?;
            }
        }

        // Show dynamic keyboard shortcuts based on process count
        let process_count = process_manager.processes().len();
        let shortcuts = if process_count <= 4 {
            // For 1-4 processes, show explicit numbers
            match process_count {
                1 => "  Ctrl+1: Switch | Ctrl+Q: Quit",
                2 => "  Ctrl+1/2: Switch | Ctrl+Q: Quit",
                3 => "  Ctrl+1/2/3: Switch | Ctrl+Q: Quit",
                4 => "  Ctrl+1/2/3/4: Switch | Ctrl+Q: Quit",
                _ => "  Ctrl+Q: Quit",
            }
        } else {
            // For 5+ processes, show range notation
            &format!("  Ctrl+1-{}: Switch | Ctrl+Q: Quit", std::cmp::min(process_count, 9))
        };

        let available_width = self.terminal_size.width as usize;
        let current_pos = process_manager.processes().len() * 15; // Rough estimate

        if current_pos + shortcuts.len() < available_width {
            write!(
                lock_output_device_as_mut!(output_device),
                "{}",
                shortcuts
            )?;
        }

        // Reset colors
        execute!(
            lock_output_device_as_mut!(output_device),
            style::ResetColor
        )?;

        Ok(())
    }

}
```

## Phase 3: Simple Example Implementation

Create `tui/examples/pty_mux_example.rs`:

```rust
use r3bl_tui::{
    core::pty_mux::{PTYMux, Process},
    set_mimalloc_in_main,
};

#[tokio::main]
async fn main() -> miette::Result<()> {
    set_mimalloc_in_main!();

    // Configure TUI processes - dynamic support for any number of processes (1-9)
    // This example shows 4 processes, but you can add more or remove some
    let processes = vec![
        Process::new("claude", "claude", vec![]),
        Process::new("btop", "btop", vec![]),
        Process::new("gitui", "gitui", vec![]),
        Process::new("iotop", "iotop", vec![]),
        // Add more processes here if needed:
        // Process::new("nvtop", "nvtop", vec![]),
        // Process::new("htop", "htop", vec![]),
    ];

    // Build and run multiplexer using the pty_mux module
    let multiplexer = PTYMux::builder()
        .processes(processes)
        .build()?;

    // Run the multiplexer event loop
    multiplexer.run().await?;

    Ok(())
}
```

## Implementation Checklist

### Phase 1: OSC Module Enhancements

- [ ] Add new OSC codes to `osc/osc_codes.rs`
- [ ] Extend `osc/osc_event.rs` with new event types
- [ ] Create `osc/osc_controller.rs` with OSC sequence methods
- [ ] Update `osc/mod.rs` to export new controller
- [ ] Test OSC sequence generation and terminal integration

### Phase 2: PTYMux Module Creation

- [ ] Create `pty_mux/mod.rs` with public API exports
- [ ] Implement `pty_mux/multiplexer.rs` with main PTYMux orchestrator
- [ ] Build `pty_mux/process_manager.rs` for PTY lifecycle management
- [ ] Create `pty_mux/input_router.rs` for dynamic keyboard input handling (Ctrl+1-9)
- [ ] Implement `pty_mux/output_renderer.rs` for dynamic display management
- [ ] Add pty_mux module to `tui/src/core/mod.rs`

### Phase 3: Example Implementation

- [ ] Create `tui/examples/pty_mux_example.rs` using PTYMux
- [ ] Test with different numbers of processes (1-9)
- [ ] Verify dynamic keyboard shortcuts work correctly
- [ ] Test terminal title updates via OSC
- [ ] Validate fake resize repainting works
- [ ] Validate clean shutdown and resource cleanup

### Phase 4: Testing & Documentation

- [ ] Unit tests for each pty_mux module component
- [ ] Integration tests for full PTYMux functionality with dynamic process counts
- [ ] Test with various terminal emulators
- [ ] Document keyboard shortcuts and features
- [ ] Add example to CI build if appropriate

## Testing Strategy

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
- **Terminal Compatibility**: Test across different terminal emulators

### Manual Testing

1. Run example with all target TUI processes available
2. Test with missing processes (fallback behavior)
3. Verify process switching triggers proper repaints
4. Test terminal resize handling
5. Validate OSC title updates in terminal
6. Test process exit and restart scenarios

## Future Enhancements

### Short Term

- [ ] Add more TUI process options (`iotop`, `nvtop`, `gitui`)
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

## Architecture Benefits

1. **TUI-Optimized Design**: Leverages `SIGWINCH` for proper display restoration
2. **Low Memory Footprint**: Minimal buffering since TUI apps maintain their own state
3. **Simple Implementation**: Much simpler than full virtual terminal emulation
4. **Maximum Code Reuse**: Leverages existing r3bl_tui infrastructure
5. **Clean Module Organization**: OSC controller in OSC module, PTYMux logic in pty_mux module
6. **Simple Example**: Minimal code in the example file
7. **Extensible Design**: Easy to add features like split panes, more OSC codes, support for more
   processes
8. **Testable Components**: Each module can be unit tested independently
9. **Resource Management**: Proper cleanup using existing RawMode and PTY infrastructure
10. **Better Compatibility**: Works with any TUI app that responds to `SIGWINCH` correctly
11. **Dynamic Process Support**: Automatically adapts UI and input handling to any number of
    processes (1-9)
