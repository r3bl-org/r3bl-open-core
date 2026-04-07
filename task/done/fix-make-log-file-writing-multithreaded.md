# Make Log File Writing Non-Blocking with Guaranteed Flush

## Diagnosis

Debug flags (`DEBUG_TUI_SHOW_MIO_POLLER`, etc.) cause blocking file I/O in the mio
poller thread via synchronous `tracing_appender::rolling::RollingFileAppender`. This
freezes the TUI because mio uses edge-triggered epoll - blocking the poller thread
causes missed stdin events.

### Root cause: Blocking file I/O + edge-triggered epoll

1. `DEBUG_TUI_SHOW_MIO_POLLER = true` fires `tracing::debug!()` on every stdin read
   (`handler_stdin.rs:95`).
2. The tracing subscriber used a synchronous `RollingFileAppender`. Each call blocked
   the mio poller thread for disk I/O.
3. mio uses edge-triggered epoll (`Interest::READABLE` on `SourceFd`). While blocked
   writing to the log file, new keystrokes arrive but the edge trigger won't re-notify
   when `poll()` resumes.
4. Keyboard input gets stuck in the kernel buffer. The app appears frozen.

### Secondary bug (fixed)

`eprintln!` at `input_device_public_api.rs:774` wrote directly to stderr in raw mode.
Changed to `tracing::warn!`.

## Implementation (DONE)

### Approach: Non-blocking writer + RAII sentinel

Wrapped the `RollingFileAppender` in `tracing_appender::non_blocking()`, which sends
writes to a background thread. The `WorkerGuard` lives in a static
`Mutex<Option<WorkerGuard>>`. Two RAII guards handle cleanup:

- **`GlobalLogFileGuard`** - lightweight sentinel returned by `install_global()` /
  `try_initialize_logging_global()`. On drop, takes the `WorkerGuard` from the static
  and drops it, flushing the buffer. Callers hold it on `main()`'s stack.
- **`ThreadLocalLogFileGuard`** - wraps both `dispatcher::DefaultGuard` + `GlobalLogFileGuard` for
  `install_thread_local()` / `try_initialize_logging_thread_local()`. Single value
  instead of managing two guards.

### Files changed

1. **`tui/src/core/log/rolling_file_appender_impl.rs`**
   - `OnceLock<WorkerGuard>` -> `Mutex<Option<WorkerGuard>>`
   - `try_create()` wraps appender in `non_blocking()`, stores guard in static
   - Early-return `Err` if already created (prevents killing first background thread)
   - Added `GlobalLogFileGuard` struct with `Drop` impl

2. **`tui/src/core/log/tracing_config.rs`**
   - `install_global()` returns `miette::Result<GlobalLogFileGuard>`
   - `install_thread_local()` returns `miette::Result<ThreadLocalLogFileGuard>`
   - Added `ThreadLocalLogFileGuard` struct wrapping `DefaultGuard` + `GlobalLogFileGuard`

3. **`tui/src/core/log/log_public_api.rs`**
   - `try_initialize_logging_global()` returns `miette::Result<GlobalLogFileGuard>`
   - `try_initialize_logging_thread_local()` returns `miette::Result<Option<ThreadLocalLogFileGuard>>`

4. **Call sites** - all store the returned guard in `_log_guard`:
   - `tui/examples/tui_apps/main.rs`
   - `tui/examples/readline_async.rs`
   - `tui/examples/pty_mux_example.rs`
   - `tui/examples/choose_interactive.rs`
   - `tui/examples/choose_with_and_without_readline_async.rs`
   - `tui/examples/pty_simple_example.rs`
   - `tui/examples/pty_rw_echo_example.rs`
   - `cmdr/src/bin/giti.rs`
   - `cmdr/src/bin/edi.rs`

5. **`tui/src/core/log/tracing_init.rs`** - test updated for `ThreadLocalLogFileGuard`

## Phase 7: Global Log File Path Standardization [COMPLETE]

Centralize all logging across the workspace to a single predictable location: `/tmp/r3bl_tui/log.txt` instead of the current working directory.

### 7.1 Core Implementation

- `tui/src/core/log/log_public_api.rs`: Change `DEFAULT_LOG_FILE_NAME` from `"log.txt"` to `"/tmp/r3bl_tui/log.txt"`.
- `tui/src/core/log/rolling_file_appender_impl.rs`: In `try_create()`, use `crate::try_mkdir(parent, crate::MkdirOptions::CreateIntermediateDirectories)` to ensure the parent directory exists before creating the file appender. Because `parent` might be `""` if someone manually passes a bare filename, only create the directory if `parent.as_os_str().is_empty()` is false.

### 7.2 Update Applications & Examples

- `cmdr/src/edi/clap_config.rs` & `cmdr/src/giti/clap_config.rs`: Update `--help` text to mention `/tmp/r3bl_tui/log.txt`.
- `tui/examples/pty_mux_example.rs` & `tui/examples/pty_simple_example.rs`: Update the `println!` and comments.
- `tui/examples/tui_apps/ex_rc/slide2.md`: Update `tail -f` and `rm`/`touch` commands to point to `/tmp/r3bl_tui/log.txt`.

### 7.3 Update Scripts & Agent Skills

- `run.fish`: Update `help` text and change the `log` command logic to unconditionally tail `/tmp/r3bl_tui/log.txt` (removing the choice between tui/cmdr directories).
- `script_lib.fish`: Update the `chown` fallback logic (around line 862) to check and `chown` `/tmp/r3bl_tui/log.txt`.
- `README.md`: Update references to `log.txt` in the CLI commands section.
- `.agents/commands/analyze-logs.md`: Update the command description to point to `/tmp/r3bl_tui/log.txt`.
- `.agents/skills/analyze-log-files/SKILL.md`: Update the `ansifilter` examples and "Common Log File Locations" section.
- `tui/src/readline_async/choose_impl/mod.rs`: Fix the rustdoc comment `tail -f log.txt`.

## Verification

1. `./check.fish --check` - typecheck
2. `cargo test -p r3bl_tui tracing_init` - run tracing tests
3. Set `DEBUG_TUI_SHOW_MIO_POLLER = true` in `tui/src/tui/mod.rs`
4. `cargo run --example tui_apps` - verify input works (up/down at menu)
5. Check `log.txt` - verify debug messages are written and flushed on exit
