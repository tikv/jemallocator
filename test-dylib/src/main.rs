//! Test that the symbol address of `malloc` in shared libraries come from
//! the place we'd expect it to.
use core::ffi::{c_char, CStr};
use std::{env, path::Path};

use tikv_jemalloc_sys as _;

extern "C-unwind" {
    fn lookup_malloc_address() -> *const c_char;
}

fn main() {
    let actual = unsafe { CStr::from_ptr(lookup_malloc_address()).to_str().unwrap() };

    if cfg!(target_vendor = "apple") {
        // macOS / Mach-O symbols are not overriden, they are hooked into with
        // `zone_register`.
        assert_eq!(actual, "/usr/lib/system/libsystem_malloc.dylib");
    } else if cfg!(all(target_os = "linux", target_env = "gnu")) {
        if cfg!(feature = "unprefixed_malloc_on_supported_platforms") {
            // When unprefixed, `malloc` is loaded from the current exe.
            // `target/*/debug/test-dylib`
            let dir = env::current_dir().unwrap();
            let exe = env::current_exe().unwrap();
            assert_eq!(Path::new(actual), exe.strip_prefix(dir).unwrap());
        } else if cfg!(target_arch = "x86_64") {
            // Otherwise, the system `libc` contains `malloc`.
            assert_eq!(actual, "/lib/x86_64-linux-gnu/libc.so.6");
        } else if cfg!(target_arch = "x86") {
            assert_eq!(actual, "/lib/i386-linux-gnu/libc.so.6");
        } else if cfg!(target_arch = "aarch64") {
            assert_eq!(actual, "/lib/aarch64-linux-gnu/libc.so.6");
        } else {
            panic!("unknown architecture. {:?}", actual);
        }
    } else {
        panic!("unsupported platform for this test. {:?}", actual);
    };
}
