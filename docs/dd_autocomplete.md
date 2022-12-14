---
Title: Autocompletion provider design document
Date: 2022-12-14
---

<!-- TOC -->

- [How does layout, rendering, and event handling work in general?](#how-does-layout-rendering-and-event-handling-work-in-general)
- [How do dialog boxes work?](#how-do-dialog-boxes-work)
- [Making HTTP request to API](#making-http-request-to-api)
- [How to add editor component to bottom of dialog box](#how-to-add-editor-component-to-bottom-of-dialog-box)

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

1. It paints on top of the entire screen.
2. Is activated by a keyboard shortcut.

So this activation trigger must be done at the `App` trait impl level (in the `app_handle_event()`
method). Also, when this trigger is detected it has to:

1. Set the focus to the dialog box, so that it will appear on the next render. When trigger is
   detected it will return a `EventPropagation::Consumed` which won't force a render.
2. Set the title and text via a dispatch of the action `SetDialogBoxTitleAndText`. This will force a
   render, and the title and text in the dialog box on next render.

## Making HTTP request to API
<a id="markdown-making-http-request-to-api" name="making-http-request-to-api"></a>


## How to add editor component to bottom of dialog box
<a id="markdown-how-to-add-editor-component-to-bottom-of-dialog-box" name="how-to-add-editor-component-to-bottom-of-dialog-box"></a>

