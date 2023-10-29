/*
 *   Copyright (c) 2023 R3BL LLC
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

extern crate r3bl_clipboard;

use r3bl_clipboard::ClipboardProvider;
#[cfg(target_os = "linux")]
use r3bl_clipboard::x11_clipboard::{X11ClipboardContext, Primary};

#[cfg(target_os = "linux")]
fn main() {
    let mut ctx: X11ClipboardContext<Primary> = ClipboardProvider::new().unwrap();

    let the_string = "Hello, world!";

    ctx.set_contents(the_string.to_owned()).unwrap();
}

#[cfg(not(target_os = "linux"))]
fn main() {
    println!("Primary selection is only available under linux!");
}
