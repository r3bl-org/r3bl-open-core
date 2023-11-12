pub mod offscreen_buffer;
pub mod paint;
pub mod render_op;
pub mod crossterm_backend;
pub mod color_converter;
pub mod terminal_lib_operations;

pub enum TerminalLibBackend {
    Crossterm,
    Termion,
}
pub const TERMINAL_LIB_BACKEND: TerminalLibBackend = TerminalLibBackend::Crossterm;

pub use offscreen_buffer::*;
pub use render_op::*;
pub use crossterm_backend::*;
pub use paint::*;
pub use color_converter::*;
pub use terminal_lib_operations::*;
