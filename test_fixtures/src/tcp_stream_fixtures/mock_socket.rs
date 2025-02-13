/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

use tokio::io::{DuplexStream, ReadHalf, WriteHalf, duplex, split};

pub struct MockSocket {
    pub client_read: ReadHalf<DuplexStream>,
    pub client_write: WriteHalf<DuplexStream>,
    pub server_read: ReadHalf<DuplexStream>,
    pub server_write: WriteHalf<DuplexStream>,
}

/// A “channel” is created by [tokio::io::duplex] that can be used as in-memory IO
/// types.
///
/// Given a "channel":
/// 1. Writing to the first of the pairs will allow that data to be read from the
///    other.
/// 2. Writing to the other pair will allow that data to be read from the first.
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
