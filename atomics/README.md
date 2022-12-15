# rustcommon-atomics

Atomic types provided with unifying traits for use in generic programming

## Overview

This crate provides wrappers around the atomics found in the rust core library
with the addition of atomic floating point types. The types exported from this
crate are unified through sets of traits which define operations which may be
performed on the atomic types. This makes it possible to use atomic types with
generic programming.

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
