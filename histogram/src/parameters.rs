/// The parameters that determine the histogram bucketing.
/// * `grouping_power` - controls the number of buckets that are used to span
///   consecutive powers of two. Lower values result in less memory usage since
///   fewer buckets will be created. However, this will result in larger
///   relative error as each bucket represents a wider range of values.
/// * `max_value_power` - controls the largest value which can be stored in the
///   histogram. `2^(max_value_power) - 1` is the inclusive upper bound for the
///   representable range of values.
///
/// # How to choose parameters for your data
/// Please see <https://observablehq.com/@iopsystems/h2histogram> for an
/// in-depth discussion about the bucketing strategy and an interactive
/// calculator that lets you explore how these parameters result in histograms
/// with varying error guarantees and memory utilization requirements.
///
/// # The short version
/// ## Grouping Power
/// `grouping_power` should be set such that `2^(-1 * grouping_power)` is an
/// acceptable relative error. Rephrased, we can plug-in the acceptable
/// relative error into `grouping_power = ceil(log2(1/e))`. For example, if we
/// want to limit the error to 0.1% (0.001) we should set `grouping_power = 7`.
///
/// ## Max Value Power
/// `max_value_power` should be the closest power of 2 that is larger than the
/// largest value you expect in your data. If your only guarantee is that the
/// values are all `u64`, then setting this to `64` may be reasonable if you
/// can tolerate a bit of relative error.
///
/// ## Resulting size
///
/// If we want to allow any value in a range of unsigned types, the amount of
/// memory for the histogram is approximately:
///
/// | power | error |     u16 |     u32 |     u64 |
/// |-------|-------|---------|---------|---------|
/// |     2 |   25% | 0.6 KiB |   1 KiB |   2 KiB |
/// |     3 | 12.5% |   1 KiB |   2 KiB |   4 KiB |
/// |     4 | 6.25% |   2 KiB |   4 KiB |   8 KiB |
/// |     5 | 3.13% |   3 KiB |   7 KiB |  15 KiB |
/// |     6 | 1.56% |   6 KiB |  14 KiB |  30 KiB |
/// |     7 | .781% |  10 KiB |  26 KiB |  58 KiB |
/// |     8 | .391% |  18 KiB |  50 KiB | 114 KiB |
/// |     9 | .195% |  32 KiB |  96 KiB | 224 KiB |
/// |    10 | .098% |  56 KiB | 184 KiB | 440 KiB |
/// |    11 | .049% |  96 KiB | 352 KiB | 864 KiB |
/// |    12 | .025% | 160 KiB | 672 KiB | 1.7 MiB |
///
/// ## Sliding Window
///
/// When using the sliding window histograms, the memory utilization is roughly
/// multiplied by the number of slices with two extra histograms worth.
///
/// For example, a histogram spanning 1 minute with 1 second resolution is
/// approximately 62 times larger than a standard histogram. This can
/// necessitate using lower grouping powers to avoid a large memory footprint,
/// particularly when many sliding window histograms are in-use, such as in
/// metrics use-cases.
///
/// # Constraints:
/// * `max_value_power` must be in the range `0..=64`
/// * `max_value_power` must be greater than `grouping_power
#[derive(PartialEq, Debug, Copy, Clone)]
pub struct Parameters {
    pub grouping_power: u8,
    pub max_value_power: u8,
}