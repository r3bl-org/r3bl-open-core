// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Parameter parsing utilities for VT100-compliant escape sequences. See [`ParamsExt`]
//! and [`parse_cursor_position`] for details.

use crate::{ColIndex, Index, RowIndex, TermCol, TermRow};
use std::{cmp::max, num::NonZeroU16};

/// Extension trait for [`vte::Params`] providing VT100-compliant parameter extraction.
///
/// This extension trait works around Rust's orphan rule, which prevents adding impl
/// blocks directly to [`vte::Params`] (a type from an external crate).
///
/// # VT100 Parameter Structure
///
/// The [`vte::Params`] type captures parameters for a **single command** (after it is
/// parsed from the bytes emitted from the child process running in PTY-slave). We have no
/// control over this. The following is an overview of how VT100 parameters are
/// structured, which informs how the [`vte::Params`] type organizes them.
///
/// > <div class="warning">
/// >
/// > The following is confusing, which is why `ParamsExt` exists. It cleans up
/// > this complexity for us, by using clear methods and type-safe return values.
/// >
/// > </div>
///
/// Semicolons are used to separate parameters from each other, and this is where
/// "positions" come in to play.
///
/// | Separator | Purpose                       | Access Pattern           |
/// |-----------|-------------------------------|--------------------------|
/// |    `;`    | Separate parameter positions  | By position index        |
/// |    `:`    | Group related parameters      | All at once in one slice |
///
/// At a given "position", each parameter can contain a "single" value (no sub-parameters)
/// or "many" values (sub-parameters) separated by colons.
///
/// | At a given "position" | Separator  | Represented as `&[u16]` |
/// |-----------------------|------------|-------------------------|
/// | "single" value        | None       | One-item slice          |
/// | "many" values         | Colon      | Multi-item slice        |
///
/// Here are some examples of how different escape sequences are parsed:
///
/// | Command         | `&[u16]`         | Details                                                                  |
/// |-----------------|------------------|--------------------------------------------------------------------------|
/// | `ESC[5;10H]`    | `[[5], [10]]`    | 2 "single" params separated by `;` each stored as a single-element slice |
/// | `ESC[38:5:196m` | `[[38, 5, 196]]` | 1 param with 3 sub-params separated by `:` stored as a 3 element slice |
///
/// - The `extract_nth_single_*` methods extract the **primary value** (first element)
///   from each parameter slice using `.first()`, which is the standard behavior for most
///   VT100 commands.
/// - In contrast, `extract_nth_many_raw` method returns the complete slice, supporting
///   complex sequences like extended colors that need all sub-parameters.
///
/// # Usage Examples
///
/// **Parameter indexing** (all methods use 0-based indexing):
/// ```text
/// params.extract_nth_single_non_zero(0)   → first parameter's primary value
/// params.extract_nth_single_non_zero(1)   → second parameter's primary value
/// params.extract_nth_single_opt_raw(0)    → first parameter's primary value (raw)
/// params.extract_nth_single_opt_raw(1)    → second parameter's primary value (raw)
/// ```
///
/// **Why these methods exist**: They handle the slice extraction (`.first()`) and
/// VT100-compliant defaults automatically:
///
/// | Sequence | `extract_nth_single_non_zero(0)` | `extract_nth_single_opt_raw(0)` |
/// |----------|----------------------------------|--------------------------------|
/// | `ESC[A`  | 1                                | `Some(0)` ✎                    |
/// | `ESC[0A` | 1                                | `Some(0)` ✎                    |
/// | `ESC[5A` | 5                                | `Some(5)`                      |
///
/// ✎ **Note**: VTE normalizes missing parameters to `0` internally, making them
/// indistinguishable from explicit zeros. Both `ESC[A` and `ESC[0A` produce identical
/// results.
///
/// # Method Selection Guide
///
/// **Use [`extract_nth_single_non_zero`]** when:
/// - Missing/zero parameters should default to 1 (VT100 standard behavior)
/// - Implementing cursor movement commands (`CUU`, `CUD`, `CUF`, `CUB`)
/// - Implementing scroll operations (`SU`, `SD`)
/// - The parameter represents a count or distance
///
/// **Use [`extract_nth_single_opt_raw`]** when:
/// - You need the raw parameter value without the "treat 0 as 1" transformation
/// - Implementing scroll margin commands (`DECSTBM`) where `Some(0)` means "use first
///   line" and `None` means "use viewport bound"
/// - You need to detect out-of-bounds parameter positions (`None`)
///
/// **Use [`extract_nth_many_raw`]** when:
/// - You need all sub-parameters at a given position, not just the first value, without
///   the “treat 0 as 1” transformation
/// - Implementing extended color commands (`SGR m`) with 256-color or RGB syntax
/// - Handling sequences where colon-separated sub-parameters carry distinct meaning
/// - The complete slice contains essential information (e.g., `38:5:196` for color index
///   196)
///
/// # Parameter Handling Rules
///
/// | Method                        | Missing Param   | Zero Param     | Non-Zero Param | Out-of-Bounds | Complex Param     |
/// |-------------------------------|-----------------|----------------|----------------|---------------|-------------------|
/// | `extract_nth_single_non_zero` | 1               | 1              | n              | 1 ✎✎          | first value       |
/// | `extract_nth_single_opt_raw`  | `Some(0)` ✎     | `Some(0)` ✎    | `Some(n)`      | `None`        | first value       |
/// | `extract_nth_many_raw`        | `Some(&[0])` ✎  | `Some(&[0])` ✎ | `Some(&[n])`   | `None`        | `Some(&[...all])` |
///
/// - ✎ VTE cannot distinguish missing parameters from explicit zeros - both produce `0`.
/// - ✎✎ Out-of-bounds is treated as a missing parameter (defaults to 1) per VT100 spec.
///   Use [`extract_nth_single_opt_raw`] if you need to distinguish missing from
///   out-of-bounds.
///
/// # Working Example
///
/// The following example demonstrates parsing a 256-color escape sequence through the VTE
/// parser and extracting its parameters using [`extract_nth_many_raw`]:
///
/// ```rust
/// # // Doc test for VT100 parameter parsing workflow
/// # use r3bl_tui::ParamsExt;
/// # use vte::{Parser, Perform};
/// #
/// # // Helper to test parameter extraction with real VTE parser
/// # struct TestPerform {
/// #     result: Option<Vec<u16>>,
/// # }
/// #
/// # impl Perform for TestPerform {
/// #     fn csi_dispatch(&mut self, params: &vte::Params, _: &[u8], _: bool, _: char) {
/// #         // Extract all sub-parameters from first position
/// #         self.result = params.extract_nth_many_raw(0).map(|s| s.to_vec());
/// #     }
/// #     fn print(&mut self, _: char) {}
/// #     fn execute(&mut self, _: u8) {}
/// #     fn hook(&mut self, _: &vte::Params, _: &[u8], _: bool, _: char) {}
/// #     fn put(&mut self, _: u8) {}
/// #     fn unhook(&mut self) {}
/// #     fn osc_dispatch(&mut self, _: &[&[u8]], _: bool) {}
/// #     fn esc_dispatch(&mut self, _: &[u8], _: bool, _: u8) {}
/// # }
/// #
/// // Parse ESC[38:5:196m (256-color foreground sequence)
/// let mut parser = Parser::new();
/// let mut performer = TestPerform { result: None };
///
/// for byte in b"\x1b[38:5:196m" {
///     parser.advance(&mut performer, *byte);
/// }
///
/// // extract_nth_many_raw returns the complete sub-parameter slice
/// assert_eq!(performer.result, Some(vec![38, 5, 196]));
///
/// // In a real CSI handler, you would pattern match:
/// if let Some(slice) = &performer.result {
///     if slice.len() >= 3 && slice[0] == 38 && slice[1] == 5 {
///         let color_index = slice[2];
///         assert_eq!(color_index, 196);
///         // Apply 256-color foreground...
///     }
/// }
/// ```
///
/// This workflow demonstrates:
/// - **Parsing**: Feed escape sequences to the VTE parser
/// - **Extraction**: Use [`ParamsExt`] methods to get parameter values
/// - **Pattern matching**: Validate and interpret the extracted parameters
///
/// See the [VT100 Parameter Structure](#vt100-parameter-structure) section for details on
/// how parameters are organized in different escape sequence formats (semicolon vs
/// colon).
///
/// # Behavior Validation
///
/// The assumptions documented above about VTE's parameter handling are validated in
/// executable form through the [`vte_params_behavior_validation`] test module. This
/// test suite feeds real escape sequences through the VTE parser and verifies that:
///
/// - **Missing parameters normalize to `0`**: `ESC[A` produces `Some(0)` / `Some(&[0])`
/// - **Explicit zeros are indistinguishable**: `ESC[A` and `ESC[0A` produce identical
///   results
/// - **Out-of-bounds access returns `None`**: Accessing non-existent positions doesn't
///   panic
///
/// These validation tests serve as **regression protection**—if VTE's behavior ever
/// changes in a future version, the tests will immediately catch the discrepancy,
/// alerting us that our documentation and implementation assumptions need updating.
///
/// # Related High-Level Parsers
///
/// While [`ParamsExt`] provides low-level parameter **extraction**, there are
/// higher-level parsing functions that use these primitives to implement **semantic
/// interpretation** for specific VT100 commands. These are distinct layers:
///
/// | Layer                              | Responsibility                                    | Example                            |
/// |------------------------------------|---------------------------------------------------|------------------------------------|
/// | **Extraction** (this trait)        | "How do I get raw parameter values?"              | [`extract_nth_single_non_zero(0)`] |
/// | **Parsing** (standalone functions) | "What do these parameters mean for this command?" | [`parse_cursor_position()`]        |
///
/// ## Available Parsers
///
/// - [`parse_cursor_position()`] - Convert VT100 cursor position parameters (`ESC[5;10H`)
///   to 0-based buffer coordinates ([`RowIndex(4)`], [`ColIndex(9)`])
///
/// ## Design Rationale
///
/// Parsers are intentionally **not** trait methods because:
///
/// 1. **Separation of concerns** - Each parser handles domain-specific logic (type
///    conversion, bounds checking, VT100-spec interpretation)
/// 2. **Scalability** - New parsers for other commands (`parse_sgr_attributes`,
///    `parse_erase_region`, etc.) don't require trait modifications
/// 3. **Composability** - Parsers can be used independently or combined
/// 4. **Testability** - Each parser can be tested in isolation
///
/// [`extract_nth_single_non_zero`]: Self::extract_nth_single_non_zero
/// [`extract_nth_single_opt_raw`]: Self::extract_nth_single_opt_raw
/// [`extract_nth_many_raw`]: Self::extract_nth_many_raw
/// [`vte_params_behavior_validation`]: vte_params_behavior_validation
/// [`vte::Params`]: vte::Params
/// [`parse_cursor_position()`]: parse_cursor_position
/// [`extract_nth_single_non_zero(0)`]: Self::extract_nth_single_non_zero
/// [`RowIndex(4)`]: crate::RowIndex
/// [`ColIndex(9)`]: crate::ColIndex
pub trait ParamsExt {
    /// Extract the nth parameter (0-based) with VT100-compliant default handling.
    ///
    /// See the [VT100 Parameter Structure section] at the trait level for
    /// comprehensive documentation on how parameters are structured, indexed, and how to
    /// choose between this method and [`extract_nth_single_opt_raw`].
    ///
    /// This method extracts only the **first** sub-parameter value (for simple
    /// parameters). For complex sequences with multiple sub-parameters, use
    /// [`extract_nth_many_raw`].
    ///
    /// # Returns
    /// [`NonZeroU16`] - Always returns a value `>= 1` per VT100 specification.
    ///
    /// > <div class="warning">
    /// >
    /// > Missing or zero parameters default to 1, ensuring VT100 compatibility.
    /// > Out-of-bounds parameter positions are also treated as missing and default to 1.
    /// > If you need to distinguish between missing and out-of-bounds, use
    /// > [`extract_nth_single_opt_raw`].
    /// >
    /// > </div>
    ///
    /// [`extract_nth_many_raw`]: Self::extract_nth_many_raw
    /// [`extract_nth_single_opt_raw`]: Self::extract_nth_single_opt_raw
    /// [VT100 Parameter Structure section]: ParamsExt#vt100-parameter-structure
    fn extract_nth_single_non_zero(&self, arg_nth_pos: impl Into<Index>) -> NonZeroU16;

