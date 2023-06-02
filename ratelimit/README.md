# ratelimit

A simple ratelimiter that can be shared between threads.

## Overview

This crate provides a ratelimiter that is based around a token bucket. It can
be used in cases where you need to control the rate of some actions or where you
may need to use admission control.

## Usage

The API documentation of this library can be found at
[docs.rs/ratelimit](https://docs.rs/ratelimit/).

## Features

* Simple token bucket ratelimiter for ratelimiting and admission control
* Thread-safe so it can be used as a global ratelimiter for multi-threaded
  programs
* Allows runtime reconfiguration that can be used to alter the effective
  ratelimit or other aspects of its behavior

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Support

Create a [new issue](https://github.com/pelikan-io/rustcommon/issues/new) on GitHub.

## Authors

* Brian Martin <brian@pelikan.io>

A full list of [contributors] can be found on GitHub.

[contributors]: https://github.com/pelikan-io/rustcommon/graphs/contributors?type=a
