// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Interactive mouse event inspector - visualize and debug mouse input.
//!
//! This example demonstrates how to:
//! - Capture mouse events using `InputDevice`
//! - Render output using `OutputDevice` and direct terminal functions
//! - Enable/disable mouse tracking
//! - Handle different mouse input kinds (press, release, drag, scroll)
//! - Work with mouse buttons and modifier keys
//!
//! ## Usage
//!
//! ```text
//! cargo run --example mouse_inspector
//! ```
//!
//! - **Click** anywhere to mark the position
//! - **Drag** to see motion events
//! - **Scroll** wheel to see scroll events
//! - **Hold modifiers** (Ctrl/Alt) while clicking
//!   - **Note**: Most terminals intercept Shift+Click for text selection, so Shift and
//!     Ctrl+Shift combinations may not be reported
//! - **Press 'q' or Ctrl+C** to quit
//! - **Press 'c'** to clear the canvas
//!
//! ## What You'll Learn
//!
//! 1. **Mouse Input Types**: `MouseDown`, `MouseUp`, `MouseDrag`, `MouseMove`, Scroll
//! 2. **Mouse Buttons**: Left, Middle, Right
//! 3. **Modifiers**: Ctrl, Shift, Alt combinations
//! 4. **Coordinates**: Terminal cell positions (col, row)
//! 5. **Scroll Direction**: Up, Down, Left, Right
//!
//! ## Architecture
//!
//! This example uses `r3bl_tui`'s core APIs:
//! - [`InputDevice`] for reading events (auto-selects backend)
//! - [`OutputDevice`] for rendering (auto-selects backend)
//! - [`RawMode`] for raw terminal mode
//! - Direct terminal output functions for painting
//!
//! [`InputDevice`]: r3bl_tui::InputDevice
//! [`OutputDevice`]: r3bl_tui::OutputDevice
//! [`RawMode`]: r3bl_tui::RawMode

use r3bl_tui::{InputDevice, InputEvent, Key, KeyPress, KeyState, ModifierKeysMask,
               MouseInput, MouseInputKind, OutputDevice, Pos, RawMode, RowIndex,
               TermCol, TermRow, clear_screen_and_home_cursor, flush_output, get_size,
               lock_output_device_as_mut, move_cursor_to, row, set_mimalloc_in_main,
               write_text};
use std::collections::VecDeque;

/// Maximum number of events to keep in history
const MAX_HISTORY: usize = 15;

/// Maximum number of click marks to display
const MAX_CLICK_MARKS: usize = 50;

/// Application state for the mouse inspector
struct MouseInspector {
    /// Visual markers where user clicked
    click_marks: Vec<Pos>,
    /// Recent mouse events (newest first)
    event_history: VecDeque<MouseEvent>,
    /// Most recent mouse event
    latest_event: Option<MouseEvent>,
    /// Whether to exit the application
    should_quit: bool,
}

/// Simplified mouse event for display purposes
#[derive(Clone, Debug)]
struct MouseEvent {
    pos: Pos,
    kind: MouseInputKind,
    ctrl: bool,
    shift: bool,
    alt: bool,
}

impl MouseEvent {
    fn from_mouse_input(input: &MouseInput) -> Self {
        let (ctrl, shift, alt) = if let Some(mask) = input.maybe_modifier_keys {
            (
                mask.ctrl_key_state == KeyState::Pressed,
                mask.shift_key_state == KeyState::Pressed,
                mask.alt_key_state == KeyState::Pressed,
            )
        } else {
            (false, false, false)
        };

        Self {
            pos: input.pos,
            kind: input.kind,
            ctrl,
            shift,
            alt,
        }
    }

    /// Format modifiers as a compact string
    fn modifiers_str(&self) -> String {
        let mut mods = Vec::new();
        if self.ctrl {
            mods.push("Ctrl");
        }
        if self.shift {
            mods.push("Shift");
        }
        if self.alt {
            mods.push("Alt");
        }
        if mods.is_empty() {
            "None".to_string()
        } else {
            mods.join("+")
        }
    }
}

impl MouseInspector {
    fn new() -> Self {
        Self {
            click_marks: Vec::new(),
            event_history: VecDeque::new(),
            latest_event: None,
            should_quit: false,
        }
    }