    /// Extract the nth parameter (0-based) without default transformation.
    ///
    /// See the [VT100 Parameter Structure section] at the trait level
    /// for comprehensive documentation on how parameters are structured, indexed, and
    /// how to choose between this method and [`extract_nth_single_non_zero`].
    ///
    /// This method extracts only the **first** sub-parameter value (for simple
    /// parameters). For complex sequences with multiple sub-parameters, use
    /// [`extract_nth_many_raw`].
    ///
    ///
    /// # Returns
    ///
    /// The raw parameter value without VT100's "treat 0 as 1" logic.
    ///
    /// - [`None`] if index n is out of bounds (position doesn't exist)
    /// - [`Some(value)`] if position n exists (value may be 0 for missing/zero params)
    ///
    /// > <div class="warning">
    /// >
    /// > VTE normalizes missing parameters to `0`, so `ESC[A` and `ESC[0A`
    /// > both return `Some(0)`.
    /// >
    /// > </div>
    ///
    /// [`Some(value)`]: Option::Some
    /// [`extract_nth_many_raw`]: Self::extract_nth_many_raw
    /// [`extract_nth_single_non_zero`]: Self::extract_nth_single_non_zero
    /// [VT100 Parameter Structure section]: ParamsExt#vt100-parameter-structure
    fn extract_nth_single_opt_raw(&self, arg_nth_pos: impl Into<Index>) -> Option<u16>;

