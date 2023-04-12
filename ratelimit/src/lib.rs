//! This crate provides a simple implementation of a ratelimiter that can be
//! shared between threads.
//!
//! ```no_run
//! use ratelimit::Ratelimiter;
//! use std::time::Duration;
//!
//! let ratelimiter = Ratelimiter::builder()
//!     .refill_rate(100, Duration::from_secs(1))
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
    pub fn builder() -> Builder {
        Builder::new()
    }

    /// Return the current effective rate of the Ratelimiter in tokens/second
    pub fn rate(&self) -> f64 {
        self.refill_amount.load(Ordering::Relaxed) as f64 * 1_000_000_000.0
            / self.refill_interval.load(Ordering::Relaxed).as_nanos() as f64
    }

    /// Return the current interval between refills
    pub fn refill_interval(&self) -> Duration {
        Duration::from_nanos(self.refill_interval.load(Ordering::Relaxed).as_nanos())
    }

    /// Return the current number of tokens to be added on each refill
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

        println!("capacity: {capacity}");

        if available + amount >= capacity {
            println!("saturating");
            self.available
                .fetch_add(capacity - available, Ordering::Release);
        } else {
            println!("adding");
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

            let new = available.saturating_sub(1);

            if self
                .available
                .compare_exchange(available, new, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                // println!("anywhere");
                return Ok(());
            }
        }
    }
}

pub struct Builder {
    available: u64,
    capacity: u64,
    refill_amount: Option<u64>,
    refill_interval: Option<Duration>,
    refill_rate: (u64, Duration),
}

impl Builder {
    fn new() -> Self {
        Self {
            available: 1,
            capacity: 0,
            refill_amount: None,
            refill_interval: None,
            refill_rate: (1, Duration::from_secs(1)),
        }
    }

    /// Set the capacity of the `Ratelimiter`. This limits the size of any
    /// bursts by limiting the number of tokens the ratelimiter can have
    /// available at any time.
    ///
    /// Capacity will be increased automatically under any of these conditions:
    /// * `capacity` was set to zero
    /// * `refill_amount` was set higher than the capacity
    /// * `refill_rate` is high enough to require adding more tokens per refill
    ///    event than the capacity would otherwise allow
    pub fn capacity(mut self, capacity: u64) -> Self {
        self.capacity = std::cmp::max(1, capacity);
        self
    }

    /// Set the number of tokens that are initially available. For admission
    /// control scenarios, you may wish for there to be some tokens initially
    /// available to avoid delays or discards until the ratelimit is hit. When
    /// using it to enforce a ratelimit on your own process, for example when
    /// generating outbound requests, you may want there to be no tokens
    /// availble initially to make your application more well-behaved in event
    /// of process restarts.
    ///
    /// The default is that no tokens are initially available.
    pub fn available(mut self, tokens: u64) -> Self {
        self.available = tokens;
        self
    }

    /// Specify the rate at which tokens refill the bucket. Unless the refill
    /// burst has been set, the default behavior is to spread these tokens
    /// evenly across time when possible.
    ///
    /// You can use this in combination with either `refill_amount` or
    /// `refill_interval` to modify this behavior.
    ///
    /// The actual rate may be slightly lower under some conditions.
    pub fn refill_rate(mut self, tokens: u64, interval: Duration) -> Self {
        self.refill_rate = (
            std::cmp::max(1, tokens),
            std::cmp::max(interval, Duration::from_nanos(1)),
        );
        self
    }

    /// Specify the exact number of tokens to add on each refill. If the refill
    /// interval is not specified, it will be calculated from the refill rate
    /// and the amount of tokens specified here. Otherwise, the provided number
    /// of tokens will be added at each refill interval.
    pub fn refill_amount(mut self, tokens: u64) -> Self {
        self.refill_amount = Some(std::cmp::max(1, tokens));
        self
    }

    /// Specify the exact refill interval. If the refill amount is not
    /// specified, it will be calculated from the refill rate and this interval.
    /// Otherwise, at each refill interval, the refill amount of tokens are
    /// added.
    pub fn refill_interval(mut self, interval: Duration) -> Self {
        self.refill_interval = Some(std::cmp::max(interval, Duration::from_nanos(1)));
        self
    }

