# Chi Terminal Multiplexer - Product Requirements Document

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Overview](#overview)
  - [Task Description](#task-description)
  - [Current State](#current-state)
  - [Goals](#goals)
    - [Key Benefits](#key-benefits)
  - [Architecture Overview](#architecture-overview)
    - [System Architecture](#system-architecture)
    - [Terminal Multiplexer Architecture](#terminal-multiplexer-architecture)
    - [State Flow Diagram](#state-flow-diagram)
  - [Detailed Specifications](#detailed-specifications)
    - [Command Line Interface](#command-line-interface)
    - [Configuration (Future)](#configuration-future)
    - [Key Bindings](#key-bindings)
      - [In Terminal Multiplexer Mode](#in-terminal-multiplexer-mode)
      - [In Chi Input Helper Mode](#in-chi-input-helper-mode)
      - [In Chi History Browser Mode](#in-chi-history-browser-mode)
    - [Data Flow & Integration](#data-flow--integration)
      - [Clipboard Integration](#clipboard-integration)
      - [History File Format](#history-file-format)
  - [Implementation Structure](#implementation-structure)
    - [Module Organization](#module-organization)
    - [Core Data Structures](#core-data-structures)
      - [Terminal Multiplexer](#terminal-multiplexer)
      - [History Data (from cmdr/src/ch/types.rs)](#history-data-from-cmdrsrcchtypesrs)
    - [Event Loop Implementation](#event-loop-implementation)
      - [Main Event Loop](#main-event-loop)
      - [Input Routing Logic](#input-routing-logic)
    - [PTY Lifecycle Management](#pty-lifecycle-management)
      - [Spawning Helper PTYs](#spawning-helper-ptys)
      - [Returning to Claude](#returning-to-claude)
    - [Output Buffering Strategy](#output-buffering-strategy)
      - [Claude Output Management](#claude-output-management)
  - [TUI Application Specifications](#tui-application-specifications)
    - [Chi Input Helper (`chi --input`)](#chi-input-helper-chi---input)
      - [Purpose](#purpose)
      - [Features](#features)
      - [UI Layout](#ui-layout)
      - [Implementation](#implementation)
    - [Chi History Browser (`chi --history`)](#chi-history-browser-chi---history)
      - [Purpose](#purpose-1)
      - [Features](#features-1)
      - [UI Layout](#ui-layout-1)
      - [Implementation](#implementation-1)
  - [Technical Requirements](#technical-requirements)
    - [Dependencies](#dependencies)
    - [File System Requirements](#file-system-requirements)
    - [Performance Requirements](#performance-requirements)
    - [Compatibility Requirements](#compatibility-requirements)
  - [User Experience Design](#user-experience-design)
    - [Accessibility Features](#accessibility-features)
    - [Visual Feedback](#visual-feedback)
    - [Transition Animations](#transition-animations)
    - [Error Handling](#error-handling)
  - [Testing Strategy](#testing-strategy)
    - [Unit Tests](#unit-tests)
    - [Integration Tests](#integration-tests)
    - [Manual Testing Scenarios](#manual-testing-scenarios)
  - [Analytics Integration](#analytics-integration)
    - [Analytics Events to Track](#analytics-events-to-track)
    - [Implementation Points](#implementation-points)
    - [Privacy Considerations](#privacy-considerations)
    - [Success Metrics](#success-metrics)
    - [Integration with Existing Infrastructure](#integration-with-existing-infrastructure)
  - [Security Considerations](#security-considerations)
  - [Future Enhancements](#future-enhancements)
    - [Phase 2 Features](#phase-2-features)
    - [Phase 3 Features](#phase-3-features)
- [Implementation plan](#implementation-plan)
  - [Step 1: Core Infrastructure [PENDING]](#step-1-core-infrastructure-pending)
  - [Step 2: Multiplexer Logic [PENDING]](#step-2-multiplexer-logic-pending)
  - [Step 3: Helper Apps & Testing [PENDING]](#step-3-helper-apps--testing-pending)
  - [Conclusion](#conclusion)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Overview

## Task Description

Chi ("claude history interactive") is a unified terminal application that provides seamless
integration between Claude Code and TUI-based helper tools. It operates in three distinct modes
based on command-line arguments, creating a powerful workflow for enhanced Claude Code interaction.

This PRD documents the complete design for a terminal multiplexer that enhances Claude Code usage by
providing quick access to input helpers and command history, all while maintaining Claude as the
primary interface.

## Current State

- **Status**: Design Complete - Ready for Implementation
- **Created**: 2025-01-16
- **Priority**: High
- **Estimated Effort**: 3-5 days (includes analytics integration)
- **Complexity**: Low (mostly integration work within same crate)

The application needs to be built to manage switching between Claude and helper apps (Chi Input
Helper and Chi History Browser), with about 85% code reuse from existing components.

## Goals

1. Create a terminal multiplexer that enhances Claude Code usage
2. Implement three distinct application modes (multiplexer, input helper, history browser)
3. Enable seamless workflow between Claude and helper tools
4. Maintain Claude running in background during tool usage
5. Integrate analytics to track feature usage
6. Achieve feature parity with all planned interactions within 3-5 days

### Key Benefits

1. **Seamless Workflow**: Quick switching between Claude and helper tools
2. **Enhanced Productivity**: Dedicated UIs for input composition and history browsing
3. **Context Preservation**: Claude continues running in background during tool usage
4. **Simple Integration**: All communication via clipboard and plain text

## Architecture Overview

### System Architecture

```
┌─────────────────────────────────────────────────────┐
│                    CHI BINARY                       │
│                                                     │
│  Command Line Parsing                               │
│  ├─ No args     → Terminal Multiplexer Mode         │
│  ├─ --input     → Input Helper TUI Mode             │
│  └─ --history   → History Browser TUI Mode          │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### Terminal Multiplexer Architecture

```
┌──────────────────────────────────────────────────────┐
│              CHI TERMINAL MULTIPLEXER                │
│                                                      │
│  ┌─────────────────────────────────────────────────┐ │
│  │              Real Terminal                      │ │
│  │              (RAW MODE)                         │ │
│  └─────────────────────────────────────────────────┘ │
│                           │                          │
│                           ▼                          │
│  ┌─────────────────────────────────────────────────┐ │
│  │          Input Router & Event Loop              │ │
│  │                                                 │ │
│  │  Ctrl+I ──────┐      ┌────────── Ctrl+H         │ │
│  │               ▼      ▼                          │ │
│  └─────────────────────────────────────────────────┘ │
│                           │                          │
│    ┌──────────────────────┼──────────────────────┐   │
│    ▼                      ▼                      ▼   │
│  ┌─────────┐      ┌──────────────┐      ┌──────────┐ │
│  │ CLAUDE  │      │ CHI --INPUT  │      │CHI --HIST│ │
│  │   PTY   │      │     PTY      │      │    PTY   │ │
│  │         │      │              │      │          │ │
│  │(Always  │      │ (On-demand   │      │(On-demand│ │
│  │Running) │      │  Spawned)    │      │ Spawned) │ │
│  └─────────┘      └──────────────┘      └──────────┘ │
│                                                      │
└──────────────────────────────────────────────────────┘
```

### State Flow Diagram

```
           ┌─────────────────────────────┐
           │      CLAUDE (Primary)       │
           │   ┌─────────────────────┐   │
           │   │ claude command      │   │
           │   │ (Always running)    │   │
           │   └─────────────────────┘   │
           │                             │
     ┌─────┤  Ctrl+I           Ctrl+H    ├──┐
     │     └─────────────────────────────┘  │
     ▼                                      ▼
┌───────────────┐                  ┌─────────────────┐
│ CHI --INPUT   │                  │CHI --HISTORY    │
│               │                  │                 │
│ ┌───────────┐ │                  │ ┌─────────────┐ │
│ │chi --input│ │                  │ │chi --history│ │
│ │    TUI    │ │                  │ │   TUI       │ │
│ └───────────┘ │                  │ └─────────────┘ │
│               │                  │                 │
│ Ctrl+Enter    │                  │ Enter: paste    │
│ (paste &      │                  │ Esc: no paste   │
│  return)      │                  │ (both return)   │
└──────┬────────┘                  └──────┬──────────┘
       │                                  │
       └─────────────────┬────────────────┘
                         ▼
                ┌─────────────────┐
                │ Return to CLAUDE│
                │   + Clipboard   │
                │     Paste       │
                └─────────────────┘
```

## Detailed Specifications

### Command Line Interface

```bash
# Terminal Multiplexer Mode
chi                    # Start multiplexer with Claude + helpers
chi --claude-cmd <cmd> # Use custom command instead of 'claude'

# Helper Modes
chi --input           # Start input composition TUI
chi --history         # Start history browser TUI
chi --history --project <path>  # Use specific project history

# Additional Options
chi --help            # Show help
chi --version         # Show version
chi --config <file>  # Use custom config file (future)
```

### Configuration (Future)

```toml
# Uses existing r3bl-cmdr config infrastructure
# Location: ~/.config/r3bl-cmdr/chi.toml (on Linux/macOS)
#           %APPDATA%/r3bl-cmdr/chi.toml (on Windows)
# Leverages cmdr/src/analytics_client/config_folder.rs

[multiplexer]
claude_command = "claude"  # Or custom wrapper script
buffer_size = 100_000      # Output buffer size in bytes

[keybindings]
switch_to_input = "ctrl+i"
switch_to_history = "ctrl+h"
exit = "ctrl+q"

[ui]
show_mode_banner = true
banner_position = "top"  # top, bottom
theme = "default"        # Theme selection
```

### Key Bindings

#### In Terminal Multiplexer Mode

| Key Combination | Action                        |
| --------------- | ----------------------------- |
| `Ctrl+I`        | Switch to Chi Input Helper    |
| `Ctrl+H`        | Switch to Chi History Browser |
| `Ctrl+Q`        | Exit multiplexer              |
| All other keys  | Pass through to active PTY    |

#### In Chi Input Helper Mode

| Key Combination | Action                             |
| --------------- | ---------------------------------- |
| `Ctrl+Enter`    | Return to Claude + paste clipboard |
| All other keys  | Handle in TUI input component      |

#### In Chi History Browser Mode

| Key Combination | Action                                 |
| --------------- | -------------------------------------- |
| `Enter`         | Return to Claude + paste selected item |
| `Esc`           | Return to Claude (no paste)            |
| `↑/↓`, `j/k`    | Navigate history                       |
| All other keys  | Handle in TUI navigation               |

### Data Flow & Integration

#### Clipboard Integration

```
┌──────────────┐    clipboard    ┌─────────────┐
│              │   ──────────▶   │             │
│ Chi Helper   │                 │   Claude    │
│    Apps      │                 │     PTY     │
│              │                 │             │
└──────────────┘                 └─────────────┘
```

All data transfer between Chi helpers and Claude occurs via:

1. **Existing clipboard service** (`tui/src/tui/editor/editor_buffer/clipboard_service.rs`)
   - Uses `SystemClipboard` with `ClipboardService` trait
   - Proven reliability (already used by TUI editor)
   - Cross-platform support via `copypasta_ext`
2. Plain text paste into Claude PTY via `PtyInputEvent::Text`
3. No direct process communication needed

#### History File Format

Chi History mode reads from `~/.claude.json` using the actual Claude Code format:

```json
{
  "projects": {
    "/path/to/project": {
      "history": [
        {
          "display": "explain this rust code",
          "pastedContents": [
            {
              "type": "text",
              "id": 1,
              "content": "fn main() { println!(\"Hello\"); }"
            }
          ]
        }
      ]
    }
  }
}
```

This leverages existing `ClaudeConfig`, `Project`, and `HistoryItem` structures from
`cmdr/src/ch/types.rs`.

## Implementation Structure

### Module Organization

```
cmdr/src/
├── bin/
│   └── chi.rs                    # Binary entry point (with analytics)
├── chi/                          # New chi module
│   ├── mod.rs                    # Module exports
│   ├── main.rs                   # Entry point & CLI parsing
│   ├── multiplexer/              # Terminal multiplexer
│   │   ├── mod.rs
│   │   ├── terminal_mux.rs       # Main multiplexer logic
│   │   ├── pty_manager.rs        # PTY lifecycle management
│   │   ├── input_router.rs       # Input event routing
│   │   └── output_buffer.rs      # Output buffering
│   ├── input_helper/             # Chi --input TUI app
│   │   ├── mod.rs
│   │   └── app.rs                # Wraps tui/editor component
│   └── history_browser/          # Chi --history TUI app
│       ├── mod.rs
│       └── app.rs                # Direct reuse of cmdr/ch logic
├── ch/                           # REUSED DIRECTLY: History logic for chi --history
│   ├── types.rs                  # ClaudeConfig, HistoryItem structures
│   ├── prompt_history.rs         # File reading/parsing logic
│   ├── choose_prompt.rs          # Prompt selection logic
│   ├── ui_str.rs                # UI string formatting
│   └── ...
└── analytics_client/             # REUSED: Analytics infrastructure
    ├── analytics_action.rs       # Event types (add Chi events)
    ├── report_analytics.rs       # Event reporting
    ├── proxy_machine_id.rs       # Anonymous user tracking
    ├── http_client.rs            # Backend communication
    └── config_folder.rs          # Config & analytics settings

tui/src/
└── tui/editor/                   # REUSED: Editor component for chi --input
    └── ...

# Refactoring Opportunities:

## Since chi.rs is in the same cmdr module as ch/, no refactoring needed!
# The chi binary can directly use the ch module's functionality:
# - ch/types.rs for ClaudeConfig, HistoryItem structures
# - ch/prompt_history.rs for history file parsing
# - ch/choose_prompt.rs for prompt selection logic
# - ch/ui_str.rs for UI string formatting
#
# This eliminates the need for extracting shared modules since
# both ch and chi are in the same crate and can share code directly.
```

### Core Data Structures

#### Terminal Multiplexer

```rust
enum ActivePty {
    Claude,      // Primary: claude command (raw mode app)
    ChiInput,    // Helper: chi --input (TUI app)
    ChiHistory,  // Helper: chi --history (TUI app)
}

struct TerminalMultiplexer {
    // Active mode
    active_pty: ActivePty,

    // PTY Sessions
    claude_pty: PtyReadWriteSession,           // Always running
    chi_input_pty: Option<PtyReadWriteSession>,    // On-demand
    chi_history_pty: Option<PtyReadWriteSession>,  // On-demand

    // Output buffering
    claude_buffer: VecDeque<Vec<u8>>,  // Buffer Claude output when in Chi mode

    // Clipboard for data transfer
    clipboard: SystemClipboard,

    // Terminal state
    terminal_size: Size,
    output_device: OutputDevice,
    input_device: InputDevice,
}
```

#### History Data (from cmdr/src/ch/types.rs)

```rust
/// Root structure for deserializing Claude Code's ~/.claude.json file
#[derive(Debug, Deserialize)]
pub struct ClaudeConfig {
    #[serde(default)]
    pub projects: HashMap<String, Project>,
}

/// Project-specific configuration containing history
#[derive(Debug, Deserialize)]
pub struct Project {
    #[serde(default)]
    pub history: Vec<HistoryItem>,
}

/// Individual history item representing a prompt
#[derive(Debug, Clone, Deserialize)]
pub struct HistoryItem {
    pub display: String,
    #[serde(rename = "pastedContents")]
    pub pasted_contents: serde_json::Value,
}
```

### Event Loop Implementation

#### Main Event Loop

```rust
impl TerminalMultiplexer {
    pub async fn run(mut self) -> miette::Result<()> {
        // Initialize raw mode once for entire session
        RawMode::start(self.terminal_size,
                      lock_output_device_as_mut!(&self.output_device),
                      false);

        // Start Claude immediately
        self.claude_pty = PtyCommandBuilder::new("claude")
            .spawn_read_write(PtyConfigOption::Output)?;

        loop {
            tokio::select! {
                // Input handling
                Some(input) = self.input_device.next_input_event() => {
                    if self.handle_input(input).await? {
                        break; // Exit requested
                    }
                }

                // Claude output (always running)
                Some(event) = self.claude_pty.output_event_receiver_half.recv() => {
                    self.handle_claude_output(event).await?;
                }

                // Chi --input output (when active)
                Some(event) = self.chi_input_output() => {
                    self.handle_chi_output(event, ActivePty::ChiInput).await?;
                }

                // Chi --history output (when active)
                Some(event) = self.chi_history_output() => {
                    self.handle_chi_output(event, ActivePty::ChiHistory).await?;
                }
            }
        }

        // Cleanup
        RawMode::end(self.terminal_size,
                    lock_output_device_as_mut!(&self.output_device),
                    false);
        Ok(())
    }

    // Signal handling (SIGTERM, SIGINT, SIGWINCH)
    fn setup_signal_handlers(&self) {
        // SIGWINCH: Terminal resize
        // - Propagate to all active PTYs
        // - Trigger re-render

        // SIGTERM/SIGINT: Graceful shutdown
        // - Save state if needed
        // - Clean up PTYs
        // - Restore terminal
    }
}
```

#### Input Routing Logic

```rust
async fn handle_input(&mut self, input: InputEvent) -> miette::Result<bool> {
    match (self.active_pty, &input) {
        // === FROM CLAUDE ===
        (ActivePty::Claude, InputEvent::Keyboard(KeyPress::Ctrl('i'))) => {
            self.switch_to_chi_input().await?;
        }
        (ActivePty::Claude, InputEvent::Keyboard(KeyPress::Ctrl('h'))) => {
            self.switch_to_chi_history().await?;
        }
        (ActivePty::Claude, InputEvent::Keyboard(KeyPress::Ctrl('q'))) => {
            return Ok(true); // Exit multiplexer
        }
        (ActivePty::Claude, _) => {
            // Pass through to Claude
            self.claude_pty.input_event_sender_half
                .send(PtyInputEvent::from(input))?;
        }

        // === FROM CHI --INPUT ===
        (ActivePty::ChiInput, InputEvent::Keyboard(KeyPress::CtrlEnter)) => {
            self.return_to_claude_with_paste().await?;
        }
        (ActivePty::ChiInput, _) => {
            // Pass through to chi --input
            if let Some(ref pty) = self.chi_input_pty {
                pty.input_event_sender_half
                    .send(PtyInputEvent::from(input))?;
            }
        }

        // === FROM CHI --HISTORY ===
        (ActivePty::ChiHistory, InputEvent::Keyboard(KeyPress::Plain(Key::Enter))) => {
            self.return_to_claude_with_paste().await?;
        }
        (ActivePty::ChiHistory, InputEvent::Keyboard(KeyPress::Esc)) => {
            self.return_to_claude_no_paste().await?;
        }
        (ActivePty::ChiHistory, _) => {
            // Pass through to chi --history
            if let Some(ref pty) = self.chi_history_pty {
                pty.input_event_sender_half
                    .send(PtyInputEvent::from(input))?;
            }
        }
    }
    Ok(false)
}
```

### PTY Lifecycle Management

#### Spawning Helper PTYs

```rust
impl TerminalMultiplexer {
    async fn switch_to_chi_input(&mut self) -> miette::Result<()> {
        // Spawn fresh chi --input instance
        self.chi_input_pty = Some(
            PtyCommandBuilder::new("chi")
                .args(&["--input"])
                .spawn_read_write(PtyConfigOption::Output)?
        );

        self.active_pty = ActivePty::ChiInput;
        self.clear_and_show_mode_banner("CHI INPUT - Ctrl+Enter to return");
        Ok(())
    }

    async fn switch_to_chi_history(&mut self) -> miette::Result<()> {
        // Spawn fresh chi --history instance
        self.chi_history_pty = Some(
            PtyCommandBuilder::new("chi")
                .args(&["--history"])
                .spawn_read_write(PtyConfigOption::Output)?
        );

        self.active_pty = ActivePty::ChiHistory;
        self.clear_and_show_mode_banner("CHI HISTORY - Enter to paste, Esc to cancel");
        Ok(())
    }
}
```

#### Returning to Claude

```rust
impl TerminalMultiplexer {
    async fn return_to_claude_with_paste(&mut self) -> miette::Result<()> {
        // Get clipboard content before cleanup
        let clipboard_content = self.clipboard.get_contents()?;

        self.cleanup_chi_session().await?;
        self.active_pty = ActivePty::Claude;

        // Restore Claude display
        self.replay_buffered_claude_output();

        // Paste to Claude
        if !clipboard_content.is_empty() {
            self.claude_pty.input_event_sender_half
                .send(PtyInputEvent::Text(clipboard_content))?;
        }

        Ok(())
    }

    async fn return_to_claude_no_paste(&mut self) -> miette::Result<()> {
        self.cleanup_chi_session().await?;
        self.active_pty = ActivePty::Claude;

        // Restore Claude display
        self.replay_buffered_claude_output();

        Ok(())
    }
}
```

### Output Buffering Strategy

#### Claude Output Management

```rust
impl TerminalMultiplexer {
    async fn handle_claude_output(&mut self, event: PtyOutputEvent) -> miette::Result<()> {
        match self.active_pty {
            ActivePty::Claude => {
                // Display directly to terminal
                if let PtyOutputEvent::Output(data) = event {
                    self.write_raw_to_terminal(&data)?;
                }
            }
            _ => {
                // Buffer while in Chi mode
                if let PtyOutputEvent::Output(data) = event {
                    self.claude_buffer.push_back(data);

                    // Limit buffer size (ring buffer behavior)
                    const MAX_BUFFER_SIZE: usize = 100_000;
                    while self.claude_buffer.len() > MAX_BUFFER_SIZE {
                        self.claude_buffer.pop_front();
                    }
                }
            }
        }
        Ok(())
    }

    fn replay_buffered_claude_output(&mut self) {
        // Clear screen first
        self.execute(Clear(ClearType::All)).ok();

        // Replay all buffered output
        while let Some(data) = self.claude_buffer.pop_front() {
            self.write_raw_to_terminal(&data).ok();
        }
    }
}
```

## TUI Application Specifications

### Chi Input Helper (`chi --input`)

#### Purpose

Provides a rich text input interface for composing complex prompts or commands before sending to
Claude.

#### Features

- Multi-line text editor
- Syntax highlighting (optional)
- Character/word count
- Input validation
- Copy to clipboard on exit

#### UI Layout

```
┌─ CHI INPUT ─────────────────────────────────────┐
│                                                 │
│ ┌─ Input Area ────────────────────────────────┐ │
│ │                                             │ │
│ │ [Multi-line text editor]                    │ │
│ │ _                                           │ │
│ │                                             │ │
│ └─────────────────────────────────────────────┘ │
│                                                 │
│ Status: 145 chars | Ctrl+Enter: Send to Claude  │
└─────────────────────────────────────────────────┘
```

#### Implementation

Uses existing TUI framework with `main_event_loop` and **TUI editor component** from
`tui/src/tui/editor/`. This provides:

- Multi-line text editing with full editor functionality
- Syntax highlighting capabilities
- Character/word counting
- Clipboard operations
- Proven editor engine for rich text input

### Chi History Browser (`chi --history`)

#### Purpose

Browse and select from Claude command history stored in `~/.claude.json`.

#### Features

- Scrollable history list
- Search/filter capability
- Preview pane
- Copy selected item to clipboard

#### UI Layout

```
┌─ CHI HISTORY ───────────────────────────────────┐
│                                                 │
│ ┌─ History List ────────┐ ┌─ Preview ─────────┐ │
│ │ > Latest command      │ │ Full command text │ │
│ │   Previous command    │ │ and response      │ │
│ │   Another command     │ │ preview...        │ │
│ │   ...                 │ │                   │ │
│ └───────────────────────┘ └───────────────────┘ │
│                                                 │
│ Status: 15 items | Enter: Paste | Esc: Cancel   │
└─────────────────────────────────────────────────┘
```

#### Implementation

Uses existing TUI framework with **direct access to cmdr/ch history logic** from `cmdr/src/ch/`
(same crate). This provides:

- Existing `ClaudeConfig` and `HistoryItem` data structures
- `get_claude_config_path()` for cross-platform file location
- Proven parsing of `~/.claude.json` format
- `ParsedPastedContents` for handling complex content types
- `ChResult` enum for consistent result handling
- `choose_prompt` module for interactive selection
- `ui_str` for consistent string formatting

Since chi.rs is in the same cmdr crate, it can directly import and use all ch module functionality
without any code duplication or extraction needed.

## Technical Requirements

### Dependencies

```toml
# NO NEW DEPENDENCIES REQUIRED!
# All components already exist in the codebase:

# Already available from cmdr crate itself:
# - History parsing       # cmdr/src/ch/ (same crate - direct access)
# - Config management     # cmdr/src/analytics_client/config_folder.rs

# Already available from tui crate dependency:
# - PTY infrastructure    # tui/src/core/pty/ (spawn_read_write, etc.)
# - TUI framework         # tui/src/tui/ (main_event_loop, editor, etc.)
# - Clipboard service     # tui/src/tui/editor/editor_buffer/clipboard_*.rs

# Already available transitive dependencies:
# - serde_json           # For ~/.claude.json parsing
# - tokio                # Async runtime
# - miette              # Error handling
# - copypasta_ext       # Clipboard backend (already used by editor)
```

### File System Requirements

- Read access to `~/.claude.json` (Claude's history file)
- Read/write access to config directory:
  - Linux/macOS: `~/.config/r3bl-cmdr/`
  - Windows: `%APPDATA%/r3bl-cmdr/`
  - Uses existing `config_folder.rs` infrastructure
- Create/write clipboard temporary files if needed
- Follows XDG Base Directory specification via `dirs` crate

### Performance Requirements

- Startup time: < 100ms for multiplexer mode
- Mode switching: < 50ms visual feedback
- Memory usage: < 50MB for normal operation
- Claude output buffering: Max 100KB ring buffer

### Compatibility Requirements

- Linux: Primary target
- macOS: Secondary target
- Windows: Future consideration
- Terminal: Any that supports ANSI escape sequences and raw mode

## User Experience Design

### Accessibility Features

- **High contrast mode**: Support for users with visual impairments
- **Screen reader compatibility**: Proper ARIA-like terminal announcements
- **Keyboard-only navigation**: All features accessible without mouse
- **Customizable timeouts**: Adjustable delays for mode switching
- **Status announcements**: Audio/visual cues for mode changes

### Visual Feedback

```
┌─ Mode Indicators ─────────────────────────────┐
│ [CLAUDE] - Normal operation                   │
│ [CHI INPUT - Ctrl+Enter to return]            │
│ [CHI HISTORY - Enter: paste, Esc: cancel]     │
└───────────────────────────────────────────────┘
```

### Transition Animations

- Screen clear with brief flash/highlight
- Mode banner appears at top
- Smooth cursor positioning

### Error Handling

- **PTY spawn failures**:
  - Claude PTY failure: Exit with error message (critical)
  - Chi helper PTY failure: Show error, return to Claude mode
  - Retry logic with exponential backoff for transient failures
- **Clipboard access failures**:
  - Show error toast/notification
  - Continue without paste operation
  - Fallback to manual copy/paste instructions
- **History file read failures**:
  - Missing file: Show empty history with helpful message
  - Parse errors: Skip malformed entries, show valid ones
  - Permission denied: Show error with fix instructions
- **Large paste operations**:
  - Chunk into 4KB segments for PTY write
  - Add small delays between chunks to prevent overflow
  - Show progress indicator for very large pastes
- **Terminal resize during mode switch**:
  - Properly propagate resize to active PTY
  - Recalculate layout dimensions
  - Preserve buffer content through resize

## Testing Strategy

### Unit Tests

- Input routing logic
- PTY lifecycle management
- Clipboard integration
- History file parsing

### Integration Tests

- Full mode switching scenarios
- Error recovery paths
- Large output buffering
- Concurrent PTY operation

### Manual Testing Scenarios

1. **Basic Flow**: Claude → Chi Input → Return with paste
2. **History Flow**: Claude → Chi History → Return with selection
3. **Error Cases**: Missing files, clipboard failures, PTY crashes
4. **Performance**: Large Claude outputs, rapid mode switching
5. **Edge Cases**: Very long inputs, special characters, binary data

## Analytics Integration

Chi will integrate with the existing analytics_client infrastructure to collect usage data for
understanding feature adoption and value. This follows the same patterns as ch, edi, and giti
binaries.

### Analytics Events to Track

```rust
// New variants to add to cmdr/src/analytics_client/analytics_action.rs
pub enum AnalyticsAction {
    // ... existing variants ...

    // Chi Terminal Multiplexer Events
    ChiAppStart,                    // Chi multiplexer launched
    ChiFailedToRun,                // Chi failed to start
    ChiModeSwitch,                 // User switched between modes

    // Chi Input Helper Events
    ChiInputStart,                  // chi --input launched
    ChiInputCompleted,              // User completed input (Ctrl+Enter)
    ChiInputCancelled,              // User cancelled input

    // Chi History Browser Events
    ChiHistoryStart,                // chi --history launched
    ChiHistorySelected,             // User selected a history item
    ChiHistoryCancelled,            // User cancelled (Esc)
    ChiHistoryNoItems,              // No history items found

    // Feature Usage Events
    ChiClipboardPaste,              // Clipboard paste to Claude
    ChiSessionDuration,             // Track session length
    ChiBufferOverflow,              // Claude buffer exceeded limit
}
```

### Implementation Points

1. **Startup Analytics** (`cmdr/src/bin/chi.rs`):

```rust
// At binary startup
report_analytics::start_task_to_generate_event(
    String::new(),
    match args {
        None => AnalyticsAction::ChiAppStart,
        Some("--input") => AnalyticsAction::ChiInputStart,
        Some("--history") => AnalyticsAction::ChiHistoryStart,
        _ => AnalyticsAction::ChiAppStart,
    }
);
```

2. **Mode Switching** (`cmdr/src/chi/multiplexer/input_router.rs`):

```rust
// Track mode switches with metadata
report_analytics::start_task_to_generate_event(
    format!("from:{},to:{}", current_mode, new_mode),
    AnalyticsAction::ChiModeSwitch,
);
```

3. **Feature Usage** (`cmdr/src/chi/multiplexer/terminal_mux.rs`):

```rust
// Track clipboard pastes
report_analytics::start_task_to_generate_event(
    format!("size:{}", clipboard_content.len()),
    AnalyticsAction::ChiClipboardPaste,
);
```

### Privacy Considerations

- **No content tracking**: Never send actual prompt content or clipboard data
- **Hashed identifiers**: Use hashed project paths like ch does
- **Opt-out support**: Respect `--no-analytics` flag
- **Minimal metadata**: Only track counts, indices, and timing
- **Rate limiting**: Prevent rapid-fire events from overwhelming

### Success Metrics

After implementation, analytics will answer:

1. **Adoption**: How many users adopt chi vs direct Claude usage?
2. **Feature Usage**: Which helper modes are most valuable?
3. **Workflow Patterns**: Common mode switching sequences
4. **Success Rates**: Completion vs cancellation rates
5. **Performance**: Session durations and buffer overflow frequency

### Integration with Existing Infrastructure

Chi will reuse all existing analytics infrastructure:

- `cmdr/src/analytics_client/report_analytics.rs` - Event reporting
- `cmdr/src/analytics_client/proxy_machine_id.rs` - Anonymous user identification
- `cmdr/src/analytics_client/config_folder.rs` - Analytics settings storage
- `cmdr/src/analytics_client/http_client.rs` - Backend communication

This ensures consistent analytics across all R3BL tools and minimal new code.

## Security Considerations

- **Clipboard sanitization**: Strip dangerous characters before paste
- **Command injection prevention**: Validate all PTY inputs
- **File permissions**: Respect OS file permissions for history
- **Sensitive data handling**:
  - No logging of clipboard contents
  - Clear clipboard after paste (optional)
  - Secure memory wiping for sensitive buffers
- **PTY security**: Run child processes with minimal privileges

## Future Enhancements

### Phase 2 Features

- Configuration file support
- Custom key bindings
- Multiple history sources
- Session recording/replay

### Phase 3 Features

- Plugin system for custom helpers
- Network clipboard sharing
- Integration with external tools
- Advanced text processing

# Implementation plan

## Step 1: Core Infrastructure [PENDING]

**Timeline**: Days 1-2

Core infrastructure and module setup for the Chi application.

**Tasks:**

- [ ] Create `cmdr/src/bin/chi.rs` binary entry point
- [ ] Set up `cmdr/src/chi/` module structure
- [ ] CLI argument parsing (multiplexer vs --input vs --history modes)
- [ ] Terminal multiplexer core with PTY manager
- [ ] Input router and event handling

## Step 2: Multiplexer Logic [PENDING]

**Timeline**: Day 3

Implement the core terminal multiplexer functionality.

**Tasks:**

- [ ] Claude PTY spawning and management
- [ ] Output buffering system for background Claude
- [ ] Mode switching between Claude and helpers
- [ ] Clipboard integration using tui's SystemClipboard

## Step 3: Helper Apps & Testing [PENDING]

**Timeline**: Days 4-5

Implement the helper applications and comprehensive testing.

**Tasks:**

- [ ] Chi Input mode (wrap tui editor component)
- [ ] Chi History mode (directly use ch module functions)
- [ ] Integration testing of all mode transitions
- [ ] Visual feedback and error handling
- [ ] Documentation and examples

## Conclusion

Chi provides a powerful enhancement to Claude Code workflows by creating a seamless interface
between Claude and helper tools. The design prioritizes simplicity, performance, and maintainability
while providing significant productivity benefits for Claude users.

The architecture leverages existing PTY infrastructure and TUI frameworks, minimizing new code while
maximizing functionality. The clipboard-based communication model ensures loose coupling and broad
compatibility.

This PRD provides sufficient detail for implementation by any developer familiar with Rust and
terminal applications.
