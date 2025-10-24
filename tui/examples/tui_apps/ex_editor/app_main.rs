// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{AppSignal, State};
use r3bl_tui::{App, BoxedSafeApp, CommonError, CommonResult, ComponentRegistry,
               ComponentRegistryMap, DEBUG_TUI_MOD, DialogBuffer, DialogChoice,
               DialogComponent, DialogEngineConfigOptions, DialogEngineMode, EditMode,
               EditorBuffer, EditorComponent, EditorEngineConfig, EventPropagation,
               FlexBox, FlexBoxId, GCStringOwned, GlobalData, HasEditorBuffers,
               HasFocus, InlineString, InputEvent, ItemsOwned, Key, KeyPress,
               LayoutDirection, LayoutManagement, LengthOps, LineMode, ModifierKeysMask,
               PerformPositioningAndSizing, RenderOpCommon, RenderOpIR, RenderOpsIR,
               RenderPipeline, SPACER_GLYPH, Size, Surface, SurfaceProps, SurfaceRender,
               SyntaxHighlightMode, TerminalWindowMainThreadSignal, TuiStylesheet, ZOrder,
               box_end, box_start, col, get_tui_style, glyphs, height, inline_string,
               new_style, ok, render_component_in_current_box,
               render_component_in_given_box, render_tui_styled_texts_into, req_size_pc,
               row, send_signal, surface, throws, throws_with_return, tui_color,
               tui_styled_text, tui_styled_texts, tui_stylesheet, width};
use tokio::sync::mpsc::Sender;

/// Constants for the ids.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Id {
    Editor = 1,
    SimpleDialog = 2,
    AutocompleteDialog = 3,
    EditorStyleNameDefault = 4,
    DialogStyleNameBorder = 5,
    DialogStyleNameTitle = 6,
    DialogStyleNameEditor = 7,
    DialogStyleNameResultsPanel = 8,
}

mod id_impl {
    use super::{FlexBoxId, Id};

    impl From<Id> for u8 {
        fn from(id: Id) -> u8 { id as u8 }
    }

    impl From<Id> for FlexBoxId {
        fn from(id: Id) -> FlexBoxId { FlexBoxId::new(id) }
    }
}

pub struct AppMain;

mod constructor {
    use super::{AppMain, AppSignal, BoxedSafeApp, DEBUG_TUI_MOD, State};

    impl Default for AppMain {
        fn default() -> Self {
            DEBUG_TUI_MOD.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(message = "🪙 construct ex_rc::AppMain");
            });
            Self
        }
    }

    impl AppMain {
        /// Note that this needs to be initialized before it can be used.
        pub fn new_boxed() -> BoxedSafeApp<State, AppSignal> {
            let it = Self;
            Box::new(it)
        }
    }
}

mod app_main_impl_app_trait {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl App for AppMain {
        type S = State;
        type AS = AppSignal;

        fn app_init(
            &mut self,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) {
            populate_component_registry::create_components(
                component_registry_map,
                has_focus,
            );
        }

        fn app_handle_input_event(
            &mut self,
            input_event: InputEvent,
            global_data: &mut GlobalData<State, AppSignal>,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) -> CommonResult<EventPropagation> {
            // Things from global scope.
            let GlobalData { state, .. } = global_data;

            // Check to see if the modal dialog should be activated.
            if let modal_dialogs::ModalActivateResult::Yes =
                modal_dialogs::should_activate(
                    input_event.clone(),
                    component_registry_map,
                    has_focus,
                    state,
                )
            {
                return Ok(EventPropagation::ConsumedRender);
            }

            // If modal not activated, route the input event to the focused component.
            ComponentRegistry::route_event_to_focused_component(
                global_data,
                input_event,
                component_registry_map,
                has_focus,
            )
        }

        fn app_handle_signal(
            &mut self,
            _action: &AppSignal,
            _global_data: &mut GlobalData<State, AppSignal>,
            _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            _has_focus: &mut HasFocus,
        ) -> CommonResult<EventPropagation> {
            Ok(EventPropagation::ConsumedRender)
        }

