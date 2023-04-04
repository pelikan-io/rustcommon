// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

//! This library provides a thread safe token bucket ratelimitier

#![deny(clippy::all)]

use clocksource::*;
use core::convert::TryFrom;
use core::sync::atomic::*;

use rand_distr::{Distribution, Normal, Uniform};
use thiserror::Error;

/// A token bucket ratelimiter
pub struct Ratelimiter {
    available: AtomicU64,
    capacity: AtomicU64,
    quantum: AtomicU64,
    strategy: AtomicUsize,
    interval: Duration<Nanoseconds<AtomicU64>>,
    tick_at: Instant<Nanoseconds<AtomicU64>>,
    normal: Normal<f64>,
    uniform: Uniform<f64>,
}

/// Possible errors returned by operations on a histogram.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("rate must be > 0")]
    /// The histogram contains no samples.
    InvalidRate,
}

/// Refill strategies define how the token bucket is refilled. The different
/// strategies can be used to alter the character of the smoothness of the
/// interval between tokens becoming available
#[derive(Copy, Clone, Debug)]
pub enum Refill {
    /// Use a fixed tick interval resulting in a smooth ratelimit
    Smooth = 0,
    /// Use a uniform distribution for tick interval resulting in a ratelimit
    /// that varies from 2/3 to 3/2 times the specified rate. Resulting in an
    /// average ratelimit that matches the configured rate.
    Uniform = 1,
    /// Use a normal distribution for the tick interval centered on the duration
    /// matching that of the smooth refill strategy. The distribution used has
    /// a standard deviation of 2x the mean and results in an average ratelimit
    /// that matches the configured rate.
    Normal = 2,
}

impl TryFrom<usize> for Refill {
    type Error = ();

    fn try_from(v: usize) -> Result<Self, Self::Error> {
        match v {
            x if x == Refill::Smooth as usize => Ok(Refill::Smooth),
            x if x == Refill::Uniform as usize => Ok(Refill::Uniform),
            x if x == Refill::Normal as usize => Ok(Refill::Normal),
            _ => Err(()),
        }
    }
}

const SECOND: u64 = 1_000_000_000;

/// A token bucket ratelimiter
impl Ratelimiter {
    /// Create a new token bucket `Ratelimiter` which can hold up to `capacity`
    /// tokens. `quantum` tokens will be added to the bucket at `rate` times
    /// per second. The token bucket initially starts without any tokens, this
    /// ensures the rate does not start high initially.
    ///
    /// # Examples
    ///
    /// ```
    /// use ratelimit::*;
    ///
    /// // ratelimit to 1/s with no burst capacity
    /// let ratelimiter = Ratelimiter::new(1, 1, 1).unwrap();
    ///
    /// // ratelimit to 100/s with bursts up to 10
    /// let ratelimiter = Ratelimiter::new(10, 1, 100).unwrap();
    /// ```
    pub fn new(capacity: u64, quantum: u64, rate: u64) -> Result<Self, Error> {
        if rate == 0 {
            return Err(Error::InvalidRate);
        }

        let interval = SECOND / rate;

        let tick_at =
            Instant::<Nanoseconds<u64>>::now() + Duration::<Nanoseconds<u64>>::from_nanos(interval);

        Ok(Self {
            available: AtomicU64::default(),
            capacity: AtomicU64::new(capacity),
            quantum: AtomicU64::new(quantum),
            strategy: AtomicUsize::new(Refill::Smooth as usize),
            interval: Duration::<Nanoseconds<AtomicU64>>::from_nanos(interval),
            tick_at: Instant::<Nanoseconds<AtomicU64>>::new(tick_at),
            normal: Normal::new(interval as f64, 2.0 * interval as f64).unwrap(),
            uniform: Uniform::new_inclusive(interval as f64 * 0.5, interval as f64 * 1.5),
        })
    }

    /// Changes the rate of the `Ratelimiter`. The new rate will be in effect on
    /// the next tick.
    pub fn set_rate(&self, rate: u64) -> Result<(), Error> {
        if rate == 0 {
            return Err(Error::InvalidRate);
        }

        self.interval.store(
            Duration::<Nanoseconds<u64>>::from_nanos(SECOND / rate),
            Ordering::Relaxed,
        );

        Ok(())
    }

    /// Returns the current rate
    pub fn rate(&self) -> u64 {
        SECOND * self.quantum.load(Ordering::Relaxed)
            / self.interval.load(Ordering::Relaxed).as_nanos()
    }

    /// Changes the refill strategy
    pub fn set_strategy(&self, strategy: Refill) {
        self.strategy.store(strategy as usize, Ordering::Relaxed)
    }