    /// Extract all (variable number of) sub-parameters at position n as a slice.
    ///
    /// See the [VT100 Parameter Structure section] and
    /// [Working Example section] at the trait level for comprehensive
    /// documentation on parameter structure, examples with real escape sequences, and use
    /// cases for handling semicolon vs colon-separated formats.
    ///
    /// This method provides access to the complete parameter slice at the given position,
    /// including all colon-separated sub-parameters. This is essential for handling
    /// extended color sequences and other complex VT100 parameters.
    ///
    /// # Use Cases
    ///
    /// - **256-color sequences**: `ESC[38:5:196m` → `Some(&[38, 5, 196])`
    /// - **RGB color sequences**: `ESC[38:2:255:128:0m` → `Some(&[38, 2, 255, 128, 0])`
    /// - **Simple parameters**: `ESC[5A` → `Some(&[5])`
    /// - **Multiple parameters**: `ESC[1;31m` → positions 0 and 1 return `Some(&[1])` and
    ///   `Some(&[31])` respectively
    ///
    /// # Returns
    /// - [`None`] if no parameter exists at index n
    /// - [`Some(slice)`] - A reference to the sub-parameter slice (zero-copy, no
    ///   allocation)
    ///
    /// [VT100 Parameter Structure section]: ParamsExt#vt100-parameter-structure
    /// [Working Example section]: ParamsExt#working-example
    /// [`Some(slice)`]: Option::Some
    fn extract_nth_many_raw(&self, arg_nth_pos: impl Into<Index>) -> Option<&[u16]>;
}

