/*
 *   Copyright (c) 2022 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

//! Here's a sample syntax to parse.
//!
//! ```no_run
//! use r3bl_macro::tui_style;
//! use r3bl_core::{ch, TuiColor, ANSIBasicColor};
//! let black = TuiColor::Basic(ANSIBasicColor::Black);
//! let white = TuiColor::Basic(ANSIBasicColor::White);
//! tui_style! {
//!     id: 12              /* Optional. */
//!     attrib: [dim, bold] /* Optional. */
//!     padding: 10         /* Optional. */
//!     color_fg: black     /* Optional. */
//!     color_bg: white     /* Optional. */
//!     lolcat: true        /* Optional. */
//! };
//! ```
//!
//! `color_fg` and `color_bg` can take any [r3bl_core::TuiColor]:
//! 1. Color enum value.
//! 2. Rgb value.
//! 3. Variable holding either of the above.

use quote::quote;
use r3bl_core::{call_if_true, ch, throws, ChUnit, ChUnitPrimitiveType};
use syn::{parse::{Parse, ParseStream},
          Expr,
          Expr::Verbatim,
          ExprArray,
          ExprPath,
          LitBool,
          LitInt,
          Path,
          PathSegment,
          Token};

use super::{Attrib, StyleMetadata, DEBUG_MAKE_STYLE_MOD};
use crate::utils::IdentExt;

/// Type alias for [syn::Result].
type SynResult<T> = std::result::Result<T, syn::Error>;

impl Parse for StyleMetadata {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut metadata = StyleMetadata {
            id: Verbatim(quote! { u8::MAX }),
            attrib_vec: vec![],
            padding: None,
            color_fg: None,
            color_bg: None,
            lolcat: None,
        };

        // Run them all.
        parse_optional_id(&input, &mut metadata)?;
        parse_optional_attrib(&input, &mut metadata)?;
        parse_optional_padding(&input, &mut metadata)?;
        parse_optional_color_fg(&input, &mut metadata)?;
        parse_optional_color_bg(&input, &mut metadata)?;
        parse_optional_lolcat(&input, &mut metadata)?;

        Ok(metadata)
    }
}

/// [syn custom keywords docs](https://docs.rs/syn/latest/syn/macro.custom_keyword.html)
pub(crate) mod custom_keywords {
    syn::custom_keyword!(id);
    syn::custom_keyword!(bold);
    syn::custom_keyword!(italic);
    syn::custom_keyword!(attrib);
    syn::custom_keyword!(dim);
    syn::custom_keyword!(underline);
    syn::custom_keyword!(reverse);
    syn::custom_keyword!(hidden);
    syn::custom_keyword!(strikethrough);
    syn::custom_keyword!(padding);
    syn::custom_keyword!(color_fg);
    syn::custom_keyword!(color_bg);
    syn::custom_keyword!(lolcat);
}

// Parse id (optional).
fn parse_optional_id(input: &ParseStream, metadata: &mut StyleMetadata) -> SynResult<()> {
    throws!({
        let lookahead = input.lookahead1();

        if lookahead.peek(custom_keywords::id) {
            input.parse::<custom_keywords::id>()?;
            input.parse::<Token![:]>()?;
            let id = input.parse::<Expr>()?;
            metadata.id = id;
        }

        call_if_true!(DEBUG_MAKE_STYLE_MOD, println!("🚀 id: {:?}", metadata.id));
    });
}

// Parse lolcat (optional).
fn parse_optional_lolcat(
    input: &ParseStream,
    metadata: &mut StyleMetadata,
) -> SynResult<()> {
    throws!({
        let lookahead = input.lookahead1();

        if lookahead.peek(custom_keywords::lolcat) {
            input.parse::<custom_keywords::lolcat>()?;
            input.parse::<Token![:]>()?;
            let lolcat = input.parse::<LitBool>()?;
            metadata.lolcat = Some(lolcat);
        }

        call_if_true!(
            DEBUG_MAKE_STYLE_MOD,
            println!("🚀 lolcat: {:?}", metadata.lolcat)
        );
    });
}

