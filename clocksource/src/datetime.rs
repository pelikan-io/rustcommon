//! Human readable datetimes.

use core::fmt::Display;
use time::OffsetDateTime;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DateTime {
    dt: OffsetDateTime,
}

impl Display for DateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        let date = self.dt.date();
        let time = self.dt.time();
        write!(
            f,
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}+00:00",
            date.year(),
            date.month() as u8,
            date.day(),
            time.hour(),
            time.minute(),
            time.second(),
            time.millisecond(),
        )
    }
}

impl From<crate::precise::UnixInstant> for DateTime {
    fn from(other: crate::precise::UnixInstant) -> Self {
        DateTime {
            dt: OffsetDateTime::from_unix_timestamp_nanos(other.ns as i128).unwrap(),
        }
    }
}

impl From<crate::coarse::UnixInstant> for DateTime {
    fn from(other: crate::coarse::UnixInstant) -> Self {
        DateTime {
            dt: OffsetDateTime::from_unix_timestamp(other.secs as i64).unwrap(),
        }
    }
}