impl ParamsExt for vte::Params {
    fn extract_nth_single_non_zero(&self, arg_nth_pos: impl Into<Index>) -> NonZeroU16 {
        let nth_pos: Index = arg_nth_pos.into();
        let value = self
            .iter()
            .nth(nth_pos.as_usize())
            .and_then(|params_at_nth_pos| params_at_nth_pos.first())
            .copied()
            .map_or(
                /* None -> 1 */ 1,
                /* Some(x) -> max(1, x) */ |it| max(1, it),
            );

        // SAFETY: value is guaranteed >= 1 by map_or logic (None->1, Some(x)->max(1,x))
        debug_assert!(value >= 1);
        unsafe { NonZeroU16::new_unchecked(value) }
    }

    fn extract_nth_single_opt_raw(&self, arg_nth_pos: impl Into<Index>) -> Option<u16> {
        let nth_pos: Index = arg_nth_pos.into();
        self.iter()
            .nth(nth_pos.as_usize())
            .and_then(|params_at_nth_pos| params_at_nth_pos.first())
            .copied()
    }

    fn extract_nth_many_raw(&self, arg_nth_pos: impl Into<Index>) -> Option<&[u16]> {
        let nth_pos: Index = arg_nth_pos.into();
        self.iter().nth(nth_pos.as_usize())
    }
}

