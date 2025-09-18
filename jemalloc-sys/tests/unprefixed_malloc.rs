#[cfg(prefixed)]
#[test]
fn malloc_is_prefixed() {
    assert_ne!(tikv_jemalloc_sys::malloc as usize, libc::malloc as usize)
}

#[cfg(not(prefixed))]
#[test]
fn malloc_is_overridden() {
    assert_eq!(tikv_jemalloc_sys::malloc as usize, libc::malloc as usize)
}

#[cfg(any(
    not(prefixed),
    all(
        feature = "override_allocator_on_supported_platforms",
        target_vendor = "apple"
    ),
))]
#[test]
fn malloc_and_libc_are_interoperable_when_overridden() {
    let ptr = unsafe { tikv_jemalloc_sys::malloc(42) };
    unsafe { libc::free(ptr) };
}
