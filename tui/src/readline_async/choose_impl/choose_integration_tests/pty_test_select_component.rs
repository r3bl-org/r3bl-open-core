// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ch, generate_pty_test, ColorSupport, Header, HowToChoose, ItemsOwned,
            OutputDevice, OutputDeviceExt, PtyTestContext, PtyTestMode, SelectComponent,
            State, StyleSheet, MSG_SUCCESS, GLYPH_CONTROLLER, BufReadExt,
            FunctionComponent,
            global_color_support::{clear_override, set_override}};
use std::time::{Duration, Instant};

generate_pty_test! {
    test_fn: test_select_component_pty,
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

    eprintln!("{GLYPH_CONTROLLER} PTY Controller: Starting select component test...");

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
    let mut state = State {
        header: Header::SingleLine("Header".into()),
        items: ItemsOwned::from(&["Item 1", "Item 2", "Item 3"]),
        max_display_height: ch(5),
        max_display_width: ch(40),
        raw_caret_row_index: ch(0),
        scroll_offset_row_index: ch(0),
        selected_items: ItemsOwned::new(),
        selection_mode: HowToChoose::Single,
        ..Default::default()
    };

    state.scroll_offset_row_index = ch(0);

    let (output_device, stdout_mock) = OutputDevice::new_mock();

    let mut component = SelectComponent {
        output_device,
        style: StyleSheet::default(),
    };

    set_override(ColorSupport::Ansi256);
    component.render(&mut state).unwrap();

    let generated_output = stdout_mock.get_copy_of_buffer_as_string();

    let expected_output = "\u{1b}[4F\u{1b}[1G\u{1b}[0m\u{1b}[2K\u{1b}[38;5;153m\u{1b}[48;5;235m Header\u{1b}[0m\u{1b}[1E\u{1b}[0m\u{1b}[1G\u{1b}[0m\u{1b}[2K\u{1b}[38;5;46m  ◉ Item 1\u{1b}[0m\u{1b}[38;5;46m                              \u{1b}[0m\u{1b}[1E\u{1b}[0m\u{1b}[1G\u{1b}[0m\u{1b}[2K  ◌ Item 2\u{1b}[0m                              \u{1b}[0m\u{1b}[1E\u{1b}[0m\u{1b}[1G\u{1b}[0m\u{1b}[2K  ◌ Item 3\u{1b}[0m                              \u{1b}[0m\u{1b}[1E\u{1b}[0m\u{1b}[4F";
    
    if generated_output != expected_output {
        eprintln!("Generated output does not match expected output!");
        eprintln!("Generated: {generated_output:?}");
        eprintln!("Expected:  {expected_output:?}");
        std::process::exit(1);
    }

    clear_override();
    println!("{MSG_SUCCESS}");
}
