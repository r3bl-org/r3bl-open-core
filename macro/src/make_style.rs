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

use proc_macro2::*;
use quote::quote;
use r3bl_rs_utils_core::*;
use syn::{parse::{Parse, ParseStream},
          parse_macro_input,
          *};

use crate::utils::IdentExt;

const DEBUG: bool = false;

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
  id: Ident,                       /* Only required field. */
  attrib_vec: Vec<StyleAttribute>, /* Attributes are optional. */
  margin: Option<UnitType>,        /* Optional. */
  color_fg: Option<Expr>,          /* Optional. */
  color_bg: Option<Expr>,          /* Optional. */
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
    let mut metadata = StyleMetadata {
      id: Ident::new("tbd", Span::call_site()),
      attrib_vec: Vec::new(),
      margin: None,
      color_fg: None,
      color_bg: None,
    };

    // Parse id (required).
    {
      let lookahead = input.lookahead1();
      if lookahead.peek(kw::id) {
        input.parse::<kw::id>()?;
        input.parse::<Token![:]>()?;
        let id = input.parse::<Ident>()?;
        metadata.id = id;
      }
      call_if_true!(DEBUG, println!("🚀 id: {:?}", metadata.id));
    }

    // Parse attrib (optional).
    {
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
            let PathSegment { ident, arguments: _ } = segments.first().unwrap();
            match ident.as_str().as_ref() {
              "bold" => metadata.attrib_vec.push(StyleAttribute::Bold),
              "dim" => metadata.attrib_vec.push(StyleAttribute::Dim),
              "underline" => metadata.attrib_vec.push(StyleAttribute::Underline),
              "reverse" => metadata.attrib_vec.push(StyleAttribute::Reverse),
              "hidden" => metadata.attrib_vec.push(StyleAttribute::Hidden),
              "strikethrough" => metadata.attrib_vec.push(StyleAttribute::Strikethrough),
              _ => panic!("🚀 unknown attrib: {}", ident),
            }
          }
        }

        call_if_true!(DEBUG, println!("🚀 attrib_vec: {:?}", metadata.attrib_vec));
      }
    }

    // Parse margin (optional).
    {
      let lookahead = input.lookahead1();
      if lookahead.peek(kw::margin) {
        input.parse::<kw::margin>()?;
        input.parse::<Token![:]>()?;
        let lit_int = input.parse::<LitInt>()?;
        let margin_int: UnitType = lit_int.base10_parse().unwrap();
        metadata.margin = Some(margin_int);
        call_if_true!(DEBUG, println!("🚀 margin: {:?}", &metadata.margin));
      }
    }

    // Parse color_fg (optional).
    {
      let lookahead = input.lookahead1();
      if lookahead.peek(kw::color_fg) {
        input.parse::<kw::color_fg>()?;
        input.parse::<Token![:]>()?;
        let color_expr = input.parse::<Expr>()?;
        metadata.color_fg = Some(color_expr);
        call_if_true!(DEBUG, println!("🚀 color_fg: {:#?}", metadata.color_fg));
      }
    }

    // Parse color_bg (optional).
    {
      let lookahead = input.lookahead1();
      if lookahead.peek(kw::color_bg) {
        input.parse::<kw::color_bg>()?;
        input.parse::<Token![:]>()?;
        let color_expr = input.parse::<Expr>()?;
        metadata.color_bg = Some(color_expr);
        call_if_true!(DEBUG, println!("🚀 color_bg: {:#?}", metadata.color_bg));
      }
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

  let has_attrib_bold = attrib_vec.contains(&StyleAttribute::Bold);
  let has_attrib_dim = attrib_vec.contains(&StyleAttribute::Dim);
  let has_attrib_underline = attrib_vec.contains(&StyleAttribute::Underline);
  let has_attrib_reverse = attrib_vec.contains(&StyleAttribute::Reverse);
  let has_attrib_hidden = attrib_vec.contains(&StyleAttribute::Hidden);
  let has_attrib_strikethrough = attrib_vec.contains(&StyleAttribute::Strikethrough);

  let id_str = format!("{}", id);

  let maybe_margin_expr = match margin {
    Some(margin_int) => {
      quote! {
        margin: Some(#margin_int),
      }
    }
    None => quote! {},
  };

  let maybe_color_fg_expr = match color_fg {
    Some(color_expr) => {
      quote! {
        color_fg: Some(crossterm::style::#color_expr.into()),
      }
    }
    None => quote! {},
  };

  let maybe_color_bg_expr = match color_bg {
    Some(color_expr) => {
      quote! {
        color_bg: Some(crossterm::style::#color_expr.into()),
      }
    }
    None => quote! {},
  };

  quote! {
    r3bl_rs_utils::Style {
      id: #id_str.to_string(),
      bold: #has_attrib_bold,
      dim: #has_attrib_dim,
      underline: #has_attrib_underline,
      reverse: #has_attrib_reverse,
      hidden: #has_attrib_hidden,
      strikethrough: #has_attrib_strikethrough,
      #maybe_margin_expr
      #maybe_color_fg_expr
      #maybe_color_bg_expr
      .. Default::default()
    }
  }
  .into()
}
