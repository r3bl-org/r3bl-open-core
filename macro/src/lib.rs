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

//! [Procedural macro guide](https://developerlife.com/2022/03/30/rust-proc-macro/).

extern crate proc_macro;

mod utils;
mod manager_of_things;
mod builder;
mod fn_wrapper;

use fn_wrapper::{make_safe_async, make_shareable};
use proc_macro::TokenStream;

#[proc_macro_derive(Builder)]
pub fn derive_macro_builder(input: TokenStream) -> TokenStream {
  builder::derive_proc_macro_impl(input)
}

#[proc_macro]
pub fn make_struct_safe_to_share_and_mutate(input: TokenStream) -> TokenStream {
  manager_of_things::fn_proc_macro_impl(input)
}

#[deprecated(since = "0.7.12", note = "please use [`AsyncMiddleware`] instead")]
#[proc_macro]
pub fn make_safe_async_fn_wrapper(input: TokenStream) -> TokenStream {
  make_safe_async::fn_proc_macro_impl(input)
}

#[proc_macro]
pub fn make_shareable_fn_wrapper(input: TokenStream) -> TokenStream {
  make_shareable::fn_proc_macro_impl(input)
}
