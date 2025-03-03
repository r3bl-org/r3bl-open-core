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
use std::ops::{AddAssign, Deref, DerefMut};

#[derive(Debug, Clone, Copy, PartialEq, size_of::SizeOf)]
pub struct Seed(pub f64);

mod seed {

    use super::*;

    impl Deref for Seed {
        type Target = f64;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for Seed {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl From<f64> for Seed {
        fn from(f: f64) -> Self { Self(f) }
    }

    impl AddAssign<SeedDelta> for Seed {
        fn add_assign(&mut self, delta: SeedDelta) { self.0 += delta.0; }
    }

    impl AddAssign<Seed> for Seed {
        fn add_assign(&mut self, other: Seed) { self.0 += other.0; }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, size_of::SizeOf)]
pub struct Spread(pub f64);

mod spread {
    use super::*;

    impl Deref for Spread {
        type Target = f64;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for Spread {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl From<f64> for Spread {
        fn from(f: f64) -> Self { Self(f) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, size_of::SizeOf)]
pub struct Frequency(pub f64);

mod frequency {
    use super::*;

    impl Deref for Frequency {
        type Target = f64;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for Frequency {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl From<f64> for Frequency {
        fn from(f: f64) -> Self { Self(f) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, size_of::SizeOf)]
pub struct SeedDelta(pub f64);

mod seed_delta {
    use super::*;

    impl Deref for SeedDelta {
        type Target = f64;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for SeedDelta {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl From<f64> for SeedDelta {
        fn from(f: f64) -> Self { Self(f) }
    }
}
