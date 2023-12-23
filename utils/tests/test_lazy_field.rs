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

use r3bl_rs_utils::{LazyExecutor, LazyField};
use r3bl_rs_utils_core::assert_eq2;

#[test]
fn test_lazy_field() {
    struct MyExecutor;
    impl LazyExecutor<i32> for MyExecutor {
        fn compute(&mut self) -> i32 { 1 }
    }

    let mut lazy_field = LazyField::new(Box::new(MyExecutor));
    assert!(!lazy_field.has_computed);

    // First access will trigger the computation.
    let value = lazy_field.compute();
    assert!(lazy_field.has_computed);
    assert_eq2!(value, 1);

    // Subsequent accesses will not trigger the computation.
    let value = lazy_field.compute();
    assert!(lazy_field.has_computed);
    assert_eq2!(value, 1);
}
