// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{PartialFlexBox, Size, StyleUSSpanLines, get_cached_syntax_set,
            get_cached_theme};
use std::fmt::Debug;
use syntect::{highlighting::Theme, parsing::SyntaxSet};

/// Do not create this struct directly. Please use [`new()`] instead.
///
/// Holds data related to rendering in between render calls. This struct is mutable when
/// render is called. That is not the case with state, which is immutable during render
/// phase.
///
/// 1. This is not stored in the [`EditorBuffer`] struct, which lives in the app's state.
///    The state provides the underlying document or buffer struct that holds the actual
///    document.
/// 2. Rather, this struct is stored in the [`EditorComponent`] struct, which lives inside
///    of the [`crate::App`].
///
/// In order to change the document, you can use the [`apply_event`] method which takes
/// [`InputEvent`] and tries to convert it to an [`EditorEvent`] and then execute them
/// against this buffer.
///
/// [`InputEvent`]: crate::InputEvent
/// [`EditorEvent`]: crate::EditorEvent
/// [`EditorBuffer`]: crate::EditorBuffer
/// [`EditorComponent`]: crate::EditorComponent
/// [`apply_event`]: crate::engine_public_api::apply_event
/// [`new()`]: Self::new
#[derive(Debug)]
pub struct EditorEngine {
    /// Set by [`crate::engine_public_api::render_engine`].
    pub current_box: PartialFlexBox,
    pub config_options: EditorEngineConfig,
    /// Syntax highlighting support. This is a very heavy object to create, re-use it.
    pub syntax_set: &'static SyntaxSet,
    /// Syntax highlighting support. This is a very heavy object to create, re-use it.
    pub theme: &'static Theme,
    pub ast_cache: Option<StyleUSSpanLines>,
}

impl Default for EditorEngine {
    fn default() -> Self { EditorEngine::new(EditorEngineConfig::default()) }
}

impl EditorEngine {
    /// Syntax highlighting support - [`SyntaxSet`] and [Theme] are a very expensive
    /// objects to create, so re-use them.
    #[must_use]
    pub fn new(config_options: EditorEngineConfig) -> Self {
        Self {
            current_box: PartialFlexBox::default(),
            config_options,
            syntax_set: get_cached_syntax_set(),
            theme: get_cached_theme(),
            ast_cache: None,
        }
    }

    #[must_use]
    pub fn viewport(&self) -> Size { self.current_box.style_adjusted_bounds_size }

    pub fn set_ast_cache(&mut self, ast_cache: StyleUSSpanLines) {
        self.ast_cache = Some(ast_cache);
    }

    pub fn clear_ast_cache(&mut self) { self.ast_cache = None }

    #[must_use]
    pub fn get_ast_cache(&self) -> Option<&StyleUSSpanLines> { self.ast_cache.as_ref() }

    #[must_use]
    pub fn ast_cache_is_empty(&self) -> bool { self.ast_cache.is_none() }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorEngineConfig {
    pub multiline_mode: LineMode,
    pub syntax_highlight: SyntaxHighlightMode,
    pub edit_mode: EditMode,
}

mod editor_engine_config_options_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Default for EditorEngineConfig {
        fn default() -> Self {
            Self {
                multiline_mode: LineMode::MultiLine,
                syntax_highlight: SyntaxHighlightMode::Enable,
                edit_mode: EditMode::ReadWrite,
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineMode {
    SingleLine,
    MultiLine,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyntaxHighlightMode {
    Disable,
    Enable,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assert_eq2, height, width};

    #[test]
    fn test_editor_engine_new() {
        // Test default construction.
        let engine = EditorEngine::default();
        assert_eq2!(engine.config_options.multiline_mode, LineMode::MultiLine);
        assert_eq2!(
            engine.config_options.syntax_highlight,
            SyntaxHighlightMode::Enable
        );
        assert_eq2!(engine.config_options.edit_mode, EditMode::ReadWrite);
        assert!(engine.ast_cache.is_none());

        // Test custom configuration.
        let custom_config = EditorEngineConfig {
            multiline_mode: LineMode::SingleLine,
            syntax_highlight: SyntaxHighlightMode::Disable,
            edit_mode: EditMode::ReadOnly,
        };
        let engine = EditorEngine::new(custom_config.clone());
        assert_eq2!(engine.config_options, custom_config);
    }

    #[test]
    fn test_viewport() {
        let mut engine = EditorEngine::default();

        // Default viewport should be empty.
        assert_eq2!(engine.viewport(), Size::default());

        // Set a custom viewport.
        engine.current_box.style_adjusted_bounds_size = width(100) + height(50);
        assert_eq2!(engine.viewport(), width(100) + height(50));
    }

    #[test]
    fn test_ast_cache_operations() {
        use crate::List;

        let mut engine = EditorEngine::default();

        // Initially cache should be empty.
        assert!(engine.ast_cache_is_empty());
        assert!(engine.get_ast_cache().is_none());

        // Set cache - create empty StyleUSSpanLines for testing.
        let test_ast: StyleUSSpanLines = List::new();
        engine.set_ast_cache(test_ast.clone());
        assert!(!engine.ast_cache_is_empty());
        assert_eq2!(engine.get_ast_cache(), Some(&test_ast));

        // Clear cache
        engine.clear_ast_cache();
        assert!(engine.ast_cache_is_empty());
        assert!(engine.get_ast_cache().is_none());
    }

    #[test]
    fn test_editor_engine_config_default() {
        let config = EditorEngineConfig::default();
        assert_eq2!(config.multiline_mode, LineMode::MultiLine);
        assert_eq2!(config.syntax_highlight, SyntaxHighlightMode::Enable);
        assert_eq2!(config.edit_mode, EditMode::ReadWrite);
    }

    #[test]
    fn test_config_enums() {
        // Test EditMode variants.
        assert_eq2!(EditMode::ReadOnly, EditMode::ReadOnly);
        assert_eq2!(EditMode::ReadWrite, EditMode::ReadWrite);
        assert!(EditMode::ReadOnly != EditMode::ReadWrite);

        // Test LineMode variants.
        assert_eq2!(LineMode::SingleLine, LineMode::SingleLine);
        assert_eq2!(LineMode::MultiLine, LineMode::MultiLine);
        assert!(LineMode::SingleLine != LineMode::MultiLine);

        // Test SyntaxHighlightMode variants.
        assert_eq2!(SyntaxHighlightMode::Enable, SyntaxHighlightMode::Enable);
        assert_eq2!(SyntaxHighlightMode::Disable, SyntaxHighlightMode::Disable);
        assert!(SyntaxHighlightMode::Enable != SyntaxHighlightMode::Disable);
    }

    #[test]
    fn test_syntax_set_and_theme_are_cached() {
        // Create two engines and verify they share the same syntax_set and theme.
        let engine1 = EditorEngine::default();
        let engine2 = EditorEngine::default();

        // Since these are static references, they should point to the same memory.
        assert!(std::ptr::eq(engine1.syntax_set, engine2.syntax_set));
        assert!(std::ptr::eq(engine1.theme, engine2.theme));
    }
}
