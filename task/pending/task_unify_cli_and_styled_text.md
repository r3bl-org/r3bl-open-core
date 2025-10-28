<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Task: Consolidate cli_text and tui_styled_text](#task-consolidate-cli_text-and-tui_styled_text)
  - [Overview](#overview)
    - [Motivation](#motivation)
    - [Current State](#current-state)
    - [Target Architecture](#target-architecture)
  - [Architectural Analysis](#architectural-analysis)
    - [CliTextInline (CLI Context)](#clitextinline-cli-context)
    - [TuiStyledText (TUI Context)](#tuistyledtext-tui-context)
    - [Why Consolidation Works](#why-consolidation-works)
    - [Convergence Points](#convergence-points)
  - [Consolidation Benefits](#consolidation-benefits)
    - [Maintenance](#maintenance)
    - [Consistency](#consistency)
    - [Extensibility](#extensibility)
  - [Trade-offs](#trade-offs)
- [Implementation plan](#implementation-plan)
  - [Step 1: Extend TuiStyledText with Builder API [PENDING] (3-4 hours)](#step-1-extend-tuistyledtext-with-builder-api-pending-3-4-hours)
    - [1.1 Add Builder Methods to TuiStyledText](#11-add-builder-methods-to-tuistyledtext)
    - [1.2 Create Constructor Functions Module](#12-create-constructor-functions-module)
    - [1.3 Add `styled_text()` Function (CLI-Compatible)](#13-add-styled_text-function-cli-compatible)
    - [1.4 Add to_pixel_chars() Method](#14-add-to_pixel_chars-method)
    - [1.5 Update Module Exports](#15-update-module-exports)
  - [Step 2: Add Compatibility Layer [PENDING] (1 hour)](#step-2-add-compatibility-layer-pending-1-hour)
    - [2.1 Create Type Alias](#21-create-type-alias)
    - [2.2 Create Compatibility Function](#22-create-compatibility-function)
    - [2.3 Re-export Constructor Functions](#23-re-export-constructor-functions)
  - [Step 3: Migrate Call Sites [PENDING] (5-7 hours)](#step-3-migrate-call-sites-pending-5-7-hours)
    - [3.1 Type Signature Updates](#31-type-signature-updates)
    - [3.2 Function Call Updates](#32-function-call-updates)
    - [3.3 Incremental Validation](#33-incremental-validation)
  - [Step 4: Remove Old Code [PENDING] (2 hours)](#step-4-remove-old-code-pending-2-hours)
    - [4.1 Verify No Remaining Uses](#41-verify-no-remaining-uses)
    - [4.2 Remove Implementation Code](#42-remove-implementation-code)
    - [4.3 Update Module Exports](#43-update-module-exports)
    - [4.4 Update Imports in Paint Implementations](#44-update-imports-in-paint-implementations)
  - [Step 5: Testing & Validation [PENDING] (2-3 hours)](#step-5-testing--validation-pending-2-3-hours)
    - [5.1 Unit Tests](#51-unit-tests)
    - [5.2 Integration Tests](#52-integration-tests)
    - [5.3 Documentation Tests](#53-documentation-tests)
    - [5.4 Code Quality](#54-code-quality)
    - [5.5 Performance Validation](#55-performance-validation)
  - [Step 6: SmallString Optimization [PENDING] (1-2 hours)](#step-6-smallstring-optimization-pending-1-2-hours)
    - [Option A: Use SmallVec-Based String](#option-a-use-smallvec-based-string)
    - [Option B: Use InlineString (if better fit)](#option-b-use-inlinestring-if-better-fit)
    - [Phase 6 Implementation:](#phase-6-implementation)
  - [Step 7: Zero-Cost Abstractions [DEFERRED] (2-3 hours)](#step-7-zero-cost-abstractions-deferred-2-3-hours)
    - [Option A: Simple (No Changes)](#option-a-simple-no-changes)
    - [Option B: Feature-Gated Compilation](#option-b-feature-gated-compilation)
    - [Option C: Newtype Wrapper](#option-c-newtype-wrapper)
    - [Recommendation for Phase 7:](#recommendation-for-phase-7)
  - [Step 8: Macro Unification [DEFERRED] & Builder DSL (1 hour)](#step-8-macro-unification-deferred--builder-dsl-1-hour)
    - [Option 1: Extend styled_text() Function](#option-1-extend-styled_text-function)
    - [Option 2: Create styled!() Macro for Builder Chains](#option-2-create-styled-macro-for-builder-chains)
  - [Testing & Validation](#testing--validation)
    - [Pre-Migration Checklist](#pre-migration-checklist)
    - [Per-File Validation During Phase 3](#per-file-validation-during-phase-3)
    - [Final Validation (Phase 5)](#final-validation-phase-5)
  - [Risk Mitigation](#risk-mitigation)
    - [Risk 1: Call Sites Missed During Migration](#risk-1-call-sites-missed-during-migration)
    - [Risk 2: Rendering Regressions](#risk-2-rendering-regressions)
    - [Risk 3: Performance Degradation](#risk-3-performance-degradation)
    - [Risk 4: Type System Confusion](#risk-4-type-system-confusion)
    - [Rollback Plan](#rollback-plan)
  - [Effort Summary](#effort-summary)
    - [Time Breakdown](#time-breakdown)
    - [Parallelization Opportunities](#parallelization-opportunities)
  - [Success Criteria](#success-criteria)
    - [Must Have (Exit Criteria for Phase 5)](#must-have-exit-criteria-for-phase-5)
    - [Should Have (Exit Criteria for Phase 8)](#should-have-exit-criteria-for-phase-8)
    - [Nice to Have (Post-Consolidation)](#nice-to-have-post-consolidation)
  - [Future Work](#future-work)
    - [Post-Consolidation (Out of Scope)](#post-consolidation-out-of-scope)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Task: Consolidate cli_text and tui_styled_text

**Status**: Planned (Ready for Implementation) **Priority**: Medium (Maintenance & Code Quality)
**Estimated Effort**: 17-23 hours **Dependencies**: None (can be done independently)

## Overview

### Motivation

The codebase currently maintains two parallel systems for styled text:

1. **CliTextInline** (~1800 lines): Ergonomic API for interactive CLI prompts (choose, readline)
2. **TuiStyledText** (~100 lines): Type-safe system for full-screen TUI components with stylesheet
   support

Both converge at the same underlying rendering infrastructure (`PixelCharRenderer` → ANSI bytes) and
represent the same conceptual entity: **styled text with colors and attributes**.

**Problem**: Maintaining two parallel implementations violates DRY and creates confusion:

- Developers must learn two APIs for the same concept
- Bug fixes and features must be implemented twice
- ~1800 lines of duplicated logic and tests
- Inconsistent feature availability (e.g., CLI can't use stylesheets, TUI can't use convenience
  functions)

**Solution**: Consolidate under `TuiStyledText` as the single unified type, extending it with the
ergonomic builder API from CLI context.

### Current State

```
┌─────────────────────────────────────┐
│         Interactive UIs             │
│  (choose, readline_async, utils)    │
└────────────┬────────────────────────┘
             │
             ├─→ CliTextInline
             │   ├─ text: InlineString
             │   ├─ attribs: TuiStyleAttribs
             │   ├─ color_fg: Option<TuiColor>
             │   └─ color_bg: Option<TuiColor>
             │   └─ API: Builder + 40+ functions
             │
             ├─→ convert() → PixelChar[]
             │   convert() → PixelCharRenderer
             │   ────────────→ ANSI bytes
             │                  ↓
             └─→ stdout


┌─────────────────────────────────────┐
│         Full-Screen TUI             │
│  (components, RenderOps, syntax hi) │
└────────────┬────────────────────────┘
             │
             ├─→ TuiStyledText
             │   ├─ style: TuiStyle (superset)
             │   │  ├─ attribs: TuiStyleAttribs
             │   │  ├─ color_fg: Option<TuiColor>
             │   │  ├─ color_bg: Option<TuiColor>
             │   │  ├─ id: Option<TuiStyleId>      ← Extra (stylesheet)
             │   │  ├─ computed: Option<Computed>  ← Extra (styles)
             │   │  ├─ padding: Option<ChUnit>     ← Extra (layout)
             │   │  └─ lolcat: Option<Lolcat>      ← Extra (effects)
             │   └─ text: StringTuiStyledText
             │   └─ API: Macro-based + getters
             │
             ├─→ RenderOps pipeline
             │   convert() → PixelCharRenderer
             │   ────────────→ ANSI bytes
             │                  ↓
             └─→ Terminal
```

### Target Architecture

```
┌─────────────────────────────────────┐
│    All Text Styling (CLI + TUI)     │
└────────────┬────────────────────────┘
             │
             └─→ TuiStyledText (UNIFIED)
                 ├─ style: TuiStyle (full featured)
                 │  ├─ attribs: TuiStyleAttribs
                 │  ├─ color_fg: Option<TuiColor>
                 │  ├─ color_bg: Option<TuiColor>
                 │  ├─ id: Option<TuiStyleId>      ← Available to CLI
                 │  ├─ computed: Option<Computed>  ← Available to CLI
                 │  ├─ padding: Option<ChUnit>     ← Available to CLI
                 │  └─ lolcat: Option<Lolcat>      ← Available to CLI
                 └─ text: String

                 ├─ Builder API (from CLI):
                 │  ├─ .bold(), .dim(), .italic()
                 │  ├─ .fg_color(c), .bg_color(c)
                 │  ├─ .bg_dark_gray(), .bg_cyan(), etc.
                 │  └─ Method chaining support
                 │
                 ├─ Constructor Functions (from CLI):
                 │  ├─ bold("text"), dim("text")
                 │  ├─ fg_red("text"), fg_green("text")
                 │  └─ ~40+ convenience constructors
                 │
                 ├─ Macros (from TUI):
                 │  ├─ tui_styled_text!(@style: ..., @text: ...)
                 │  └─ tui_styled_texts![...]
                 │
                 └─ Rendering:
                    ├─ to_pixel_chars() → PixelChar[]
                    ├─ PixelCharRenderer → ANSI bytes
                    └─ Terminal output

Interactive + Full-Screen ──→ Single unified path to ANSI
```

## Architectural Analysis

### CliTextInline (CLI Context)

**Location**: `tui/src/core/ansi/generator/cli_text.rs`

**Structure**:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliTextInline {
    pub text: InlineString,                    // ~64 bytes, stack-allocated
    pub attribs: TuiStyleAttribs,              // 4 bytes
    pub color_fg: Option<TuiColor>,            // 12 bytes
    pub color_bg: Option<TuiColor>,            // 12 bytes
}
// Total: ~92 bytes on stack (no heap allocation for typical text)
```

**API Style**: Ergonomic builder pattern

```rust
let styled = cli_text_inline("Hello", new_style!(bold))
    .fg_red()
    .bg_dark_gray();

let styled = fg_red("Hello").bold().bg_dark_gray();
```

**Rendering Path**:

1. `CliTextInline` → `convert()` → `InlineVec<PixelChar>`
2. `PixelCharRenderer::render_line(&pixels)` → ANSI bytes
3. Output to stdout (lazy evaluation on `.to_string()`)

**Use Cases**:

- `choose()` / `readline_async()` headers and items
- Error messages with colors
- Help text in CLI utilities
- `cmdr` project UI prompts

**Statistics**:

- ~1800 lines of implementation + tests
- 40+ convenience constructor functions
- 15+ builder methods
- ~150+ call sites in codebase

### TuiStyledText (TUI Context)

**Location**: `tui/src/core/tui_styled_text/tui_styled_text_impl.rs`

**Structure**:

```rust
#[derive(Debug, Clone)]
pub struct TuiStyledText {
    pub style: TuiStyle,                       // ~56 bytes (includes extra fields)
    pub text: StringTuiStyledText,             // 24 bytes (SmallVec<[u8; 16]>)
}
// Total: ~80 bytes on stack

// TuiStyle contains:
pub struct TuiStyle {
    pub id: Option<TuiStyleId>,                // 2 bytes (used for stylesheets)
    pub attribs: TuiStyleAttribs,              // 4 bytes (same as CliTextInline)
    pub computed: Option<Computed>,            // 16 bytes (computed styles)
    pub color_fg: Option<TuiColor>,            // 12 bytes (same as CliTextInline)
    pub color_bg: Option<TuiColor>,            // 12 bytes (same as CliTextInline)
    pub padding: Option<ChUnit>,               // 8 bytes (layout in FlexBox)
    pub lolcat: Option<Lolcat>,                // 16 bytes (rainbow effect)
}
// Total: ~56 bytes (superset of CliTextInline styling)
```

**API Style**: Macro-based declarative

```rust
let styled = tui_styled_text! {
    @style: new_style!(bold color_fg: {color}),
    @text: "Hello"
};

let styled = TuiStyledText::new(
    new_style!(bold color_fg: Color::Red),
    "Hello"
);
```

**Rendering Path**:

1. `TuiStyledText` embedded in RenderOps
2. RenderOps → Backend (DirectToAnsi or Crossterm)
3. Backend calls `PixelCharRenderer` internally
4. ANSI bytes to terminal

**Use Cases**:

- Syntax highlighting output
- Component text rendering
- Layout with stylesheet styling
- Rainbow effects (lolcat)

**Statistics**:

- ~100 lines of core implementation
- 2 constructor functions (`.new()`, `.default()`)
- Used in ~50+ call sites
- Tight integration with RenderOps pipeline

### Why Consolidation Works

**Key Insight**: Both systems represent the same conceptual entity with different feature sets:

| Aspect          | CliTextInline           | TuiStyledText                | Solution                          |
| --------------- | ----------------------- | ---------------------------- | --------------------------------- |
| **Core Data**   | text + 4 style fields   | text + TuiStyle (⊇ 4 fields) | Use TuiStyle everywhere           |
| **Storage**     | InlineString (64 bytes) | SmallString (24 bytes)       | Optimize: benchmark both          |
| **API**         | Builder + functions     | Macro + methods              | Port builder API to TuiStyledText |
| **Features**    | Basic styling           | Full (stylesheet, effects)   | Make all features available       |
| **Rendering**   | Direct to PixelChar     | RenderOps → PixelChar        | Same final path                   |
| **Type Safety** | Good (separate type)    | Better (TuiStyle fields)     | Keep TuiStyle                     |

**Proof of Convergence**:

- Both call `PixelCharRenderer::render_line()` for ANSI generation
- Both use `TuiStyle` or equivalent fields internally
- Both implement `FastStringify` for display
- Both work with `InlineVec` for collections

### Convergence Points

```
CliTextInline                         TuiStyledText
      ↓                                    ↓
  convert() to PixelChar[]            RenderOp → PixelChar
      ↓                                    ↓
      └──────────→ PixelCharRenderer ←─────┘
                       ↓
                  style_diff()
                       ↓
                   SgrCode generation
                       ↓
                   ANSI bytes (Vec<u8>)
                       ↓
                    Terminal
```

## Consolidation Benefits

### Maintenance

- **Reduced Code**: Eliminate ~1800 lines of duplication
- **Single Path**: One code path means fewer bugs
- **Easier Testing**: One test suite instead of two parallel ones
- **Consistency**: Bug fixes and features only need one implementation

### Consistency

- **Unified API**: Developers learn one system for styled text
- **Feature Parity**: CLI code can use stylesheets and effects
- **Type Safety**: Single type across all modules
- **Familiar Patterns**: Builder API works the same everywhere

### Extensibility

- **One Optimization Point**: Performance improvements benefit all uses
- **New Features**: Add once, available everywhere (stylesheets, computed styles, effects)
- **Module Organization**: Cleaner dependency graph
- **Future Proofing**: Easier to add new style features

## Trade-offs

| Trade-off                  | Impact                                                                     | Mitigation                                                      |
| -------------------------- | -------------------------------------------------------------------------- | --------------------------------------------------------------- |
| **Memory Overhead**        | CLI context has 8-16 unused bytes/instance (id, computed, padding, lolcat) | Negligible (typical CLI has <100 styled text instances at once) |
| **Migration Effort**       | ~100-150 call sites to update                                              | Systematic approach with incremental validation                 |
| **String Type Change**     | May need InlineString → String (or optimize with SmallString)              | Phase 6 optimizes this                                          |
| **Feature Learning Curve** | CLI developers now have more features (good, but unfamiliar)               | Good documentation + examples                                   |

# Implementation plan

## Step 1: Extend TuiStyledText with Builder API [PENDING] (3-4 hours)

**Goal**: Add the ergonomic builder API from `CliTextInline` to `TuiStyledText`.

**Files Modified**:

- `tui/src/core/tui_styled_text/tui_styled_text_impl.rs`
- `tui/src/core/tui_styled_text/mod.rs` (exports)

**Steps**:

#### 1.1 Add Builder Methods to TuiStyledText

In `tui_styled_text_impl.rs`, add `impl TuiStyledText { ... }` block with:

**Attribute Methods** (6 methods):

```rust
impl TuiStyledText {
    /// Make text bold
    pub fn bold(mut self) -> Self {
        self.style.attribs += Bold;
        self
    }

    pub fn dim(mut self) -> Self {
        self.style.attribs += Dim;
        self
    }

    pub fn italic(mut self) -> Self {
        self.style.attribs += Italic;
        self
    }

    pub fn underline(mut self) -> Self {
        self.style.attribs += Underline;
        self
    }

    pub fn strikethrough(mut self) -> Self {
        self.style.attribs += Strikethrough;
        self
    }

    pub fn dim_underline(mut self) -> Self {
        self.style.attribs += Dim;
        self.style.attribs += Underline;
        self
    }
}
```

**Color Methods** (9 methods):

```rust
    /// Set foreground color
    pub fn fg_color(mut self, color: impl Into<TuiColor>) -> Self {
        self.style.color_fg = Some(color.into());
        self
    }

    /// Set background color
    pub fn bg_color(mut self, color: impl Into<TuiColor>) -> Self {
        self.style.color_bg = Some(color.into());
        self
    }

    // Background color convenience methods
    pub fn bg_cyan(mut self) -> Self {
        self.style.color_bg = Some(TuiColor::Ansi(51.into()));
        self
    }

    pub fn bg_yellow(mut self) -> Self {
        self.style.color_bg = Some(TuiColor::Ansi(226.into()));
        self
    }

    pub fn bg_green(mut self) -> Self {
        self.style.color_bg = Some(TuiColor::Ansi(34.into()));
        self
    }

    pub fn bg_slate_gray(mut self) -> Self {
        self.style.color_bg = Some(tui_color!(slate_gray));
        self
    }

    pub fn bg_dark_gray(mut self) -> Self {
        self.style.color_bg = Some(TuiColor::Ansi(236.into()));
        self
    }

    pub fn bg_night_blue(mut self) -> Self {
        self.style.color_bg = Some(tui_color!(night_blue));
        self
    }

    pub fn bg_moonlight_blue(mut self) -> Self {
        self.style.color_bg = Some(tui_color!(moonlight_blue));
        self
    }
}
```

**Validation**:

```bash
cargo test tui_styled_text --lib
```

#### 1.2 Create Constructor Functions Module

Create new file: `tui/src/core/tui_styled_text/tui_styled_text_constructors.rs`

Port ~40 convenience functions from `cli_text.rs`:

**Attribute Constructors** (6 functions):

```rust
/// Create styled text that is bold
pub fn bold(text: impl AsRef<str>) -> TuiStyledText {
    TuiStyledText::new(new_style!(bold), text.as_ref().to_string())
}

pub fn dim(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn italic(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn underline(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn strikethrough(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn dim_underline(text: impl AsRef<str>) -> TuiStyledText { ... }
```

**Color Constructors** (~33 functions):

```rust
/// Create styled text with foreground color
pub fn fg_color(color: impl Into<TuiColor>, text: &str) -> TuiStyledText {
    let mut style = TuiStyle::default();
    style.color_fg = Some(color.into());
    TuiStyledText::new(style, text.to_string())
}

// Named color constructors
pub fn fg_red(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_green(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_blue(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_yellow(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_cyan(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_magenta(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_white(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_black(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_dark_gray(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_medium_gray(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_light_cyan(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_light_purple(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_deep_purple(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_soft_pink(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_hot_pink(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_light_yellow_green(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_dark_teal(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_bright_cyan(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_dark_purple(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_sky_blue(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_lavender(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_dark_lizard_green(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_orange(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_silver_metallic(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_lizard_green(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_pink(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_dark_pink(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_frozen_blue(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_guards_red(text: impl AsRef<str>) -> TuiStyledText { ... }
pub fn fg_slate_gray(text: impl AsRef<str>) -> TuiStyledText { ... }
```

**Documentation**: Each function includes example:

````rust
/// Create styled red text.
///
/// # Example
/// ```
/// use r3bl_tui::fg_red;
/// let styled = fg_red("Error message");
/// println!("{}", styled);
/// ```
pub fn fg_red(text: impl AsRef<str>) -> TuiStyledText { ... }
````

**Validation**:

```bash
cargo test tui_styled_text_constructors
```

#### 1.3 Add `styled_text()` Function (CLI-Compatible)

In `tui_styled_text_constructors.rs`:

````rust
/// Create styled text with explicit style (replaces cli_text_inline).
///
/// This is the primary function for creating styled text in both CLI and TUI contexts.
///
/// # Example
/// ```
/// use r3bl_tui::{styled_text, new_style};
/// let text = styled_text("Hello", new_style!(bold color_fg: Color::Red));
/// ```
pub fn styled_text(text: impl AsRef<str>, style: impl Into<TuiStyle>) -> TuiStyledText {
    TuiStyledText::new(style.into(), text.as_ref().to_string())
}
````

#### 1.4 Add to_pixel_chars() Method

In `tui_styled_text_impl.rs`:

```rust
impl TuiStyledText {
    /// Convert to pixel characters for rendering.
    ///
    /// Used internally by rendering backends.
    pub fn to_pixel_chars(&self) -> InlineVec<PixelChar> {
        let mut pixels = InlineVec::new();
        let gc_string = GCStringOwned::from(self.text.as_str());

        for cluster in gc_string.iter() {
            pixels.push(PixelChar::PlainText {
                display_char: cluster.string.into(),
                style: self.style,
            });
        }
        pixels
    }
}
```

#### 1.5 Update Module Exports

In `tui/src/core/tui_styled_text/mod.rs`:

```rust
// Private modules
mod tui_styled_text_impl;
mod tui_styled_text_constructors;  // NEW
mod tui_styled_texts_impl;

// Public re-exports
pub use tui_styled_text_impl::*;
pub use tui_styled_text_constructors::*;  // NEW: Export all constructor functions
pub use tui_styled_texts_impl::*;
```

**Validation**:

```bash
cargo check
cargo test tui_styled_text
cargo test --doc tui_styled_text
```

## Step 2: Add Compatibility Layer [PENDING] (1 hour)

**Goal**: Make `CliTextInline` a transparent alias to `TuiStyledText` to support gradual migration.

**Files Modified**:

- `tui/src/core/ansi/generator/cli_text.rs`

**Steps**:

#### 2.1 Create Type Alias

At top of `cli_text.rs` after imports:

```rust
// COMPATIBILITY ALIAS
// TODO (Phase 4): Remove this when all call sites migrated to TuiStyledText
#[deprecated(
    since = "0.x.0",
    note = "Use `TuiStyledText` directly instead. This type will be removed in the next major version."
)]
pub type CliTextInline = TuiStyledText;

// Type aliases (updated to use TuiStyledText)
pub type CliTextLine = InlineVec<TuiStyledText>;
pub type CliTextLines = InlineVec<CliTextLine>;
```

#### 2.2 Create Compatibility Function

```rust
/// Compatibility wrapper for `styled_text()`.
///
/// # Deprecated
/// Use [`styled_text()`] instead.
#[deprecated(
    since = "0.x.0",
    note = "Use `styled_text()` instead. This function will be removed in the next major version."
)]
pub fn cli_text_inline(text: impl AsRef<str>, style: impl Into<TuiStyle>) -> TuiStyledText {
    styled_text(text, style)
}
```

#### 2.3 Re-export Constructor Functions

```rust
// Re-export all TuiStyledText constructors for compatibility
pub use crate::tui_styled_text_constructors::{
    bold, dim, italic, underline, strikethrough, dim_underline,
    fg_color, fg_red, fg_green, fg_blue, fg_yellow, fg_cyan, fg_magenta, fg_white, fg_black,
    fg_dark_gray, fg_medium_gray, fg_light_cyan, fg_light_purple, fg_deep_purple,
    fg_soft_pink, fg_hot_pink, fg_light_yellow_green, fg_dark_teal, fg_bright_cyan,
    fg_dark_purple, fg_sky_blue, fg_lavender, fg_dark_lizard_green, fg_orange,
    fg_silver_metallic, fg_lizard_green, fg_pink, fg_dark_pink, fg_frozen_blue,
    fg_guards_red, fg_slate_gray,
    styled_text,
};
```

**Validation**:

```bash
cargo check
cargo build  # Should build with deprecation warnings
```

## Step 3: Migrate Call Sites [PENDING] (5-7 hours)

**Goal**: Update all ~100-150 call sites to use `TuiStyledText` directly.

**Files to Migrate** (in rough priority order):

| File                                                     | Count | Complexity |
| -------------------------------------------------------- | ----- | ---------- |
| `tui/src/readline_async/choose_impl/select_component.rs` | ~20   | Medium     |
| `tui/examples/choose_interactive.rs`                     | ~20   | Low        |
| `cmdr/src/giti/branch/branch_delete_command.rs`          | ~15   | Low        |
| `cmdr/src/common/ui_templates.rs`                        | ~10   | Low        |
| `tui/src/core/log/custom_event_formatter.rs`             | ~8    | Low        |
| `tui/examples/choose_quiz_game.rs`                       | ~10   | Low        |
| `tui/src/core/color_wheel/color_wheel_impl.rs`           | ~5    | Medium     |
| `tui/src/core/ansi/terminal_output.rs`                   | ~2    | Low        |
| Tests (30+ files)                                        | ~30   | Low        |
| Other files (~20)                                        | ~30   | Low        |

**Migration Steps**:

#### 3.1 Type Signature Updates

**Before**:

```rust
fn render_header(header_lines: &InlineVec<InlineVec<CliTextInline>>) -> Result<()> {
    // ...
    use CliTextInline;
}
```

**After**:

```rust
fn render_header(header_lines: &InlineVec<InlineVec<TuiStyledText>>) -> Result<()> {
    // ...
    use TuiStyledText;
}
```

**Approach**:

1. Use find-and-replace for simple cases:
   - `CliTextInline` → `TuiStyledText`
   - `use CliTextInline` → `use TuiStyledText` (or remove if using constructor functions)
2. Manual review for complex patterns

#### 3.2 Function Call Updates

**Before**:

```rust
let styled = cli_text_inline("Hello", new_style!(bold));
let styled = cli_text_inline(&text, style).fg_red().bg_dark_gray();
```

**After**:

```rust
let styled = styled_text("Hello", new_style!(bold));
let styled = styled_text(&text, style).fg_red().bg_dark_gray();
```

**Constructor Functions** (no changes needed, already re-exported):

```rust
let styled = fg_red("Hello").bold();  // Works as-is
```

#### 3.3 Incremental Validation

After migrating each file or module:

```bash
cargo check
cargo test --lib
```

**Suggested Order**:

1. Migrate examples first (test with actual execution)
2. Migrate utility functions
3. Migrate internal implementations
4. Migrate tests (usually last, since they're less critical)

**Validation Checklist**:

- [COMPLETE] `cargo check` succeeds
- [COMPLETE] `cargo test --lib --package r3bl_tui` passes
- [COMPLETE] `cargo test --lib --package cmdr` passes (if applicable)
- [COMPLETE] Examples compile and run
- [COMPLETE] No new deprecation warnings (only from old code)

## Step 4: Remove Old Code [PENDING] (2 hours)

**Goal**: Delete the original `CliTextInline` implementation and clean up.

**Files Modified**:

- `tui/src/core/ansi/generator/cli_text.rs`
- `tui/src/core/ansi/mod.rs`

**Steps**:

#### 4.1 Verify No Remaining Uses

```bash
rg "CliTextInline" --type rust
```

Should only appear in:

- Type aliases in `cli_text.rs`
- Deprecated function wrapper
- Tests (if any remain)

#### 4.2 Remove Implementation Code

In `cli_text.rs`, **KEEP**:

- `CliTextLine` and `CliTextLines` type aliases (they still work with `InlineVec`)
- `cli_text_line!` and `cli_text_lines!` macros (they're generic over element type)
- Any other generic utilities

**DELETE**:

- `struct CliTextInline` definition (lines ~11-95)
- `impl CliTextInline { ... }` (lines ~200-820)
- Constructor functions (lines ~355-738) — these are now in `tui_styled_text_constructors.rs`
- `impl FastStringify for CliTextInline` (lines ~907-925)
- All tests (lines ~930-1836)
- Deprecated wrapper functions (from Phase 2)

#### 4.3 Update Module Exports

In `tui/src/core/ansi/mod.rs`:

**Before**:

```rust
pub use cli_text::{CliTextInline, CliTextLine, CliTextLines, cli_text_inline, /* ... */};
```

**After**:

```rust
pub use cli_text::{CliTextLine, CliTextLines};
// Constructor functions now come from tui_styled_text
pub use crate::tui_styled_text_constructors::*;
```

#### 4.4 Update Imports in Paint Implementations

Files:

- `tui/src/tui/terminal_lib_backends/crossterm_backend/paint_render_op_impl.rs`
- `tui/src/tui/terminal_lib_backends/direct_to_ansi/paint_render_op_impl.rs`

**Before**:

```rust
use crate::{CliTextInline, ...};

fn paint_text_with_attributes(...) {
    let cli_text = CliTextInline {
        text: text_arg.into(),
        attribs: style.attribs,
        color_fg: style.color_fg,
        color_bg: style.color_bg,
    };
    let pixel_chars = cli_text.convert(...);
}
```

**After**:

```rust
use crate::{TuiStyledText, ...};

fn paint_text_with_attributes(...) {
    let styled = TuiStyledText::new(style, text_arg.to_string());
    let pixel_chars = styled.to_pixel_chars();
}
```

**Validation**:

```bash
cargo check
cargo test --lib
```

## Step 5: Testing & Validation [PENDING] (2-3 hours)

**Goal**: Comprehensive testing before moving to optimizations.

#### 5.1 Unit Tests

```bash
# Test TuiStyledText and constructors
cargo test tui_styled_text --lib

# Test CLI components (which now use TuiStyledText)
cargo test choose --lib
cargo test readline --lib

# Full library tests
cargo test --lib --package r3bl_tui

# Tests with deprecated warnings should be minimal
cargo test --lib --package r3bl_tui 2>&1 | grep -i deprecated
```

#### 5.2 Integration Tests

```bash
# Build and run examples
cargo run --example choose_interactive
cargo run --example choose_quiz_game

# Test CLI utilities
cargo test --package cmdr --lib
cargo test --package cmdr
```

#### 5.3 Documentation Tests

```bash
cargo test --doc --package r3bl_tui
```

#### 5.4 Code Quality

```bash
# Check for any remaining issues
cargo clippy --all-targets

# Check formatting
cargo fmt --check
```

#### 5.5 Performance Validation

Establish baseline for next phases:

```bash
# Run benchmarks if available
cargo bench --package r3bl_tui 2>/dev/null || echo "No benchmarks yet"

# Generate flamegraph for comparison with Phase 6 results
./run.fish run-examples-flamegraph-fold --benchmark 2>/dev/null || \
  echo "Flamegraph tools not available, skip this step"
```

## Step 6: SmallString Optimization [PENDING] (1-2 hours)

**Goal**: Optimize the text field storage for better memory efficiency.

**Background**: `CliTextInline` uses `InlineString` (~64 bytes, stack-allocated for typical text),
while `TuiStyledText` currently uses `String` (heap-allocated, 24 bytes base). For most CLI use
cases, the text is short enough to fit on the stack.

**Options**:

#### Option A: Use SmallVec-Based String

Update `StringTuiStyledText` type alias:

```rust
// Before
pub type StringTuiStyledText = SmallString;  // Already 24 bytes

// This is already optimized! SmallString uses SmallVec internally
```

**Action**: Just verify existing SmallString is sufficient.

#### Option B: Use InlineString (if better fit)

```rust
// Alternative: Use InlineString for better stack allocation
pub type StringTuiStyledText = InlineString;  // ~64 bytes on stack

// Update TuiStyledText::new() to accept InlineString
impl TuiStyledText {
    pub fn new(style: TuiStyle, text: impl Into<InlineString>) -> Self {
        TuiStyledText {
            style,
            text: text.into(),
        }
    }
}
```

**Decision Matrix**:

| Metric               | SmallString           | InlineString           |
| -------------------- | --------------------- | ---------------------- |
| Stack size           | 24 bytes              | 64 bytes               |
| Heap alloc threshold | ~8 bytes              | None (always stack)    |
| Lookup performance   | 1 alloc for long text | Perfect cache locality |
| TUI fit              | Good                  | Better for CLI         |

**Recommendation**: Profile with benchmarks (Phase 5) to decide. If most text is <50 chars,
InlineString might be better. If mostly <10 chars, SmallString is fine.

#### Phase 6 Implementation:

1. **Benchmark Current**: Measure allocation patterns

   ```bash
   cargo bench --package r3bl_tui tui_styled_text
   ```

2. **Profile with Flamegraph**: Check if allocation shows up

   ```bash
   ./run.fish run-examples-flamegraph-fold --benchmark
   ```

3. **Try Alternative** (e.g., InlineString)

   ```rust
   pub type StringTuiStyledText = InlineString;
   // Update all creation sites
   ```

4. **Benchmark Again**: Compare results

5. **Document Decision**: Add comment explaining choice

**Success Criteria**:

- [COMPLETE] No performance regression
- [COMPLETE] Better or equal memory usage for typical workloads
- [COMPLETE] All tests pass

## Step 7: Zero-Cost Abstractions [DEFERRED] (2-3 hours)

**Goal**: Use conditional compilation to exclude unused fields from TuiStyle when not needed
(advanced optimization).

**Problem**: In CLI context, `TuiStyle` has 8-16 unused bytes:

- `id: Option<TuiStyleId>` (2 bytes) — stylesheet tracking
- `computed: Option<Computed>` (16 bytes) — computed styles
- `padding: Option<ChUnit>` (8 bytes) — layout
- `lolcat: Option<Lolcat>` (16 bytes) — rainbow effects

**Solutions**:

#### Option A: Simple (No Changes)

Keep all fields. Overhead is negligible for CLI use case (~100 instances max).

**Recommendation**: Choose this unless profiling shows memory pressure.

#### Option B: Feature-Gated Compilation

Add to `Cargo.toml`:

```toml
[features]
default = ["full-featured"]
full-featured = []
cli-only = []
```

In `tui_style_impl.rs`:

```rust
#[derive(Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct TuiStyle {
    #[cfg(feature = "full-featured")]
    pub id: Option<TuiStyleId>,

    pub attribs: TuiStyleAttribs,

    #[cfg(feature = "full-featured")]
    pub computed: Option<Computed>,

    pub color_fg: Option<TuiColor>,
    pub color_bg: Option<TuiColor>,

    #[cfg(feature = "full-featured")]
    pub padding: Option<ChUnit>,

    #[cfg(feature = "full-featured")]
    pub lolcat: Option<Lolcat>,
}
```

**Pros**:

- Zero-cost abstraction
- Memory optimal for each use case

**Cons**:

- Complex build configuration
- Feature combinations might not work well
- Compilation overhead (multiple builds)
- Harder to debug

#### Option C: Newtype Wrapper

Create a lightweight CLI-specific type:

```rust
pub struct CliStyle {
    pub attribs: TuiStyleAttribs,
    pub color_fg: Option<TuiColor>,
    pub color_bg: Option<TuiColor>,
}

// Conversion trait
impl From<TuiStyle> for CliStyle {
    fn from(style: TuiStyle) -> Self {
        CliStyle {
            attribs: style.attribs,
            color_fg: style.color_fg,
            color_bg: style.color_bg,
        }
    }
}
```

**Pros**:

- Explicit separation of concerns
- Clear memory layout
- Can coexist with TuiStyle

**Cons**:

- Adds new type (more complexity)
- Still need conversion logic

#### Recommendation for Phase 7:

**Skip zero-cost abstractions for now.** Reasons:

1. Memory overhead is negligible (~16 bytes per instance)
2. CLI typically has <100 instances at once
3. Complexity cost outweighs benefits
4. Can be revisited if memory profiling shows issue

**If Memory Becomes Problem Later**:

- Choice Option A (feature-gated compilation) for binary size optimization
- Document decision in `tui/src/core/tui_style/README.md`

**Implementation** (if deciding to proceed):

1. Add feature flags to `Cargo.toml`
2. Add `#[cfg(...)]` attributes to struct fields
3. Update conditional compilation in any code that depends on these fields
4. Test with both feature combinations

## Step 8: Macro Unification [DEFERRED] & Builder DSL (1 hour)

**Goal**: Provide a unified, ergonomic way to create styled text across all contexts.

**Current State**:

- `tui_styled_text!(@style: ..., @text: ...)` — macro
- `styled_text(text, style)` — function
- `bold("text").fg_red()` — builder chain

**Target**: Single ergonomic way that works everywhere.

#### Option 1: Extend styled_text() Function

Current:

```rust
let styled = styled_text("Hello", new_style!(bold));
```

This is already simple and ergonomic. Consider this complete.

#### Option 2: Create styled!() Macro for Builder Chains

```rust
#[macro_export]
macro_rules! styled {
    // Pattern 1: text with style
    ($text:expr, $style:expr) => {
        $crate::styled_text($text, $style)
    };

    // Pattern 2: builder chain
    ($text:expr => { $($builder:tt)* }) => {
        {
            let s = $crate::styled_text($text, TuiStyle::default());
            $(s.$builder)*
        }
    };
}

// Usage:
let styled = styled!("Hello" => { bold().fg_red() });
let styled = styled!("World", new_style!(bold));
```

**Implementation**:

In `tui/src/core/tui_styled_text/macros.rs` (new file):

````rust
/// Create styled text with optional builder chain.
///
/// # Examples
///
/// ```
/// use r3bl_tui::styled;
///
/// // Simple: text with style
/// let s = styled!("Hello", new_style!(bold));
///
/// // With builder chain
/// let s = styled!("Hello" => bold().fg_red());
/// ```
#[macro_export]
macro_rules! styled {
    // Case 1: text, style
    ($text:expr, $style:expr) => {
        $crate::styled_text($text, $style)
    };

    // Case 2: text with builder methods
    ($text:expr => $($method:ident())* ) => {
        {
            let mut s = $crate::styled_text($text, $crate::TuiStyle::default());
            $(
                s = s.$method();
            )*
            s
        }
    };

    // Case 3: just text (default style)
    ($text:expr) => {
        $crate::styled_text($text, $crate::TuiStyle::default())
    };
}
````

**Add to Module**:

In `tui/src/core/tui_styled_text/mod.rs`:

```rust
#[macro_use]
mod macros;

pub use macros::*;
```

**Validation**:

```bash
cargo test styled --lib
cargo test --doc styled
```

**Decision**: Implement if it improves ergonomics. If `styled_text()` + builders is already
sufficient, skip.

## Testing & Validation

### Pre-Migration Checklist

- [ ] Phase 1 complete: Builder methods added to `TuiStyledText`
- [ ] Constructor functions module created
- [ ] `styled_text()` function added
- [ ] `to_pixel_chars()` method works
- [ ] All tests pass: `cargo test tui_styled_text`

### Per-File Validation During Phase 3

For each migrated file:

```bash
# 1. Update file to use TuiStyledText
# 2. Check compilation
cargo check

# 3. Run relevant tests
cargo test <module_name>

# 4. If example, run it manually
cargo run --example <example_name>

# 5. Check no new deprecation warnings from new code
cargo build 2>&1 | grep "deprecated" | grep -v "COMPATIBILITY"
```

### Final Validation (Phase 5)

```bash
# Full build and test
cargo clean
cargo build --release
cargo test --all

# Check specific areas
cargo test choose                    # Interactive UI
cargo test cli                       # CLI utilities
cargo test tui_styled_text           # Core type
cargo test --doc                     # Documentation examples

# Code quality
cargo clippy --all-targets
cargo fmt --check

# Performance comparison (optional)
./run.fish run-examples-flamegraph-fold --benchmark
```

## Risk Mitigation

### Risk 1: Call Sites Missed During Migration

**Scenario**: Some `cli_text_inline()` calls not updated, leading to incomplete consolidation.

**Mitigation**:

1. Maintain checklist of files to migrate
2. Use `rg "cli_text_inline|CliTextInline"` to verify complete migration
3. Leave deprecation warnings in place until Phase 4
4. Add CI check: `cargo build 2>&1 | grep -c deprecated`

### Risk 2: Rendering Regressions

**Scenario**: Changes to `to_pixel_chars()` or rendering break visual output.

**Mitigation**:

1. Verify all unit tests pass before migration
2. Test examples manually
3. Run visual regression tests (if available)
4. Keep old rendering code as fallback initially

### Risk 3: Performance Degradation

**Scenario**: Memory overhead or allocation performance worse after consolidation.

**Mitigation**:

1. Benchmark before starting (Phase 5)
2. Profile with flamegraph
3. Complete Phase 6 (optimization)
4. Document any trade-offs

### Risk 4: Type System Confusion

**Scenario**: Developers use `CliTextInline` and `TuiStyledText` inconsistently.

**Mitigation**:

1. Strong deprecation messages
2. Documentation and examples
3. Code review during Phase 3
4. Migration guide in `MIGRATION.md`

### Rollback Plan

If unrecoverable issues arise:

1. **Phase 3 Rollback** (mid-migration):
   - Revert migration commits
   - Keep Phase 1 & 2 (non-breaking additions)
   - Continue migration more slowly

2. **Phase 4 Rollback** (after deletion):
   - Restore `CliTextInline` implementation from git history
   - Update code paths to use restored type
   - Replan consolidation for future

3. **Feature Flag Approach** (if needed):
   - Add `legacy-cli-text` feature flag
   - Compile both types, use feature to select
   - Gradually deprecate flag over releases

## Effort Summary

| Phase     | Description              | Hours     | Dependency         |
| --------- | ------------------------ | --------- | ------------------ |
| 1         | Extend TuiStyledText API | 3-4       | None               |
| 2         | Compatibility Layer      | 1         | Phase 1            |
| 3         | Migrate Call Sites       | 5-7       | Phase 2            |
| 4         | Remove Old Code          | 2         | Phase 3            |
| 5         | Testing & Validation     | 2-3       | Phase 4            |
| 6         | SmallString Optimization | 1-2       | Phase 5            |
| 7         | Zero-Cost Abstractions   | 2-3       | Phase 5 (optional) |
| 8         | Macro Unification        | 1         | Phase 5            |
| **Total** |                          | **17-23** |                    |

### Time Breakdown

- **Core Consolidation**: 13 hours (Phases 1-5)
- **Optimizations**: 4-6 hours (Phases 6-8)
- **Buffer**: 0-3 hours (unexpected issues)

### Parallelization Opportunities

- Phase 2 can start during Phase 1 (after 2 hours)
- Phase 3 call sites can be split among multiple developers
- Phase 6-8 can happen in parallel after Phase 5

## Success Criteria

### Must Have (Exit Criteria for Phase 5)

- [COMPLETE] All tests pass: `cargo test --all`
- [COMPLETE] All examples compile and run
- [COMPLETE] No remaining references to old `CliTextInline` implementation
- [COMPLETE] Deprecation warnings only for compatibility stubs
- [COMPLETE] Documentation updated
- [COMPLETE] Code review approved

### Should Have (Exit Criteria for Phase 8)

- [COMPLETE] Memory usage same or better
- [COMPLETE] No performance regression (flamegraph comparison)
- [COMPLETE] Unified ergonomic API (builder methods + constructor functions)
- [COMPLETE] Comprehensive examples

### Nice to Have (Post-Consolidation)

- [COMPLETE] Feature-gated zero-cost abstractions (Phase 7)
- [COMPLETE] Macro DSL for enhanced ergonomics (Phase 8)
- [COMPLETE] Migration guide for users
- [COMPLETE] Blog post / internal documentation

## Future Work

### Post-Consolidation (Out of Scope)

1. **Universal Styled Text Type**: Investigate if other text types (CliTextLine, CliTextLines) could
   be unified
2. **Rendering Abstraction**: Create trait for `to_pixel_chars()` for other text types
3. **Style Composition**: Add style merging/inheritance for advanced styling
4. **Performance Specialized Types**: If needed, create optimized variants (e.g., `TinyStyledText`
   for <16 char strings)

---

**Document Version**: 1.0 **Last Updated**: 2025-10-27 **Author**: Claude Code **Status**: Ready for
Implementation
