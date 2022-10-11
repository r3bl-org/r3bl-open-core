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

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_macros)]
#![allow(non_camel_case_types)]

use core::panic;

use quote::{quote, ToTokens};
use r3bl_rs_utils_core::debug;
use syn::{parse::{Parse, ParseBuffer, ParseStream},
          parse2,
          parse_macro_input,
          punctuated::Punctuated,
          token::Comma,
          Expr,
          GenericArgument,
          GenericParam,
          Generics,
          Ident,
          PathArguments,
          Result,
          Token,
          Type,
          TypePath,
          Visibility,
          WhereClause};

use crate::utils::{IdentExt, TypeExtHasGenericArgs, TypeExtHasIdent};

/// Example of syntax to parse:
/// ```ignore
/// make_safe_fn_wrapper! {
///   ╭─L1──────────────────────────────────────────
///   │     wrapper_name_type
///   │     ▾▾▾▾▾▾▾▾▾▾▾▾▾▾▾▾▾
///   named FnWrapper<K, V>
///   │     ▴▴▴▴▴▴▴▴▴ ▴▴▴▴
///   │     │         wrapper_name_type_generic_args
///   │     wrapper_name_ident
///   ╰─────────────────────────────────────────────
///   ╭─L2──────────────────────────────────────────
///   containing fn_mut
///   │          ▴▴▴▴▴▴
///   │          property_name_ident
///   ╰─────────────────────────────────────────────
///   ╭─L3──────────────────────────────────────────
///   of_type FnMut(A) -> Option<A>
///   │       ▴▴▴▴▴▴▴▴▴▴▴▴▴▴▴▴▴▴▴▴▴
///   │       property_fn_type
///   ╰─────────────────────────────────────────────
/// }
/// ```
#[derive(Debug)]
pub struct SafeFnWrapperSyntaxInfo {
  pub wrapper_name_ident: Ident,
  pub wrapper_name_type: Type,
  pub wrapper_name_type_generic_args: Option<Punctuated<GenericArgument, Comma>>,
  pub property_name_ident: Ident,
  pub property_fn_type: Type,
}

/// [syn custom keywords docs](https://docs.rs/syn/latest/syn/macro.custom_keyword.html)
mod kw {
  syn::custom_keyword!(named);
  syn::custom_keyword!(containing);
  syn::custom_keyword!(of_type);
}

/// [Parse docs](https://docs.rs/syn/latest/syn/parse/index.html)
impl Parse for SafeFnWrapperSyntaxInfo {
  fn parse(input: ParseStream) -> Result<Self> {
    // 👀 "named" keyword.
    input.parse::<kw::named>()?;

    // 👀 Wrapper Name Type, eg: `FnWrapper<K,V>`.
    let wrapper_name_type: Type = input.parse()?;

    // 👀 Wrapper Name Type generic args, eg: `<K,V>`.
    let wrapper_name_type_generic_args = match wrapper_name_type.has_angle_bracketed_generic_args() {
      true => Some(wrapper_name_type.get_angle_bracketed_generic_args_result().unwrap()),
      false => None,
    };

    // 👀 "containing" keyword.
    input.parse::<kw::containing>()?;

    // 👀 use Ident, eg: `fn_mut`.
    let property_name_ident: Ident = input.parse()?;

    // 👀 "of_type" keyword.
    input.parse::<kw::of_type>()?;

    // 👀 Fn Type, eg: `FnMut(A) -> Option(A) + Sync + Send + 'static`.
    let property_fn_type: Type = input.parse()?;

    // Done parsing. Extract the manager name.
    let wrapper_name_ident: Ident = if wrapper_name_type.has_ident() {
      wrapper_name_type.get_ident().unwrap()
    } else {
      panic!("Expected Type::Path::TypePath.segments to have an Ident")
    };

    Ok(SafeFnWrapperSyntaxInfo {
      wrapper_name_ident,
      wrapper_name_type,
      wrapper_name_type_generic_args,
      property_name_ident,
      property_fn_type,
    })
  }
}

/// Given optional `GenericArgument`s that are separated by `Comma`s, generate
/// an optional where clause.
/// - Eg of input: `A, B`
/// - Eg of return: `where A : Sync + Send + 'static, B : Sync + Send + 'static`
pub fn make_opt_where_clause_from_generic_args(
  wrapper_name_type_generic_args: Option<Punctuated<GenericArgument, Comma>>,
) -> proc_macro2::TokenStream {
  if wrapper_name_type_generic_args.is_some() {
    let generic_args_list = wrapper_name_type_generic_args.as_ref().unwrap();

    let generic_args_ident_vec: Vec<Ident> = generic_args_list
      .iter()
      .map(|it: &GenericArgument| match it {
        GenericArgument::Type(Type::Path(it)) => {
          if !it.path.segments.is_empty() {
            it.path.segments.first().unwrap().ident.clone()
          } else {
            panic!("Expected generic arg type")
          }
        }
        _ => panic!("Expected generic arg type"),
      })
      .collect();

    let ts_vec: Vec<proc_macro2::TokenStream> = generic_args_ident_vec
      .iter()
      .map(|it| {
        quote! { #it: Sync + Send + 'static }
      })
      .collect();

    quote! { where #( #ts_vec ),* }
  } else {
    quote! {}
  }
}