/// Parse VT100 cursor position parameters and convert to 0-based
/// buffer coordinates.
///
/// # Conversion Flow
///
/// ```text
/// VTE Params           1-based VT100        0-based Buffer
/// (from parser)        Coordinates          Indices
/// ─────────────        ───────────────      ──────────────
/// ESC[5;10H       →    TermRow(5)      →    RowIndex(4)
///                 →    TermCol(10)     →    ColIndex(9)
///     ↓                     ↓                     ↓
/// extract_nth_        wrap in             call
/// single_non_zero()   TermRow/TermCol     to_zero_based()
/// (ensures >= 1)      (1-based coords)    (for buffers)
/// ```
///
/// **VT100 Spec**: Coordinates are 1-based; missing/zero parameters default to 1.
///
/// **Result**: 0-based [`RowIndex`]/[`ColIndex`] ready for buffer operations.
///
/// [`RowIndex`]: crate::RowIndex
/// [`ColIndex`]: crate::ColIndex
#[must_use]
pub fn parse_cursor_position(params: &vte::Params) -> (RowIndex, ColIndex) {
    // Step 1: Extract 1-based parameters (NonZeroU16, guaranteed >= 1)
    let row_param_nz = params.extract_nth_single_non_zero(0);
    let col_param_nz = params.extract_nth_single_non_zero(1);

    // Step 2: Convert 1-based → 0-based via type-safe conversion
    let row = TermRow::from_raw_non_zero_value(row_param_nz).to_zero_based();
    let col = TermCol::from_raw_non_zero_value(col_param_nz).to_zero_based();
    (row, col)
}

/// # Why there are no direct tests for [`ParamsExt`] trait methods
///
/// The [`vte::Params`] type has private fields and cannot be meaningfully constructed
/// with test data. While it implements `Default`, you can only populate it through
/// the VTE parser by feeding it real escape sequences.
///
/// The [`ParamsExt`] methods are thoroughly tested through integration tests in separate
/// test modules:
/// - [`extract_nth_all_tests`] - Tests `extract_nth_many_raw()` with various parameter
///   formats
/// - [`vte_params_behavior_validation`] - Validates VTE parser behavior assumptions
/// - `parse_cursor_position` tests validate cursor position parameter parsing
/// - `MarginRequest` tests (in margin.rs) exercise `extract_nth_single_opt_raw`
///
/// This integration testing approach is preferred because it validates the entire
/// parsing pipeline with real VTE parser output, ensuring correctness with actual
/// terminal escape sequences rather than mocked data.
#[cfg(any(test, doc))]
mod test_fixtures {
    use vte::{Parser, Perform};

    /// Integration test helper - process CSI sequence and extract params.
    ///
    /// This helper feeds a complete CSI escape sequence through the VTE parser
    /// and captures the resulting [`vte::Params`] for testing. This is the only
    /// way to properly test [`ParamsExt`] methods since [`vte::Params`] cannot be
    /// manually constructed.
    pub(super) fn process_csi_sequence_and_test<F>(sequence: &str, test_fn: F)
    where
        F: Fn(&vte::Params),
    {
        let mut parser = Parser::new();
        let mut performer = TestPerformerAdapter::new(test_fn);

        for byte in sequence.bytes() {
            parser.advance(&mut performer, byte);
        }
    }

    /// Adapter that bridges VTE's callback-based API to test closures.
    ///
    /// # What It Does
    ///
    /// This adapter implements the [`Perform`] trait to receive parsed [`vte::Params`]
    /// from the VTE parser, then forwards them to a test closure for inspection.
    ///
    /// # Why It's Needed
    ///
    /// [`vte::Params`] has private fields and cannot be manually constructed. The ONLY
    /// way to get populated params is to feed escape sequences through the VTE parser.
    /// Since VTE uses a callback-based API (the [`Perform`] trait), we need this adapter
    /// to bridge VTE's callback API to our test closure API.
    ///
    /// # Design Pattern
    ///
    /// This is an **Adapter pattern**, not a mock or stub. It doesn't simulate behavior—
    /// it observes real VTE parser output. The [`Option<F>`] pattern allows us to move
    /// the closure out of the struct to call it, working around the `&mut self`
    /// requirement of the [`Perform`] trait while keeping the closure as [`Fn`] (not
    /// [`FnMut`]).
    struct TestPerformerAdapter<F> {
        test_fn: Option<F>,
    }

    impl<F> TestPerformerAdapter<F>
    where
        F: Fn(&vte::Params),
    {
        fn new(test_fn: F) -> Self {
            Self {
                test_fn: Some(test_fn),
            }
        }
    }

