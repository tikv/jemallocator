//! Stub implementation for MSVC targets that forwards to the system allocator

#![allow(unused_variables)]
#![allow(non_camel_case_types)]

/// Void type used for C FFI compatibility.
pub type c_void = core::ffi::c_void;
/// Character type used for C FFI compatibility (equivalent to i8).
pub type c_char = i8;
/// Integer type used for C FFI compatibility (equivalent to i32).
pub type c_int = i32;

/// The malloc configuration string.
/// This is a stub that matches the expected type but doesn't provide any configuration.
#[cfg(not(test))] // Don't define this symbol in test mode to avoid conflicts
#[cfg_attr(prefixed, export_name = "_rjem_malloc_conf")]
#[cfg_attr(not(prefixed), no_mangle)]
pub static mut malloc_conf: *const c_char = 0 as *const c_char;

// Forward to the system allocator using raw FFI calls to MSVC's allocator

// Define Windows CRT allocator functions we'll use internally
#[link(name = "vcruntime")]
extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(num: usize, size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

// Now implement our functions by forwarding to the CRT

/// Allocate `size` bytes of memory.
#[no_mangle]
pub extern "C" fn _rjem_malloc(size: usize) -> *mut c_void {
    unsafe { malloc(size) }
}

/// Allocate and zero-initialize an array of `nmemb` elements of `size` bytes each.
#[no_mangle]
pub extern "C" fn _rjem_calloc(nmemb: usize, size: usize) -> *mut c_void {
    unsafe { calloc(nmemb, size) }
}

/// Resize the memory block pointed to by `ptr` to `size` bytes.
#[no_mangle]
pub extern "C" fn _rjem_realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    unsafe { realloc(ptr, size) }
}

/// Free the memory space pointed to by `ptr`.
#[no_mangle]
pub extern "C" fn _rjem_free(ptr: *mut c_void) {
    unsafe { free(ptr) }
}

/// Control jemalloc's behavior in various ways.
/// In this stub implementation, always returns EINVAL (22).
#[no_mangle]
pub extern "C" fn _rjem_mallctl(
    name: *const c_char,
    oldp: *mut c_void,
    oldlenp: *mut usize,
    newp: *mut c_void, 
    newlen: usize,
) -> c_int {
    // Return error code - not implemented
    22 // EINVAL
}

/// Determine number of bytes that would be allocated by `mallocx`.
/// This stub implementation simply returns the requested size.
#[no_mangle]
pub extern "C" fn _rjem_nallocx(size: usize, flags: c_int) -> usize {
    size
}

/// Allocate `size` bytes of memory with specified `flags`.
/// This stub implementation calls standard malloc.
#[no_mangle]
pub extern "C" fn _rjem_mallocx(size: usize, flags: c_int) -> *mut c_void {
    _rjem_malloc(size)
}

/// Resizes/reallocates memory with specified `flags`.
/// This stub implementation simply returns the size.
#[no_mangle]
pub extern "C" fn _rjem_xallocx(ptr: *mut c_void, size: usize, extra: usize, flags: c_int) -> usize {
    size
}

/// Get size of allocation pointed to by `ptr`.
/// This stub implementation returns 0 as we can't determine the size.
#[no_mangle]
pub extern "C" fn _rjem_sallocx(ptr: *mut c_void, flags: c_int) -> usize {
    0 // We don't know the size
}

/// Free memory allocated by `mallocx`.
/// This stub implementation simply forwards to free.
#[no_mangle]
pub extern "C" fn _rjem_dallocx(ptr: *mut c_void, flags: c_int) {
    _rjem_free(ptr);
}

/// Free memory with specified `size`, allocated by `mallocx`.
/// This stub implementation simply forwards to free.
#[no_mangle]
pub extern "C" fn _rjem_sdallocx(ptr: *mut c_void, size: usize, flags: c_int) {
    _rjem_free(ptr);
}

/// Resize the allocation pointed to by `ptr` to be `size` bytes.
/// This stub implementation forwards to standard realloc.
#[no_mangle]
pub extern "C" fn _rjem_rallocx(ptr: *mut c_void, size: usize, flags: c_int) -> *mut c_void {
    _rjem_realloc(ptr, size)
}

/// Get the usable size of the allocation pointed to by `ptr`.
/// This stub implementation just returns 0 since we can't know.
#[no_mangle]
pub extern "C" fn _rjem_malloc_usable_size(ptr: *const c_void) -> usize {
    0 // We don't know the usable size
}

/// Control jemalloc's behavior by name and index.
/// Always returns EINVAL (22) in this stub implementation.
#[no_mangle]
pub extern "C" fn _rjem_mallctlbymib(
    mib: *const usize,
    miblen: usize,
    oldp: *mut c_void,
    oldlenp: *mut usize,
    newp: *mut c_void,
    newlen: usize,
) -> c_int {
    22 // EINVAL
}

/// Convert a name to a Management Interface Byte (MIB).
/// Always returns EINVAL (22) in this stub implementation.
#[no_mangle]
pub extern "C" fn _rjem_mallctlnametomib(
    name: *const c_char,
    mibp: *mut usize,
    miblenp: *mut usize,
) -> c_int {
    22 // EINVAL
}

/// Allocate `size` bytes of memory with specified `flags`.
/// This stub implementation forwards to standard malloc.
#[no_mangle]
pub extern "C" fn mallocx(size: usize, flags: c_int) -> *mut c_void {
    _rjem_mallocx(size, flags)
}

/// Resize the allocation pointed to by `ptr` to be `size` bytes.
/// This stub implementation forwards to standard realloc.
#[no_mangle]
pub extern "C" fn rallocx(ptr: *mut c_void, size: usize, flags: c_int) -> *mut c_void {
    _rjem_rallocx(ptr, size, flags)
}

/// Free memory with specified `size`, allocated by `mallocx`.
/// This stub implementation simply forwards to free.
#[no_mangle]
pub extern "C" fn sdallocx(ptr: *mut c_void, size: usize, flags: c_int) {
    _rjem_sdallocx(ptr, size, flags);
}

/// Get the usable size of the allocation pointed to by `ptr`.
/// This stub implementation just returns 0 since we can't know.
#[no_mangle]
pub extern "C" fn malloc_usable_size(ptr: *const c_void) -> usize {
    _rjem_malloc_usable_size(ptr)
}

/// Control jemalloc's behavior in various ways.
/// In this stub implementation, always returns EINVAL (22).
#[no_mangle]
pub extern "C" fn mallctl(
    name: *const c_char,
    oldp: *mut c_void,
    oldlenp: *mut usize,
    newp: *mut c_void,
    newlen: usize,
) -> c_int {
    _rjem_mallctl(name, oldp, oldlenp, newp, newlen)
}

/// Convert a name to a Management Interface Byte (MIB).
/// Always returns EINVAL (22) in this stub implementation.
#[no_mangle]
pub extern "C" fn mallctlnametomib(
    name: *const c_char,
    mibp: *mut usize,
    miblenp: *mut usize,
) -> c_int {
    _rjem_mallctlnametomib(name, mibp, miblenp)
}

/// Control jemalloc's behavior by name and index.
/// Always returns EINVAL (22) in this stub implementation.
#[no_mangle]
pub extern "C" fn mallctlbymib(
    mib: *const usize,
    miblen: usize,
    oldp: *mut c_void,
    oldlenp: *mut usize,
    newp: *mut c_void,
    newlen: usize,
) -> c_int {
    _rjem_mallctlbymib(mib, miblen, oldp, oldlenp, newp, newlen)
}
