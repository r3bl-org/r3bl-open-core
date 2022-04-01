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

use core::panic;

use quote::{quote, ToTokens};
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
          Result,
          Token,
          Type,
          Visibility,
          WhereClause};

use crate::utils::type_ext::TypeExt;

/// See [`ManagerOfThingInfo`] for more information on the syntax that this macro accepts.
///
/// For reference, here's an example from syn called
/// [`lazy-static`](https://github.com/dtolnay/syn/blob/master/examples/lazy-static/lazy-static/src/lib.rs)
pub fn fn_proc_macro_impl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
  let manager_of_thing_info: ManagerOfThingInfo = parse_macro_input!(input);

  let ManagerOfThingInfo {
    manager_name_ident,
    manager_ty,
    thing_ty,
    manager_ty_generic_args,
    where_clause,
    property_name_ident,
  } = manager_of_thing_info;

  let doc_str_struct = format!(
    " Generated {} struct for {}.",
    &manager_name_ident,
    &thing_ty.to_string()
  );

  let doc_str_default_impl_for_struct = format!(
    " Generated Default trait impl for {}.",
    &manager_name_ident,
  );

  let doc_str_impl_for_struct = format!(
    " Generated impl for {}.",
    &manager_name_ident,
  );

  let doc_str_setter_fn = " Directly mutate the property.";
  let doc_str_getter_fn = " Get a clone of the arc. This can be passed around safely, \
                           instead of passing the manager instance itself.";
  let doc_str_static_lock_w = " 🔒 Static method that allow you to indirectly access \
                               the property via `Arc` produced by `get_arc()`.";
  let doc_str_static_lock_r = " 🔒 Static method that allow you to indirectly access \
                               the property via `Arc` produced by `get_arc()`.";
  let doc_str_static_with_arc_setter_fn = " Static method that allow you to indirectly \
                                           mutate the property via `Arc` produced by \
                                           `get_arc()`.";

  let opt_generic_args = if manager_ty_generic_args.is_some() {
    let args = manager_ty_generic_args.unwrap();
    quote! { < #args > }
  } else {
    quote! {}
  };

  quote! {
    type ARC<T> = std::sync::Arc<T>;
    type RWLOCK<T> = tokio::sync::RwLock<T>;
    type RWLOCK_WG<'a, T> = tokio::sync::RwLockWriteGuard<'a, T>;
    type RWLOCK_RG<'a, T> = tokio::sync::RwLockReadGuard<'a, T>;

    #[doc = #doc_str_struct]
    #[derive(Debug)]
    struct #manager_ty #where_clause {
      #property_name_ident: ARC<RWLOCK<#thing_ty>>
    }

    #[doc = #doc_str_default_impl_for_struct]
    impl #opt_generic_args Default for #manager_ty #where_clause {
      fn default() -> #manager_ty {
        #manager_name_ident {
          #property_name_ident: ARC::new(RWLOCK::new(Default::default())),
        }
      }
    }

    #[doc = #doc_str_impl_for_struct]
    impl #opt_generic_args #manager_ty #where_clause {
      #[doc = #doc_str_setter_fn]
      pub async fn set_value_of_wrapped_thing(
        &self,
        value: #thing_ty,
      ) {
        *self.#property_name_ident.write().await = value;
      }

      #[doc = #doc_str_getter_fn]
      pub fn get_arc(&self) -> ARC<RWLOCK<#thing_ty>> {
        self.#property_name_ident.clone()
      }

      #[doc = #doc_str_static_lock_w]
      pub async fn with_arc_get_locked_thing_w<'a>(
        my_arc: &'a ARC<RWLOCK<#thing_ty>>
      ) -> RWLOCK_WG<'a, #thing_ty> {
        my_arc.write().await
      }

      #[doc = #doc_str_static_lock_r]
      pub async fn with_arc_get_locked_thing_r<'a>(
        my_arc: &'a ARC<RWLOCK<#thing_ty>>
      ) -> RWLOCK_RG<'a, #thing_ty> {
        my_arc.read().await
      }

      #[doc = #doc_str_static_with_arc_setter_fn]
      pub async fn with_arc_set_value_of_wrapped_thing(
        my_arc: &ARC<RWLOCK<#thing_ty>>,
        value: #thing_ty,
      ) {
        *my_arc.write().await = value;
      }
    }
  }
  .into()
}

/// Example of syntax to parse:
/// ```no_run
/// fn_macro_custom_syntax! {
///   ThingManager<K, V>
///   where K: Send + Sync + 'static, V: Send + Sync + 'static
///   for my_property_name
///   as type std::collections::HashMap<K, V>
/// }
/// ```
#[derive(Debug)]
struct ManagerOfThingInfo {
  manager_name_ident: Ident,
  manager_ty: Type,
  manager_ty_generic_args: Option<Punctuated<GenericArgument, Comma>>,
  where_clause: Option<WhereClause>,
  thing_ty: Type,
  property_name_ident: Ident,
}

/// [Parse docs](https://docs.rs/syn/latest/syn/parse/index.html)
impl Parse for ManagerOfThingInfo {
  fn parse(input: ParseStream) -> Result<Self> {
    // 👀 Manager Type, eg: `ThingManager<K,V>`.
    let manager_ty: Type = input.parse()?;
    let manager_ty_generic_args = match manager_ty.has_angle_bracketed_generic_args() {
      true => Some(
        manager_ty
          .get_angle_bracketed_generic_args_result()
          .unwrap(),
      ),
      false => None,
    };
    // debug!(manager_ty_has_generic_args);

    // 👀 Optional where clause, eg: `where K: Send+Sync+'static, V: Send+Sync+'static`.
    let mut where_clause: Option<WhereClause> = None;
    if input.peek(Token![where]) {
      where_clause = Some(input.parse::<WhereClause>()?);
    } else {
      if manager_ty.has_angle_bracketed_generic_args() {
        let ident_vec = manager_ty
          .get_angle_bracketed_generic_args_idents_result()
          .unwrap();
        let my_ts = quote! {
          where #(#ident_vec: Default + Send + Sync + 'static),*
        }
        .into();
        let my_where_clause: WhereClause = syn::parse(my_ts).unwrap();
        where_clause = Some(my_where_clause)
      }
    }

    // 👀 use Ident, eg: `for my_map`.
    input.parse::<Token![for]>()?;
    let property_name_ident: Ident = input.parse()?;

    // 👀 type keyword.
    input.parse::<Token![as]>()?;
    input.parse::<Token![type]>()?;

    // 👀 Thing Type, eg: `std::collections::HashMap<K, V>`.
    let thing_ty: Type = input.parse()?;

    // Done parsing. Extract the manager name.
    let manager_name_ident = if manager_ty.has_ident() {
      manager_ty.get_ident().unwrap()
    } else {
      panic!("Expected Type::Path::TypePath.segments to have an Ident")
    };

    Ok(ManagerOfThingInfo {
      manager_ty_generic_args,
      manager_name_ident,
      manager_ty,
      thing_ty,
      where_clause,
      property_name_ident,
    })
  }
}
