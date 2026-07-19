// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Example demonstrating bidirectional [`PTY`] communication with Python REPL.
//!
//! This program shows how to interact with a Python interpreter running in a [`PTY`],
//! sending commands and receiving output. It demonstrates:
//! - Sending Python code to the interpreter
//! - Receiving and displaying output
//! - Handling control characters (Ctrl-D for exit)
//! - Processing both stdout and stderr combined
//!
//! # Usage
//!
//! Run this binary to see an interactive Python session:
//! ```bash
//! cargo run --example spawn_pty_read_write
//! ```
//!
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use miette::IntoDiagnostic;
use r3bl_tui::{DefaultIoDevices, SGR_FG_BRIGHT_BLUE_STR, SGR_FG_BRIGHT_CYAN_STR,
               SGR_FG_BRIGHT_GREEN_STR, SGR_FG_BRIGHT_RED_STR, SGR_FG_BRIGHT_YELLOW_STR,
               SGR_RESET_STR, TuiAvailabilityChooseExt, assert_terminal_is_interactive,
               choose,
               core::pty::{ControlSequence, CursorKeyMode, DefaultPtySessionConfig,
                           PtyInputEvent, PtyOutputEvent, PtySessionBuilder,
                           PtySessionConfigOption},
               ok,
               readline_async::{HowToChoose, style::StyleSheet},
               set_mimalloc_in_main, vp_height, vp_width};
use tokio::time::{Duration, sleep};

// ANSI color constants for terminal output.
const YELLOW: &str = SGR_FG_BRIGHT_YELLOW_STR;
const GREEN: &str = SGR_FG_BRIGHT_GREEN_STR;
const BLUE: &str = SGR_FG_BRIGHT_BLUE_STR;
const CYAN: &str = SGR_FG_BRIGHT_CYAN_STR;
const RED: &str = SGR_FG_BRIGHT_RED_STR;
const RESET: &str = SGR_RESET_STR;

