// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// CSI (Control Sequence Introducer) handling
mod csi_codes;
pub use csi_codes::*;

// Utility trait for parsing VTE parameters.
mod vte_params_ext;
pub use vte_params_ext::*;

// Types for parsing VTE OSC parameters.
mod vte_osc_params;
pub use vte_osc_params::*;

// NOTE: Constants have been moved to `core::ansi::constants::*` module
// NOTE: ESC sequence builders moved to `core::ansi::generator::esc`
// NOTE: DSR sequence builders moved to `core::ansi::generator::dsr`
// NOTE: Generic ANSI constants moved to `core::ansi::constants::generic`
//
// Old modules (for reference during transition):
// - dsr_codes.rs (enums moved to generator/dsr.rs)
// - esc_codes.rs (enums moved to generator/esc.rs)
// - generic_ansi_constants.rs (moved to constants/generic.rs)
