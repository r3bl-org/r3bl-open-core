---
Title: Autocompletion provider design document
Date: 2022-12-14
---

<!-- TOC -->

- [How does layout, rendering, and event handling work in general?](#how-does-layout-rendering-and-event-handling-work-in-general)
- [How do dialog boxes work?](#how-do-dialog-boxes-work)
  - [Two callback functions](#two-callback-functions)
- [How to use this dialog to make an HTTP request & pipe the results into a selection area?](#how-to-use-this-dialog-to-make-an-http-request--pipe-the-results-into-a-selection-area)
- [Implementation details of the autocomplete mode](#implementation-details-of-the-autocomplete-mode)
- [How to make HTTP requests](#how-to-make-http-requests)

<!-- /TOC -->

## How does layout, rendering, and event handling work in general?
<a id="markdown-how-does-layout%2C-rendering%2C-and-event-handling-work-in-general%3F" name="how-does-layout%2C-rendering%2C-and-event-handling-work-in-general%3F"></a>


- The `App` trait impl is the main entry point for laying out the entire application. And this is
  where the `component_registry` lives and all the `Component`s are created and added to the
  registry.
- When an `App` trait impl is created by a call to `App::new_shared()`, then the `init()` method is
  called, which should populate the `component_registry` with all the `Component`s that will be used
  in the application.
- This sets everything up so that `app_render()` and `app_handle_event()` can be called at a later
  time.
- The `app_render()` method is responsible for creating the layout by using `Surface` and `FlexBox`
  to arrange whatever `Component`s are in the `component_registry`.
- The `app_handle_event()` method is responsible for handling events that are sent to the `App`
  trait when user input is detected from the keyboard or mouse.

![](https://raw.githubusercontent.com/r3bl-org/r3bl_rs_utils/main/docs/memory-architecture.drawio.svg)

## How do dialog boxes work?
<a id="markdown-how-do-dialog-boxes-work%3F" name="how-do-dialog-boxes-work%3F"></a>


A modal dialog box is different than a normal reusable component. This is because:

1. It paints on top of the entire screen (in front of all other components, in ZOrder::Glass, and
   outside of any layouts using `FlexBox`es).
2. Is "activated" by a keyboard shortcut (hidden otherwise). Once activated, the user can accept or
   cancel the dialog box. And this results in a callback being called w/ the result.

So this activation trigger must be done at the `App` trait impl level (in the `app_handle_event()`
method). Also, when this trigger is detected it has to:

1. Set the focus to the dialog box, so that it will appear on the next render. When trigger is
   detected it will return a `EventPropagation::Consumed` which won't force a render.
2. Set the title and text via a dispatch of the action `SetDialogBoxTitleAndText`. This will force a
   render, and the title and text in the dialog box on next render.

There is a question about where does the response from the user (once a dialog is shown) go? This
seems as though it would be different in nature from an `EditorComponent` but it is the same. Here's
why:

- The `EditorComponent` is always updating its buffer based on user input, and there's no "handler"
  for when the user performs some action on the editor. The editor needs to save all the changes to
  the buffer to the state. This requires the trait bound `HasEditorBuffers` to be implemented by the
  state.
- The dialog box seems different in that you would think that it doesn't always updating its state
  and that the only time we really care about what state the dialog box has is when the user has
  accepted something they've typed into the dialog box and this needs to be sent to the callback
  function that was passed in when the component was created. However, due to the reactive nature of
  the TUI engine, even before the callback is called (due to the user accepting or cancelling),
  while the user is typing things into the dialog box, it has to be updating the state, otherwise,
  re-rendering the dialog box won't be triggered and the user won't see what they're typing. This
  means that even intermediate information needs to be recorded into the state via the
  `HasDialogBuffers` trait bound. This will hold stale data once the dialog is dismissed or
  accepted, but that's ok since the title and text should always be set before it is shown.
  - **Note**: it might be possible to save this type of intermediate data in
    `ComponentRegistry::user_data`. And it is possible for `handle_event()` to return a
    `EventPropagation::ConsumedRerender` to make sure that changes are re-rendered. This approach
    may have other issues related to having both immutable and mutable borrows at the same time to
    some portion of the component registry if one is not careful.

### Two callback functions
<a id="markdown-two-callback-functions" name="two-callback-functions"></a>


When creating a new dialog box component, two callback functions are passed in:

1. `on_dialog_press_handler()` - this will be called if the user choose no, or yes (w/ their typed
   text).
2. `on_dialog_editors_changed_handler()` - this will be called if the user types something into the
   editor.

## How to use this dialog to make an HTTP request & pipe the results into a selection area?
<a id="markdown-how-to-use-this-dialog-to-make-an-http-request-%26-pipe-the-results-into-a-selection-area%3F" name="how-to-use-this-dialog-to-make-an-http-request-%26-pipe-the-results-into-a-selection-area%3F"></a>


So far we have covered the use case for a simple modal dialog box. In order to provide
auto-completion capabilities, via some kind of web service, there needs to be a slightly more
complex version of this. This is where the `DialogEngineConfigOptions` struct comes in. It allows us
to create a dialog component and engine to be configured w/ the appropriate mode - simple or
autocomplete.

In autocomplete mode, an extra "results panel" is displayed, and the layout of the dialog is
different on the screen. Instead of being in the middle of the screen, it starts at the top of the
screen. The callbacks are the same.

## Implementation details of the autocomplete mode
<a id="markdown-implementation-details-of-the-autocomplete-mode" name="implementation-details-of-the-autocomplete-mode"></a>


**dialog_engine_api.rs**

- [ ] `apply_event()` - called by `DialogComponent::handle_event()`
  - [ ] up / down handler to navigate the results panel (if in autocomplete mode)
    - up / down will change the following in the `dialog_engine` data (**not state**)
      - `selected_row_index` - tracks the selected row
        - used for `render`, `on_dialog_press_handler`
      - `scroll_offset_row_index` - tracks scroll offset
        - you can get viewport from `DialogEngine::flex_box`, saved by `DialogComponent::render()`
        - if viewport is smaller than num results then scroll offset is applied
    - this code path should return an `ConsumedRender` so that the results panel is re-rendered
- [ ] add `render_results_panel()` to display results panel (if in autocomplete mode)
  - `DialogBuffer.results` is saved in the **state** & gets passed in here
  - paint the `Vec<String>` in the panel
  - paint the selected row
  - deal w/ `scroll_offset_row_index`
  - [x] actually call `make_flex_box_for_dialog()`
    - [x] drop the `current_box` arg
    - [x] pass the mode (`self.dialog_engine.dialog_options.mode`) into `make_flex_box_for_dialog()`
      - this `flex_box` is used later by the `DialogEngineApi::apply_event()` to perform scrolling
    - [x] save flex box to `dialog_engine.maybe_flex_box`
- [ ] `make_flex_box_for_dialog()`
  - [x] pass arg into the function
    - `DialogEngineMode` - this is the mode of the dialog engine (normal or autocomplete)
  - [x] make the `Surface` arg optional (since it won't be passed in)
  - [x] this should just be a private function, in `internal_impl`, remove from `DialogEngineApi`
  - [ ] based on the mode (normal / autocomplete) generate the correct flex box
- [ ] `render_engine()` - called by `DialogComponent::render()`
  - [ ] called `render_results_panel()`

**dialog_buffer.rs**

- [ ] add `results` field

**dialog_engine.rs**

- [ ] add `selected_row_index: ChUnit`
- [ ] add `scroll_offset_row_index: ChUnit`
- [x] add `maybe_flex_box: Option<FlexBox>` & document it

**app.rs**

- [ ] add a new dialog component w/ autocomplete (copy `insert_dialog_component()`)
  - [ ] `on_dialog_press_handler` - add result to editor by dispatching action
  - [ ] `on_dialog_editor_change_handler` -
    - [ ] call web service
    - [ ] get results
    - [ ] dispatch action to update results panel
      - the results are added to `Action::SetDialogBufferResults(FlexBoxId, Vec<String>`
- [ ] add new keyboard shortcut to show autocomplete dialog (<kbd>ctrl + k</kbd>)
  - [ ] change `try_input_event_activate_modal()`
  - [ ] change status bar to show this shortcut
- [x] `ContainerSurfaceRender::render_in_surface()` can't call `make_flex_box_for_dialog()`
  - Pass in `Default::default()` knowing that it will be ignored by the dialog component

**reducer.rs**

- [ ] handle `SetDialogBufferResults(FlexBoxId, Vec<String>` case in `run()`
  - add the results to the state (in `DialogBuffer`)

**dialog_component.rs**

- [x] `render()` - ignore the 3rd arg (`current_box: FlexBox`) & document it
- [x] `new_shared()` - add `DialogEngineConfigOptions` arg
- [x] `new()` - add `DialogEngineConfigOptions` arg & save to `self.dialog_engine: DialogEngine`

## How to make HTTP requests
<a id="markdown-how-to-make-http-requests" name="how-to-make-http-requests"></a>


Instead of using the `reqwest` crate, we should use the `hyper` crate (which is part of Tokio) and
drop support for `reqwest` in all our crates.

- https://blessed.rs/crates#section-networking-subsection-http-foundations
