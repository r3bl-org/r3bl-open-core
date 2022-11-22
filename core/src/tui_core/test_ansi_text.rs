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

#[cfg(test)]
mod tests {
    use crate::{assert_eq2, ANSIText};

    #[test]
    fn test_lolcat_no_max_display_cols() {
        let test_data = "\u{1b}[38;2;51;254;77mS\u{1b}[39m\u{1b}[38;2;52;254;77mt\u{1b}[39m\u{1b}[38;2;52;254;77ma\u{1b}[39m\u{1b}[38;2;52;254;76mt\u{1b}[39m\u{1b}[38;2;53;254;76me\u{1b}[39m\u{1b}[38;2;53;254;76m \u{1b}[39m\u{1b}[38;2;53;254;75m{\u{1b}[39m\u{1b}[38;2;54;254;75m \u{1b}[39m\u{1b}[38;2;54;254;74ms\u{1b}[39m\u{1b}[38;2;54;254;74mt\u{1b}[39m\u{1b}[38;2;55;254;74ma\u{1b}[39m\u{1b}[38;2;55;254;73mc\u{1b}[39m\u{1b}[38;2;56;254;73mk\u{1b}[39m\u{1b}[38;2;56;254;72m:\u{1b}[39m\u{1b}[38;2;56;254;72m \u{1b}[39m\u{1b}[38;2;57;254;72m[\u{1b}[39m\u{1b}[38;2;57;254;71m0\u{1b}[39m\u{1b}[38;2;57;254;71m]\u{1b}[39m\u{1b}[38;2;58;254;71m \u{1b}[39m\u{1b}[38;2;58;254;70m}\u{1b}[39m";
        let unparsed_ansi_text = ANSIText::new(test_data);
        dbg!(unparsed_ansi_text.segments(None));
        dbg!(unparsed_ansi_text.segments(None).len());
        assert_eq2!(unparsed_ansi_text.segments(None).len(), 21);
    }

    #[test]
    fn test_lolcat_with_max_display_cols() {
        let test_data = "\u{1b}[38;2;51;254;77mS\u{1b}[39m\u{1b}[38;2;52;254;77mt\u{1b}[39m\u{1b}[38;2;52;254;77ma\u{1b}[39m\u{1b}[38;2;52;254;76mt\u{1b}[39m\u{1b}[38;2;53;254;76me\u{1b}[39m\u{1b}[38;2;53;254;76m \u{1b}[39m\u{1b}[38;2;53;254;75m{\u{1b}[39m\u{1b}[38;2;54;254;75m \u{1b}[39m\u{1b}[38;2;54;254;74ms\u{1b}[39m\u{1b}[38;2;54;254;74mt\u{1b}[39m\u{1b}[38;2;55;254;74ma\u{1b}[39m\u{1b}[38;2;55;254;73mc\u{1b}[39m\u{1b}[38;2;56;254;73mk\u{1b}[39m\u{1b}[38;2;56;254;72m:\u{1b}[39m\u{1b}[38;2;56;254;72m \u{1b}[39m\u{1b}[38;2;57;254;72m[\u{1b}[39m\u{1b}[38;2;57;254;71m0\u{1b}[39m\u{1b}[38;2;57;254;71m]\u{1b}[39m\u{1b}[38;2;58;254;71m \u{1b}[39m\u{1b}[38;2;58;254;70m}\u{1b}[39m";
        let unparsed_ansi_text = ANSIText::new(test_data);
        dbg!(unparsed_ansi_text.segments(Some(4)));
        dbg!(unparsed_ansi_text.segments(Some(4)).len());
        assert_eq2!(unparsed_ansi_text.segments(Some(4)).len(), 4);
    }
}