// Parse attrib (optional).
fn parse_optional_attrib(
    input: &ParseStream,
    metadata: &mut StyleMetadata,
) -> SynResult<()> {
    throws!({
        let lookahead = input.lookahead1();
        if lookahead.peek(custom_keywords::attrib) {
            input.parse::<custom_keywords::attrib>()?;
            input.parse::<Token![:]>()?;

            let expr_array: ExprArray = input.parse()?;
            for item in expr_array.elems {
                if let Expr::Path(ExprPath {
                    attrs: _,
                    qself: _,
                    path: Path { segments, .. },
                }) = item
                {
                    let PathSegment {
                        ident,
                        arguments: _,
                    } = segments.first().unwrap();
                    match ident.as_str().as_ref() {
                        "bold" => metadata.attrib_vec.push(Attrib::Bold),
                        "italic" => metadata.attrib_vec.push(Attrib::Italic),
                        "dim" => metadata.attrib_vec.push(Attrib::Dim),
                        "underline" => metadata.attrib_vec.push(Attrib::Underline),
                        "reverse" => metadata.attrib_vec.push(Attrib::Reverse),
                        "hidden" => metadata.attrib_vec.push(Attrib::Hidden),
                        "strikethrough" => {
                            metadata.attrib_vec.push(Attrib::Strikethrough)
                        }
                        _ => panic!("🚀 unknown attrib: {ident}"),
                    }
                }
            }

            call_if_true!(
                DEBUG_MAKE_STYLE_MOD,
                println!("🚀 attrib_vec: {:?}", metadata.attrib_vec)
            );
        }
    });
}

// Parse padding (optional).
fn parse_optional_padding(
    input: &ParseStream,
    metadata: &mut StyleMetadata,
) -> SynResult<()> {
    throws!({
        let lookahead = input.lookahead1();

        if lookahead.peek(custom_keywords::padding) {
            input.parse::<custom_keywords::padding>()?;
            input.parse::<Token![:]>()?;

            let lit_int = input.parse::<LitInt>()?;
            let val: ChUnitPrimitiveType = lit_int.base10_parse().unwrap();
            let padding_int: ChUnit = ch(val);

            metadata.padding = Some(padding_int);

            call_if_true!(
                DEBUG_MAKE_STYLE_MOD,
                println!("🚀 padding: {:?}", &metadata.padding)
            );
        }
    });
}

// Parse color_fg (optional).
fn parse_optional_color_fg(
    input: &ParseStream,
    metadata: &mut StyleMetadata,
) -> SynResult<()> {
    throws!({
        let lookahead = input.lookahead1();

        if lookahead.peek(custom_keywords::color_fg) {
            input.parse::<custom_keywords::color_fg>()?;
            input.parse::<Token![:]>()?;
            let color_expr = input.parse::<Expr>()?;
            metadata.color_fg = Some(color_expr);
            call_if_true!(
                DEBUG_MAKE_STYLE_MOD,
                println!("🚀 color_fg: {:#?}", metadata.color_fg)
            );
        }
    });
}

// Parse color_bg (optional).
fn parse_optional_color_bg(
    input: &ParseStream,
    metadata: &mut StyleMetadata,
) -> SynResult<()> {
    throws!({
        let lookahead = input.lookahead1();

        if lookahead.peek(custom_keywords::color_bg) {
            input.parse::<custom_keywords::color_bg>()?;
            input.parse::<Token![:]>()?;
            let color_expr = input.parse::<Expr>()?;
            metadata.color_bg = Some(color_expr);
            call_if_true!(
                DEBUG_MAKE_STYLE_MOD,
                println!("🚀 color_bg: {:#?}", metadata.color_bg)
            );
        }
    });
}
