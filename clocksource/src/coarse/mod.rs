//! Coarse time representations represent times and durations as a whole number
//! of seconds and use 32bit primitives as the internal representation.
//!
//! Unlike `std::time`, these types always have a fixed size representation and
//! also includes atomic types.

mod atomic_duration;
mod atomic_instant;
mod atomic_unix_instant;
mod duration;
mod instant;
mod unix_instant;

pub use atomic_duration::AtomicDuration;
pub use atomic_instant::AtomicInstant;
pub use atomic_unix_instant::AtomicUnixInstant;
pub use duration::Duration;
pub use instant::Instant;
pub use unix_instant::UnixInstant;
