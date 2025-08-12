// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use tokio::io::{DuplexStream, ReadHalf, WriteHalf, duplex, split};

#[derive(Debug)]
pub struct MockSocket {
    pub client_read: ReadHalf<DuplexStream>,
    pub client_write: WriteHalf<DuplexStream>,
    pub server_read: ReadHalf<DuplexStream>,
    pub server_write: WriteHalf<DuplexStream>,
}

/// A “channel” is created by [`tokio::io::duplex`] that can be used as in-memory IO
/// types.
///
/// Given a "channel":
/// 1. Writing to the first of the pairs will allow that data to be read from the other.
/// 2. Writing to the other pair will allow that data to be read from the first.
#[must_use]
pub fn get_mock_socket_halves() -> MockSocket {
    let (client_stream, server_stream) = duplex(1024);

    let (client_read, client_write) = split(client_stream);

    let (server_read, server_write) = split(server_stream);

    MockSocket {
        client_read,
        client_write,
        server_read,
        server_write,
    }
}
