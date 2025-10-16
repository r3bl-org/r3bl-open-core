// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Margin request types for DECSTBM (Set Top and Bottom Margins) operations.
//!
//! This module handles scrolling region margin settings, which define the area
//! where scrolling operations occur.

        use crate::ParamsExt;
use crate::{TermRow, term_row};
use std::{cmp::max, num::NonZeroU16};

/// Margin request types for DECSTBM (Set Top and Bottom Margins) operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarginRequest {
    /// Reset margins to full screen (ESC[r, ESC[0r, ESC[0;0r)
    Reset,
    /// Set specific scrolling region margins
    SetRegion { top: TermRow, bottom: TermRow },
}

impl From<(Option<u16>, Option<u16>)> for MarginRequest {
    fn from((maybe_top, maybe_bottom): (Option<u16>, Option<u16>)) -> Self {
        // VT100 spec: missing params or zero params mean reset to full screen.
        match (maybe_top, maybe_bottom) {
            (None | Some(0), None) | (Some(0), Some(0)) => Self::Reset,
            _ => {
                // Convert to 1-based terminal coordinates (VT100 spec uses 1-based).
                let top_row = maybe_top.map_or(1, |v| max(v, 1));
                let bottom_row = maybe_bottom.map_or(24, |v| max(v, 1)); // Default to 24

                // SAFETY: max(v, 1) guarantees values >= 1
                debug_assert!(top_row > 0 && bottom_row > 0);
                let top_nz = unsafe { NonZeroU16::new_unchecked(top_row) };
                let bottom_nz = unsafe { NonZeroU16::new_unchecked(bottom_row) };

                Self::SetRegion {
                    top: term_row(top_nz),
                    bottom: term_row(bottom_nz),
                }
            }
        }
    }
}

impl From<&vte::Params> for MarginRequest {
    fn from(params: &vte::Params) -> Self {
        let maybe_top = params.extract_nth_single_opt_raw(0);
        let maybe_bottom = params.extract_nth_single_opt_raw(1);
        (maybe_top, maybe_bottom).into()
    }
}
