// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Parameter parsing utilities for VT100-compliant CSI sequences. See [`ParamsExt`] for
//! details.

use crate::{ColIndex, ColWidth, Index, Length, RowHeight, RowIndex, TermCol, TermRow,
            TermUnit};
use std::{cmp::max, num::NonZeroU16};

/// Extension trait for [`vte::Params`] providing VT100-compliant parameter extraction.
///
/// This extension trait works around Rust's orphan rule, which prevents adding impl
/// blocks directly to [`vte::Params`] (a type from an external crate).
///
/// # VT100 Parameter Structure
///
/// The [`vte::Params`] type handles CSI parameters with support for sub-parameters.
/// Each parameter position is stored as a slice `[u16]` that can contain multiple
/// values separated by colons.
///
/// - The following sequence holds 2 simple parameters: `5` and `10`, each stored as a
///   single-element slice:
///
///   ```text
///   ESC[5;10H      → Simple params: [[5], [10]]
///   ```
///
/// - The following sequence holds 1 parameter with 3 sub-parameters: `38`, `5`, and
///   `196`:
///
///   ```text
///   ESC[38:5:196m  → Sub-params: [[38, 5, 196]]
///   ```
///
/// Both methods in this trait extract the **primary value** (first element) from each
/// parameter slice using `.first()`, which is the standard behavior for most VT100
/// commands.
///
/// # Usage Examples
///
/// **Parameter indexing** (both methods use 0-based indexing):
/// ```text
/// params.extract_nth_single_non_zero(0)  → first parameter's primary value
/// params.extract_nth_single_non_zero(1)  → second parameter's primary value
/// params.extract_nth_single_opt_raw(0)   → first parameter's primary value (raw)
/// params.extract_nth_single_opt_raw(1)   → second parameter's primary value (raw)
/// ```
///
/// **Why these methods exist**: They handle the slice extraction (`.first()`) and
/// VT100-compliant defaults automatically:
///
/// | Sequence | `extract_nth_single_non_zero(0)` | `extract_nth_single_opt_raw(0)` |
/// |----------|----------------------------------|---------------------------------|
/// | `ESC[A`  | 1                                | `Some(0)` ✎                     |
/// | `ESC[0A` | 1                                | `Some(0)` ✎                     |
/// | `ESC[5A` | 5                                | `Some(5)`                       |
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
/// # Parameter Handling Rules
///
/// | Method                         | Missing Param   | Zero Param     | Non-Zero Param | Out-of-Bounds | Complex Param     |
/// |--------------------------------|-----------------|----------------|----------------|---------------|-------------------|
/// | `extract_nth_single_non_zero`  | 1               | 1              | n              | 1             | first value       |
/// | `extract_nth_single_opt_raw`   | `Some(0)` ✎     | `Some(0)` ✎    | `Some(n)`      | `None`        | first value       |
/// | `extract_nth_all`              | `Some(&[0])` ✎  | `Some(&[0])` ✎ | `Some(&[n])`   | `None`        | `Some(&[...all])` |
///
/// ✎ VTE cannot distinguish missing parameters from explicit zeros - both produce `0`.
///
/// # [`vte::Params`] Behavior Validation
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
/// [`extract_nth_single_non_zero`]: ParamsExt::extract_nth_single_non_zero
/// [`extract_nth_single_opt_raw`]: ParamsExt::extract_nth_single_opt_raw
/// [`extract_nth_all`]: ParamsExt::extract_nth_all
/// [`vte::Params`]: vte::Params
pub trait ParamsExt {
    /// Extract the nth parameter (0-based) with VT100-compliant default handling.
    ///
    /// This method extracts only the **first** sub-parameter value (for simple
    /// parameters). For complex sequences with multiple sub-parameters, use
    /// [`extract_nth_all`].
    ///
    /// Missing or zero parameters default to 1, ensuring VT100 compatibility.
    ///
    /// # Returns
    /// [`NonZeroU16`] - Always returns a value `>= 1` per VT100 specification.
    ///
    /// [`extract_nth_all`]: ParamsExt::extract_nth_all
    fn extract_nth_single_non_zero(&self, arg_n: impl Into<Index>) -> NonZeroU16;

