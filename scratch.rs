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


code --remote ssh-remote+nazmul-desktop.local /home/nazmul/github/r3bl-open-core/


call_if_true!(DEBUG_TUI_COMPOSITOR, {
    let message = string_storage!(
        "print_plain_text() {ar} {ch}",
        ar = glyphs::RIGHT_ARROW_GLYPH,
        ch = glyphs::PAINT_GLYPH,
    );
    let details = string_storage!(
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
  let message = format!(
      "ColumnComponent::render {ch}",
      ch = glyphs::RENDER_GLYPH
  );
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


call_if_true!(DEBUG_TUI_MOD, {
  let message = "\n💾💾💾✅ Successfully read file";
  let details = string_storage!("{file_path:?}");
  let details_fmt = style_primary(&details);
  // % is Display, ? is Debug.
  tracing::debug!(
      message = %message,
      file_path = ?file_path,
      details = %details_fmt
  );
});


// % is Display, ? is Debug.
tracing::info!("main_event_loop -> Tick: 🌄 " = ?input_event);

let message = format!(
    "AppWithLayout::app_handle_event -> switch focus {ch}",
    ch = glyphs::FOCUS_GLYPH
);
// % is Display, ? is Debug.
tracing::info!(
    message = message,
    has_focus = ?has_focus
);

/*
- EXP sha: 88c55be0843a57259c932a81396792eb9b4ff7d4
  - FPS: 240
- LKG sha: 4cb67595cfaa055175713e0e8c2db66c01866e4a
  - FPS:

git switch -
 */

# For smallstr & smallvec.
smallstr = { version = "0.3.0", features = ["serde", "std"] }
smallvec = { version = "1.6.1", features = ["serde"] }

# For lolcat_each_char_in_unicode_string.
let mut lolcat_temp = LolcatBuilder::new()
  .set_color_change_speed(ColorChangeSpeed::Rapid)
  .build();

// let mut my_lolcat: Cow<'a, Lolcat> = match lolcat {
//     Some(lolcat_arg) => {
//         saved_orig_speed = Some(lolcat_arg.color_wheel_control.color_change_speed);
//         lolcat_arg.color_wheel_control.color_change_speed = ColorChangeSpeed::Rapid;
//         Cow::Borrowed(lolcat_arg)
//     }
//     None => {
//         let lolcat_temp = LolcatBuilder::new()
//             .set_color_change_speed(ColorChangeSpeed::Rapid)
//             .build();
//         Cow::Owned(lolcat_temp)
//     }
// };
//
// let it = my_lolcat.to_mut().colorize_to_styled_texts(unicode_string);
//
// // Restore saved_orig_speed if it was set.
// if let Some(orig_speed) = saved_orig_speed {
//     my_lolcat.to_mut().color_wheel_control.color_change_speed = orig_speed;
// }
//
// it

// PERF: [ ] use this instead of [Micro/Tiny/Small/Normal/Large]StringBackingStore
pub type StringStorage = SmallStringBackingStore;

// PERF: [ ] replace with StringStorage and avoid allocation using write! & index > 0 check to write \n

/*
UnicodeString no longer owns the underlying data
Just like GraphemeClusterSegment, it has to be given the &str data for it to get a slice
Look at:
- GraphemeClusterSegment::get_str()
- UnicodeString::get_str()
*/

// DO_NOT_COMMIT:
|ApplyChangeArgs {
  lines,
  caret,
  scroll_offset,
}|

.unicode_string().display_width -> .display_width()




pub fn try_parse_and_highlight(
  editor_text_lines: &[impl AsRef<str>],
  maybe_current_box_computed_style: &Option<TuiStyle>,
  maybe_syntect_tuple: Option<(&SyntaxSet, &Theme)>,
) -> CommonResult<StyleUSSpanLines> {
  // PERF: This is a known performance bottleneck. The underlying storage mechanism for content in the editor will have to change (from Vec<String>) for this to be possible.
  // Convert the editor text into a StringStorage (unfortunately requires allocating to
  // get the new lines back, since they're stripped out when loading content into the
  // editor buffer struct).
  let mut acc = DocumentStorage::new();
  use std::fmt::Write as _;
  for line in editor_text_lines {
      _ = writeln!(acc, "{}\n", line.as_ref());
  }
  let it = parse_markdown(&acc);
