<!-- cspell:words OPOST -->

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Fix flaky test: test_backend_compat_output_compare

## Status: TODO

## Symptom

`test_backend_compat_output_compare` fails intermittently under parallel test load
(`check.fish --test`). The failure manifests as an `OffscreenBuffer` comparison mismatch:

```text
OffscreenBuffers DIFFER!
85 positions differ:
  [0] Pos [c: 0, r: 0]:  P '3': ^WIDTH$
  [1] Pos [c: 1, r: 0]:  P '8': ^WIDTH$
  [2] Pos [c: 2, r: 0]:  P ';': ^WIDTH$
  [3] Pos [c: 3, r: 0]:  P '2': ^WIDTH$
```

The characters `3`, `8`, `;`, `2` are fragments of an ANSI SGR truecolor sequence
(`\x1b[38;2;R;G;Bm`), indicating that the beginning of the ANSI output was lost.

## Root cause analysis

The `wait_for_ready()` function in `controller::run()` reads bytes in 256-byte chunks
until it finds `CONTROLLED_READY`. It then returns, **discarding everything it read**
including any ANSI bytes that arrived in the same chunk after the ready signal.

### The race timeline

```text
Controlled process                        Controller (wait_for_ready)
------------------------------------------  ---------------------------
println!("CONTROLLED_READY")
flush()
enable_raw_mode()
write ANSI sequences (render ops)
flush()                                     reader.read(&temp[..256])
                                            gets: "CONTROLLED_READY\n\x1b[38;2;..."
                                            finds CONTROLLED_READY -> returns
                                            ANSI bytes after signal = LOST
```

The controlled process calls `println!("CONTROLLED_READY")`, `flush()`, then immediately
enables raw mode and starts writing ANSI output. Under CPU load, the OS may batch these
writes into the same PTY buffer. When the controller reads, it gets both the ready signal
and the beginning of the ANSI output in one chunk, then discards the entire buffer.

### Why it's intermittent

On a lightly loaded machine, the controller reads `CONTROLLED_READY\n` in its own chunk
before the controlled process starts writing ANSI output. On a heavily loaded machine
(parallel tests), the timing shifts and the ANSI output starts arriving before
`wait_for_ready()` returns.

# Implementation plan

## Step 0: Choose the fix approach [COMPLETE]

**Fix**: Modify `wait_for_ready()` to return any **leftover bytes** read after the
`CONTROLLED_READY` signal, and prepend them to the main capture buffer in `run()`.

This is the simplest and most correct fix. The controller should not discard bytes it
has already read from the PTY.

## Step 1: Fix wait_for_ready to preserve leftover bytes

File: `tui/src/core/terminal_io/backend_compat_tests/backend_compat_output_test.rs`

### Step 1.0: Change wait_for_ready return type

Change `wait_for_ready()` to return `Vec<u8>` containing any bytes read after
`CONTROLLED_READY\n`:

```rust
fn wait_for_ready(reader: &mut impl Read, backend_name: &str) -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut temp = [0u8; 256];

    loop {
        match reader.read(&mut temp) {
            Ok(0) => panic!("EOF before controlled ready"),
            Ok(n) => {
                buffer.extend_from_slice(&temp[..n]);
                let text = String::from_utf8_lossy(&buffer);

                if let Some(pos) = text.find(CONTROLLED_READY) {
                    eprintln!("  {backend_name} Controlled is ready");
                    // Return any bytes AFTER "CONTROLLED_READY\n"
                    let signal_end = pos + CONTROLLED_READY.len();
                    // Skip the newline after CONTROLLED_READY if present
                    let leftover_start = if buffer.get(signal_end) == Some(&b'\n') {
                        signal_end + 1
                    } else {
                        signal_end
                    };
                    return buffer[leftover_start..].to_vec();
                }
            }
            Err(e) => panic!("Read error: {e}"),
        }
    }
}
```

### Step 1.1: Update run() to use leftover bytes

In `controller::run()`, seed the capture buffer with the leftover bytes:

```rust
pub fn run((backend_name, pty_pair): (&str, PtyPair)) -> Vec<u8> {
    // ...
    let leftover = wait_for_ready(&mut reader, backend_name);

    let mut all_bytes = leftover;  // Start with any bytes after CONTROLLED_READY
    let mut temp = [0u8; 4096];
    // ... rest unchanged
}
```

## Step 2: Verify the fix

### Step 2.0: Run the test in isolation

```bash
cargo test -p r3bl_tui --lib test_backend_compat_output_compare -- --nocapture
```

### Step 2.1: Run it 100 times in a loop

```bash
for i in $(seq 1 100); do
  cargo test -p r3bl_tui --lib test_backend_compat_output_compare 2>/dev/null
  if [ $? -ne 0 ]; then echo "FAILED on iteration $i"; break; fi
done
```

### Step 2.2: Run check.fish --test for 5 minutes

```bash
for i in $(seq 1 20); do
  ./check.fish --test 2>/dev/null
  if [ $? -ne 0 ]; then echo "FAILED on iteration $i"; break; fi
done
```

## Files to change

- [ ] `tui/src/core/terminal_io/backend_compat_tests/backend_compat_output_test.rs`
  -- Fix wait_for_ready to preserve leftover bytes