    /// Extract the nth parameter (0-based) without default transformation.
    ///
    /// This method extracts only the **first** sub-parameter value (for simple
    /// parameters). For complex sequences with multiple sub-parameters, use
    /// [`extract_nth_all`].
    ///
    /// Returns the raw parameter value without VT100's "treat 0 as 1" logic.
    /// **Note**: VTE normalizes missing parameters to `0`, so `ESC[A` and `ESC[0A`
    /// both return `Some(0)`.
    ///
    /// # Returns
    /// - [`None`] if index n is out of bounds (position doesn't exist)
    /// - [`Some(value)`] if position n exists (value may be 0 for missing/zero params)
    ///
    /// [`Some(value)`]: Option::Some
    /// [`extract_nth_all`]: ParamsExt::extract_nth_all
    fn extract_nth_single_opt_raw(&self, arg_n: impl Into<Index>) -> Option<u16>;

    /// Extract all (variable number of) sub-parameters at position n as a slice.
    ///
    /// This method provides access to the complete parameter slice at the given position,
    /// including all colon-separated sub-parameters. This is essential for handling
    /// extended color sequences and other complex VT100 parameters.
    ///
    /// # Use Cases
    ///
    /// **Extended color sequences** (256-color and RGB):
    /// ```text
    /// ESC[38:5:196m    → extract_nth_all(0) = Some(&[38, 5, 196])
    /// ESC[38:2:255:128:0m → extract_nth_all(0) = Some(&[38, 2, 255, 128, 0])
    /// ```
    ///
    /// **Simple parameters** (single values):
    /// ```text
    /// ESC[5A           → extract_nth_all(0) = Some(&[5])
    /// ESC[1;31m        → extract_nth_all(0) = Some(&[1]), extract_nth_all(1) = Some(&[31])
    /// ```
    ///
    /// **Semicolon vs Colon format**:
    /// ```text
    /// ESC[38:5:196m    → One parameter:   [[38, 5, 196]]
    /// ESC[38;5;196m    → Three parameters: [[38], [5], [196]]
    /// ```
    ///
    /// Both formats are valid VT100, but the colon format groups related sub-parameters
    /// together, making it easier to parse complex sequences.
    ///
    /// # Returns
    /// - [`None`] if no parameter exists at index n
    /// - [`Some(&[u16])`] - A reference to the sub-parameter slice (zero-copy, no
    ///   allocation)
    ///
    /// # Examples
    ///
    /// ```
    /// use r3bl_tui::ParamsExt;
    /// # use vte::{Parser, Perform, Params};
    /// # struct TestPerformer;
    /// # impl Perform for TestPerformer {
    /// #     fn csi_dispatch(&mut self, params: &Params, _: &[u8], _: bool, _: char) {
    /// #         // Simple parameter
    /// #         if let Some(slice) = params.extract_nth_all(0) {
    /// #             assert_eq!(slice, &[5]);
    /// #         }
    /// #     }
    /// #     fn print(&mut self, _: char) {}
    /// #     fn execute(&mut self, _: u8) {}
    /// #     fn hook(&mut self, _: &Params, _: &[u8], _: bool, _: char) {}
    /// #     fn put(&mut self, _: u8) {}
    /// #     fn unhook(&mut self) {}
    /// #     fn osc_dispatch(&mut self, _: &[&[u8]], _: bool) {}
    /// #     fn esc_dispatch(&mut self, _: &[u8], _: bool, _: u8) {}
    /// # }
    ///
    /// // In a CSI dispatch handler:
    /// // ESC[38:5:196m → 256-color foreground
    /// if let Some(slice) = params.extract_nth_all(0) {
    ///     if slice.len() >= 3 && slice[0] == 38 && slice[1] == 5 {
    ///         let color_index = slice[2];
    ///         // Apply 256-color foreground...
    ///     }
    /// }
    /// ```
    ///
    /// [`Some(&[u16])`]: Option::Some
    /// [`None`]: Option::None
    fn extract_nth_all(&self, arg_n: impl Into<Index>) -> Option<&[u16]>;
}

