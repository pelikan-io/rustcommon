//! This crate provides a simple implementation of a ratelimiter that can be
//! shared between threads.
//!
//! ```no_run
//! use ratelimit::Ratelimiter;
//! use std::time::Duration;
//!
//! let ratelimiter = Ratelimiter::builder(1, Duration::from_millis(10))
//!     .build();
//!
//! loop {
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
use core::time::Duration;

type Instant = clocksource::Instant<Nanoseconds<u64>>;
type AtomicInstant = clocksource::Instant<Nanoseconds<AtomicU64>>;
type AtomicDuration = clocksource::Duration<Nanoseconds<AtomicU64>>;

pub struct Ratelimiter {
    available: AtomicU64,
    capacity: AtomicU64,
    refill_amount: AtomicU64,
    refill_at: AtomicInstant,
    refill_interval: AtomicDuration,
}

impl Ratelimiter {
    pub fn builder(amount: u64, interval: Duration) -> Builder {
        Builder::new(amount, interval)
    }

    /// Return the current effective rate of the Ratelimiter in tokens/second
    pub fn rate(&self) -> f64 {
        self.refill_amount.load(Ordering::Relaxed) as f64 * 1_000_000_000.0
            / self.refill_interval.load(Ordering::Relaxed).as_nanos() as f64
    }

    /// Return the current interval between refills.
    pub fn refill_interval(&self) -> Duration {
        Duration::from_nanos(self.refill_interval.load(Ordering::Relaxed).as_nanos())
    }

    /// Allows for changing the interval between refills at runtime.
    pub fn set_refill_interval(&self, duration: Duration) {
        self.refill_interval.store(
            clocksource::Duration::<Nanoseconds<u64>>::from_nanos(duration.as_nanos() as u64),
            Ordering::Relaxed,
        )
    }

    /// Return the current number of tokens to be added on each refill.
    pub fn refill_amount(&self) -> u64 {
        self.refill_amount.load(Ordering::Relaxed)
    }

    /// Internal function to refill the token bucket. Called as part of
    /// `try_wait()`
    fn refill(&self, time: Instant) -> Result<(), core::time::Duration> {
        let mut interval = self.refill_interval.load(Ordering::Relaxed);
        let mut refill_amount = self.refill_amount.load(Ordering::Relaxed);

        let mut intervals;

        loop {
            // determine when next refill should occur
            let refill_at = self.refill_at.load(Ordering::Relaxed);

            // if this time is before the next refill is due, return
            if time < refill_at {
                return Err(core::time::Duration::from_nanos(
                    (refill_at - time).as_nanos(),
                ));
            }

            intervals = (time - refill_at).as_nanos() / interval.as_nanos() + 1;

            // calculate when the following refill would be
            let next_refill = refill_at
                + clocksource::Duration::<Nanoseconds<u64>>::from_nanos(
                    intervals * interval.as_nanos(),
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

            // if we lost the compare/exchange, these may have changed in the
            // meantime (by updating the rate in another thread)
            interval = self.refill_interval.load(Ordering::Relaxed);
            refill_amount = self.refill_amount.load(Ordering::Relaxed);
        }

        // figure out how many tokens we might add
        let amount = intervals * refill_amount;

        let available = self.available.load(Ordering::Acquire);
        let capacity = self.capacity.load(Ordering::Relaxed);

        if available + amount >= capacity {
            self.available
                .fetch_add(capacity - available, Ordering::Release);
        } else {
            self.available.fetch_add(amount, Ordering::Release);
        }

        Ok(())
    }

    /// Non-blocking function to "wait" for a single token. On success, a single
    /// token has been acquired. On failure, a `Duration` hinting at when the
    /// next refill would occur is returned.
    pub fn try_wait(&self) -> Result<(), core::time::Duration> {
        let refill_result = self.refill(Instant::now());

        loop {
            let available = self.available.load(Ordering::Acquire);
            if available == 0 {
                refill_result?
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

pub struct Builder {
    initial_available: u64,
    max_tokens: u64,
    refill_amount: u64,
    refill_interval: Duration,
}

impl Builder {
    pub fn new(amount: u64, interval: Duration) -> Self {
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
    /// This value will be increased automatically under any of these
    /// conditions:
    /// * `max_tokens` was set to zero, in which case we will increase it to at
    ///   least one
    /// * `refill_amount` was set higher than the `max_token`, in which case the
    ///   refill amount will be used instead.
    ///
    /// By default, the max_tokens will be set to `1` unless the `refill_amount`
    /// requires a higher value.
    pub fn max_tokens(mut self, tokens: u64) -> Self {
        self.max_tokens = std::cmp::max(1, tokens);
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

    /// Consumes this `Builder` and produces a `Ratelimiter`.
    pub fn build(self) -> Ratelimiter {
        let available = AtomicU64::new(self.initial_available);
        let capacity = AtomicU64::new(std::cmp::max(self.max_tokens, self.refill_amount));

        let refill_amount = AtomicU64::new(self.refill_amount);
        let refill_at = AtomicInstant::new(
            Instant::now()
                + clocksource::Duration::<Nanoseconds<u64>>::from_nanos(
                    self.refill_interval.as_nanos() as u64,
                ),
        );
        let refill_interval = AtomicDuration::from_nanos(self.refill_interval.as_nanos() as u64);

        Ratelimiter {
            available,
            capacity,
            refill_amount,
            refill_at,
            refill_interval,
        }
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
        let rl = Ratelimiter::builder(4, Duration::from_nanos(333)).build();

        approx_eq!(rl.rate(), 12012012.0);
    }

    // quick test that a ratelimiter yields tokens at the desired rate
    #[test]
    pub fn wait() {
        let rl = Ratelimiter::builder(1, Duration::from_micros(10)).build();

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
            .build();

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
            .build();

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
