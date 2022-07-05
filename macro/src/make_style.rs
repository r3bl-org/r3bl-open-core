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

use proc_macro2::Literal;
use quote::quote;
use r3bl_rs_utils_core::*;
use syn::{parse::{Parse, ParseStream},
          parse_macro_input,
          Result,
          Token};

#[derive(Debug, Clone, PartialEq, Eq)]
enum StyleAttribute {
  Bold,
  Dim,
  Underline,
  Reverse,
  Hidden,
  Strikethrough,
}

/// Docs: https://docs.rs/syn/1.0.98/syn/parse/struct.ParseBuffer.html
#[derive(Debug, Clone)]
struct StyleMetadata {
  id: Literal,                     /* Only required field. */
  attrib_vec: Vec<StyleAttribute>, /* Attributes are optional. */
  margin: Option<UnitType>,        /* Optional. */
  color_fg: Option<TWColor>,       /* Optional. */
  color_bg: Option<TWColor>,       /* Optional. */
}

/// [syn custom keywords docs](https://docs.rs/syn/latest/syn/macro.custom_keyword.html)
mod kw {
  syn::custom_keyword!(id);
  syn::custom_keyword!(bold);
  syn::custom_keyword!(attrib);
  syn::custom_keyword!(dim);
  syn::custom_keyword!(underline);
  syn::custom_keyword!(reverse);
  syn::custom_keyword!(hidden);
  syn::custom_keyword!(strikethrough);
  syn::custom_keyword!(margin);
  syn::custom_keyword!(color_fg);
  syn::custom_keyword!(color_bg);
}

/// Here's a sample syntax to parse.
///
/// ```
/// style! {
///   id: my_style,          /* Required. */
///   attrib: bold, dim,     /* Optional. */
///   margin: 10,            /* Optional. */
///   color_fg: Color::Blue, /* Optional. */
///   color_bg: Color::Red,  /* Optional. */
/// }
/// ```
impl Parse for StyleMetadata {
  fn parse(input: ParseStream) -> Result<Self> {
    // TODO: parse the tokens & make a struct
    // Sample code:
    // let lookahead = input.lookahead1();
    // if lookahead.peek(Ident) {
    //     input.parse().map(GenericParam::Type)
    // } else if lookahead.peek(Lifetime) {
    //     input.parse().map(GenericParam::Lifetime)
    // } else if lookahead.peek(Token![const]) {
    //     input.parse().map(GenericParam::Const)
    // } else {
    //     Err(lookahead.error())
    // }

    let mut metadata = StyleMetadata {
      id: Literal::string(""),
      attrib_vec: Vec::new(),
      margin: None,
      color_fg: None,
      color_bg: None,
    };

    // Parse id (required).
    let lookahead = input.lookahead1();
    if lookahead.peek(kw::id) {
      input.parse::<kw::id>()?;
      input.parse::<Token![:]>()?;
      let id = input.parse::<Literal>()?;
      metadata.id = id;
    }

    // Parse attrib (optional).
    let lookahead = input.lookahead1();
    if lookahead.peek(kw::attrib) {
      input.parse::<kw::attrib>()?;
      input.parse::<Token![:]>()?;

      let punct_attrs: syn::punctuated::Punctuated<StyleAttribute, Token![,]> = input.parse_terminated(|input| {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::bold) {
          input.parse::<kw::bold>()?;
          Ok(StyleAttribute::Bold)
        } else if lookahead.peek(kw::dim) {
          input.parse::<kw::dim>()?;
          Ok(StyleAttribute::Dim)
        } else if lookahead.peek(kw::underline) {
          input.parse::<kw::underline>()?;
          Ok(StyleAttribute::Underline)
        } else if lookahead.peek(kw::reverse) {
          input.parse::<kw::reverse>()?;
          Ok(StyleAttribute::Reverse)
        } else if lookahead.peek(kw::hidden) {
          input.parse::<kw::hidden>()?;
          Ok(StyleAttribute::Hidden)
        } else if lookahead.peek(kw::strikethrough) {
          input.parse::<kw::strikethrough>()?;
          Ok(StyleAttribute::Strikethrough)
        } else {
          Err(lookahead.error())
        }
      })?;

      punct_attrs.iter().for_each(|attrib| {
        metadata.attrib_vec.push(attrib.clone());
      });

      println!("🚀 punct_attrs: {:?}", punct_attrs);
    }

    Ok(metadata)
  }
}

pub fn fn_proc_macro_impl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
  let style_metadata: StyleMetadata = parse_macro_input!(input);
  let StyleMetadata {
    id,
    attrib_vec,
    margin,
    color_fg,
    color_bg,
  } = style_metadata;

  println!("🚀 attrib_vec: {:?}", attrib_vec);

  let has_attrib_bold = attrib_vec.contains(&StyleAttribute::Bold);
  let has_attrib_dim = attrib_vec.contains(&StyleAttribute::Dim);
  let has_attrib_underline = attrib_vec.contains(&StyleAttribute::Underline);
  let has_attrib_reverse = attrib_vec.contains(&StyleAttribute::Reverse);
  let has_attrib_hidden = attrib_vec.contains(&StyleAttribute::Hidden);
  let has_attrib_strikethrough = attrib_vec.contains(&StyleAttribute::Strikethrough);

  // TODO: gen the source using style_info
  quote! {
    Style {
      id: #id,
      bold: #has_attrib_bold,
      dim: #has_attrib_dim,
      underline: #has_attrib_underline,
      reverse: #has_attrib_reverse,
      hidden: #has_attrib_hidden,
      strikethrough: #has_attrib_strikethrough,
    }
  }
  .into()
}
