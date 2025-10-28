// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// CSI (Control Sequence Introducer) handling
mod csi_codes;
pub use csi_codes::*;

// Utility trait for parsing VTE parameters
mod params_ext;
pub use params_ext::*;

// NOTE: Constants have been moved to `core::ansi::constants::*` module
// NOTE: ESC sequence builders moved to `core::ansi::generator::esc_sequence`
// NOTE: DSR sequence builders moved to `core::ansi::generator::dsr_sequence`
// NOTE: Generic ANSI constants moved to `core::ansi::constants::generic`
//
// Old modules (for reference during transition):
// - dsr_codes.rs (enums moved to generator/dsr_sequence.rs)
// - esc_codes.rs (enums moved to generator/esc_sequence.rs)
// - generic_ansi_constants.rs (moved to constants/generic.rs)
