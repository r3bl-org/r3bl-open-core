# Task: Fix Windows ConPTY EOF and Session Lifecycle (`fix-win-conpty-eof`)

## Background

When running pseudoterminal (`PTY`) end-to-end integration tests on Windows
(`nazmul-win.local`), tests that spawn interactive child processes hung indefinitely:

```text
test core::pty::e2e_tests::osc_capture_test::test_osc_capture_disabled has been running for over 60 seconds
test core::pty::e2e_tests::osc_capture_test::test_osc_capture_enabled has been running for over 60 seconds
test core::pty::e2e_tests::session_test::test_session_with_cat has been running for over 60 seconds
```

This task resolves the root architectural divergences between POSIX pseudoterminals and
the Windows Console Pseudoterminal (`ConPTY`) subsystem, enabling all PTY end-to-end
integration tests to run cleanly on Windows without deadlocks or platform-specific skips.

---

## Technical Root Cause Analysis

### 1. POSIX PTY vs Windows ConPTY Architecture

In POSIX operating systems (Linux, macOS):

- A pseudoterminal consists of a kernel-managed master and replica pair connected via a
  TTY line discipline driver.
- When the parent process closes the master write file descriptor, the line discipline
  delivers an EOF condition (`EIO` on Linux, 0-byte read on macOS) to the child process.
- When the child process exits, its replica file descriptor is closed, delivering EOF to
  the master reader.

In Windows ConPTY:

- ConPTY is not a kernel TTY driver. It is an IPC layer over a background headless console
  server instance (`conhost.exe`).
- The parent process connects to `conhost.exe` via two anonymous pipes (`hPipeIn` and
  `hPipeOut`).
- `conhost.exe` translates VT/ANSI sequences received from `hPipeIn` into Win32
  `INPUT_RECORD` events inside the child's console input buffer.
- `conhost.exe` reads the child's console screen buffer and formats it as VT/ANSI escape
  sequences written to `hPipeOut`.

```text
Parent Process                   ConPTY (conhost.exe)              Child Process
 [Writer Pipe]   ───(Write)───>  Translates to Win32               [cmd / findstr]
                                 INPUT_RECORD events ─────────>    ReadFile(hStdIn)
                                                                   (Never receives EOF)

 [Reader Pipe]   <───(Read)────  Reads console screen buffer <───  Writes stdout/stderr
                                 (Kept open until HPCON closes)
```

### 2. Failure Discovery 1: ConPTY DSR Cursor Position Freeze

When `portable_pty` allocates a Windows pseudoconsole with `PSEUDOCONSOLE_INHERIT_CURSOR`:

- `conhost.exe` immediately emits a Device Status Report request (`\x1b[6n`) through the
  output pipe to query the terminal's current cursor coordinates.
- Crucially, `conhost.exe` halts all input processing until the terminal controller
  replies with a cursor position report (`\x1b[1;1R`).
- If the controller writes to `hPipeIn` without first parsing and answering `\x1b[6n`,
  ConPTY drops or ignores the input, freezing child process execution indefinitely.
- **Solution**: Implemented `perform_conpty_handshake()` in `orchestrator.rs`. It
  reads from the PTY output pipe until `\x1b[6n` is detected, immediately replies with
  `\x1b[1;1R`, and splices any remaining bytes back onto the reader stream.

### 3. Failure Discovery 2: Premature Writer Pipe Closure (`STATUS_CONTROL_C_EXIT`)

Win32 console handles (`STD_INPUT_HANDLE`) do not support native pipe EOF semantics:

- In cooked mode, Win32 console EOF is signaled by transmitting `Ctrl+Z` (`\x1a`) followed
  by carriage return and newline (`\r\n`).
- However, if the writer task in `r3bl_tui` drops its anonymous write pipe immediately
  after sending `Ctrl+Z`, ConPTY detects that the terminal closed the input pipe while
  console processes were still active.
- Upon writer pipe closure, `conhost.exe` raises `STATUS_CONTROL_C_EXIT` (`0xC000013A`) on
  all child processes attached to the pseudoconsole, killing them instantly before they
  can finish writing their buffers or terminate normally.
- **Solution**: In `writer_task.rs`, on `PtyInputEvent::Close`, we write `\x1a\r\n` but
  keep the writer pipe open (`Continuation::Continue`), letting the child exit normally
  while the orchestrator monitors its lifecycle.

### 4. Failure Discovery 3: ConPTY Teardown Sequence and Reader Deadlock

Even when a child process exits voluntarily, ConPTY does not close `hPipeOut`:

- `conhost.exe` holds the write end of the output pipe open until `ClosePseudoConsole()`
  is invoked on the `HPCON` handle.
- In `r3bl_tui`, `PtySession` spawns a dedicated blocking reader thread reading
  `hPipeOut`.
- If the orchestrator waits for the reader thread to finish reading before tearing down
  the session, a circular deadlock occurs:
    1. The reader thread blocks in `read()` waiting for `hPipeOut` to close (which only
       happens on `ClosePseudoConsole`).
    2. The orchestrator task waits for the reader thread to finish before dropping the
       controller.