    impl<F> Perform for TestPerformerAdapter<F>
    where
        F: Fn(&vte::Params),
    {
        fn csi_dispatch(
            &mut self,
            params: &vte::Params,
            _intermediates: &[u8],
            _ignore: bool,
            _c: char,
        ) {
            // Rust idiom: Option::take() moves the closure out while replacing with None.
            // This works around the `&mut self` requirement - we can't call an `Fn`
            // closure through `&mut self` without this trick. After take(),
            // the closure is consumed and test_fn becomes None (ensuring
            // single execution).
            if let Some(test_fn) = self.test_fn.take() {
                test_fn(params);
            }
        }

        // Required by Perform trait but unused.
        fn print(&mut self, _c: char) {}
        fn execute(&mut self, _byte: u8) {}
        fn hook(
            &mut self,
            _params: &vte::Params,
            _intermediates: &[u8],
            _ignore: bool,
            _c: char,
        ) {
        }
        fn put(&mut self, _byte: u8) {}
        fn unhook(&mut self) {}
        fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
        fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
    }
}

/// These tests validate the fundamental assumptions documented in the [`ParamsExt`] trait
/// about how VTE's [`vte::Params`] type handles missing parameters, explicit zeros, and
/// out-of-bounds access. If VTE's behavior ever changes, these tests will catch it.
#[cfg(any(test, doc))]
pub mod vte_params_behavior_validation {
    use super::{test_fixtures::process_csi_sequence_and_test, *};

    // ============================================================================================
    // Validation: Missing Parameters → Some(0) / Some(&[0])
    // ============================================================================================

    #[test]
    fn test_vte_behavior_missing_param_opt_raw() {
        // ASSUMPTION: VTE normalizes missing parameters to 0
        // ESC[A has no parameter, VTE should represent this as [0]
        process_csi_sequence_and_test("\x1b[A", |params| {
            let result = params.extract_nth_single_opt_raw(0);
            assert_eq!(
                result,
                Some(0),
                "VTE should normalize missing parameter to Some(0)"
            );
        });
    }

    #[test]
    fn test_vte_behavior_missing_param_all() {
        // ASSUMPTION: VTE normalizes missing parameters to [0]
        process_csi_sequence_and_test("\x1b[A", |params| {
            let result = params.extract_nth_many_raw(0);
            assert_eq!(
                result,
                Some(&[0][..]),
                "VTE should normalize missing parameter to Some(&[0])"
            );
        });
    }

    #[test]
    fn test_vte_behavior_missing_param_in_sgr() {
        // ASSUMPTION: Even in SGR sequences, missing params become 0
        // ESC[m (SGR reset with missing parameter)
        process_csi_sequence_and_test("\x1b[m", |params| {
            assert_eq!(params.extract_nth_single_opt_raw(0), Some(0));
            assert_eq!(params.extract_nth_many_raw(0), Some(&[0][..]));
        });
    }

    // ============================================================================================
    // Validation: Explicit Zeros → Some(0) / Some(&[0])
    // ============================================================================================

    #[test]
    fn test_vte_behavior_explicit_zero_opt_raw() {
        // ASSUMPTION: Explicit zero parameters are represented as 0
        // ESC[0A has explicit 0 parameter
        process_csi_sequence_and_test("\x1b[0A", |params| {
            let result = params.extract_nth_single_opt_raw(0);
            assert_eq!(
                result,
                Some(0),
                "VTE should preserve explicit zero as Some(0)"
            );
        });
    }

    #[test]
    fn test_vte_behavior_explicit_zero_all() {
        // ASSUMPTION: Explicit zero parameters are represented as [0]
        process_csi_sequence_and_test("\x1b[0A", |params| {
            let result = params.extract_nth_many_raw(0);
            assert_eq!(
                result,
                Some(&[0][..]),
                "VTE should preserve explicit zero as Some(&[0])"
            );
        });
    }

    #[test]
    fn test_vte_behavior_explicit_zero_in_sgr() {
        // ASSUMPTION: Explicit zeros in SGR are also represented as 0
        // ESC[0m (SGR reset with explicit 0)
        process_csi_sequence_and_test("\x1b[0m", |params| {
            assert_eq!(params.extract_nth_single_opt_raw(0), Some(0));
            assert_eq!(params.extract_nth_many_raw(0), Some(&[0][..]));
        });
    }

    // ============================================================================================
    // Validation: Missing vs Explicit Zero Are Indistinguishable
    // ============================================================================================

