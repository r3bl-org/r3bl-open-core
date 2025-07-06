/*
 *   Copyright (c) 2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */
use std::io::{Read, Write};

use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use miette::IntoDiagnostic;

use crate::{Buffer, BufferAtom};

/// Compress the payload using the [`flate2`] crate.
pub fn compress(data: &[BufferAtom]) -> miette::Result<Buffer> {
    let uncompressed_size = data.len();
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).into_diagnostic()?;
    let it = encoder.finish().into_diagnostic();
    let compressed_size = it.as_ref().map(Vec::len).unwrap_or(0);

    log_compression_stats("Compression", uncompressed_size, compressed_size);

    it
}

/// Decompress the payload using the [`flate2`] crate.
pub fn decompress(data: &[BufferAtom]) -> miette::Result<Buffer> {
    let compressed_size = data.len();
    let mut decoder = GzDecoder::new(data);
    let mut decompressed_data = Vec::new();
    decoder
        .read_to_end(&mut decompressed_data)
        .into_diagnostic()?;
    let uncompressed_size = decompressed_data.len();

    log_compression_stats("Decompression", uncompressed_size, compressed_size);

    Ok(decompressed_data)
}

/// Helper function to log compression/decompression statistics using integer arithmetic
/// to avoid floating-point precision issues.
fn log_compression_stats(
    operation: &str,
    uncompressed_size: usize,
    compressed_size: usize,
) {
    let (
        uncompressed_kb,
        uncompressed_remainder,
        compressed_kb,
        compressed_remainder,
        ratio_percent,
        ratio_remainder,
    ) = calculate_compression_stats(uncompressed_size, compressed_size);

    tracing::info!(
        message = operation,
        "{a}.{b:03} kb -> {c}.{d:03} kb ({e}.{f:02}%)",
        a = uncompressed_kb,
        b = uncompressed_remainder,
        c = compressed_kb,
        d = compressed_remainder,
        e = ratio_percent,
        f = ratio_remainder
    );
}