impl ParamsExt for vte::Params {
    fn extract_nth_single_non_zero(&self, arg_n: impl Into<Index>) -> NonZeroU16 {
        let n: Index = arg_n.into();
        let value = self
            .iter()
            .nth(n.as_usize())
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

    fn extract_nth_single_opt_raw(&self, arg_n: impl Into<Index>) -> Option<u16> {
        let n: Index = arg_n.into();
        self.iter()
            .nth(n.as_usize())
            .and_then(|params_at_nth_pos| params_at_nth_pos.first())
            .copied()
    }

    fn extract_nth_all(&self, arg_n: impl Into<Index>) -> Option<&[u16]> {
        let n: Index = arg_n.into();
        self.iter().nth(n.as_usize())
    }
}

/// Movement count for cursor and scroll operations.
///
/// VT100 specification: missing parameters or zero parameters default to 1.
/// This type encapsulates that logic for all movement operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementCount(pub u16);

impl MovementCount {
    /// Parse VT100 movement parameters as a generic [`Length`].
    ///
    /// VT100 specification: missing parameters or zero parameters default to 1.
    ///
    /// # Returns
    /// [`Length`] type for type-safe usage with the bounds checking system.
    #[must_use]
    pub fn parse_first_as_length_non_zero(params: &vte::Params) -> Length {
        let count = params.extract_nth_single_non_zero(0);
        count.get().into()
    }

    /// Parse VT100 movement parameters as a [`RowHeight`] for vertical operations.
    ///
    /// VT100 specification: missing parameters or zero parameters default to 1.
    ///
    /// # Returns
    /// [`RowHeight`] type for type-safe usage with the bounds checking system.
    #[must_use]
    pub fn parse_first_as_row_height_non_zero(params: &vte::Params) -> RowHeight {
        let count = params.extract_nth_single_non_zero(0);
        count.get().into()
    }

    /// Parse VT100 movement parameters as a [`ColWidth`] for horizontal operations.
    ///
    /// VT100 specification: missing parameters or zero parameters default to 1.
    ///
    /// # Returns
    /// [`ColWidth`] type for type-safe usage with the bounds checking system.
    #[must_use]
    pub fn parse_first_as_col_width_non_zero(params: &vte::Params) -> ColWidth {
        let count = params.extract_nth_single_non_zero(0);
        count.get().into()
    }
}

impl From<&vte::Params> for MovementCount {
    fn from(params: &vte::Params) -> Self {
        let count = params.extract_nth_single_non_zero(0);
        Self(count.get())
    }
}

/// Absolute position for cursor positioning operations.
///
/// VT100 specification: position parameters are 1-based, with
/// missing/zero parameters defaulting to 1. This type encapsulates
/// position parameters for absolute cursor positioning commands like
/// CHA (Cursor Horizontal Absolute) and VPA (Vertical Position Absolute).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AbsolutePosition(pub u16);

impl AbsolutePosition {
    /// Parse VT100 position parameter as a 0-based [`RowIndex`].
    ///
    /// VT100 specification: position parameters are 1-based, with missing/zero
    /// parameters defaulting to 1. This method uses type-safe `TermRow` for the
    /// 1-based value and converts it to 0-based `RowIndex`.
    ///
    /// # Returns
    /// [`RowIndex`] with 0-based position ready for use in buffer operations.
    #[must_use]
    pub fn parse_first_as_row_index_non_zero_to_index_type(
        params: &vte::Params,
    ) -> RowIndex {
        TermRow::new(params.extract_nth_single_non_zero(0)).to_zero_based()
    }

