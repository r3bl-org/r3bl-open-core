// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.
use std::{collections::HashMap,
          fmt::Display,
          time::{Duration, Instant}};

use smallstr::SmallString;
use strum_macros::{Display, EnumString};

use crate::{Pc, RateLimitStatus, RateLimiter, RingBuffer, RingBufferStack, TimeDuration,
            glyphs};

pub mod telemetry_sizing {
    use super::SmallString;

    pub type TelemetryReportLineStorage = SmallString<[u8; TELEMETRY_REPORT_STRING_SIZE]>;

    pub const TELEMETRY_REPORT_STRING_SIZE: usize = 128;
}

/// These are the default constants for the telemetry module. They are reasonable
/// defaults, but you can override them to suit your needs.
pub mod telemetry_default_constants {
    use super::Duration;

    /// The size of the ring buffer to store the response times.
    pub const RING_BUFFER_SIZE: usize = 100;

    /// The rate limiter for generating the report is set to run once every `n sec`.
    pub const RATE_LIMIT_TIME_THRESHOLD: Duration =
        Duration::from_millis(/* 16ms */ 16);

    /// Any response time below this threshold will be filtered out.
    pub const FILTER_MIN_RESPONSE_TIME: Duration = Duration::from_micros(20);

    /// Calculate cluster sensitivity range based on the `min_duration_filter`.
    /// - If `min_duration_filter` is `Some(filter)` and filter > 0, uses `filter * 5`
    /// - Otherwise, uses the default `FILTER_MIN_RESPONSE_TIME * 5`
    #[must_use]
    pub fn calculate_cluster_sensitivity_range(
        min_duration_filter: Option<Duration>,
    ) -> Duration {
        match min_duration_filter {
            Some(min_filter) if min_filter.as_micros() > 0 => {
                let micros = min_filter.as_micros().saturating_mul(5);
                // Safely convert u128 to u64, clamping to u64::MAX if needed.
                let micros_u64 = u64::try_from(micros).unwrap_or(u64::MAX);
                Duration::from_micros(micros_u64)
            }
            _ => {
                let micros = FILTER_MIN_RESPONSE_TIME.as_micros().saturating_mul(5);
                let micros_u64 = u64::try_from(micros).unwrap_or(u64::MAX);
                Duration::from_micros(micros_u64)
            }
        }
    }
}

/// You can use this struct to track the response times of an operation. It stores the
/// response times in a ring buffer and provides methods to calculate the average, min,
/// max, median, and session duration. It also provides a method to generate
/// a report of the response times.
///
/// 1. The report is rate limited to run once every `n sec`, where `n` is the time
///    duration defined in [`Self::rate_limiter_generate_report`].
/// 2. You can also filter out the lowest response times by providing a minimum duration
///    filter in [`Self::min_duration_filter`].
///
/// # Examples
///
/// You have a lot of flexibility in constructing this, using
/// [`mod@telemetry_constructor`] and the [`Telemetry::new`] constructor function.
///
/// ```
/// use std::time::Duration;
/// use r3bl_tui::Telemetry;
///
/// let buffer = Telemetry::<5>::new(());
/// let buffer_with_duration = Telemetry::<5>::new(
///     Duration::from_secs(1)
/// );
/// let buffer_with_rate_limit_duration_and_filter_min_duration =
///     Telemetry::<5>::new((
///         Duration::from_secs(1),
///         Duration::from_micros(100)
/// ));
/// ```
#[derive(Debug, PartialEq)]
pub struct Telemetry<const N: usize> {
    pub ring_buffer: RingBufferStack<TelemetryAtom, N>,
    pub start_timestamp: Instant,
    /// Pre-allocated buffer to store the report (after generating it). This is a cache
    /// that is used to avoid generating the report too frequently (rate limited with
    /// [`Self::rate_limiter_generate_report`]).
    pub report: TelemetryHudReport,
    pub rate_limiter_generate_report: RateLimiter,
    pub min_duration_filter: Option<Duration>,
    pub cluster_sensitivity_range: Duration,
}

#[derive(Debug, PartialEq, Copy, Clone, EnumString, Display, Eq, Hash, Default)]
pub enum TelemetryAtomHint {
    #[strum(serialize = "REND")]
    Render,
    #[strum(serialize = "SIGN")]
    Signal,
    #[strum(serialize = "RESZ")]
    Resize,
    #[strum(serialize = "INPT")]
    Input,
    #[strum(serialize = "NONE")]
    #[default]
    None,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct TelemetryAtom {
    pub duration: Duration,
    pub hint: TelemetryAtomHint,
}

impl TelemetryAtom {
    #[must_use]
    pub fn new(duration: Duration, hint: TelemetryAtomHint) -> Self {
        Self { duration, hint }
    }