        fn app_render(
            &mut self,
            global_data: &mut GlobalData<State, AppSignal>,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) -> CommonResult<RenderPipeline> {
            throws_with_return!({
                let window_size = global_data.window_size;

                // Create a surface and then run the SurfaceRenderer
                // (ContainerSurfaceRender) on it.
                let mut surface = {
                    let mut it = surface!(stylesheet: stylesheet::create_stylesheet()?);

                    it.surface_start(SurfaceProps {
                        pos: col(0) + row(0),
                        size: {
                            let col_count = window_size.col_width;
                            let row_count = window_size.row_height -
                                height(2) /* Bottom row for for status bar & HUD. */;
                            col_count + row_count
                        },
                    })?;

                    perform_layout::ContainerSurfaceRender { _app: self }
                        .render_in_surface(
                            &mut it,
                            global_data,
                            component_registry_map,
                            has_focus,
                        )?;

                    it.surface_end()?;

                    it
                };

                // Render HUD.
                hud::create_hud(
                    &mut surface.render_pipeline,
                    window_size,
                    global_data.get_hud_report_with_spinner(),
                );

                // Render status bar.
                status_bar::render_status_bar(&mut surface.render_pipeline, window_size);

                // Return RenderOps pipeline (which will actually be painted elsewhere).
                surface.render_pipeline
            });
        }
    }
}

mod modal_dialogs {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    // This runs on every keystroke, so it should be fast.
    pub fn dialog_component_update_content(state: &mut State, id: FlexBoxId) {
        // This is Some only if the content has changed (ignoring caret movements).
        let maybe_changed_results: Option<ItemsOwned> = {
            if let Some(dialog_buffer) = state.dialog_buffers.get_mut(&id) {
                let vec_result = generate_random_results(
                    dialog_buffer
                        .editor_buffer
                        .get_as_string_with_comma_instead_of_newlines()
                        .as_str(),
                );
                Some(vec_result)
            } else {
                None
            }
        };

        state
            .dialog_buffers
            .entry(id)
            .and_modify(|it| {
                if let Some(results) = maybe_changed_results {
                    it.maybe_results = Some(results);
                }
            })
            .or_insert_with(
                // This code path should never execute, since to update the buffer given
                // an id, it should have already existed in the first
                // place, which is created by:
                // 1. [Action::SimpleDialogComponentInitializeFocused]
                // 2. [Action::AutocompleteDialogComponentInitializeFocused]
                || {
                    let mut it = DialogBuffer::new_empty();
                    it.editor_buffer = EditorBuffer::new_empty(None, None);
                    it
                },
            );

        // Content is empty.
        if let Some(dialog_buffer) = state.dialog_buffers.get_mut(&id)
            && dialog_buffer
                .editor_buffer
                .get_as_string_with_comma_instead_of_newlines()
                == ""
            && let Some(it) = state.dialog_buffers.get_mut(&id)
        {
            it.maybe_results = None;
        }
    }

