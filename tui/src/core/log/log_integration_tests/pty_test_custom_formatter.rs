// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{BufReadExt, GLYPH_CONTROLLER, MSG_SUCCESS, PtyTestContext, PtyTestMode,
            generate_pty_test, CustomEventFormatter, StdoutMock};
use tracing::info;
use tracing_subscriber::fmt::SubscriberBuilder;
use std::time::{Duration, Instant};
use chrono::Local;
use std::sync::Mutex;

generate_pty_test! {
    test_fn: test_custom_formatter_pty,
    controller: controller,
    controlled: controlled,
    mode: PtyTestMode::Cooked,
}

fn controller(context: PtyTestContext) {
    let PtyTestContext {
        mut buf_reader,
        pty_pair,
        child,
        ..
    } = context;

    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting custom formatter test...");

    let mut test_passed = false;
    let start_timeout = Instant::now();

    while start_timeout.elapsed() < Duration::from_secs(5) {
        let mut line = String::new();
        let result = buf_reader.read_line_eio_to_eof(&mut line);
        match result {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim();
                eprintln!("  ← Controlled output: {trimmed}");

                if trimmed.contains(MSG_SUCCESS) {
                    test_passed = true;
                    break;
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(err) => panic!("Read error: {err}"),
        }
    }

    assert!(test_passed, "Controlled process did not report success");

    child.drain_and_wait(buf_reader, pty_pair);
}

fn controlled() {
    let mock_stdout = StdoutMock::new();
    let mock_stdout_clone = mock_stdout.clone();
    let subscriber = SubscriberBuilder::default()
        .event_format(CustomEventFormatter)
        .with_writer(Mutex::new(mock_stdout))
        .finish();

    let _drop_guard = tracing::subscriber::set_default(subscriber);

    info!(
        message = "This is now the heading, not the body!",
        "foo" = "bar"
    );

    let time = Local::now().format("%I:%M%P").to_string();
    let it = mock_stdout_clone.get_copy_of_buffer_as_string();
    let it_no_ansi = mock_stdout_clone.get_copy_of_buffer_as_string_strip_ansi();

    // Check that heading is correct.
    assert!(!it_no_ansi.contains("message"));
    assert!(it_no_ansi.contains("This is now the heading, not the body!"));

    // Check that body contains the other fields.
    assert!(it_no_ansi.contains("foo"));
    assert!(it.contains("bar"));

    // Check that timestamp is present.
    assert!(it.contains(&time));

    println!("{MSG_SUCCESS}");
}