    #[must_use]
    pub fn as_duration(&self) -> Duration { self.duration }
}

/// - Calls the [`Telemetry::record_start_auto_stop`] method.
/// - Runs `$block`, and then drops the handle to stop recording the response time.
/// - Finally it calls `$after_block`.
#[macro_export]
macro_rules! telemetry_record {
    (
        @telemetry: $telemetry:ident,
        @hint: $hint:expr,
        @block: $block:block,
        @after_block: $after_block:block
    ) => {{
        let _stop_record_on_drop = $telemetry.record_start_auto_stop($hint);
        $block
        drop(_stop_record_on_drop);
        $after_block
    }};
}

pub mod telemetry_constructor {
    use super::{Duration, Instant, RateLimiter, RingBufferStack, Telemetry,
                TelemetryHudReport, TimeDuration, telemetry_default_constants};

    #[derive(Debug)]
    pub struct ResponseTimesRingBufferOptions {
        pub rate_limit_min_time_threshold: Duration,
        pub min_duration_filter: Option<Duration>,
        pub cluster_sensitivity_range: Duration,
    }

    impl From<()> for ResponseTimesRingBufferOptions {
        fn from((): ()) -> Self {
            let min_duration_filter =
                Some(telemetry_default_constants::FILTER_MIN_RESPONSE_TIME);
            Self {
                rate_limit_min_time_threshold:
                    telemetry_default_constants::RATE_LIMIT_TIME_THRESHOLD,
                min_duration_filter,
                cluster_sensitivity_range:
                    telemetry_default_constants::calculate_cluster_sensitivity_range(
                        min_duration_filter,
                    ),
            }
        }
    }

    impl From<Duration> for ResponseTimesRingBufferOptions {
        fn from(rate_limit_min_time_threshold: Duration) -> Self {
            let min_duration_filter =
                Some(telemetry_default_constants::FILTER_MIN_RESPONSE_TIME);
            Self {
                rate_limit_min_time_threshold,
                min_duration_filter,
                cluster_sensitivity_range:
                    telemetry_default_constants::calculate_cluster_sensitivity_range(
                        min_duration_filter,
                    ),
            }
        }
    }

    impl From<(Duration, Duration)> for ResponseTimesRingBufferOptions {
        fn from(
            (rate_limit_min_time_threshold, min_duration_filter): (Duration, Duration),
        ) -> Self {
            let min_duration_filter = Some(min_duration_filter);
            Self {
                rate_limit_min_time_threshold,
                min_duration_filter,
                cluster_sensitivity_range:
                    telemetry_default_constants::calculate_cluster_sensitivity_range(
                        min_duration_filter,
                    ),
            }
        }
    }

    // XMARK: Clever Rust, use of `impl Into<struct>` for constructor & `const N: usize`
    // for arrays.

    impl<const N: usize> Telemetry<N> {
        pub fn new(arg_opts: impl Into<ResponseTimesRingBufferOptions>) -> Self {
            // "Dynamically" convert the options argument into the actual options struct.
            let options: ResponseTimesRingBufferOptions = arg_opts.into();
            Self {
                ring_buffer: RingBufferStack::new(),
                start_timestamp: Instant::now(),
                report: TelemetryHudReport::default(),
                rate_limiter_generate_report: RateLimiter::new(
                    options.rate_limit_min_time_threshold,
                ),
                min_duration_filter: options.min_duration_filter,
                cluster_sensitivity_range: options.cluster_sensitivity_range,
            }
        }

        #[must_use]
        pub fn session_duration(&self) -> TimeDuration {
            let it = self.start_timestamp.elapsed();
            TimeDuration::from(it)
        }
    }

    impl<const N: usize> Default for Telemetry<N> {
        fn default() -> Self { Self::new(()) }
    }
}

mod mutator {
    use super::{Instant, RingBuffer, Telemetry, TelemetryAtom, TelemetryAtomHint};

    #[derive(Debug, PartialEq)]
    pub struct RecordStartDropHandle<'a, const N: usize> {
        telemetry_ref_mut: &'a mut Telemetry<N>,
        start_timestamp: Instant,
        hint: TelemetryAtomHint,
    }