    fn generate_random_results(content: &str) -> ItemsOwned {
        let start_rand_num = rand::random::<u8>() as usize;
        let max = 10;
        let mut it = ItemsOwned::with_capacity(max);
        for index in start_rand_num..(start_rand_num + max) {
            it.push(format!("{content}{index}").into());
        }
        it
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum ModalActivateResult {
        Yes,
        No,
    }

    pub fn should_activate(
        input_event: InputEvent,
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        has_focus: &mut HasFocus,
        state: &mut State,
    ) -> ModalActivateResult {
        // "Ctrl + l" => activate Simple.
        if input_event.matches_keypress(KeyPress::WithModifiers {
            key: Key::Character('l'),
            mask: ModifierKeysMask::new().with_ctrl(),
        }) {
            // Reset the dialog component prior to activating / showing it.
            ComponentRegistry::reset_component(
                component_registry_map,
                FlexBoxId::from(Id::SimpleDialog),
            );
            return match activate_simple_modal(component_registry_map, has_focus, state) {
                Ok(()) => ModalActivateResult::Yes,
                Err(err) => {
                    match err.downcast_ref::<CommonError>() {
                        // err is of concrete type CommonError.
                        Some(common_error) => {
                            // % is Display, ? is Debug.
                            tracing::error!(
                                message = "📣 Error activating simple modal",
                                error = ?common_error
                            );
                        }
                        // err is not of concrete type CommonError.
                        _ => { /* do nothing */ }
                    }
                    ModalActivateResult::No
                }
            };
        }

        // "Ctrl + k" => activate Autocomplete.
        if input_event.matches_keypress(KeyPress::WithModifiers {
            key: Key::Character('k'),
            mask: ModifierKeysMask::new().with_ctrl(),
        }) {
            // Reset the dialog component prior to activating / showing it.
            ComponentRegistry::reset_component(
                component_registry_map,
                FlexBoxId::from(Id::AutocompleteDialog),
            );
            return match activate_autocomplete_modal(
                component_registry_map,
                has_focus,
                state,
            ) {
                Ok(()) => ModalActivateResult::Yes,
                Err(err) => {
                    match err.downcast_ref::<CommonError>() {
                        // err is of concrete type CommonError.
                        Some(common_error) => {
                            // % is Display, ? is Debug.
                            tracing::error!(
                                message = "📣 Error activating autocomplete modal",
                                error = ?common_error
                            );
                        }
                        // err is not of concrete type CommonError.
                        _ => { /* do nothing */ }
                    }
                    ModalActivateResult::No
                }
            };
        }

        ModalActivateResult::No
    }

    /// If `input_event` matches <kbd>Ctrl+l</kbd> or <kbd>Ctrl+k</kbd>, then toggle the
    /// modal dialog.
    ///
    /// Note that this returns a [`EventPropagation::Consumed`] and not
    /// [`EventPropagation::ConsumedRender`] because both the following dispatched to the
    /// store & that will cause a rerender:
    /// 1. [`Action::SimpleDialogComponentInitializeFocused`].
    /// 2. [`Action::AutocompleteDialogComponentInitializeFocused`].
    pub fn dialog_component_initialize_focused(
        state: &mut State,
        id: FlexBoxId,
        title: InlineString,
        text: InlineString,
    ) {
        let dialog_buffer = {
            let mut it = DialogBuffer::new_empty();
            it.title = title;

            let start_display_col_index = col(0);
            let max_display_col_count = width(100);

            let text_gcs = GCStringOwned::from(&text);

            let content = {
                if text_gcs.display_width > max_display_col_count {
                    text_gcs.clip(start_display_col_index, max_display_col_count)
                } else {
                    text.as_str()
                }
            };

            it.editor_buffer.init_with(content.lines());
            it
        };
        state.dialog_buffers.insert(id, dialog_buffer);
    }

    fn activate_simple_modal(
        _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        has_focus: &mut HasFocus,
        state: &mut State,
    ) -> CommonResult<()> {
        throws!({
            // Initialize the dialog buffer with title & text.
            let title = "Simple Modal Dialog Title";
            let text = {
                match state.get_mut_editor_buffer(FlexBoxId::from(Id::Editor)) {
                    Some(editor_buffer) => {
                        editor_buffer.get_as_string_with_comma_instead_of_newlines()
                    }
                    None => "".into(),
                }
            };

            // Setting the has_focus to ComponentId::SimpleDialog will cause the dialog to
            // appear on the next render.
            has_focus.try_set_modal_id(FlexBoxId::from(Id::SimpleDialog))?;

            // Change the state so that it will trigger a render. This will show the title
            // & text on the next render.
            dialog_component_initialize_focused(
                state,
                FlexBoxId::from(Id::SimpleDialog),
                title.into(),
                text,
            );

            DEBUG_TUI_MOD.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "📣 activate modal simple",
                    has_focus = ?has_focus
                );
            });
        });
    }

    fn activate_autocomplete_modal(
        _component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        has_focus: &mut HasFocus,
        state: &mut State,
    ) -> CommonResult<()> {
        // Initialize the dialog buffer with title & text.
        let title = "Autocomplete Modal Dialog Title";
        let text = {
            match state.get_mut_editor_buffer(FlexBoxId::from(Id::Editor)) {
                Some(editor_buffer) => {
                    editor_buffer.get_as_string_with_comma_instead_of_newlines()
                }
                None => "".into(),
            }
        };

        // Setting the has_focus to Id::Dialog will cause the dialog to appear on the next
        // render.
        has_focus.try_set_modal_id(FlexBoxId::from(Id::AutocompleteDialog))?;

        // Change the state so that it will trigger a render. This will show the title and
        // text on the next render.
        dialog_component_initialize_focused(
            state,
            FlexBoxId::from(Id::AutocompleteDialog),
            title.into(),
            text,
        );

        DEBUG_TUI_MOD.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "📣 activate modal autocomplete",
                has_focus = ?has_focus
            );
        });

        ok!()
    }
}

