// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module is standalone, you can use it any project that needs to communicate
//! between a client and a server using a length-prefix, binary payload, protocol.

use std::time::Duration;

use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize};
use tokio::{io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader,
                 BufWriter},
            time::timeout};

use crate::{bincode_serde, compress, ok, protocol_types::LengthPrefixType};

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
    /// Returns an error if:
    /// - The handshake times out
    /// - The magic number validation fails
    /// - I/O errors occur during read/write operations
    pub async fn try_connect_or_timeout<W: AsyncWrite + Unpin, R: AsyncRead + Unpin>(
        read_half: &mut R,
        write_half: &mut W,
    ) -> miette::Result<()> {
        let result = timeout(
            protocol_constants::TIMEOUT_DURATION,
            try_connect(read_half, write_half),
        )
        .await;

        match result {
            Ok(Err(handshake_err)) => {
                miette::bail!("Handshake failed due to: {}", handshake_err.root_cause())
            }
            Err(_elapsed_err) => {
                miette::bail!("Handshake timed out")
            }
            _ => {
                ok!()
            }
        }
    }

    #[allow(clippy::missing_errors_doc)]
    async fn try_connect<W: AsyncWrite + Unpin, R: AsyncRead + Unpin>(
        read_half: &mut R,
        write_half: &mut W,
    ) -> miette::Result<()> {
        // Send the magic number.
        write_half
            .write_u64(protocol_constants::MAGIC_NUMBER)
            .await
            .into_diagnostic()?;

        // Send the protocol version.
        write_half
            .write_u64(protocol_constants::PROTOCOL_VERSION)
            .await
            .into_diagnostic()?;

        // Flush the buffer.
        write_half.flush().await.into_diagnostic()?;

        // Read the magic number back from the server.
        let received_magic_number = read_half.read_u64().await.into_diagnostic()?;
        if received_magic_number != protocol_constants::MAGIC_NUMBER {
            miette::bail!("Invalid protocol magic number")
        }

        ok!()
    }

    /// Server side handshake.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The handshake times out
    /// - The magic number validation fails
    /// - I/O errors occur during read/write operations
    pub async fn try_accept_or_timeout<W: AsyncWrite + Unpin, R: AsyncRead + Unpin>(
        read_half: &mut R,
        write_half: &mut W,
    ) -> miette::Result<()> {
        let result = timeout(
            protocol_constants::TIMEOUT_DURATION,
            try_accept(read_half, write_half),
        )
        .await
        .into_diagnostic();

        match result {
            Ok(handshake_result) => match handshake_result {
                Ok(()) => ok!(),
                Err(handshake_err) => {
                    miette::bail!(
                        "Handshake failed due to: {}",
                        handshake_err.root_cause()
                    )
                }
            },
            Err(_elapsed_err) => miette::bail!("Handshake timed out"),
        }
    }

    #[allow(clippy::missing_errors_doc)]
    async fn try_accept<W: AsyncWrite + Unpin, R: AsyncRead + Unpin>(
        read_half: &mut R,
        write_half: &mut W,
    ) -> miette::Result<()> {
        // Read and validate the magic number.
        let received_magic_number = read_half.read_u64().await.into_diagnostic()?;
        if received_magic_number != protocol_constants::MAGIC_NUMBER {
            miette::bail!("Invalid protocol magic number")
        }

        // Read and validate the protocol version.
        let received_protocol_version = read_half.read_u64().await.into_diagnostic()?;
        if received_protocol_version != protocol_constants::PROTOCOL_VERSION {
            miette::bail!("Invalid protocol version")
        }

        // Write the magic number back to the client.
        write_half
            .write_u64(protocol_constants::MAGIC_NUMBER)
            .await
            .into_diagnostic()?;

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

    /// Write the payload to the client. Use the length-prefix, binary payload, protocol.
    /// - The trait bounds on this function are so that this function can be tested w/ a
    ///   mock from `tokio_test::io::Builder`.
    /// - More info: <https://tokio.rs/tokio/topics/testing>
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Serialization of the data fails
    /// - Compression of the payload fails
    /// - Writing to the buffer fails
    /// - Flushing the buffer fails
    pub async fn try_write<W: AsyncWrite + Unpin, T: Serialize>(
        buf_writer: &mut BufWriter<W>,
        data: &T,
    ) -> miette::Result<()> {
        // Try to serialize the data.
        let payload_buffer = bincode_serde::try_serialize(data)?;

        // Compress the payload.
        let payload_buffer = compress::compress(&payload_buffer)?;

        // Write the length prefix number of bytes.
        let payload_size = payload_buffer.len();
        buf_writer
            .write_u64(payload_size as LengthPrefixType)
            .await
            .into_diagnostic()?;

        // Write the payload.
        buf_writer
            .write_all(&payload_buffer)
            .await
            .into_diagnostic()?;

        // Flush the buffer.
        buf_writer.flush().await.into_diagnostic()?;

        Ok(())
    }

    /// Ready the payload from the client. Use the length-prefix [`LengthPrefixType`],
    /// binary payload, protocol.
    /// - The trait bounds on this function are so that this function can be tested w/ a
    ///   mock from `tokio_test::io::Builder`.
    /// - More info: <https://tokio.rs/tokio/topics/testing>
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Reading from the buffer fails
    /// - The payload size exceeds the maximum allowed size
    /// - Decompression of the payload fails
    /// - Deserialization of the data fails
    pub async fn try_read<R: AsyncRead + Unpin, T: for<'d> Deserialize<'d>>(
        buf_reader: &mut BufReader<R>,
    ) -> miette::Result<T> {
        // Read the length prefix number of bytes.
        let size_of_payload = buf_reader.read_u64().await.into_diagnostic()?;

        // Ensure that the payload size is within the expected range.
        if size_of_payload > protocol_constants::MAX_PAYLOAD_SIZE {
            // Adjust this threshold as needed
            miette::bail!("Payload size is too large")
        }

        // Read the payload.

        // This an intentional cast to `usize` because the payload size is a
        // `LengthPrefixType`, which is a `u64`.
        #[allow(clippy::cast_possible_truncation)]
        let size_of_payload = size_of_payload as usize;

        let mut payload_buffer = vec![0; size_of_payload];
        buf_reader
            .read_exact(&mut payload_buffer)
            .await
            .into_diagnostic()?;

        // Decompress the payload.
        let payload_buffer = compress::decompress(&payload_buffer)?;

        // Deserialize the payload.
        let payload_buffer = bincode_serde::try_deserialize::<T>(&payload_buffer)?;
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
                .unwrap();

            let received_payload: String =
                byte_io::try_read(&mut BufReader::new(&mut server_read))
                    .await
                    .unwrap();

            assert_eq!(received_payload, sent_payload);
        }
    }
}
