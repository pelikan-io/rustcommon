use crate::NANOS_PER_SEC;
use core::sync::atomic::{AtomicU64, Ordering};

pub mod monotonic {
    use super::*;

    use winapi::um::winnt::LARGE_INTEGER;

    static FREQUENCY: AtomicU64 = AtomicU64::new(0);

    fn frequency() -> u64 {
        let cached = FREQUENCY.load(Ordering::Relaxed);

        if cached != 0 {
            return cached;
        }

        let frequency;
        unsafe {
            let mut frq: LARGE_INTEGER = core::mem::zeroed();
            let _ = winapi::um::profileapi::QueryPerformanceFrequency(&mut frq);
            frequency = *frq.QuadPart() as u64;
        }

        FREQUENCY.store(frequency, Ordering::Relaxed);
        frequency
    }

    fn count() -> u64 {
        unsafe {
            let mut cnt: LARGE_INTEGER = core::mem::zeroed();
            let _ = winapi::um::profileapi::QueryPerformanceCounter(&mut cnt);
            *cnt.QuadPart() as u64
        }
    }

    pub fn coarse() -> crate::coarse::Instant {
        crate::coarse::Instant {
            secs: (count() / frequency()) as u32,
        }
    }

    pub fn precise() -> crate::precise::Instant {
        let count = count();
        let frequency = frequency();

        let secs = count / frequency;
        let ns = count % frequency;

        crate::precise::Instant {
            ns: secs * NANOS_PER_SEC + ns * NANOS_PER_SEC / frequency,
        }
    }
}

pub mod realtime {
    use super::*;

    use winapi::shared::minwindef::FILETIME;

    const UNIX_EPOCH_INTERVALS: u64 = 116_444_736 * NANOS_PER_SEC;
    const NANOS_PER_INTERVAL: u64 = 100;

    const INTERVALS_PER_SEC: u64 = NANOS_PER_SEC / NANOS_PER_INTERVAL;

    fn unix_intervals() -> u64 {
        let filetime;
        unsafe {
            let mut ft: FILETIME = core::mem::zeroed();
            let _ = winapi::um::sysinfoapi::GetSystemTimePreciseAsFileTime(&mut ft);
            filetime = (core::mem::transmute::<FILETIME, i64>(ft)) as u64;
        }

        filetime - UNIX_EPOCH_INTERVALS
    }

    pub fn coarse() -> crate::coarse::UnixInstant {
        crate::coarse::UnixInstant {
            secs: (unix_intervals() / INTERVALS_PER_SEC) as u32,
        }
    }

    pub fn precise() -> crate::precise::UnixInstant {
        crate::precise::UnixInstant {
            ns: unix_intervals() * NANOS_PER_INTERVAL,
        }
    }
}
