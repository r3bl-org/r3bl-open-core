// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#[cfg(test)]
pub(super) mod tests_fixtures; // Make fixtures accessible to parent module.

#[cfg(test)]
mod tests_basic_csi_operations;

#[cfg(test)]
mod tests_character_encoding;

#[cfg(test)]
mod tests_character_operations;

#[cfg(test)]
mod tests_control_characters;

#[cfg(test)]
mod tests_cursor_operations;

#[cfg(test)]
mod tests_dsr_responses;

#[cfg(test)]
mod tests_integration;

#[cfg(test)]
mod tests_line_operations;

#[cfg(test)]
mod tests_line_wrap_and_scroll_control;

#[cfg(test)]
mod tests_osc_sequences;

#[cfg(test)]
mod tests_performer_lifecycle;

#[cfg(test)]
mod tests_sgr_and_character_sets;