    /// Parse VT100 position parameter as a 0-based [`ColIndex`].
    ///
    /// VT100 specification: position parameters are 1-based, with missing/zero
    /// parameters defaulting to 1. This method uses type-safe `TermCol` for the
    /// 1-based value and converts it to 0-based `ColIndex`.
    ///
    /// # Returns
    /// [`ColIndex`] with 0-based position ready for use in buffer operations.
    #[must_use]
    pub fn parse_first_as_col_index_non_zero_to_index_type(
        params: &vte::Params,
    ) -> ColIndex {
        TermCol::new(params.extract_nth_single_non_zero(0)).to_zero_based()
    }
}

/// Cursor position request for CUP (Cursor Position) operations.
///
/// VT100 specification: coordinates are 1-based, but internally converted to 0-based.
/// Missing or zero parameters default to 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CursorPositionRequest {
    /// Row position (0-based, converted from 1-based VT100)
    pub row: u16,
    /// Column position (0-based, converted from 1-based VT100)
    pub col: u16,
}

impl From<(NonZeroU16, NonZeroU16)> for CursorPositionRequest {
    fn from((row_param, col_param): (NonZeroU16, NonZeroU16)) -> Self {
        // Convert from 1-based VT100 coordinates to 0-based internal coordinates.
        Self {
            row: row_param.get().saturating_sub(1),
            col: col_param.get().saturating_sub(1),
        }
    }
}

impl From<&vte::Params> for CursorPositionRequest {
    fn from(params: &vte::Params) -> Self {
        let row_param = params.extract_nth_single_non_zero(0);
        let col_param = params.extract_nth_single_non_zero(1);
        (row_param, col_param).into()
    }
}

