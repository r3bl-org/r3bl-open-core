// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module is standalone, you can use it any project that needs to communicate
//! between a client and a server using a length-prefix, binary payload, protocol.

use super::{compress::{self, CompressionError},
            json_serde::{self, JsonSerdeError}};
use crate::{LengthPrefixType, NarrowingCastToUsize};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::{io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader,
                 BufWriter},
            time::timeout};

/// Error returned by [`handshake`] and [`byte_io`] protocol operations.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum LengthPrefixProtocolError {
    /// Handshake did not complete within [`protocol_constants::TIMEOUT_DURATION`].
    #[error("Handshake timed out")]
    HandshakeTimeout(#[source] tokio::time::error::Elapsed),

    /// Peer sent an unexpected protocol magic number.
    #[error("Invalid protocol magic number: expected {expected:#x}, got {actual:#x}")]
    InvalidMagicNumber { expected: u64, actual: u64 },

    /// Peer sent an unsupported protocol version.
    #[error("Invalid protocol version: expected {expected}, got {actual}")]
    InvalidProtocolVersion { expected: u64, actual: u64 },

    /// Received payload length prefix exceeded [`protocol_constants::MAX_PAYLOAD_SIZE`].
    #[error("Payload size {size} exceeds maximum allowed {max}")]
    PayloadTooLarge { size: u64, max: u64 },

    /// Outgoing payload byte length could not fit into [`LengthPrefixType`].
    #[error("Payload byte length {size} exceeds protocol length integer bounds")]
    PayloadLengthOverflow {
        size: usize,
        #[source]
        src_err: std::num::TryFromIntError,
    },

    /// Underlying async stream read, write, or flush failed.
    #[error("Protocol stream I/O failed")]
    Io(#[source] std::io::Error),

    /// Payload compression or decompression failed.
    #[error(transparent)]
    #[diagnostic(transparent)]
    Compression(CompressionError),

    /// Payload JSON serialization or deserialization failed.
    #[error(transparent)]
    #[diagnostic(transparent)]
    JsonSerde(JsonSerdeError),
}

pub mod protocol_constants {
    use super::Duration;

    pub const MAGIC_NUMBER: u64 = 0xACED_FACE_BABE_CAFE; // DEED, CEDE, FADE
    pub const PROTOCOL_VERSION: u64 = 1;
    pub const TIMEOUT_DURATION: Duration = Duration::from_secs(1);
    pub const MAX_PAYLOAD_SIZE: u64 = 10_000_000;
}

/// Extend the protocol to validate that it is connecting to the correct type of server,
/// by implementing the following handshake mechanism:
///
/// # Client side - [`handshake::try_connect_or_timeout`]
/// 1. The client **writes** a "magic number" or protocol identifier, and version number
///    as the first message when establishing a connection.
/// 2. This number is then **read** back from the server to ensure that it is valid.
///
/// # Server side - [`handshake::try_accept_or_timeout`]
/// 1. The server **reads** the magic number and protocol version number, and checks to
///    make sure they are valid.
/// 2. It then **writes** the magic number back to the client (for it to validate).
pub mod handshake {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Client side handshake.
    ///
    /// # Errors
    ///
    /// Returns [`LengthPrefixProtocolError`] if the handshake times out, magic number
    /// validation fails, or stream I/O fails.
    pub async fn try_connect_or_timeout<W: AsyncWrite + Unpin, R: AsyncRead + Unpin>(
        read_half: &mut R,
        write_half: &mut W,
    ) -> Result<(), LengthPrefixProtocolError> {
        let res_timeout = timeout(
            protocol_constants::TIMEOUT_DURATION,
            try_connect(read_half, write_half),
        )
        .await;

        match res_timeout {
            Ok(res_handshake) => res_handshake,
            Err(elapsed) => Err(LengthPrefixProtocolError::HandshakeTimeout(elapsed)),
        }
    }

    #[allow(clippy::missing_errors_doc)]
    async fn try_connect<W: AsyncWrite + Unpin, R: AsyncRead + Unpin>(
        read_half: &mut R,
        write_half: &mut W,
    ) -> Result<(), LengthPrefixProtocolError> {
        // Send the magic number.
        write_half
            .write_u64(protocol_constants::MAGIC_NUMBER)
            .await
            .map_err(LengthPrefixProtocolError::Io)?;

        // Send the protocol version.
        write_half
            .write_u64(protocol_constants::PROTOCOL_VERSION)
            .await
            .map_err(LengthPrefixProtocolError::Io)?;

        // Flush the buffer.
        write_half
            .flush()
            .await
            .map_err(LengthPrefixProtocolError::Io)?;

        // Read the magic number back from the server.
        let received_magic_number = read_half
            .read_u64()
            .await
            .map_err(LengthPrefixProtocolError::Io)?;
        if received_magic_number != protocol_constants::MAGIC_NUMBER {
            return Err(LengthPrefixProtocolError::InvalidMagicNumber {
                expected: protocol_constants::MAGIC_NUMBER,
                actual: received_magic_number,
            });
        }

        ok!()
    }

    /// Server side handshake.
    ///
    /// # Errors
    ///
    /// Returns [`LengthPrefixProtocolError`] if the handshake times out, magic number or
    /// version validation fails, or stream I/O fails.
    pub async fn try_accept_or_timeout<W: AsyncWrite + Unpin, R: AsyncRead + Unpin>(
        read_half: &mut R,
        write_half: &mut W,
    ) -> Result<(), LengthPrefixProtocolError> {
        let res_timeout = timeout(
            protocol_constants::TIMEOUT_DURATION,
            try_accept(read_half, write_half),
        )
        .await;

        match res_timeout {
            Ok(res_handshake) => res_handshake,
            Err(elapsed) => Err(LengthPrefixProtocolError::HandshakeTimeout(elapsed)),
        }
    }

    #[allow(clippy::missing_errors_doc)]
    async fn try_accept<W: AsyncWrite + Unpin, R: AsyncRead + Unpin>(
        read_half: &mut R,
        write_half: &mut W,
    ) -> Result<(), LengthPrefixProtocolError> {
        // Read and validate the magic number.
        let received_magic_number = read_half
            .read_u64()
            .await
            .map_err(LengthPrefixProtocolError::Io)?;
        if received_magic_number != protocol_constants::MAGIC_NUMBER {
            return Err(LengthPrefixProtocolError::InvalidMagicNumber {
                expected: protocol_constants::MAGIC_NUMBER,
                actual: received_magic_number,
            });
        }

        // Read and validate the protocol version.
        let received_protocol_version = read_half
            .read_u64()
            .await
            .map_err(LengthPrefixProtocolError::Io)?;
        if received_protocol_version != protocol_constants::PROTOCOL_VERSION {
            return Err(LengthPrefixProtocolError::InvalidProtocolVersion {
                expected: protocol_constants::PROTOCOL_VERSION,
                actual: received_protocol_version,
            });
        }

        // Write the magic number back to the client.
        write_half
            .write_u64(protocol_constants::MAGIC_NUMBER)
            .await
            .map_err(LengthPrefixProtocolError::Io)?;

        ok!()
    }
}

#[cfg(test)]
mod tests_handshake {
    use super::*;
    use crate::{MockSocket, get_mock_socket_halves};

    #[tokio::test]
    async fn test_handshake() {
        let MockSocket {
            mut client_read,
            mut client_write,
            mut server_read,
            mut server_write,
        } = get_mock_socket_halves();

        let client_handshake =
            handshake::try_connect_or_timeout(&mut client_read, &mut client_write);

        let server_handshake =
            handshake::try_accept_or_timeout(&mut server_read, &mut server_write);

        let (client_handshake_result, server_handshake_result) =
            tokio::join!(client_handshake, server_handshake);

        assert!(client_handshake_result.is_ok());
        assert!(server_handshake_result.is_ok());
    }
}

pub mod byte_io {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Writes the payload to the client. Use the length-prefix, binary payload, protocol.
    /// - The trait bounds on this function are so that this function can be tested w/ a
    ///   mock from `tokio_test::io::Builder`.
    /// - More info on [tokio testing].
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Serialization of the data fails
    /// - Compression of the payload fails
    /// - Writing to the buffer fails
    /// - Flushing the buffer fails
    ///
    /// [tokio testing]: https://tokio.rs/tokio/topics/testing
    pub async fn try_write<W: AsyncWrite + Unpin, T: Serialize>(
        buf_writer: &mut BufWriter<W>,
        data: &T,
    ) -> Result<(), LengthPrefixProtocolError> {
        // Try to serialize the data.
        let payload_buffer = json_serde::try_serialize(data)
            .map_err(LengthPrefixProtocolError::JsonSerde)?;

        // Compress the payload.
        let payload_buffer = compress::try_compress(&payload_buffer)
            .map_err(LengthPrefixProtocolError::Compression)?;

        // Write the length prefix number of bytes.
        let payload_size = payload_buffer.len();
        let payload_size_u64 =
            LengthPrefixType::try_from(payload_size).map_err(|src_err| {
                LengthPrefixProtocolError::PayloadLengthOverflow {
                    size: payload_size,
                    src_err,
                }
            })?;
        buf_writer
            .write_u64(payload_size_u64)
            .await
            .map_err(LengthPrefixProtocolError::Io)?;

        // Write the payload.
        buf_writer
            .write_all(&payload_buffer)
            .await
            .map_err(LengthPrefixProtocolError::Io)?;

        // Flush the buffer.
        buf_writer
            .flush()
            .await
            .map_err(LengthPrefixProtocolError::Io)?;

        ok!()
    }

    /// Ready the payload from the client. Use the length-prefix [`LengthPrefixType`],
    /// binary payload, protocol.
    /// - The trait bounds on this function are so that this function can be tested w/ a
    ///   mock from `tokio_test::io::Builder`.
    /// - More info on [tokio testing].
    ///
    /// # Errors
    ///
    /// Returns [`LengthPrefixProtocolError`] if reading, size validation, decompression,
    /// or deserialization fails.
    ///
    /// [tokio testing]: https://tokio.rs/tokio/topics/testing
    pub async fn try_read<R: AsyncRead + Unpin, T: for<'d> Deserialize<'d>>(
        buf_reader: &mut BufReader<R>,
    ) -> Result<T, LengthPrefixProtocolError> {
        // Read the length prefix number of bytes.
        let size_of_payload = buf_reader
            .read_u64()
            .await
            .map_err(LengthPrefixProtocolError::Io)?;

        // Ensure that the payload size is within the expected range.
        if size_of_payload > protocol_constants::MAX_PAYLOAD_SIZE {
            // Adjust this threshold as needed.
            return Err(LengthPrefixProtocolError::PayloadTooLarge {
                size: size_of_payload,
                max: protocol_constants::MAX_PAYLOAD_SIZE,
            });
        }

        // Read the payload.

        let size_of_payload = size_of_payload.as_usize_narrowing();

        let mut payload_buffer = vec![0; size_of_payload];
        buf_reader
            .read_exact(&mut payload_buffer)
            .await
            .map_err(LengthPrefixProtocolError::Io)?;

        // Decompress the payload.
        let payload_buffer = compress::try_decompress(&payload_buffer)
            .map_err(LengthPrefixProtocolError::Compression)?;

        // Deserialize the payload.
        let payload_buffer = json_serde::try_deserialize::<T>(&payload_buffer)
            .map_err(LengthPrefixProtocolError::JsonSerde)?;
        Ok(payload_buffer)
    }
}

#[cfg(test)]
mod tests_byte_io {
    use super::*;
    use crate::{MockSocket, get_mock_socket_halves};

    pub fn get_all_client_messages<'a>() -> Vec<&'a str> { vec!["one", "two", "three"] }

    #[tokio::test]
    async fn test_byte_io() {
        let MockSocket {
            client_read: _,
            mut client_write,
            mut server_read,
            server_write: _,
        } = get_mock_socket_halves();

        for sent_payload in get_all_client_messages() {
            byte_io::try_write(&mut BufWriter::new(&mut client_write), &sent_payload)
                .await
                .expect("conversion error");

            let received_payload: String =
                byte_io::try_read(&mut BufReader::new(&mut server_read))
                    .await
                    .expect("conversion error");

            assert_eq!(received_payload, sent_payload);
        }
    }
}
