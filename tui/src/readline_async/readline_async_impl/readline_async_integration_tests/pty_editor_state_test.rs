// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words ONLCR

//! [`PTY`]-based integration tests for [`Readline`] editor state.
//!
//! This test verifies that the internal state of the [`Readline`] editor (buffer content,
//! cursor position, and event processing) is correctly managed and rendered.
//!
//! # Why [`PTY`] isolation?
//!
//! It uses [`PTY`] isolation because [`Readline::drop()`] unconditionally calls
//! [`disable_raw_mode()`], which modifies the global [`SAVED_TERMIOS`] static and alters
//! the terminal settings (specifically the [`ONLCR`] flag) of the process that runs it.
//! Without [`PTY`] isolation, these tests would mutate the `cargo test` runner's own
//! terminal, causing the "staircase effect" (newlines moving the cursor down but not back
//! to column 0) in the test output.
//!
//! By running in a [`PTY`], terminal state mutations only affect the **[child process's
//! `TTY`]**, leaving the developer's terminal environment untouched.
//!
//! # Run with:
//!
//! ```bash
//! cargo test -p r3bl_tui test_pty_editor_state -- --nocapture
//! ```
//!
//! [`disable_raw_mode()`]: crate::disable_raw_mode
//! [`ONLCR`]: https://man7.org/linux/man-pages/man4/tty_ioctl.4.html
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`Readline`]: crate::Readline
//! [`SAVED_TERMIOS`]: crate::terminal_raw_mode::raw_mode_unix::SAVED_TERMIOS
//! [child process's `TTY`]: crate::core::pty::pty_engine::pty_pair#what-is-a-tty

use crate::{MSG_CONTROLLED_READY, MSG_CONTROLLED_STARTING, ChannelCapacity, ControlFlowExtended,
            CursorPositionBoundsStatus, GCStringOwned, GLYPH_FAILURE, GLYPH_SUCCESS,
            History, InputDevice, OutputDevice, OutputDeviceExt, PtyTestContext,
            PtyTestMode, Readline, MSG_SUCCESS, Size, StdMutex, generate_pty_test, height,
            lock_output_device_as_mut, readline_internal, seg_index, width};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::{io::Write, sync::Arc};
use tokio::sync::broadcast;

generate_pty_test! {
    test_fn: test_pty_editor_state,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Raw,
}

fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        ..
    } = context;

    child
        .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
        .unwrap();

    let result = child.read_until_marker(&mut buf_reader, MSG_SUCCESS, |line| {
        line.contains(GLYPH_SUCCESS) || line.contains(GLYPH_FAILURE)
    });

    assert!(
        result.found_marker,
        "Controlled process did not print SUCCESS"
    );

    // Check that all 4 tests passed in the controlled process.
    let pass_count = result
        .lines
        .iter()
        .filter(|l| l.contains(GLYPH_SUCCESS))
        .count();
    assert_eq!(
        pass_count, 4,
        "Not all editor state tests passed: {:?}",
        result.lines
    );

    child.drain_and_wait(buf_reader, pty_pair);
}

/// The harness performs [`std::process::exit(0)`] after this function returns.
fn controlled() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        println!("{MSG_CONTROLLED_STARTING}");
        println!("{MSG_CONTROLLED_READY}");
        std::io::stdout().flush().unwrap();

        if run_test_readline_internal_process_event_and_terminal_output() {
            println!("{GLYPH_SUCCESS} test_readline_internal_process_event_and_terminal_output passed");
        } else {
            println!("{GLYPH_FAILURE} test_readline_internal_process_event_and_terminal_output failed");
        }

        if run_test_editor_state_empty_buffer() {
            println!("{GLYPH_SUCCESS} test_editor_state_empty_buffer passed");
        } else {
            println!("{GLYPH_FAILURE} test_editor_state_empty_buffer failed");
        }

        if run_test_editor_state_with_content() {
            println!("{GLYPH_SUCCESS} test_editor_state_with_content passed");
        } else {
            println!("{GLYPH_FAILURE} test_editor_state_with_content failed");
        }

        if run_test_editor_state_cursor_at_start_with_content() {
            println!("{GLYPH_SUCCESS} test_editor_state_cursor_at_start_with_content passed");
        } else {
            println!("{GLYPH_FAILURE} test_editor_state_cursor_at_start_with_content failed");
        }

        println!("{MSG_SUCCESS}");
        std::io::stdout().flush().unwrap();
    });
}

