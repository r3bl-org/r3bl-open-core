---
Title: Design document for compositor
Date: 2022-11-05
Status: Copied to tui/src/lib.rs, tui/README.md
---

<!-- TOC -->

- [Rendering and painting](#rendering-and-painting)
  - [Offscreen buffer](#offscreen-buffer)
  - [Render pipeline](#render-pipeline)
  - [First render](#first-render)
  - [Subsequent render](#subsequent-render)

<!-- /TOC -->

## Rendering and painting
<a id="markdown-rendering-and-painting" name="rendering-and-painting"></a>


The R3BL TUI engine uses a high performance compositor to render the UI to the terminal. This
ensures that only "pixels" that have changed are painted to the terminal. This is done by creating a
concept of `PixelChar` which represents a single "pixel" in the terminal screen at a given col and
row index position. There are only as many `PixelChar`s as there are rows and cols in a terminal
screen. And the index maps directly to the position of the pixel in the terminal screen.

### Offscreen buffer
<a id="markdown-offscreen-buffer" name="offscreen-buffer"></a>


Here is an example of what a single row of rendered output might look like in a row of the
`OffscreenBuffer`. This diagram shows each `PixelChar` in `row_index: 1` of the `OffscreenBuffer`.
In this example, there are 80 columns in the terminal screen. This actual log output generated by
the TUI engine when logging is enabled.

```text
row_index: 1
000 S ░░░░░░░╳░░░░░░░░001 P    'j'→fg‐bg    002 P    'a'→fg‐bg    003 P    'l'→fg‐bg    004 P    'd'→fg‐bg    005 P    'k'→fg‐bg
006 P    'f'→fg‐bg    007 P    'j'→fg‐bg    008 P    'a'→fg‐bg    009 P    'l'→fg‐bg    010 P    'd'→fg‐bg    011 P    'k'→fg‐bg
012 P    'f'→fg‐bg    013 P    'j'→fg‐bg    014 P    'a'→fg‐bg    015 P     '▒'→rev     016 S ░░░░░░░╳░░░░░░░░017 S ░░░░░░░╳░░░░░░░░
018 S ░░░░░░░╳░░░░░░░░019 S ░░░░░░░╳░░░░░░░░020 S ░░░░░░░╳░░░░░░░░021 S ░░░░░░░╳░░░░░░░░022 S ░░░░░░░╳░░░░░░░░023 S ░░░░░░░╳░░░░░░░░
024 S ░░░░░░░╳░░░░░░░░025 S ░░░░░░░╳░░░░░░░░026 S ░░░░░░░╳░░░░░░░░027 S ░░░░░░░╳░░░░░░░░028 S ░░░░░░░╳░░░░░░░░029 S ░░░░░░░╳░░░░░░░░
030 S ░░░░░░░╳░░░░░░░░031 S ░░░░░░░╳░░░░░░░░032 S ░░░░░░░╳░░░░░░░░033 S ░░░░░░░╳░░░░░░░░034 S ░░░░░░░╳░░░░░░░░035 S ░░░░░░░╳░░░░░░░░
036 S ░░░░░░░╳░░░░░░░░037 S ░░░░░░░╳░░░░░░░░038 S ░░░░░░░╳░░░░░░░░039 S ░░░░░░░╳░░░░░░░░040 S ░░░░░░░╳░░░░░░░░041 S ░░░░░░░╳░░░░░░░░
042 S ░░░░░░░╳░░░░░░░░043 S ░░░░░░░╳░░░░░░░░044 S ░░░░░░░╳░░░░░░░░045 S ░░░░░░░╳░░░░░░░░046 S ░░░░░░░╳░░░░░░░░047 S ░░░░░░░╳░░░░░░░░
048 S ░░░░░░░╳░░░░░░░░049 S ░░░░░░░╳░░░░░░░░050 S ░░░░░░░╳░░░░░░░░051 S ░░░░░░░╳░░░░░░░░052 S ░░░░░░░╳░░░░░░░░053 S ░░░░░░░╳░░░░░░░░
054 S ░░░░░░░╳░░░░░░░░055 S ░░░░░░░╳░░░░░░░░056 S ░░░░░░░╳░░░░░░░░057 S ░░░░░░░╳░░░░░░░░058 S ░░░░░░░╳░░░░░░░░059 S ░░░░░░░╳░░░░░░░░
060 S ░░░░░░░╳░░░░░░░░061 S ░░░░░░░╳░░░░░░░░062 S ░░░░░░░╳░░░░░░░░063 S ░░░░░░░╳░░░░░░░░064 S ░░░░░░░╳░░░░░░░░065 S ░░░░░░░╳░░░░░░░░
066 S ░░░░░░░╳░░░░░░░░067 S ░░░░░░░╳░░░░░░░░068 S ░░░░░░░╳░░░░░░░░069 S ░░░░░░░╳░░░░░░░░070 S ░░░░░░░╳░░░░░░░░071 S ░░░░░░░╳░░░░░░░░
072 S ░░░░░░░╳░░░░░░░░073 S ░░░░░░░╳░░░░░░░░074 S ░░░░░░░╳░░░░░░░░075 S ░░░░░░░╳░░░░░░░░076 S ░░░░░░░╳░░░░░░░░077 S ░░░░░░░╳░░░░░░░░
078 S ░░░░░░░╳░░░░░░░░079 S ░░░░░░░╳░░░░░░░░080 S ░░░░░░░╳░░░░░░░░spacer [ 0, 16-80 ]
```

When `RenderOps` are executed and used to create an `OffscreenBuffer` that maps to the size of the
terminal window, clipping is performed automatically. This means that it isn't possible to move the
caret outside of the bounds of the viewport (terminal window size). And it isn't possible to paint
text that is larger than the size of the offscreen buffer. The buffer really represents the current
state of the viewport. Scrolling has to be handled by the component itself (an example of this is
the editor component).

Each `PixelChar` can be one of 4 things:

1. **Space**. This is just an empty space. There is no flickering in the TUI engine. When a new
   offscreen buffer is created, it is fulled w/ spaces. Then components paint over the spaces. Then
   the diffing algorithm only paints over the pixels that have changed. You don't have to worry
   about clearing the screen and painting, which typically will cause flickering in terminals. You
   also don't have to worry about printing empty spaces over areas that you would like to clear
   between renders. All of this handled by the TUI engine.
2. **Void**. This is a special pixel that is used to indicate that the pixel should be ignored. It
   is used to indicate a wide emoji is to the left somewhere. Most terminals don't support emojis,
   so there's a discrepancy between the display width of the character and its index in the string.
3. **Plain text**. This is a normal pixel which wraps a single character that maybe a grapheme
   cluster segment. Styling information is encoded in each `PixelChar::PlainText` and is used to
   paint the screen via the diffing algorithm which is smart enough to "stack" styles that appear
   beside each other for quicker rendering in terminals.
4. **ANSI text**. Styling information in not available w/ these characters because the styling
   information is encoded in the ANSI escape codes. `lolcat_api.rs` generates these ANSI strings for
   the rainbow effect. An example of this is the outline around a modal dialog box.

### Render pipeline
<a id="markdown-render-pipeline" name="render-pipeline"></a>


The following diagram provides a high level overview of how apps (that contain components, which may
contain components, and so on) are rendered to the terminal screen.

![](compositor.svg)

Each component produces a `RenderPipeline`, which is a map of `ZOrder` and `Vec<RenderOps>`.
`RenderOps` are the instructions that are grouped together, such as move the caret to a position,
set a color, and paint some text.

Inside of each `RenderOps` the caret is stateful, meaning that the caret position is remembered
after each `RenderOp` is executed. However, once a new `RenderOps` is executed, the caret position
reset just for that `RenderOps`. Caret position is not stored globally. You should read more about
"atomic paint operations" in the `RenderOp` documentation.

Once a set of these `RenderPipeline`s have been generated, typically after the user enters some
input event, and that produces a new state which then has to be rendered, they are combined and
painted into an `OffscreenBuffer`.

### First render
<a id="markdown-first-render" name="first-render"></a>


The `paint.rs` file contains the `paint` function, which is the entry point for all rendering. Once
the first render occurs, the `OffscreenBuffer` that is generated is saved to `GlobalSharedState`.
The following table shows the various tasks that have to be performed in order to render to an
`OffscreenBuffer`. There is a different code path that is taken for ANSI text and plain text (which
includes `StyledText` which is just plain text with a color). Syntax highlighted text is also just
`StyledText`. The ANSI text is an example of text that is generated by the `lolcat_api.rs`.

| UTF-8 | ANSI | Task                                                                                                    |
| ----- | ---- | ------------------------------------------------------------------------------------------------------- |
| Y     | Y    | convert `RenderPipeline` to `List<List<PixelChar>>` (`OffscreenBuffer`)                                 |
| Y     | Y    | paint each `PixelChar` in `List<List<PixelChar>>` to stdout using `OffscreenBufferPainterImplCrossterm` |
| Y     | Y    | save the `List<List<PixelChar>>` to `GlobalSharedState`                                                 |

Currently only `crossterm` is supported for actually painting to the terminal. But this process is
really simple making it very easy to swap out other terminal libraries such as `termion`, or even a
GUI backend, or some other custom output driver.

### Subsequent render
<a id="markdown-subsequent-render" name="subsequent-render"></a>


Since the `OffscreenBuffer` is cached in `GlobalSharedState` a diff to be performed for subsequent
renders. And only those diff chunks are painted to the screen. This ensures that there is no flicker
when the content of the screen changes. It also minimizes the amount of work that the terminal or
terminal emulator has to do put the `PixelChar`s on the screen.
