# Proposed Reorganization for `tui/src/core`

The current folder structure mixes different levels of abstraction. For instance, the high-level `pty_mux` contains the low-level `ansi_parser`, and the `ansi` folder contains a mix of low-level protocol definitions and higher-level terminal control functions.

A better organization would group files by their architectural layer and functionality.

## Proposed High-Level Structure

I suggest reorganizing the `core` directory into three main modules that represent the distinct layers of the application:

1.  **`terminal/`**: A new module to contain everything related to terminal emulation and interaction. This includes low-level protocols, the ANSI parser, color handling, and high-level abstractions like styled text.
2.  **`pty/`**: This module would be dedicated to the logic of spawning and managing a **single** pseudo-terminal (PTY) process. It becomes a foundational, reusable component.
3.  **`pty_multiplexer/`**: This module remains for the top-level application logic, but it would be streamlined to focus purely on orchestrating multiple PTY sessions, using the other two modules as its building blocks.

---

## Detailed Breakdown

Here’s how the existing files would map to this new structure:

### 1. `terminal/` - Terminal Emulation and Abstraction

This module would become the home for all code that deals with what a terminal *is* and how to *talk* to it, independent of PTYs.

-   **`terminal/protocols/`**: A new sub-directory for raw protocol definitions.
    -   `ansi.rs`: For CSI, SGR, and ESC sequence definitions (from `ansi/`, `csi_codes.rs`, `esc_codes.rs`).
    -   `osc.rs`: For OSC sequence definitions (from `osc/`).
    -   `dsr.rs`: For DSR sequence definitions (from `dsr/`).

-   **`terminal/parser/`**: The new home for the `ansi_parser` currently in `pty_mux`. Its responsibility is to take a byte stream and update a virtual screen (`OffscreenBuffer`), making it a core part of terminal emulation.

-   **`terminal/color/`**: Consolidates all color-related logic, including `ASTColor`, color conversion, and transformation traits (from `ansi/`).

-   **`terminal/styled_text.rs`**: The high-level `AnsiStyledText` struct, which is a user-facing abstraction for creating styled output.

-   **`terminal/capabilities.rs`**: For detecting terminal features like color and hyperlink support (from `ansi/detect_color_support.rs`).

-   **`terminal/commands.rs`**: For high-level, imperative commands to control the terminal, like `clear_screen()` (from `ansi/terminal_output.rs`).

### 2. `pty/` - Single PTY Process Management

This module's responsibility would be narrowed to managing the lifecycle of a single PTY process. It provides the foundational components needed by the multiplexer.

-   It would contain the logic from the current `pty/` directory but with a flatter, cleaner structure.
-   Files like `pty_command_builder.rs`, `pty_config.rs`, and the logic from `pty_read_only.rs` and `pty_read_write.rs` would live here.
-   The `pty_core` sub-directory would be removed, and its contents merged into the parent `pty` module for simplicity.

### 3. `pty_multiplexer/` - High-Level Orchestration

This module would now be purely for the application logic of managing *multiple* PTY sessions.

-   It would contain the main `PTYMux` struct, the `ProcessManager`, `InputRouter`, and `OutputRenderer`.
-   It would **use** the `pty` module to spawn its processes and the `terminal/parser` to interpret the output from each process into its respective `OffscreenBuffer`.
-   By moving the low-level parser out, this module's code becomes much more focused on its core responsibility: multiplexing.

### Benefits of this Reorganization

-   **Clear Separation of Concerns**: Low-level protocol details are separated from high-level application logic.
-   **Improved Layering**: The architecture becomes clearly layered, from raw protocols up to the final application.
-   **Enhanced Reusability**: The `terminal` and `pty` modules become more self-contained and could potentially be reused in other projects.
-   **Easier Navigation**: It's more intuitive to find code related to terminal emulation versus PTY process management.
