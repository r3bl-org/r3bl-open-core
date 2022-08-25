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

use super::*;

pub(crate) fn code_gen(
  StyleMetadata {
    id,
    attrib_vec,
    padding,
    color_fg,
    color_bg,
  }: StyleMetadata,
) -> proc_macro::TokenStream {
  let has_attrib_bold = attrib_vec.contains(&Attrib::Bold);
  let has_attrib_dim = attrib_vec.contains(&Attrib::Dim);
  let has_attrib_underline = attrib_vec.contains(&Attrib::Underline);
  let has_attrib_reverse = attrib_vec.contains(&Attrib::Reverse);
  let has_attrib_hidden = attrib_vec.contains(&Attrib::Hidden);
  let has_attrib_strikethrough = attrib_vec.contains(&Attrib::Strikethrough);

  let maybe_padding_expr = match padding {
    Some(padding_int) => {
      quote! {
        padding: Some(#padding_int),
      }
    }
    None => quote! {},
  };

  let maybe_color_fg_expr = match color_fg {
    Some(color_expr) => {
      quote! {
        color_fg: Some(#color_expr.into()),
      }
    }
    None => quote! {},
  };

  let maybe_color_bg_expr = match color_bg {
    Some(color_expr) => {
      quote! {
        color_bg: Some(#color_expr.into()),
      }
    }
    None => quote! {},
  };

  quote! {
    r3bl_rs_utils::Style {
      id: #id.to_string(),
      bold: #has_attrib_bold,
      dim: #has_attrib_dim,
      underline: #has_attrib_underline,
      reverse: #has_attrib_reverse,
      hidden: #has_attrib_hidden,
      strikethrough: #has_attrib_strikethrough,
      #maybe_padding_expr
      #maybe_color_fg_expr
      #maybe_color_bg_expr
      .. Default::default()
    }
  }
  .into()
}
