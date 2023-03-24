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
    fn percentile() {
        let histogram = Histogram::new(0, 2, 10).unwrap();

        for v in 1..1024 {
            assert!(histogram.increment(v, 1).is_ok());
            assert!(histogram.percentile(100.0).map(|b| b.high()).unwrap_or(0) >= v);
            assert!(histogram.percentile(100.0).map(|b| b.low()).unwrap_or(0) <= v);
        }
    }

    #[test]
    fn percentiles() {
        let histogram = Histogram::builder().build().unwrap();
        histogram.increment(1, 1).unwrap();
        histogram.increment(10000000, 1).unwrap();

        let percentiles = histogram.percentiles(&[25.0, 75.0]).unwrap();

        assert_eq!(histogram.percentile(25.0).map(|b| b.high()), Ok(1));
        assert_eq!(histogram.percentile(75.0).map(|b| b.high()), Ok(10010623));

        for p in &percentiles {
            println!(
                "{} {} {}",
                p.percentile(),
                p.bucket().low(),
                p.bucket().count()
            );
        }

        assert_eq!(percentiles.get(0).map(|b| b.bucket().high()), Some(1));
    }
}