    /// Process a mouse event, updating state
    fn handle_mouse_event(&mut self, mouse: MouseInput) {
        let event = MouseEvent::from_mouse_input(&mouse);

        // Add click marks for mouse down events
        if matches!(event.kind, MouseInputKind::MouseDown(_)) {
            self.click_marks.push(event.pos);
            // Limit marks to prevent memory growth
            if self.click_marks.len() > MAX_CLICK_MARKS {
                self.click_marks.remove(0);
            }
        }

        // Update history (newest first)
        self.event_history.push_front(event.clone());
        if self.event_history.len() > MAX_HISTORY {
            self.event_history.pop_back();
        }

        self.latest_event = Some(event);
    }

    /// Process a keyboard event
    fn handle_keyboard_event(&mut self, key: KeyPress) {
        // Quit on 'q' or Ctrl+C
        match key {
            KeyPress::Plain {
                key: Key::Character('q' | 'Q'),
            }
            | KeyPress::WithModifiers {
                key: Key::Character('c'),
                mask:
                    ModifierKeysMask {
                        ctrl_key_state: KeyState::Pressed,
                        ..
                    },
            } => {
                self.should_quit = true;
            }
            KeyPress::Plain {
                key: Key::Character('c' | 'C'),
            } => {
                // Clear on 'c' (without Ctrl)
                self.click_marks.clear();
                self.event_history.clear();
                self.latest_event = None;
            }
            _ => {}
        }
    }
}

#[tokio::main]
#[allow(clippy::needless_return)]
async fn main() -> miette::Result<()> {
    set_mimalloc_in_main!();

    // Setup terminal
    let terminal_size = get_size()?;
    let mut output_device = OutputDevice::new_stdout();
    let mut input_device = InputDevice::default();

    // Start raw mode (enables mouse tracking automatically)
    RawMode::start(
        terminal_size,
        lock_output_device_as_mut!(&output_device),
        false,
    );

    // Clear screen
    clear_screen_and_home_cursor(&output_device);

    // Run the inspector
    let result = run_inspector(&mut input_device, &mut output_device).await;

    // Cleanup terminal (disable raw mode and mouse tracking)
    RawMode::end(
        terminal_size,
        lock_output_device_as_mut!(&output_device),
        false,
    );

    result
}

/// Main event loop
async fn run_inspector(
    input_device: &mut InputDevice,
    output_device: &mut OutputDevice,
) -> miette::Result<()> {
    let mut inspector = MouseInspector::new();

    // Initial render
    render(&inspector, output_device);

    // Event loop
    while !inspector.should_quit {
        // Wait for next input event
        if let Some(event) = input_device.next().await {
            match event {
                InputEvent::Mouse(mouse) => inspector.handle_mouse_event(mouse),
                InputEvent::Keyboard(key) => inspector.handle_keyboard_event(key),
                _ => {
                    // Ignore other events (Resize, Focus, BracketedPaste)
                }
            }

            // Re-render after state update
            render(&inspector, output_device);
        }
    }

    Ok(())
}