    impl<'a, const N: usize> RecordStartDropHandle<'a, N> {
        pub fn new(
            telemetry_ref_mut: &'a mut Telemetry<N>,
            hint: TelemetryAtomHint,
        ) -> Self {
            Self {
                telemetry_ref_mut,
                start_timestamp: Instant::now(),
                hint,
            }
        }
    }

    impl<const N: usize> Drop for RecordStartDropHandle<'_, N> {
        fn drop(&mut self) {
            let time_elapsed = self.start_timestamp.elapsed();
            self.telemetry_ref_mut
                .try_record(TelemetryAtom::new(time_elapsed, self.hint));
        }
    }

    #[derive(Debug, PartialEq)]
    pub enum TryRecordResult {
        Ok,
        FilteredOut,
    }

    impl<const N: usize> Telemetry<N> {
        /// Start recording the response time.
        /// 1. This will record the time when the operation started.
        /// 2. This returns a handle that will automatically stop recording the response
        ///    time when it's dropped.
        pub fn record_start_auto_stop(
            &mut self,
            hint: TelemetryAtomHint,
        ) -> RecordStartDropHandle<'_, N> {
            RecordStartDropHandle::new(self, hint)
        }

        /// Insert a new response time into the ring buffer. And sort the internal
        /// storage. If the response time is below the minimum duration filter, it
        /// will be filtered out.
        pub fn try_record(&mut self, atom: TelemetryAtom) -> TryRecordResult {
            if self
                .min_duration_filter
                .is_none_or(|min| atom.duration >= min)
            {
                self.ring_buffer.add(atom);
                TryRecordResult::Ok
            } else {
                TryRecordResult::FilteredOut
            }
        }
    }
}

mod calculator {
    use super::{Duration, HashMap, Pc, RingBuffer, Telemetry, TelemetryAtom,
                TelemetryAtomHint, TimeDuration};

    impl<const N: usize> Telemetry<N> {
        #[must_use]
        pub fn average(&self) -> Option<TimeDuration> {
            // Calling sum() on an empty iterator will return 0.
            if self.ring_buffer.is_empty() {
                return None;
            }
            let sum: Duration = self
                .ring_buffer
                .iter()
                .map(TelemetryAtom::as_duration)
                .sum();
            let avg: Duration = sum / self.ring_buffer.len().as_u32();
            Some(TimeDuration::from(avg))
        }

        pub fn min(&self) -> Option<TimeDuration> {
            // Calling min() on an empty iterator will return None.
            let maybe_min = self
                .ring_buffer
                .iter()
                .map(TelemetryAtom::as_duration)
                .min();
            maybe_min.map(TimeDuration::from)
        }

        pub fn max(&self) -> Option<TimeDuration> {
            // Calling max() on an empty iterator will return None.
            let maybe_max = self
                .ring_buffer
                .iter()
                .map(TelemetryAtom::as_duration)
                .max();
            maybe_max.map(TimeDuration::from)
        }