mod perform_layout {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub struct ContainerSurfaceRender<'a> {
        pub _app: &'a mut AppMain,
    }

    impl SurfaceRender<State, AppSignal> for ContainerSurfaceRender<'_> {
        fn render_in_surface(
            &mut self,
            surface: &mut Surface,
            global_data: &mut GlobalData<State, AppSignal>,
            component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
            has_focus: &mut HasFocus,
        ) -> CommonResult<()> {
            throws!({
                // Layout editor component, and render it.
                {
                    box_start! (
                        in:                     surface,
                        id:                     FlexBoxId::from(Id::Editor),
                        dir:                    LayoutDirection::Vertical,
                        requested_size_percent: req_size_pc!(width: 100, height: 100),
                        styles:                 [Id::EditorStyleNameDefault]
                    );
                    render_component_in_current_box!(
                        in:                 surface,
                        component_id:       FlexBoxId::from(Id::Editor),
                        from:               component_registry_map,
                        global_data:        global_data,
                        has_focus:          has_focus
                    );
                    box_end!(in: surface);
                }

                // Then, render simple modal dialog (if it is active, on top of the editor
                // component).
                if has_focus.is_modal_id(FlexBoxId::from(Id::SimpleDialog)) {
                    render_component_in_given_box! {
                      in:                 surface,
                      box:                FlexBox::default(), /* This is not used as the modal breaks out of its box. */
                      component_id:       FlexBoxId::from(Id::SimpleDialog),
                      from:               component_registry_map,
                      global_data:        global_data,
                      has_focus:          has_focus
                    };
                }

                // Or, render autocomplete modal dialog (if it is active, on top of the
                // editor component).
                if has_focus.is_modal_id(FlexBoxId::from(Id::AutocompleteDialog)) {
                    render_component_in_given_box! {
                      in:                 surface,
                      box:                FlexBox::default(), /* This is not used as the modal breaks out of its box. */
                      component_id:       FlexBoxId::from(Id::AutocompleteDialog),
                      from:               component_registry_map,
                      global_data:        global_data,
                      has_focus:          has_focus
                    };
                }
            });
        }
    }
}

mod populate_component_registry {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub fn create_components(
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
        has_focus: &mut HasFocus,
    ) {
        insert_editor_component(component_registry_map);
        insert_dialog_component_simple(component_registry_map);
        insert_dialog_component_autocomplete(component_registry_map);

        // Switch focus to the editor component if focus is not set.
        let id = FlexBoxId::from(Id::Editor);
        has_focus.set_id(id);

        DEBUG_TUI_MOD.then(|| {
            // % is Display, ? is Debug.
            tracing::info!(
                message = %inline_string!("app_main init has_focus {ch}", ch = glyphs::FOCUS_GLYPH),
                has_focus = ?has_focus.get_id()
            );
        });
    }

