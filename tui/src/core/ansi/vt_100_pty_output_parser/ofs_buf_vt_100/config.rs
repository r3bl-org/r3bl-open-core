// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{StorageLineLimit, VPSize};

/// Configuration for the [`OfsBufVT100`] terminal state parser.
///
/// This struct holds the initial dimensions of the offscreen buffer ([`VPSize`]) and the
/// capacity limit for retaining lines in storage ([`StorageLineLimit`]).
///
/// It supports flexible initialization through the [`From`] trait, defaulting to infinite
/// line storage capacity if only a [`VPSize`] is provided.
///
/// [`OfsBufVT100`]: crate::core::ansi::OfsBufVT100
/// [`StorageLineLimit`]: crate::StorageLineLimit
/// [`VPSize`]: crate::VPSize
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OfsBufVT100Config {
    pub window_size: VPSize,
    pub storage_line_limit: StorageLineLimit,
}

/// This module provides [`From`] trait implementations to build an [`OfsBufVT100Config`]
/// from a [`VPSize`] or tuple.
///
/// [`OfsBuf`]: crate::tui::OfsBuf
/// [`OfsBufVT100`]: crate::core::ansi::OfsBufVT100
/// [`OfsBufVT100Config`]: crate::OfsBufVT100Config
/// [`VPSize`]: crate::VPSize
mod impl_ofs_buf_vt_100_config {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<VPSize> for OfsBufVT100Config {
        fn from(window_size: VPSize) -> OfsBufVT100Config {
            OfsBufVT100Config {
                window_size,
                storage_line_limit: StorageLineLimit::Unlimited,
            }
        }
    }

    impl From<(VPSize, StorageLineLimit)> for OfsBufVT100Config {
        fn from(
            (window_size, storage_line_limit): (VPSize, StorageLineLimit),
        ) -> OfsBufVT100Config {
            OfsBufVT100Config {
                window_size,
                storage_line_limit,
            }
        }
    }
}