        /// Find the most common cluster of durations within a specified range in an array
        /// of [`Duration`]. The cluster sensitivity range is configured during
        /// construction in
        /// [`super::telemetry_constructor::ResponseTimesRingBufferOptions`] and
        /// automatically calculated based on the `min_duration_filter`:
        /// - If a custom `min_duration_filter` is set, uses `min_duration_filter * 5`
        /// - Otherwise, uses the default (5x the filter minimum response time)
        ///
        /// This function creates a **frequency histogram** of duration measurements using
        /// a bucketing strategy to group similar durations together. This approach is
        /// more useful for performance analysis than exact timing matches, since
        /// exact matches are rare in real-world telemetry data.
        ///
        /// ## Algorithm
        ///
        /// 1. **Bucketing**: Each duration is converted to microseconds and divided by
        ///    the cluster sensitivity range to create bucket keys that group similar
        ///    durations.
        /// 2. **Counting**: A `HashMap` counts occurrences of each bucket key.
        /// 3. **Finding Maximum**: The bucket with the highest count is selected.
        /// 4. **Percentage Calculation**: Uses integer arithmetic to calculate what
        ///    percentage of measurements fall into the most common bucket.
        ///
        /// ## Example
        ///
        /// Given durations: [100μs, 110μs, 120μs, 200μs, 210μs] and:
        /// - If `min_duration_filter` = 10μs, cluster range = 50μs (10 * 5)
        /// - If `min_duration_filter` = 20μs, cluster range = 100μs (20 * 5)
        ///
        /// With cluster range = 50μs:
        /// - 100μs, 110μs, 120μs → bucket key 2 (100/50, 110/50, 120/50) → count: 3
        /// - 200μs, 210μs → bucket key 4 (200/50, 210/50) → count: 2
        ///
        /// Result: bucket 2 has the highest count (3), representing 60% of measurements.
        /// The function returns the representative duration for bucket 2 (100μs) along
        /// with the percentage (60%) and the most frequent hint for that bucket.
        ///
        /// ## Returns
        ///
        /// Returns the representative duration for the most common bucket, the
        /// percentage of occurrences of that bucket, and the most frequent hint
        /// associated with that bucket.
        #[must_use]
        pub fn median(&self) -> Option<(Duration, Pc, TelemetryAtomHint)> {
            // The count can't be greater than N.
            type BucketCount = u16;
            debug_assert!(BucketCount::MAX as usize >= N);

            if self.ring_buffer.is_empty() {
                return None;
            }

            if **self.ring_buffer.len() == 1 {
                let atom = self.ring_buffer.iter().next().copied()?;
                let percent = Pc::try_and_convert(100)?;
                return Some((atom.as_duration(), percent, atom.hint));
            }

            if **self.ring_buffer.len() == 2 {
                let mut it = self.ring_buffer.iter();
                let first_atom = it.next().copied()?;
                let second_atom = it.next().copied()?;
                let median = (first_atom.as_duration() + second_atom.as_duration()) / 2;
                let pc = Pc::try_and_convert(50)?;
                return Some((median, pc, first_atom.hint));
            }

            // Count occurrences of each duration in buckets.
            // Use the pre-calculated cluster sensitivity range from options.
            let range_micros = self.cluster_sensitivity_range.as_micros();
            let count_map: HashMap<u128, BucketCount> =
                self.ring_buffer
                    .iter()
                    .fold(HashMap::new(), |mut map, atom| {
                        let key = atom.as_duration().as_micros() / range_micros;
                        *map.entry(key).or_default() += 1;
                        map
                    });

            // Find the bucket with the maximum count.
            let (max_key, max_count) =
                count_map.into_iter().max_by_key(|&(_, count)| count)?;

            // Determine the most frequent hint for the max_key.
            let most_frequent_hint_for_max_key = self
                .ring_buffer
                .iter()
                .filter(|atom| atom.as_duration().as_micros() / range_micros == max_key)
                .map(|atom| atom.hint)
                .fold(HashMap::new(), |mut map, hint| {
                    *map.entry(hint).or_insert(0) += 1;
                    map
                })
                .into_iter()
                .max_by_key(|&(_, count)| count)
                .map_or(TelemetryAtomHint::None, |(hint, _)| hint);

            // Calculate percentage using integer arithmetic (avoiding floating point).
            let ring_buffer_len = **self.ring_buffer.len();
            let percent = if ring_buffer_len > 0 {
                // Use checked multiplication to avoid overflow, similar to compress.rs.
                // Convert BucketCount to u16 for the calculation to avoid overflow.
                let max_count_u16 = max_count;
                match max_count_u16.checked_mul(100) {
                    Some(product) => product / ring_buffer_len,
                    None => {
                        // Overflow case: max_count is very large.
                        // Fallback calculation that avoids overflow.
                        max_count_u16 / (ring_buffer_len / 100).max(1)
                    }
                }
            } else {
                0
            };
            let percent = Pc::try_and_convert(percent)?;

            if max_key == 0 {
                None
            } else {
                // Calculate the representative duration for the most common bucket.
                let max_key_u64 = u64::try_from(max_key).ok()?;
                let range_micros_u64 = u64::try_from(range_micros).ok()?;
                let representative_duration =
                    Duration::from_micros(max_key_u64 * range_micros_u64);
                Some((
                    representative_duration,
                    percent,
                    most_frequent_hint_for_max_key,
                ))
            }
        }
    }
}

mod report_generator {
    use super::{Instant, RateLimitStatus, RingBuffer, Telemetry, TelemetryHudReport,
                TimeDuration};

    impl<const N: usize> Telemetry<N> {
        /// Generate a report of the response times.
        /// - This function is rate limited to run once every `n sec`, where `n` is the
        ///   time duration defined in [`Self::rate_limiter_generate_report`].
        /// - If called more frequently, it will return the cached result.
        /// - The `generate_report` function is actually responsible for generating the
        ///   report (and saving it).
        ///
        /// This returns a [`TelemetryHudReport`] containing the telemetry data.
        pub fn report(&mut self) -> TelemetryHudReport {
            match self
                .rate_limiter_generate_report
                .get_status_and_update_last_run(Instant::now())
            {
                RateLimitStatus::NotStarted | RateLimitStatus::Expired => {
                    self.generate_report();
                }
                RateLimitStatus::Active => { /* Do nothing & return cached report */ }
            }
            self.report
        }

