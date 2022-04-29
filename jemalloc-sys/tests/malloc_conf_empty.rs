#[test]
fn malloc_conf_empty() {
    unsafe {
        assert!(jemalloc_sys::malloc_conf.is_none());
    }
}
