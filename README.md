# jemallocator

[![ci]][github actions] [![Latest Version]][crates.io] [![docs]][docs.rs]

This project is the successor of [jemallocator](https://github.com/gnzlbg/jemallocator).

The project is published as `tikv-jemallocator` and `jemallocator` for historical reasons. The two crates are the same except names. It's OK to use either crate. For new projects, `jemallocator` should be used.

> Links against `jemalloc` and provides a `Jemalloc` unit type that implements
> the allocator APIs and can be set as the `#[global_allocator]`

## Overview

The `jemalloc` support ecosystem consists of the following crates:

* `jemalloc-sys`: builds and links against `jemalloc` exposing raw C bindings to it.
* `jemallocator`: provides the `Jemalloc` type which implements the
  `GlobalAlloc` and `Alloc` traits. 
* `jemalloc-ctl`: high-level wrapper over `jemalloc`'s control and introspection
  APIs (the `mallctl*()` family of functions and the _MALLCTL NAMESPACE_)'

## Documentation

* [Latest release (docs.rs)][docs.rs]

To use `jemallocator` add it as a dependency:

```toml
# Cargo.toml
[dependencies]

[target.'cfg(not(target_env = "msvc"))'.dependencies]
jemallocator = "0.4.0"
```

To set `jemallocator::Jemalloc` as the global allocator add this to your project:

```rust
# main.rs
#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;
```

And that's it! Once you've defined this `static` then jemalloc will be used for
all allocations requested by Rust code in the same program.

## Platform support

The following table describes the supported platforms: 

* `build`: does the library compile for the target?
* `run`: do `jemallocator` and `jemalloc-sys` tests pass on the target?
* `jemalloc`: do `jemalloc`'s tests pass on the target?

Tier 1 targets are tested on all Rust channels (stable, beta, and nightly). All
other targets are only tested on Rust nightly.

| Linux targets:                      | build     | run     | jemalloc     |
|-------------------------------------|-----------|---------|--------------|
| `aarch64-unknown-linux-gnu`         | ✓         | ✓       | ✗            |
| `powerpc64le-unknown-linux-gnu`     | ✓         | ✓       | ✗            |
| `x86_64-unknown-linux-gnu` (tier 1) | ✓         | ✓       | ✓            |
| **MacOSX targets:**                 | **build** | **run** | **jemalloc** |
| `x86_64-apple-darwin` (tier 1)      | ✓         | ✓       | ✗            |

## Features

The `jemallocator` crate re-exports the [features of the `jemalloc-sys`
dependency](https://github.com/tikv/jemallocator/blob/master/jemalloc-sys/README.md).

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in `jemallocator` by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.

[Latest Version]: https://img.shields.io/crates/v/jemallocator.svg
[crates.io]: https://crates.io/crates/jemallocator
[docs]: https://docs.rs/jemallocator/badge.svg
[docs.rs]: https://docs.rs/jemallocator/
[ci]: https://github.com/tikv/jemallocator/actions/workflows/main.yml/badge.svg
[github actions]: https://github.com/tikv/jemallocator/actions
