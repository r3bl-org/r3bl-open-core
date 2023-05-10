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

//! # Watch macro expansion
//!
//! To watch for changes run this script:
//! `./cargo-watch-macro-expand-one-test.fish test_builder_macro`
//!
//! # Watch test output
//!
//! To watch for test output run this script:
//! `./cargo-watch-one-test.fish test_builder_macro`

mod test1 {
    use r3bl_rs_utils::Builder;

    #[derive(Builder)]
    pub struct MyStruct {
        my_string: String,
        my_enum: MyEnum,
        my_number: i32,
    }

    #[derive(Default)]
    pub enum MyEnum {
        #[default]
        MyVariant1,
    }

    #[test]
    fn test_proc_macro_struct_and_enum() {
        let my_struct: MyStruct = MyStructBuilder::new()
            .set_my_string("Hello".to_string())
            .set_my_number(1_i32)
            .build();
        assert_eq!(my_struct.my_string, "Hello");
        match my_struct.my_enum {
            MyEnum::MyVariant1 => {}
        }
        assert_eq!(my_struct.my_number, 1);
    }
}

mod test2 {
    use r3bl_rs_utils::Builder;

    #[derive(Builder)]
    pub struct Point<X, Y> {
        x: X,
        y: Y,
    }

    #[test]
    fn test_proc_macro_no_where_clause() {
        let my_pt: Point<i32, i32> =
            PointBuilder::new().set_x(1_i32).set_y(2_i32).build();

        assert_eq!(my_pt.x, 1);
        assert_eq!(my_pt.y, 2);
    }
}

mod test3 {
    use r3bl_rs_utils::Builder;

    #[derive(Builder)]
    pub struct Point<X, Y>
    where
        X: std::fmt::Display + Clone,
        Y: std::fmt::Display + Clone,
    {
        x: X,
        y: Y,
    }

    #[test]
    fn test_proc_macro_generics() {
        let my_pt: Point<i32, i32> =
            PointBuilder::new().set_x(1_i32).set_y(2_i32).build();
        assert_eq!(my_pt.x, 1);
        assert_eq!(my_pt.y, 2);
    }
}