/// # Why there are no direct tests for [`ParamsExt`] trait methods
///
/// The [`vte::Params`] type has private fields and cannot be meaningfully constructed
/// with test data. While it implements `Default`, you can only populate it through
/// the VTE parser by feeding it real escape sequences.
///
/// The [`ParamsExt`] methods are thoroughly tested through integration tests in separate
/// test modules:
/// - [`extract_nth_all_tests`] - Tests `extract_nth_all()` with various parameter formats
/// - [`movement_count_tests`] - Tests `extract_nth_single_non_zero()` via MovementCount
/// - [`absolute_position_tests`] - Tests `extract_nth_single_non_zero()` via
///   AbsolutePosition
/// - [`cursor_position_request_tests`] - Tests `extract_nth_single_non_zero()` with
///   multiple positions
/// - MarginRequest tests (in margin.rs) exercise `extract_nth_single_opt_raw`
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
            let result = params.extract_nth_all(0);
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
            assert_eq!(params.extract_nth_all(0), Some(&[0][..]));
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
            let result = params.extract_nth_all(0);
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
            assert_eq!(params.extract_nth_all(0), Some(&[0][..]));
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
                params.extract_nth_all(0).map(|s| s.to_vec());
        });

        // Explicit zero
        process_csi_sequence_and_test("\x1b[0A", |params| {
            *explicit_result_opt_raw.borrow_mut() = params.extract_nth_single_opt_raw(0);
            *explicit_result_all.borrow_mut() =
                params.extract_nth_all(0).map(|s| s.to_vec());
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
            let result = params.extract_nth_all(1);
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
            assert_eq!(params.extract_nth_all(10), None);
        });
    }

    #[test]
    fn test_vte_behavior_out_of_bounds_with_missing_param() {
        // ASSUMPTION: Even with missing parameter, out-of-bounds still returns None
        // ESC[A creates position 0 with value [0], but position 1 doesn't exist
        process_csi_sequence_and_test("\x1b[A", |params| {
            // Position 0 exists (as [0])
            assert_eq!(params.extract_nth_single_opt_raw(0), Some(0));
            assert_eq!(params.extract_nth_all(0), Some(&[0][..]));

            // Position 1 doesn't exist
            assert_eq!(params.extract_nth_single_opt_raw(1), None);
            assert_eq!(params.extract_nth_all(1), None);
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

            assert_eq!(params.extract_nth_all(0), Some(&[5][..]));
            assert_eq!(params.extract_nth_all(1), Some(&[10][..]));
            assert_eq!(params.extract_nth_all(2), None); // Out of bounds
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
            let result = params.extract_nth_all(0);
            assert_eq!(result, Some(&[5][..]));
        });
    }

    #[test]
    fn test_extract_nth_all_colon_separated() {
        process_csi_sequence_and_test("\x1b[38:5:196m", |params| {
            // Colon format groups sub-parameters: [[38, 5, 196]]
            let result = params.extract_nth_all(0);
            assert_eq!(result, Some(&[38, 5, 196][..]));
        });
    }

    #[test]
    fn test_extract_nth_all_semicolon_separated() {
        process_csi_sequence_and_test("\x1b[38;5;196m", |params| {
            // Semicolon format creates separate positions: [[38], [5], [196]]
            assert_eq!(params.extract_nth_all(0), Some(&[38][..]));
            assert_eq!(params.extract_nth_all(1), Some(&[5][..]));
            assert_eq!(params.extract_nth_all(2), Some(&[196][..]));
        });
    }

    #[test]
    fn test_extract_nth_all_rgb_color() {
        process_csi_sequence_and_test("\x1b[38:2:255:128:0m", |params| {
            // RGB color: [[38, 2, 255, 128, 0]]
            let result = params.extract_nth_all(0);
            assert_eq!(result, Some(&[38, 2, 255, 128, 0][..]));
        });
    }

    #[test]
    fn test_extract_nth_all_mixed_sequence() {
        process_csi_sequence_and_test("\x1b[1;31;38:5:196m", |params| {
            // Mixed: [[1], [31], [38, 5, 196]]
            assert_eq!(params.extract_nth_all(0), Some(&[1][..]));
            assert_eq!(params.extract_nth_all(1), Some(&[31][..]));
            assert_eq!(params.extract_nth_all(2), Some(&[38, 5, 196][..]));
        });
    }

    #[test]
    fn test_extract_nth_all_out_of_bounds() {
        process_csi_sequence_and_test("\x1b[5A", |params| {
            // Only one parameter, index 1 doesn't exist
            assert_eq!(params.extract_nth_all(1), None);
        });
    }

    #[test]
    fn test_extract_nth_all_empty_sequence() {
        process_csi_sequence_and_test("\x1b[A", |params| {
            // Missing parameter - VTE represents this as [0]
            // This is consistent with VT100 spec where missing params often default to 0
            let result = params.extract_nth_all(0);
            assert_eq!(result, Some(&[0][..]));
        });
    }
}

#[cfg(test)]
mod movement_count_tests {
    use super::{test_fixtures::process_csi_sequence_and_test, *};

    // # Implementation Note: Intentional Use of Raw `usize`
    //
    // Test assertions use `.as_usize()` for comparison with numeric literals.
    // Type-safe `Length` values need conversion to `usize` for test validation.

    #[test]
    fn test_parse_first_as_length_non_zero_with_valid_value() {
        process_csi_sequence_and_test("\x1b[5A", |params| {
            let result = MovementCount::parse_first_as_length_non_zero(params);
            assert_eq!(result.as_usize(), 5);
        });
    }

    #[test]
    fn test_parse_first_as_length_non_zero_with_missing_params() {
        process_csi_sequence_and_test("\x1b[A", |params| {
            let result = MovementCount::parse_first_as_length_non_zero(params);
            assert_eq!(result.as_usize(), 1); // Should default to 1
        });
    }

    #[test]
    fn test_parse_first_as_length_non_zero_with_zero_param() {
        process_csi_sequence_and_test("\x1b[0A", |params| {
            let result = MovementCount::parse_first_as_length_non_zero(params);
            assert_eq!(result.as_usize(), 1); // Zero should become 1
        });
    }

    #[test]
    fn test_parse_first_as_row_height_non_zero_with_valid_value() {
        process_csi_sequence_and_test("\x1b[10A", |params| {
            let result = MovementCount::parse_first_as_row_height_non_zero(params);
            assert_eq!(result.as_u16(), 10);
        });
    }

