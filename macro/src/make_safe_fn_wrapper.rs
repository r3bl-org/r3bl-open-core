/*
 *   Copyright (c) 2022 R3BL LLC
 *   All rights reserved.

 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at

 *   http://www.apache.org/licenses/LICENSE-2.0

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

use crate::utils::{IdentExt, TypeExt};

pub fn fn_proc_macro_impl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
  let safe_wrapper_syntax_info: SafeFnWrapperSyntaxInfo = parse_macro_input!(input);

  let SafeFnWrapperSyntaxInfo {
    wrapper_name_ident,
    wrapper_name_type,
    wrapper_name_type_generic_args,
    property_name_ident,
    property_fn_type,
  } = safe_wrapper_syntax_info;

  let fn_input_arg_type_vec = get_fn_input_args_from(&property_fn_type);
  let (fn_input_arg_expr_vec, fn_input_arg_name_ident_vec) =
    gen_fn_input_args_expr_list(&fn_input_arg_type_vec);
  let fn_output_return_type = get_fn_output_type_from(&property_fn_type);

  let opt_generic_args = if wrapper_name_type_generic_args.is_some() {
    let args = wrapper_name_type_generic_args
      .as_ref()
      .unwrap();
    quote! { < #args > }
  } else {
    quote! {}
  };

  let opt_where_clause = if wrapper_name_type_generic_args.is_some() {
    let where_clause = wrapper_name_type_generic_args
      .as_ref()
      .unwrap();
    quote! { where #where_clause: Sync + Send + 'static }
  } else {
    quote! {}
  };

  quote! {
    // Type aliases to make the code more readable.
    type ARC<T> = std::sync::Arc<T>;
    type RWLOCK<T> = tokio::sync::RwLock<T>;
    type WRITE_G<'a, T> = tokio::sync::RwLockWriteGuard<'a, T>;
    type READ_G<'a, T> = tokio::sync::RwLockReadGuard<'a, T>;
    type J_HANDLE<T> = tokio::task::JoinHandle<T>;

    pub struct #wrapper_name_type {
      pub #property_name_ident: ARC<RWLOCK<dyn #property_fn_type + Send + Sync + 'static>>
    }

    impl #opt_generic_args #wrapper_name_type
    #opt_where_clause
    {
      pub fn from(
        #property_name_ident: impl #property_fn_type + Send + Sync + 'static
      ) -> Self {
        Self { #property_name_ident: ARC::new(RWLOCK::new(#property_name_ident)) }
      }

      pub fn get(&self) -> ARC<RWLOCK<dyn #property_fn_type + Send + Sync + 'static>> {
        self.#property_name_ident.clone()
      }

      pub fn spawn(&self, #(#fn_input_arg_expr_vec),*) -> J_HANDLE<#fn_output_return_type> {
        let arc_lock_fn_mut = self.get();
        tokio::spawn(async move {
          let mut fn_mut = arc_lock_fn_mut.write().await;
          fn_mut(#(#fn_input_arg_name_ident_vec),*)
        })
      }
    }
  }
  .into()
}

fn gen_fn_input_args_expr_list(
  fn_arg_type_list: &Vec<Ident>
) -> (
  Vec<proc_macro2::TokenStream>,
  Vec<Ident>,
) {
  let mut count = 0;
  let mut arg_name_ident_vec: Vec<Ident> = Vec::new();
  let arg_with_type_vec: Vec<proc_macro2::TokenStream> = fn_arg_type_list
    .iter()
    .map(|arg_ty_ident| {
      count = count + 1;
      let arg_name_ident: Ident = arg_ty_ident.from_string(&format!("arg{}", count));
      arg_name_ident_vec.push(arg_name_ident.clone());
      quote! { #arg_name_ident: #arg_ty_ident, }
    })
    .collect::<Vec<proc_macro2::TokenStream>>();
  (
    arg_with_type_vec,
    arg_name_ident_vec,
  )
}

fn get_fn_input_args_from(property_fn_type: &Type) -> Vec<Ident> {
  let mut args: Vec<Ident> = Vec::new();
  if let Type::Path(type_path) = property_fn_type {
    if type_path
      .path
      .segments
      .first()
      .is_some()
    {
      let path_segment = type_path
        .path
        .segments
        .first()
        .unwrap();
      let path_arguments = &path_segment.arguments;
      if let PathArguments::Parenthesized(p_g_args) = path_arguments {
        let inputs = &p_g_args.inputs;
        inputs.iter().for_each(|ty| {
          if ty.has_ident() {
            let ident = ty.get_ident().unwrap();
            args.push(ident.clone());
          }
        })
      }
    }
  }
  args
}

fn get_fn_output_type_from(property_fn_type: &Type) -> Option<proc_macro2::TokenStream> {
  if let Type::Path(type_path) = property_fn_type {
    if type_path
      .path
      .segments
      .first()
      .is_some()
    {
      let path_segment = type_path
        .path
        .segments
        .first()
        .unwrap();
      let path_arguments = &path_segment.arguments;
      if let PathArguments::Parenthesized(p_g_args) = path_arguments {
        let output = &p_g_args.output;
        if let syn::ReturnType::Type(_, return_ty) = output {
          return Some(return_ty.to_token_stream());
        }
      }
    }
  }
  None
}

/// Example of syntax to parse:
/// ```no_run
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
struct SafeFnWrapperSyntaxInfo {
  wrapper_name_ident: Ident,
  wrapper_name_type: Type,
  wrapper_name_type_generic_args: Option<Punctuated<GenericArgument, Comma>>,
  property_name_ident: Ident,
  property_fn_type: Type,
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
    let wrapper_name_type_generic_args =
      match wrapper_name_type.has_angle_bracketed_generic_args() {
        true => Some(
          wrapper_name_type
            .get_angle_bracketed_generic_args_result()
            .unwrap(),
        ),
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
      wrapper_name_type
        .get_ident()
        .unwrap()
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