/// Calculate compression statistics using integer arithmetic to avoid floating-point
/// precision issues. Returns (`uncompressed_kb`, `uncompressed_remainder`,
/// `compressed_kb`, `compressed_remainder`, `ratio_percent`, `ratio_remainder`)
fn calculate_compression_stats(
    uncompressed_size: usize,
    compressed_size: usize,
) -> (usize, usize, usize, usize, usize, usize) {
    // Convert to kilobytes using integer division, with remainder for precision
    let uncompressed_kb = uncompressed_size / 1000;
    let uncompressed_remainder = uncompressed_size % 1000;
    let compressed_kb = compressed_size / 1000;
    let compressed_remainder = compressed_size % 1000;

    // Calculate compression ratio as percentage (avoiding division by zero and overflow)
    let (ratio_percent, ratio_remainder) = if uncompressed_size > 0 {
        // Use checked multiplication to avoid overflow
        match compressed_size.checked_mul(100) {
            Some(product) => {
                let percent = product / uncompressed_size;
                // Calculate remainder using checked multiplication for precision
                let remainder = match compressed_size.checked_mul(10000) {
                    Some(product_10k) => (product_10k / uncompressed_size) % 100,
                    None => 0, // Overflow case, just use 0 for remainder
                };
                (percent, remainder)
            }
            None => {
                // Overflow case: compressed_size is very large
                // Fallback to a simple calculation that avoids overflow
                (compressed_size / (uncompressed_size / 100).max(1), 0)
            }
        }
    } else {
        (0, 0)
    };

    (
        uncompressed_kb,
        uncompressed_remainder,
        compressed_kb,
        compressed_remainder,
        ratio_percent,
        ratio_remainder,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_calculate_compression_stats_basic_compression() {
        // Test basic compression scenario: 1500 bytes -> 750 bytes (50% ratio)
        let (
            uncompressed_kb,
            uncompressed_remainder,
            compressed_kb,
            compressed_remainder,
            ratio_percent,
            ratio_remainder,
        ) = calculate_compression_stats(1500, 750);

        assert_eq!(uncompressed_kb, 1);
        assert_eq!(uncompressed_remainder, 500);
        assert_eq!(compressed_kb, 0);
        assert_eq!(compressed_remainder, 750);
        assert_eq!(ratio_percent, 50);
        assert_eq!(ratio_remainder, 0);
    }

    #[test]
    fn test_calculate_compression_stats_zero_uncompressed_size() {
        // Test edge case: zero uncompressed size (should not cause division by zero)
        let (
            uncompressed_kb,
            uncompressed_remainder,
            compressed_kb,
            compressed_remainder,
            ratio_percent,
            ratio_remainder,
        ) = calculate_compression_stats(0, 100);

        assert_eq!(uncompressed_kb, 0);
        assert_eq!(uncompressed_remainder, 0);
        assert_eq!(compressed_kb, 0);
        assert_eq!(compressed_remainder, 100);
        assert_eq!(ratio_percent, 0);
        assert_eq!(ratio_remainder, 0);
    }

    #[test]
    fn test_calculate_compression_stats_small_sizes() {
        // Test small sizes (less than 1KB): 456 bytes -> 123 bytes
        let (
            uncompressed_kb,
            uncompressed_remainder,
            compressed_kb,
            compressed_remainder,
            ratio_percent,
            ratio_remainder,
        ) = calculate_compression_stats(456, 123);

        assert_eq!(uncompressed_kb, 0);
        assert_eq!(uncompressed_remainder, 456);
        assert_eq!(compressed_kb, 0);
        assert_eq!(compressed_remainder, 123);

        // Calculate expected ratio: (123 * 100) / 456 = 26.97...
        // Integer division gives us 26
        assert_eq!(ratio_percent, 26);
        // For remainder: ((123 * 10000) / 456) % 100
        // = (1230000 / 456) % 100 = 2697 % 100 = 97
        assert_eq!(ratio_remainder, 97);
    }

    #[test]
    fn test_calculate_compression_stats_large_sizes() {
        // Test large sizes: 5MB -> 1MB
        let (
            uncompressed_kb,
            uncompressed_remainder,
            compressed_kb,
            compressed_remainder,
            ratio_percent,
            ratio_remainder,
        ) = calculate_compression_stats(5_000_000, 1_000_000);

        assert_eq!(uncompressed_kb, 5000);
        assert_eq!(uncompressed_remainder, 0);
        assert_eq!(compressed_kb, 1000);
        assert_eq!(compressed_remainder, 0);
        assert_eq!(ratio_percent, 20);
        assert_eq!(ratio_remainder, 0);
    }

    #[test]
    fn test_calculate_compression_stats_no_compression() {
        // Test case where "compressed" size equals original (100% ratio)
        let (
            uncompressed_kb,
            uncompressed_remainder,
            compressed_kb,
            compressed_remainder,
            ratio_percent,
            ratio_remainder,
        ) = calculate_compression_stats(1000, 1000);

        assert_eq!(uncompressed_kb, 1);
        assert_eq!(uncompressed_remainder, 0);
        assert_eq!(compressed_kb, 1);
        assert_eq!(compressed_remainder, 0);
        assert_eq!(ratio_percent, 100);
        assert_eq!(ratio_remainder, 0);
    }

    #[test]
    fn test_calculate_compression_stats_expansion() {
        // Test case where compressed size is larger than original (>100% ratio)
        let (
            uncompressed_kb,
            uncompressed_remainder,
            compressed_kb,
            compressed_remainder,
            ratio_percent,
            ratio_remainder,
        ) = calculate_compression_stats(100, 150);

        assert_eq!(uncompressed_kb, 0);
        assert_eq!(uncompressed_remainder, 100);
        assert_eq!(compressed_kb, 0);
        assert_eq!(compressed_remainder, 150);
        assert_eq!(ratio_percent, 150);
        assert_eq!(ratio_remainder, 0);
    }

    #[test]
    fn test_calculate_compression_stats_precision() {
        // Test precision with specific values that test remainder calculations
        let (
            uncompressed_kb,
            uncompressed_remainder,
            compressed_kb,
            compressed_remainder,
            ratio_percent,
            ratio_remainder,
        ) = calculate_compression_stats(3333, 1111);

        assert_eq!(uncompressed_kb, 3);
        assert_eq!(uncompressed_remainder, 333);
        assert_eq!(compressed_kb, 1);
        assert_eq!(compressed_remainder, 111);

        // Calculate expected ratio: (1111 * 100) / 3333 = 33.33...
        assert_eq!(ratio_percent, 33);
        // For remainder: ((1111 * 10000) / 3333) % 100
        // = (11110000 / 3333) % 100 = 3333 % 100 = 33
        assert_eq!(ratio_remainder, 33);
    }

    #[test]
    fn test_calculate_compression_stats_exact_kilobytes() {
        // Test with exact kilobyte values
        let (
            uncompressed_kb,
            uncompressed_remainder,
            compressed_kb,
            compressed_remainder,
            ratio_percent,
            ratio_remainder,
        ) = calculate_compression_stats(4000, 2000);

        assert_eq!(uncompressed_kb, 4);
        assert_eq!(uncompressed_remainder, 0);
        assert_eq!(compressed_kb, 2);
        assert_eq!(compressed_remainder, 0);
        assert_eq!(ratio_percent, 50);
        assert_eq!(ratio_remainder, 0);
    }

    #[test]
    fn test_calculate_compression_stats_fractional_ratio() {
        // Test a case that produces a fractional ratio
        let (
            uncompressed_kb,
            uncompressed_remainder,
            compressed_kb,
            compressed_remainder,
            ratio_percent,
            ratio_remainder,
        ) = calculate_compression_stats(7, 3);

        assert_eq!(uncompressed_kb, 0);
        assert_eq!(uncompressed_remainder, 7);
        assert_eq!(compressed_kb, 0);
        assert_eq!(compressed_remainder, 3);

        // Calculate expected ratio: (3 * 100) / 7 = 42.857...
        assert_eq!(ratio_percent, 42);
        // For remainder: ((3 * 10000) / 7) % 100
        // = (30000 / 7) % 100 = 4285 % 100 = 85
        assert_eq!(ratio_remainder, 85);
    }

    #[test]
    fn test_log_compression_stats_does_not_panic() {
        // Test that the logging function doesn't panic with various inputs
        // We can't easily test the actual log output without complex setup,
        // but we can ensure it doesn't crash.
        log_compression_stats("Test", 1000, 500);
        log_compression_stats("Test", 0, 100);
        log_compression_stats("Test", 100, 0);
        log_compression_stats("Test", 1, 1);
        log_compression_stats("Test", usize::MAX, 1);
        log_compression_stats("Test", 1, usize::MAX);
    }
}