        /// Actually generate the report. This can be an expensive function to execute in
        /// a tight loop.
        ///
        /// This report is a measure of the latency of seeing output on the screen, after
        /// providing user input.
        ///
        /// It is similar to web performance metrics like "Time to Interactive (TTI)" or
        /// or "First Input Delay (FID)".
        fn generate_report(&mut self) {
            // No data available to generate a report.
            if self.ring_buffer.is_empty() {
                return;
            }

            // Generate the new report.
            if let (Some(avg), Some(min), Some(max), Some(median)) =
                (self.average(), self.min(), self.max(), self.median())
            {
                let (med, pc, hint) = median;
                let med = TimeDuration::from(med);
                let fps = med.get_as_fps();
                self.report = TelemetryHudReport {
                    avg,
                    min,
                    max,
                    med,
                    pc,
                    hint,
                    fps,
                };
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct TelemetryHudReport {
    pub avg: TimeDuration,
    pub min: TimeDuration,
    pub max: TimeDuration,
    pub med: TimeDuration,
    pub pc: Pc,
    pub hint: TelemetryAtomHint,
    pub fps: u32,
}

impl Display for TelemetryHudReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            return write!(f, "No data");
        }

        let st_ch = self.pc.as_glyph();
        let sep = glyphs::RIGHT_ARROW_DASHED_GLYPH;
        write!(
            f,
            "Latency ⣼ Avg{sep} {avg}, Min{sep} {min}, Max{sep} {max}, Med{sep} {med} ({fps}fps {st_ch} {pc:?} {hint})",
            avg = self.avg,
            min = self.min,
            max = self.max,
            med = self.med,
            fps = self.fps,
            pc = self.pc,
            hint = self.hint
        )
    }
}

impl TelemetryHudReport {
    #[must_use]
    pub fn is_empty(&self) -> bool { self.avg.inner == Duration::default() }
}

#[cfg(test)]
mod tests_fixtures {
    use super::*;
    pub const TEST_RING_BUFFER_SIZE: usize = 5;

    pub mod create {
        use super::*;

        /// Create a default telemetry instance with rate limiting and filtering.
        pub fn create_default_telemetry() -> Telemetry<TEST_RING_BUFFER_SIZE> {
            Telemetry::default()
        }

        /// Create a telemetry instance with just rate limiting.
        pub fn create_rate_limit_telemetry(
            duration: Duration,
        ) -> (Telemetry<TEST_RING_BUFFER_SIZE>, Duration) {
            (Telemetry::new(duration), duration)
        }

        /// Create a telemetry instance with just filtering. No rate limiting.
        pub fn create_filter_telemetry(
            min_duration_filter: Duration,
        ) -> (Telemetry<TEST_RING_BUFFER_SIZE>, Duration) {
            (
                Telemetry::new((Duration::from_secs(0), min_duration_filter)),
                min_duration_filter,
            )
        }

        /// Disable rate limiting and filtering.
        pub fn create_no_filter_no_rate_limit_telemetry()
        -> Telemetry<TEST_RING_BUFFER_SIZE> {
            Telemetry::new((Duration::from_secs(0), Duration::from_micros(0)))
        }
    }
}

#[cfg(test)]
mod tests_display_format {
    use std::fmt::Write;

    use super::{telemetry_sizing::{TELEMETRY_REPORT_STRING_SIZE,
                                   TelemetryReportLineStorage},
                *};

