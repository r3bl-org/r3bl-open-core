// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Parameter parsing utilities for VT100-compliant CSI sequences.
//!
//! This module provides traits and structs for extracting and parsing parameters from
//! CSI sequences according to VT100 specifications.

use crate::{ColIndex, ColWidth, Length, RowHeight, RowIndex, col, height, len, row, width};
use std::cmp::max;

/// Extension trait for vte::Params providing VT100-compliant parameter extraction.
pub trait ParamsExt {
    fn extract_nth_non_zero(&self, n: usize) -> u16;
    fn extract_nth_opt(&self, n: usize) -> Option<u16>;
}

impl ParamsExt for vte::Params {
    /// Extract the nth parameter (0-indexed) with VT100-compliant default handling.
    ///
    /// ## Parameter Handling Rules
    /// - **Missing parameters** (None) default to 1
    /// - **Zero parameters** (Some(0)) are treated as 1
    /// - **Non-zero parameters** (Some(n)) are used as-is
    ///
    /// This ensures compatibility with real VT100 terminals and modern terminal
    /// emulators.
    ///
    /// ## Examples
    /// - `extract_nth_param_non_zero(params, 0)` extracts the first parameter
    /// - `extract_nth_param_non_zero(params, 1)` extracts the second parameter
    /// - `ESC[A` (no param) → returns 1 for any n
    /// - `ESC[0;5A` → returns 1 for n=0, 5 for n=1
    fn extract_nth_non_zero(&self, n: usize) -> u16 {
        self.iter().nth(n).and_then(|p| p.first()).copied().map_or(
            /* None -> 1 */ 1,
            /* Some(0) -> 1 */ |v| max(v, 1),
        )
    }