    /// Insert editor component into registry if it's not already there.
    fn insert_editor_component(
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
    ) {
        let id = FlexBoxId::from(Id::Editor);
        let boxed_editor_component = {
            fn on_buffer_change(
                my_id: FlexBoxId,
                main_thread_channel_sender: Sender<
                    TerminalWindowMainThreadSignal<AppSignal>,
                >,
            ) {
                send_signal!(
                    main_thread_channel_sender,
                    TerminalWindowMainThreadSignal::Render(Some(my_id))
                );
            }

            let config_options = EditorEngineConfig::default();
            EditorComponent::new_boxed(id, config_options, on_buffer_change)
        };

        ComponentRegistry::put(component_registry_map, id, boxed_editor_component);

        DEBUG_TUI_MOD.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "app_main construct EditorComponent [ on_buffer_change ]"
            );
        });
    }

    /// Insert simple dialog component into registry if it's not already there.
    fn insert_dialog_component_simple(
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
    ) {
        mod handler_fn {
            #[allow(clippy::wildcard_imports)]
            use super::*;
            pub fn on_dialog_press_handler(
                dialog_choice: DialogChoice,
                state: &mut State,
                _main_thread_channel_sender: &mut Sender<
                    TerminalWindowMainThreadSignal<AppSignal>,
                >,
            ) {
                match dialog_choice {
                    DialogChoice::Yes(text) => {
                        modal_dialogs::dialog_component_initialize_focused(
                            state,
                            FlexBoxId::from(Id::SimpleDialog),
                            "Yes".into(),
                            text,
                        );
                    }
                    DialogChoice::No => {
                        modal_dialogs::dialog_component_initialize_focused(
                            state,
                            FlexBoxId::from(Id::SimpleDialog),
                            "No".into(),
                            "".into(),
                        );
                    }
                }
            }

            pub fn on_dialog_editor_change_handler(
                state: &mut State,
                _main_thread_channel_sender: &mut Sender<
                    TerminalWindowMainThreadSignal<AppSignal>,
                >,
            ) {
                modal_dialogs::dialog_component_update_content(
                    state,
                    FlexBoxId::from(Id::SimpleDialog),
                );
            }
        }

        let result_stylesheet = stylesheet::create_stylesheet();

        let dialog_options = DialogEngineConfigOptions {
            mode: DialogEngineMode::ModalSimple,
            maybe_style_border: get_tui_style! { @from_result: result_stylesheet , Id::DialogStyleNameBorder },
            maybe_style_title: get_tui_style! { @from_result: result_stylesheet , Id::DialogStyleNameTitle },
            maybe_style_editor: get_tui_style! { @from_result: result_stylesheet , Id::DialogStyleNameEditor },
            maybe_style_results_panel: get_tui_style! { @from_result: result_stylesheet , Id::DialogStyleNameResultsPanel },
            ..Default::default()
        };

        let editor_options = EditorEngineConfig {
            multiline_mode: LineMode::SingleLine,
            syntax_highlight: SyntaxHighlightMode::Disable,
            edit_mode: EditMode::ReadWrite,
        };

        let boxed_dialog_component = DialogComponent::new_boxed(
            FlexBoxId::from(Id::SimpleDialog),
            dialog_options,
            editor_options,
            handler_fn::on_dialog_press_handler,
            handler_fn::on_dialog_editor_change_handler,
        );

        ComponentRegistry::put(
            component_registry_map,
            FlexBoxId::from(Id::SimpleDialog),
            boxed_dialog_component,
        );

        DEBUG_TUI_MOD.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message =
                    "app_main construct DialogComponent (simple) [ on_dialog_press ]"
            );
        });
    }

    /// Insert autocomplete dialog component into registry if it's not already there.
    fn insert_dialog_component_autocomplete(
        component_registry_map: &mut ComponentRegistryMap<State, AppSignal>,
    ) {
        mod handler_fn {
            #[allow(clippy::wildcard_imports)]
            use super::*;

            pub fn on_dialog_press_handler(
                dialog_choice: DialogChoice,
                state: &mut State,
                _main_thread_channel_sender: &mut Sender<
                    TerminalWindowMainThreadSignal<AppSignal>,
                >,
            ) {
                match dialog_choice {
                    DialogChoice::Yes(text) => {
                        modal_dialogs::dialog_component_initialize_focused(
                            state,
                            FlexBoxId::from(Id::AutocompleteDialog),
                            "Yes".into(),
                            text,
                        );
                    }
                    DialogChoice::No => {
                        modal_dialogs::dialog_component_initialize_focused(
                            state,
                            FlexBoxId::from(Id::AutocompleteDialog),
                            "No".into(),
                            "".into(),
                        );
                    }
                }
            }

            pub fn on_dialog_editor_change_handler(
                state: &mut State,
                _main_thread_channel_sender: &mut Sender<
                    TerminalWindowMainThreadSignal<AppSignal>,
                >,
            ) {
                modal_dialogs::dialog_component_update_content(
                    state,
                    FlexBoxId::from(Id::AutocompleteDialog),
                );
            }
        }

        let result_stylesheet = stylesheet::create_stylesheet();

        let dialog_options = DialogEngineConfigOptions {
            mode: DialogEngineMode::ModalAutocomplete,
            maybe_style_border: get_tui_style! { @from_result: result_stylesheet , Id::DialogStyleNameBorder },
            maybe_style_title: get_tui_style! { @from_result: result_stylesheet , Id::DialogStyleNameTitle },
            maybe_style_editor: get_tui_style! { @from_result: result_stylesheet , Id::DialogStyleNameEditor },
            maybe_style_results_panel: get_tui_style! { @from_result: result_stylesheet , Id::DialogStyleNameResultsPanel },
            ..Default::default()
        };

        let editor_options = EditorEngineConfig {
            multiline_mode: LineMode::SingleLine,
            syntax_highlight: SyntaxHighlightMode::Disable,
            edit_mode: EditMode::ReadWrite,
        };

        let boxed_dialog_component = DialogComponent::new_boxed(
            FlexBoxId::from(Id::AutocompleteDialog),
            dialog_options,
            editor_options,
            handler_fn::on_dialog_press_handler,
            handler_fn::on_dialog_editor_change_handler,
        );

        ComponentRegistry::put(
            component_registry_map,
            FlexBoxId::from(Id::AutocompleteDialog),
            boxed_dialog_component,
        );

        DEBUG_TUI_MOD.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "app_main construct DialogComponent (autocomplete) [ on_dialog_press ]"
            );
        });
    }
}

