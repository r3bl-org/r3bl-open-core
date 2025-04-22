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

/// Compress the payload using the [flate2] crate.
pub fn compress(data: &[BufferAtom]) -> miette::Result<Buffer> {
    let uncompressed_size = data.len();
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).into_diagnostic()?;
    let it = encoder.finish().into_diagnostic();
    let compressed_size = it.as_ref().map(|it| it.len()).unwrap_or(0);
    tracing::info!(
        "Compression: {:.2} kb -> {:.2} kb ({:.2}%)",
        uncompressed_size as f64 / 1000.0,
        compressed_size as f64 / 1000.0,
        (compressed_size as f64 / uncompressed_size as f64) * 100.0
    );
    it
}

/// Decompress the payload using the [flate2] crate.
pub fn decompress(data: &[BufferAtom]) -> miette::Result<Buffer> {
    let compressed_size = data.len();
    let mut decoder = GzDecoder::new(data);
    let mut decompressed_data = Vec::new();
    decoder
        .read_to_end(&mut decompressed_data)
        .into_diagnostic()?;
    let uncompressed_size = decompressed_data.len();
    tracing::info!(
        "Decompression: {:.2} kb -> {:.2} kb ({:.2}%)",
        uncompressed_size as f64 / 1000.0,
        compressed_size as f64 / 1000.0,
        (compressed_size as f64 / uncompressed_size as f64) * 100.0
    );
    Ok(decompressed_data)
}
