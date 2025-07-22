<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Task: Remove Crossterm - Direct ANSI Terminal Control](#task-remove-crossterm---direct-ansi-terminal-control)
  - [Overview](#overview)
  - [Current Architecture Analysis](#current-architecture-analysis)
    - [Render Pipeline Flow](#render-pipeline-flow)
    - [Performance Bottleneck](#performance-bottleneck)
  - [Why Direct ANSI is Feasible](#why-direct-ansi-is-feasible)
  - [Platform Compatibility](#platform-compatibility)
    - [ANSI Support by Platform](#ansi-support-by-platform)
    - [Windows Virtual Terminal Processing](#windows-virtual-terminal-processing)
  - [Implementation Plan](#implementation-plan)
    - [Phase 1: Direct ANSI Backend (Immediate Performance Win)](#phase-1-direct-ansi-backend-immediate-performance-win)
      - [1.1 Create New Backend Structure](#11-create-new-backend-structure)
      - [1.2 Key Implementation Details](#12-key-implementation-details)
      - [1.3 Integration Points](#13-integration-points)
    - [Phase 2: Optimization Opportunities](#phase-2-optimization-opportunities)
      - [2.1 Batched Writing](#21-batched-writing)
      - [2.2 Sequence Optimization](#22-sequence-optimization)
      - [2.3 Pre-computed Sequences](#23-pre-computed-sequences)
    - [Phase 3: Complete Crossterm Removal](#phase-3-complete-crossterm-removal)
      - [3.1 Cross-Platform Input Handling](#31-cross-platform-input-handling)
      - [3.2 Remove Crossterm Dependencies](#32-remove-crossterm-dependencies)
  - [Benefits](#benefits)
    - [Performance](#performance)
    - [Architecture](#architecture)
    - [Maintainability](#maintainability)
  - [Testing Strategy](#testing-strategy)
  - [Migration Timeline](#migration-timeline)
  - [Risks and Mitigation](#risks-and-mitigation)
  - [Success Metrics](#success-metrics)
  - [Conclusion](#conclusion)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Task: Remove Crossterm - Direct ANSI Terminal Control

## Overview

This document outlines the plan to remove the crossterm dependency and implement direct ANSI escape
sequence generation for terminal control. This optimization targets the 15M samples bottleneck
identified in `write_command_ansi` from flamegraph profiling.

**Platform Support**: Linux, macOS, and Windows 10+ (all modern platforms with ANSI support).

## Current Architecture Analysis

### Render Pipeline Flow

1. **Input Event** → State generation → App renders to `RenderOps`
2. **RenderOps** → Rendered to `OffscreenBuffer` (PixelChar grid)
3. **OffscreenBuffer** → Diffed with previous buffer → Generate diff chunks
4. **Diff chunks** → Converted back to `RenderOps` for painting
5. **RenderOps execution** → Each op routed through crossterm backend
6. **Crossterm** → Converts to ANSI escape sequences → Queued to stdout → Flushed

### Performance Bottleneck

- 15M samples in ANSI formatting overhead
- Crossterm's command abstraction layer adds unnecessary overhead
- Multiple trait dispatches and error handling for simple ANSI writes

## Why Direct ANSI is Feasible

Every `RenderOp` ultimately becomes simple ANSI escape sequences:

| RenderOp                     | ANSI Sequence           | Notes                                       |
| ---------------------------- | ----------------------- | ------------------------------------------- |
| `EnterRawMode`               | `\x1b[?1049h` + raw mode| Alternate screen + raw mode setup           |
| `ExitRawMode`                | `\x1b[?1049l` + raw mode| Leave alternate screen + restore            |
| `MoveCursorPositionAbs(pos)` | `\x1b[{row};{col}H`     | 1-based indexing                            |
| `ClearScreen`                | `\x1b[2J`               | Clear entire screen                         |
| `SetFgColor(color)`          | Various SGR codes       | Already optimized in `ansi_escape_codes.rs` |
| `SetBgColor(color)`          | Various SGR codes       | Already optimized                           |
| `ResetColor`                 | `\x1b[0m`               | Reset all attributes                        |
| `ApplyColors(style)`         | Combination of SGR      | Set fg/bg from TuiStyle                     |
| `PaintTextWithAttributes`    | SGR + text              | Set attributes then write text              |

## Platform Compatibility

### ANSI Support by Platform

| Platform | ANSI Support | Raw Mode Implementation | Notes |
|----------|--------------|------------------------|-------|
| Linux | Native | termios via libc | Full support |
| macOS | Native | termios via libc | Full support |
| Windows 10+ | Native (with VT enable) | Windows Console API | Enable Virtual Terminal Processing |

### Windows Virtual Terminal Processing

Windows 10+ supports ANSI escape sequences natively, but requires enabling Virtual Terminal Processing:

```rust
#[cfg(windows)]
fn enable_virtual_terminal_processing() -> std::io::Result<()> {
    use winapi::um::consoleapi::SetConsoleMode;
    use winapi::um::wincon::{ENABLE_VIRTUAL_TERMINAL_PROCESSING, ENABLE_PROCESSED_OUTPUT};
    
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut mode = 0;
        GetConsoleMode(handle, &mut mode);
        mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING | ENABLE_PROCESSED_OUTPUT;
        SetConsoleMode(handle, mode);
    }
    Ok(())
}
```

## Implementation Plan

### Phase 1: Direct ANSI Backend (Immediate Performance Win)

#### 1.1 Create New Backend Structure

```
/tui/src/tui/terminal_lib_backends/direct_ansi_backend/
├── mod.rs                    # Main backend implementation
├── render_op_impl.rs         # Direct ANSI implementation using OutputDevice
├── offscreen_buffer_impl.rs  # Direct buffer painting
├── raw_mode/
│   ├── mod.rs               # Platform detection and routing
│   ├── unix.rs              # Linux/macOS implementation using termios
│   └── windows.rs           # Windows implementation using Console API
└── ansi_sequences.rs        # ANSI sequence constants and helpers
```

#### 1.2 Key Implementation Details

**Using OutputDevice Abstraction:**

```rust
impl PaintRenderOp for RenderOpImplDirectAnsi {
    fn paint(
        &mut self,
        skip_flush: &mut bool,
        render_op: &RenderOp,
        window_size: Size,
        local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>, // Already locked!
        is_mock: bool,
    ) {
        match render_op {
            RenderOp::MoveCursorPositionAbs(pos) => {
                // Direct ANSI write to OutputDevice
                write!(locked_output_device, "\x1b[{};{}H",
                       pos.row_index.as_u16() + 1,
                       pos.col_index.as_u16() + 1).ok();
            }
            RenderOp::SetFgColor(color) => {
                // Reuse existing optimized WriteToBuf implementation
                let mut buf = String::new();
                let sgr = color_to_sgr_code(*color, true);
                sgr.write_to_buf(&mut buf).ok();
                locked_output_device.write_all(buf.as_bytes()).ok();
            }
            RenderOp::ClearScreen => {
                locked_output_device.write_all(b"\x1b[2J").ok();
            }
            RenderOp::ResetColor => {
                locked_output_device.write_all(b"\x1b[0m").ok();
            }
            RenderOp::EnterRawMode => {
                enter_raw_mode(window_size, locked_output_device, is_mock);
                *skip_flush = true;
            }
            RenderOp::ExitRawMode => {
                exit_raw_mode(window_size, locked_output_device, is_mock);
                *skip_flush = true;
            }
            // ... implement all other RenderOps
        }
    }
}
```

**Cross-Platform Raw Mode Implementation:**

```rust
// In raw_mode/mod.rs
pub fn enter_raw_mode(
    window_size: Size,
    locked_output_device: LockedOutputDevice<'_>,
    is_mock: bool,
) {
    if !is_mock {
        // Platform-specific raw mode setup
        #[cfg(unix)]
        unix::enable_raw_mode();
        
        #[cfg(windows)]
        windows::enable_raw_mode();
    }

    // Write ANSI sequences using OutputDevice (works on all platforms)
    write!(locked_output_device,
        concat!(
            "\x1b[?1049h",  // Enter alternate screen
            "\x1b[?1000h",  // Enable mouse tracking
            "\x1b[?2004h",  // Enable bracketed paste
            "\x1b[2J",      // Clear screen
            "\x1b[H",       // Move cursor to 0,0
            "\x1b[?25l"     // Hide cursor
        )
    ).ok();

    locked_output_device.flush().ok();
}

// In raw_mode/unix.rs
#[cfg(unix)]
pub fn enable_raw_mode() {
    use libc::{termios, tcgetattr, tcsetattr, STDIN_FILENO, TCSANOW};
    
    unsafe {
        let mut termios: termios = std::mem::zeroed();
        tcgetattr(STDIN_FILENO, &mut termios);
        
        // Store original for restoration
        ORIGINAL_TERMIOS.store(termios);
        
        // Make raw
        libc::cfmakeraw(&mut termios);
        tcsetattr(STDIN_FILENO, TCSANOW, &termios);
    }
}

// In raw_mode/windows.rs
#[cfg(windows)]
pub fn enable_raw_mode() {
    use winapi::um::consoleapi::{GetConsoleMode, SetConsoleMode};
    use winapi::um::processenv::GetStdHandle;
    use winapi::um::winbase::{STD_INPUT_HANDLE, STD_OUTPUT_HANDLE};
    use winapi::um::wincon::{
        ENABLE_VIRTUAL_TERMINAL_PROCESSING, ENABLE_PROCESSED_OUTPUT,
        ENABLE_VIRTUAL_TERMINAL_INPUT
    };
    
    unsafe {
        // Enable VT processing for output
        let output_handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut output_mode = 0;
        GetConsoleMode(output_handle, &mut output_mode);
        output_mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING | ENABLE_PROCESSED_OUTPUT;
        SetConsoleMode(output_handle, output_mode);
        
        // Enable VT input processing
        let input_handle = GetStdHandle(STD_INPUT_HANDLE);
        let mut input_mode = 0;
        GetConsoleMode(input_handle, &mut input_mode);
        
        // Store original for restoration
        ORIGINAL_INPUT_MODE.store(input_mode);
        
        // Disable line input, echo, etc. (similar to raw mode)
        input_mode &= !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT);
        input_mode |= ENABLE_VIRTUAL_TERMINAL_INPUT;
        SetConsoleMode(input_handle, input_mode);
    }
}
```

#### 1.3 Integration Points

1. **Add Backend Variant:**

   ```rust
   pub enum TerminalLibBackend {
       Crossterm,
       DirectAnsi,  // New!
       Termion,
   }
   ```

2. **Update Router:**

   ```rust
   match TERMINAL_LIB_BACKEND {
       TerminalLibBackend::Crossterm => { /* existing */ }
       TerminalLibBackend::DirectAnsi => {
           RenderOpImplDirectAnsi {}.paint(/* params */);
       }
       TerminalLibBackend::Termion => unimplemented!(),
   }
   ```

3. **Feature Flag:**
   ```toml
   [features]
   default = ["crossterm-backend"]
   direct-ansi = []
   crossterm-backend = ["dep:crossterm"]
   ```

4. **Platform Dependencies:**
   ```toml
   [target.'cfg(unix)'.dependencies]
   libc = "0.2"
   
   [target.'cfg(windows)'.dependencies]
   winapi = { version = "0.3", features = ["consoleapi", "processenv", "wincon", "winbase"] }
   ```

### Phase 2: Optimization Opportunities

#### 2.1 Batched Writing

- Pre-allocate buffer for entire frame
- Batch multiple ANSI sequences before writing
- Single flush per frame instead of per-operation

#### 2.2 Sequence Optimization

- **Color changes**: Detect when colors don't change between ops
- **Cursor movement**: Use relative moves (`\x1b[{n}A/B/C/D`) when more efficient
- **Text batching**: Combine adjacent text with same attributes

#### 2.3 Pre-computed Sequences

- Cache frequently used ANSI sequences
- Pre-compute color codes for common colors
- Use lookup tables for all u8 values (already implemented)

### Phase 3: Complete Crossterm Removal

#### 3.1 Cross-Platform Input Handling

```rust
// New cross-platform input device
pub struct DirectAnsiInputDevice {
    #[cfg(unix)]
    poll: mio::Poll,
    #[cfg(unix)]
    stdin_fd: RawFd,
    
    #[cfg(windows)]
    input_handle: HANDLE,
}

impl InputDevice {
    pub fn new_direct_ansi() -> InputDevice {
        InputDevice {
            resource: Box::pin(DirectAnsiEventStream::new()),
        }
    }
}

// Cross-platform ANSI input parsing
fn parse_ansi_input(bytes: &[u8]) -> Option<Event> {
    match bytes {
        b"\x1b[A" => Some(Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))),
        b"\x1b[B" => Some(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))),
        b"\x1b[C" => Some(Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE))),
        b"\x1b[D" => Some(Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE))),
        // Function keys
        b"\x1bOP" => Some(Event::Key(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE))),
        b"\x1bOQ" => Some(Event::Key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE))),
        // Mouse events (same format on all platforms with VT enabled)
        b"\x1b[M" => parse_mouse_event(&bytes[3..]),
        // ... etc
    }
}

#[cfg(unix)]
impl DirectAnsiEventStream {
    async fn poll_input(&mut self) -> Option<Vec<u8>> {
        // Use mio for async input on Unix
        self.poll.poll(&mut self.events, Some(Duration::from_millis(10)))?;
        // Read from stdin
    }
}

#[cfg(windows)]
impl DirectAnsiEventStream {
    async fn poll_input(&mut self) -> Option<Vec<u8>> {
        // Use ReadConsoleInput or async overlapped I/O
        unsafe {
            let mut buffer = vec![0u8; 256];
            let mut bytes_read = 0;
            ReadFile(self.input_handle, buffer.as_mut_ptr(), 256, &mut bytes_read, null_mut());
            if bytes_read > 0 {
                buffer.truncate(bytes_read as usize);
                Some(buffer)
            } else {
                None
            }
        }
    }
}
```

#### 3.2 Remove Crossterm Dependencies

- Remove from Cargo.toml
- Update all imports
- Remove crossterm-specific code

## Benefits

### Performance

- **Eliminate 15M samples overhead**: No crossterm command abstraction
- **Reduce allocations**: Direct writes to pre-allocated buffers
- **Better batching**: Full control over when to flush
- **Simpler code path**: Direct ANSI is more transparent

### Architecture

- **Maintain testing**: OutputDevice abstraction unchanged
- **Gradual migration**: Feature flags allow switching backends
- **Future proof**: Foundation for custom optimizations
- **Reduced dependencies**: One less major dependency

### Maintainability

- **Direct control**: We own the entire terminal interaction layer
- **Simpler debugging**: ANSI sequences are human-readable
- **Better understanding**: No abstraction hiding what's happening

## Testing Strategy

1. **Unit Tests**: Compare output between crossterm and direct backends
2. **Integration Tests**: Use OutputDevice mocks to verify ANSI sequences
3. **Platform Testing**:
   - **Linux**: Test on Ubuntu, Fedora, Arch
   - **macOS**: Test on macOS 12+ (Monterey and newer)
   - **Windows**: Test on Windows 10 21H2+, Windows 11, Windows Terminal, PowerShell, cmd.exe
4. **Visual Tests**: Side-by-side comparison of rendering
5. **Performance Tests**: Benchmark to verify 15M sample reduction on each platform

## Migration Timeline

1. **Week 1**: Implement Phase 1 (Direct ANSI Backend)
2. **Week 2**: Testing and optimization
3. **Week 3**: Enable by default, keep crossterm as fallback
4. **Week 4**: Begin Phase 3 (Input handling)
5. **Week 5-6**: Complete crossterm removal

## Risks and Mitigation

| Risk                     | Mitigation                                              |
| ------------------------ | ------------------------------------------------------- |
| Platform compatibility   | Use platform-specific code with #[cfg], test thoroughly |
| Windows Console quirks   | Enable VT processing, test on multiple Windows terminals|
| Input parsing complexity | Start with keyboard, add mouse/resize incrementally     |
| Raw mode differences     | Abstract platform differences in raw_mode module        |
| ANSI sequence variations | Stick to well-supported subset, document any quirks     |

## Success Metrics

1. **Performance**: 15M sample reduction in flamegraph on all platforms
2. **Correctness**: All tests pass with new backend on Linux, macOS, and Windows
3. **Compatibility**: 
   - Linux: Works on xterm, gnome-terminal, kitty, alacritty
   - macOS: Works on Terminal.app, iTerm2, kitty, alacritty
   - Windows: Works on Windows Terminal, PowerShell, cmd.exe
4. **Code reduction**: Net reduction in lines of code despite platform-specific implementations

## Conclusion

Removing crossterm in favor of direct ANSI control is feasible across all major platforms. With 
Windows 10+'s native ANSI support via Virtual Terminal Processing, we can use the same ANSI 
sequences everywhere while handling platform-specific raw mode setup. The existing `OutputDevice` 
abstraction makes this transition smooth while preserving testing capabilities. This change will 
improve performance, reduce dependencies, and give us complete control over terminal interactions
on Linux, macOS, and modern Windows systems.
