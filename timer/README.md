# rustcommon-timer

A hash wheel timer implementation focused on low cost addition, cancellation,
and expiration of timers

## Overview

This crate provides a hash wheel timer implementation which can be used to hold
many timers with short timeouts. It is designed to be used for use in providing
timeouts for network requests and as such tries to minimize the cost of adding
and canceling timers

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