- Furthermore, when `ClosePseudoConsole` runs, the reader's blocking `ReadFile` call
  returns `ERROR_BROKEN_PIPE` (`io::ErrorKind::BrokenPipe`).
- **Solution**:
    - In `orchestrator.rs`, the `Controller` is wrapped in
      `Arc<Mutex<Option<Controller>>>`.
    - As soon as `controlled_child.wait()` completes, the orchestrator invokes
      `guard.take()` to drop the `Controller` immediately. This calls
      `ClosePseudoConsole()`, closing `conhost.exe` and breaking `hPipeOut`.
    - In `reader_task.rs`, `ErrorKind::BrokenPipe` is treated as clean EOF on Windows.

### 5. Failure Discovery 4: Cross-Platform Child Command Quirks

Standard commands used in tests have subtle platform incompatibilities under ConPTY:

- `cross_platform_commands::cat()`: On Windows, `findstr.exe ^` acts as an interactive
  passthrough filter matching every line from console input.
- `cross_platform_commands::sleep()`: Windows `timeout.exe` rejects redirected input with
  "ERROR: Input redirection is not supported". It is replaced with
  `powershell.exe -NoProfile -Command "Start-Sleep -Seconds N"`.
- `resize_test.rs`: Chaining commands in `cmd.exe` required appending `& exit` outside
  quotes so the parent command shell terminates promptly upon PowerShell completion.

### 6. Failure Discovery 5: Upstream `portable-pty 0.9.0` Inverted Return Value Bug

In `portable-pty 0.9.0/src/win/mod.rs`, `WinChildKiller::kill()` has an inverted check:

```rust
let res = unsafe { TerminateProcess(self.proc.as_raw_handle() as _, 1) };
let err = IoError::last_os_error();
if res != 0 { Err(err) } else { Ok(()) }
```

- Win32 `TerminateProcess` returns nonzero on success and 0 on failure.
- `portable-pty 0.9.0` returns `Err` on success, causing `kill().expect(...)` to fail in
  tests.
- This was fixed upstream in `wezterm/wezterm` PR #7709 (`if res == 0`), but is not yet
  published to crates.io.
- **Solution**: Implemented `WindowsProcessKiller` and `ControlledChild` newtype in
  `controlled_child.rs`. It borrows the child's raw handle (`child.as_raw_handle()`),
  duplicates it into an independent `OwnedHandle` via `DuplicateHandle`, and calls
  `TerminateProcess` with the correct return check (`if res == 0 { Err } else { Ok }`).
  On non-Windows platforms, it delegates directly to `child.clone_killer()`.

---

## Architectural Sequence Diagram

```text
Parent Process (r3bl_tui)                  ConPTY (conhost.exe)              Child Process
       │                                            │                              │
[PtyPair::open_and_spawn] ────────────────────────> │ ──(Spawn Child)────────────> │
       │                                            │                              │
       │ <──(DSR: \x1b[6n)───────────────────────── │                              │
       │                                            │ (Input frozen)               │
[perform_conpty_handshake]                          │                              │
       │ ──(Report: \x1b[1;1R)────────────────────> │                              │
       │                                            │ (Input unfrozen)             │
       │                                            │                              │
[Writer: PtyInputEvent::Write] ───────────────────> │ ──(Translate to INPUT_REC)─> │
[Writer: PtyInputEvent::Close]                      │                              │
       │ ──(\x1a\r\n Ctrl+Z EOF)──────────────────> │ ──(Deliver Console EOF)────> │
       │ (Keep writer pipe open!)                   │                              │
       │                                            │                              │
       │ <──(PtyOutputEvent::Output)─────────────── │ <──(stdout / stderr)──────── │
       │                                            │                              │
       │                                            │                              ├──(Exits)
       │                                            │ <──(Process Exited)──────────┘
[Orchestrator: child.wait()]                        │
       │                                            │
[Drop Controller: guard.take()]                     │
       │ ──(ClosePseudoConsole)───────────────────> │
       │                                            ├──(conhost.exe exits)
       │                                            ├──(hPipeOut broken pipe)
[Reader Task: BrokenPipe -> EOF]                    │
       │                                            │
[Orchestrator Joins Reader & Writer]                │
       │                                            │
[Session Terminates Cleanly]                        │
```

---

## Verification Results

### Windows (`nazmul-win.local`)

- **End-to-end PTY test suite** (`cargo test -p r3bl_tui --lib e2e_tests`):

    ```text
    running 5 tests
    test core::pty::e2e_tests::error_handling_test::test_unexpected_exit_reporting ... ok
    test core::pty::e2e_tests::session_test::test_session_with_cat ... ok
    test core::pty::e2e_tests::resize_test::test_pty_resize ... ok
    test core::pty::e2e_tests::osc_capture_test::test_osc_capture_enabled ... ok
    test core::pty::e2e_tests::osc_capture_test::test_osc_capture_disabled ... ok

    test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 2859 filtered out; finished in 0.28s
    ```

- **Full crate test suite** (`cargo test -p r3bl_tui --lib`):
  `2,863 passed; 0 failed; finished in 4.01s`.

