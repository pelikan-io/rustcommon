//! This crate provides a simple implementation of a ratelimiter that can be
//! shared between threads.
//!
//! ```
//! use ratelimit::Ratelimiter;
//! use std::time::Duration;
//!
//! // Constructs a ratelimiter that generates 1 tokens/s with no burst. This
//! // can be used to produce a steady rate of requests. The ratelimiter starts
//! // with no tokens available, which means across application restarts, we
//! // cannot exceed the configured ratelimit.
//! let ratelimiter = Ratelimiter::builder(1, Duration::from_secs(1))
//!     .build()
//!     .unwrap();
//!
//! // Another use case might be admission control, where we start with some
//! // initial budget and replenish it periodically. In this example, our
//! // ratelimiter allows 1000 tokens/hour. For every hour long sliding window,
//! // no more than 1000 tokens can be acquired. But all tokens can be used in
//! // a single burst. Additional calls to `try_wait()` will return an error
//! // until the next token addition.
//! //
//! // This is popular approach with public API ratelimits.
//! let ratelimiter = Ratelimiter::builder(1000, Duration::from_secs(3600))
//!     .max_tokens(1000)
//!     .initial_available(1000)
//!     .build()
//!     .unwrap();
//!
//! // For very high rates, we should avoid using too short of an interval due
//! // to limits of system clock resolution. Instead, it's better to allow some
//! // burst and add multiple tokens per interval. The resulting ratelimiter
//! // here generates 50 million tokens/s and allows no more than 50 tokens to
//! // be acquired in any 1 microsecond long window.
//! let ratelimiter = Ratelimiter::builder(50, Duration::from_micros(1))
//!     .max_tokens(50)
//!     .build()
//!     .unwrap();
//!
//! // constructs a ratelimiter that generates 100 tokens/s with no burst
//! let ratelimiter = Ratelimiter::builder(1, Duration::from_millis(10))
//!     .build()
//!     .unwrap();
//!
//! for _ in 0..10 {
//!     // a simple sleep-wait
//!     if let Err(sleep) = ratelimiter.try_wait() {
//!            std::thread::sleep(sleep);
//!            continue;
//!     }
//!     
//!     // do some ratelimited action here    
//! }
//! ```

use clocksource::Nanoseconds;
use core::sync::atomic::{AtomicU64, Ordering};
use parking_lot::RwLock;
use thiserror::Error;

type Duration = clocksource::Duration<Nanoseconds<u64>>;
type Instant = clocksource::Instant<Nanoseconds<u64>>;
type AtomicInstant = clocksource::Instant<Nanoseconds<AtomicU64>>;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("available tokens cannot be set higher than max tokens")]
    AvailableTokensTooHigh,
    #[error("max tokens cannot be less than the refill amount")]
    MaxTokensTooLow,
    #[error("refill amount cannot exceed the max tokens")]
    RefillAmountTooHigh,
    #[error("refill interval in nanoseconds exceeds maximum u64")]
    RefillIntervalTooLong,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct Parameters {
    capacity: u64,
    refill_amount: u64,
    refill_interval: Duration,
}

pub struct Ratelimiter {
    available: AtomicU64,
    parameters: RwLock<Parameters>,
    refill_at: AtomicInstant,
}

impl Ratelimiter {
    /// Initialize a builder that will construct a `Ratelimiter` that adds the
    /// specified `amount` of tokens to the token bucket after each `interval`
    /// has elapsed.
    ///
    /// Note: In practice, the system clock resolution imposes a lower bound on
    /// the `interval`. To be safe, it is recommended to set the interval to be
    /// no less than 1 microsecond. This also means that the number of tokens
    /// per interval should be > 1 to achieve rates beyond 1 million tokens/s.
    pub fn builder(amount: u64, interval: core::time::Duration) -> Builder {
        Builder::new(amount, interval)
    }

    /// Return the current effective rate of the Ratelimiter in tokens/second
    pub fn rate(&self) -> f64 {
        let parameters = self.parameters.read();

        parameters.refill_amount as f64 * 1_000_000_000.0
            / parameters.refill_interval.as_nanos() as f64
    }

    /// Return the current interval between refills.
    pub fn refill_interval(&self) -> Duration {
        let parameters = self.parameters.read();

        Duration::from_nanos(parameters.refill_interval.as_nanos())
    }

