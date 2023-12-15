#[cfg(any(target_os = "macos", target_os = "ios"))]
const CLOCK_MONOTONIC_COARSE: u32 = libc::CLOCK_MONOTONIC;

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
const CLOCK_MONOTONIC_COARSE: i32 = libc::CLOCK_MONOTONIC_COARSE;

#[cfg(any(target_os = "macos", target_os = "ios"))]
const CLOCK_REALTIME_COARSE: u32 = libc::CLOCK_REALTIME;

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
const CLOCK_REALTIME_COARSE: i32 = libc::CLOCK_REALTIME_COARSE;

pub fn read_clock(clock: i32) -> libc::timespec {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };

    unsafe {
        libc::clock_gettime(clock as _, &mut ts);
    }

    ts
}

pub mod monotonic {
    use super::*;

    pub fn coarse() -> crate::coarse::Instant {
        let ts = read_clock(CLOCK_MONOTONIC_COARSE as _);

        let now = ts.tv_sec as u32;

        crate::coarse::Instant { secs: now }
    }

    pub fn precise() -> crate::precise::Instant {
        let ts = read_clock(libc::CLOCK_MONOTONIC as _);

        let now = (ts.tv_sec as u64)
            .wrapping_mul(1_000_000_000)
            .wrapping_add(ts.tv_nsec as u64);

        crate::precise::Instant { ns: now }
    }
}

pub mod realtime {
    use super::*;

    pub fn coarse() -> crate::coarse::UnixInstant {
        let ts = read_clock(CLOCK_REALTIME_COARSE as _);

        let now = ts.tv_sec as u32;

        crate::coarse::UnixInstant { secs: now }
    }

    pub fn precise() -> crate::precise::UnixInstant {
        let ts = read_clock(libc::CLOCK_REALTIME as _);

        let now = (ts.tv_sec as u64)
            .wrapping_mul(1_000_000_000)
            .wrapping_add(ts.tv_nsec as u64);

        crate::precise::UnixInstant { ns: now }
    }
}
