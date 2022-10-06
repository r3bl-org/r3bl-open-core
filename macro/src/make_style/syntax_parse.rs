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

use quote::*;
use r3bl_rs_utils_core::*;
use syn::{parse::*, Expr::Verbatim, *};

use super::*;
use crate::utils::IdentExt;

/// Here's a sample syntax to parse.
///
/// ```ignore
/// style! {
///   id: "my_style",          /* Optional. */
///   attrib: [dim, bold]      /* Optional. */
///   padding: 10,             /* Optional. */
///   color_fg: TWColor::Blue, /* Optional. */
///   color_bg: TWColor::Red,  /* Optional. */
/// }
/// ```
///
/// `color_fg` and `color_bg` can take any of the following:
/// 1. Color enum value.
/// 2. Rgb value.
/// 3. Variable holding either of the above.
impl Parse for StyleMetadata {
  fn parse(input: ParseStream) -> Result<Self> {
    let mut metadata = StyleMetadata {
      id: Verbatim(quote! { "_id" }),
      attrib_vec: Vec::new(),
      padding: None,
      color_fg: None,
      color_bg: None,
    };

    // Run them all.
    parse_optional_id(&input, &mut metadata)?;
    parse_optional_attrib(&input, &mut metadata)?;
    parse_optional_padding(&input, &mut metadata)?;
    parse_optional_color_fg(&input, &mut metadata)?;
    parse_optional_color_bg(&input, &mut metadata)?;

    Ok(metadata)
  }
}

/// [syn custom keywords docs](https://docs.rs/syn/latest/syn/macro.custom_keyword.html)
pub(crate) mod kw {
  syn::custom_keyword!(id);
  syn::custom_keyword!(bold);
  syn::custom_keyword!(attrib);
  syn::custom_keyword!(dim);
  syn::custom_keyword!(underline);
  syn::custom_keyword!(reverse);
  syn::custom_keyword!(hidden);
  syn::custom_keyword!(strikethrough);
  syn::custom_keyword!(padding);
  syn::custom_keyword!(color_fg);
  syn::custom_keyword!(color_bg);
}

// Parse id (optional).
fn parse_optional_id(input: &ParseStream, metadata: &mut StyleMetadata) -> Result<()> {
  let lookahead = input.lookahead1();
  if lookahead.peek(kw::id) {
    input.parse::<kw::id>()?;
    input.parse::<Token![:]>()?;
    let id = input.parse::<Expr>()?;
    metadata.id = id;
  }
  call_if_true!(DEBUG_MAKE_STYLE_MOD, println!("🚀 id: {:?}", metadata.id));
  Ok(())
}

// Parse attrib (optional).
fn parse_optional_attrib(input: &ParseStream, metadata: &mut StyleMetadata) -> Result<()> {
  let lookahead = input.lookahead1();
  if lookahead.peek(kw::attrib) {
    input.parse::<kw::attrib>()?;
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
          "dim" => metadata.attrib_vec.push(Attrib::Dim),
          "underline" => metadata.attrib_vec.push(Attrib::Underline),
          "reverse" => metadata.attrib_vec.push(Attrib::Reverse),
          "hidden" => metadata.attrib_vec.push(Attrib::Hidden),
          "strikethrough" => metadata.attrib_vec.push(Attrib::Strikethrough),
          _ => panic!("🚀 unknown attrib: {}", ident),
        }
      }
    }

    call_if_true!(
      DEBUG_MAKE_STYLE_MOD,
      println!("🚀 attrib_vec: {:?}", metadata.attrib_vec)
    );
  }
  Ok(())
}

// Parse padding (optional).
fn parse_optional_padding(input: &ParseStream, metadata: &mut StyleMetadata) -> Result<()> {
  let lookahead = input.lookahead1();
  if lookahead.peek(kw::padding) {
    input.parse::<kw::padding>()?;
    input.parse::<Token![:]>()?;

    let lit_int = input.parse::<LitInt>()?;
    let val: ChUnitPrimitiveType = lit_int.base10_parse().unwrap();
    let padding_int: ChUnit = ch!(val);

    metadata.padding = Some(padding_int);

    call_if_true!(
      DEBUG_MAKE_STYLE_MOD,
      println!("🚀 padding: {:?}", &metadata.padding)
    );
  }
  Ok(())
}

// Parse color_fg (optional).
fn parse_optional_color_fg(input: &ParseStream, metadata: &mut StyleMetadata) -> Result<()> {
  let lookahead = input.lookahead1();
  if lookahead.peek(kw::color_fg) {
    input.parse::<kw::color_fg>()?;
    input.parse::<Token![:]>()?;
    let color_expr = input.parse::<Expr>()?;
    metadata.color_fg = Some(color_expr);
    call_if_true!(
      DEBUG_MAKE_STYLE_MOD,
      println!("🚀 color_fg: {:#?}", metadata.color_fg)
    );
  }

  Ok(())
}

// Parse color_bg (optional).
fn parse_optional_color_bg(input: &ParseStream, metadata: &mut StyleMetadata) -> Result<()> {
  let lookahead = input.lookahead1();
  if lookahead.peek(kw::color_bg) {
    input.parse::<kw::color_bg>()?;
    input.parse::<Token![:]>()?;
    let color_expr = input.parse::<Expr>()?;
    metadata.color_bg = Some(color_expr);
    call_if_true!(
      DEBUG_MAKE_STYLE_MOD,
      println!("🚀 color_bg: {:#?}", metadata.color_bg)
    );
  }

  Ok(())
}
