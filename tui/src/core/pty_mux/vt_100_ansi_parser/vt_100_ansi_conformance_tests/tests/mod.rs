// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test modules for VT100 ANSI conformance validation.
//!
//! This module organizes conformance tests by functionality and architectural layer:
//! - Operations tests (test_*_ops.rs) - Test operations modules directly
//! - Protocol tests (test_protocol_*.rs) - Test ANSI/VT100 protocol parsing
//! - System tests (test_system_*.rs) - Test system components and lifecycle
//! - Integration tests (test_integration_*.rs) - Test cross-cutting scenarios

// === OPERATIONS TESTS ===
// These test files map 1:1 with operations/ modules

#[cfg(test)]
mod test_char_ops;

#[cfg(test)]
mod test_control_ops;

#[cfg(test)]
mod test_cursor_ops;

#[cfg(test)]
mod test_dsr_ops;

#[cfg(test)]
mod test_line_ops;

#[cfg(test)]
mod test_margin_ops;

#[cfg(test)]
mod test_mode_ops;

#[cfg(test)]
mod test_osc_ops;

#[cfg(test)]
mod test_scroll_ops_regions;

#[cfg(test)]
mod test_scroll_ops_wrap;

#[cfg(test)]
mod test_sgr_ops;

#[cfg(test)]
mod test_terminal_ops;

// === PROTOCOL TESTS ===
// These test ANSI/VT100 protocol parsing and sequence handling

#[cfg(test)]
mod test_protocol_csi_basic;

#[cfg(test)]
mod test_protocol_char_encoding;

#[cfg(test)]
mod test_protocol_control_chars;

// === SYSTEM TESTS ===
// These test system components and lifecycle management

#[cfg(test)]
mod test_system_error_handling;

#[cfg(test)]
mod test_system_performer_lifecycle;

#[cfg(test)]
mod test_system_state_management;

// === INTEGRATION TESTS ===
// These test cross-cutting scenarios and real-world use cases

#[cfg(test)]
mod test_integration_basic;

#[cfg(test)]
mod test_integration_real_world;