/// Runs an interactive Python REPL session.
#[allow(clippy::too_many_lines)]
async fn run_python_repl_demo() -> miette::Result<()> {
    println!("{YELLOW}🐍 Starting Python REPL session...{RESET}\n");

    // Start Python with unbuffered output for immediate feedback.
    let mut session = PtySessionBuilder::new("python3")
        .cli_args(["-u", "-i"]) // -u: unbuffered, -i: interactive
        .with_config(
            DefaultPtySessionConfig
                + PtySessionConfigOption::Size(vp_width(80) + vp_height(24)),
        )
        .start()?;

    // Spawn a task to handle output.
    let output_handle = tokio::spawn(async move {
        let mut buffer = String::new();

        while let Some(event) = session.rx_output_event.recv().await {
            match event {
                PtyOutputEvent::Output(data) => {
                    let text = String::from_utf8_lossy(&data);
                    buffer.push_str(&text);

                    // Print output with color coding.
                    for line in text.lines() {
                        if line.starts_with(">>>") || line.starts_with("...") {
                            print!("{CYAN}{line}{RESET}");
                        } else if line.contains("Error") || line.contains("Traceback") {
                            print!("{RED}{line}{RESET}");
                        } else {
                            print!("{GREEN}{line}{RESET}");
                        }
                        println!();
                    }
                }
                PtyOutputEvent::Exit(status) => {
                    println!("\n{YELLOW}Python exited with status: {status:?}{RESET}");
                    break;
                }
                _ => {}
            }
        }

        buffer
    });

    // Wait a bit for Python to start.
    sleep(Duration::from_millis(500)).await;

    // Demo: Basic arithmetic.
    println!("{BLUE}📝 Sending: Basic arithmetic{RESET}");
    session
        .tx_input_event
        .try_send(PtyInputEvent::WriteLine("2 + 2".into()))
        .expect("conversion error");
    sleep(Duration::from_millis(200)).await;

    // Demo: Variables and strings.
    println!("\n{BLUE}📝 Sending: Variable assignment{RESET}");
    session
        .tx_input_event
        .try_send(PtyInputEvent::WriteLine("name = 'PTY Demo'".into()))
        .expect("conversion error");
    sleep(Duration::from_millis(200)).await;

    println!("{BLUE}📝 Sending: Print variable{RESET}");
    session
        .tx_input_event
        .try_send(PtyInputEvent::WriteLine(
            "print(f'Hello from {name}!')".into(),
        ))
        .expect("conversion error");
    sleep(Duration::from_millis(200)).await;

    // Demo: Lists and loops.
    println!("\n{BLUE}📝 Sending: Create a list{RESET}");
    session
        .tx_input_event
        .try_send(PtyInputEvent::WriteLine("numbers = [1, 2, 3, 4, 5]".into()))
        .expect("conversion error");
    sleep(Duration::from_millis(200)).await;

    println!("{BLUE}📝 Sending: List comprehension{RESET}");
    session
        .tx_input_event
        .try_send(PtyInputEvent::WriteLine("[x**2 for x in numbers]".into()))
        .expect("conversion error");
    sleep(Duration::from_millis(200)).await;

    // Demo: Functions.
    println!("\n{BLUE}📝 Sending: Define a function{RESET}");
    session
        .tx_input_event
        .try_send(PtyInputEvent::WriteLine("def greet(name):".into()))
        .expect("conversion error");
    sleep(Duration::from_millis(100)).await;
    session
        .tx_input_event
        .try_send(PtyInputEvent::WriteLine(
            "    return f'Hello, {name}!'".into(),
        ))
        .expect("conversion error");
    sleep(Duration::from_millis(100)).await;
    session
        .tx_input_event
        .try_send(PtyInputEvent::WriteLine(String::new()))
        .expect("conversion error"); // Empty line to end function
    sleep(Duration::from_millis(200)).await;

    println!("{BLUE}📝 Sending: Call the function{RESET}");
    session
        .tx_input_event
        .try_send(PtyInputEvent::WriteLine("greet('PTY User')".into()))
        .expect("conversion error");
    sleep(Duration::from_millis(200)).await;

    // Demo: Error handling.
    println!("\n{BLUE}📝 Sending: Intentional error to show error handling{RESET}");
    session
        .tx_input_event
        .try_send(PtyInputEvent::WriteLine("1 / 0".into()))
        .expect("conversion error");
    sleep(Duration::from_millis(200)).await;

    // Demo: Import a module.
    println!("\n{BLUE}📝 Sending: Import a module{RESET}");
    session
        .tx_input_event
        .try_send(PtyInputEvent::WriteLine("import sys".into()))
        .expect("conversion error");
    sleep(Duration::from_millis(200)).await;

    println!("{BLUE}📝 Sending: Check Python version{RESET}");
    session
        .tx_input_event
        .try_send(PtyInputEvent::WriteLine("sys.version".into()))
        .expect("conversion error");
    sleep(Duration::from_millis(200)).await;

    // Exit Python.
    println!("\n{BLUE}📝 Sending: Exit command (Ctrl-D){RESET}");
    session
        .tx_input_event
        .try_send(PtyInputEvent::SendControl(
            ControlSequence::CtrlD,
            CursorKeyMode::default(),
        ))
        .expect("conversion error");

    // Wait for output task to complete.
    let final_output = output_handle.await.into_diagnostic()?;

    println!(
        "\n{YELLOW}═══════════════════════════════════════════════════════════{RESET}"
    );
    println!("{GREEN}✅ Python REPL session completed successfully!{RESET}");
    println!(
        "{YELLOW}Total output captured: {} bytes{RESET}",
        final_output.len()
    );

    ok!()
}

