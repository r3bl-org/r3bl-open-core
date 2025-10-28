<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Dual Channel PTY Interactive Design](#dual-channel-pty-interactive-design)
  - [Overview](#overview)
  - [Implementation plan](#implementation-plan)
    - [Step 1: Core API Implementation [COMPLETE]](#step-1-core-api-implementation-complete)
    - [Step 2: Integration with Existing Components [COMPLETE]](#step-2-integration-with-existing-components-complete)
    - [Step 3: Testing and Validation [COMPLETE]](#step-3-testing-and-validation-complete)
  - [Core API Design](#core-api-design)
    - [Main Function](#main-function)
    - [PtySession Structure](#ptysession-structure)
    - [Input Types (TO Child)](#input-types-to-child)
    - [Output Types (FROM Child)](#output-types-from-child)
    - [Configuration (Using Existing Types)](#configuration-using-existing-types)
  - [Usage Examples](#usage-examples)
    - [Example 1: Python REPL Interaction](#example-1-python-repl-interaction)
    - [Example 2: Interactive Shell Commands](#example-2-interactive-shell-commands)
    - [Example 3: SSH Session](#example-3-ssh-session)
    - [Example 4: Controlling another TUI application (Vim)](#example-4-controlling-another-tui-application-vim)
  - [Implementation Considerations](#implementation-considerations)
    - [1. Writer Task](#1-writer-task)
    - [2. Resource Management](#2-resource-management)
    - [3. Error Handling](#3-error-handling)
    - [4. Buffering Strategy](#4-buffering-strategy)
    - [5. Flush Implementation](#5-flush-implementation)
  - [Code Refactoring and Shared Components](#code-refactoring-and-shared-components)
    - [Shared Components (common_impl.rs)](#shared-components-common_implrs)
    - [Modified Existing Types](#modified-existing-types)
    - [Refactoring Strategy](#refactoring-strategy)
  - [Comparison with Read-Only Function](#comparison-with-read-only-function)
  - [Future Enhancements](#future-enhancements)
  - [Testing Strategy](#testing-strategy)
  - [Security Considerations](#security-considerations)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Dual Channel PTY Interactive Design

## Overview

This document outlines the design for `spawn_pty_capture_output_and_provide_input`, a bidirectional
PTY communication function that allows both reading from and writing to a child process running in a
pseudo-terminal. This function will be implemented in
`tui/src/core/pty/spawn_pty_read_write_channels.rs`.

**Design Philosophy**: The API treats input and output channels as dumb pipes of events, making no
assumptions about the child process. The child process itself determines terminal modes
(cooked/raw), interprets terminal environment variables, and handles all terminal-specific behavior.
We simply provide the transport layer for bidirectional communication.

## Implementation plan

The dual channel PTY design has been successfully implemented and deployed. This section documents
the completed work.

### Step 1: Core API Implementation [COMPLETE]

Implemented the bidirectional PTY communication function with proper channel management.

- [x] Design and implement `spawn_pty_capture_output_and_provide_input` function
- [x] Create `PtySession` structure with sender/receiver channels
- [x] Implement `PtyInput` and `PtyEvent` enum types
- [x] Handle process lifecycle and resource cleanup

### Step 2: Integration with Existing Components [COMPLETE]

Integrated the dual channel implementation with the existing PTY infrastructure.

- [x] Refactor shared components into common_impl.rs
- [x] Update existing read-only function to use shared implementation
- [x] Ensure compatibility with PtyConfig and CommandBuilder
- [x] Add proper error handling and status conversion

### Step 3: Testing and Validation [COMPLETE]

Validated the implementation with comprehensive test cases.

- [x] Tested Python REPL interaction
- [x] Tested interactive shell command execution
- [x] Tested SSH session handling
- [x] Validated error handling and edge cases

## Core API Design

### Main Function

```rust
pub fn spawn_pty_capture_output_and_provide_input(
    command: CommandBuilder,
    config: impl Into<PtyConfig>,
) -> miette::Result<PtySession>
```

### PtySession Structure

```rust
pub struct PtySession {
    /// Send input TO the child process
    pub stdin: UnboundedSender<PtyInput>,

    /// Receive output FROM the child process
    pub stdout: UnboundedReceiver<PtyEvent>,

    /// Await completion
    pub handle: Pin<Box<JoinHandle<miette::Result<ExitStatus>>>>,
}
```

### Input Types (TO Child)

```rust
pub enum PtyInput {
    /// Send raw bytes to child's stdin
    Write(Vec<u8>),

    /// Send text with automatic newline
    WriteLine(String),

    /// Send control sequences (Ctrl-C, Ctrl-D, etc.)
    SendControl(ControlChar),

    /// Resize the PTY window
    Resize(PtySize),

    /// Explicit flush without writing data
    /// Forces any buffered data to be sent to the child immediately
    Flush,

    /// Close stdin (EOF)
    Close,
}

pub enum ControlChar {
    // Common control characters
    CtrlC,      // SIGINT (interrupt)
    CtrlD,      // EOF (end of file)
    CtrlZ,      // SIGTSTP (suspend)
    CtrlL,      // Clear screen
    CtrlU,      // Clear line
    CtrlA,      // Move to beginning of line
    CtrlE,      // Move to end of line
    CtrlK,      // Kill to end of line

    // Common keys
    Tab,        // Autocomplete
    Enter,      // Newline
    Escape,     // ESC key
    Backspace,
    Delete,

    // Arrow keys
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,

    // Navigation keys
    Home,
    End,
    PageUp,
    PageDown,

    // Function keys (F1-F12)
    F(u8),      // F(1) for F1, F(2) for F2, etc.

    // Raw escape sequence for advanced use cases
    RawSequence(Vec<u8>),
}
```

### Output Types (FROM Child)

```rust
pub enum PtyEvent {
    /// Raw output from the child process
    Output(Vec<u8>),

    /// OSC (Operating System Command) sequences
    Osc(Vec<u8>),

    /// Child process exited normally
    Exit(ExitStatus),

    /// Child process crashed or terminated unexpectedly
    UnexpectedExit(String),

    /// Write operation failed - session will terminate
    /// This gives users a chance to understand why the session ended
    WriteError(std::io::Error),
}
```

### Configuration (Using Existing Types)

The function will use the existing `PtyConfig` and `PtyConfigOption` types:

```rust
use PtyConfigOption::*;

// Examples:
spawn_pty_capture_output_and_provide_input(cmd, Output)?;  // Capture output only
spawn_pty_capture_output_and_provide_input(cmd, Osc + Output)?;  // Capture OSC and output
spawn_pty_capture_output_and_provide_input(cmd, Size(custom_size) + Output)?;  // Custom size
```

## Usage Examples

### Example 1: Python REPL Interaction

```rust
use PtyConfigOption::*;

// Start Python REPL
let cmd = PtyCommandBuilder::new("python3")
    .args(["-u"])  // Unbuffered output
    .build()?;

let mut session = spawn_pty_capture_output_and_provide_input(cmd, Output)?;

// Send Python code
session.stdin.send(PtyInput::WriteLine("print('Hello, World!')".into())).await?;
session.stdin.send(PtyInput::WriteLine("2 + 2".into())).await?;

// Example of explicit flush for partial input (like building a multi-line function)
session.stdin.send(PtyInput::Write(b"def hello():".to_vec())).await?;
session.stdin.send(PtyInput::Flush).await?;  // Ensure prompt updates
session.stdin.send(PtyInput::Write(b"\n    print('hi')".to_vec())).await?;

// Read responses
while let Some(event) = session.stdout.recv().await {
    match event {
        PtyEvent::Output(data) => {
            print!("{}", String::from_utf8_lossy(&data));
        }
        PtyEvent::Exit(status) => {
            println!("Python exited with: {:?}", status);
            break;
        }
        _ => {}
    }
}
```

### Example 2: Interactive Shell Commands

```rust
use PtyConfigOption::*;

// Start bash shell
let cmd = PtyCommandBuilder::new("bash")
    .args(["--norc"])  // Skip RC files for predictable behavior
    .build()?;

let mut session = spawn_pty_capture_output_and_provide_input(cmd, Osc + Output)?;

// Execute commands
session.stdin.send(PtyInput::WriteLine("ls -la".into())).await?;
session.stdin.send(PtyInput::WriteLine("pwd".into())).await?;

// Send Ctrl-C to interrupt a command
session.stdin.send(PtyInput::WriteLine("sleep 100".into())).await?;
tokio::time::sleep(Duration::from_secs(1)).await;
session.stdin.send(PtyInput::SendControl(ControlChar::CtrlC)).await?;

// Exit the shell
session.stdin.send(PtyInput::WriteLine("exit".into())).await?;

// Wait for completion
let exit_status = session.handle.await??;
```

### Example 3: SSH Session

```rust
use PtyConfigOption::*;

// SSH to remote server
let cmd = PtyCommandBuilder::new("ssh")
    .args(["user@server.com"])
    .build()?;

let mut session = spawn_pty_capture_output_and_provide_input(
    cmd,
    Size(PtySize { rows: 40, cols: 120, pixel_width: 0, pixel_height: 0 }) + Output
)?;

// Handle password prompt
tokio::spawn(async move {
    while let Some(event) = session.stdout.recv().await {
        match event {
            PtyEvent::Output(data) => {
                let text = String::from_utf8_lossy(&data);
                print!("{}", text);

                // Detect password prompt and respond
                if text.contains("password:") {
                    session.stdin.send(PtyInput::WriteLine("secret".into())).await?;
                }
            }
            _ => {}
        }
    }
});
```

### Example 4: Controlling another TUI application (Vim)

```rust
use PtyConfigOption::*;
use tempfile::TempDir;

// Create temp directory for test
let temp_dir = TempDir::new()?;
let file_path = temp_dir.path().join("test.txt");

// Start vim in the temp directory
let cmd = PtyCommandBuilder::new("vim")
    .args([file_path.to_str().unwrap()])
    .current_dir(temp_dir.path())
    .build()?;

let mut session = spawn_pty_capture_output_and_provide_input(cmd, Output)?;

// Wait for vim to start
tokio::time::sleep(Duration::from_millis(500)).await;

// Enter insert mode
session.stdin.send(PtyInput::SendControl(ControlChar::Escape)).await?;
session.stdin.send(PtyInput::Write(b"i".to_vec())).await?;

// Type "hello world"
session.stdin.send(PtyInput::Write(b"hello world".to_vec())).await?;

// Exit insert mode
session.stdin.send(PtyInput::SendControl(ControlChar::Escape)).await?;

// Save and quit (:wq)
session.stdin.send(PtyInput::Write(b":wq".to_vec())).await?;
session.stdin.send(PtyInput::SendControl(ControlChar::Enter)).await?;

// Wait for vim to exit
let exit_status = session.handle.await??;
assert!(exit_status.success());

// Verify file was created with expected content
let content = std::fs::read_to_string(&file_path)?;
assert_eq!(content, "hello world\n");
```

## Implementation Considerations

### 1. Writer Task

A second `spawn_blocking` task will be needed to write to the PTY:

```rust
let writer_task = tokio::task::spawn_blocking(move || {
    let mut controller_writer = controller.try_clone_writer()?;

    while let Some(input) = input_receiver.recv() {
        match input {
            PtyInput::Write(bytes) => {
                controller_writer.write_all(&bytes)?;
                controller_writer.flush()?;
            }
            PtyInput::WriteLine(text) => {
                controller_writer.write_all(text.as_bytes())?;
                controller_writer.write_all(b"\n")?;
                controller_writer.flush()?;
            }
            PtyInput::SendControl(ctrl) => {
                let bytes = control_char_to_bytes(ctrl);  // Helper function
                controller_writer.write_all(&bytes)?;
                controller_writer.flush()?;
            }
            PtyInput::Resize(size) => {
                controller.resize(size)?;
            }
            PtyInput::Flush => {
                // Explicit flush without writing any data
                // This ensures all previously written data reaches the child
                controller_writer.flush()?;
            }
            PtyInput::Close => break,
        }
    }
    Ok(())
});
```

### 2. Resource Management

- Both reader and writer tasks must be properly cleaned up
- The controlled (slave) side must be dropped after child exits
- Input channel should be closed when child exits
- Handle three termination scenarios:
  1. Child process self-terminates (normal or crash)
  2. We explicitly terminate the session
  3. Unexpected termination (report as UnexpectedExit event)

### 3. Error Handling

- Write errors terminate the session (no automatic retry)
- Send `PtyEvent::WriteError` before termination for visibility
- Report unexpected child exit via event channel
- Handle SIGPIPE when child dies unexpectedly
- Propagate all errors to caller for visibility

### 4. Buffering Strategy

- Use unbounded channels for simplicity
- No backpressure handling (trust the OS PTY buffer)
- No max buffer size limits

### 5. Flush Implementation

The `PtyInput::Flush` variant provides explicit control over when buffered data is sent to the
child:

- **How it works**: Calls `flush()` on the PTY master's writer without writing new data
- **Use case**: Useful for protocols sensitive to message boundaries or timing
- **Implementation**: The PTY master's internal buffers (if any) are flushed to the kernel
- **Note**: Most writes already include automatic flush, but Flush gives explicit control
- **Example**: Sending a partial command, then Flush to ensure it reaches the child before
  continuing

## Code Refactoring and Shared Components

### Shared Components (common_impl.rs)

The following components should be extracted from `spawn_pty_capture_output_no_input` into a shared
module:

1. **PTY Setup and Initialization**
   - Creating the PTY system and pair
   - Configuring PTY dimensions
   - Type aliases (Controller, Controlled, ControlledChild)

2. **Reader Task Logic**
   - The `spawn_blocking` reader task implementation
   - Buffer management (READ_BUFFER_SIZE constant)
   - OSC sequence processing logic
   - Event sending patterns

3. **Resource Management Patterns**
   - PTY lifecycle management (critical drop ordering)
   - File descriptor cleanup
   - Task synchronization patterns

### Modified Existing Types

1. **PtyEvent Enum** - Extend with new variants:

   ```rust
   pub enum PtyEvent {
       // Existing
       Osc(OscEvent),
       Output(Vec<u8>),
       Exit(portable_pty::ExitStatus),
       // New variants
       UnexpectedExit(String),
       WriteError(std::io::Error),
   }
   ```

2. **New Types in pty_core.rs**:
   - PtyInput enum
   - ControlChar enum
   - PtySession struct
   - control_char_to_bytes helper function

### Refactoring Strategy

1. **Phase 1: Extract Common Code**
   - Create `common_impl.rs` with shared PTY setup logic
   - Extract reader task as `create_reader_task()`
   - Move constants and type aliases

2. **Phase 2: Implement Writer Task**
   - Add `create_writer_task()` in common_impl.rs
   - Handle all PtyInput variants
   - Implement error propagation

3. **Phase 3: Integrate**
   - Update `spawn_pty_capture_output_no_input` to use common_impl
   - Implement `spawn_pty_capture_output_and_provide_input` using both reader and writer
   - Ensure backward compatibility

## Comparison with Read-Only Function

| Feature            | `spawn_pty_capture_output_no_input` | `spawn_pty_capture_output_and_provide_input` |
| ------------------ | ----------------------------------- | -------------------------------------------- |
| Read from child    | [COMPLETE]                          | [COMPLETE]                                   |
| Write to child     | [BLOCKED]                           | [COMPLETE]                                   |
| OSC capture        | [COMPLETE]                          | [COMPLETE]                                   |
| Raw output capture | [COMPLETE]                          | [COMPLETE]                                   |
| Control sequences  | [BLOCKED]                           | [COMPLETE]                                   |
| PTY resize         | [BLOCKED]                           | [COMPLETE]                                   |
| Use cases          | Monitoring, logging                 | REPL, SSH, shells                            |

## Future Enhancements

1. **Terminal emulation**: Full VT100/xterm emulation for parsing output
2. **Expect-like functionality**: Pattern matching on output to automate interactions
3. **Recording/playback**: Record sessions for replay or testing
4. **Multiplexing**: Share single PTY session across multiple consumers
5. **Timeout support**: Configurable timeouts for read/write operations

## Testing Strategy

1. **Unit tests**: Test input encoding (control chars, escape sequences)
2. **Integration tests**: Test with actual commands (echo, cat, etc.)
3. **Interactive tests**: Test with Python/Node.js REPLs
4. **TUI test**: Vim test case (create file, verify content)
5. **Edge cases**: Test child crash, write errors, early termination
6. **Platform focus**: Linux first, then macOS, Windows last

## Security Considerations

1. **Input sanitization**: Option to escape/validate input
2. **Command injection**: Careful with shell command construction
3. **Password handling**: Secure methods for sending passwords
4. **Resource limits**: Prevent unbounded buffer growth
