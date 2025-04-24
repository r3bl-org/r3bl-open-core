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

use std::{fmt::{Display, Formatter, Result},
          time::Duration};

use crate::ok;

/// This is a wrapper struct around [Duration] so that we can "implement"
/// [std::fmt::Write] for it. This allows us to format the duration in a human readable
/// format.
///
/// For performance reasons this struct does not heap allocate the string, in its
/// [std::fmt::Display] trait implementation and simply uses the [std::fmt::Formatter] to
/// [core::fmt::Write] to a backing store (in some other struct).`
///
/// To create one, you can use the [From] trait to convert from a [Duration].
///
/// ```rust
/// use std::time::Duration;
/// use r3bl_tui::TimeDuration;
/// let duration = Duration::from_secs(1) + Duration::from_millis(100) + Duration::from_micros(100);
/// let time_duration = TimeDuration::from(duration);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TimeDuration {
    pub inner: Duration,
}

mod accessor {
    use super::*;
    impl TimeDuration {
        pub fn get_only_micros(&self) -> u32 { self.inner.subsec_micros() % 1_000 }

        pub fn get_only_millis(&self) -> u32 { self.inner.subsec_millis() }

        pub fn get_only_secs(&self) -> u64 { self.inner.as_secs() }

        pub fn get_as_fps(&self) -> u32 {
            let num_of_micros_in_one_sec = 1_000_000;
            let total_micros = self.inner.as_secs() * num_of_micros_in_one_sec
                + self.inner.subsec_micros() as u64;
            // Avoid division by zero.
            if total_micros > 0 {
                (num_of_micros_in_one_sec / total_micros) as u32
            } else {
                0
            }
        }
    }
}

mod adapters {
    use super::*;
    impl std::ops::Deref for TimeDuration {
        type Target = Duration;

        fn deref(&self) -> &Self::Target { &self.inner }
    }
}

mod converters {
    use super::*;

    impl From<Duration> for TimeDuration {
        fn from(duration: Duration) -> Self { Self { inner: duration } }
    }

    impl From<TimeDuration> for Duration {
        fn from(time_duration: TimeDuration) -> Self { time_duration.inner }
    }

    impl From<&TimeDuration> for Duration {
        fn from(time_duration: &TimeDuration) -> Self { time_duration.inner }
    }
}

mod display_formatter {
    use super::*;

    impl Display for TimeDuration {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            let secs = self.get_only_secs();
            let millis = self.get_only_millis();
            let micros = self.get_only_micros();

            if secs > 3600 {
                let hours = secs / 3600;
                let mins = (secs % 3600) / 60;
                let secs = secs % 60;
                write!(f, "{hours}h:{mins}m:{secs}s{millis:}ms")?;
            } else if secs > 60 {
                let mins = secs / 60;
                let secs = secs % 60;
                write!(f, "{mins}m:{secs}s:{millis}ms")?;
            } else if secs > 0 {
                write!(f, "{secs}s:{millis}ms")?;
            } else if millis > 0 {
                write!(f, "{millis}ms:{micros}μs")?;
            } else {
                write!(f, "{micros}μs")?;
            }

            ok!()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn test_converters() {
        let time_duration = TimeDuration::from(
            Duration::from_secs(3600)
                + Duration::from_secs(1)
                + Duration::from_millis(100)
                + Duration::from_micros(100),
        );
        assert_eq!(format!("{time_duration}"), "1h:0m:1s100ms");

        let time_duration = TimeDuration::from(
            Duration::from_secs(60)
                + Duration::from_secs(1)
                + Duration::from_millis(100)
                + Duration::from_micros(100),
        );
        assert_eq!(format!("{time_duration}"), "1m:1s:100ms");

        let time_duration = TimeDuration::from(
            Duration::from_secs(1)
                + Duration::from_millis(100)
                + Duration::from_micros(100),
        );
        assert_eq!(format!("{time_duration}"), "1s:100ms");
    }
}
