# Task: Remove `smallvec` and `smallstr`

<!-- BEGIN mktoc -->

- [Task: Remove `smallvec` and `smallstr`](#task-remove-smallvec-and-smallstr)
- [Overview](#overview)
    - [Target Types & Signatures](#target-types--signatures)
        - [1. `StackString<const N: usize>` & `InlineString` (`tui/src/core/stack_alloc_types/stack_string.rs` & `sizes.rs`)](#1-stackstringconst-n-usize--inlinestring-tuisrccorestack_alloc_typesstack_stringrs--sizesrs)
        - [2. Heap Collection Wrappers (`tui/src/core/heap_alloc_types/` & `stack_alloc_types/items.rs`)](#2-heap-collection-wrappers-tuisrccoreheap_alloc_types--stack_alloc_typesitemsrs)
        - [3. Analytics Schema & Direct `SmallVec` Replacements](#3-analytics-schema--direct-smallvec-replacements)
- [Implementation plan](#implementation-plan)
    - [Phase 1: Safe `StackString` & `smallstr` Removal](#phase-1-safe-stackstring--smallstr-removal)
    - [Phase 2: Analytics Schema `SmallVec` Removal](#phase-2-analytics-schema-smallvec-removal)
    - [Phase 3: List Wrappers & `InlineVec` Removal](#phase-3-list-wrappers--inlinevec-removal)
    - [Phase 4: Direct `SmallVec` & Dependency Removal](#phase-4-direct-smallvec--dependency-removal)
    - [Phase 5: Docs, Quality & Perf Verification](#phase-5-docs-quality--perf-verification)

<!-- END mktoc -->

## Overview

Eliminate all direct dependencies on `smallvec` and the unmaintained `smallstr` (`v0.3.1`,
which wraps `smallvec` internally) across `r3bl_tui`, `r3bl-cmdr`, and
`r3bl_analytics_schema`:

1. **Zero-`unsafe` `StackString<const N: usize>` (`InlineString`):** Replace
   `smallstr::SmallString` with a ~150-line, 100%-safe Rust newtype wrapping a private
   enum in `tui/src/core/stack_alloc_types/stack_string.rs`. Using `len: u16` and
   `buf: [u8; N]` supports any inline byte capacity up to `u16::MAX` (`65,535`) without
   `MaybeUninit`, raw pointers, or custom `Drop` glue, while keeping invalid `len`,
   non-`UTF-8` bytes, and `N > 65_535` unrepresentable outside the module.
2. **Unify Small String Aliases (`DEFAULT_STRING_STORAGE_SIZE = 28`):** Because `String`
   is 24 bytes (`align 8`), the minimum `size_of::<StackString<N>>()` on 64-bit is **32
   bytes** (`4`-byte discriminant/`u16` header + `28`-byte `buf`, zero wasted padding).
   Smaller capacities like `TinyInlineString` (`[u8; 4]`) occupied ≥24 bytes anyway while
   secretly spilling short strings like `"id: N/A"` (7 bytes) or `"@tags: "` (7 bytes) to
   the heap. Unifying `TinyInlineString` / `tiny_inline_string!` into `InlineString`
   (`StackString<28>`) / `inline_string!` eliminates redundant types while keeping custom
   capacities like `TelemetryReportLineStorage = StackString<256>` inline.
3. **Standard `String` for File Loading:** Replace the 128 KiB stack-allocated
   `DocumentStorage` (`SmallString<[u8; 131_072]>`) with standard heap `String`.
4. **Standard `Vec<T>` Everywhere Else:** Replace `InlineVec<T>`, `InlineVecStr<'a>`,
   `inline_vec!`, `TinyVecBackingStore<T>`, and all direct `SmallVec` / `smallvec!` usages
   with honest `Vec<T>` and `vec!`. Move `RenderList<T>` (pre-allocated with
   `Vec::with_capacity(16)`) and `List<T>` from `stack_alloc_types` to `heap_alloc_types`
   alongside `ParseList<T>`, and remove the three obsolete `SmallVec`-vs-`Vec` benchmark
   modules.

### Target Types & Signatures

#### 1. `StackString<const N: usize>` & `InlineString` (`tui/src/core/stack_alloc_types/stack_string.rs` & `sizes.rs`)

```rust
// tui/src/core/stack_alloc_types/stack_string.rs
// Zero `unsafe` blocks; complies with `#![deny(clippy::as_conversions)]` and
// `#![cfg_attr(not(test), deny(clippy::unwrap_in_result, clippy::unwrap_used))]`.

/// Stack-allocated [`UTF-8`] string with inline capacity `N` bytes (up to [`u16::MAX`]),
/// spilling transparently to a heap [`String`] when exceeded.
///
/// Wraps a private [`StackStringStorage`] enum so `len <= N`, valid [`UTF-8`] in
/// `buf[..len]`, and `N <= u16::MAX` cannot be violated from outside this module.
///
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
#[derive(Clone)]
pub struct StackString<const N: usize>(StackStringStorage<N>);

#[derive(Clone)]
enum StackStringStorage<const N: usize> {
    Inline {
        len: u16,
        buf: [u8; N],
    },
    Heap(String),
}

impl<const N: usize> StackString<N> {
    /// Wraps [`StackStringStorage`] after validating `N <= u16::MAX` at compile time.
    ///
    /// All constructors ([`Self::new`], [`Self::with_capacity`], and [`From`] impls)
    /// route through this helper. While compiling the generic `impl<const N: usize>`
    /// block, `N` has no concrete value yet; `rustc` evaluates the inline `const { ... }`
    /// assertion during monomorphization for each instantiated `N`, failing compilation
    /// when `N > 65_535` and emitting zero runtime instructions otherwise.
    const fn from_storage(storage: StackStringStorage<N>) -> Self {
        const {
            // XMARK: Intentional numeric casting using as (`usize::from` is not `const`).
            #[allow(clippy::as_conversions)]
            let max_inline_bytes = u16::MAX as usize;
            assert!(
                N <= max_inline_bytes,
                "StackString<N> inline capacity N must fit in u16 (<= 65,535 bytes)"
            );
        }
        Self(storage)
    }

    #[must_use]
    pub const fn new() -> Self;

    /// Returns `Self::new()` if `capacity <= N`, otherwise
    /// `Self::from_storage(StackStringStorage::Heap(String::with_capacity(capacity)))`.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self;

    /// Uses `std::str::from_utf8(&buf[..usize::from(*len)]).unwrap_or("")` for `Inline`.
    #[must_use]
    pub fn as_str(&self) -> &str;

    #[must_use]
    pub fn len(&self) -> usize;

    #[must_use]
    pub fn is_empty(&self) -> bool;

    /// Returns `N` when `Inline`, or `s.capacity()` when `Heap(s)`.
    #[must_use]
    pub fn capacity(&self) -> usize;

    /// Resets `len = 0` when `Inline`, or calls `s.clear()` retaining capacity when `Heap(s)`.
    pub fn clear(&mut self);

    /// Appends `string` inline if `cur_len + string.len() <= N`, otherwise spills to `Heap(String)`.
    pub fn push_str(&mut self, string: &str);

    /// Encodes `ch` via `ch.encode_utf8(&mut [0u8; 4])` and delegates to `self.push_str(...)`.
    pub fn push(&mut self, ch: char);

    /// Truncates to `new_len` (panics if not on a char boundary, matching `String::truncate`).
    pub fn truncate(&mut self, new_len: usize);

    /// Test-only spill inspection helper (avoids boolean blindness in production APIs).
    #[cfg(test)]
    #[must_use]
    pub const fn spilled(&self) -> bool;
}
```

Required trait implementations on `StackString<const N: usize>`:

- `Default` -> `Self::new()`
- `Deref<Target = str>` -> `self.as_str()`
- `AsRef<str>` -> `self.as_str()` (single `AsRef` impl to avoid `.as_ref()` inference
  ambiguity)
- `Borrow<str>` -> `self.as_str()`
- `Debug` -> **must** delegate to `Debug::fmt(self.as_str(), f)` (not `#[derive(Debug)]`)
  so `PauseBuffer` (`Vec<InlineString>`) formats as `["abc"]` for PTY tests
- `Display` -> `Display::fmt(self.as_str(), f)`
- `std::fmt::Write` -> `write_str` (`self.push_str`), `write_char` (`self.push`)
- `PartialEq<StackString<M>>`, `Eq`, `PartialOrd`, `Ord`, `Hash` (delegating to
  `self.as_str()` so `Hash` matches `Borrow<str>`)
- Symmetric `PartialEq` with `str`, `&str`, `String`, and `Cow<'_, str>`
- `From<&str>`, `From<&mut str>`, `From<&String>`, `From<String>` (moves `Heap(value)`
  without reallocation when `value.len() > N`), `From<char>`, `From<Cow<'_, str>>`, and
  `From<StackString<N>> for String` (moves `s` without allocation when `Heap(s)`)

Aliases in `sizes.rs`, `telemetry.rs`, `tui_styled_text_impl.rs`, and
`color_wheel_config.rs`:

```rust
// tui/src/core/stack_alloc_types/sizes.rs
pub const DEFAULT_STRING_STORAGE_SIZE: usize = 28; // 4B header + 28B = 32B total
pub type InlineString = StackString<DEFAULT_STRING_STORAGE_SIZE>;

// tui/src/core/common/telemetry.rs
pub const TELEMETRY_REPORT_STRING_SIZE: usize = 256;
pub type TelemetryReportLineStorage = StackString<TELEMETRY_REPORT_STRING_SIZE>;

// tui/src/core/tui_styled_text/tui_styled_text_impl.rs
pub(crate) type StringTuiStyledText = InlineString;

// tui/src/core/color_wheel/color_wheel_config.rs
pub type StringHexColor = InlineString;
```

#### 2. Heap Collection Wrappers (`tui/src/core/heap_alloc_types/` & `stack_alloc_types/items.rs`)

```rust
// tui/src/core/heap_alloc_types/render_list.rs
pub type RenderListStorage<T> = Vec<T>;
pub const RENDER_LIST_INITIAL_CAPACITY: usize = 16;

#[derive(Clone, PartialEq, Debug)]
pub struct RenderList<T> {
    pub inner: RenderListStorage<T>,
}

// Manual `Default` impl (NOT `#[derive(Default)]`) so `RenderList::default()` pre-allocates 16 items:
impl<T> Default for RenderList<T> {
    fn default() -> Self { Self::new() } // Vec::with_capacity(RENDER_LIST_INITIAL_CAPACITY)
}

// tui/src/core/heap_alloc_types/list_of.rs
pub type ListStorage<T> = Vec<T>;

#[derive(Clone, Default, PartialEq, Debug)]
pub struct List<T> {
    pub inner: ListStorage<T>,
}

// tui/src/core/stack_alloc_types/items.rs
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ItemsOwned(pub Vec<InlineString>);
```

#### 3. Analytics Schema & Direct `SmallVec` Replacements

```rust
// analytics_schema/src/analytics_data.rs
#[derive(Debug, Deserialize, Serialize)]
pub struct AnalyticsRecord {
    pub events: Vec<AnalyticsEvent>,
}

// tui/src/core/ansi/vt_100_pty_output_parser/protocols/csi_codes/sequence.rs
EnablePrivateMode(Vec<PrivateModeType>),
DisablePrivateMode(Vec<PrivateModeType>),

// tui/src/core/ansi/vt_100_pty_output_parser/protocols/csi_codes/sgr_attribute_sequences.rs
// Direct zero-allocation writes to `buf: &mut BufTextStorage` using `convert_u16_to_ascii_str_slice!`
// (eliminates both `SmallVec<[u16; 9]>` and per-attribute `code.to_string()` heap allocations).
```

---

## Implementation plan

### Phase 1: Safe `StackString` & `smallstr` Removal

Implement `StackString<const N: usize>`, unify `TinyInlineString` / `tiny_inline_string!`
into `InlineString` / `inline_string!`, replace `DocumentStorage` with `String`, and drop
the `smallstr` dependency.

- [ ] Create `tui/src/core/stack_alloc_types/stack_string.rs` implementing zero-`unsafe`
      `StackString<const N: usize>` with unit tests covering inline storage, exact
      boundary (`N = 28`, `N = 256`), heap spill, `From<String>` move optimization,
      `Debug`/`Display` formatting, `fmt::Write`, `Borrow<str>`/`Hash` equivalence, and
      UTF-8 `truncate`.
- [ ] Attach and re-export `stack_string` in `tui/src/core/stack_alloc_types/mod.rs`.
- [ ] Update `tui/src/core/stack_alloc_types/sizes.rs`: set
      `DEFAULT_STRING_STORAGE_SIZE = 28`, define
      `pub type InlineString = StackString<DEFAULT_STRING_STORAGE_SIZE>;`, and delete
      `TinyInlineString`, `DEFAULT_CHAR_STORAGE_SIZE`, `DocumentStorage`, and
      `DEFAULT_DOCUMENT_SIZE`.
- [ ] Update `tui/src/core/stack_alloc_types/constructors.rs`: delete
      `tiny_inline_string!` and its unit test.
- [ ] Update `tui/src/core/stack_alloc_types/into_existing.rs`: change
      `try_read_file_path_into_inline_string` from
      `<A: smallvec::Array<Item = u8>>(acc: &mut SmallString<A>, ...)` to
      `<const N: usize>(acc: &mut StackString<N>, ...)`, update doc comments referencing
      `TinyInlineString`, and update `tests_read_from_file` (replacing `DocumentStorage`
      with `StackString` / `String` test coverage).
- [ ] Update `cmdr/src/edi/state.rs`: change
      `read_file_into_storage(maybe_file_path: Option<&str>) -> String` (using
      `std::fs::read_to_string`), change `get_file_extension` return type from
      `TinyInlineString` to `InlineString`, and replace `InlineString::from_str(...)`
      calls with `InlineString::from(...)`.
- [ ] Update `tui/src/core/common/telemetry.rs`,
      `tui/src/core/common/ring_buffer_stack.rs`, and
      `tui/src/core/common/ring_buffer_heap.rs` to use
      `StackString<TELEMETRY_REPORT_STRING_SIZE>` (`256`) and
      `StackString<DEFAULT_SMALL_STRING_SIZE>` (`32`).
- [ ] Update `StringTuiStyledText` in
      `tui/src/core/tui_styled_text/tui_styled_text_impl.rs` and `StringHexColor` in
      `tui/src/core/color_wheel/color_wheel_config.rs` (plus doc comment in
      `tui/src/core/color_wheel/gradients/truecolor.rs`) to alias `InlineString`.
- [ ] Replace `TinyInlineString`, `tiny_inline_string!`, `SmallString::as_str`, and
      `InlineString::from_str` across remaining `tui` files (`tui_style_impl.rs`,
      `buffer_struct.rs`, `buffer_config_struct.rs`, `sizing.rs`,
      `parse_smart_list_block.rs`, `parse_metadata_kv.rs`, `intermediate_types.rs`,
      `pixel_char_line.rs`, `paint_impl.rs`), and replace `it.arg(item.to_string())` with
      `it.arg(item.as_str())` in `tui/src/core/script/command_impl/command_run.rs`.
- [ ] Remove `smallstr` from `tui/Cargo.toml` and verify compilation with
      `./check.fish --check`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/Cargo.toml`
    - [ ] `tui/src/core/stack_alloc_types/stack_string.rs`
    - [ ] `tui/src/core/stack_alloc_types/mod.rs`
    - [ ] `tui/src/core/stack_alloc_types/sizes.rs`
    - [ ] `tui/src/core/stack_alloc_types/constructors.rs`
    - [ ] `tui/src/core/stack_alloc_types/into_existing.rs`
    - [ ] `tui/src/core/common/telemetry.rs`
    - [ ] `tui/src/core/common/ring_buffer_stack.rs`
    - [ ] `tui/src/core/common/ring_buffer_heap.rs`
    - [ ] `tui/src/core/tui_styled_text/tui_styled_text_impl.rs`
    - [ ] `tui/src/core/color_wheel/color_wheel_config.rs`
    - [ ] `tui/src/core/color_wheel/gradients/truecolor.rs`
    - [ ] `tui/src/core/tui_style/tui_style_impl.rs`
    - [ ] `tui/src/core/script/command_impl/command_run.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/buffer_struct.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/buffer_config_struct.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/sizing.rs`
    - [ ] `tui/src/tui/md_parser/block/parse_smart_list_block.rs`
    - [ ] `tui/src/tui/md_parser/extended/parse_metadata_kv.rs`
    - [ ] `tui/src/tui/syntax_highlighting/intermediate_types.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/pixel_char_line.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/paint_impl.rs`
    - [ ] `cmdr/src/edi/state.rs`

### Phase 2: Analytics Schema `SmallVec` Removal

Remove `smallvec` from `r3bl_analytics_schema` while preserving 100% `serde_json` wire
compatibility.

- [ ] Update `analytics_schema/src/lib.rs` and `analytics_schema/README.md`: delete
      `DEFAULT_TINY_VEC_SIZE` and `TinyVecBackingStore<T>`, and update crate-level rustdoc
      comments.
- [ ] Update `analytics_schema/src/analytics_data.rs`: change `AnalyticsRecord.events` to
      `Vec<AnalyticsEvent>` (`Vec::new()`), and update unit tests.
- [ ] Remove `smallvec` from `analytics_schema/Cargo.toml` and verify compilation with
      `./check.fish --check`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `analytics_schema/Cargo.toml`
    - [ ] `analytics_schema/README.md`
    - [ ] `analytics_schema/src/lib.rs`
    - [ ] `analytics_schema/src/analytics_data.rs`

### Phase 3: List Wrappers & `InlineVec` Removal

Move `RenderList<T>` and `List<T>` to `heap_alloc_types`, update `ItemsOwned` to
`Vec<InlineString>`, and delete `InlineVec<T>`, `InlineVecStr<'a>`, and `inline_vec!` in
favor of `Vec<T>`, `Vec<&'a str>`, and `vec!`.

- [ ] Move `list_of.rs` and `render_list.rs` from `tui/src/core/stack_alloc_types/` to
      `tui/src/core/heap_alloc_types/`, backing both with `Vec<T>` (`RenderList::new()`
      and manual `Default` using `Vec::with_capacity(16)`, removing redundant
      `From<InlineVecStr<'_>>` impls that would collide with `From<Vec<T>>`).
- [ ] Update `tui/src/core/stack_alloc_types/mod.rs`,
      `tui/src/core/heap_alloc_types/mod.rs`, and `tui/src/core/mod.rs` (moving
      `#[macro_use] mod heap_alloc_types;` directly below `stack_alloc_types` so upward
      `#[macro_use]` propagation for `list!` and `render_list!` covers all downstream
      `core` modules).
- [ ] Update `tui/src/core/stack_alloc_types/items.rs`
      (`ItemsOwned(pub Vec<InlineString>)`), removing `From<SmallVec<[&str; N]>>` and
      `test_from_smallvec_str`/`test_from_inline_vec_str`.
- [ ] Delete `InlineVec<T>`, `InlineVecStr<'a>`, and `INLINE_VEC_SIZE` from
      `tui/src/core/stack_alloc_types/sizes.rs`, and delete `inline_vec!` (and
      `constructors.rs` if only `inline_string!` remains, or keep `inline_string!` in
      `constructors.rs`).
- [ ] Replace all `InlineVec`, `InlineVecStr`, and `inline_vec!` usages (along with paired
      `smallvec![]` initializers on those bindings and `SmallVec::clear` on `StdoutMock`)
      across `tui/src/`, `tui/examples/`, and `cmdr/src/` with `Vec` and `vec!`, then
      verify with `./check.fish --check`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/src/core/mod.rs`
    - [ ] `tui/src/core/heap_alloc_types/mod.rs`
    - [ ] `tui/src/core/heap_alloc_types/list_of.rs`
    - [ ] `tui/src/core/heap_alloc_types/render_list.rs`
    - [ ] `tui/src/core/stack_alloc_types/mod.rs`
    - [ ] `tui/src/core/stack_alloc_types/sizes.rs`
    - [ ] `tui/src/core/stack_alloc_types/constructors.rs`
    - [ ] `tui/src/core/stack_alloc_types/items.rs`
    - [ ] `tui/src/core/ansi/generator/cli_text.rs`
    - [ ] `tui/src/core/common/ordered_map.rs`
    - [ ] `tui/src/core/graphemes/gc_string/owned/gc_string_owned.rs`
    - [ ] `tui/src/core/log/tracing_init.rs`
    - [ ] `tui/src/core/script/command_impl/command_run_report.rs`
    - [ ] `tui/src/core/script/git/branch_ops.rs`
    - [ ] `tui/src/core/terminal_io/input_device.rs`
    - [ ] `tui/src/core/test_fixtures/input_device_fixtures/async_input_stream_mock.rs`
    - [ ] `tui/src/core/test_fixtures/input_device_fixtures/input_device_ext_mock.rs`
    - [ ] `tui/src/core/test_fixtures/input_device_fixtures/mock_input_device.rs`
    - [ ] `tui/src/core/test_fixtures/output_device_fixtures/stdout_mock.rs`
    - [ ] `tui/src/core/tui_style/tui_style_impl.rs`
    - [ ] `tui/src/core/tui_style/tui_stylesheet.rs`
    - [ ] `tui/src/readline_async/choose_impl/select_component.rs`
    - [ ] `tui/src/readline_async/choose_impl/state.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/event_conversion.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/line_state/output.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/clipboard/clipboard_service.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/render_cache.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/selection/selection_container.rs`
    - [ ] `tui/src/tui/editor/editor_buffer/selection/selection_state_machine.rs`
    - [ ] `tui/src/tui/editor/editor_component/editor_event.rs`
    - [ ] `tui/src/tui/editor/editor_engine/content_mut.rs`
    - [ ] `tui/src/tui/layout/props.rs`
    - [ ] `tui/src/tui/layout/surface.rs`
    - [ ] `tui/src/tui/md_parser/block/parse_smart_list_block.rs`
    - [ ] `tui/src/tui/md_parser/extended/parse_metadata_kcsv.rs`
    - [ ] `tui/src/tui/md_parser/md_parser_types.rs`
    - [ ] `tui/src/tui/syntax_highlighting/convert_syntect_to_styled_text.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/ofs_buf/pixel_char_line.rs`
    - [ ] `tui/src/tui/terminal_window/manage_focus/has_focus.rs`
    - [ ] `tui/examples/choose_interactive.rs`
    - [ ] `tui/examples/readline_async.rs`
    - [ ] `tui/examples/tui_apps/ex_app_no_layout/app_main.rs`
    - [ ] `tui/examples/tui_apps/ex_app_with_1col_layout/state.rs`
    - [ ] `tui/examples/tui_apps/ex_app_with_2col_layout/state.rs`
    - [ ] `cmdr/src/common/ui_templates.rs`
    - [ ] `cmdr/src/edi/state.rs`
    - [ ] `cmdr/src/edi/ui_templates.rs`
    - [ ] `cmdr/src/giti/branch/branch_checkout_command.rs`
    - [ ] `cmdr/src/giti/branch/branch_command.rs`
    - [ ] `cmdr/src/giti/branch/branch_delete_command.rs`

### Phase 4: Direct `SmallVec` & Dependency Removal

Migrate all remaining direct `SmallVec` / `smallvec!` call sites, remove the three
obsolete `SmallVec`-vs-`Vec` benchmark files, and delete `smallvec` from `tui/Cargo.toml`
and `cmdr/Cargo.toml`.

- [ ] Rewrite `SgrAttributeSequence::write_to_buf` in
      `tui/src/core/ansi/vt_100_pty_output_parser/protocols/csi_codes/sgr_attribute_sequences.rs`
      to format directly into `buf` using `convert_u16_to_ascii_str_slice!` with zero
      `SmallVec` and zero `u16::to_string()` heap allocations.
- [ ] Migrate `CsiSequence::{EnablePrivateMode, DisablePrivateMode}` in
      `tui/src/core/ansi/vt_100_pty_output_parser/protocols/csi_codes/sequence.rs`
      (removing `MAX_CHAINED_PRIVATE_MODES` if unused elsewhere) and update all VT-100
      conformance & output tests + ANSI doc comments to `Vec<PrivateModeType>` /
      `vec![...]`.
- [ ] Migrate `SharedLedger.addresses` (`policy_impl.rs`), `ColorWheel` (`VecConfigs`,
      `VecStops`, `VecSteps` in `color_wheel_config.rs` & `color_wheel_impl.rs`),
      `PauseBuffer` (`readline_async/mod.rs`), `CrateVersion` split
      (`cmdr/src/self_upgrade/jsonc_manifest/crate_version.rs`), and all remaining
      `smallvec!` literals across `tui/src/`, `tui/examples/`, and `cmdr/src/` to `Vec` /
      `vec!`.
- [ ] Delete `tui/src/tui/terminal_lib_backends/pixel_char_bench.rs`,
      `tui/src/tui/terminal_lib_backends/render_op_bench.rs`, and
      `tui/src/core/tui_styled_text/vec_vs_smallvec_bench_tests.rs`, and remove their
      `mod` declarations from `tui/src/tui/terminal_lib_backends/mod.rs` and
      `tui/src/core/tui_styled_text/mod.rs`.
- [ ] Remove `smallvec` from `tui/Cargo.toml` and `cmdr/Cargo.toml`, and verify
      compilation with `./check.fish --check`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/Cargo.toml`
    - [ ] `cmdr/Cargo.toml`
    - [ ] `tui/src/core/ansi/constants/generic.rs`
    - [ ] `tui/src/core/ansi/constants/input_sequences.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/protocols/csi_codes/sgr_attribute_sequences.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/protocols/csi_codes/sequence.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_control_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_mode_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_scroll_ops.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_system_state_management.rs`
    - [ ] `tui/src/core/color_wheel/color_wheel_config.rs`
    - [ ] `tui/src/core/color_wheel/color_wheel_impl.rs`
    - [ ] `tui/src/core/common/scoped_mutex/deadlock_prevention/policy_impl.rs`
    - [ ] `tui/src/core/resilient_reactor_thread/rrt_integration_tests/double_panic_prevention_test.rs`
    - [ ] `tui/src/core/tui_styled_text/mod.rs`
    - [ ] `tui/src/core/tui_styled_text/vec_vs_smallvec_bench_tests.rs` (deleted)
    - [ ] `tui/src/readline_async/modal_terminal_guard.rs`
    - [ ] `tui/src/readline_async/mod.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_async_integration_tests/pty_editor_state_test.rs`
    - [ ] `tui/src/readline_async/spinner_impl/spinner_style.rs`
    - [ ] `tui/src/tui/dialog/dialog_engine/dialog_engine_struct.rs`
    - [ ] `tui/src/tui/syntax_highlighting/md_parser_syn_hi/md_parser_stylesheet.rs`
    - [ ] `tui/src/tui/syntax_highlighting/md_parser_syn_hi/md_parser_syn_hi_impl.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/tests.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/pixel_char_bench.rs` (deleted)
    - [ ] `tui/src/tui/terminal_lib_backends/render_op_bench.rs` (deleted)
    - [ ] `tui/src/tui/terminal_window/main_event_loop.rs`
    - [ ] `tui/examples/tui_apps/ex_app_with_1col_layout/single_column_component.rs`
    - [ ] `tui/examples/tui_apps/ex_app_with_2col_layout/column_render_component.rs`
    - [ ] `tui/examples/tui_apps/ex_rc/app_main.rs`
    - [ ] `cmdr/src/analytics_client/upgrade_check.rs`
    - [ ] `cmdr/src/edi/app_main.rs`
    - [ ] `cmdr/src/self_upgrade/jsonc_manifest/crate_version.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/prompt.rs`

### Phase 5: Docs, Quality & Perf Verification

Clean up stale rustdoc/README references to `SmallVec`/`SmallString`, format rustdoc, and
run full workspace verification and performance regression checks.

- [ ] Update stale `SmallVec`, `SmallString`, `InlineVec`, and `TinyInlineString`
      documentation references in `tui/src/lib.rs`, `tui/README.md`,
      `tui/src/core/heap_alloc_types/parse_list.rs`,
      `tui/src/core/common/fast_strings/fast_stringify.rs`,
      `tui/src/core/stack_alloc_types/memory_allocator.rs`,
      `tui/src/core/common/get_mem_size.rs`,
      `tui/src/core/common/flat_2d_array/benches.rs`, and
      `tui/src/tui/terminal_lib_backends/backend_selection.rs`.
- [ ] Confirm zero remaining `smallvec` or `smallstr` references in active crate sources
      (`rg "smallvec|SmallVec|smallstr|SmallString" tui/ cmdr/ analytics_schema/ --glob '!*.perf-folded'`).
- [ ] Run `./check.fish --fmt`, `./check.fish --clippy`, `./check.fish --quick-doc`, and
      `./check.fish --test`, plus flamegraph regression verification
      (`analyze-performance`).
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/README.md`
    - [ ] `tui/src/lib.rs`
    - [ ] `tui/src/core/heap_alloc_types/parse_list.rs`
    - [ ] `tui/src/core/common/fast_strings/fast_stringify.rs`
    - [ ] `tui/src/core/stack_alloc_types/memory_allocator.rs`
    - [ ] `tui/src/core/common/get_mem_size.rs`
    - [ ] `tui/src/core/common/flat_2d_array/benches.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/backend_selection.rs`
