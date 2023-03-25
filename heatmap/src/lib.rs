// Copyright 2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

mod error;
mod heatmap;

use clocksource::Nanoseconds;
use core::sync::atomic::AtomicU64;

pub use self::heatmap::Heatmap;
pub use error::Error;

pub type Instant = clocksource::Instant<Nanoseconds<u64>>;
pub type Duration = clocksource::Duration<Nanoseconds<u64>>;

type AtomicInstant = clocksource::Instant<Nanoseconds<AtomicU64>>;
