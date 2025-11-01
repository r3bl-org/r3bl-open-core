// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Private modules (hide internal structure).
mod input_device;
mod input_device_ext;
mod output_device;
mod shared_writer;
mod terminal_io_type_aliases;
mod enhanced_keys;
mod input_event;
mod key_press;
mod modifier_keys_mask;
mod mouse_input;

// Re-exports for flat public API.
pub use enhanced_keys::*;
pub use input_device::*;
pub use input_device_ext::*;
pub use input_event::*;
pub use key_press::*;
pub use modifier_keys_mask::*;
pub use mouse_input::*;
pub use output_device::*;
pub use shared_writer::*;
pub use terminal_io_type_aliases::*;
