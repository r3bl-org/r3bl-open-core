<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Dual Channel PTY Interactive Design](#dual-channel-pty-interactive-design)
  - [Overview](#overview)
  - [Core API Design](#core-api-design)
    - [Main Function](#main-function)
    - [PtySession Structure](#ptysession-structure)
    - [Input Types (TO Child)](#input-types-to-child)
    - [Configuration (Using Existing Types)](#configuration-using-existing-types)
  - [Usage Examples](#usage-examples)
    - [Example 1: Python REPL Interaction](#example-1-python-repl-interaction)
    - [Example 2: Interactive Shell Commands](#example-2-interactive-shell-commands)
    - [Example 3: SSH Session](#example-3-ssh-session)
    - [Example 4: Controlling another TUI application](#example-4-controlling-another-tui-application)
    - [2. Resource Management](#2-resource-management)
    - [3. Terminal Modes](#3-terminal-modes)
    - [4. Error Handling](#4-error-handling)
    - [5. Buffering Strategy](#5-buffering-strategy)
  - [Comparison with Read-Only Function](#comparison-with-read-only-function)
  - [Future Enhancements](#future-enhancements)
  - [Testing Strategy](#testing-strategy)
  - [Security Considerations](#security-considerations)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Dual Channel PTY Interactive Design

## Overview

This document outlines the design for `spawn_pty_interactive`, a bidirectional PTY communication
function that allows both reading from and writing to a child process running in a pseudo-terminal.

## Core API Design

### Main Function

```rust
pub fn spawn_pty_interactive(
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

    /// Close stdin (EOF)
    Close,
}

pub enum ControlChar {
    CtrlC,  // SIGINT (interrupt)
    CtrlD,  // EOF (end of file)
    CtrlZ,  // SIGTSTP (suspend)
    CtrlL,  // Clear screen
    CtrlU,  // Clear line
    Tab,    // Autocomplete
    Enter,  // Newline
    Escape, // ESC key
    Backspace,
}
```

### Configuration (Using Existing Types)

The function will use the existing `PtyConfig` and `PtyConfigOption` types:

```rust
use PtyConfigOption::*;

// Examples:
spawn_pty_interactive(cmd, Output)?;  // Capture output only
spawn_pty_interactive(cmd, Osc + Output)?;  // Capture OSC and output
spawn_pty_interactive(cmd, Size(custom_size) + Output)?;  // Custom size
```

## Usage Examples

### Example 1: Python REPL Interaction

```rust
use PtyConfigOption::*;

// Start Python REPL
let cmd = PtyCommandBuilder::new("python3")
    .args(["-u"])  // Unbuffered output
    .build()?;

let mut session = spawn_pty_interactive(cmd, Output)?;

// Send Python code
session.stdin.send(PtyInput::WriteLine("print('Hello, World!')".into())).await?;
session.stdin.send(PtyInput::WriteLine("2 + 2".into())).await?;

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

let mut session = spawn_pty_interactive(cmd, Osc + Output)?;

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

let mut session = spawn_pty_interactive(
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

### Example 4: Controlling another TUI application

TODO: Spawn something like vim, in a temp dir, send keystrokes to type "hello", then ":wq" to name
and save the file. at the end see if the file exists and has the expected content.

````rust

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
                let byte = match ctrl {
                    ControlChar::CtrlC => 0x03,
                    ControlChar::CtrlD => 0x04,
                    ControlChar::CtrlZ => 0x1A,
                    // ... etc
                };
                controller_writer.write_all(&[byte])?;
                controller_writer.flush()?;
            }
            PtyInput::Resize(size) => {
                controller.resize(size)?;
            }
            PtyInput::Close => break,
        }
    }
    Ok(())
});
````

### 2. Resource Management

- Both reader and writer tasks must be properly cleaned up
- The controlled (slave) side must be dropped after child exits
- Input channel should be closed when child exits

### 3. Terminal Modes

Consider adding terminal mode configuration:

```rust
pub enum TerminalMode {
    /// Line-buffered input with echo (default)
    Cooked,
    /// Raw input, no echo, no line buffering
    Raw,
    /// Custom termios settings
    Custom(Termios),
}
```

### 4. Error Handling

- Handle write errors gracefully (child might have exited)
- Detect when child process is no longer reading
- Handle partial writes

### 5. Buffering Strategy

- Consider implementing write queue with backpressure
- May need to buffer writes when child is slow to read
- Handle large writes that exceed PTY buffer size

## Comparison with Read-Only Function

| Feature            | `spawn_pty_capture_output_no_input` | `spawn_pty_interactive` |
| ------------------ | ----------------------------------- | ----------------------- |
| Read from child    | ✅                                  | ✅                      |
| Write to child     | ❌                                  | ✅                      |
| OSC capture        | ✅                                  | ✅                      |
| Raw output capture | ✅                                  | ✅                      |
| Control sequences  | ❌                                  | ✅                      |
| PTY resize         | ❌                                  | ✅                      |
| Use cases          | Monitoring, logging                 | REPL, SSH, shells       |

## Future Enhancements

1. **Terminal emulation**: Full VT100/xterm emulation for parsing output
2. **Expect-like functionality**: Pattern matching on output to automate interactions
3. **Recording/playback**: Record sessions for replay or testing
4. **Multiplexing**: Share single PTY session across multiple consumers
5. **Timeout support**: Configurable timeouts for read/write operations

## Testing Strategy

1. **Unit tests**: Test input encoding (control chars, etc.)
2. **Integration tests**: Test with actual commands (echo, cat, etc.)
3. **Interactive tests**: Test with Python/Node.js REPLs
4. **Edge cases**: Test partial writes, buffer overflow, early termination
5. **Platform tests**: Ensure cross-platform compatibility

## Security Considerations

1. **Input sanitization**: Option to escape/validate input
2. **Command injection**: Careful with shell command construction
3. **Password handling**: Secure methods for sending passwords
4. **Resource limits**: Prevent unbounded buffer growth
