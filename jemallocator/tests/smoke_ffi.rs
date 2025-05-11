// Work around https://github.com/gnzlbg/jemallocator/issues/19
#[global_allocator]
static A: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[test]
#[cfg(not(target_env = "msvc"))]
fn smoke() {
    unsafe {
        let ptr = tikv_jemalloc_sys::malloc(4);
        *(ptr as *mut u32) = 0xDECADE;
        assert_eq!(*(ptr as *mut u32), 0xDECADE);
        tikv_jemalloc_sys::free(ptr);
    }
}

#[test]
#[cfg(target_env = "msvc")]
fn smoke_msvc() {
    unsafe {
        // On MSVC we use the system allocator through our stub
        let ptr = tikv_jemalloc_sys::malloc(4);
        assert!(!ptr.is_null());
        *(ptr as *mut u32) = 0xDECADE;
        assert_eq!(*(ptr as *mut u32), 0xDECADE);
        tikv_jemalloc_sys::free(ptr);
    }
}
