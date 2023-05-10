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
    fn percentiles_1() {
        let histogram = Histogram::new(0, 2, 10).unwrap();

        for v in 1..1024 {
            assert!(histogram.increment(v, 1).is_ok());
            assert_eq!(histogram.percentile(0.0).map(|b| b.high()), Ok(1));

            assert!(histogram.percentile(100.0).map(|b| b.high()).unwrap_or(0) >= v);
            assert!(histogram.percentile(100.0).map(|b| b.low()).unwrap_or(0) <= v);
        }

        let percentiles: Vec<(u64, u64)> = histogram
            .percentiles(&[1.0, 10.0, 25.0, 50.0, 75.0, 90.0, 99.0])
            .unwrap()
            .iter()
            .map(|p| (p.bucket().low(), p.bucket().high()))
            .collect();

        // this histogram config doesn't have much resolution, which results in
        // the upper percentiles falling into buckets that are rather wide
        assert_eq!(
            &percentiles,
            &[
                (8, 11),
                (96, 127),
                (256, 383),
                (512, 767),
                (768, 1023),
                (768, 1023),
                (768, 1023)
            ]
        );
    }

    #[test]
    fn percentiles_2() {
        let histogram = Histogram::new(0, 5, 10).unwrap();

        for v in 1..1024 {
            assert!(histogram.increment(v, 1).is_ok());
            assert_eq!(histogram.percentile(0.0).map(|b| b.high()), Ok(1));

            assert!(histogram.percentile(100.0).map(|b| b.high()).unwrap_or(0) >= v);
            assert!(histogram.percentile(100.0).map(|b| b.low()).unwrap_or(0) <= v);
        }

        let percentiles: Vec<(u64, u64)> = histogram
            .percentiles(&[1.0, 10.0, 25.0, 50.0, 75.0, 90.0, 99.0])
            .unwrap()
            .iter()
            .map(|p| (p.bucket().low(), p.bucket().high()))
            .collect();

        // this histogram config has enough resolution to keep the error lower
        assert_eq!(
            &percentiles,
            &[
                (11, 11),
                (100, 103),
                (256, 271),
                (512, 543),
                (768, 799),
                (896, 927),
                (992, 1023)
            ]
        );
    }

    #[test]
    fn percentiles_3() {
        let histogram = Histogram::builder().build().unwrap();
        histogram.increment(1, 1).unwrap();
        histogram.increment(10000000, 1).unwrap();

        let percentiles = histogram.percentiles(&[25.0, 75.0]).unwrap();

        assert_eq!(histogram.percentile(25.0).map(|b| b.high()), Ok(1));
        assert_eq!(histogram.percentile(75.0).map(|b| b.high()), Ok(10010623));

        assert_eq!(percentiles.get(0).map(|b| b.bucket().high()), Some(1));
        assert_eq!(
            percentiles.get(1).map(|b| b.bucket().high()),
            Some(10010623)
        );
    }

    #[test]
    fn test_increment_and_decrement() {
        let histogram = Histogram::builder().build().unwrap();
        assert_eq!(
            histogram.percentile(0.0).map(|b| b.count()),
            Err(Error::Empty)
        );

        histogram.increment(1, 1).unwrap();
        assert_eq!(histogram.percentile(0.0).map(|b| b.count()), Ok(1));

        histogram.decrement(1, 1).unwrap();
        assert_eq!(
            histogram.percentile(0.0).map(|b| b.count()),
            Err(Error::Empty)
        );
    }
}