/// Demonstrates an interactive shell session with multiple commands.
async fn run_shell_demo() -> miette::Result<()> {
    println!("\n{YELLOW}🐚 Starting shell session demo...{RESET}\n");

    // Start a shell session.
    let mut session = PtySessionBuilder::new("sh")
        .cli_args(["-i"]) // Interactive mode
        .with_config(
            DefaultPtySessionConfig
                + PtySessionConfigOption::Size(vp_width(80) + vp_height(24)),
        )
        .start()?;

    // Spawn output handler.
    let output_handle = tokio::spawn(async move {
        while let Some(event) = session.rx_output_event.recv().await {
            match event {
                PtyOutputEvent::Output(data) => {
                    print!("{CYAN}{}{RESET}", String::from_utf8_lossy(&data));
                }
                PtyOutputEvent::Exit(status) => {
                    println!("\n{YELLOW}Shell exited with status: {status:?}{RESET}");
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for shell to start.
    sleep(Duration::from_millis(500)).await;

    // Demo: Basic commands.
    println!("{BLUE}📝 Sending: pwd{RESET}");
    session
        .tx_input_event
        .try_send(PtyInputEvent::WriteLine("pwd".into()))
        .expect("conversion error");
    sleep(Duration::from_millis(200)).await;

    println!("\n{BLUE}📝 Sending: echo command{RESET}");
    session
        .tx_input_event
        .try_send(PtyInputEvent::WriteLine(
            "echo 'Hello from PTY shell!'".into(),
        ))
        .expect("conversion error");
    sleep(Duration::from_millis(200)).await;

    println!("\n{BLUE}📝 Sending: List files{RESET}");
    session
        .tx_input_event
        .try_send(PtyInputEvent::WriteLine("ls -la | head -5".into()))
        .expect("conversion error");
    sleep(Duration::from_millis(200)).await;

    println!("\n{BLUE}📝 Sending: Environment variable{RESET}");
    session
        .tx_input_event
        .try_send(PtyInputEvent::WriteLine("echo \"Home: $HOME\"".into()))
        .expect("conversion error");
    sleep(Duration::from_millis(200)).await;

    // Demo: Interrupt a long-running command.
    println!("\n{BLUE}📝 Sending: Start a long command and interrupt it{RESET}");
    session
        .tx_input_event
        .try_send(PtyInputEvent::WriteLine(
            "sleep 10 && echo 'This should not print'".into(),
        ))
        .expect("conversion error");
    sleep(Duration::from_millis(500)).await;

    println!("{BLUE}📝 Sending: Ctrl-C to interrupt{RESET}");
    session
        .tx_input_event
        .try_send(PtyInputEvent::SendControl(
            ControlSequence::CtrlC,
            CursorKeyMode::default(),
        ))
        .expect("conversion error");
    sleep(Duration::from_millis(200)).await;

    // Exit shell.
    println!("\n{BLUE}📝 Sending: exit{RESET}");
    session
        .tx_input_event
        .try_send(PtyInputEvent::WriteLine("exit".into()))
        .expect("conversion error");

    output_handle.await.into_diagnostic()?;

    println!("\n{GREEN}✅ Shell session completed successfully!{RESET}");

    ok!()
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    set_mimalloc_in_main!();
    assert_terminal_is_interactive();

    println!(
        "\
        {YELLOW}╔═══════════════════════════════════════════════════════════════╗\n\
        {YELLOW}║     Demo: Bidirectional PTY Communication (Read-Write)        ║\n\
        {YELLOW}╚═══════════════════════════════════════════════════════════════╝{RESET}"
    );

    println!(
        "\n{CYAN}This demo shows how to interact with child processes through PTY.{RESET}"
    );
    println!("{CYAN}We'll demonstrate both Python REPL and shell interactions.{RESET}\n");

    // Ask user which demo to run.
    let maybe_user_choice = {
        let mut default_io_devices = DefaultIoDevices::default();
        choose(
            "🚀 Which PTY Demo would you like to run?",
            &[
                "1. Run Both Demos (Python REPL + Shell Interruption)",
                "2. Demo 1: Python REPL Interaction",
                "3. Demo 2: Shell Session with Command Interruption",
                "4. Exit",
            ],
            None,
            None,
            HowToChoose::Single,
            StyleSheet::default(),
            default_io_devices.as_mut_tuple(),
        )
        .get_first_result()
        .await?
    };

    let Some(choice) = maybe_user_choice else {
        println!("👋 Exiting.");
        return ok!();
    };

    if choice.starts_with('4') {
        println!("👋 Exiting.");
        return ok!();
    }

    if choice.starts_with('1') || choice.starts_with('2') {
        println!("\n{YELLOW}▶ Demo 1: Python REPL Interaction{RESET}");
        println!(
            "{YELLOW}═══════════════════════════════════════════════════════════{RESET}"
        );
        run_python_repl_demo().await?;
    }

    if choice.starts_with('1') || choice.starts_with('3') {
        println!("\n{YELLOW}▶ Demo 2: Shell Session with Command Interruption{RESET}");
        println!(
            "{YELLOW}═══════════════════════════════════════════════════════════{RESET}"
        );
        run_shell_demo().await?;
    }

    println!(
        "\n{GREEN}✨ Demo complete! The start() API successfully demonstrated:\n\
        {GREEN}   • Sending commands to child processes\n\
        {GREEN}   • Receiving and processing output\n\
        {GREEN}   • Handling control characters (Ctrl-C, Ctrl-D)\n\
        {GREEN}   • Managing interactive sessions{RESET}"
    );

    ok!()
}
