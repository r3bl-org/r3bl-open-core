// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Percentage-based types for specifications and metrics.
//!
//! This module provides types for working with percentages (0-100%) across various
//! domains. These are **specification types** distinct from concrete coordinate values.
//!
//! # Core Types
//!
//! - [`Pc`]: Percentage value (0-100%) for metrics and specifications
//! - [`ReqSizePc`]: Requested UI size expressed as width/height percentages
//!
//! # Usage Domains
//!
//! ## 1. UI Layout Specifications
//!
//! Used as **inputs** to layout calculations which produce concrete buffer coordinates:
//! - Flexbox-style layout engines
//! - Responsive UI sizing
//! - Proportional distribution of space
//!
//! The layout engine converts these specifications into concrete [`ColWidth`] and
//! [`RowHeight`] values.
//!
//! ## 2. Telemetry & Performance Metrics
//!
//! Used to represent **percentage-based measurements**:
//! - Distribution percentages in performance histograms
//! - Frequency analysis of telemetry data
//! - Statistical clustering metrics
//!
//! See [`crate::Telemetry`] for usage in performance monitoring.
//!
//! [`ColWidth`]: crate::coordinates::buffer_coords::ColWidth
//! [`RowHeight`]: crate::coordinates::buffer_coords::RowHeight
//! [`ReqSizePc`]: crate::coordinates::percent_spec::ReqSizePc
//! [`Pc`]: crate::coordinates::percent_spec::Pc

// Attach source files.
pub mod pc;
pub mod req_size_pc;

// Re-export.
pub use pc::*;
pub use req_size_pc::*;
