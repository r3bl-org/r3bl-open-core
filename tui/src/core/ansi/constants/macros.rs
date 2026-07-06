// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// This macro is used to define [`ANSI`] constants that are **composed** of parts ([Tier
/// 2]).
///
/// Under the hood, this relies on the external [`const_format`] crate (specifically
/// [`formatcp!`]) to concatenate string slices entirely at compile time, guaranteeing
/// zero runtime overhead.
///
/// Because it concatenates strings at compile time, it requires foundational building
/// blocks ([Tier 1]) to be defined plainly without any macros. This is why our core
/// sequence starters, like [`ESC_STR`], are written as raw, hardcoded string literals.
/// They act as the pure constants that this macro combines to generate more complex
/// sequences.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`const_format`]: const_format
/// [`const_format`]: https://docs.rs/const_format/latest/const_format/
/// [`ESC_STR`]: crate::core::ansi::constants::ESC_STR
/// [`formatcp!`]: const_format::formatcp
/// [Tier 1]: mod@crate::constants#tier-1---foundational-parts
/// [Tier 2]: mod@crate::constants#tier-2---composed-sequences
#[macro_export]
macro_rules! define_ansi_const {
    // === CSI (Control Sequence Introducer) ===
    // &str variant
    (@csi_str : $const_name:ident = [$val:literal] => $doc_title:literal : $doc_details:literal) => {
        #[doc = concat!($doc_title, ": ", $doc_details)]
        #[doc = ""]
        #[doc = concat!("Full sequence: `CSI ", $val, "`")]
        #[doc = ""]
        #[doc = "[`CSI`]: crate::CsiSequence"]
        pub const $const_name: &str = const_format::formatcp!(
            "{CSI_START}{val}",
            CSI_START = $crate::core::ansi::constants::CSI_START,
            val = $val
        );
    };

    // === ESC (Escape Sequence) ===
    // &str variant
    (@esc_str : $const_name:ident = [$val:literal] => $doc_title:literal : $doc_details:literal) => {
        #[doc = concat!($doc_title, ": ", $doc_details)]
        #[doc = ""]
        #[doc = concat!("Full sequence: `ESC ", $val, "`")]
        #[doc = ""]
        #[doc = "[`ESC`]: crate::EscSequence"]
        pub const $const_name: &str = const_format::formatcp!(
            "{ESC_STR}{val}",
            ESC_STR = $crate::core::ansi::constants::ESC_STR,
            val = $val
        );
    };

    // === SGR (Select Graphic Rendition) - Subset of CSI ===
    // &str variant
    (@sgr_str : $const_name:ident = [$val:literal] => $doc_title:literal : $doc_details:literal) => {
        #[doc = concat!($doc_title, ": ", $doc_details)]
        #[doc = ""]
        #[doc = concat!("Full sequence: `SGR ", $val, "` (CSI ", $val, ")")]
        #[doc = ""]
        #[doc = "[`CSI`]: crate::CsiSequence"]
        #[doc = "[`SGR`]: crate::SgrCode"]
        pub const $const_name: &str = const_format::formatcp!(
            "{CSI_START}{val}",
            CSI_START = $crate::core::ansi::constants::CSI_START,
            val = $val
        );
    };

    // === OSC (Operating System Command) ===
    // &str variant
    (@osc_str : $const_name:ident = [$val:literal] => $doc_title:literal : $doc_details:literal) => {
        #[doc = concat!($doc_title, ": ", $doc_details)]
        #[doc = ""]
        #[doc = concat!("Full sequence: `OSC ", $val, "`")]
        #[doc = ""]
        #[doc = "[`OSC`]: crate::osc_codes::OscSequence"]
        pub const $const_name: &str = const_format::formatcp!(
            "{OSC_START}{val}",
            OSC_START = $crate::core::osc::osc_codes::OSC_START,
            val = $val
        );
    };

    // === DSR (Device Status Report) - Subset of CSI ===
    // &str variant
    (@dsr_str : $const_name:ident = [$val:literal] => $doc_title:literal : $doc_details:literal) => {
        #[doc = concat!($doc_title, ": ", $doc_details)]
        #[doc = ""]
        #[doc = concat!("Full sequence: `DSR ", $val, "` (CSI ", $val, ")")]
        #[doc = ""]
        #[doc = "[`CSI`]: crate::CsiSequence"]
        #[doc = "[`DSR`]: crate::DsrSequence"]
        pub const $const_name: &str = const_format::formatcp!(
            "{CSI_START}{val}",
            CSI_START = $crate::core::ansi::constants::CSI_START,
            val = $val
        );
    };

    // === DA (Device Attributes) - Subset of CSI ===
    // &str variant
    (@da_str : $const_name:ident = [$val:literal] => $doc_title:literal : $doc_details:literal) => {
        #[doc = concat!($doc_title, ": ", $doc_details)]
        #[doc = ""]
        #[doc = concat!("Full sequence: `DA ", $val, "` (CSI ", $val, ")")]
        #[doc = ""]
        #[doc = "[`CSI`]: crate::CsiSequence"]
        #[doc = "[`DA`]: crate::DaSequence"]
        pub const $const_name: &str = const_format::formatcp!(
            "{CSI_START}{val}",
            CSI_START = $crate::core::ansi::constants::CSI_START,
            val = $val
        );
    };
}
