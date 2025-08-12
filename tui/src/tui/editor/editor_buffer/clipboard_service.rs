// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use copypasta_ext::{copypasta::ClipboardProvider, x11_fork::ClipboardContext};

use super::{ClipboardResult, ClipboardService};
use crate::{DEBUG_TUI_COPY_PASTE, throws};

#[derive(Debug)]
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

pub mod clipboard_test_fixtures {
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
