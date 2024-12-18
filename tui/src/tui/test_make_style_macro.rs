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

//! # Watch macro expansion
//!
//! To watch for changes run this script:
//! `./cargo-watch-macro-expand-one-test.fish test_make_style_macro`
//!
//! # Watch test output
//!
//! To watch for test output run this script:
//! `./cargo-watch-one-test.fish test_make_style_macro`

#[cfg(test)]
mod tests {
    use r3bl_core::{assert_eq2, ch, color, with, ANSIBasicColor, TuiStyle};
    use r3bl_macro::tui_style;

    #[test]
    fn test_syntax_expansion() {
        let _ = tui_style! {
          id:       1
          attrib:   [dim, bold]
          padding:  1
          color_fg: color!(@red)
          color_bg: color!(0, 0, 0)
        };
    }

    #[test]
    fn test_syntax_expansion_dsl() {
        let _ = tui_style! {
          id: 1
          attrib: [dim, bold]
          padding: 1
          color_fg: color!(@red)
          color_bg: color!(0, 0, 0)
        };
        let _ = tui_style! {
          id: 1
          attrib: [dim, bold]
          padding: 1
          color_fg: color!(@red)
          color_bg: color!(0, 0, 0)
        };
        let _ = tui_style! {
          id: 1
          attrib: [dim, bold]
          padding: 1
          color_fg: color!(@red)
          color_bg: color!(0, 0, 0)
        };
    }

    #[test]
    fn test_with_nothing() {
        let style: TuiStyle = tui_style! {};
        assert_eq2!(style.id, u8::MAX);
    }

    #[test]
    fn test_with_attrib() {
        let style_no_attrib = tui_style! {
          id: 1
        };
        assert_eq!(style_no_attrib.id, 1);
        assert!(!style_no_attrib.bold);
        assert!(!style_no_attrib.dim);

        let style_with_attrib = tui_style! {
          id: 2
          attrib: [dim, bold]
        };
        assert_eq!(style_with_attrib.id, 2);
        assert!(style_with_attrib.bold);
        assert!(style_with_attrib.dim);
        assert!(!style_with_attrib.underline);
        assert!(!style_with_attrib.reverse);
        assert!(!style_with_attrib.hidden);
        assert!(!style_with_attrib.strikethrough);
    }

    #[test]
    fn test_with_padding() {
        with! {
          tui_style! {
            id: 1
            padding: 1
          },
          as it,
          run {
            assert_eq!(it.padding, Some(ch!(1)));
          }
        }
    }

    #[test]
    fn test_with_color_fg() {
        with! {
          tui_style! {
            id: 1
            color_fg: color!(@red)
          },
          as it,
          run {
            assert_eq!(it.color_fg, color!(@red).into());
          }
        }
    }

    #[test]
    fn test_with_color_bg() {
        with! {
          tui_style! {
            id: 1
            color_bg: color!(0, 0, 0)
          },
          as it,
          run {
            assert_eq!(it.color_bg, color!(0, 0, 0).into());
          }
        }
    }
}
