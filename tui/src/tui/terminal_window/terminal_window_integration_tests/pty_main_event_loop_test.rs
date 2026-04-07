// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`PTY`]-based integration test for [`main_event_loop_impl`].
//!
//! Verifies that the full event loop works end-to-end in a real pseudoterminal:
//! the controller sends keystrokes (Up, Up, x) through the [`PTY`], and the
//! controlled process runs [`main_event_loop_impl`] with a minimal [`App`] that
//! counts Up-arrow presses and exits on 'x'.
//!
//! [`App`]: crate::App
//! [`main_event_loop_impl`]: crate::main_event_loop_impl
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use crate::{App, CommonResult, ComponentRegistryMap, EventPropagation, GlobalData,
            HasFocus, InputDevice, InputEvent, Key, KeyPress, MSG_CONTROLLED_READY,
            MSG_CONTROLLED_STARTING, MSG_SUCCESS, OutputDevice, PtyTestContext,
            RenderPipeline, TerminalWindowMainThreadSignal, generate_pty_test, height,
            key_press, main_event_loop_impl, throws_with_return, width};
use std::{fmt::{Debug, Display, Formatter},
          io::Write};

generate_pty_test! {
    test_fn: test_pty_main_event_loop_impl,
    controller: controller,
    controlled: controlled,
    mode: crate::PtyTestMode::Cooked,
}

fn controller(context: PtyTestContext) {
    let PtyTestContext {
        pty_pair,
        child,
        mut buf_reader,
        mut writer,
    } = context;

    child
        .wait_for_ready(&mut buf_reader, MSG_CONTROLLED_READY)
        .unwrap();

    // Give the event loop time to start and render.
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Send Up, Up, 'x' (exit key).
    writer.write_all(b"\x1b[A").unwrap(); // Up arrow
    writer.flush().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));
    writer.write_all(b"\x1b[A").unwrap(); // Up arrow
    writer.flush().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));
    writer.write_all(b"x").unwrap(); // Exit
    writer.flush().unwrap();

    let result = child.read_until_marker(&mut buf_reader, MSG_SUCCESS, |line| {
        line.contains("Counter:") || line.contains("EventLoopResult:")
    });

    assert!(
        result.found_marker,
        "Controlled process did not print SUCCESS"
    );
    assert!(
        result.lines.iter().any(|l| l.contains("Counter: 2")),
        "Expected counter to be 2 after two Up arrows, got: {:?}",
        result.lines
    );
    assert!(
        result
            .lines
            .iter()
            .any(|l| l.contains("EventLoopResult: Ok")),
        "Event loop should have exited successfully"
    );

    child.drain_and_wait(buf_reader, pty_pair);
}

/// The harness performs [`std::process::exit(0)`] after this function returns.
fn controlled() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        println!("{MSG_CONTROLLED_STARTING}");

        let app = Box::<AppMainTest>::default();
        let exit_keys = vec![InputEvent::Keyboard(key_press! { @char 'x' })];
        let initial_size = width(65) + height(11);
        let input_device = InputDevice::new();
        let output_device = OutputDevice::new_stdout();
        let state = State::default();

        println!("{MSG_CONTROLLED_READY}");
        std::io::stdout().flush().unwrap();

        let result = main_event_loop_impl(
            app,
            exit_keys,
            state,
            initial_size,
            input_device,
            output_device,
        )
        .await;

        match &result {
            Ok((global_data, _, _)) => {
                println!("Counter: {}", global_data.state.counter);
                println!("EventLoopResult: Ok");
            }
            Err(e) => {
                println!("EventLoopResult: Err({e})");
            }
        }

        println!("{MSG_SUCCESS}");
        std::io::stdout().flush().unwrap();
    });
}

// --- Test fixtures ---

#[derive(Default, Clone, Debug)]
#[allow(dead_code)]
enum AppSignal {
    Add,
    Sub,
    #[default]
    Noop,
}

#[derive(Clone, PartialEq, Eq, Default)]
struct State {
    counter: isize,
}

impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "State{{counter:{}}}", self.counter)
    }
}

impl Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "State {{ counter: {:?} }}", self.counter)
    }
}

#[derive(Default)]
struct AppMainTest;

impl App for AppMainTest {
    type S = State;
    type AS = AppSignal;

    fn app_render(
        &mut self,
        _global_data: &mut GlobalData<State, AppSignal>,
        _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        _has_focus: &mut HasFocus,
    ) -> CommonResult<RenderPipeline> {
        throws_with_return!({ RenderPipeline::default() });
    }

    /// Handles input events for the test application.
    ///
    /// Specifically, this method listens for the "Up" arrow key. If pressed, it consumes
    /// the event and dispatches an [`AppSignal::Add`] signal.
    fn app_handle_input_event(
        &mut self,
        input_event: InputEvent,
        global_data: &mut GlobalData<State, AppSignal>,
        _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        _has_focus: &mut HasFocus,
    ) -> CommonResult<EventPropagation> {
        throws_with_return!({
            if matches!(
                input_event,
                InputEvent::Keyboard(
                    KeyPress::Plain {
                        key: Key::SpecialKey(crate::SpecialKey::Up),
                    } | KeyPress::WithModifiers {
                        key: Key::SpecialKey(crate::SpecialKey::Up),
                        ..
                    }
                )
            ) {
                crate::send_signal!(
                    global_data.main_thread_channel_sender,
                    TerminalWindowMainThreadSignal::ApplyAppSignal(AppSignal::Add,)
                );
                EventPropagation::ConsumedRender
            } else {
                EventPropagation::Propagate
            }
        });
    }

    fn app_handle_signal(
        &mut self,
        action: &AppSignal,
        global_data: &mut GlobalData<State, AppSignal>,
        _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        _has_focus: &mut HasFocus,
    ) -> CommonResult<EventPropagation> {
        throws_with_return!({
            if let AppSignal::Add = action {
                global_data.state.counter += 1;
            }
            EventPropagation::ConsumedRender
        });
    }

    fn app_init(
        &mut self,
        _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        _has_focus: &mut HasFocus,
    ) {
    }
}