    #[test]
    fn test_parse_first_as_row_height_non_zero_with_missing_params() {
        process_csi_sequence_and_test("\x1b[A", |params| {
            let result = MovementCount::parse_first_as_row_height_non_zero(params);
            assert_eq!(result.as_u16(), 1); // Should default to 1
        });
    }

    #[test]
    fn test_parse_first_as_col_width_non_zero_with_valid_value() {
        process_csi_sequence_and_test("\x1b[25C", |params| {
            let result = MovementCount::parse_first_as_col_width_non_zero(params);
            assert_eq!(result.as_u16(), 25);
        });
    }

    #[test]
    fn test_parse_first_as_col_width_non_zero_with_missing_params() {
        process_csi_sequence_and_test("\x1b[C", |params| {
            let result = MovementCount::parse_first_as_col_width_non_zero(params);
            assert_eq!(result.as_u16(), 1); // Should default to 1
        });
    }

    #[test]
    fn test_from_params_trait() {
        process_csi_sequence_and_test("\x1b[42A", |params| {
            let movement_count = MovementCount::from(params);
            assert_eq!(movement_count.0, 42);
        });
    }

    #[test]
    fn test_from_params_trait_with_empty() {
        process_csi_sequence_and_test("\x1b[A", |params| {
            let movement_count = MovementCount::from(params);
            assert_eq!(movement_count.0, 1); // Should default to 1
        });
    }

    #[test]
    fn test_from_params_trait_with_zero() {
        process_csi_sequence_and_test("\x1b[0A", |params| {
            let movement_count = MovementCount::from(params);
            assert_eq!(movement_count.0, 1); // Zero should become 1
        });
    }
}

#[cfg(test)]
mod absolute_position_tests {
    use super::{test_fixtures::process_csi_sequence_and_test, *};

    #[test]
    fn test_parse_first_as_row_index_non_zero_to_index_type_with_valid_value() {
        process_csi_sequence_and_test("\x1b[5d", |params| {
            // VPA command
            let result =
                AbsolutePosition::parse_first_as_row_index_non_zero_to_index_type(params);
            assert_eq!(result.as_u16(), 4); // Should be 0-based (5-1=4)
        });
    }

    #[test]
    fn test_parse_first_as_row_index_non_zero_to_index_type_with_missing_params() {
        process_csi_sequence_and_test("\x1b[d", |params| {
            let result =
                AbsolutePosition::parse_first_as_row_index_non_zero_to_index_type(params);
            assert_eq!(result.as_u16(), 0); // Missing param defaults to 1, then 1-1=0
        });
    }

    #[test]
    fn test_parse_first_as_row_index_non_zero_to_index_type_with_zero() {
        process_csi_sequence_and_test("\x1b[0d", |params| {
            let result =
                AbsolutePosition::parse_first_as_row_index_non_zero_to_index_type(params);
            assert_eq!(result.as_u16(), 0); // Zero becomes 1, then 1-1=0
        });
    }

    #[test]
    fn test_parse_first_as_row_index_non_zero_to_index_type_with_one() {
        process_csi_sequence_and_test("\x1b[1d", |params| {
            let result =
                AbsolutePosition::parse_first_as_row_index_non_zero_to_index_type(params);
            assert_eq!(result.as_u16(), 0); // Should be 0-based (1-1=0)
        });
    }

    #[test]
    fn test_parse_first_as_col_index_non_zero_to_index_type_with_valid_value() {
        process_csi_sequence_and_test("\x1b[10G", |params| {
            // CHA command
            let result =
                AbsolutePosition::parse_first_as_col_index_non_zero_to_index_type(params);
            assert_eq!(result.as_u16(), 9); // Should be 0-based (10-1=9)
        });
    }

    #[test]
    fn test_parse_first_as_col_index_non_zero_to_index_type_with_missing_params() {
        process_csi_sequence_and_test("\x1b[G", |params| {
            let result =
                AbsolutePosition::parse_first_as_col_index_non_zero_to_index_type(params);
            assert_eq!(result.as_u16(), 0); // Missing param defaults to 1, then 1-1=0
        });
    }

