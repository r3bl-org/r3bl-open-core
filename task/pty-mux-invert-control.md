# Refactor PTY Mux: Invert Control and Decouple UI (SOC)

## Context
Currently, the `PTYMux` engine takes over the main thread with a blocking `loop {}`, meaning it owns the event loop, handles all rendering itself, and hardcodes the status bar text directly in its `OutputRenderer`. This limits composability and makes it difficult for applications to integrate `PTYMux` seamlessly.

To solve this, we need to invert the control flow, turning `PTYMux` into an event-driven engine that the application can drive.

## Goals & Benefits
- **Widget Composability**: `PTYMux` becomes a reusable component yielding an `OffscreenBuffer`, allowing apps to seamlessly place terminals in split panes or layouts.
- **App-Level Interception**: The app owns the event loop, meaning it can easily intercept keys to render global overlays (like a Command Palette) without the engine swallowing the events.
- **True Decoupling**: The application handles the final screen composite and `.flush()`, letting it draw its own custom status bars without the engine knowing about them.

## [ ] Phase 1: Decouple Status Bar UI
- Refactor `output_renderer.rs` so that the `pty_mux` engine does not hardcode the status bar string (e.g., `F1: Switch | Ctrl+Q: Quit`).
- Remove the status bar compositing logic from the core engine.

## [ ] Phase 2: Invert the Event Loop
- Refactor `PTYMux` into an event-driven engine. Instead of a blocking `loop {}`, expose methods like `engine.tick()` and `engine.handle_input()`.
- Shift the blocking `crossterm` event polling loop out of `PTYMux` and into `pty_mux_example`.

## [ ] Phase 3: Application-Driven Rendering
- Let `pty_mux_example` retrieve the `OffscreenBuffer` from the engine, composite its own custom status bar UI on top of it, and then explicitly call `.flush()` to draw to the terminal.
