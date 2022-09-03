# Design document for editor component, Aug 28 2022
<a id="markdown-design-document-for-editor-component%2C-aug-28-2022" name="design-document-for-editor-component%2C-aug-28-2022"></a>


<!-- TOC -->

- [Goal](#goal)
- [Timeline & features](#timeline--features)
- [Milestones](#milestones)
- [Resources](#resources)
- [Proposed solution - add an EditorEngine field to the EditorComponent and add an EditorBuffer field to the State](#proposed-solution---add-an-editorengine-field-to-the-editorcomponent-and-add-an-editorbuffer-field-to-the-state)
  - [Scope](#scope)
  - [Constraints](#constraints)
  - [Solution overview](#solution-overview)
- [Painting caret using cursor and another approach](#painting-caret-using-cursor-and-another-approach)
  - [**GlobalCursor** - Use the terminal's cursor show / hide.](#globalcursor---use-the-terminals-cursor-show--hide)
  - [**LocalPaintedEffect** - Paint the character at the cursor w/ the colors inverted or some](#localpaintedeffect---paint-the-character-at-the-cursor-w-the-colors-inverted-or-some)
  - [Using both](#using-both)

<!-- /TOC -->

## Goal
<a id="markdown-goal" name="goal"></a>


Create an editor component that is very similar to
[`micro` text editor](https://micro-editor.github.io/). But it must live in the tui layout engine:

1. Meaning that it can be fit in different shaped boxes in the main terminal window.
2. We can't assume that it will take up 100% height & width of the terminal since there are other UI
   components on the same "screen" / terminal window.

## Timeline & features
<a id="markdown-timeline-%26-features" name="timeline-%26-features"></a>


1. Editor component that can fit in a `TWBox` and is implemented as a `Component`. Example of a
   `Component`
   [`column_render_component.rs`](https://github.com/r3bl-org/r3bl-cmdr/blob/main/src/ex_app_with_layout/column_render_component.rs).
2. Supports editing but not saving.
3. Supports focus management, so there may be multiple editor components in a single `TWApp`.
4. Support unicode grapheme clusters (cursor navigation).
5. Rudimentary support for syntax highlighting (using an extensible backend to implement support for
   various languages / file formats).

Timeline is about 6 weeks.

## Milestones
<a id="markdown-milestones" name="milestones"></a>


1. Start building an example in `r3bl-cmdr` repo for editor component.
2. Create an app that has a 2 column layout, w/ a different editor component in each column.
3. Get the code solid in the example. Then migrate the code w/ tests into the `tui` module of
   `r3bl_rs_utils` repo.

## Resources
<a id="markdown-resources" name="resources"></a>


This is a great series of videos on syntax highlighting & unicode support in editors:

- https://www.youtube.com/playlist?list=PLP2yfE2-FXdQw0I6O4YdIX_mzBeF5TDdv
- There are a lot of videos here, but they're organized by topic
- Topics include: unicode handling, text wrapping, and syntax highlighting engines

Here are a few resources related to editors, and syntax highlighting:

- https://github.com/ndd7xv/heh
- https://docs.rs/syntect/latest/syntect/
- https://github.com/zee-editor/zee
- https://github.com/helix-editor/helix

Here are some other TUI frameworks:

- https://dioxuslabs.com/
- https://github.com/veeso/tui-realm/blob/main/docs/en/get-started.md

## Proposed solution - add an `EditorEngine` field to the `EditorComponent` and add an `EditorBuffer` field to the `State`
<a id="markdown-proposed-solution---add-an-editorengine-field-to-the-editorcomponent-and-add-an-editorbuffer-field-to-the-state" name="proposed-solution---add-an-editorengine-field-to-the-editorcomponent-and-add-an-editorbuffer-field-to-the-state"></a>


![editor_component drawio](https://raw.githubusercontent.com/r3bl-org/r3bl_rs_utils/main/docs/editor_component.drawio.svg)

### Scope
<a id="markdown-scope" name="scope"></a>


The goal is to create a reusable editor component. This example
[here](https://github.com/r3bl-org/r3bl-cmdr/tree/main/src/ex_editor) is a very simple application,
w/ a single column layout that takes up the full height & width of the terminal window. The goal is
to add a reusable editor component to this example to get the most basic editor functionality
created.

### Constraints
<a id="markdown-constraints" name="constraints"></a>


The application has a `State` and `Action` that are specific to the `AppWithLayout` (which
implements the `App<S,A>` trait).

1. The `launcher.rs` creates the store, and app itself, and passes it to `main_event_loop` to get
   everything started.
2. This means that the `EditorComponent` struct which implements `Component<S,A>` trait is actually
   directly coupled to the `App` itself. So something else has to be reusable, since
   `EditorComponent` can't be.

The `EditorComponent` struct might be a good place to start looking for possible solutions.

- This struct can hold data in its own memory. It already has a `Lolcat` struct inside of it.
- It also implements the `Component<S,A>` trait.
- However, for the reusable editor component we need the data representing the document being edited
  to be stored in the `State` and not inside of the `EditorComponent` itself (like the `lolcat`
  field).

### Solution overview
<a id="markdown-solution-overview" name="solution-overview"></a>


1. Add two new structs:

   1. `EditorEngine` -> **This goes in `EditorComponent`**
      - Contains the logic to process keypresses and modify an editor buffer.
   2. `EditorBuffer` -> **This goes in the `State`**
      - Contains the data that represents the document being edited. This can also contain the
        undo/redo history.

2. Here are the connection points w/ the impl of `Component<S,A>` in `EditorComponent`:
   1. `handle_event(input_event: &InputEvent, state: &S, shared_store: &SharedStore<S, A>)`
      - Can simply relay the arguments to `EditorEngine::apply(state.editor_buffer, input_event)`
        which will return another `EditorBuffer`.
      - Return value can be dispatched to the store via an action
        `UpdateEditorBuffer(EditorBuffer)`.
   2. `render(has_focus: &HasFocus, current_box: &FlexBox, state: &S, shared_store: &SharedStore<S, A>)`
      - Can simply relay the arguments to `EditorEngine::render(state.editor_buffer)`
      - Which will return a `TWCommandQueue`.

Sample code:

```rust
pub struct EditorEngine;
impl EditorEngine {
  fn async apply(
    editor_buffer: &EditorBuffer, input_event: &InputEvent
  ) -> EditorBuffer {
    todo!();
  }

  fn async render(
    editor_buffer: &EditorBuffer, has_focus: &HasFocus, current_box: &FlexBox
  ) -> TWCommandQueue {
    todo!();
  }
}

pub struct EditorBuffer {
  // TODO
}
```

These commits are related to the work described here:

1. [Add EditorEngine & EditorBuffer skeleton](https://github.com/r3bl-org/r3bl_rs_utils/commit/6dea59b68f90330b3e95639751f92a18bf28bee4)
2. [Add EditorEngine & EditorBuffer integration for editor component](https://github.com/r3bl-org/r3bl-cmdr/commit/1041c1f7cfee91f9ca0166384dabeb8fe6b21a01)

## Painting caret (using cursor and another approach)
<a id="markdown-painting-caret-using-cursor-and-another-approach" name="painting-caret-using-cursor-and-another-approach"></a>


> Definitions
>
> **`Caret`** - the block that is visually displayed in a terminal which represents the insertion
> point for whatever is in focus. While only one insertion point is editable for the local user,
> there may be multiple of them, in which case there has to be a way to distinguish a local caret
> from a remote one (this can be done w/ bg color).
>
> **`Cursor`** - the global "thing" provided in terminals that shows by blinking usually where the
> cursor is. This cursor is moved around and then paint operations are performed on various
> different areas in a terminal window to paint the output of render operations.

There are two ways of showing cursors which are quite different (each w/ very different
constraints).

### 1. **`GlobalCursor`** - Use the terminal's cursor show / hide.
<a id="markdown-**globalcursor**---use-the-terminal's-cursor-show-%2F-hide." name="**globalcursor**---use-the-terminal's-cursor-show-%2F-hide."></a>


1.  Both [termion::cursor](https://docs.rs/termion/1.5.6/termion/cursor/index.html) and
    [crossterm::cursor](https://docs.rs/crossterm/0.25.0/crossterm/cursor/index.html) support this.
    The cursor has lots of effects like blink, etc.
2.  The downside is that there is one global cursor for any given terminal window. And this cursor
    is constantly moved around in order to paint anything (eg:
    `MoveTo(col, row), SetColor, PaintText(...)` sequence).
3.  So it must be guaranteed by
    [TWCommandQueue via TWCommand::ShowCaretAtPosition???To(...)](https://github.com/r3bl-org/r3bl_rs_utils/blob/main/src/tui/crossterm_helpers/tw_command.rs#L171).
    The downside here too is that there's a chance that different components and render functions
    will clobber this value that's already been set. There's currently a weak warning that's
    displayed after the 1st time this value is set which isn't robust either.
4.  This is what that code looks like:
    ```rust
    // Approach 1 - using cursor show / hide.
    tw_command_queue! {
      queue push
      TWCommand::ShowCaretAtPositionRelTo(box_origin_pos, editor_buffer.caret)
    };
    ```

### 2. **`LocalPaintedEffect`** - Paint the character at the cursor w/ the colors inverted (or some
<a id="markdown-**localpaintedeffect**---paint-the-character-at-the-cursor-w%2F-the-colors-inverted-or-some" name="**localpaintedeffect**---paint-the-character-at-the-cursor-w%2F-the-colors-inverted-or-some"></a>


other bg color) giving the visual effect of a cursor.

1.  This has the benefit that we can display multiple cursors in the app, since this is not global,
    rather it is component specific. For the use case requiring google docs style multi user editing
    where multiple cursors need to be shown, this approach can be used in order to implement that.
    Each user for eg can get a different caret background color to differentiate their caret from
    others.
2.  The downside is that it isn't possible to blink the cursor or have all the other "standard"
    cursor features that are provided by the actual global cursor (discussed above).
3.  This is what that code looks like:
    ```rust
    // Approach 2 - painting the editor_buffer.caret position w/ reverse.
    tw_command_queue! {
      queue push
      TWCommand::MoveCursorPositionRelTo(box_origin_pos, editor_buffer.caret),
      TWCommand::PrintWithAttributes(
        editor_buffer.get_char_at_caret().unwrap_or(DEFAULT_CURSOR_CHAR).into(),
        style! { attrib: [reverse] }.into()),
      TWCommand::MoveCursorPositionRelTo(box_origin_pos, editor_buffer.caret)
    };
    ```

### Using both
<a id="markdown-using-both" name="using-both"></a>


It might actually be necessary to use both `GlobalCursor` and `LocalPaintedEffect` approaches
simultaneously.

1. `GlobalCursor` might be used to show the local user's caret since it blinks, etc
2. `LocalPaintedEffect` might be used to show remote user's carets since it doesn't blink and
   supports a multitude of background colors that can be applied to distinguish users.