fn run_test_readline_internal_process_event_and_terminal_output() -> bool {
    let prompt_str = "> ";
    let (output_device, stdout_mock) = OutputDevice::new_mock();
    let input_device = InputDevice::new_mock(smallvec::smallvec![]);
    let (shutdown_sender, _) = broadcast::channel::<()>(1);
    let test_size = Size::new((width(100), height(100)));
    let (readline, _) = Readline::try_new(
        prompt_str.into(),
        output_device.clone(),
        input_device,
        shutdown_sender,
        ChannelCapacity::Minimal,
        test_size,
    )
    .unwrap();

    let safe_is_spinner_active = Arc::new(StdMutex::new(None));
    let history = History::new();
    let safe_history = Arc::new(StdMutex::new(history.0));

    // Simulate 'a'.
    let event = Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    let input_event = readline_internal::convert_crossterm_event_to_input_event(event);
    let control_flow = readline_internal::apply_event_to_line_state_and_render(
        input_event.unwrap(),
        &readline.safe_line_state,
        lock_output_device_as_mut!(output_device),
        &safe_history,
        &safe_is_spinner_active,
    );

    matches!(control_flow, ControlFlowExtended::Continue)
        && readline.safe_line_state.lock().unwrap().line.as_str() == "a"
        && stdout_mock
            .get_copy_of_buffer_as_string_strip_ansi()
            .contains("> a")
}

fn run_test_editor_state_empty_buffer() -> bool {
    let prompt_str = "> ";
    let (output_device, _) = OutputDevice::new_mock();
    let input_device = InputDevice::new_mock(smallvec::smallvec![]);
    let (shutdown_sender, _) = broadcast::channel::<()>(1);
    let test_size = Size::new((width(100), height(100)));
    let (readline, _) = Readline::try_new(
        prompt_str.into(),
        output_device,
        input_device,
        shutdown_sender,
        ChannelCapacity::Minimal,
        test_size,
    )
    .unwrap();

    readline.get_buffer().is_empty()
        && readline.get_cursor_position_status() == CursorPositionBoundsStatus::AtStart
        && readline.get_cursor_position() == seg_index(0)
        && readline.get_buffer().as_str() == ""
}

fn run_test_editor_state_with_content() -> bool {
    let prompt_str = "> ";
    let (output_device, _) = OutputDevice::new_mock();
    let input_device = InputDevice::new_mock(smallvec::smallvec![]);
    let (shutdown_sender, _) = broadcast::channel::<()>(1);
    let test_size = Size::new((width(100), height(100)));
    let (readline, _) = Readline::try_new(
        prompt_str.into(),
        output_device,
        input_device,
        shutdown_sender,
        ChannelCapacity::Minimal,
        test_size,
    )
    .unwrap();

    {
        let mut line_state = readline.safe_line_state.lock().unwrap();
        line_state.line = GCStringOwned::new("hello");
        line_state.line_cursor_grapheme = seg_index(5);
    }

    !readline.get_buffer().is_empty()
        && readline.get_cursor_position_status() == CursorPositionBoundsStatus::AtEnd
        && readline.get_cursor_position() == seg_index(5)
        && readline.get_buffer().as_str() == "hello"
}

fn run_test_editor_state_cursor_at_start_with_content() -> bool {
    let prompt_str = "> ";
    let (output_device, _) = OutputDevice::new_mock();
    let input_device = InputDevice::new_mock(smallvec::smallvec![]);
    let (shutdown_sender, _) = broadcast::channel::<()>(1);
    let test_size = Size::new((width(100), height(100)));
    let (readline, _) = Readline::try_new(
        prompt_str.into(),
        output_device,
        input_device,
        shutdown_sender,
        ChannelCapacity::Minimal,
        test_size,
    )
    .unwrap();

    {
        let mut line_state = readline.safe_line_state.lock().unwrap();
        line_state.line = GCStringOwned::new("hello");
        line_state.line_cursor_grapheme = seg_index(0);
    }

    !readline.get_buffer().is_empty()
        && readline.get_cursor_position_status() == CursorPositionBoundsStatus::AtStart
        && readline.get_cursor_position() == seg_index(0)
        && readline.get_buffer().as_str() == "hello"
}
