// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Example demonstrating bidirectional PTY communication with Python REPL.
//!
//! This program shows how to interact with a Python interpreter running in a PTY,
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

use miette::IntoDiagnostic;
use r3bl_tui::{core::pty::{ControlChar, PtyCommandBuilder, PtyConfigOption, PtyEvent,
                           PtyInput},
               set_mimalloc_in_main};
use tokio::time::{Duration, sleep};

// ANSI color constants for terminal output.
const YELLOW: &str = "\x1b[93m";
const GREEN: &str = "\x1b[92m";
const BLUE: &str = "\x1b[94m";
const CYAN: &str = "\x1b[96m";
const RED: &str = "\x1b[91m";
const RESET: &str = "\x1b[0m";

/// Runs an interactive Python REPL session.
#[allow(clippy::too_many_lines)]
async fn run_python_repl_demo() -> miette::Result<()> {
    println!("{YELLOW}🐍 Starting Python REPL session...{RESET}\n");

    // Start Python with unbuffered output for immediate feedback
    let mut session = PtyCommandBuilder::new("python3")
        .args(["-u", "-i"]) // -u: unbuffered, -i: interactive
        .spawn_read_write(PtyConfigOption::Output)?;

    // Spawn a task to handle output
    let output_handle = tokio::spawn(async move {
        let mut buffer = String::new();

        while let Some(event) = session.event_receiver_half.recv().await {
            match event {
                PtyEvent::Output(data) => {
                    let text = String::from_utf8_lossy(&data);
                    buffer.push_str(&text);

                    // Print output with color coding
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
                PtyEvent::Exit(status) => {
                    println!("\n{YELLOW}Python exited with status: {status:?}{RESET}");
                    break;
                }
                _ => {}
            }
        }

        buffer
    });

    // Wait a bit for Python to start
    sleep(Duration::from_millis(500)).await;

    // Demo: Basic arithmetic
    println!("{BLUE}📝 Sending: Basic arithmetic{RESET}");
    session
        .input_sender_half
        .send(PtyInput::WriteLine("2 + 2".into()))
        .unwrap();
    sleep(Duration::from_millis(200)).await;

    // Demo: Variables and strings
    println!("\n{BLUE}📝 Sending: Variable assignment{RESET}");
    session
        .input_sender_half
        .send(PtyInput::WriteLine("name = 'PTY Demo'".into()))
        .unwrap();
    sleep(Duration::from_millis(200)).await;

    println!("{BLUE}📝 Sending: Print variable{RESET}");
    session
        .input_sender_half
        .send(PtyInput::WriteLine("print(f'Hello from {name}!')".into()))
        .unwrap();
    sleep(Duration::from_millis(200)).await;

    // Demo: Lists and loops
    println!("\n{BLUE}📝 Sending: Create a list{RESET}");
    session
        .input_sender_half
        .send(PtyInput::WriteLine("numbers = [1, 2, 3, 4, 5]".into()))
        .unwrap();
    sleep(Duration::from_millis(200)).await;

    println!("{BLUE}📝 Sending: List comprehension{RESET}");
    session
        .input_sender_half
        .send(PtyInput::WriteLine("[x**2 for x in numbers]".into()))
        .unwrap();
    sleep(Duration::from_millis(200)).await;

    // Demo: Functions
    println!("\n{BLUE}📝 Sending: Define a function{RESET}");
    session
        .input_sender_half
        .send(PtyInput::WriteLine("def greet(name):".into()))
        .unwrap();
    sleep(Duration::from_millis(100)).await;
    session
        .input_sender_half
        .send(PtyInput::WriteLine("    return f'Hello, {name}!'".into()))
        .unwrap();
    sleep(Duration::from_millis(100)).await;
    session
        .input_sender_half
        .send(PtyInput::WriteLine(String::new()))
        .unwrap(); // Empty line to end function
    sleep(Duration::from_millis(200)).await;

    println!("{BLUE}📝 Sending: Call the function{RESET}");
    session
        .input_sender_half
        .send(PtyInput::WriteLine("greet('PTY User')".into()))
        .unwrap();
    sleep(Duration::from_millis(200)).await;

    // Demo: Error handling
    println!("\n{BLUE}📝 Sending: Intentional error to show error handling{RESET}");
    session
        .input_sender_half
        .send(PtyInput::WriteLine("1 / 0".into()))
        .unwrap();
    sleep(Duration::from_millis(200)).await;

    // Demo: Import a module
    println!("\n{BLUE}📝 Sending: Import a module{RESET}");
    session
        .input_sender_half
        .send(PtyInput::WriteLine("import sys".into()))
        .unwrap();
    sleep(Duration::from_millis(200)).await;

    println!("{BLUE}📝 Sending: Check Python version{RESET}");
    session
        .input_sender_half
        .send(PtyInput::WriteLine("sys.version".into()))
        .unwrap();
    sleep(Duration::from_millis(200)).await;

    // Exit Python
    println!("\n{BLUE}📝 Sending: Exit command (Ctrl-D){RESET}");
    session
        .input_sender_half
        .send(PtyInput::SendControl(ControlChar::CtrlD))
        .unwrap();

    // Wait for output task to complete
    let final_output = output_handle.await.into_diagnostic()?;

    println!(
        "\n{YELLOW}═══════════════════════════════════════════════════════════{RESET}"
    );
    println!("{GREEN}✅ Python REPL session completed successfully!{RESET}");
    println!(
        "{YELLOW}Total output captured: {} bytes{RESET}",
        final_output.len()
    );

    Ok(())
}

/// Demonstrates an interactive shell session with multiple commands.
async fn run_shell_demo() -> miette::Result<()> {
    println!("\n{YELLOW}🐚 Starting shell session demo...{RESET}\n");

    // Start a shell session
    let mut session = PtyCommandBuilder::new("sh")
        .args(["-i"]) // Interactive mode
        .spawn_read_write(PtyConfigOption::Output)?;

    // Spawn output handler
    let output_handle = tokio::spawn(async move {
        while let Some(event) = session.event_receiver_half.recv().await {
            match event {
                PtyEvent::Output(data) => {
                    print!("{CYAN}{}{RESET}", String::from_utf8_lossy(&data));
                }
                PtyEvent::Exit(status) => {
                    println!("\n{YELLOW}Shell exited with status: {status:?}{RESET}");
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for shell to start
    sleep(Duration::from_millis(500)).await;

    // Demo: Basic commands
    println!("{BLUE}📝 Sending: pwd{RESET}");
    session
        .input_sender_half
        .send(PtyInput::WriteLine("pwd".into()))
        .unwrap();
    sleep(Duration::from_millis(200)).await;

    println!("\n{BLUE}📝 Sending: echo command{RESET}");
    session
        .input_sender_half
        .send(PtyInput::WriteLine("echo 'Hello from PTY shell!'".into()))
        .unwrap();
    sleep(Duration::from_millis(200)).await;

    println!("\n{BLUE}📝 Sending: List files{RESET}");
    session
        .input_sender_half
        .send(PtyInput::WriteLine("ls -la | head -5".into()))
        .unwrap();
    sleep(Duration::from_millis(200)).await;

    println!("\n{BLUE}📝 Sending: Environment variable{RESET}");
    session
        .input_sender_half
        .send(PtyInput::WriteLine("echo \"Home: $HOME\"".into()))
        .unwrap();
    sleep(Duration::from_millis(200)).await;

    // Demo: Interrupt a long-running command
    println!("\n{BLUE}📝 Sending: Start a long command and interrupt it{RESET}");
    session
        .input_sender_half
        .send(PtyInput::WriteLine(
            "sleep 10 && echo 'This should not print'".into(),
        ))
        .unwrap();
    sleep(Duration::from_millis(500)).await;

    println!("{BLUE}📝 Sending: Ctrl-C to interrupt{RESET}");
    session
        .input_sender_half
        .send(PtyInput::SendControl(ControlChar::CtrlC))
        .unwrap();
    sleep(Duration::from_millis(200)).await;

    // Exit shell
    println!("\n{BLUE}📝 Sending: exit{RESET}");
    session
        .input_sender_half
        .send(PtyInput::WriteLine("exit".into()))
        .unwrap();

    output_handle.await.into_diagnostic()?;

    println!("\n{GREEN}✅ Shell session completed successfully!{RESET}");

    Ok(())
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    set_mimalloc_in_main!();

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

    // Run Python REPL demo
    println!("{YELLOW}▶ Demo 1: Python REPL Interaction{RESET}");
    println!(
        "{YELLOW}═══════════════════════════════════════════════════════════{RESET}"
    );
    run_python_repl_demo().await?;

    // Run shell demo
    println!("\n{YELLOW}▶ Demo 2: Shell Session with Command Interruption{RESET}");
    println!(
        "{YELLOW}═══════════════════════════════════════════════════════════{RESET}"
    );
    run_shell_demo().await?;

    println!(
        "\n{GREEN}✨ Demo complete! The spawn_read_write() API successfully demonstrated:\n\
        {GREEN}   • Sending commands to child processes\n\
        {GREEN}   • Receiving and processing output\n\
        {GREEN}   • Handling control characters (Ctrl-C, Ctrl-D)\n\
        {GREEN}   • Managing interactive sessions{RESET}"
    );

    Ok(())
}
