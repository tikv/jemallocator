/// `jemalloc` is known not to work on these targets:
pub static UNSUPPORTED_TARGETS: &[&str] = &[
    "rumprun",
    "bitrig",
    "emscripten",
    "fuchsia",
    "redox",
    "wasm32",
];

/// `jemalloc-sys` is not tested on these targets in CI:
pub static UNTESTED_TARGETS: &[&str] = &["openbsd", "msvc"];

/// `jemalloc`'s background_thread support is known not to work on these targets:
pub static NO_BG_THREAD_TARGETS: &[&str] = &["musl"];

/// targets that don't support unprefixed `malloc`
// “it was found that the `realpath` function in libc would allocate with libc malloc
//  (not jemalloc malloc), and then the standard library would free with jemalloc free,
//  causing a segfault.”
// https://github.com/rust-lang/rust/commit/e3b414d8612314e74e2b0ebde1ed5c6997d28e8d
// https://github.com/rust-lang/rust/commit/536011d929ecbd1170baf34e09580e567c971f95
// https://github.com/rust-lang/rust/commit/9f3de647326fbe50e0e283b9018ab7c41abccde3
// https://github.com/rust-lang/rust/commit/ed015456a114ae907a36af80c06f81ea93182a24
pub static NO_UNPREFIXED_MALLOC_TARGETS: &[&str] = &["android", "dragonfly", "musl", "darwin"];
