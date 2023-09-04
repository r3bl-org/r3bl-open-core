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

#![allow(unused)]
#![allow(dead_code)]

/// The code in the test is equivalent to the code w/out using cyclic arc.
/// ```ignore
/// fn test_simple_macro_expansion_4() {
///   type _Arc<T> = std::sync::Arc<T>;
///   type _RwLock<T> = std::sync::RwLock<T>;
///   type WRITE_G<'a, T> = std::sync::RwLockWriteGuard<'a, T>;
///   type READ_G<'a, T> = std::sync::RwLockReadGuard<'a, T>;
///
///   pub struct FnWrapper4<S, A> {
///     pub fn_mut: _Arc<_RwLock<dyn Fn(&S, &A) -> S + Send + Sync + 'static>>,
///   }
///
///   impl<S, A> FnWrapper4<S, A>
///   where
///     S: Sync + Send + 'static,
///     A: Sync + Send + 'static,
///   {
///     pub fn from(fn_mut: impl Fn(&S, &A) -> S + Send + Sync + 'static) -> Self {
///       Self {
///         fn_mut: _Arc::new(_RwLock::new(fn_mut)),
///       }
///     }
///
///     pub fn get(&self) -> _Arc<_RwLock<dyn Fn(&S, &A) -> S + Send + Sync + 'static>> {
///       self.fn_mut.clone()
///     }
///
///     pub fn invoke(&self, arg1: &S, arg2: &A) -> S {
///       let arc_lock_fn_mut = self.get();
///       let mut fn_mut = arc_lock_fn_mut.write().unwrap();
///       fn_mut(arg1, arg2)
///     }
///   }
/// }
/// ```

#[test]
fn test_new() {
    type _Weak<T> = std::sync::Weak<T>;
    type _Arc<T> = std::sync::Arc<T>;
    type _RwLock<T> = std::sync::RwLock<T>;
    type _Box<T> = std::boxed::Box<T>;

    pub struct FnWrapper<S, A> {
        pub weak_me: _Weak<_RwLock<Self>>,
        pub fn_mut: _Box<dyn Fn(S, A) -> S + Send + Sync + 'static>,
    }

    impl<S, A> FnWrapper<S, A>
    where
        S: Send + Sync + 'static,
        A: Send + Sync + 'static,
    {
        /// Constructor.
        pub fn from(
            fn_mut: impl Fn(S, A) -> S + Sync + Send + 'static,
        ) -> _Arc<_RwLock<Self>> {
            _Arc::new_cyclic(|weak_me_ref| {
                _RwLock::new(FnWrapper {
                    weak_me: weak_me_ref.clone(),
                    fn_mut: _Box::new(fn_mut),
                })
            })
        }

        /// Returns a clone of my `Arc`.
        pub fn get(&self) -> _Arc<_RwLock<Self>> { self.weak_me.upgrade().unwrap() }

        /// Proxy for `fu_mut` invocation.
        pub fn invoke(&self, arg1: S, arg2: A) -> S {
            let arc_me = self.get();
            let box_fn_mut = &arc_me.write().unwrap().fn_mut;
            box_fn_mut(arg1, arg2)
        }
    }
}