    #[test]
    fn test_vte_behavior_cannot_distinguish_missing_from_zero() {
        use std::cell::RefCell;

        // CRITICAL ASSUMPTION: VTE cannot distinguish ESC[A from ESC[0A
        // This validates our documentation that claims they produce identical results

        let missing_result_opt_raw = RefCell::new(None);
        let missing_result_all: RefCell<Option<Vec<u16>>> = RefCell::new(None);
        let explicit_result_opt_raw = RefCell::new(None);
        let explicit_result_all: RefCell<Option<Vec<u16>>> = RefCell::new(None);

        // Missing parameter
        process_csi_sequence_and_test("\x1b[A", |params| {
            *missing_result_opt_raw.borrow_mut() = params.extract_nth_single_opt_raw(0);
            *missing_result_all.borrow_mut() =
                params.extract_nth_many_raw(0).map(<[u16]>::to_vec);
        });

        // Explicit zero
        process_csi_sequence_and_test("\x1b[0A", |params| {
            *explicit_result_opt_raw.borrow_mut() = params.extract_nth_single_opt_raw(0);
            *explicit_result_all.borrow_mut() =
                params.extract_nth_many_raw(0).map(<[u16]>::to_vec);
        });

        // They MUST be identical
        assert_eq!(
            *missing_result_opt_raw.borrow(),
            *explicit_result_opt_raw.borrow(),
            "VTE should produce identical results for missing param and explicit zero (opt_raw)"
        );
        assert_eq!(
            *missing_result_all.borrow(),
            *explicit_result_all.borrow(),
            "VTE should produce identical results for missing param and explicit zero (all)"
        );
    }

    // ============================================================================================
    // Validation: Out-of-Bounds Access → None
    // ============================================================================================

    #[test]
    fn test_vte_behavior_out_of_bounds_opt_raw() {
        // ASSUMPTION: Accessing position that doesn't exist returns None
        // ESC[5A has only 1 parameter at position 0, position 1 doesn't exist
        process_csi_sequence_and_test("\x1b[5A", |params| {
            let result = params.extract_nth_single_opt_raw(1);
            assert_eq!(
                result, None,
                "VTE should return None for out-of-bounds access"
            );
        });
    }

    #[test]
    fn test_vte_behavior_out_of_bounds_all() {
        // ASSUMPTION: Accessing position that doesn't exist returns None
        process_csi_sequence_and_test("\x1b[5A", |params| {
            let result = params.extract_nth_many_raw(1);
            assert_eq!(
                result, None,
                "VTE should return None for out-of-bounds access"
            );
        });
    }

    #[test]
    fn test_vte_behavior_out_of_bounds_far_index() {
        // ASSUMPTION: Even far out-of-bounds indices return None (not panic)
        process_csi_sequence_and_test("\x1b[5A", |params| {
            assert_eq!(params.extract_nth_single_opt_raw(10), None);
            assert_eq!(params.extract_nth_many_raw(10), None);
        });
    }

    #[test]
    fn test_vte_behavior_out_of_bounds_with_missing_param() {
        // ASSUMPTION: Even with missing parameter, out-of-bounds still returns None
        // ESC[A creates position 0 with value [0], but position 1 doesn't exist
        process_csi_sequence_and_test("\x1b[A", |params| {
            // Position 0 exists (as [0])
            assert_eq!(params.extract_nth_single_opt_raw(0), Some(0));
            assert_eq!(params.extract_nth_many_raw(0), Some(&[0][..]));

            // Position 1 doesn't exist
            assert_eq!(params.extract_nth_single_opt_raw(1), None);
            assert_eq!(params.extract_nth_many_raw(1), None);
        });
    }

    // ============================================================================================
    // Validation: Multi-Parameter Sequences
    // ============================================================================================

    #[test]
    fn test_vte_behavior_multiple_params_all_valid() {
        // ASSUMPTION: When multiple positions exist, each can be accessed independently
        // ESC[5;10H has two parameters
        process_csi_sequence_and_test("\x1b[5;10H", |params| {
            assert_eq!(params.extract_nth_single_opt_raw(0), Some(5));
            assert_eq!(params.extract_nth_single_opt_raw(1), Some(10));
            assert_eq!(params.extract_nth_single_opt_raw(2), None); // Out of bounds

            assert_eq!(params.extract_nth_many_raw(0), Some(&[5][..]));
            assert_eq!(params.extract_nth_many_raw(1), Some(&[10][..]));
            assert_eq!(params.extract_nth_many_raw(2), None); // Out of bounds
        });
    }

    #[test]
    fn test_vte_behavior_mixed_missing_and_values() {
        // ASSUMPTION: Semicolons create positions even for missing values
        // ESC[5;;10H has param at pos 0, missing at pos 1, param at pos 2
        process_csi_sequence_and_test("\x1b[5;;10H", |params| {
            assert_eq!(params.extract_nth_single_opt_raw(0), Some(5));
            assert_eq!(params.extract_nth_single_opt_raw(1), Some(0)); // Missing → 0
            assert_eq!(params.extract_nth_single_opt_raw(2), Some(10));
            assert_eq!(params.extract_nth_single_opt_raw(3), None); // Out of bounds
        });
    }
}

