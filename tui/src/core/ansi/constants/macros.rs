// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// This macro is used to define [`ANSI`] constants that are **composed** of parts
/// ([Tier 2]). For foundational bytes or characters ([Tier 1]), use manual `pub const`
/// definitions instead.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
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
        pub const $const_name: &str = concat!("\x1b[", $val);
    };

    // === ESC (Escape Sequence) ===
    // &str variant
    (@esc_str : $const_name:ident = [$val:literal] => $doc_title:literal : $doc_details:literal) => {
        #[doc = concat!($doc_title, ": ", $doc_details)]
        #[doc = ""]
        #[doc = concat!("Full sequence: `ESC ", $val, "`")]
        #[doc = ""]
        #[doc = "[`ESC`]: crate::EscSequence"]
        pub const $const_name: &str = concat!("\x1b", $val);
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
        pub const $const_name: &str = concat!("\x1b[", $val);
    };

    // === OSC (Operating System Command) ===
    // &str variant
    (@osc_str : $const_name:ident = [$val:literal] => $doc_title:literal : $doc_details:literal) => {
        #[doc = concat!($doc_title, ": ", $doc_details)]
        #[doc = ""]
        #[doc = concat!("Full sequence: `OSC ", $val, "`")]
        #[doc = ""]
        #[doc = "[`OSC`]: crate::osc_codes::OscSequence"]
        pub const $const_name: &str = concat!("\x1b]", $val);
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
        pub const $const_name: &str = concat!("\x1b[", $val);
    };
}