    /// Consumes this `Builder` and produces a `Ratelimiter`.
    pub fn build(self) -> Ratelimiter {
        let (amount, interval) = match (self.refill_amount, self.refill_interval) {
            (Some(amount), Some(interval)) => {
                // if we have an explicit amount and interval, use those
                // directly
                (amount, interval.as_nanos() as u64)
            }
            (Some(amount), None) => {
                let a = self.refill_rate.0;
                let i = self.refill_rate.1.as_nanos() as u64;

                // tokens per nanosecond
                let target_rate = a as f64 / i as f64;

                let interval = (amount as f64 / target_rate).round() as u64;

                (amount, interval)
            }
            (None, Some(interval)) => {
                let a = self.refill_rate.0;
                let i = self.refill_rate.1.as_nanos() as u64;

                // tokens per nanosecond
                let target_rate = a as f64 / i as f64;

                let amount = (interval.as_nanos() as u64 as f64 * target_rate).round() as u64;
                let interval = interval.as_nanos() as u64;

                (amount, interval)
            }
            (None, None) => {
                // work out an amount and interval based on the rate

                let mut amount = self.refill_rate.0;
                let mut interval = self.refill_rate.1.as_nanos() as u64;

                // copy the original values for use later
                let a = amount;
                let i = interval;

                // reduce things to the smallest amount and interval possible
                let gcd = gcd(amount, interval);
                amount /= gcd;
                interval /= gcd;

                // scale things up to reduce bookeeping
                while interval < 100 {
                    amount += a;
                    interval += i;
                }

                (amount, interval)
            }
        };

        let available = AtomicU64::new(self.available);
        let capacity = AtomicU64::new(std::cmp::max(self.capacity, amount));

        let refill_amount = AtomicU64::new(amount);
        let refill_at = AtomicInstant::new(
            Instant::now() + clocksource::Duration::<Nanoseconds<u64>>::from_nanos(interval),
        );
        let refill_interval = AtomicDuration::from_nanos(interval);

        Ratelimiter {
            available,
            capacity,
            refill_amount,
            refill_at,
            refill_interval,
        }
    }
}

// Taken from Wikipedia: https://en.wikipedia.org/wiki/Binary_GCD_algorithm
fn gcd(mut u: u64, mut v: u64) -> u64 {
    use std::cmp::min;
    use std::mem::swap;

    // Base cases: gcd(n, 0) = gcd(0, n) = n
    if u == 0 {
        return v;
    } else if v == 0 {
        return u;
    }

    // Using identities 2 and 3:
    // gcd(2ⁱ u, 2ʲ v) = 2ᵏ gcd(u, v) with u, v odd and k = min(i, j)
    // 2ᵏ is the greatest power of two that divides both u and v
    let i = u.trailing_zeros();
    u >>= i;
    let j = v.trailing_zeros();
    v >>= j;
    let k = min(i, j);

    loop {
        // u and v are odd at the start of the loop
        debug_assert!(u % 2 == 1, "u = {} is even", u);
        debug_assert!(v % 2 == 1, "v = {} is even", v);

        // Swap if necessary so u <= v
        if u > v {
            swap(&mut u, &mut v);
        }
        // u and v are still both odd after (potentially) swapping

        // Using identity 4 (gcd(u, v) = gcd(|v-u|, min(u, v))
        v -= u;
        // v is now even, but u is unchanged (and odd)

        // Identity 1: gcd(u, 0) = u
        // The shift by k is necessary to add back the 2ᵏ factor that was removed before the loop
        if v == 0 {
            return u << k;
        }

        // Identity 3: gcd(u, 2ʲ v) = gcd(u, v) (u is known to be odd)
        v >>= v.trailing_zeros();
        // v is now odd again
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
        // rate
        let rl = Ratelimiter::builder()
            .refill_rate(100000, Duration::from_secs(1))
            .build();

        approx_eq!(rl.rate(), 100000.0);

        let rl = Ratelimiter::builder()
            .refill_rate(3, Duration::from_secs(1))
            .build();

        approx_eq!(rl.rate(), 3.0);

        let rl = Ratelimiter::builder()
            .refill_rate(333333333, Duration::from_secs(1))
            .build();

        approx_eq!(rl.rate(), 333333333.0);

        // amount + interval
        let rl = Ratelimiter::builder()
            .refill_amount(4)
            .refill_interval(Duration::from_nanos(333))
            .build();

        approx_eq!(rl.rate(), 12012012.0);

        // interval + rate
        let rl = Ratelimiter::builder()
            .refill_rate(333333333, Duration::from_secs(1))
            .refill_interval(Duration::from_nanos(333))
            .build();

        approx_eq!(rl.rate(), 333333333.0);

        // amount + rate
        let rl = Ratelimiter::builder()
            .refill_rate(333333333, Duration::from_secs(1))
            .refill_amount(4)
            .build();

        approx_eq!(rl.rate(), 333333333.0);
    }

    // quick test that a ratelimiter yields tokens at the desired rate
    #[test]
    pub fn wait() {
        let rl = Ratelimiter::builder()
            .refill_rate(100000, Duration::from_secs(1))
            .build();

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
        let rl = Ratelimiter::builder()
            .refill_rate(1000, Duration::from_secs(1))
            .available(1)
            .build();

        std::thread::sleep(Duration::from_millis(10));
        assert!(rl.try_wait().is_ok());
        assert!(rl.try_wait().is_err());
    }

    // quick test that capacity acts as expected
    #[test]
    pub fn capacity() {
        let rl = Ratelimiter::builder()
            .refill_rate(100, Duration::from_secs(1))
            .capacity(10)
            .available(0)
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
