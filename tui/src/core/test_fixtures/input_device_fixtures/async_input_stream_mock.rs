// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::time::Duration;

use async_stream::stream;

use crate::{InlineVec, PinnedInputStream};

/// The main constructors are:
/// - [`super::InputDeviceExtMock::new_mock()`]
/// - [`super::InputDeviceExtMock::new_mock_with_delay()`]
pub fn gen_input_stream<T>(generator_vec: InlineVec<T>) -> PinnedInputStream<T>
where
    T: Send + Sync + 'static,
{
    let it = stream! {
        for item in generator_vec {
            yield item;
        }
    };
    Box::pin(it)
}

pub fn gen_input_stream_with_delay<T>(
    generator_vec: InlineVec<T>,
    delay: Duration,
) -> PinnedInputStream<T>
where
    T: Send + Sync + 'static,
{
    let it = stream! {
        for item in generator_vec {
            tokio::time::sleep(delay).await;
            yield item;
        }
    };
    Box::pin(it)
}

#[cfg(test)]
mod tests {
    use futures_util::StreamExt;
    use smallvec::smallvec;

    use super::*;

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_gen_input_stream() {
        let mut input_stream = gen_input_stream(smallvec![1, 2, 3]);
        for _ in 1..=3 {
            input_stream.next().await;
        }
        pretty_assertions::assert_eq!(input_stream.next().await, None);
    }

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_gen_input_stream_with_delay() {
        const DELAY: u64 = 100;

        // Start timer.
        let start_time = std::time::Instant::now();

        let mut input_stream =
            gen_input_stream_with_delay(smallvec![1, 2, 3], Duration::from_millis(DELAY));
        for _ in 1..=3 {
            input_stream.next().await;
        }

        // End timer.
        let end_time = std::time::Instant::now();

        pretty_assertions::assert_eq!(input_stream.next().await, None);

        assert!(end_time - start_time >= Duration::from_millis(DELAY * 3));
    }
}