mod stylesheet {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub fn create_stylesheet() -> CommonResult<TuiStylesheet> {
        throws_with_return!({
            tui_stylesheet! {
                new_style! {
                    id: {Id::EditorStyleNameDefault}
                    padding: {1}
                    // These are ignored due to syntax highlighting.
                    // attrib: [bold]
                    // color_fg: TuiColor::Blue
                },
                new_style! {
                    id: {Id::DialogStyleNameTitle}
                    lolcat
                    // These are ignored due to lolcat: true.
                    // attrib: [bold]
                    // color_fg: TuiColor::Yellow
                },
                new_style! {
                    id: {Id::DialogStyleNameBorder}
                    lolcat
                    // These are ignored due to lolcat: true.
                    // attrib: [dim]
                    // color_fg: TuiColor::Green
                },
                new_style! {
                    id: {Id::DialogStyleNameEditor}
                    bold
                    color_fg: {tui_color!(magenta)}
                },
                new_style! {
                    id: {Id::DialogStyleNameResultsPanel}
                    // bold
                    color_fg: {tui_color!(blue)}
                }
            }
        })
    }
}

mod hud {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub fn create_hud(pipeline: &mut RenderPipeline, size: Size, hud_report_str: &str) {
        let color_bg = tui_color!(hex "#fdb6fd");
        let color_fg = tui_color!(hex "#942997");
        let styled_texts = tui_styled_texts! {
            tui_styled_text! {
                @style: new_style!(dim color_fg: {color_fg} color_bg: {color_bg}),
                @text: hud_report_str
            },
        };
        let display_width = styled_texts.display_width();
        let col_idx = col(*(size.col_width - display_width) / 2);
        let row_idx = size.row_height.index_from_end(height(1)); /* 1 row above bottom */
        let cursor = col_idx + row_idx;

