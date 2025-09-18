// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test modules for VT100 ANSI conformance validation.
//!
//! This module organizes conformance tests by functionality and architectural layer:
//! - Operations tests (test_*_ops.rs) - Test operations modules directly
//! - Protocol tests (protocol_*.rs) - Test ANSI/VT100 protocol parsing
//! - System tests (system_*.rs) - Test system components and lifecycle
//! - Integration tests (integration_*.rs) - Test cross-cutting scenarios
//! - Utilities (util_*.rs) - Test utilities and fixtures

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
mod protocol_csi_basic;

#[cfg(test)]
mod protocol_char_encoding;

#[cfg(test)]
mod protocol_control_chars;

// === SYSTEM TESTS ===
// These test system components and lifecycle management

#[cfg(test)]
mod system_error_handling;

#[cfg(test)]
mod system_performer_lifecycle;

#[cfg(test)]
mod system_state_management;

// === INTEGRATION TESTS ===
// These test cross-cutting scenarios and real-world use cases

#[cfg(test)]
mod integration_basic;

#[cfg(test)]
mod integration_real_world;

// === UTILITIES ===
// Test utilities and fixtures

#[cfg(test)]
mod util_fixtures;