    #[test]
    fn test_parse_first_as_col_index_non_zero_to_index_type_with_zero() {
        process_csi_sequence_and_test("\x1b[0G", |params| {
            let result =
                AbsolutePosition::parse_first_as_col_index_non_zero_to_index_type(params);
            assert_eq!(result.as_u16(), 0); // Zero becomes 1, then 1-1=0
        });
    }

    #[test]
    fn test_parse_first_as_col_index_non_zero_to_index_type_large_value() {
        process_csi_sequence_and_test("\x1b[100G", |params| {
            let result =
                AbsolutePosition::parse_first_as_col_index_non_zero_to_index_type(params);
            assert_eq!(result.as_u16(), 99); // Should be 0-based (100-1=99)
        });
    }
}

#[cfg(test)]
mod cursor_position_request_tests {
    use super::{test_fixtures::process_csi_sequence_and_test, *};

    #[test]
    fn test_from_params_with_both_values() {
        process_csi_sequence_and_test("\x1b[5;10H", |params| {
            // CUP command
            let result = CursorPositionRequest::from(params);
            assert_eq!(result.row, 4); // Should be 0-based (5-1=4)
            assert_eq!(result.col, 9); // Should be 0-based (10-1=9)
        });
    }

    #[test]
    fn test_from_params_with_only_row() {
        process_csi_sequence_and_test("\x1b[3H", |params| {
            let result = CursorPositionRequest::from(params);
            assert_eq!(result.row, 2); // Should be 0-based (3-1=2)
            assert_eq!(result.col, 0); // Missing col defaults to 1, then 1-1=0
        });
    }

    #[test]
    fn test_from_params_with_empty() {
        process_csi_sequence_and_test("\x1b[H", |params| {
            let result = CursorPositionRequest::from(params);
            assert_eq!(result.row, 0); // Missing row defaults to 1, then 1-1=0
            assert_eq!(result.col, 0); // Missing col defaults to 1, then 1-1=0
        });
    }

    #[test]
    fn test_from_params_with_zeros() {
        process_csi_sequence_and_test("\x1b[0;0H", |params| {
            let result = CursorPositionRequest::from(params);
            assert_eq!(result.row, 0); // Zero becomes 1, then 1-1=0
            assert_eq!(result.col, 0); // Zero becomes 1, then 1-1=0
        });
    }

    #[test]
    fn test_from_params_with_column_only() {
        process_csi_sequence_and_test("\x1b[;5H", |params| {
            // Empty row, col=5
            let result = CursorPositionRequest::from(params);
            assert_eq!(result.row, 0); // Missing row defaults to 1, then 1-1=0
            assert_eq!(result.col, 4); // Should be 0-based (5-1=4)
        });
    }

    #[test]
    fn test_from_tuple() {
        use crate::vt_100_ansi_parser::vt_100_ansi_conformance_tests::test_fixtures_vt_100_ansi_conformance::nz;
        let result = CursorPositionRequest::from((nz(5), nz(10)));
        assert_eq!(result.row, 4); // Tuple input is 1-based, so 5-1=4
        assert_eq!(result.col, 9); // Tuple input is 1-based, so 10-1=9
    }

    #[test]
    fn test_cursor_position_request_equality() {
        let request1 = CursorPositionRequest { row: 5, col: 10 };
        let request2 = CursorPositionRequest { row: 5, col: 10 };
        let request3 = CursorPositionRequest { row: 5, col: 11 };

        assert_eq!(request1, request2);
        assert_ne!(request1, request3);
    }

    #[test]
    fn test_cursor_position_request_debug() {
        let request = CursorPositionRequest { row: 5, col: 10 };
        let debug_output = format!("{request:?}");
        assert!(debug_output.contains("CursorPositionRequest"));
        assert!(debug_output.contains("row: 5"));
        assert!(debug_output.contains("col: 10"));
    }
}