    #[test]
    fn test_display_formatter() {
        let mut backing_store = TelemetryReportLineStorage::new();

        assert!(!backing_store.spilled());
        assert_eq!(backing_store.capacity(), TELEMETRY_REPORT_STRING_SIZE);

        let avg = TimeDuration::from(
            Duration::from_secs(3600)
                + Duration::from_secs(1)
                + Duration::from_millis(100)
                + Duration::from_micros(100),
        );

        let min = TimeDuration::from(
            Duration::from_secs(60)
                + Duration::from_secs(1)
                + Duration::from_millis(100)
                + Duration::from_micros(100),
        );

        let max = TimeDuration::from(
            Duration::from_secs(1)
                + Duration::from_millis(100)
                + Duration::from_micros(100),
        );

        let median = TimeDuration::from(
            Duration::from_secs(1)
                + Duration::from_millis(100)
                + Duration::from_micros(500),
        );

        let median_micros = median.subsec_micros() % 1_000;
        let median_fps = 1_000_000 / median_micros;

        write!(backing_store,
            "Response time ⣼ Avg: {avg}, Min: {min}, Max: {max}, Median: {median}, FPS ⵚ Median: {median_fps}",
        ).unwrap();

        assert_eq!(
            backing_store.as_str(),
            "Response time ⣼ Avg: 1h:0m:1s100ms, Min: 1m:1s:100ms, Max: 1s:100ms, Median: 1s:100ms, FPS ⵚ Median: 2000"
        );

        println!("backing_store.len(): {}", backing_store.len());
        println!("backing_store.capacity(): {}", backing_store.capacity());

        assert!(!backing_store.spilled());
        assert_eq!(backing_store.capacity(), 128);
    }
}

/// Note that in all these tests, we are generally using the [`Telemetry::default`]
/// instance, which has baked into it lots of default configuration values from
/// [`mod@telemetry_default_constants`] for filtering, rate limiting, etc.
#[cfg(test)]
mod tests_record {
    use std::thread::sleep;

    use mutator::TryRecordResult;
    use tests_fixtures::create::*;

    use super::*;

    #[test]
    fn test_record_auto_stop() {
        let mut response_times = create_default_telemetry();
        assert_eq!(response_times.ring_buffer.len(), 0.into());

        // This block causes the _auto_stop handle to drop, which will record the response
        // time.
        {
            let _auto_stop =
                response_times.record_start_auto_stop(TelemetryAtomHint::None);
            sleep(Duration::from_micros(100));
        }

        assert_eq!(response_times.ring_buffer.len(), 1.into());
        let vec = response_times.ring_buffer.iter().collect::<Vec<_>>();
        let first = **vec.first().unwrap();
        assert!(first.as_duration() >= Duration::from_micros(100));
    }

    #[test]
    fn test_session_duration() {
        let ts_1 = Instant::now();
        let mut response_times = create_default_telemetry();

        assert!(response_times.start_timestamp > ts_1);
        assert!(response_times.session_duration() <= ts_1.elapsed().into());

        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(100),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );
        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(200),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );

        let sleep_time = Duration::from_nanos(100);
        sleep(sleep_time);
        assert!(response_times.session_duration() > sleep_time.into());
    }

    #[test]
    fn test_telemetry_recording() {
        let mut response_times = create_default_telemetry();
        assert_eq!(response_times.ring_buffer.len(), 0.into());

        let durations = [
            Duration::from_micros(100),
            Duration::from_micros(200),
            Duration::from_micros(300),
            Duration::from_micros(400),
            Duration::from_micros(500),
            Duration::from_micros(600),
            Duration::from_micros(700),
            Duration::from_micros(800),
            Duration::from_micros(900),
            Duration::from_micros(1000),
        ];

        for &duration in &durations {
            assert_eq!(
                response_times
                    .try_record(TelemetryAtom::new(duration, TelemetryAtomHint::None)),
                TryRecordResult::Ok
            );
        }

        let expected_final_buffer = vec![
            Duration::from_micros(600),
            Duration::from_micros(700),
            Duration::from_micros(800),
            Duration::from_micros(900),
            Duration::from_micros(1000),
        ];
        assert_eq!(
            response_times
                .ring_buffer
                .iter()
                .copied()
                .map(|it| it.as_duration())
                .collect::<Vec<_>>(),
            expected_final_buffer
        );
    }

    #[test]
    fn test_report_size() {
        let mut response_times = create_default_telemetry();

        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(100),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );
        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(200),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );

        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(300),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );
        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(300),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );
        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(500),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );

        let it = response_times.report().to_string();
        assert_eq!(it.len(), 93);
    }

    #[test]
    fn test_report_generate_and_rate_limit() {
        let (mut response_times, rate_limit_time_threshold) = create_rate_limit_telemetry(
            // This delay should be long enough for the report generation to occur,
            // which might take a few hundred micros.
            Duration::from_micros(1000),
        );

        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(100),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );
        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(200),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );
        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(300),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );
        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(300),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );
        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(500),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );

        // No report is generated yet.
        assert_eq!(response_times.report, TelemetryHudReport::default());

        // Generate the report.
        let report = response_times.report().to_string();
        let expected_len = 93;
        let expected_output_str = "Latency ⣼ Avg⇢ 280μs, Min⇢ 100μs, Max⇢ 500μs, Med⇢ 300μs (3333fps ◑ 40% NONE)";
        assert_eq!(report.len(), expected_len);
        assert_eq!(report, expected_output_str);
        let og_report_copy = response_times.report;

        // Generate the report again, but due to the rate limiter in effect, the report
        // should be the same as the previous one (and out of date).
        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(300),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );
        let report = response_times.report().to_string();
        assert_eq!(report.len(), expected_len);
        assert_eq!(report, expected_output_str);

        // Wait for the rate limiter to expire. The report should be different now.
        sleep(rate_limit_time_threshold);
        let report = response_times.report();
        let expected_output_str_new = "Latency ⣼ Avg⇢ 320μs, Min⇢ 200μs, Max⇢ 500μs, Med⇢ 300μs (3333fps ◕ 60% NONE)";
        assert_ne!(report, og_report_copy);
        assert_eq!(expected_output_str_new, report.to_string());
        assert_ne!(expected_output_str_new, og_report_copy.to_string());
    }

    #[test]
    fn test_response_times_filter() {
        let (mut response_times, min_filter_duration) =
            create_filter_telemetry(Duration::from_micros(100));

        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                min_filter_duration / 3,
                TelemetryAtomHint::None
            )),
            TryRecordResult::FilteredOut
        );

        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                min_filter_duration / 2,
                TelemetryAtomHint::None
            )),
            TryRecordResult::FilteredOut
        );

        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                min_filter_duration,
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );

        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                min_filter_duration * 2,
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );
    }
}

