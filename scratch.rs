/*
 *   Copyright (c) 2024 R3BL LLC
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

/*
Open vscode connected to a remote machine via SSH:
code --remote ssh-remote+nazmul-desktop.local /home/nazmul/github/r3bl-open-core/
*/

call_if_true!(DEBUG_TUI_COMPOSITOR, {
    let message = inline_string!(
        "print_plain_text() {ar} {ch}",
        ar = glyphs::RIGHT_ARROW_GLYPH,
        ch = glyphs::PAINT_GLYPH,
    );
    let details = inline_string!(
        "insertion at: display_row_index: {a}, display_col_index: {b}, window_size: {c:?},
        text: '{d}',
        width: {e:?}",
        a = display_row_index,
        b = display_col_index,
        c = my_offscreen_buffer.window_size,
        d = str!(clip_2_gcs),
        e = clip_2_gcs.get_display_width(),
    );
    // % is Display, ? is Debug.
    tracing::info! {
        message = %message,
        details = %details
    };
});

call_if_true!(DEBUG_TUI_MOD, {
    let message = format!("ColumnComponent::render {ch}", ch = glyphs::RENDER_GLYPH);
    // % is Display, ? is Debug.
    tracing::info!(
        message = message,
        current_box = ?current_box,
        box_origin_pos = ?box_origin_pos,
        box_bounds_size = ?box_bounds_size,
        content_pos = ?content_cursor_pos,
        render_pipeline = ?pipeline,
    );
});
