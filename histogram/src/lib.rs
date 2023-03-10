// Copyright 2022 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

mod bucket;
mod error;
mod histogram;
mod percentile;

pub use self::histogram::{Builder, Histogram};
pub use bucket::Bucket;
pub use error::Error;
pub use percentile::Percentile;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // run some test cases for various histogram sizes
    fn num_buckets() {
        let histogram = Histogram::new(0, 2, 10).unwrap();
        assert_eq!(histogram.buckets(), 20);

        let histogram = Histogram::new(0, 10, 20).unwrap();
        assert_eq!(histogram.buckets(), 6144);

        let histogram = Histogram::new(0, 10, 30).unwrap();
        assert_eq!(histogram.buckets(), 11264);

        let histogram = Histogram::new(1, 10, 20).unwrap();
        assert_eq!(histogram.buckets(), 3072);

        let histogram = Histogram::new(0, 9, 20).unwrap();
        assert_eq!(histogram.buckets(), 3328);
    }

    #[test]
    fn percentiles() {
        let histogram = Histogram::new(0, 2, 10).unwrap();

        for v in 1..1024 {
            assert!(histogram.increment(v, 1).is_ok());
            assert!(histogram.percentile(100.0).map(|b| b.high()).unwrap_or(0) >= v);
            assert!(histogram.percentile(100.0).map(|b| b.low()).unwrap_or(0) <= v);
        }
    }
}
