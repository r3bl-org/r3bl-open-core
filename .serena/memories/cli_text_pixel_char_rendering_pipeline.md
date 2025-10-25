# CliTextInline → PixelChar → ANSI Rendering Pipeline

## The Hidden Pipeline in `cli_text_inline()`

When `CliTextInline` is converted to a String via `.to_string()` or `println!("{}")`, it triggers a **hidden PixelChar conversion**:

### Call Chain

1. `CliTextInline::to_string()` calls Display implementation
2. Display calls `FastStringify::write_to_buf()` (line 908-926 of cli_text.rs)
3. Inside `write_to_buf()`:
   ```rust
   let pixels = self.convert(CliTextConvertOptions::default());  // Line 911
   // ↓ Converts CliTextInline → InlineVec<PixelChar>
   
   let mut renderer = PixelCharRenderer::new();  // Line 914
   // ↓ Creates ANSI renderer
   
   let ansi_output = renderer.render_line(&pixels);  // Line 915
   // ↓ Converts PixelChar[] → ANSI bytes
   
   acc.push_str(std::str::from_utf8(ansi_output)?);  // Line 919
   // ↓ Writes UTF-8 ANSI codes to output
   
   SgrCode::Reset.write_to_buf(acc)?;  // Line 922
   // ↓ Emits final reset code
   ```

## Key Methods

### `CliTextInline::convert()` (line 296-350)
- **Input**: `CliTextInline` (text + TuiStyle)
- **Output**: `InlineVec<PixelChar>` (array of styled characters)
- **Process**:
  1. Create TuiStyle from struct fields
  2. Use GCStringOwned for display-width-aware clipping
  3. For each grapheme cluster: Create `PixelChar::PlainText { display_char, style }`

### `PixelCharRenderer::render_line()` (pixel_char_renderer.rs:127)
- **Input**: `&[PixelChar]` (array of styled characters)
- **Output**: `&[u8]` (ANSI escape sequences + character bytes)
- **Smart style diffing**: Only emits ANSI codes when style changes
- **Example output**: `"\x1b[1mH\x1b[0mi"` (bold H, reset, normal i)

## Why This Design?

This architecture allows unified ANSI generation across multiple contexts:

1. **CliTextInline → String**: For interactive UI (choose, readline) - converts on-demand
2. **OffscreenBuffer → Terminal**: For full screen rendering - converts per frame
3. **RenderOps → Terminal**: For component rendering - uses same renderer

## Integration Points (from pixel_char_renderer.rs docs, line 67-70)

- `OffscreenBuffer::render_to_ansi()` → will call this renderer
- `CliTextInline::Display` → **uses this renderer** (line 908-926)
- `choose()` and `readline_async` → **use this renderer** (indirectly via CliTextInline)
- `RenderOp::PaintTextWithAttributes` → uses this renderer

## Rendering Path for `choose()` and `readline()`

```
choose() renders header/items
    ↓
cli_text_inline(&text, style).to_string()
    ↓
CliTextInline::FastStringify::write_to_buf()
    ↓
self.convert() → InlineVec<PixelChar>
    ↓
PixelCharRenderer::new() → renderer.render_line(&pixels)
    ↓
Style diffing + ANSI code generation
    ↓
ANSI escape sequence bytes
    ↓
queue!(Print(ansi_string))
    ↓
stdout
```

## Comparison: What Actually Happens

| Step | Old Understanding | Actual Implementation |
|------|-------------------|----------------------|
| Create styled text | String with ANSI codes | CliTextInline struct (text + TuiStyle) |
| Conversion to output | Direct ANSI | Convert → PixelChar[] → PixelCharRenderer |
| ANSI generation | Embedded in string | PixelCharRenderer applies style diffing |
| Final output | ANSI string | ANSI bytes from renderer |

## Key Finding

**Yes, `cli_text_inline()` DOES use PixelChar conversion**, but:
- ✅ It's **hidden** in the `FastStringify::write_to_buf()` implementation
- ✅ Only happens **on-demand** when converting to String (lazy conversion)
- ✅ Uses **PixelCharRenderer** for smart style diffing
- ✅ Same unified renderer as full RenderOps pipeline
- ✅ Not pre-converted; conversion happens at display time

This allows reusing the same high-quality ANSI generation logic across all rendering paths.