/// Render the current inspector state using direct terminal output
#[allow(clippy::too_many_lines)] // UI rendering naturally requires many lines
fn render(inspector: &MouseInspector, output: &OutputDevice) {
    // Canvas area for click marks (rows 4-18, using 0-based row indices)
    let canvas_start_row = row(4);
    let canvas_end_row = row(18);

    // Clear screen
    clear_screen_and_home_cursor(output);

    // Draw title and instructions
    move_cursor_to(output, TermRow::from(row(0)).as_u16(), 1);
    write_text(output, "┌──── Mouse Event Inspector ─────┐");

    move_cursor_to(output, TermRow::from(row(1)).as_u16(), 1);
    write_text(output, "│ Click, drag, scroll anywhere!  │");

    move_cursor_to(output, TermRow::from(row(2)).as_u16(), 1);
    write_text(output, "└────────────────────────────────┘");

    // Draw canvas border
    move_cursor_to(output, TermRow::from(canvas_start_row).as_u16(), 1);
    write_text(output, "┌─ Canvas (click anywhere) ──────┐");

    // Draw click marks
    for mark in &inspector.click_marks {
        // Check if mark is within canvas bounds (0-based comparison)
        if mark.row_index > canvas_start_row && mark.row_index < canvas_end_row {
            // Convert to 1-based terminal coordinates for display
            let term_row = TermRow::from(mark.row_index);
            let term_col = TermCol::from(mark.col_index);
            move_cursor_to(output, term_row.as_u16(), term_col.as_u16());
            write_text(output, "●");
        }
    }

    move_cursor_to(output, TermRow::from(canvas_end_row).as_u16(), 1);
    write_text(output, "└────────────────────────────────┘");

    // Display latest event (row 20+)
    move_cursor_to(output, TermRow::from(row(20)).as_u16(), 1);
    write_text(output, "┌─ Latest Event ─────────────────┐");

    if let Some(evt) = &inspector.latest_event {
        move_cursor_to(output, TermRow::from(row(21)).as_u16(), 1);
        write_text(
            output,
            &format!(
                "│ Position: ({:>2}, {:>2})             │",
                evt.pos.col_index.as_usize(),
                evt.pos.row_index.as_usize()
            ),
        );

        move_cursor_to(output, TermRow::from(row(22)).as_u16(), 1);
        let kind_str = match evt.kind {
            MouseInputKind::MouseDown(btn) => format!("MouseDown({btn:?})"),
            MouseInputKind::MouseUp(btn) => format!("MouseUp({btn:?})"),
            MouseInputKind::MouseDrag(btn) => format!("MouseDrag({btn:?})"),
            MouseInputKind::MouseMove => "MouseMove".to_string(),
            MouseInputKind::ScrollUp => "ScrollUp".to_string(),
            MouseInputKind::ScrollDown => "ScrollDown".to_string(),
            MouseInputKind::ScrollLeft => "ScrollLeft".to_string(),
            MouseInputKind::ScrollRight => "ScrollRight".to_string(),
        };
        write_text(output, &format!("│ Kind:     {kind_str:<21}│"));

        move_cursor_to(output, TermRow::from(row(23)).as_u16(), 1);
        write_text(output, &format!("│ Mods:     {:<21}│", evt.modifiers_str()));
    } else {
        move_cursor_to(output, TermRow::from(row(21)).as_u16(), 1);
        write_text(output, "│ No events yet                  │");
        move_cursor_to(output, TermRow::from(row(22)).as_u16(), 1);
        write_text(output, "│                                │");
        move_cursor_to(output, TermRow::from(row(23)).as_u16(), 1);
        write_text(output, "│                                │");
    }

    move_cursor_to(output, TermRow::from(row(24)).as_u16(), 1);
    write_text(output, "└────────────────────────────────┘");

    // Event history (row 26+)
    let history_start_row = row(26);
    let history_first_item_row = row(27);
    let history_end_row = row(37);
    let instructions_row = row(39);

    move_cursor_to(output, TermRow::from(history_start_row).as_u16(), 1);
    write_text(
        output,
        "┌─ Event History (last 10) ────────────────────────────────────────────┐",
    );

    for (i, evt) in inspector.event_history.iter().take(10).enumerate() {
        let row_idx = RowIndex::from(history_first_item_row.as_usize() + i);
        move_cursor_to(output, TermRow::from(row_idx).as_u16(), 1);

        let mods = if evt.ctrl || evt.shift || evt.alt {
            format!(" [{}]", evt.modifiers_str())
        } else {
            String::new()
        };

        let kind_str = match evt.kind {
            MouseInputKind::MouseDown(btn) => format!("Down({btn:?})"),
            MouseInputKind::MouseUp(btn) => format!("Up({btn:?})"),
            MouseInputKind::MouseDrag(btn) => format!("Drag({btn:?})"),
            MouseInputKind::MouseMove => "Move".to_string(),
            MouseInputKind::ScrollUp => "ScrollUp".to_string(),
            MouseInputKind::ScrollDown => "ScrollDown".to_string(),
            MouseInputKind::ScrollLeft => "ScrollLeft".to_string(),
            MouseInputKind::ScrollRight => "ScrollRight".to_string(),
        };

        let line = format!(
            "│ • ({:>2},{:>2}) {:<15}{:<44}│",
            evt.pos.col_index.as_usize(),
            evt.pos.row_index.as_usize(),
            kind_str,
            mods
        );
        write_text(output, &line);
    }

    // Fill remaining history lines with empty bordered lines
    for i in inspector.event_history.len()..10 {
        let row_idx = RowIndex::from(history_first_item_row.as_usize() + i);
        move_cursor_to(output, TermRow::from(row_idx).as_u16(), 1);
        write_text(
            output,
            "│                                                                      │",
        );
    }

    move_cursor_to(output, TermRow::from(history_end_row).as_u16(), 1);
    write_text(
        output,
        "└──────────────────────────────────────────────────────────────────────┘",
    );

    // Instructions at bottom
    move_cursor_to(output, TermRow::from(instructions_row).as_u16(), 1);
    write_text(
        output,
        "[Q] Quit  [C] Clear  • Try Ctrl/Alt with clicks (Shift reserved for text selection)",
    );

    // Flush output to make changes visible
    flush_output(output);
}