        let mut render_ops = RenderOpsIR::new();
        render_ops.push(RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(col(0) + row_idx)));
        render_ops.push(RenderOpIR::Common(RenderOpCommon::ResetColor));
        render_ops.push(RenderOpIR::Common(RenderOpCommon::SetBgColor(color_bg)));
        render_ops.push(RenderOpIR::PaintTextWithAttributes(
            SPACER_GLYPH.repeat(size.col_width.as_usize()).into(),
            None,
        ));
        render_ops.push(RenderOpIR::Common(RenderOpCommon::ResetColor));
        render_ops.push(RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(cursor)));
        render_tui_styled_texts_into(&styled_texts, &mut render_ops);
        pipeline.push(ZOrder::Normal, render_ops);
    }
}

mod status_bar {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Shows helpful messages at the bottom row of the screen.
    pub fn render_status_bar(pipeline: &mut RenderPipeline, size: Size) {
        let color_bg = tui_color!(hex "#076DEB");
        let color_fg = tui_color!(hex "#E9C940");
        let styled_texts = tui_styled_texts! {
            tui_styled_text! {
                @style: new_style!(bold dim color_fg: {color_fg} color_bg: {color_bg}),
                @text: "Hints: "
            },
            tui_styled_text! {
                @style: new_style!(dim underline color_fg: {color_fg} color_bg: {color_bg}),
                @text: "Ctrl + q"
            },
            tui_styled_text! {
                @style: new_style!(bold color_fg: {color_fg} color_bg: {color_bg}),
                @text: " : Exit 🖖"
            },
            tui_styled_text! {
                @style: new_style!(dim color_fg: {color_fg} color_bg: {color_bg}),
                @text: " … "
            },
            tui_styled_text! {
                @style: new_style!(dim underline color_fg: {color_fg} color_bg: {color_bg}),
                @text: "Ctrl + l"
            },
            tui_styled_text! {
                @style: new_style!(bold color_fg: {color_fg} color_bg: {color_bg}),
                @text: " : Simple 📣"
            },
            tui_styled_text! {
                @style: new_style!(dim color_fg: {color_fg} color_bg: {color_bg}),
                @text: " … "
            },
            tui_styled_text! {
                @style: new_style!(dim underline color_fg: {color_fg} color_bg: {color_bg}),
                @text: "Ctrl + k"
            },
            tui_styled_text! {
                @style: new_style!(bold color_fg: {color_fg} color_bg: {color_bg}),
                @text: " : Autocomplete 🤖"
            },
            tui_styled_text! {
                @style: new_style!(dim color_fg: {color_fg} color_bg: {color_bg}),
                @text: " … "
            },
            tui_styled_text! {
                @style: new_style!(underline color_fg: {color_fg} color_bg: {color_bg}),
                @text: "Type content 🌊"
            },
        };

        let display_width = styled_texts.display_width();
        let col_idx = col(*(size.col_width - display_width) / 2);
        let row_idx = size.row_height.convert_to_index(); /* Bottom row */
        let cursor = col_idx + row_idx;

        let mut render_ops = RenderOpsIR::new();
        render_ops.push(RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(col(0) + row_idx)));
        render_ops.push(RenderOpIR::Common(RenderOpCommon::ResetColor));
        render_ops.push(RenderOpIR::Common(RenderOpCommon::SetBgColor(color_bg)));
        render_ops.push(RenderOpIR::PaintTextWithAttributes(
            SPACER_GLYPH.repeat(size.col_width.as_usize()).into(),
            None,
        ));
        render_ops.push(RenderOpIR::Common(RenderOpCommon::ResetColor));
        render_ops.push(RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(cursor)));
        render_tui_styled_texts_into(&styled_texts, &mut render_ops);
        pipeline.push(ZOrder::Normal, render_ops);
    }
}
