# R3BL Open Core Project Overview

## Purpose
R3BL Open Core is a Rust monorepo focused on building TUI (Terminal User Interface) applications and libraries. The project aims to create modern, powerful CLI and TUI experiences from the ground up in Rust, using an innovative async, immediate mode reactive UI architecture.

## Tech Stack
- **Language**: Rust (Edition 2024)
- **Build System**: Cargo with workspace configuration
- **Scripts**: Nushell (nu) for unified build/dev scripts
- **Architecture**: Async, immediate mode reactive UI
- **Memory Allocator**: mimalloc for performance optimization

## Key Architectural Innovations
- Purely async, immediate mode reactive UI (every state change triggers render from scratch)
- Non-blocking main thread (unlike traditional POSIX readline approaches)
- Clean separation between rendering and state mutation
- Cross-platform support (Linux, macOS, Windows)
- SSH-optimized rendering (paint only diffs)

## Workspace Structure
The monorepo contains these main crates:
- **`tui/`**: Core TUI library (`r3bl_tui`)
- **`cmdr/`**: Binary applications (`r3bl-cmdr`) including:
  - `edi`: Markdown editor with advanced rendering
  - `giti`: Interactive git workflows  
  - `rc`: Command runner/launcher
- **`analytics_schema/`**: Analytics data structures
- **`docs/`**: Comprehensive documentation and planning

## Applications in cmdr/
- **edi**: Beautiful Markdown editor with syntax highlighting and advanced features
- **giti**: Interactive git workflows made easy
- **rc**: Command runner that provides interactive access to other tools

## Key Features
- Full TUI (raw mode, alternate screen, full async)
- Partial TUI (async readline, choice-based interaction)
- CSS-like styling with JSX-inspired layouts
- Gradient color support with terminal capability detection
- Markdown parser with syntax highlighting
- Process orchestration
- Async REPL infrastructure