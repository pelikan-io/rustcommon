# rustcommon-ratelimiter

Token bucket ratelimiting with various refill strategies

## Overview

This crate provides token bucket ratelimiting implementations. The typical
use-case would be to control the rate of requests or other actions.

This particular implementation allows for setting a refill strategy for the
token bucket. This allows for creating noise in the interval between additions
of tokens into the bucket. By doing this, we can create workloads that are
bursty and can more closely mirror production workload characteristics.

## Getting Started

### Building

rustcommon is built with the standard Rust toolchain which can be installed and
managed via [rustup](https://rustup.rs) or by following the directions on the
Rust [website](https://www.rust-lang.org/).

#### View library documentation
```bash
cargo doc --open
```

## Support

Create a [new issue](https://github.com/pelikan-io/rustcommon/issues/new) on GitHub.

## Authors

* Brian Martin <brayniac@gmail.com>

A full list of [contributors] can be found on GitHub.

[contributors]: https://github.com/pelikan-io/rustcommon/graphs/contributors?type=a