    /// Allows for changing the interval between refills at runtime.
    pub fn set_refill_interval(&self, duration: core::time::Duration) -> Result<(), Error> {
        if duration.as_nanos() > u64::MAX as u128 {
            return Err(Error::RefillIntervalTooLong);
        }

        let mut parameters = self.parameters.write();

        parameters.refill_interval = Duration::from_nanos(duration.as_nanos() as u64);
        Ok(())
    }

    /// Return the current number of tokens to be added on each refill.
    pub fn refill_amount(&self) -> u64 {
        let parameters = self.parameters.read();

        parameters.refill_amount
    }

    /// Allows for changing the number of tokens to be added on each refill.
    pub fn set_refill_amount(&self, amount: u64) -> Result<(), Error> {
        let mut parameters = self.parameters.write();

        if amount > parameters.capacity {
            Err(Error::RefillAmountTooHigh)
        } else {
            parameters.refill_amount = amount;
            Ok(())
        }
    }

    /// Returns the maximum number of tokens that can
    pub fn max_tokens(&self) -> u64 {
        let parameters = self.parameters.read();

        parameters.capacity
    }

    /// Allows for changing the maximum number of tokens that can be held by the
    /// ratelimiter for immediate use. This effectively sets the burst size. The
    /// configured value must be greater than or equal to the refill amount.
    pub fn set_max_tokens(&self, amount: u64) -> Result<(), Error> {
        let mut parameters = self.parameters.write();

        if amount < parameters.refill_amount {
            Err(Error::MaxTokensTooLow)
        } else {
            parameters.capacity = amount;
            loop {
                let available = self.available();
                if amount > available {
                    if self
                        .available
                        .compare_exchange(available, amount, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok()
                    {
                        break;
                    }
                } else {
                    break;
                }
            }
            Ok(())
        }
    }

    pub fn available(&self) -> u64 {
        self.available.load(Ordering::Relaxed)
    }

    pub fn set_available(&self, amount: u64) -> Result<(), Error> {
        let parameters = self.parameters.read();
        if amount > parameters.capacity {
            Err(Error::AvailableTokensTooHigh)
        } else {
            self.available.store(amount, Ordering::Release);
            Ok(())
        }
    }

    /// Internal function to refill the token bucket. Called as part of
    /// `try_wait()`
    fn refill(&self, time: Instant) -> Result<(), core::time::Duration> {
        // will hold the number of elapsed refill intervals
        let mut intervals;
        // will hold a read lock for the refill parameters
        let mut parameters;

        loop {
            // determine when next refill should occur
            let refill_at = self.refill_at.load(Ordering::Relaxed);

            // if this time is before the next refill is due, return
            if time < refill_at {
                return Err(core::time::Duration::from_nanos(
                    (refill_at - time).as_nanos(),
                ));
            }

            // acquire read lock for refill parameters
            parameters = self.parameters.read();

            intervals = (time - refill_at).as_nanos() / parameters.refill_interval.as_nanos() + 1;

            // calculate when the following refill would be
            let next_refill = refill_at
                + clocksource::Duration::<Nanoseconds<u64>>::from_nanos(
                    intervals * parameters.refill_interval.as_nanos(),
                );

            // compare/exchange, if race, loop and check if we still need to
            // refill before trying again
            if self
                .refill_at
                .compare_exchange(refill_at, next_refill, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                break;
            }
        }

        // figure out how many tokens we might add
        let amount = intervals * parameters.refill_amount;

        let available = self.available.load(Ordering::Acquire);

        if available + amount >= parameters.capacity {
            self.available
                .fetch_add(parameters.capacity - available, Ordering::Release);
        } else {
            self.available.fetch_add(amount, Ordering::Release);
        }

        Ok(())
    }

    /// Non-blocking function to "wait" for a single token. On success, a single
    /// token has been acquired. On failure, a `Duration` hinting at when the
    /// next refill would occur is returned.
    pub fn try_wait(&self) -> Result<(), core::time::Duration> {
        loop {
            let refill_result = self.refill(Instant::now());

            loop {
                let available = self.available.load(Ordering::Acquire);
                if available == 0 {
                    refill_result?;
                    break;
                }

                let new = available - 1;

                if self
                    .available
                    .compare_exchange(available, new, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    return Ok(());
                }
            }
        }
    }
}

pub struct Builder {
    initial_available: u64,
    max_tokens: u64,
    refill_amount: u64,
    refill_interval: core::time::Duration,
}

impl Builder {
    /// Initialize a new builder that will add `amount` tokens after each
    /// `interval` has elapsed.
    fn new(amount: u64, interval: core::time::Duration) -> Self {
        Self {
            // default of zero tokens initially
            initial_available: 0,
            // default of one to prohibit bursts
            max_tokens: 1,
            refill_amount: amount,
            refill_interval: interval,
        }
    }