#[cfg(test)]
mod tests_math {
    use mutator::TryRecordResult;
    use tests_fixtures::create::*;

    use super::*;

    #[test]
    fn test_overview() {
        let mut response_times = create_default_telemetry();

        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(100),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );
        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(200),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );
        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(300),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );
        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(300),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );
        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(500),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );

        assert_eq!(
            response_times.average(),
            Some(Duration::from_micros(280).into())
        );
        assert_eq!(
            response_times.min(),
            Some(Duration::from_micros(100).into())
        );
        assert_eq!(
            response_times.max(),
            Some(Duration::from_micros(500).into())
        );
        assert_eq!(
            response_times.median(),
            Some((
                Duration::from_micros(300),
                Pc::try_and_convert(40).unwrap(),
                TelemetryAtomHint::None
            ))
        );

        let avg = response_times.average().unwrap();
        let min = response_times.min().unwrap();
        let max = response_times.max().unwrap();
        let (med, pc, hint) = response_times.median().unwrap();

        assert_eq!(avg, TimeDuration::from(Duration::from_micros(280)));
        assert_eq!(min, TimeDuration::from(Duration::from_micros(100)));
        assert_eq!(max, TimeDuration::from(Duration::from_micros(500)));
        assert_eq!(hint, TelemetryAtomHint::None);
        assert_eq!(*pc, 40);
        assert_eq!(med, Duration::from_micros(300));
    }
}

#[cfg(test)]
mod tests_median {
    use mutator::TryRecordResult;
    use tests_fixtures::{create::*, *};

    use super::*;

    #[test]
    fn test_overview_no_hint() {
        let mut response_times = create_no_filter_no_rate_limit_telemetry();

        assert_eq!(response_times.median(), None);

        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(100),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );

