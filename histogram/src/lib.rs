// Copyright 2022 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

mod bucket;
mod compact;
mod error;
mod histogram;
mod percentile;

pub use self::histogram::{Builder, Histogram};
pub use bucket::Bucket;
pub use compact::CompactHistogram;
pub use error::Error;
pub use percentile::Percentile;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder() {
        let h = Histogram::builder().build().unwrap();
        let p = h.parameters();

        assert_eq!(p.0, 0);
        assert_eq!(p.1, 10);
        assert_eq!(p.2, 30);
    }

    #[test]
    fn min_resolution() {
        let h = Histogram::builder().min_resolution(10).build().unwrap();
        assert_eq!(h.parameters().0, 3);

        let h = Histogram::builder().min_resolution(8).build().unwrap();
        assert_eq!(h.parameters().0, 3);

        let h = Histogram::builder().min_resolution(0).build().unwrap();
        assert_eq!(h.parameters().0, 0);
    }

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
    fn increment_and_decrement() {
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

    #[test]
    fn compact_histogram() {
        let h = CompactHistogram::new();
        assert_eq!(h.m, 0);
        assert_eq!(h.r, 0);
        assert_eq!(h.n, 0);
        assert_eq!(&h.index, &[]);
        assert_eq!(&h.count, &[]);

        assert_eq!(Histogram::try_from(&h).is_err(), true);
    }

    #[test]
    fn hydrate_and_dehydrate() {
        let histogram = Histogram::new(0, 5, 10).unwrap();

        for v in (1..1024).step_by(128) {
            assert!(histogram.increment(v, 1).is_ok());
        }

        let h = CompactHistogram::from(&histogram);
        assert_eq!(h.m, 0);
        assert_eq!(h.r, 5);
        assert_eq!(h.n, 10);
        assert_eq!(&h.index, &[1, 64, 80, 88, 96, 100, 104, 108]);
        assert_eq!(&h.count, &[1, 1, 1, 1, 1, 1, 1, 1]);

        let rehydrated = Histogram::try_from(&h).unwrap();
        assert_eq!(rehydrated.parameters(), histogram.parameters());
        assert_eq!(rehydrated.buckets(), histogram.buckets());
        assert!(itertools::equal(
            rehydrated.into_iter(),
            histogram.into_iter()
        ));
    }
}
