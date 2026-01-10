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
      - [In Terminal Multiplexer Mode (Local)](#in-terminal-multiplexer-mode-local)
      - [In Chi Input Helper Mode](#in-chi-input-helper-mode)
      - [In Chi History Browser Mode](#in-chi-history-browser-mode)
      - [In Network Discovery Mode](#in-network-discovery-mode)
      - [In Remote Control Mode (as Controller)](#in-remote-control-mode-as-controller)
      - [In Remote Control Mode (as Controlled)](#in-remote-control-mode-as-controlled)
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
  - [Remote Control Mode](#remote-control-mode)
    - [Remote Control Architecture](#remote-control-architecture)
    - [Network Thread Architecture](#network-thread-architecture)
    - [State Machine](#state-machine)
    - [Network Protocol](#network-protocol)
    - [Output Transport: OffscreenBuffer Serialization](#output-transport-offscreenbuffer-serialization)
    - [Controlled Mode Display](#controlled-mode-display)
    - [Network Discovery TUI (`chi --network`)](#network-discovery-tui-chi---network)
    - [Security Model](#security-model)
    - [Core Data Structures](#core-data-structures-1)
    - [Viewport Handling](#viewport-handling)
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
  - [Phase 1: Local Mode](#phase-1-local-mode)
    - [Step 1: Core Infrastructure [PENDING]](#step-1-core-infrastructure-pending)
    - [Step 2: Multiplexer Logic [PENDING]](#step-2-multiplexer-logic-pending)
    - [Step 3: Helper Apps & Testing [PENDING]](#step-3-helper-apps--testing-pending)
  - [Phase 2: Remote Control Mode](#phase-2-remote-control-mode)
    - [Step 4: Network Thread Infrastructure [PENDING]](#step-4-network-thread-infrastructure-pending)
    - [Step 5: mDNS Discovery [PENDING]](#step-5-mdns-discovery-pending)
    - [Step 6: TLS Communication [PENDING]](#step-6-tls-communication-pending)
    - [Step 7: Network Discovery TUI [PENDING]](#step-7-network-discovery-tui-pending)
    - [Step 8: Controller Mode [PENDING]](#step-8-controller-mode-pending)
    - [Step 9: Controlled Mode [PENDING]](#step-9-controlled-mode-pending)
    - [Step 10: Integration & Testing [PENDING]](#step-10-integration--testing-pending)
    - [Step 11: Analytics & Documentation [PENDING]](#step-11-analytics--documentation-pending)
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
2. Implement four distinct application modes (multiplexer, input helper, history browser,
   network/remote)
3. Enable seamless workflow between Claude and helper tools
4. Maintain Claude running in background during tool usage
5. Enable Remote Control Mode for multi-host collaboration and remote access
6. Integrate analytics to track feature usage
7. Achieve feature parity with all planned interactions

### Key Benefits

1. **Seamless Workflow**: Quick switching between Claude and helper tools
2. **Enhanced Productivity**: Dedicated UIs for input composition and history browsing
3. **Context Preservation**: Claude continues running in background during tool usage
4. **Simple Integration**: All communication via clipboard and plain text
5. **Remote Access**: Control chi sessions from any machine on the LAN via mDNS discovery
6. **Multi-Host Collaboration**: Share Claude sessions across multiple workstations

## Architecture Overview

### System Architecture

```
┌─────────────────────────────────────────────────────┐
│                    CHI BINARY                       │
│                                                     │
│  Command Line Parsing                               │
│  ├─ No args     → Terminal Multiplexer Mode         │
│  ├─ --input     → Input Helper TUI Mode             │
│  ├─ --history   → History Browser TUI Mode          │
│  └─ --network   → Network Discovery TUI Mode        │
│                                                     │
│  Runtime Modes (from Multiplexer)                   │
│  ├─ Local Mode     → Normal operation (Claude PTY)  │
│  └─ Remote Control → Controller or Controlled role  │
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

# Network/Remote Control Mode
chi --network         # Start network discovery TUI
chi --network --psk <key>  # Use pre-shared key for connections

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
switch_to_network = "ctrl+n"
exit = "ctrl+q"
break_remote_control = "ctrl+shift+esc"  # Break out of controlled mode

[ui]
show_mode_banner = true
banner_position = "top"  # top, bottom
theme = "default"        # Theme selection

[network]
# Pre-shared key for Remote Control Mode (optional, can also be passed via --psk)
# Generate with: openssl rand -base64 32
psk = ""                    # Leave empty to prompt on connection
mdns_service_name = "_chi._tcp.local"
listen_port = 0             # 0 = auto-assign, or specify fixed port
advertise = true            # Advertise via mDNS when in network mode
```

### Key Bindings

#### In Terminal Multiplexer Mode (Local)

| Key Combination | Action                        |
| --------------- | ----------------------------- |
| `Ctrl+I`        | Switch to Chi Input Helper    |
| `Ctrl+H`        | Switch to Chi History Browser |
| `Ctrl+N`        | Switch to Network Discovery   |
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

#### In Network Discovery Mode

| Key Combination | Action                                       |
| --------------- | -------------------------------------------- |
| `Enter`         | Connect to selected peer (become Controller) |
| `Esc`           | Return to Claude (exit network mode)         |
| `↑/↓`, `j/k`    | Navigate discovered peers                    |
| `a`             | Accept incoming connection request           |
| `r`             | Refresh peer list                            |
| All other keys  | Handle in TUI navigation                     |

#### In Remote Control Mode (as Controller)

| Key Combination  | Action                                   |
| ---------------- | ---------------------------------------- |
| `Ctrl+Shift+Esc` | End remote control, return to local mode |
| All other keys   | Forward to controlled chi instance       |

#### In Remote Control Mode (as Controlled)

| Key Combination  | Action                                        |
| ---------------- | --------------------------------------------- |
| `Ctrl+Shift+Esc` | Break remote control, regain local control    |
| All other keys   | Ignored (remote controller has input control) |

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
│   ├── history_browser/          # Chi --history TUI app
│   │   ├── mod.rs
│   │   └── app.rs                # Direct reuse of cmdr/ch logic
│   └── network/                  # NEW: Remote Control Mode
│       ├── mod.rs                # Module exports
│       ├── discovery_browser/    # Chi --network TUI app
│       │   ├── mod.rs
│       │   └── app.rs            # Peer list and connection UI
│       ├── network_thread/       # Dedicated network I/O thread (like mio_poller)
│       │   ├── mod.rs
│       │   ├── thread.rs         # Main thread loop (mDNS + TLS)
│       │   ├── mdns_handler.rs   # mDNS advertise/discover logic
│       │   ├── tls_server.rs     # Accept incoming connections
│       │   ├── tls_client.rs     # Connect to peers
│       │   └── protocol.rs       # ChiNetworkMessage serialization
│       ├── controller.rs         # Controller mode logic
│       ├── controlled.rs         # Controlled mode logic (puppet mode)
│       └── types.rs              # NetworkEvent, NetworkCommand, PeerInfo
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
├── tui/editor/                   # REUSED: Editor component for chi --input
│   └── ...
└── tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/
    └── ...                       # REFERENCE: Pattern for network_thread

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
#
# For Remote Control Mode, we follow the mio_poller pattern:
# - Dedicated std::thread for blocking I/O (mDNS, TLS)
# - tokio::sync::broadcast channel for event distribution
# - mpsc channel for commands to the thread
# - Lifecycle state management for thread restart capability
```

### Core Data Structures

#### Terminal Multiplexer

```rust
enum ActivePty {
    // Local modes
    Claude,           // Primary: claude command (raw mode app)
    ChiInput,         // Helper: chi --input (TUI app)
    ChiHistory,       // Helper: chi --history (TUI app)

    // Network modes (Remote Control)
    NetworkDiscovery, // Chi --network TUI (peer list)
    RemoteController, // Controlling another chi instance
    RemoteControlled, // Being controlled by another chi instance
}

struct TerminalMultiplexer {
    // Active mode
    active_pty: ActivePty,

    // PTY Sessions (Local Mode)
    claude_pty: PtyReadWriteSession,               // Always running (in local mode)
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

    // Network (Remote Control Mode)
    network_handle: Option<NetworkHandle>,         // Handle to network thread
    remote_session: Option<RemoteSession>,         // Active remote control session
    original_viewport: Option<Size>,               // Stored when entering controlled mode
}

/// Active remote control session state
struct RemoteSession {
    session_id: String,
    role: RemoteRole,
    peer_hostname: String,
    started_at: Instant,
}

enum RemoteRole {
    Controller,  // We are controlling a remote chi
    Controlled,  // We are being controlled by a remote chi
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

## Remote Control Mode

Remote Control Mode enables multi-host collaboration by allowing one chi instance to control another
chi instance running on a different machine on the same LAN. This leverages mDNS for zero-config
discovery and TLS with pre-shared keys (PSK) for secure communication.

### Remote Control Architecture

```
┌───────────────────────────────────────────────────────────────────────────────┐
│                        REMOTE CONTROL TOPOLOGY                                 │
│                                                                               │
│  ┌──────────────────────────┐              ┌────────────────────────────────┐ │
│  │  CONTROLLER CHI          │   TLS/TCP    │  CONTROLLED CHI                │ │
│  │  (e.g., nazmul-desktop)  │─────────────▶│  (e.g., nazmul-laptop)         │ │
│  │                          │              │                                │ │
│  │  • Displays remote view  │   Input ──▶  │  • Runs chi in "puppet mode"   │ │
│  │  • Captures local input  │              │  • Executes all commands       │ │
│  │  • User interaction here │  ◀── Output  │  • Sends OffscreenBuffer bytes │ │
│  │  • Sets viewport size    │              │  • Local display blanked       │ │
│  └──────────────────────────┘              └────────────────────────────────┘ │
└───────────────────────────────────────────────────────────────────────────────┘
```

### Network Thread Architecture

Similar to the `mio_poller` pattern, Remote Control Mode uses a dedicated thread for network I/O:

```
┌───────────────────────────────────────────────────────────────────────────────┐
│                       NETWORK THREAD ARCHITECTURE                              │
│                                                                               │
│  ┌────────────────────────────────────┐    ┌────────────────────────────────┐ │
│  │ Network Thread (std::thread)       │    │ Main Thread (tokio runtime)    │ │
│  │                                    │    │                                │ │
│  │ mDNS (mdns-sd crate):              │───▶│ rx.recv().await for:           │ │
│  │   • Advertise "_chi._tcp.local"    │    │   • PeerDiscovered { info }    │ │
│  │   • Discover other chi instances   │    │   • PeerLost { hostname }      │ │
│  │                                    │    │   • RemoteControlRequest       │ │
│  │ TLS Server (rustls):               │    │   • InputEvent (as controlled) │ │
│  │   • Accept incoming connections    │◀───│   • OutputFrame (as controller)│ │
│  │   • Validate PSK                   │    │                                │ │
│  │                                    │    │ Orchestrates mode transitions  │ │
│  │ TLS Client (rustls):               │    │                                │ │
│  │   • Connect to discovered peers    │    │                                │ │
│  │   • Send input events              │    │                                │ │
│  └────────────────────────────────────┘    └────────────────────────────────┘ │
└───────────────────────────────────────────────────────────────────────────────┘
```

### State Machine

```
                     ┌──────────────────────────────────────┐
                     │            LOCAL MODE                │
                     │  (Normal chi operation with Claude)  │
                     └──────────────────┬───────────────────┘
                                        │
                          User presses Ctrl+N or
                          starts with chi --network
                                        │
                                        ▼
                     ┌──────────────────────────────────────┐
                     │          DISCOVERY MODE              │
                     │  • mDNS advertising this instance    │
                     │  • Shows list of discovered peers    │
                     │  • User can select peer to control   │
                     │  • User can accept incoming requests │
                     └──────────────────┬───────────────────┘
                                        │
              ┌─────────────────────────┼─────────────────────────┐
              │                         │                         │
    User selects peer         Accepts incoming           Esc to cancel
    and presses Enter         request (press 'a')
              │                         │                         │
              ▼                         ▼                         ▼
┌─────────────────────┐   ┌─────────────────────┐   ┌────────────────────┐
│   CONTROLLER MODE   │   │   CONTROLLED MODE   │   │    LOCAL MODE      │
│                     │   │                     │   │                    │
│ • Displays remote   │   │ • "Puppet mode"     │   │ (back to Claude)   │
│   OffscreenBuffer   │   │ • Receives input    │   │                    │
│ • Sends InputEvents │   │ • Sends output      │   │                    │
│ • Full chi control  │   │ • Local display:    │   │                    │
│   (incl. Ctrl+I/H)  │   │   "Being controlled │   │                    │
│                     │   │    from [host]      │   │                    │
│                     │   │    Ctrl+Shift+Esc   │   │                    │
│                     │   │    to break"        │   │                    │
└────────┬────────────┘   └────────┬────────────┘   └────────────────────┘
         │                         │
         └─────────┬───────────────┘
                   │
         Either end presses Ctrl+Shift+Esc
         or connection is lost
                   │
                   ▼
         ┌─────────────────────┐
         │    LOCAL MODE       │
         │ (graceful restore)  │
         └─────────────────────┘
```

### Network Protocol

```rust
/// Chi Network Protocol - all messages are serialized with bincode
/// and prefixed with length (big-endian u32)
pub enum ChiNetworkMessage {
    // === Discovery Phase ===
    /// Sent via mDNS TXT record or initial handshake
    Announce {
        hostname: String,
        username: String,
        chi_version: String,
        session_info: Option<SessionInfo>,
    },

    // === Control Negotiation ===
    /// Controller requests to take control
    RequestControl {
        controller_hostname: String,
        viewport_size: Size,
        psk_hash: [u8; 32],  // SHA-256 hash of PSK for verification
    },
    /// Controlled accepts the request
    AcceptControl {
        session_id: String,
        controlled_hostname: String,
    },
    /// Controlled rejects the request
    RejectControl {
        reason: String,
    },

    // === During Remote Control ===
    /// Input event from Controller → Controlled
    InputEvent(InputEvent),
    /// Raw OffscreenBuffer bytes from Controlled → Controller
    OutputFrame {
        width: u16,
        height: u16,
        buffer: Vec<u8>,  // Serialized OffscreenBuffer
    },
    /// Viewport resize notification (Controller → Controlled)
    ViewportResize(Size),

    // === Session Management ===
    /// Graceful end of remote control (either direction)
    EndRemoteControl {
        reason: EndReason,
    },
    /// Keep connection alive
    Heartbeat,
}

pub enum EndReason {
    UserRequested,      // Ctrl+Shift+Esc pressed
    Timeout,            // Heartbeat timeout
    Error(String),      // Something went wrong
}

pub struct SessionInfo {
    pub current_mode: ActiveMode,  // Claude, ChiInput, ChiHistory
    pub project_path: Option<String>,
}
```

### Output Transport: OffscreenBuffer Serialization

When in Controlled Mode, chi sends its `OffscreenBuffer` to the Controller:

```rust
/// Output frame sent from Controlled → Controller
impl ControlledChi {
    fn send_output_frame(&self) -> ChiNetworkMessage {
        // Serialize the current OffscreenBuffer state
        let buffer_bytes = self.offscreen_buffer.serialize();

        ChiNetworkMessage::OutputFrame {
            width: self.terminal_size.width,
            height: self.terminal_size.height,
            buffer: buffer_bytes,
        }
    }
}

/// Controller receives and renders the frame
impl ControllerChi {
    fn handle_output_frame(&mut self, frame: OutputFrame) {
        // Deserialize into local OffscreenBuffer
        let remote_buffer = OffscreenBuffer::deserialize(&frame.buffer);

        // Render to local terminal
        self.render_offscreen_buffer(&remote_buffer);
    }
}
```

### Controlled Mode Display

When a chi instance enters Controlled Mode, the local display shows:

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│                                                                 │
│                                                                 │
│          ┌─────────────────────────────────────────┐            │
│          │                                         │            │
│          │   🖥️  REMOTE CONTROLLED                 │            │
│          │                                         │            │
│          │   Being controlled from:                │            │
│          │   nazmul-desktop.local                  │            │
│          │                                         │            │
│          │   Session ID: abc-123-def               │            │
│          │   Duration: 00:05:23                    │            │
│          │                                         │            │
│          │   Press Ctrl+Shift+Esc to break         │            │
│          │   remote control and regain access      │            │
│          │                                         │            │
│          └─────────────────────────────────────────┘            │
│                                                                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Network Discovery TUI (`chi --network`)

```
┌─ CHI NETWORK ───────────────────────────────────────────────────┐
│                                                                 │
│  Advertising as: nazmul-desktop.local (port 54321)              │
│  PSK: ●●●●●●●● (configured)                                     │
│                                                                 │
│ ┌─ Discovered Peers ────────────────────────────────────────┐   │
│ │                                                           │   │
│ │  > nazmul-laptop.local                                    │   │
│ │      User: nazmul | Mode: Claude | Project: ~/github/roc  │   │
│ │                                                           │   │
│ │    nazmul-mac.local                                       │   │
│ │      User: nazmul | Mode: ChiInput | Project: ~/code/app  │   │
│ │                                                           │   │
│ └───────────────────────────────────────────────────────────┘   │
│                                                                 │
│ ┌─ Incoming Requests ───────────────────────────────────────┐   │
│ │                                                           │   │
│ │  (none)                                                   │   │
│ │                                                           │   │
│ └───────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Enter: Connect | a: Accept request | r: Refresh | Esc: Back    │
└─────────────────────────────────────────────────────────────────┘
```

### Security Model

Remote Control Mode uses Pre-Shared Keys (PSK) for simplicity on trusted LANs:

1. **PSK Configuration**: Users configure the same PSK on all machines (via `chi.toml` or `--psk`)
2. **TLS with PSK**: Connections use TLS 1.3 with PSK cipher suites (no certificates needed)
3. **Hash Verification**: PSK hash is included in `RequestControl` to verify both sides have the key
4. **No Key Exchange**: Unlike mTLS, no certificate infrastructure required

```toml
# ~/.config/r3bl-cmdr/chi.toml
[network]
# Generate with: openssl rand -base64 32
psk = "K7xH2mN9pQ3rT5vW8yA1bC4dE6fG0hI2jL8kM5nO7qR="
```

### Core Data Structures

```rust
/// Extended ActivePty enum for Remote Control Mode
enum ActivePty {
    // Local modes (existing)
    Claude,
    ChiInput,
    ChiHistory,

    // Network modes (new)
    NetworkDiscovery,      // Showing peer list TUI
    RemoteController,      // Controlling a remote chi
    RemoteControlled,      // Being controlled by remote chi
}

/// Network thread handle and communication
struct NetworkHandle {
    /// Channel to receive network events
    event_receiver: broadcast::Receiver<NetworkEvent>,

    /// Channel to send commands to network thread
    command_sender: mpsc::Sender<NetworkCommand>,

    /// Thread join handle for graceful shutdown
    thread_handle: Option<JoinHandle<()>>,
}

/// Events from network thread → main thread
enum NetworkEvent {
    PeerDiscovered { info: PeerInfo },
    PeerLost { hostname: String },
    IncomingControlRequest { from: String, viewport: Size },
    ControlAccepted { session_id: String },
    ControlRejected { reason: String },
    InputReceived { event: InputEvent },       // When controlled
    OutputReceived { frame: OutputFrame },     // When controller
    ConnectionLost { reason: String },
    Heartbeat,
}

/// Commands from main thread → network thread
enum NetworkCommand {
    StartAdvertising,
    StopAdvertising,
    ConnectToPeer { hostname: String, viewport: Size },
    AcceptIncomingRequest { from: String },
    RejectIncomingRequest { from: String, reason: String },
    SendInput { event: InputEvent },
    SendOutput { frame: OutputFrame },
    EndSession { reason: EndReason },
    Shutdown,
}

/// Information about a discovered peer
struct PeerInfo {
    hostname: String,
    username: String,
    port: u16,
    chi_version: String,
    session_info: Option<SessionInfo>,
    last_seen: Instant,
}
```

### Viewport Handling

When the Controller connects, its terminal size becomes the viewport for the Controlled:

```rust
impl ControlledChi {
    fn handle_control_request(&mut self, request: RequestControl) {
        // Store original terminal size for restoration later
        self.original_viewport = self.terminal_size;

        // Resize internal viewport to match controller's terminal
        self.resize_viewport(request.viewport_size);

        // Resize the Claude PTY to match
        self.claude_pty.resize(request.viewport_size);
    }

    fn end_remote_control(&mut self) {
        // Restore original viewport
        self.resize_viewport(self.original_viewport);
        self.claude_pty.resize(self.original_viewport);
    }
}
```

## Technical Requirements

### Dependencies

```toml
# EXISTING DEPENDENCIES (no changes needed):

# Already available from cmdr crate itself:
# - History parsing       # cmdr/src/ch/ (same crate - direct access)
# - Config management     # cmdr/src/analytics_client/config_folder.rs

# Already available from tui crate dependency:
# - PTY infrastructure    # tui/src/core/pty/ (spawn_read_write, etc.)
# - TUI framework         # tui/src/tui/ (main_event_loop, editor, etc.)
# - Clipboard service     # tui/src/tui/editor/editor_buffer/clipboard_*.rs
# - mio_poller pattern    # tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/

# Already available transitive dependencies:
# - serde_json           # For ~/.claude.json parsing
# - tokio                # Async runtime
# - miette              # Error handling
# - copypasta_ext       # Clipboard backend (already used by editor)
# - bincode             # Binary serialization (used in tcp-api-server)
# - rustls              # TLS implementation

# NEW DEPENDENCIES FOR REMOTE CONTROL MODE:
[dependencies]
mdns-sd = "0.11"         # mDNS service discovery (zero-config LAN discovery)
# Note: rustls already available, will be used with PSK cipher suites
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

    // Remote Control Mode Events (NEW)
    ChiNetworkStart,                // chi --network launched or Ctrl+N pressed
    ChiNetworkPeerDiscovered,       // New peer discovered via mDNS
    ChiNetworkConnectAttempt,       // User initiated connection to peer
    ChiNetworkConnectSuccess,       // Successfully connected to peer
    ChiNetworkConnectFailed,        // Connection to peer failed
    ChiNetworkIncomingAccepted,     // Accepted incoming control request
    ChiNetworkIncomingRejected,     // Rejected incoming control request
    ChiRemoteControlStarted,        // Remote control session began (either role)
    ChiRemoteControlEnded,          // Remote control session ended
    ChiRemoteControlDuration,       // Track remote session length
    ChiRemoteControlBreak,          // User broke remote control via Ctrl+Shift+Esc
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

The implementation is divided into two phases: **Phase 1** delivers the core local-mode
functionality (multiplexer, input helper, history browser), and **Phase 2** adds Remote Control
Mode.

## Phase 1: Local Mode

### Step 1: Core Infrastructure [PENDING]

**Timeline**: Days 1-2

Core infrastructure and module setup for the Chi application.

**Tasks:**

- [ ] Create `cmdr/src/bin/chi.rs` binary entry point
- [ ] Set up `cmdr/src/chi/` module structure
- [ ] CLI argument parsing (multiplexer vs --input vs --history vs --network modes)
- [ ] Terminal multiplexer core with PTY manager
- [ ] Input router and event handling

### Step 2: Multiplexer Logic [PENDING]

**Timeline**: Day 3

Implement the core terminal multiplexer functionality.

**Tasks:**

- [ ] Claude PTY spawning and management
- [ ] Output buffering system for background Claude
- [ ] Mode switching between Claude and helpers
- [ ] Clipboard integration using tui's SystemClipboard

### Step 3: Helper Apps & Testing [PENDING]

**Timeline**: Days 4-5

Implement the helper applications and comprehensive testing.

**Tasks:**

- [ ] Chi Input mode (wrap tui editor component)
- [ ] Chi History mode (directly use ch module functions)
- [ ] Integration testing of all mode transitions
- [ ] Visual feedback and error handling
- [ ] Documentation and examples

---

## Phase 2: Remote Control Mode

### Step 4: Network Thread Infrastructure [PENDING]

**Timeline**: Days 6-7

Build the dedicated network thread following the `mio_poller` pattern.

**Tasks:**

- [ ] Create `cmdr/src/chi/network/` module structure
- [ ] Implement `NetworkHandle` with broadcast channel for events
- [ ] Implement `NetworkCommand` mpsc channel for commands
- [ ] Add thread lifecycle management (spawn, graceful shutdown, restart capability)
- [ ] Define `ChiNetworkMessage` protocol with bincode serialization

### Step 5: mDNS Discovery [PENDING]

**Timeline**: Day 8

Implement zero-config peer discovery using mDNS.

**Tasks:**

- [ ] Integrate `mdns-sd` crate for service discovery
- [ ] Implement service advertisement (`_chi._tcp.local`)
- [ ] Implement peer discovery and tracking
- [ ] Add `PeerInfo` with hostname, user, port, session info
- [ ] Handle peer timeout/expiry (stale peer removal)

### Step 6: TLS Communication [PENDING]

**Timeline**: Days 9-10

Implement secure communication using TLS with PSK.

**Tasks:**

- [ ] Configure `rustls` with PSK cipher suites
- [ ] Implement TLS server to accept incoming connections
- [ ] Implement TLS client to connect to peers
- [ ] Add PSK configuration via `chi.toml` and `--psk` flag
- [ ] Implement PSK hash verification in `RequestControl`

### Step 7: Network Discovery TUI [PENDING]

**Timeline**: Day 11

Build the Network Discovery browser (`chi --network`).

**Tasks:**

- [ ] Create `discovery_browser/app.rs` TUI
- [ ] Display discovered peers list with navigation
- [ ] Show incoming connection requests
- [ ] Implement key bindings (Enter, 'a', 'r', Esc)
- [ ] Add status bar showing local advertisement info

### Step 8: Controller Mode [PENDING]

**Timeline**: Days 12-13

Implement the Controller role (controlling a remote chi).

**Tasks:**

- [ ] Implement connection initiation with viewport negotiation
- [ ] Forward local `InputEvent`s to remote via `ChiNetworkMessage::InputEvent`
- [ ] Receive and render `OutputFrame` (OffscreenBuffer deserialization)
- [ ] Handle viewport resize propagation
- [ ] Implement `Ctrl+Shift+Esc` to end remote control
- [ ] Graceful disconnection with state restoration

### Step 9: Controlled Mode [PENDING]

**Timeline**: Days 14-15

Implement the Controlled role (being controlled by remote chi).

**Tasks:**

- [ ] Accept incoming control requests with PSK verification
- [ ] Resize viewport to match controller's terminal size
- [ ] Send `OutputFrame` on each render (OffscreenBuffer serialization)
- [ ] Receive and process `InputEvent`s from controller
- [ ] Display "Being Controlled" screen locally
- [ ] Implement `Ctrl+Shift+Esc` to break remote control
- [ ] Restore original viewport on disconnect

### Step 10: Integration & Testing [PENDING]

**Timeline**: Days 16-17

End-to-end testing of Remote Control Mode.

**Tasks:**

- [ ] Integration tests for mDNS discovery across hosts
- [ ] Integration tests for TLS connection establishment
- [ ] Test Controller → Controlled input forwarding
- [ ] Test OffscreenBuffer transport and rendering
- [ ] Test graceful disconnect from both ends
- [ ] Test connection loss handling
- [ ] Test viewport resize during remote session
- [ ] Performance testing (latency, throughput)

### Step 11: Analytics & Documentation [PENDING]

**Timeline**: Day 18

Add analytics events and update documentation.

**Tasks:**

- [ ] Add Remote Control analytics events to `analytics_action.rs`
- [ ] Instrument network and remote control code paths
- [ ] Update README with Remote Control Mode usage
- [ ] Add configuration examples for PSK setup
- [ ] Document troubleshooting for network issues

---

## Conclusion

Chi provides a powerful enhancement to Claude Code workflows by creating a seamless interface
between Claude and helper tools. The design prioritizes simplicity, performance, and maintainability
while providing significant productivity benefits for Claude users.

**Phase 1** delivers the core local-mode functionality: terminal multiplexer with seamless switching
between Claude, input helper, and history browser. The architecture leverages existing PTY
infrastructure and TUI frameworks, minimizing new code while maximizing functionality. The
clipboard-based communication model ensures loose coupling and broad compatibility.

**Phase 2** extends chi with Remote Control Mode, enabling multi-host collaboration through:

- Zero-config mDNS discovery of chi instances on the LAN
- Secure TLS communication with pre-shared keys
- Full remote control including mode switching (Ctrl+I, Ctrl+H)
- OffscreenBuffer-based output transport for efficient rendering
- Graceful connection management from either end

This PRD provides sufficient detail for implementation by any developer familiar with Rust, terminal
applications, and network programming.
