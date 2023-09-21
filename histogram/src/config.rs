//! The configuration of a histogram which determines the buckets and how to
//! convert a value to a bucket index and vice versa.

use super::{BuildError, Error};
use crate::RangeInclusive;

#[derive(Clone, Copy)]
pub(crate) struct Config {
    max: u64,
    p: u8,
    n: u8,
    cutoff_power: u8,
    cutoff_value: u64,
    lower_bin_count: u32,
    upper_bin_divisions: u32,
    upper_bin_count: u32,
}

impl Config {
    pub(crate) fn new(p: u8, n: u8) -> Result<Self, BuildError> {
        // we only allow values up to 2^64
        if n > 64 {
            return Err(BuildError::MaxPowerTooHigh);
        }

        // check that the other parameters make sense together
        if p >= n {
            return Err(BuildError::MaxPowerTooLow);
        }

        // the cutoff is the point at which the linear range divisions and the
        // logarithmic range subdivisions diverge.
        //
        // for example:
        // when a = 0, the linear range has bins with width 1.
        // if b = 7 the logarithmic range has 128 subdivisions.
        // this means that for 0..128 we must be representing the values exactly
        // but we also represent 128..256 exactly since the subdivisions divide
        // that range into bins with the same width as the linear portion.
        //
        // therefore our cutoff power = a + b + 1

        // note: because a + b must be less than n which is a u8, a + b + 1 must
        // be less than or equal to u8::MAX. This means our cutoff power will
        // always fit in a u8
        let cutoff_power = p + 1;
        let cutoff_value = 2_u64.pow(cutoff_power.into());
        let lower_bin_width = 2_u32.pow(0);
        let upper_bin_divisions = 2_u32.pow(p.into());

        let max = if n == 64 { u64::MAX } else { 2_u64.pow(n.into()) };

        let lower_bin_count = (cutoff_value / lower_bin_width as u64) as u32;
        let upper_bin_count = (n - cutoff_power) as u32 * upper_bin_divisions;

        Ok(Self {
            max,
            p,
            n,
            cutoff_power,
            cutoff_value,
            lower_bin_count,
            upper_bin_divisions,
            upper_bin_count,
        })
    }

    /// Returns the parameters `a`, `b`, and `n` that were used to create the
    /// config.
    pub(crate) fn params(&self) -> crate::Parameters {
        crate::Parameters {
            p: self.p,
            n: self.n,
        }
    }

    /// Converts a value to a bucket index. Returns an error if the value is
    /// outside of the range for the config.
    pub(crate) fn value_to_index(&self, value: u64) -> Result<usize, Error> {
        if value < self.cutoff_value {
            return Ok(value as usize);
        }

        if value > self.max {
            return Err(Error::OutOfRange);
        }

        let power = 63 - value.leading_zeros();
        let log_bin = power - self.cutoff_power as u32;
        let offset = (value - (1 << power)) >> (power - self.p as u32);

        Ok((self.lower_bin_count + log_bin * self.upper_bin_divisions + offset as u32) as usize)
    }

    /// Convert a bucket index to a lower bound.
    fn index_to_lower_bound(&self, index: usize) -> u64 {
        let g = index as u64 >> self.p;
        let h = index as u64 - g * (1 << self.p);

        if g < 1 {
            h
        } else {
            (1 << (self.p as u64 + g - 1)) + (1 << (g - 1)) * h
        }
    }

    /// Convert a bucket index to a upper inclusive bound.
    fn index_to_upper_bound(&self, index: usize) -> u64 {
        if index as u32 == self.lower_bin_count + self.upper_bin_count - 1 {
            return self.max;
        }
        let g = index as u64 >> self.p;
        let h = index as u64 - g * (1 << self.p) + 1;

        if g < 1 {
            h - 1
        } else {
            (1 << (self.p as u64 + g - 1)) + (1 << (g - 1)) * h - 1
        }
    }

    /// Convert a bucket index to a range.
    pub(crate) fn index_to_range(&self, index: usize) -> RangeInclusive<u64> {
        self.index_to_lower_bound(index)..=self.index_to_upper_bound(index)
    }

    /// Return the total number of bins (buckets) needed for this config.
    pub(crate) fn total_bins(&self) -> usize {
        (self.lower_bin_count + self.upper_bin_count) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(std::mem::size_of::<Config>(), 32);
    }

    #[test]
    // Test that the number of bins matches the expected count
    fn total_bins() {
        let config = Config::new(2, 64).unwrap();
        assert_eq!(config.total_bins(), 252);

        let config = Config::new(7, 64).unwrap();
        assert_eq!(config.total_bins(), 7424);

        let config = Config::new(14, 64).unwrap();
        assert_eq!(config.total_bins(), 835_584);

        let config = Config::new(2, 4).unwrap();
        assert_eq!(config.total_bins(), 12);
    }

    #[test]
    // Test value to index conversions
    fn value_to_idx() {
        let config = Config::new(7, 64).unwrap();
        assert_eq!(config.value_to_index(0), Ok(0));
        assert_eq!(config.value_to_index(1), Ok(1));
        assert_eq!(config.value_to_index(256), Ok(256));
        assert_eq!(config.value_to_index(257), Ok(256));
        assert_eq!(config.value_to_index(258), Ok(257));
        assert_eq!(config.value_to_index(512), Ok(384));
        assert_eq!(config.value_to_index(515), Ok(384));
        assert_eq!(config.value_to_index(516), Ok(385));
        assert_eq!(config.value_to_index(1024), Ok(512));
        assert_eq!(config.value_to_index(1031), Ok(512));
        assert_eq!(config.value_to_index(1032), Ok(513));
        assert_eq!(config.value_to_index(u64::MAX - 1), Ok(7423));
        assert_eq!(config.value_to_index(u64::MAX), Ok(7423));
    }

    #[test]
    // Test index to lower bound conversion
    fn idx_to_lower_bound() {
        let config = Config::new(7, 64).unwrap();
        assert_eq!(config.index_to_lower_bound(0), 0);
        assert_eq!(config.index_to_lower_bound(1), 1);
        assert_eq!(config.index_to_lower_bound(256), 256);
        assert_eq!(config.index_to_lower_bound(384), 512);
        assert_eq!(config.index_to_lower_bound(512), 1024);
        assert_eq!(
            config.index_to_lower_bound(7423),
            18_374_686_479_671_623_680
        );
    }

    #[test]
    // Test index to upper bound conversion
    fn idx_to_upper_bound() {
        let config = Config::new(7, 64).unwrap();
        assert_eq!(config.index_to_upper_bound(0), 0);
        assert_eq!(config.index_to_upper_bound(1), 1);
        assert_eq!(config.index_to_upper_bound(256), 257);
        assert_eq!(config.index_to_upper_bound(384), 515);
        assert_eq!(config.index_to_upper_bound(512), 1031);
        assert_eq!(config.index_to_upper_bound(7423), u64::MAX);
    }

    #[test]
    // Test index to range conversion
    fn idx_to_range() {
        let config = Config::new(7, 64).unwrap();
        assert_eq!(config.index_to_range(0), 0..=0);
        assert_eq!(config.index_to_range(1), 1..=1);
        assert_eq!(config.index_to_range(256), 256..=257);
        assert_eq!(config.index_to_range(384), 512..=515);
        assert_eq!(config.index_to_range(512), 1024..=1031);
        assert_eq!(
            config.index_to_range(7423),
            18_374_686_479_671_623_680..=u64::MAX
        );
    }
}