    /// Extract the nth parameter (0-indexed) without any default transformation.
    ///
    /// This is useful for cases where the parameter's absence has different
    /// semantics than a default value.
    ///
    /// ## Returns
    /// - `None` if no parameter is present at index n
    /// - `Some(value)` if a parameter is present (including 0)
    ///
    /// ## Examples
    /// - `extract_nth_optional_param(params, 0)` extracts the first parameter
    /// - `extract_nth_optional_param(params, 1)` extracts the second parameter
    /// - `ESC[5A` → returns Some(5) for n=0, None for n=1
    /// - `ESC[0;7A` → returns Some(0) for n=0, Some(7) for n=1
    fn extract_nth_opt(&self, n: usize) -> Option<u16> {
        self.iter().nth(n).and_then(|p| p.first()).copied()
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
    pub fn parse_as_length_non_zero(params: &vte::Params) -> Length {
        let count = params.extract_nth_non_zero(0);
        len(count)
    }

    /// Parse VT100 movement parameters as a [`RowHeight`] for vertical operations.
    ///
    /// VT100 specification: missing parameters or zero parameters default to 1.
    ///
    /// # Returns
    /// [`RowHeight`] type for type-safe usage with the bounds checking system.
    #[must_use]
    pub fn parse_as_row_height_non_zero(params: &vte::Params) -> RowHeight {
        let count = params.extract_nth_non_zero(0);
        height(count)
    }

    /// Parse VT100 movement parameters as a [`ColWidth`] for horizontal operations.
    ///
    /// VT100 specification: missing parameters or zero parameters default to 1.
    ///
    /// # Returns
    /// [`ColWidth`] type for type-safe usage with the bounds checking system.
    #[must_use]
    pub fn parse_as_col_width_non_zero(params: &vte::Params) -> ColWidth {
        let count = params.extract_nth_non_zero(0);
        width(count)
    }
}

impl From<&vte::Params> for MovementCount {
    fn from(params: &vte::Params) -> Self {
        // ParamsExt::extract_nth_non_zero() guarantees count >= 1
        // per VT100 spec: missing or zero parameters default to 1
        let count = params.extract_nth_non_zero(0);
        Self(count)
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
    /// parameters defaulting to 1. This method handles the 1-based to 0-based
    /// conversion internally.
    ///
    /// # Returns
    /// [`RowIndex`] with 0-based position ready for use in buffer operations.
    #[must_use]
    pub fn parse_as_row_index_non_zero_to_index_type(params: &vte::Params) -> RowIndex {
        let position = params.extract_nth_non_zero(0); // Gets 1-based position
        row(position.saturating_sub(1)) // Convert to 0-based
    }

    /// Parse VT100 position parameter as a 0-based [`ColIndex`].
    ///
    /// VT100 specification: position parameters are 1-based, with missing/zero
    /// parameters defaulting to 1. This method handles the 1-based to 0-based
    /// conversion internally.
    ///
    /// # Returns
    /// [`ColIndex`] with 0-based position ready for use in buffer operations.
    #[must_use]
    pub fn parse_as_col_index_non_zero_to_index_type(params: &vte::Params) -> ColIndex {
        let position = params.extract_nth_non_zero(0); // Gets 1-based position
        col(position.saturating_sub(1)) // Convert to 0-based
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

impl From<(u16, u16)> for CursorPositionRequest {
    fn from((row_param, col_param): (u16, u16)) -> Self {
        // Convert from 1-based VT100 coordinates to 0-based internal coordinates.
        Self {
            row: row_param.saturating_sub(1),
            col: col_param.saturating_sub(1),
        }
    }
}

impl From<&vte::Params> for CursorPositionRequest {
    fn from(params: &vte::Params) -> Self {
        let row_param = params.extract_nth_non_zero(0);
        let col_param = params.extract_nth_non_zero(1);
        (row_param, col_param).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration test helper - process CSI sequence and extract params.
    fn process_csi_sequence_and_test<F>(sequence: &str, test_fn: F)
    where
        F: Fn(&vte::Params),
    {
        use vte::{Parser, Perform};

        struct TestPerformer<F> {
            test_fn: Option<F>,
        }

        impl<F> TestPerformer<F>
        where
            F: Fn(&vte::Params),
        {
            fn new(test_fn: F) -> Self {
                Self {
                    test_fn: Some(test_fn),
                }
            }
        }

        impl<F> Perform for TestPerformer<F>
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

        let mut parser = Parser::new();
        let mut performer = TestPerformer::new(test_fn);

        for byte in sequence.bytes() {
            parser.advance(&mut performer, byte);
        }
    }

    mod movement_count_tests {
        use super::*;

        // # Implementation Note: Intentional Use of Raw `usize`
        //
        // Test assertions use `.as_usize()` for comparison with numeric literals.
        // Type-safe `Length` values need conversion to `usize` for test validation.

        #[test]
        fn test_parse_as_length_non_zero_with_valid_value() {
            process_csi_sequence_and_test("\x1b[5A", |params| {
                let result = MovementCount::parse_as_length_non_zero(params);
                assert_eq!(result.as_usize(), 5);
            });
        }

        #[test]
        fn test_parse_as_length_non_zero_with_missing_params() {
            process_csi_sequence_and_test("\x1b[A", |params| {
                let result = MovementCount::parse_as_length_non_zero(params);
                assert_eq!(result.as_usize(), 1); // Should default to 1
            });
        }

        #[test]
        fn test_parse_as_length_non_zero_with_zero_param() {
            process_csi_sequence_and_test("\x1b[0A", |params| {
                let result = MovementCount::parse_as_length_non_zero(params);
                assert_eq!(result.as_usize(), 1); // Zero should become 1
            });
        }

        #[test]
        fn test_parse_as_row_height_non_zero_with_valid_value() {
            process_csi_sequence_and_test("\x1b[10A", |params| {
                let result = MovementCount::parse_as_row_height_non_zero(params);
                assert_eq!(result.as_u16(), 10);
            });
        }

        #[test]
        fn test_parse_as_row_height_non_zero_with_missing_params() {
            process_csi_sequence_and_test("\x1b[A", |params| {
                let result = MovementCount::parse_as_row_height_non_zero(params);
                assert_eq!(result.as_u16(), 1); // Should default to 1
            });
        }

        #[test]
        fn test_parse_as_col_width_non_zero_with_valid_value() {
            process_csi_sequence_and_test("\x1b[25C", |params| {
                let result = MovementCount::parse_as_col_width_non_zero(params);
                assert_eq!(result.as_u16(), 25);
            });
        }

        #[test]
        fn test_parse_as_col_width_non_zero_with_missing_params() {
            process_csi_sequence_and_test("\x1b[C", |params| {
                let result = MovementCount::parse_as_col_width_non_zero(params);
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

    mod absolute_position_tests {
        use super::*;

        #[test]
        fn test_parse_as_row_index_non_zero_to_index_type_with_valid_value() {
            process_csi_sequence_and_test("\x1b[5d", |params| {
                // VPA command
                let result =
                    AbsolutePosition::parse_as_row_index_non_zero_to_index_type(params);
                assert_eq!(result.as_u16(), 4); // Should be 0-based (5-1=4)
            });
        }

        #[test]
        fn test_parse_as_row_index_non_zero_to_index_type_with_missing_params() {
            process_csi_sequence_and_test("\x1b[d", |params| {
                let result =
                    AbsolutePosition::parse_as_row_index_non_zero_to_index_type(params);
                assert_eq!(result.as_u16(), 0); // Missing param defaults to 1, then 1-1=0
            });
        }

        #[test]
        fn test_parse_as_row_index_non_zero_to_index_type_with_zero() {
            process_csi_sequence_and_test("\x1b[0d", |params| {
                let result =
                    AbsolutePosition::parse_as_row_index_non_zero_to_index_type(params);
                assert_eq!(result.as_u16(), 0); // Zero becomes 1, then 1-1=0
            });
        }

        #[test]
        fn test_parse_as_row_index_non_zero_to_index_type_with_one() {
            process_csi_sequence_and_test("\x1b[1d", |params| {
                let result =
                    AbsolutePosition::parse_as_row_index_non_zero_to_index_type(params);
                assert_eq!(result.as_u16(), 0); // Should be 0-based (1-1=0)
            });
        }

        #[test]
        fn test_parse_as_col_index_non_zero_to_index_type_with_valid_value() {
            process_csi_sequence_and_test("\x1b[10G", |params| {
                // CHA command
                let result =
                    AbsolutePosition::parse_as_col_index_non_zero_to_index_type(params);
                assert_eq!(result.as_u16(), 9); // Should be 0-based (10-1=9)
            });
        }

        #[test]
        fn test_parse_as_col_index_non_zero_to_index_type_with_missing_params() {
            process_csi_sequence_and_test("\x1b[G", |params| {
                let result =
                    AbsolutePosition::parse_as_col_index_non_zero_to_index_type(params);
                assert_eq!(result.as_u16(), 0); // Missing param defaults to 1, then 1-1=0
            });
        }

        #[test]
        fn test_parse_as_col_index_non_zero_to_index_type_with_zero() {
            process_csi_sequence_and_test("\x1b[0G", |params| {
                let result =
                    AbsolutePosition::parse_as_col_index_non_zero_to_index_type(params);
                assert_eq!(result.as_u16(), 0); // Zero becomes 1, then 1-1=0
            });
        }

        #[test]
        fn test_parse_as_col_index_non_zero_to_index_type_large_value() {
            process_csi_sequence_and_test("\x1b[100G", |params| {
                let result =
                    AbsolutePosition::parse_as_col_index_non_zero_to_index_type(params);
                assert_eq!(result.as_u16(), 99); // Should be 0-based (100-1=99)
            });
        }
    }

    mod cursor_position_request_tests {
        use super::*;

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
            let result = CursorPositionRequest::from((5, 10)); // Already 0-based from tuple
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
}
