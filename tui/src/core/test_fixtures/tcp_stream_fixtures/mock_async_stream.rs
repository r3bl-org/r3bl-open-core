// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::ok;
use std::{io::Result,
          pin::Pin,
          task::{Context, Poll}};
use tokio::io::{AsyncRead, AsyncWrite};

/// A mock struct for the [`tokio::net::TcpStream`].
/// - Alternative to [`tokio_test::io::Builder`] in the `tokio-test` crate.
/// - The difference is that [`MockAsyncStream`] allows access to the expected write
///   buffer.
///
/// [`tokio_test::io::Builder`]: https://docs.rs/tokio-test/latest/tokio_test/io/struct.Builder.html
#[derive(Debug)]
pub struct MockAsyncStream {
    pub expected_buffer: Vec<u8>,
}

/// Implement the [`AsyncWrite`] trait for the mock struct. This struct also automatically
/// implements [Unpin], because it contains no self-referencing pointers.
impl AsyncWrite for MockAsyncStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize>> {
        self.expected_buffer.extend_from_slice(buf);
        Poll::Ready(ok!(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(ok!())
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(ok!())
    }
}

/// Implement the [`AsyncRead`] trait for the mock struct. This struct also automatically
/// implements [Unpin], because it contains no self-referencing pointers.
impl AsyncRead for MockAsyncStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::result::Result<(), std::io::Error>> {
        let data = self.expected_buffer.as_slice();
        let len = std::cmp::min(data.len(), buf.remaining());
        buf.put_slice(&data[..len]);
        self.expected_buffer.drain(..len);
        Poll::Ready(ok!())
    }
}
