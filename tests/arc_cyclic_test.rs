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

//! New in Rust v 1.60.0: Arc cyclic references.
//! https://doc.rust-lang.org/stable/std/sync/struct.Arc.html#method.new_cyclic

#![allow(unused)]
#![allow(dead_code)]

use std::sync::RwLock;
use std::sync::{Arc, Weak};

struct Gadget {
  pub weak_me: Weak<RwLock<Gadget>>,
  pub data: String,
}

impl Gadget {
  /// Construct a reference counted Gadget.
  fn new() -> Arc<RwLock<Self>> {
    Arc::new_cyclic(|weak_me_ref| {
      let weak_me_clone = weak_me_ref.clone();
      RwLock::new(Gadget {
        weak_me: weak_me_clone,
        data: Default::default(),
      })
    })
  }

  /// Return a reference counted pointer to Self.
  pub fn clone_arc(&self) -> Arc<RwLock<Self>> {
    self.weak_me.upgrade().unwrap()
  }

  pub fn set_data(
    &mut self,
    arg: &str,
  ) {
    self.data = String::from(arg);
  }

  pub fn get_data(&self) -> String {
    self.data.clone()
  }
}

#[test]
fn test_new() {
  let g_arc: Arc<RwLock<Gadget>> = Gadget::new();

  g_arc
    .write()
    .unwrap()
    .set_data("foo");
  assert_eq!(
    g_arc.read().unwrap().get_data(),
    "foo"
  );

  let g_arc_clone: Arc<RwLock<Gadget>> = g_arc.read().unwrap().clone_arc();
  g_arc_clone
    .write()
    .unwrap()
    .set_data("dummy");
  assert_eq!(
    g_arc_clone
      .read()
      .unwrap()
      .get_data(),
    "dummy"
  );
}