    // internal function to move the time forward
    fn tick(&self) {
        // get the time when tick() was called
        let now = Instant::<Nanoseconds<u64>>::now();

        // we loop here so that if we lose the compare_exchange, we can try
        // again if tick_at has not advanced far enough
        loop {
            // load the current value
            let tick_at = self.tick_at.load(Ordering::Relaxed);

            // if `tick_at` is in the future, we return.
            if tick_at > now {
                return;
            }

            // Depending on our refill strategy, we have different ways to
            // calculate how many `ticks` have elapsed.
            let (next, ticks) = match Refill::try_from(self.strategy.load(Ordering::Relaxed)) {
                Ok(Refill::Smooth) | Err(_) => {
                    // This case is the most straightforward, since the interval
                    // between ticks is constant, we can directly calculate how
                    // many have elapsed.

                    let interval = self.interval.load(Ordering::Relaxed);

                    let ticks = 1 + (now - tick_at).as_nanos() / interval.as_nanos();
                    let next = now + interval;

                    (next, ticks)
                }
                Ok(Refill::Uniform) => {
                    // For this refill strategy, the tick interval is variable.
                    // Therefore, we must sample the distribution repeatedly to
                    // determine how many ticks would have elapsed.

                    let mut tick_to = tick_at;
                    let mut ticks = 0;

                    while tick_to <= tick_at {
                        tick_to += Duration::<Nanoseconds<u64>>::from_nanos(
                            self.uniform.sample(&mut rand::thread_rng()) as u64,
                        );
                        ticks += 1;
                    }

                    (tick_to, ticks)
                }
                Ok(Refill::Normal) => {
                    // For this refill strategy, the tick interval is variable.
                    // Therefore, we must sample the distribution repeatedly to
                    // determine how many ticks would have elapsed.

                    let mut next = tick_at;
                    let mut ticks = 0;

                    while next <= tick_at {
                        next += Duration::<Nanoseconds<u64>>::from_nanos(
                            self.normal.sample(&mut rand::thread_rng()) as u64,
                        );
                        ticks += 1;
                    }

                    (next, ticks)
                }
            };

            // Now we attempt to atomically update `tick_at`. If we lose the
            // race, we will loop and find we hit one of two cases:
            // - `tick_at` has advanced into the future and we early return
            // - `tick_at` has advanced, but is not in the future, so we need to
            //    recalculate how many ticks to advance by and attempt the
            //    compare/exchange again
            if self
                .tick_at
                .compare_exchange(tick_at, next, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                let tokens = self.quantum.load(Ordering::Relaxed) * ticks;
                let capacity = self.capacity.load(Ordering::Relaxed);
                let available = self.available.load(Ordering::Relaxed);
                if available + tokens >= capacity {
                    self.available.store(capacity, Ordering::Relaxed);
                } else {
                    self.available.fetch_add(tokens, Ordering::Relaxed);
                }
            }
        }
    }

    /// Non-blocking wait for one token, returns an `Ok` if a token was
    /// successfully acquired, returns an `Err` if it would block.
    ///
    /// # Examples
    ///
    /// ```
    /// use ratelimit::*;
    ///
    /// let ratelimiter = Ratelimiter::new(1, 1, 100).unwrap();
    /// for i in 0..100 {
    ///     // do some work here
    ///     while ratelimiter.try_wait().is_err() {
    ///         // do some other work
    ///     }
    /// }
    /// ```
    pub fn try_wait(&self) -> Result<(), std::io::Error> {
        self.tick();
        let mut current = self.available.load(Ordering::Relaxed);
        while current > 0 {
            match self.available.compare_exchange(
                current,
                current - 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Ok(()),
                Err(v) => {
                    current = v;
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::WouldBlock,
            "no tokens in bucket",
        ))
    }

    /// Blocking wait implemented as a busy loop. Returns only after a token is
    /// successfully acquired
    ///
    /// # Examples
    ///
    /// ```
    /// use ratelimit::*;
    ///
    /// let ratelimiter = Ratelimiter::new(1, 1, 100).unwrap();
    /// for i in 0..100 {
    ///     // do some work here
    ///     ratelimiter.wait();
    /// }
    /// ```
    pub fn wait(&self) {
        while self.try_wait().is_err() {}
    }
}

#[cfg(test)]
mod tests {
    use std::{thread::sleep, time::Duration};

    use crate::Ratelimiter;

    #[test]
    fn should_not_burst_beyond_capacity() {
        let rl = Ratelimiter::new(1, 1, 1000).unwrap();

        // Sleep 100ms
        sleep(Duration::from_millis(100));

        let mut tokens = 0;
        while rl.try_wait().is_ok() {
            tokens += 1;
        }

        assert_eq!(tokens, 1);
    }
}
