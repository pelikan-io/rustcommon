# rustcommon

rustcommon is a collection of common libraries we use in our Rust projects. This
includes datastructures, logging, metrics, timers, and ratelimiting.

## Overview

rustcommon is a workspace repository which contains several crates (libraries)
which act as foundational libraries for other Rust projects, such as Pelikan,
rpc-perf, and Rezolus.

Each crate within this repository contains its own readme and changelog
detailing the purpose and history of the library.

## Getting Started

### Building

rustcommon is built with the standard Rust toolchain which can be installed and
managed via [rustup](https://rustup.rs) or by following the directions on the
Rust [website](https://www.rust-lang.org/).

#### Clone and build rustcommon from source
```bash
git clone https://github.com/pelikan-io/rustcommon
cd rustcommon

# run tests
cargo test --all
```

## Support

Create a [new issue](https://github.com/pelikan-io/rustcommon/issues/new) on GitHub.

## Authors

* Brian Martin <brayniac@gmail.com>

A full list of [contributors] can be found on GitHub.

[contributors]: https://github.com/pelikan-io/rustcommon/graphs/contributors?type=a
