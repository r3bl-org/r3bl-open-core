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

//! # Watch macro expansion
//!
//! To watch for changes run this script:
//! `./cargo-watch-macro-expand-one-test.fish test_builder_macro`
//!
//! # Watch test output
//!
//! To watch for test output run this script:
//! `./cargo-watch-one-test.fish test_builder_macro`

use my_proc_macros_lib::Builder;

#[test]
fn test_proc_macro_struct_and_enum() {
  #[derive(Builder)]
  struct MyStruct {
    my_string: String,
    my_enum: MyEnum,
    my_number: i32,
  }

  enum MyEnum {
    MyVariant1,
  }

  impl Default for MyEnum {
    fn default() -> Self { MyEnum::MyVariant1 }
  }
}

#[test]
fn test_proc_macro_no_where_clause() {
  #[derive(Builder)]
  struct Point<X, Y> {
    x: X,
    y: Y,
  }

  let my_pt: Point<i32, i32> = PointBuilder::new()
    .set_x(1 as i32)
    .set_y(2 as i32)
    .build();

  assert_eq!(my_pt.x, 1);
  assert_eq!(my_pt.y, 2);
}

#[test]
fn test_proc_macro_generics() {
  #[derive(Builder)]
  struct Point<X, Y>
  where
    X: std::fmt::Display + Clone,
    Y: std::fmt::Display + Clone,
  {
    x: X,
    y: Y,
  }

  let my_pt: Point<i32, i32> = PointBuilder::new()
    .set_x(1 as i32)
    .set_y(2 as i32)
    .build();

  assert_eq!(my_pt.x, 1);
  assert_eq!(my_pt.y, 2);
}
