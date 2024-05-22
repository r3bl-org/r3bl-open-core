/*
 *   Copyright (c) 2024 R3BL LLC
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

use miette::{Context, IntoDiagnostic};

pub fn find_first_available_port_in_range(
    maybe_range: Option<std::ops::Range<u16>>,
    host: &str,
) -> Option<u16> {
    let mut range = maybe_range.unwrap_or(8000..9000);
    range.find(|port| std::net::TcpListener::bind((host, *port)).is_ok())
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Status {
    Free,
    Occupied,
}

/// Check whether a port is available. Returns an error if there is an issue creating a
/// TCP socket. Does not return an error, if the port is not available.
pub async fn check(addr: std::net::SocketAddr) -> miette::Result<Status> {
    let socket = tokio::net::TcpSocket::new_v4()
        .into_diagnostic()
        .with_context(|| "Failed to create a new TCP socket".to_string())?;
    let result_socket_connect = socket.connect(addr).await;
    match result_socket_connect {
        Ok(_) => Ok(Status::Occupied),
        Err(_) => Ok(Status::Free),
    }
}

#[tokio::test]
async fn test_is_port_available() {
    // Find a free port. Check if it's available.
    let host = "127.0.0.1";
    let free_port = find_first_available_port_in_range(None, host).unwrap();
    let addr_str = format!("{}:{}", host, free_port);

    use std::str::FromStr;
    let addr = std::net::SocketAddr::from_str(&addr_str).unwrap();

    assert_eq!(check(addr).await.unwrap(), Status::Free);

    // Occupy that port. Check if it's not available.
    let _listener = std::net::TcpListener::bind(addr).unwrap();
    assert_eq!(check(addr).await.unwrap(), Status::Occupied);
}
