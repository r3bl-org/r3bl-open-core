<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Task: Enhance cmdr Upgrade with PTY spawn_read_only and OSC Progress](#task-enhance-cmdr-upgrade-with-pty-spawn_read_only-and-osc-progress)
  - [Summary](#summary)
  - [Current Analysis](#current-analysis)
    - [Current Implementation](#current-implementation)
    - [Key Components](#key-components)
  - [Implementation Plan](#implementation-plan)
    - [Phase 1: Make Spinner interval_message Updatable](#phase-1-make-spinner-interval_message-updatable)
      - [1.1 Modify Spinner struct](#11-modify-spinner-struct)
      - [1.2 Add type alias for clarity](#12-add-type-alias-for-clarity)
      - [1.3 Add update method to Spinner](#13-add-update-method-to-spinner)
      - [1.4 Update Spinner initialization](#14-update-spinner-initialization)
    - [Phase 2: Integrate PTY in upgrade_check.rs](#phase-2-integrate-pty-in-upgrade_checkrs)
      - [2.1 Update imports](#21-update-imports)
      - [2.2 Implement dual-command execution](#22-implement-dual-command-execution)
      - [2.3 Implement rustup update (silent)](#23-implement-rustup-update-silent)
      - [2.4 Implement cargo install with progress](#24-implement-cargo-install-with-progress)
      - [2.5 Handle OSC progress events](#25-handle-osc-progress-events)
    - [Phase 3: Update UI Messages](#phase-3-update-ui-messages)
  - [Benefits of This Approach](#benefits-of-this-approach)
  - [Testing Plan](#testing-plan)
  - [Rollback Plan](#rollback-plan)
  - [Success Criteria](#success-criteria)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Task: Enhance cmdr Upgrade with PTY spawn_read_only and OSC Progress

## Summary

Integrate the PTY `spawn_read_only` API into the `install_upgrade_command_with_spinner_and_ctrl_c()`
function to handle OSC progress codes and run commands with better control. The key insight is to
make the Spinner's `interval_message` updatable so we can dynamically change it to show progress
percentages.

## Current Analysis

### Current Implementation

- Uses `TokioCommand` to spawn `cargo install`
- Spinner shows static message "Installing {crate_name}..."
- No progress reporting from cargo build
- Output is piped but not processed

### Key Components

1. **Spinner** (`tui/src/readline_async/spinner.rs`):
   - Has `interval_message: InlineString` field (currently static after creation)
   - Uses `SafeBool = Arc<StdMutex<bool>>` pattern for thread-safe state
   - Spawns async task that renders the message on each tick

2. **PTY API** (`tui/src/core/pty/`):
   - `PtyCommandBuilder` for command configuration
   - `spawn_read_only()` returns `PtyReadOnlySession`
   - `PtyOutputEvent` enum includes `Osc(OscEvent)` for progress
   - `OscEvent::ProgressUpdate(u8)` provides 0-100% progress values

## Implementation Plan

### Phase 1: Make Spinner interval_message Updatable

#### 1.1 Modify Spinner struct

```rust
// In tui/src/readline_async/spinner.rs
pub struct Spinner {
    pub tick_delay: Duration,
    // Change from:
    // pub interval_message: InlineString,
    // To:
    safe_interval_message: Arc<StdMutex<InlineString>>,
    pub final_message: InlineString,
    // ... rest of fields
}
```

#### 1.2 Add type alias for clarity

```rust
// In tui/src/readline_async/mod.rs
pub type SafeInlineString = Arc<StdMutex<InlineString>>;
```

#### 1.3 Add update method to Spinner

```rust
impl Spinner {
    /// Updates the interval message that's displayed during spinner animation.
    /// This can be called from another task/thread to update progress.
    pub fn update_message(&self, new_message: impl Into<InlineString>) {
        let msg = new_message.into();
        // Strip ANSI codes if present
        let clean_msg = if contains_ansi_escape_sequence(&msg) {
            strip_ansi_escapes::strip_str(&msg).into()
        } else {
            msg
        };
        *self.safe_interval_message.lock().unwrap() = clean_msg;
    }
}
```

#### 1.4 Update Spinner initialization

- Modify `try_start()` to wrap interval_message in `Arc<Mutex<>>`
- Clone the Arc for the spawned task
- In the render loop, lock and read the current message value

### Phase 2: Integrate PTY in upgrade_check.rs

#### 2.1 Update imports

```rust
use r3bl_tui::{
    // ... existing imports ...
    core::pty::{
        PtyCommandBuilder, PtyReadOnlySession,
        PtyOutputEvent, PtyConfigOption, OscEvent
    },
};
```

#### 2.2 Implement dual-command execution

```rust
async fn install_upgrade_command_with_spinner_and_ctrl_c() {
    let crate_name = get_self_crate_name();

    // Setup spinner
    let mut maybe_spinner = if let Ok(Some(spinner)) = Spinner::try_start(
        "Updating Rust toolchain...",  // Initial message
        ui_str::upgrade_install::stop_msg(),
        Duration::from_millis(100),
        SpinnerStyle::default(),
        OutputDevice::default(),
        None,
    ).await {
        Some(spinner)
    } else {
        None
    };

    // First: Run rustup update (silent, no progress)
    let rustup_result = run_rustup_update().await;
    if let Err(e) = rustup_result {
        // Handle error, stop spinner
        if let Some(mut spinner) = maybe_spinner.take() {
            spinner.request_shutdown();
            spinner.await_shutdown().await;
        }
        report_upgrade_install_result(Err(e));
        return;
    }

    // Update spinner message for cargo install
    if let Some(ref spinner) = maybe_spinner {
        spinner.update_message(format!("Installing {}...", crate_name));
    }

    // Second: Run cargo install with OSC progress
    let install_result = run_cargo_install_with_progress(
        crate_name,
        maybe_spinner.as_ref()
    ).await;

    // Stop spinner
    if let Some(mut spinner) = maybe_spinner.take() {
        spinner.request_shutdown();
        spinner.await_shutdown().await;
    }

    // Report result
    report_upgrade_install_result(install_result);
}
```

#### 2.3 Implement rustup update (silent)

```rust
async fn run_rustup_update() -> Result<ExitStatus, Error> {
    let mut session = PtyCommandBuilder::new("rustup")
        .args(["update"])
        .spawn_read_only(PtyConfigOption::NoCaptureOutput)?;

    // Wait for completion with Ctrl+C support
    tokio::select! {
        _ = signal::ctrl_c() => {
            // PTY session will be dropped and cleaned up
            Err(Error::new(ErrorKind::Interrupted, "Update cancelled by user"))
        }
        status = session.completion_handle => {
            status.map_err(|e| Error::new(ErrorKind::Other, e))
        }
    }
}
```

#### 2.4 Implement cargo install with progress

```rust
async fn run_cargo_install_with_progress(
    crate_name: &str,
    spinner: Option<&Spinner>
) -> Result<ExitStatus, Error> {
    let mut session = PtyCommandBuilder::new("cargo")
        .args(["install", crate_name])
        .enable_osc_sequences()  // Enable OSC 9;4 progress
        .spawn_read_only(PtyConfigOption::Osc)?;

    let mut ctrl_c = signal::ctrl_c();

    loop {
        tokio::select! {
            _ = &mut ctrl_c => {
                // User pressed Ctrl+C
                return Err(Error::new(ErrorKind::Interrupted, "Installation cancelled by user"));
            }
            event = session.output_event_receiver_half.recv() => {
                match event {
                    Some(PtyOutputEvent::Osc(osc_event)) => {
                        handle_osc_event(osc_event, crate_name, spinner);
                    }
                    Some(PtyOutputEvent::Exit(status)) => {
                        return if status.success() {
                            Ok(status.into())
                        } else {
                            Err(Error::new(ErrorKind::Other,
                                format!("Installation failed with status: {:?}", status)))
                        };
                    }
                    Some(PtyOutputEvent::UnexpectedExit(msg)) => {
                        return Err(Error::new(ErrorKind::Other, msg));
                    }
                    None => {
                        // Channel closed unexpectedly
                        return Err(Error::new(ErrorKind::Other, "PTY session ended unexpectedly"));
                    }
                    _ => {} // Ignore Output events since we're using Osc mode
                }
            }
        }
    }
}
```

#### 2.5 Handle OSC progress events

```rust
fn handle_osc_event(event: OscEvent, crate_name: &str, spinner: Option<&Spinner>) {
    if let Some(spinner) = spinner {
        match event {
            OscEvent::ProgressUpdate(percentage) => {
                spinner.update_message(
                    format!("Installing {}... {}%", crate_name, percentage)
                );
            }
            OscEvent::IndeterminateProgress => {
                spinner.update_message(
                    format!("Installing {}... (building)", crate_name)
                );
            }
            OscEvent::ProgressCleared => {
                spinner.update_message(
                    format!("Installing {}...", crate_name)
                );
            }
            OscEvent::BuildError => {
                spinner.update_message(
                    format!("Installing {}... (error occurred)", crate_name)
                );
            }
        }
    }
}
```

### Phase 3: Update UI Messages

Add new messages in `ui_str.rs`:

```rust
pub mod upgrade_install {
    // ... existing ...

    pub fn rustup_update_msg_raw() -> String {
        "Updating Rust toolchain...".to_string()
    }

    pub fn install_with_progress_msg_raw(crate_name: &str, percentage: u8) -> String {
        format!("Installing {}... {}%", crate_name, percentage)
    }
}
```

## Benefits of This Approach

1. **Simplicity**: Reuses existing Spinner infrastructure, just makes the message updatable
2. **Thread-safe**: Uses same `Arc<Mutex<>>` pattern as other Spinner fields
3. **Non-breaking**: Existing Spinner API remains compatible
4. **Clean separation**: PTY handling is separate from spinner rendering
5. **Real-time updates**: Progress percentages update smoothly during build

## Testing Plan

1. **Manual Testing**:
   - Normal upgrade flow with progress updates
   - Ctrl+C during rustup update
   - Ctrl+C during cargo install at various progress points
   - Network disconnection scenarios
   - Permission errors

2. **Unit Tests**:
   - Test Spinner message updates from multiple threads
   - Test OSC event parsing and handling
   - Test error conditions

3. **Integration Tests**:
   - Mock PTY sessions with test OSC sequences
   - Verify spinner updates correctly with progress events

## Rollback Plan

If issues arise:

1. The changes are isolated to:
   - Spinner struct and implementation
   - `install_upgrade_command_with_spinner_and_ctrl_c()` function
2. Can easily revert to static message by removing the `Arc<Mutex<>>` wrapper
3. Can fallback to TokioCommand if PTY issues occur

## Success Criteria

- [x] Spinner message updates dynamically with progress percentage
- [ ] Silent rustup update (no output shown)
- [ ] Cargo install shows real-time progress via OSC codes
- [ ] Ctrl+C works cleanly at any point
- [ ] Error messages are clear and actionable
- [ ] No regression in existing functionality
