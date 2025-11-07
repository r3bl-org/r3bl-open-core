# List Component Example

This example demonstrates the R3BL TUI list component with both **Phase 1 (Simple)** and **Phase 3 (Complex)** rendering modes.

## Features Demonstrated

### Phase 1: Simple List Items
- Single-line text rendering (fast path)
- Simple TodoItem with checkbox, priority, and status
- Keyboard shortcuts: `t` (toggle), `p` (cycle priority)
- Batch operations: `d` (delete), `c` (complete), `h` (set high priority)

### Phase 3: Complex List Items
- Multi-line FlexBox-based rendering with automatic layout
- Complex ProjectItem with:
  - Header bar with project name and status badge
  - Indented description line
  - Progress bar with task counter
- Keyboard shortcuts: `s` (cycle status), `+/-` (adjust progress)
- Batch operations: `d` (delete), `c` (complete projects)
- **Automatic FlexBoxId pooling** - IDs are dynamically assigned as items scroll into view and recycled when they scroll out

## Running the Example

```bash
cargo run --example tui_apps
# Select "List Component" from the menu
```

## Key Bindings

### Global
- **`m`** - Toggle between SIMPLE and COMPLEX mode
- **`↑/↓`** - Navigate items
- **`Space`** - Select/deselect items (multi-select)
- **`q`** - Quit

### Simple Mode (TodoItem)
- **`t`** - Toggle item completion
- **`p`** - Cycle priority (Low → Medium → High)
- **`d`** - Delete selected items (batch)
- **`c`** - Complete selected items (batch)
- **`h`** - Set high priority (batch)

### Complex Mode (ProjectItem)
- **`s`** - Cycle project status (Planning → InProgress → Review → Completed)
- **`+`** - Increase progress by 10%
- **`-`** - Decrease progress by 10%
- **`d`** - Delete selected projects (batch)
- **`c`** - Complete selected projects (batch)

## Architecture Highlights

### Simple Items (Phase 1)
```rust
impl SimpleListItem<AppState, AppSignal> for TodoItem {
    fn render_line(...) -> String {
        // Fast string-based rendering
    }

    fn handle_event(...) -> EventPropagation {
        // Handle single-item events
    }
}
```

### Complex Items (Phase 3)
```rust
impl ComplexListItem<AppState, AppSignal> for ProjectItem {
    fn render_as_component(...) -> RenderPipeline {
        // Full FlexBox rendering with nested layouts
        // Each item takes 3 rows with hierarchical information
    }

    fn get/set_flexbox_id(...) {
        // FlexBoxId pooling managed automatically by ListComponent
    }
}
```

### Virtual Scrolling for FlexBoxIds

The complex list uses a **FlexBoxId pool** to efficiently manage rendering resources:

```
List with 1000 items, viewport shows ~7:

ListItemId space (u64):      FlexBoxId pool (u8):
┌────────────┐              ┌──────┐
│ Item #0    │              │ ID 3 │ ← Assigned to visible item
│ Item #1    │              │ ID 4 │ ← Assigned to visible item
│   ...      │              │  ... │
│ Item #100  │ ← visible    │ ID 9 │ ← Assigned to visible item
│ Item #101  │ ← visible    ├──────┤
│   ...      │ ← visible    │ ID 10│ ← Free (in pool)
│ Item #106  │ ← visible    │ ID 11│ ← Free (in pool)
│ Item #107  │              └──────┘
│   ...      │
│ Item #999  │
└────────────┘

Pool size = viewport_height + 5 (buffer)
IDs recycled automatically on scroll
```

## Code Structure

- **`state.rs`** - Application state with DisplayMode enum
- **`todo_item.rs`** - Simple list item (Phase 1)
- **`project_item.rs`** - Complex list item with FlexBox layouts (Phase 3)
- **`app_main.rs`** - Application logic with mode toggling
- **`launcher.rs`** - Entry point

## What This Demonstrates

1. **Clean trait hierarchy**: `ListItem` → `SimpleListItem` / `ComplexListItem`
2. **Performance**: Simple items for speed, complex items for power
3. **Type safety**: Compiler prevents mixing rendering approaches
4. **Resource efficiency**: Only visible items consume FlexBoxIds
5. **Automatic management**: Pool handles ID assignment/recycling
6. **Batch operations**: Multi-select with custom actions
7. **State integration**: Items can modify application state

## Learning Path

1. Start in **SIMPLE mode** to see basic list functionality
2. Press **`m`** to switch to **COMPLEX mode**
3. Observe how complex items render across multiple lines
4. Use batch operations (`Space` + action key) on multiple items
5. Scroll to see automatic FlexBoxId recycling (transparent to user)
6. Review source code to understand trait implementations

---

This example serves as both a working demo and a template for building your own list-based UIs with R3BL TUI.
