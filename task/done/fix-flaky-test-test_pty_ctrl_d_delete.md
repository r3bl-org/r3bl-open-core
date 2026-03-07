<!-- cspell:words ello -->

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Fix flaky test: test_pty_ctrl_d_delete

## Status: COMPLETE

## Symptom

`test_pty_ctrl_d_delete` fails intermittently under parallel test load (`check.fish
--test`) but passes when run in isolation. The failure manifests as:

- **Controller panic at line 127**: `"EOF reached before getting line state"` -- the
  controlled process exits before the controller receives the expected `Line:` response.

## Root cause analysis

The test uses a request-response protocol between controller and controlled processes
over a PTY. The controller sends input, then reads the controlled process's line state
response. Two timing issues caused flakiness under CPU load:

### Fixed sleep durations

The controller used `thread::sleep(Duration::from_millis(200))` and
`thread::sleep(Duration::from_millis(100))` between sending input and reading the
response. These sleeps were unnecessary because `read_line_state()` already blocks until
a `Line:` response arrives. The sleeps only added dead time during which the inactivity
watchdog ticked down.

### Short inactivity watchdog

The controlled process had a 2-second inactivity watchdog. Under heavy CPU load (parallel
test execution), the OS might not schedule the controlled process's tokio runtime
promptly enough to see input events within 2 seconds. The watchdog would fire, the
controlled process would exit, and the controller would see EOF.

# Implementation plan

## Step 0: Choose the fix approach [COMPLETE]

Remove the fixed sleep durations from the controller -- the blocking `read_line_state()`
call already provides the synchronization needed. Extend the inactivity watchdog from 2s
to 5s to provide headroom for parallel test execution.

## Step 1: Remove unnecessary sleeps from controller [COMPLETE]

File:
`tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_d_delete_test.rs`

Removed three `thread::sleep()` calls (200ms, 100ms, 100ms) between controller
write/read cycles. The `read_line_state()` closure already blocks on `read_line()` until
the controlled process responds.

## Step 2: Extend inactivity watchdog timeout [COMPLETE]

Changed `AsyncDebouncedDeadline::new(Duration::from_secs(2))` to
`AsyncDebouncedDeadline::new(Duration::from_secs(5))` in the controlled process's event
loop.

## Step 3: Update doc comment [COMPLETE]

Updated the test protocol description to remove the reference to the 200ms sleep.

## Verification

- Isolated test: 100/100 passed
- `check.fish --test` (full parallel suite): 20/20 passed

## Files changed

- [x] `tui/src/readline_async/readline_async_impl/integration_tests/pty_ctrl_d_delete_test.rs`
  -- Remove sleeps, extend watchdog, update docs