    /// Set the max tokens that can be held in the the `Ratelimiter` at any
    /// time. This limits the size of any bursts by placing an upper bound on
    /// the number of tokens available for immediate use.
    ///
    /// By default, the max_tokens will be set to one unless the refill amount
    /// requires a higher value.
    ///
    /// The selected value cannot be lower than the refill amount.
    pub fn max_tokens(mut self, tokens: u64) -> Self {
        self.max_tokens = tokens;
        self
    }

    /// Set the number of tokens that are initially available. For admission
    /// control scenarios, you may wish for there to be some tokens initially
    /// available to avoid delays or discards until the ratelimit is hit. When
    /// using it to enforce a ratelimit on your own process, for example when
    /// generating outbound requests, you may want there to be zero tokens
    /// availble initially to make your application more well-behaved in event
    /// of process restarts.
    ///
    /// The default is that no tokens are initially available.
    pub fn initial_available(mut self, tokens: u64) -> Self {
        self.initial_available = tokens;
        self
    }

    /// Consumes this `Builder` and attempts to construct a `Ratelimiter`.
    pub fn build(self) -> Result<Ratelimiter, Error> {
        if self.max_tokens < self.refill_amount {
            return Err(Error::MaxTokensTooLow);
        }

        if self.refill_interval.as_nanos() > u64::MAX as u128 {
            return Err(Error::RefillIntervalTooLong);
        }

        let available = AtomicU64::new(self.initial_available);

        let parameters = Parameters {
            capacity: self.max_tokens,
            refill_amount: self.refill_amount,
            refill_interval: Duration::from_nanos(self.refill_interval.as_nanos() as u64),
        };

        let refill_at = AtomicInstant::new(
            Instant::now()
                + clocksource::Duration::<Nanoseconds<u64>>::from_nanos(
                    self.refill_interval.as_nanos() as u64,
                ),
        );

        Ok(Ratelimiter {
            available,
            parameters: parameters.into(),
            refill_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use std::time::{Duration, Instant};

    macro_rules! approx_eq {
        ($value:expr, $target:expr) => {
            let value: f64 = $value;
            let target: f64 = $target;
            assert!(value >= target * 0.999, "{value} >= {}", target * 0.999);
            assert!(value <= target * 1.001, "{value} <= {}", target * 1.001);
        };
    }

    // test that the configured rate and calculated effective rate are close
    #[test]
    pub fn rate() {
        // amount + interval
        let rl = Ratelimiter::builder(4, Duration::from_nanos(333))
            .max_tokens(4)
            .build()
            .unwrap();

        approx_eq!(rl.rate(), 12012012.0);
    }

    // quick test that a ratelimiter yields tokens at the desired rate
    #[test]
    pub fn wait() {
        let rl = Ratelimiter::builder(1, Duration::from_micros(10))
            .build()
            .unwrap();

        let mut count = 0;

        let now = Instant::now();
        let end = now + Duration::from_millis(10);
        while Instant::now() < end {
            if rl.try_wait().is_ok() {
                count += 1;
            }
        }

        assert!(count >= 900);
        assert!(count <= 1100);
    }

    // quick test that an idle ratelimiter doesn't build up excess capacity
    #[test]
    pub fn idle() {
        let rl = Ratelimiter::builder(1, Duration::from_millis(1))
            .initial_available(1)
            .build()
            .unwrap();

        std::thread::sleep(Duration::from_millis(10));
        assert!(rl.try_wait().is_ok());
        assert!(rl.try_wait().is_err());
    }

    // quick test that capacity acts as expected
    #[test]
    pub fn capacity() {
        let rl = Ratelimiter::builder(1, Duration::from_millis(10))
            .max_tokens(10)
            .initial_available(0)
            .build()
            .unwrap();

        std::thread::sleep(Duration::from_millis(100));
        assert!(rl.try_wait().is_ok());
        assert!(rl.try_wait().is_ok());
        assert!(rl.try_wait().is_ok());
        assert!(rl.try_wait().is_ok());
        assert!(rl.try_wait().is_ok());
        assert!(rl.try_wait().is_ok());
        assert!(rl.try_wait().is_ok());
        assert!(rl.try_wait().is_ok());
        assert!(rl.try_wait().is_ok());
        assert!(rl.try_wait().is_ok());
        assert!(rl.try_wait().is_err());
    }
}
