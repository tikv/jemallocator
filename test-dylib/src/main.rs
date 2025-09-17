//! Test that the symbol address of `malloc` in shared libraries come from
//! the place we'd expect it to.
use std::ffi::{c_char, c_void, CStr};

use tikv_jemalloc_sys as _;

extern "C-unwind" {
    fn dep_lookup_malloc_address() -> *const c_char;
    fn dep_malloc(size: libc::size_t) -> *mut c_void;
    fn dep_free(ptr: *mut c_void);
}

fn lookup_malloc_address() -> *const c_char {
    unsafe {
        let mut info: libc::Dl_info = core::mem::zeroed();
        let fnptr: unsafe extern "C" fn(libc::size_t) -> *mut c_void = libc::malloc;
        let fnptr = fnptr as *const c_void;
        if libc::dladdr(fnptr, &mut info) == 0 {
            libc::printf(b"failed finding `malloc`\n\0".as_ptr().cast());
            libc::abort();
        }
        info.dli_fname
    }
}
fn main() {
    // Check that pointers created with `malloc` in a dylib dependency can be
    // free'd with `free` here, or vice-versa.
    let ptr = unsafe { libc::malloc(10) };
    unsafe { dep_free(ptr) };
    let ptr = unsafe { dep_malloc(10) };
    unsafe { libc::free(ptr) };

    // If overidden, test that the same is true for `tikv_jemalloc_sys`'
    // symbols being interoperable with `free`.
    if cfg!(feature = "unprefixed_malloc_on_supported_platforms") {
        let ptr = unsafe { tikv_jemalloc_sys::malloc(10) };
        unsafe { dep_free(ptr) };
        let ptr = unsafe { tikv_jemalloc_sys::malloc(10) };
        unsafe { libc::free(ptr) };

        let ptr = unsafe { libc::malloc(10) };
        unsafe { tikv_jemalloc_sys::free(ptr) };
        let ptr = unsafe { dep_malloc(10) };
        unsafe { tikv_jemalloc_sys::free(ptr) };
    }

    // Extra check that the `malloc` symbol was actually from the same place.
    let dep = unsafe { CStr::from_ptr(dep_lookup_malloc_address()) };
    let here = unsafe { CStr::from_ptr(lookup_malloc_address()) };
    assert_eq!(dep, here);
}
