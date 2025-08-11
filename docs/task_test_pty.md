# Testing PTY-based OSC Sequence Capture

## Overview

Testing PTY-based code that captures OSC sequences is challenging because it requires spawning
processes with pseudo-terminals. This document outlines strategies for testing the OSC sequence
capture functionality in `real.rs`.

## Testing Approaches

### 1. Unit Test the Parser Components

The easiest win - test `OscBuffer` and OSC parsing logic in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc_buffer_parsing() {
        let mut buffer = OscBuffer::new();
        let input = b"\x1b]9;4;1;50\x1b\\other text\x1b]9;4;0;0\x1b\\";
        let events = buffer.append_and_extract(input, input.len());
        assert_eq!(events, vec![
            OscEvent::ProgressUpdate(50),
            OscEvent::ProgressCleared
        ]);
    }
}
```

### 2. Integration Tests with Smaller Scope

Test with `printf` or `echo` commands that output known sequences instead of full cargo builds.

#### Required Refactoring

First, generalize the spawning function from `spawn_cargo_task_with_osc_capture` to:

```rust
fn spawn_pty_task_with_osc_capture(
    command: String,
    args: Vec<String>,
    event_sender: UnboundedSender<OscEvent>,
) -> Pin<Box<tokio::task::JoinHandle<miette::Result<portable_pty::ExitStatus>>>>
```

## Test Cases to Consider

### Common Cases

1. **Single complete sequence** - `\x1b]9;4;1;50\x1b\\`
2. **Multiple sequences** - `\x1b]9;4;1;25\x1b\\\x1b]9;4;1;50\x1b\\\x1b]9;4;0;0\x1b\\`
3. **Sequences with text between** - `Building...\x1b]9;4;1;30\x1b\\Done!`
4. **All state types**:
   - Progress Update (state 1): 0-100%
   - Clear (state 0)
   - Error (state 2)
   - Indeterminate (state 3)

### Edge Cases

1. **Split sequences across reads** - Simulate partial reads by using `sleep` between outputs
2. **Invalid sequences** - `\x1b]9;4;1\x1b\\` (missing progress value)
3. **Malformed terminators** - `\x1b]9;4;1;50\x1b` (missing `\\`)
4. **Non-numeric values** - `\x1b]9;4;1;abc\x1b\\`
5. **Out of range** - `\x1b]9;4;1;150\x1b\\` (>100%)
6. **Interleaved incomplete** - `\x1b]9;4;1;25\x1b]9;4;1;50\x1b\\` (nested starts)
7. **Empty buffer scenarios**
8. **Very long sequences** - Test buffer limits
9. **Unicode in surrounding text** - Ensure UTF-8 handling works correctly

## Test Implementation Structure

### Basic Integration Test

```rust
#[tokio::test]
async fn test_osc_sequence_parsing() {
    let test_cases = vec![
        ("printf", vec![r"\x1b]9;4;1;50\x1b\\"], vec![OscEvent::ProgressUpdate(50)]),
        ("printf", vec![r"\x1b]9;4;0;0\x1b\\"], vec![OscEvent::ProgressCleared]),
        ("printf", vec![r"\x1b]9;4;2;0\x1b\\"], vec![OscEvent::BuildError]),
        ("printf", vec![r"\x1b]9;4;3;0\x1b\\"], vec![OscEvent::IndeterminateProgress]),
    ];

    for (cmd, args, expected) in test_cases {
        let (sender, mut receiver) = unbounded_channel();
        let handle = spawn_pty_task_with_osc_capture(cmd.into(), args, sender);

        let mut received = vec![];
        while let Ok(event) = receiver.try_recv() {
            received.push(event);
        }

        handle.await??;
        assert_eq!(received, expected);
    }
}
```

### Testing Split Sequences

For testing sequences split across multiple reads:

```bash
# Test script that outputs in chunks with delays
echo -n $'\x1b]9;4;1;' && sleep 0.1 && echo -n '50' && sleep 0.1 && echo $'\x1b\\'
```

Or create a test helper binary:

```rust
// test_emitter.rs - outputs sequences with controlled timing
use std::{thread, time::Duration};

fn main() {
    // Output partial sequence
    print!("\x1b]9;4;1;");
    std::io::stdout().flush().unwrap();
    thread::sleep(Duration::from_millis(100));

    // Complete the sequence
    print!("75\x1b\\");
    std::io::stdout().flush().unwrap();
}
```

## Alternative Testing Strategies

### 3. Mock Command Approach

Create a test binary that emits predictable OSC sequences:

```rust
// test_osc_emitter.rs
fn main() {
    let sequences = std::env::args().skip(1).collect::<Vec<_>>();
    for seq in sequences {
        match seq.as_str() {
            "progress-25" => print!("\x1b]9;4;1;25\x1b\\"),
            "progress-50" => print!("\x1b]9;4;1;50\x1b\\"),
            "clear" => print!("\x1b]9;4;0;0\x1b\\"),
            "error" => print!("\x1b]9;4;2;0\x1b\\"),
            _ => {}
        }
        std::io::stdout().flush().unwrap();
    }
}
```

### 4. Abstraction Layer

Introduce a trait to make PTY system injectable for testing:

```rust
trait PtySystem {
    fn spawn_command(&self, cmd: CommandBuilder) -> Result<(Controller, Child)>;
}

struct RealPtySystem;
struct MockPtySystem { /* mock implementation */ }
```

### 5. Record/Replay Testing

1. Capture real cargo output once
2. Save it to a file
3. Create a mock that replays this output
4. Verify parser handles real-world data correctly

## Platform Considerations

### Cross-platform printf

Different platforms handle escape sequences differently:

- **Linux/macOS**: `printf '\x1b]9;4;1;50\x1b\\'`
- **Windows**: May need special handling or use of test binaries

### PTY Availability

- PTYs may not be available in all CI environments
- Consider feature flags for PTY tests: `#[cfg(feature = "pty_tests")]`

## Testing Utilities

### Helper Functions

```rust
async fn collect_osc_events(
    command: &str,
    args: Vec<String>,
    timeout: Duration,
) -> Vec<OscEvent> {
    let (sender, mut receiver) = unbounded_channel();
    let handle = spawn_pty_task_with_osc_capture(
        command.to_string(),
        args,
        sender,
    );

    let mut events = vec![];
    let timeout_at = tokio::time::Instant::now() + timeout;

    loop {
        tokio::select! {
            Some(event) = receiver.recv() => events.push(event),
            _ = tokio::time::sleep_until(timeout_at) => break,
            result = &mut handle => {
                result??;
                break;
            }
        }
    }

    events
}
```

## Next Steps

1. Start with unit tests for `OscBuffer` - lowest complexity, highest confidence
2. Create a test helper binary for controlled OSC emission
3. Implement integration tests using the helper binary
4. Add CI configuration to run PTY tests where supported
5. Consider property-based testing for the parser (using `proptest` or `quickcheck`)
