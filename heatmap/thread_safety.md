A `Heatmap` is made up of a collection of `Histogram`s covering the `span` of
interest. When using a windowed approach to discretize timespans, a common
technique is to create many small windows of the same `resolution`. The
`Heatmap` is then associated with and compared to a clock, where the passing of
time equal to `resolution` generates a "tick". Each tick introduces a new window
and therefore `Histogram`, while the oldest `Histogram` is retired. Compared to
periodically resetting the entire `Histogram`, this approach ensures a smooth
reading of the underlying distribution, as each tick only refreshes a small
portion of the data.

## Basic Operation

A `Histogram` needs to support a couple of basic operations expected by the
users:

- `increment`: an update that increments the count of a particular bucket;
- `summary`: a read operation that reports the counts from all buckets of the
  span covered.

The underlying slice construction as well as windowing also implies that each
`Histogram` needs to reset per-bucket count to zero when the corresponding tick
expires. And because a `Histogram` is not a singleton but a collection of
counts organized by buckets, one needs to consider how to accomplish this
operation without intefering with the user facing operations both in terms
of data correctness as well as runtime performance.

## Performance and Concurrency Considerations

The most numerous operation is likely `increment` when using `Heatmap` in a
service. And if said service is multithreaded, which is increasingly likely
in a modern software architecture, this operation would also be called by
multiple threads. Furthermore, `increment` is typically called on the fast
path of request handling. These requirements mean thread-safe access with
minimal overhead. On the flip side, `increment` only needs updating one variable
which is also of a primitive type, and on most CPU architectures that guarantee
comes "for free". Therefore, `increment` from different threads won't interfere
with each other.

The summary stats of all time slices can be obtained by summing up counts by
bucket from all active `Histogram` slices. However, this makes `summary`
relatively expensive to perform, as one may need to add hundreds of thousands
of numbers for a reasonably fine-grained `Heatmap` with decent range coverage.
To ensure reporting doesn't feel sluggish, an optimization is introduced to
cache the current summary in a separate `Histogram`. The summary and time-sliced
`Histogram`s are not always consistant, but their differences are no more than
what `increment`s are currently in-flight, which seems to be a reasonable
tradeoff in exchange for a 10-100x speed-up on `summary` performance.

The more thorny case is when a tick expires, which then triggers resetting a
`Histogram` slice. It's undesirable for `increment` to land in a `Histogram`
that is being cleared out, since having the increment succeed after the reset
will pollute reporting in the future (the non-zero value is now treated as
belonging to a different tick versus just a count discrepancy but for the
correct span). This could also introduce temporary inconsistencies, including
the possibility of underflow, into the summary `Histogram` if only atomicity
is guaranteed. (As an exercise, readers are encouraged
to construct some examples that demonstrate this behavior assuming updates to
the summary and time-sliced `Histogram` can be reordered and intermixed with
resetting a `Histogram` slice.)

Fortunately, there is a simple fix to avoid concurrent increment and reset in
normal cases. We add an extra slice on top of what is necessary to cover the
`span` of the `Heatmap`, to ensure no `increment` is ever issued against the
`Histogram` being cleared. With this extra buffer, all we need to do is to
atomically move the tick forward and ensure that change is visible by all
threads (by using the `Release` ordering), before starting to clear the counters
of the oldest slice(s). The time range lookup will spare the now out of `span`
`Histogram` from ongoing `increment`s.