        assert_eq!(response_times.ring_buffer.len(), 1.into());
        assert_eq!(
            response_times.median(),
            Some((
                Duration::from_micros(100),
                Pc::try_and_convert(100).unwrap(),
                TelemetryAtomHint::None
            ))
        );

        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(200),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );
        assert_eq!(response_times.ring_buffer.len(), 2.into());

        let vec = response_times
            .ring_buffer
            .iter()
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(
            vec,
            [Duration::from_micros(100), Duration::from_micros(200)]
                .map(|it| TelemetryAtom::new(it, TelemetryAtomHint::None))
        );

        assert_eq!(
            response_times.median(),
            Some((
                Duration::from_micros(150),
                Pc::try_and_convert(50).unwrap(),
                TelemetryAtomHint::None
            ))
        );

        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(300),
                TelemetryAtomHint::None
            ),),
            TryRecordResult::Ok
        );
        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(300),
                TelemetryAtomHint::None
            ),),
            TryRecordResult::Ok
        );
        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(500),
                TelemetryAtomHint::None
            ),),
            TryRecordResult::Ok
        );

        assert_eq!(response_times.ring_buffer.len(), 5.into());
        assert_eq!(
            response_times.median(),
            Some((
                Duration::from_micros(300),
                Pc::try_and_convert(40).unwrap(),
                TelemetryAtomHint::None
            ))
        );
    }

    #[test]
    fn test_single_element() {
        let mut response_times = create_no_filter_no_rate_limit_telemetry();

        assert_eq!(response_times.median(), None);

        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(100),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );

        assert_eq!(response_times.ring_buffer.len(), 1.into());
        assert_eq!(
            response_times.median(),
            Some((
                Duration::from_micros(100),
                Pc::try_and_convert(100).unwrap(),
                TelemetryAtomHint::None
            ))
        );
    }

    #[test]
    fn test_two_elements() {
        let mut response_times = create_no_filter_no_rate_limit_telemetry();

        assert_eq!(response_times.median(), None);

        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(100),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );

        assert_eq!(
            response_times.try_record(TelemetryAtom::new(
                Duration::from_micros(200),
                TelemetryAtomHint::None
            )),
            TryRecordResult::Ok
        );

        assert_eq!(response_times.ring_buffer.len(), 2.into());
        assert_eq!(
            response_times.median(),
            Some((
                Duration::from_micros(150),
                Pc::try_and_convert(50).unwrap(),
                TelemetryAtomHint::None
            ))
        );
    }

    #[test]
    fn test_with_clusters() {
        let mut response_times = create_no_filter_no_rate_limit_telemetry();

        let durations = [
            Duration::from_micros(100),
            Duration::from_micros(110),
            Duration::from_micros(120),
            Duration::from_micros(200),
            Duration::from_micros(210),
        ];

        assert_eq!(durations.len(), TEST_RING_BUFFER_SIZE);

        for &duration in &durations {
            assert_eq!(
                response_times
                    .try_record(TelemetryAtom::new(duration, TelemetryAtomHint::None)),
                TryRecordResult::Ok
            );
        }

        assert_eq!(
            response_times.ring_buffer.len(),
            TEST_RING_BUFFER_SIZE.into()
        );

        assert_eq!(
            response_times.median(),
            Some((
                Duration::from_micros(100),
                Pc::try_and_convert(60).unwrap(),
                TelemetryAtomHint::None
            ))
        );
    }

    #[test]
    fn test_with_different_hints() {
        let mut response_times = create_no_filter_no_rate_limit_telemetry();

        let atoms = [
            TelemetryAtom::new(Duration::from_micros(100), TelemetryAtomHint::Render),
            TelemetryAtom::new(Duration::from_micros(110), TelemetryAtomHint::Render),
            TelemetryAtom::new(Duration::from_micros(120), TelemetryAtomHint::Render),
            TelemetryAtom::new(Duration::from_micros(200), TelemetryAtomHint::Signal),
            TelemetryAtom::new(Duration::from_micros(210), TelemetryAtomHint::Resize),
        ];

        assert_eq!(atoms.len(), TEST_RING_BUFFER_SIZE);

        for &atom in &atoms {
            assert_eq!(response_times.try_record(atom), TryRecordResult::Ok);
        }

        assert_eq!(
            response_times.ring_buffer.len(),
            TEST_RING_BUFFER_SIZE.into()
        );
        assert_eq!(
            response_times.median(),
            Some((
                Duration::from_micros(100),
                Pc::try_and_convert(60).unwrap(),
                TelemetryAtomHint::Render
            ))
        );
    }
}
#[test]
fn test_cluster_sensitivity_range_calculation() {
    use telemetry_constructor::ResponseTimesRingBufferOptions;

    // Test default case - should be 5x the default FILTER_MIN_RESPONSE_TIME (20μs * 5 =
    // 100μs) This demonstrates that our fix correctly calculates based on the actual
    // filter value instead of using a hardcoded value.
    let opts_default: ResponseTimesRingBufferOptions = ().into();
    assert_eq!(
        opts_default.cluster_sensitivity_range,
        Duration::from_micros(100) // 20μs (FILTER_MIN_RESPONSE_TIME) * 5
    );

    // Test custom filter - should be 5x the filter value.
    let custom_filter = Duration::from_micros(100);
    let opts_custom: ResponseTimesRingBufferOptions =
        (Duration::from_secs(1), custom_filter).into();
    assert_eq!(
        opts_custom.cluster_sensitivity_range,
        Duration::from_micros(500) // 100 * 5
    );

    // Test zero filter - should fall back to default (5x FILTER_MIN_RESPONSE_TIME).
    let opts_zero: ResponseTimesRingBufferOptions =
        (Duration::from_secs(1), Duration::from_micros(0)).into();
    assert_eq!(
        opts_zero.cluster_sensitivity_range,
        Duration::from_micros(100) /* Falls back to 5x FILTER_MIN_RESPONSE_TIME (20μs
                                    * * 5) */
    );
}
