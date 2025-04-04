/*
 *   Copyright (c) 2025 R3BL LLC
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

use r3bl_core::{console_log,
                height,
                items_owned,
                width,
                InputDevice,
                ItemsBorrowed,
                ItemsOwned,
                OutputDevice,
                SharedWriter};
use r3bl_tuify::{choose, Header, HowToChoose, StyleSheet};

#[tokio::main]
async fn main() {
    println!("This is a placeholder for the async choose function.");

    let id = &mut InputDevice::new_event_stream();
    let od = &mut OutputDevice::new_stdout();
    let msw: Option<SharedWriter> = None;

    let header = Header::SingleLine("Header".into());
    let inline_vec: ItemsOwned = items_owned(ItemsBorrowed(&["one", "two", "three"]));
    let max_size = height(5) + width(0);
    let how = HowToChoose::Single;
    let style_sheet = StyleSheet::hot_pink_style();

    let res_user_chose = choose(
        header,
        inline_vec,
        max_size,
        how,
        style_sheet,
        (od, id, msw),
    )
    .await;

    console_log!(res_user_chose);
}
