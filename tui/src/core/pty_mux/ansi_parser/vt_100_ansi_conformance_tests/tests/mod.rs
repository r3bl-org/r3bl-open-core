// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test modules for VT100 ANSI conformance validation.
//!
//! This module organizes conformance tests by functionality, using the
//! sequence builders from the conformance_data module to create readable,
//! maintainable tests that validate against VT100/ANSI specifications.

#[cfg(test)]
mod test_basic_csi_operations;

#[cfg(test)]
mod test_character_encoding;

#[cfg(test)]
mod test_character_operations;

#[cfg(test)]
mod test_control_characters;

#[cfg(test)]
mod test_cursor_operations;

#[cfg(test)]
mod test_dsr_responses;

#[cfg(test)]
mod test_integration;

#[cfg(test)]
mod test_line_operations;

#[cfg(test)]
mod test_line_wrap_and_scroll_control;

#[cfg(test)]
mod test_mode_operations;

#[cfg(test)]
mod test_osc_sequences;

#[cfg(test)]
mod test_performer_lifecycle;

#[cfg(test)]
mod test_sgr_and_character_sets;

#[cfg(test)]
mod test_state_management;

#[cfg(test)]
mod test_scroll_region_edge_cases;

#[cfg(test)]
mod test_tab_operations;

#[cfg(test)]
mod test_error_handling;

// New test module for real-world scenarios
#[cfg(test)]
mod test_real_world_scenarios;