### Linux (Local)

- `./check.fish --fmt`: 0 errors.
- `./check.fish --clippy`: 0 errors.
- `./check.fish --quick-doc`: 0 errors.
- `./check.fish --test`: all unit and doctests passed.

---

## Implementation Plan & Verification Lifecycle

### Phase 1: Research and Prototype Windows ConPTY Lifecycle

- [x] Inspect `portable_pty` implementation of `MasterPty::drop` on Windows to verify how
      `ClosePseudoConsole` is handled.
- [x] Prototype `Ctrl+Z` transmission in a standalone test to confirm `findstr "^"` exits
      cleanly upon receiving `b"\x1a\r\n"`.
- [x] Inspect `get_writer_with_handshake` in `pty_test_child_impl.rs` to allow graceful
      fallback when `\x1b[6n` is not emitted.
- [x] Mandatory manual review of Phase 1 findings.

### Phase 2: Input EOF (`Ctrl+Z`) Support and ConPTY Handshake

- [x] Implement `perform_conpty_handshake()` in `tasks/orchestrator.rs` responding to
      `\x1b[6n` with `\x1b[1;1R`.
- [x] Update `PtyInputEvent::Close` handling in `writer_task.rs` to emit `b"\x1a\r\n"` on
      Windows while keeping writer open until child exit to prevent
      `STATUS_CONTROL_C_EXIT`.
- [x] Verify that `session_test::test_session_with_cat` completes without hanging when
      executed against Windows ConPTY.
- [x] Mandatory manual review of Phase 2 modifications:
    - [x] `tui/src/core/ansi/constants/dsr.rs`
    - [x] `tui/src/core/pty/pty_session/pty_session_struct.rs`
    - [x] `tui/src/core/pty/pty_session/tasks/writer_task.rs`
    - [x] `tui/src/core/pty/e2e_tests/session_test.rs`

### Phase 3: Orchestrator Child Exit Signaling and Pseudo-Console Teardown

- [x] Wrap `controller` in `Arc<Mutex<Option<Controller>>>` in `orchestrator.rs`.
- [x] Drop controller immediately upon `controlled_child.wait()` to trigger
      `ClosePseudoConsole`, breaking `hPipeOut` and unblocking reader.
- [x] Update `reader_task.rs` to treat `ErrorKind::BrokenPipe` as normal EOF on Windows.
- [x] Verify `osc_capture_test::test_osc_capture_enabled` and `test_osc_capture_disabled`
      complete cleanly on Windows.
- [x] Mandatory manual review of Phase 3 modifications:
    - [x] `tui/src/core/pty/pty_session/tasks/orchestrator.rs`
    - [x] `tui/src/core/pty/pty_session/tasks/reader_task.rs`

### Phase 4: Subprocess Portability and Process Killer Bypass

- [x] Migrate `double_panic_prevention_test` to `generate_isolated_process_test!`.
- [x] Update `cross_platform_commands::cat()` to `findstr.exe ^`.
- [x] Update `cross_platform_commands::sleep()` to PowerShell `Start-Sleep`.
- [x] Fix `resize_test.rs` with `& exit` command suffix.
- [x] Implement `WindowsProcessKiller` and `ControlledChild` newtype in
      `controlled_child.rs` duplicating the raw handle to bypass `portable-pty 0.9.0`
      inverted return value bug (PR #7709).
- [x] Update `error_handling_test.rs` to call `kill().expect(...)` unconditionally on all
      platforms.
- [x] Un-gate `e2e_tests` in `tui/src/core/pty/mod.rs` and `e2e_tests/mod.rs`.
- [x] Verify full test suites pass on both Linux and Windows.
- [x] Mandatory manual review of Phase 4 modifications:
    - [x] `tui/src/core/common/common_enums.rs`
    - [x] `tui/src/core/pty/mod.rs`
    - [x] `tui/src/core/pty/pty_engine/pty_engine_types.rs`
    - [x] `tui/src/core/pty/pty_engine/controlled_child.rs`
    - [x] `tui/src/core/pty/pty_engine/windows_terminate_process.rs`
    - [x] `tui/src/core/pty/pty_session/mod.rs`
    - [x] `tui/src/core/pty/pty_session/pty_session_struct.rs`
    - [x] `tui/src/core/pty/pty_session/tasks/orchestrator.rs`
    - [x] `tui/src/core/script/command_impl/command_output_result.rs`
    - [x] `tui/src/core/pty/e2e_tests/cross_platform_commands.rs`
    - [x] `tui/src/core/pty/e2e_tests/error_handling_test.rs`
    - [x] `tui/src/core/pty/e2e_tests/resize_test.rs`
    - [x] `tui/src/core/test_fixtures/mod.rs`
    - [x] `tui/src/core/test_fixtures/pty_test_fixtures/pty_test_child/pty_test_child_impl.rs`
    - [x] `tui/src/core/pty/pty_engine/pty_pair.rs`
    - [x] `tui/src/core/resilient_reactor_thread/rrt_integration_tests/double_panic_prevention_test.rs`

