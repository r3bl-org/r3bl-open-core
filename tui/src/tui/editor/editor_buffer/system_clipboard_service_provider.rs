/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

use copypasta_ext::{copypasta::ClipboardProvider, x11_fork::ClipboardContext};
use r3bl_core::throws;

use super::{ClipboardResult, ClipboardService};
use crate::DEBUG_TUI_COPY_PASTE;

pub struct SystemClipboard;

impl ClipboardService for SystemClipboard {
    fn try_to_put_content_into_clipboard(
        &mut self,
        content: String,
    ) -> ClipboardResult<()> {
        throws!({
            let mut ctx = ClipboardContext::new()?;
            ctx.set_contents(content.clone())?;

            DEBUG_TUI_COPY_PASTE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "📋📋📋 Selected Text was copied to clipboard",
                    copied = %content,
                );
            });
        })
    }

    fn try_to_get_content_from_clipboard(&mut self) -> ClipboardResult<String> {
        let mut ctx = ClipboardContext::new()?;
        let content = ctx.get_contents()?;

        Ok(content)
    }
}

pub mod test_fixtures {
    use super::{ClipboardResult, ClipboardService};

    #[derive(Debug, Default)]
    pub struct TestClipboard {
        pub content: String,
    }

    impl ClipboardService for TestClipboard {
        fn try_to_put_content_into_clipboard(
            &mut self,
            content: String,
        ) -> ClipboardResult<()> {
            self.content = content;
            Ok(())
        }

        fn try_to_get_content_from_clipboard(&mut self) -> ClipboardResult<String> {
            Ok(self.content.clone())
        }
    }
}