#[cfg(test)]
mod extract_nth_all_tests {
    use super::{test_fixtures::process_csi_sequence_and_test, *};

    #[test]
    fn test_extract_nth_all_simple_parameter() {
        process_csi_sequence_and_test("\x1b[5A", |params| {
            // Simple parameter: [[5]]
            let result = params.extract_nth_many_raw(0);
            assert_eq!(result, Some(&[5][..]));
        });
    }

    #[test]
    fn test_extract_nth_all_colon_separated() {
        process_csi_sequence_and_test("\x1b[38:5:196m", |params| {
            // Colon format groups sub-parameters: [[38, 5, 196]]
            let result = params.extract_nth_many_raw(0);
            assert_eq!(result, Some(&[38, 5, 196][..]));
        });
    }

    #[test]
    fn test_extract_nth_all_semicolon_separated() {
        process_csi_sequence_and_test("\x1b[38;5;196m", |params| {
            // Semicolon format creates separate positions: [[38], [5], [196]]
            assert_eq!(params.extract_nth_many_raw(0), Some(&[38][..]));
            assert_eq!(params.extract_nth_many_raw(1), Some(&[5][..]));
            assert_eq!(params.extract_nth_many_raw(2), Some(&[196][..]));
        });
    }

    #[test]
    fn test_extract_nth_all_rgb_color() {
        process_csi_sequence_and_test("\x1b[38:2:255:128:0m", |params| {
            // RGB color: [[38, 2, 255, 128, 0]]
            let result = params.extract_nth_many_raw(0);
            assert_eq!(result, Some(&[38, 2, 255, 128, 0][..]));
        });
    }

    #[test]
    fn test_extract_nth_all_mixed_sequence() {
        process_csi_sequence_and_test("\x1b[1;31;38:5:196m", |params| {
            // Mixed: [[1], [31], [38, 5, 196]]
            assert_eq!(params.extract_nth_many_raw(0), Some(&[1][..]));
            assert_eq!(params.extract_nth_many_raw(1), Some(&[31][..]));
            assert_eq!(params.extract_nth_many_raw(2), Some(&[38, 5, 196][..]));
        });
    }

    #[test]
    fn test_extract_nth_all_out_of_bounds() {
        process_csi_sequence_and_test("\x1b[5A", |params| {
            // Only one parameter, index 1 doesn't exist
            assert_eq!(params.extract_nth_many_raw(1), None);
        });
    }

    #[test]
    fn test_extract_nth_all_empty_sequence() {
        process_csi_sequence_and_test("\x1b[A", |params| {
            // Missing parameter - VTE represents this as [0]
            // This is consistent with VT100 spec where missing params often default to 0
            let result = params.extract_nth_many_raw(0);
            assert_eq!(result, Some(&[0][..]));
        });
    }
}

#[cfg(test)]
mod parse_cursor_position_tests {
    use super::{test_fixtures::process_csi_sequence_and_test, *};

    #[test]
    fn test_parse_cursor_position_with_both_values() {
        process_csi_sequence_and_test("\x1b[5;10H", |params| {
            let (row, col) = parse_cursor_position(params);
            assert_eq!(row.as_u16(), 4); // Should be 0-based (5-1=4)
            assert_eq!(col.as_u16(), 9); // Should be 0-based (10-1=9)
        });
    }

    #[test]
    fn test_parse_cursor_position_with_missing_params() {
        process_csi_sequence_and_test("\x1b[H", |params| {
            let (row, col) = parse_cursor_position(params);
            assert_eq!(row.as_u16(), 0); // Missing row defaults to 1, then 1-1=0
            assert_eq!(col.as_u16(), 0); // Missing col defaults to 1, then 1-1=0
        });
    }

    #[test]
    fn test_parse_cursor_position_with_zeros() {
        process_csi_sequence_and_test("\x1b[0;0H", |params| {
            let (row, col) = parse_cursor_position(params);
            assert_eq!(row.as_u16(), 0); // Zero becomes 1, then 1-1=0
            assert_eq!(col.as_u16(), 0); // Zero becomes 1, then 1-1=0
        });
    }
}
